use crate::protocol::Direction;
use crate::protocol::*;
use crate::shared::LOCAL_ADDR;
use crate::shared::PlayerAnimationTimer;
use crate::shared::PlayerSpriteSheetResource;
use crate::shared::SERVER_ADDR;
use crate::shared::SHARED_SETTINGS;
use crate::shared::get_player_anim_config;
use crate::shared::shared_animation_behaviour;
use crate::shared::shared_movement_behaviour;
use aeronet_websocket::client::ClientConfig;
use bevy::prelude::*;
use lightyear::netcode::NetcodeClient;
use lightyear::prelude::client::NetcodeConfig;
use lightyear::prelude::client::WebSocketClientIo;
use lightyear::prelude::client::input::*;
use lightyear::prelude::input::native::*;
use lightyear::prelude::*;
use lightyear::websocket::client::WebSocketTarget;

pub struct GameClientPlugin {
    pub client_id: u64,
}

#[derive(Resource)]
pub struct ClientId {
    pub client_id: u64,
}

impl Plugin for GameClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, startup);
        app.insert_resource(ClientId {
            client_id: self.client_id,
        });
        app.add_systems(
            FixedPreUpdate,
            buffer_input.in_set(InputSystems::WriteClientInputs),
        );
        app.add_systems(FixedUpdate, local_player_movement);
        app.add_systems(FixedUpdate, local_player_animation);
        // app.add_systems(Update, debug_sync);
        app.add_observer(handle_predicted_spawn);
        app.add_observer(handle_interpolated_spawn);
    }
}

// fn debug_sync(
//     query: Query<(
//         Entity,
//         Option<&ActionState<Inputs>>,
//         Option<&Predicted>,
//         Option<&InputMarker<Inputs>>,
//     )>,
// ) {
//     for (ent, action_state, pred, marker) in query.iter() {
//         info!(
//             "ENTITY: {:?} | HasActionState: {} | Predicted: {} | HasMarker: {}",
//             ent,
//             action_state.is_some(),
//             pred.is_some(),
//             marker.is_some()
//         );
//     }
// }

pub fn startup(mut commands: Commands, config: Res<ClientId>) {
    let auth = Authentication::Manual {
        client_id: config.client_id,
        server_addr: SERVER_ADDR,
        protocol_id: SHARED_SETTINGS.protocol_id,
        private_key: SHARED_SETTINGS.private_key,
    };
    let config = {
        #[cfg(target_family = "wasm")]
        {
            ClientConfig::default()
        }
        #[cfg(not(target_family = "wasm"))]
        {
            ClientConfig::builder().with_no_cert_validation()
        }
    };

    // ClientConfig::builder().with_no_cert_validation(),
    let client = commands
        .spawn((
            Client::default(),
            NetcodeClient::new(auth, NetcodeConfig::default()).unwrap(),
            // UdpIo::default(),
            WebSocketClientIo {
                config,
                target: WebSocketTarget::Addr(Default::default()),
            },
            LocalAddr(LOCAL_ADDR), // обязательно
            PeerAddr(SERVER_ADDR), // сервер
            ReplicationReceiver::default(),
            PredictionManager::default(),
        ))
        .id();

    commands.trigger(Connect { entity: client });
    info!("Client created");
}

pub fn buffer_input(
    mut query: Query<&mut ActionState<Inputs>, With<InputMarker<Inputs>>>,
    keypress: Res<ButtonInput<KeyCode>>,
) {
    if let Ok(mut action_state) = query.single_mut() {
        let mut direction = Direction {
            front: false,
            back: false,
            left: false,
            right: false,
        };

        if keypress.pressed(KeyCode::KeyW) || keypress.pressed(KeyCode::ArrowUp) {
            direction.back = true;
        }
        if keypress.pressed(KeyCode::KeyS) || keypress.pressed(KeyCode::ArrowDown) {
            direction.front = true;
        }
        if keypress.pressed(KeyCode::KeyA) || keypress.pressed(KeyCode::ArrowLeft) {
            direction.left = true;
        }
        if keypress.pressed(KeyCode::KeyD) || keypress.pressed(KeyCode::ArrowRight) {
            direction.right = true;
        }
        // we always set the value. Setting it to None means that the input was missing, it's not the same
        // as saying that the input was 'no keys pressed'
        action_state.0 = Inputs::Direction(direction);
    }
}

fn handle_predicted_spawn(
    trigger: On<Add, Predicted>,
    mut commands: Commands,
    player_resources: Res<PlayerSpriteSheetResource>,
) {
    let entity = trigger.entity;
    info!("Adding InputMarker to entity {:?}", entity);

    commands.entity(entity).insert((
        InputMarker::<Inputs>::default(),
        Sprite::from_atlas_image(
            player_resources.player_image.clone(),
            TextureAtlas {
                layout: player_resources.atlas.clone(),
                index: 0,
            },
        ),
        Transform::from_scale(Vec3::splat(6.0)),
        PlayerAnimationTimer::new(2),
    ));
}

fn handle_interpolated_spawn(
    trigger: On<Add, Interpolated>,
    mut commands: Commands,
    player_resources: Res<PlayerSpriteSheetResource>,
) {
    let entity = trigger.entity;
    // info!("Adding InputMarker to entity {:?}", entity);

    commands.entity(entity).insert((
        // InputMarker::<Inputs>::default(),
        Sprite::from_atlas_image(
            player_resources.player_image.clone(),
            TextureAtlas {
                layout: player_resources.atlas.clone(),
                index: 0,
            },
        ),
        Transform::from_scale(Vec3::splat(6.0)),
        PlayerAnimationTimer::new(2),
    ));
}

fn local_player_movement(
    // timeline: Single<&LocalTimeline>,
    mut position_query: Query<(&mut PlayerPosition, &ActionState<Inputs>), With<Predicted>>,
) {
    // let tick = timeline.tick();
    for (position, input) in position_query.iter_mut() {
        // trace!(?tick, ?position, ?input, "client");
        // NOTE: be careful to directly pass Mut<PlayerPosition>
        // getting a mutable reference triggers change detection, unless you use `as_deref_mut()`
        shared_movement_behaviour(position, input);
    }
}

fn local_player_animation(
    // timeline: Res<LocalTimeline>,
    mut player_query: Query<
        (
            &mut PlayerState,
            // &mut PlayerAnimations,
            &ActionState<Inputs>,
        ),
        With<Predicted>,
    >,
) {
    // let tick = timeline.tick();
    for (state, inputs) in player_query.iter_mut() {
        // trace!(?tick, ?state, ?anims, ?inputs, "server");
        shared_animation_behaviour(state, inputs);
    }
}
