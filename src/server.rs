use crate::{
    protocol::{
        BotMarker, BulletMarker, Inputs, PlayerId, PlayerMarker, PlayerState, Score, WorldConfig,
    },
    shared::{
        BULLET_COLLISION_DISTANCE_CHECK, PlayerAnimationTimer, PlayerSpriteSheetResource,
        SEND_INTERVAL, SERVER_ADDR, SHARED_SETTINGS, get_player_anim_config,
        shared_animation_behaviour, shared_movement_behaviour, shared_world_generator,
    },
};
use aeronet_websocket::server::ServerConfig;
use avian2d::prelude::{
    Collider, LinearVelocity, PhysicsSchedule, Position, RigidBody, SpatialQueryFilter,
};
use bevy::prelude::*;
use leafwing_input_manager::prelude::ActionState;
use lightyear_avian2d::prelude::{
    LagCompensationHistory, LagCompensationPlugin, LagCompensationSpatialQuery,
    LagCompensationSystems,
};

use lightyear::{
    netcode::{NetcodeServer, prelude::server},
    prelude::{
        server::{ClientOf, Start, WebSocketServerIo},
        *,
    },
};

pub struct GameServerPlugin;

impl Plugin for GameServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(LagCompensationPlugin);
        app.add_systems(Startup, (start_server, generate_seed, spawn_bots).chain());
        // app.add_systems(FixedUpdate, (movement, animation));
        app.add_observer(on_player_link);
        app.add_observer(on_player_connected);
        app.add_observer(on_seed_generated);
        // the lag compensation systems need to run after LagCompensationSet::UpdateHistory
        // app.add_systems(FixedUpdate, interpolated_bot_movement);
        app.add_systems(
            PhysicsSchedule,
            // lag compensation collisions must run after the SpatialQuery has been updated
            compute_hit_lag_compensation.in_set(LagCompensationSystems::Collisions),
        );
        // app.add_systems(
        //     FixedPostUpdate,
        //     // check collisions after physics have run
        //     // compute_hit_prediction.after(PhysicsSystems::StepSimulation),
        //     compute_hit_prediction.after(PhysicsSystems::StepSimulation),
        // );

        // app.add_systems(Update, debug_server_replicate);
    }
}

fn on_player_link(trigger: On<Add, LinkOf>, mut commands: Commands) {
    info!(
        "Incoming UDP packet → spawned LinkOf entity: {:?}",
        trigger.entity
    );
    commands.entity(trigger.entity).insert((
        ReplicationSender::new(SEND_INTERVAL, SendUpdatesMode::SinceLastAck, false),
        Name::from("Client"),
    ));
}

pub fn on_seed_generated(
    trigger: On<Add, WorldConfig>,
    query: Single<&WorldConfig>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    #[cfg(all(not(feature = "atlas"), feature = "render"))] array_texture_loader: Res<
        ArrayTextureLoader,
    >,
) {
    info!("World config was generated {:#?}!", trigger.entity);

    //generate world using seed, common function
    shared_world_generator(query.seed, query.world_size, commands, asset_server);
}

fn on_player_connected(
    trigger: On<Add, Connected>,
    query: Query<&RemoteId, With<ClientOf>>,
    mut commands: Commands,
    player_resources: Res<PlayerSpriteSheetResource>,
    replicated_players: Query<
        (Entity, &InitialReplicated),
        (Added<InitialReplicated>, With<PlayerId>),
    >,
) {
    info!(
        "Handshake complete → client fully connected: {:?}",
        trigger.entity
    );

    let Ok(client_id) = query.get(trigger.entity) else {
        return;
    };

    let client_id = client_id.0;

    let entity = commands
        .spawn((
            // PlayerPosition::default(),
            PlayerState::default(),
            Score(0),
            PlayerId(client_id),
            PlayerMarker,
            ActionState::<Inputs>::default(),
            //
            DisableReplicateHierarchy,
            get_player_anim_config(),
            //
            RigidBody::Kinematic,
            //
            Replicate::to_clients(NetworkTarget::All),
            PredictionTarget::to_clients(NetworkTarget::Single(client_id)),
            InterpolationTarget::to_clients(NetworkTarget::AllExceptSingle(client_id)),
            ControlledBy {
                owner: trigger.entity,
                lifetime: Default::default(),
            },
            Transform::default(),
            GlobalTransform::default(),
            InheritedVisibility::default(),
        ))
        .with_children(|parent| {
            parent.spawn((
                //visuals
                InheritedVisibility::default(),
                Transform::from_scale(Vec3::splat(6.0)),
                Sprite::from_atlas_image(
                    player_resources.player_image.clone(),
                    TextureAtlas {
                        layout: player_resources.atlas.clone(),
                        index: 0,
                    },
                ),
                PlayerAnimationTimer::new(2),
            ));
        })
        .id();

    info!(
        "Created player entity {:?} for client {:?}",
        entity, client_id
    );
}

pub fn start_server(mut commands: Commands) {
    info!("Server created");

    let sans = vec![
        "localhost".to_string(),
        "127.0.0.1".to_string(),
        "::1".to_string(),
    ];
    let config = ServerConfig::builder()
        .with_bind_address(SERVER_ADDR)
        .with_identity(lightyear::websocket::server::Identity::self_signed(sans).unwrap());

    let server = commands
        .spawn((
            NetcodeServer::new(server::NetcodeConfig {
                protocol_id: SHARED_SETTINGS.protocol_id,
                private_key: SHARED_SETTINGS.private_key,
                ..Default::default()
            }),
            LocalAddr(SERVER_ADDR),
            // ServerUdpIo::default(),
            WebSocketServerIo { config },
        ))
        .id();
    commands.trigger(Start { entity: server });
}

pub fn generate_seed(mut commands: Commands) {
    let seed: u32 = rand::random();
    let world_config = commands.spawn((
        WorldConfig {
            seed,
            world_size: 64,
        },
        Replicate::to_clients(NetworkTarget::All),
    ));
}

pub fn spawn_bots(mut commands: Commands) {
    static BOT_RADIUS: f32 = 15.0;
    // commands.spawn((
    //     BotMarker,
    //     Replicate::to_clients(NetworkTarget::All),
    //     InterpolationTarget::to_clients(NetworkTarget::All),
    //     RigidBody::Kinematic,
    //     Collider::circle(BOT_RADIUS),
    //     LagCompensationHistory::default(),
    //     Transform::from_xyz(200.0, 10.0, 0.0),
    //     Visibility::default(),
    //     DisableReplicateHierarchy,
    // ));
}

/// Compute hits if the bullet hits the bot, and increment the score on the player
pub(crate) fn compute_hit_lag_compensation(
    // instead of directly using avian's SpatialQuery, we want to use the LagCompensationSpatialQuery
    // to apply lag-compensation (i.e. compute the collision between the bullet and the collider as it
    // was seen by the client when they fired the shot)
    mut commands: Commands,
    timeline: Res<LocalTimeline>,
    query: LagCompensationSpatialQuery,
    bullets: Query<
        (Entity, &PlayerId, &Position, &LinearVelocity, &ControlledBy),
        With<BulletMarker>,
    >,
    // the InterpolationDelay component is stored directly on the client entity
    // (the server creates one entity for each client to store client-specific
    // metadata)
    client_query: Query<&InterpolationDelay, With<ClientOf>>,
    mut player_query: Query<(&mut Score, &PlayerId), With<PlayerMarker>>,
) {
    let tick = timeline.tick();
    bullets
        .iter()
        .for_each(|(entity, id, position, velocity, controlled_by)| {
            let Ok(delay) = client_query.get(controlled_by.owner) else {
                error!("Could not retrieve InterpolationDelay for client {id:?}");
                return;
            };
            if let Some(hit_data) = query.cast_ray(
                // the delay is sent in every input message; the latest InterpolationDelay received
                // is stored on the client entity
                *delay,
                position.0,
                Dir2::new_unchecked(velocity.0.normalize()),
                // TODO: shouldn't this be based on velocity length?
                BULLET_COLLISION_DISTANCE_CHECK,
                false,
                &mut SpatialQueryFilter::default(),
            ) {
                info!(
                    ?tick,
                    ?hit_data,
                    ?entity,
                    "Collision with interpolated bot! Despawning bullet"
                );
                // if there is a hit, increment the score
                player_query
                    .iter_mut()
                    .find(|(_, player_id)| player_id.0 == id.0)
                    .map(|(mut score, _)| {
                        score.0 += 1;
                    });
                commands.entity(entity).despawn();
            }
        })
}

// pub(crate) fn compute_hit_prediction(
//     mut commands: Commands,
//     timeline: Res<LocalTimeline>,
//     query: SpatialQuery,
//     bullets: Query<(Entity, &PlayerId, &Position, &LinearVelocity), With<BulletMarker>>,
//     // bot_query: Query<(), With<PredictedBot>>,
//     // the InterpolationDelay component is stored directly on the client entity
//     // (the server creates one entity for each client to store client-specific
//     // metadata)
//     mut player_query: Query<(&mut Score, &PlayerId), With<PlayerMarker>>,
// ) {
//     let tick = timeline.tick();
//     bullets.iter().for_each(|(entity, id, position, velocity)| {
//         if let Some(hit_data) = query.cast_ray_predicate(
//             position.0,
//             Dir2::new_unchecked(velocity.0.normalize()),
//             // TODO: shouldn't this be based on velocity length?
//             BULLET_COLLISION_DISTANCE_CHECK,
//             false,
//             &SpatialQueryFilter::default(),
//             &|entity| {
//                 // only confirm the hit on predicted bots
//                 bot_query.get(entity).is_ok()
//             },
//         ) {
//             info!(
//                 ?tick,
//                 ?hit_data,
//                 ?entity,
//                 "Collision with predicted bot! Despawn bullet"
//             );
//             // if there is a hit, increment the score
//             player_query
//                 .iter_mut()
//                 .find(|(_, player_id)| player_id.0 == id.0)
//                 .map(|(mut score, _)| {
//                     score.0 += 1;
//                 });
//             commands.entity(entity).despawn();
//         }
//     })
// }
