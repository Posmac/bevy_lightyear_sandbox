use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::str::FromStr;

use crate::protocol::*;
use crate::shared::constants::LOCAL_ADDR;
use crate::shared::constants::PlayerAnimationTimer;
use crate::shared::constants::PlayerSpriteSheetResource;
use crate::shared::constants::SERVER_ADDR;
// use crate::shared::constants::SERVER_IP;
use crate::shared::constants::SERVER_PORT;
use crate::shared::constants::SHARED_SETTINGS;
use crate::shared::world_generator::shared_world_generator;
use aeronet_websocket::client::ClientConfig;
use avian2d::prelude::LinearVelocity;
use avian2d::prelude::Position;
use avian2d::prelude::Rotation;
use bevy::color::palettes::css::GREEN;
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
        app.add_systems(PostUpdate, camera_follow);
        // app.add_systems(Update, debug_bullets_system);
        app.add_observer(handle_predicted_spawn);
        // app.add_observer(handle_interpolated_spawn);
        app.add_observer(handle_world_config_spawn);
    }
}

pub fn debug_bullets_system(
    query: Query<(Entity, &Position, &LinearVelocity, &Transform), With<BulletMarker>>,
    mut gizmos: Gizmos,
) {
    for (entity, pos, vel, transform) in query.iter() {
        // 1. Рисуем вектор скорости (зеленый)
        gizmos.ray_2d(pos.0, vel.0 * 0.1, GREEN);

        // 2. Рисуем маленькую сферу в месте нахождения Трансформа (белая)
        // Если белая сфера и позиция Avian расходятся — у нас проблема синхронизации
        gizmos.circle_2d(transform.translation.truncate(), 2.0, Color::WHITE);

        // 3. Вывод в лог (аккуратно, чтобы не засрать консоль)
        // Мы будем видеть, меняются ли цифры у "чужих" пуль
        println!(
            "Bullet {:?} | Pos: {:?} | Vel: {:?} | Speed: {}",
            entity,
            pos.0,
            vel.0,
            vel.0.length()
        );
    }
}

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
    let random_id: u64 = rand::random();
    let auth = Authentication::Manual {
        client_id: random_id,
        server_addr: SERVER_ADDR,
        protocol_id: SHARED_SETTINGS.protocol_id,
        private_key: SHARED_SETTINGS.private_key,
    };

    // let server_url = "ws://127.0.0.1:5888";
    let server_url = "wss://vktjh6ln63sz.share.zrok.io";

    let config = {
        #[cfg(target_family = "wasm")]
        {
            ClientConfig
        }
        #[cfg(not(target_family = "wasm"))]
        {
            ClientConfig::builder().with_no_cert_validation()
        }
    };

    let client = commands
        .spawn((
            Client::default(),
            NetcodeClient::new(auth, NetcodeConfig::default()).unwrap(),
            // UdpIo::default(),
            WebSocketClientIo {
                config,
                target: WebSocketTarget::Url(server_url.to_string()),
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

fn handle_predicted_spawn(
    trigger: On<Add, (PlayerId, Predicted)>,
    mut commands: Commands,
    // player_resources: Res<PlayerSpriteSheetResource>,
    mut query: Query<&PlayerMarker, With<Predicted>>,
) {
    let entity = trigger.entity;
    if let Ok(marker) = query.get_mut(trigger.entity) {
        info!("Adding InputMarker to entity {:?} {:?}", entity, marker);

        let mut input_map = InputMap::default();
        input_map.insert_multiple([
            (Inputs::Up, KeyCode::KeyW),
            (Inputs::Up, KeyCode::ArrowUp),
            (Inputs::Down, KeyCode::KeyS),
            (Inputs::Down, KeyCode::ArrowDown),
            (Inputs::Left, KeyCode::KeyA),
            (Inputs::Left, KeyCode::ArrowLeft),
            (Inputs::Right, KeyCode::KeyD),
            (Inputs::Right, KeyCode::ArrowRight),
            (Inputs::Shoot, KeyCode::Space),
        ]);
        input_map.insert(Inputs::Shoot, MouseButton::Left);

        commands.entity(entity).insert((
            input_map,
            FrameInterpolate::<Position>::default(),
            FrameInterpolate::<Rotation>::default(),
            Transform::default(),
            GlobalTransform::default(),
            InheritedVisibility::default(),
            PlayerPhysicsBundle::player(),
        ));

        commands.entity(entity).with_children(|parent| {
            // parent.spawn((
            //     Sprite::from_atlas_image(
            //         player_resources.player_image.clone(),
            //         TextureAtlas {
            //             layout: player_resources.atlas.clone(),
            //             index: 0,
            //         },
            //     ),
            //     Transform::from_scale(Vec3::splat(6.0)),
            //     GlobalTransform::default(),
            //     InheritedVisibility::default(),
            //     PlayerAnimationTimer::new(2),
            // ));
        });
    }
}

fn handle_interpolated_spawn(
    trigger: On<Add, PlayerMarker>,
    mut commands: Commands,
    player_resources: Res<PlayerSpriteSheetResource>,
    player_query: Query<(), Added<Interpolated>>,
) {
    let entity = trigger.entity;

    if let Ok(v) = player_query.get(entity) {
        info!("Spawned interpolated player");
        commands.entity(entity).insert((
            Transform::default(),
            GlobalTransform::default(),
            InheritedVisibility::default(),
        ));

        commands.entity(entity).with_children(|parent| {
            parent.spawn((
                Sprite::from_atlas_image(
                    player_resources.player_image.clone(),
                    TextureAtlas {
                        layout: player_resources.atlas.clone(),
                        index: 0,
                    },
                ),
                Transform::from_scale(Vec3::splat(6.0)),
                GlobalTransform::default(),
                InheritedVisibility::default(),
                PlayerAnimationTimer::new(2),
            ));
        });
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
