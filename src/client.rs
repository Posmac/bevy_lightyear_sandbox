use crate::protocol::*;
use crate::shared::LOCAL_ADDR;
use crate::shared::PlayerAnimationTimer;
use crate::shared::PlayerSpriteSheetResource;
use crate::shared::SERVER_ADDR;
use crate::shared::SHARED_SETTINGS;
use crate::shared::shared_animation_behaviour;
use crate::shared::shared_movement_behaviour;
use crate::shared::shared_world_generator;
use aeronet_websocket::client::ClientConfig;
use avian2d::prelude::Position;
use avian2d::prelude::Rotation;
use bevy::prelude::*;
use leafwing_input_manager::plugin::InputManagerSystem;
use leafwing_input_manager::prelude::ActionState;
use leafwing_input_manager::prelude::InputMap;
use lightyear::frame_interpolation::FrameInterpolate;
use lightyear::input::client::InputSystems;
use lightyear::netcode::NetcodeClient;
use lightyear::prelude::client::NetcodeConfig;
use lightyear::prelude::client::WebSocketClientIo;
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
            update_cursor_state_from_window
                .before(InputSystems::BufferClientInputs)
                .in_set(InputManagerSystem::ManualControl), // buffer_input.in_set(InputSystems::WriteClientInputs),
        );
        // app.add_systems(FixedUpdate, local_player_movement);
        // app.add_systems(FixedUpdate, local_player_animation);
        // app.add_systems(Update, debug_sync);
        app.add_systems(Update, camera_follow);
        app.add_observer(handle_predicted_spawn);
        app.add_observer(handle_interpolated_spawn);
        app.add_observer(handle_world_config_spawn);
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
//
fn camera_follow(
    player_query: Single<&Position, (With<Predicted>, With<PlayerMarker>)>,
    camera_query: Single<&mut Transform, With<Camera2d>>,
) {
    // let player_pos = player_query.into_inner();
    // let mut camera_transform = camera_query.into_inner();
    // let target = Vec3::new(player_pos.0.x, player_pos.0.y, 0.0);
    // camera_transform.translation = camera_transform.translation.lerp(target, 0.1);
}

/// Compute the world-position of the cursor and set it in the DualAxis input
fn update_cursor_state_from_window(
    window: Single<&Window>,
    q_camera: Query<(&Camera, &GlobalTransform)>,
    mut action_state_query: Query<&mut ActionState<Inputs>, With<Predicted>>,
) {
    let Ok((camera, camera_transform)) = q_camera.single() else {
        error!("Expected to find only one camera");
        return;
    };
    if let Some(world_position) = window
        .cursor_position()
        .and_then(|cursor| Some(camera.viewport_to_world(camera_transform, cursor).unwrap()))
        .map(|ray| ray.origin.truncate())
    {
        for mut action_state in action_state_query.iter_mut() {
            action_state.set_axis_pair(&Inputs::Mouse, world_position);
        }
    }
}

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

// pub fn buffer_input(
//     mut query: Query<&mut ActionState<Inputs>, With<InputMarker<Inputs>>>,
//     keypress: Res<ButtonInput<KeyCode>>,
// ) {
//     if let Ok(mut action_state) = query.single_mut() {
//         let mut direction = Direction {
//             front: false,
//             back: false,
//             left: false,
//             right: false,
//         };

//         if keypress.pressed(KeyCode::KeyW) || keypress.pressed(KeyCode::ArrowUp) {
//             direction.back = true;
//         }
//         if keypress.pressed(KeyCode::KeyS) || keypress.pressed(KeyCode::ArrowDown) {
//             direction.front = true;
//         }
//         if keypress.pressed(KeyCode::KeyA) || keypress.pressed(KeyCode::ArrowLeft) {
//             direction.left = true;
//         }
//         if keypress.pressed(KeyCode::KeyD) || keypress.pressed(KeyCode::ArrowRight) {
//             direction.right = true;
//         }
//         // we always set the value. Setting it to None means that the input was missing, it's not the same
//         // as saying that the input was 'no keys pressed'
//         action_state.0 = Inputs::Direction(direction);
//     }
// }

fn handle_predicted_spawn(
    trigger: On<Add, (PlayerId, Predicted)>,
    mut commands: Commands,
    player_resources: Res<PlayerSpriteSheetResource>,
    mut query: Query<&PlayerMarker, With<Predicted>>,
) {
    let entity = trigger.entity;
    if let Ok(marker) = query.get_mut(trigger.entity) {
        info!("Adding InputMarker to entity {:?} {:?}", entity, marker);

        commands.entity(entity).insert((
            Sprite::from_atlas_image(
                player_resources.player_image.clone(),
                TextureAtlas {
                    layout: player_resources.atlas.clone(),
                    index: 0,
                },
            ),
            // Transform::from_scale(Vec3::splat(6.0)),
            PlayerAnimationTimer::new(2),
            InputMap::new([
                (Inputs::Up, KeyCode::KeyW),
                (Inputs::Down, KeyCode::KeyS),
                (Inputs::Left, KeyCode::KeyA),
                (Inputs::Right, KeyCode::KeyD),
                (Inputs::Shoot, KeyCode::Space),
                (Inputs::Up, KeyCode::ArrowUp),
                (Inputs::Down, KeyCode::ArrowDown),
                (Inputs::Left, KeyCode::ArrowLeft),
                (Inputs::Right, KeyCode::ArrowRight),
            ]),
            FrameInterpolate::<Transform>::default(),
        ));
    }
}

fn handle_world_config_spawn(
    trigger: On<Add, WorldConfig>,
    mut commands: Commands,
    world_config: Single<&WorldConfig>,
    asset_server: Res<AssetServer>,
    #[cfg(all(not(feature = "atlas"), feature = "render"))] array_texture_loader: Res<
        ArrayTextureLoader,
    >,
) {
    info!("World config generator started");
    shared_world_generator(
        world_config.seed as u32,
        world_config.world_size,
        commands,
        asset_server,
    );
}

fn handle_interpolated_spawn(
    trigger: On<Add, Interpolated>,
    mut commands: Commands,
    player_resources: Res<PlayerSpriteSheetResource>,
) {
    let entity = trigger.entity;
    // info!("Adding InputMarker to entity {:?}", entity);

    commands.entity(entity).insert((
        Sprite::from_atlas_image(
            player_resources.player_image.clone(),
            TextureAtlas {
                layout: player_resources.atlas.clone(),
                index: 0,
            },
        ),
        // Transform::from_scale(Vec3::splat(6.0)),
        PlayerAnimationTimer::new(2),
        // FrameInterpolate::<Transform>::default(),
    ));
}

// fn local_player_movement(
//     timeline: Res<LocalTimeline>,
//     mut position_query: Query<
//         (
//             &mut Position,
//             &mut Rotation,
//             &ActionState<Inputs>,
//             &PlayerId,
//         ),
//         (With<Predicted>, With<PlayerMarker>),
//     >,
// ) {
//     let tick = timeline.tick();
//     for (position, rotation, input, player_id) in position_query.iter_mut() {
//         shared_movement_behaviour(position, rotation, input);
//     }
// }

// fn local_player_animation(
//     // timeline: Res<LocalTimeline>,
//     mut player_query: Query<
//         (
//             &mut PlayerState,
//             // &mut PlayerAnimations,
//             &ActionState<Inputs>,
//         ),
//         With<Predicted>,
//     >,
// ) {
//     // let tick = timeline.tick();
//     for (state, inputs) in player_query.iter_mut() {
//         // trace!(?tick, ?state, ?anims, ?inputs, "server");
//         shared_animation_behaviour(state, inputs);
//     }
// }
