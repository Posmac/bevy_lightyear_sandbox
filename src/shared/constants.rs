use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::Duration,
};

use avian2d::{
    PhysicsPlugins,
    math::FRAC_PI_2,
    prelude::{
        Collider, ColliderDensity, CollisionLayers, Gravity, IslandPlugin, IslandSleepingPlugin,
        LinearVelocity, PhysicsInterpolationPlugin, PhysicsLayer, PhysicsTransformPlugin, Position,
        RigidBody, Rotation,
    },
};
use bevy::prelude::*;
use leafwing_input_manager::prelude::ActionState;
use lightyear::{
    avian2d::plugin::{AvianReplicationMode, LightyearAvianPlugin},
    prelude::{
        ControlledBy, InterpolationTarget, LocalTimeline, NetworkTarget, PreSpawned, Predicted,
        PredictionTarget, Replicate,
    },
};
use lightyear_avian2d::prelude::LagCompensationHistory;

use crate::protocol::{
    AnimationConfig, BulletMarker, HitboxMarker, Inputs, PlayerAnimations, PlayerId, PlayerMarker,
    PlayerState, PlayerStateEnum, ProtocolPlugin, StaticPhysicsBundle,
};

// pub const SERVER_IP: &str = "droplets.it.com";
pub const FIXED_TIMESTEP_HZ: f64 = 64.0;
pub const SERVER_PORT: u16 = 5888;
/// 0 means that the OS will assign any available port
pub const CLIENT_PORT: u16 = 0;
pub const SERVER_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), SERVER_PORT);
pub const LOCAL_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), CLIENT_PORT);
pub const SEND_INTERVAL: Duration = Duration::from_millis(50);
pub const SHARED_SETTINGS: SharedSettings = SharedSettings {
    protocol_id: 1997,
    private_key: [0; 32],
};

//physics common
pub const EPS: f64 = 0.0001;
pub const BULLET_MOVE_SPEED: f32 = 300.0;
pub const MAP_LIMIT: f32 = 2000.0;
pub const BULLET_SIZE: f32 = 3.0;
pub const PLAYER_SIZE: f32 = 80.0;
pub const BULLET_COLLISION_DISTANCE_CHECK: f32 = 4.0;
pub const BOT_RADIUS: f32 = 15.0;
pub const ITEM_RADIUS: f32 = 10.0;
pub const WALL_SIZE: f32 = 100.0;
pub const ITEM_PICKUP_BOX_RADIUS: f32 = 80.0;

//health
pub const BOT_MAX_HEALTH: u16 = 255;
pub const PLAYER_MAX_HEALTH: u16 = 100;
pub const BULLET_BASE_DAMAGE: u16 = 22;
pub const WALL_MAX_HEALTH: u16 = 1000;
pub const HEALTH_BAR_SIZE: Vec2 = Vec2::new(100.0, 10.0);

#[derive(Copy, Clone, Debug)]
pub struct SharedSettings {
    /// An id to identify the protocol version
    pub protocol_id: u64,

    /// a 32-byte array to authenticate via the Netcode.io protocol
    pub private_key: [u8; 32],
}

pub struct SharedPlugin;

impl Plugin for SharedPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ProtocolPlugin);

        app.add_plugins(LightyearAvianPlugin {
            replication_mode: AvianReplicationMode::Position,
            rollback_islands: false,
            ..Default::default()
        });

        app.add_systems(PreUpdate, despawn_after);
        app.add_systems(Startup, init_walls);
        //debug systems

        app.add_systems(
            FixedUpdate,
            (
                player_movement,
                // debug_player_hierarchy,
                player_animation,
                shoot_bullet,
            )
                .chain(),
        );

        app.add_plugins(
            PhysicsPlugins::default()
                .build()
                .disable::<IslandPlugin>()
                .disable::<IslandSleepingPlugin>()
                .disable::<PhysicsTransformPlugin>()
                .disable::<PhysicsInterpolationPlugin>(),
        )
        .insert_resource(Gravity(Vec2::ZERO));

        // app.add_systems(Startup, load_resources);
    }
}

#[derive(PhysicsLayer, Default)]
pub enum GamePhysicsLayer {
    //none
    #[default]
    None,
    //player rigidboddy collider
    PlayerRigidBody,
    //player hitbox collider
    PlayerHitbox,
    //player projectile
    PlayerProjectile,
    //World object (other rigid bodies, non action)
    WorldStatic,
    //Bot collider
    Bot,
    //Items
    Item,
    //Itembox
    ItemPickUpBox,
}

fn debug_player_hierarchy(
    q_parent: Query<
        (Entity, &Position, &Transform, &GlobalTransform, &Children),
        With<PlayerMarker>,
    >,
    q_child: Query<(Entity, Option<&Position>, &Transform, &GlobalTransform), With<HitboxMarker>>,
) {
    for (p_entity, p_pos, p_trans, p_global, children) in q_parent.iter() {
        info!("=== PLAYER (Parent) ===");
        info!("Entity: {:?}", p_entity);
        info!("Avian Position: {:?}", p_pos.0);
        info!("Bevy Transform: {:?}", p_trans.translation);
        info!("World GlobalTransform: {:?}", p_global.translation());

        for child_entity in children.iter() {
            if let Ok((c_entity, c_pos, c_trans, c_global)) = q_child.get(child_entity) {
                info!("--- HITBOX (Child) ---");
                info!("Entity: {:?}", c_entity);
                info!("Has Position Component: {}", c_pos.is_some());
                if let Some(pos) = c_pos {
                    info!("Child Avian Position: {:?}", pos.0);
                }
                info!("Local Transform: {:?}", c_trans.translation);
                info!("World GlobalTransform: {:?}", c_global.translation());
            }
        }
    }
}

#[derive(Debug, Resource, Clone, PartialEq, Reflect)]
pub struct PlayerSpriteSheetResource {
    pub player_image: Handle<Image>,
    pub atlas: Handle<TextureAtlasLayout>,
}

#[derive(Debug, Component, Clone, PartialEq, Reflect)]
pub struct PlayerAnimationTimer {
    pub fps: u8,
    pub frame_timer: Timer,
}

impl PlayerAnimationTimer {
    pub fn new(fps: u8) -> Self {
        Self {
            fps,
            frame_timer: Timer::new(
                Duration::from_secs_f32(1.0 / (fps as f32)),
                TimerMode::Repeating,
            ),
        }
    }
}

pub fn player_movement(
    timeline: Res<LocalTimeline>,
    mut player_query: Query<
        (
            &mut Position,
            &mut Rotation,
            &mut LinearVelocity,
            &ActionState<Inputs>,
            &PlayerId,
        ),
        (Or<(With<Predicted>, With<Replicate>)>, With<PlayerMarker>),
    >,
) {
    for (position, rotation, vel, action_state, player_id) in player_query.iter_mut() {
        debug!(tick = ?timeline.tick(), action = ?action_state.dual_axis_data(&Inputs::Mouse), "Data in Movement (FixedUpdate)");
        shared_movement_behaviour(position, rotation, vel, action_state);
    }
}

fn player_animation(
    timeline: Res<LocalTimeline>,
    mut player_query: Query<(
        &mut PlayerState,
        // &mut PlayerAnimations,
        &ActionState<Inputs>,
    )>,
) {
    let tick = timeline.tick();
    for (state, inputs) in player_query.iter_mut() {
        trace!(?tick, ?state, ?inputs, "server");
        shared_animation_behaviour(state, inputs);
    }
}

// This system defines how we update the player's positions when we receive an input
pub fn shared_movement_behaviour(
    mut position: Mut<Position>,
    mut rotation: Mut<Rotation>,
    mut velocity: Mut<LinearVelocity>,
    action: &ActionState<Inputs>,
) {
    const MOVE_SPEED: f32 = 350.0;

    // if let Some(cursor_data) = action.dual_axis_data(&Inputs::Mouse) {
    // } else {
    // }
    const MAX_VELOCITY: f32 = 200.0;
    // *velocity = LinearVelocity(Vec2::ZERO);

    if action.pressed(&Inputs::Up) {
        // position.y += MOVE_SPEED;
        velocity.y += MOVE_SPEED;
    }
    if action.pressed(&Inputs::Down) {
        // position.y -= MOVE_SPEED;
        velocity.y -= MOVE_SPEED;
    }
    if action.pressed(&Inputs::Left) {
        // position.x -= MOVE_SPEED;
        velocity.x -= MOVE_SPEED;
    }
    if action.pressed(&Inputs::Right) {
        // position.x += MOVE_SPEED;
        velocity.x += MOVE_SPEED;
    }
    *velocity = LinearVelocity(velocity.clamp_length_max(MAX_VELOCITY));
}

pub fn shared_animation_behaviour(
    mut player_state: Mut<PlayerState>,
    // mut player_animations: Mut<PlayerAnimations>,
    action: &ActionState<Inputs>,
) {
    let mut is_none = true;
    if action.pressed(&Inputs::Up)
        || action.pressed(&Inputs::Down)
        || action.pressed(&Inputs::Left)
        || action.pressed(&Inputs::Right)
    {
        is_none = false;
    }

    player_state.prev_state = player_state.current_state.clone();
    if is_none {
        if player_state.current_state.is_walking() {
            let inverse_state = player_state.current_state.get_opposite_state();
            // let inverse_animation = player_animations.get_anim(&inverse_state);
            player_state.current_state = inverse_state;
            // player_animations.current_animation = inverse_animation;
        }
        return;
    }

    if action.pressed(&Inputs::Up) {
        player_state.current_state = PlayerStateEnum::WalkingBack;
        // player_animations.current_animation = player_animations.move_front;
    }
    if action.pressed(&Inputs::Down) {
        player_state.current_state = PlayerStateEnum::WalkingFront;
        // player_animations.current_animation = player_animations.move_back;
    }
    if action.pressed(&Inputs::Left) {
        player_state.current_state = PlayerStateEnum::WalkingLeft;
        // player_animations.current_animation = player_animations.move_left;
    }
    if action.pressed(&Inputs::Right) {
        player_state.current_state = PlayerStateEnum::WalkingRight;
        // player_animations.current_animation = player_animations.move_right;
    }
}

pub fn shoot_bullet(
    timeline: Res<LocalTimeline>,
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
            &PlayerId,
            &Position,
            &mut ActionState<Inputs>,
            Option<&ControlledBy>,
        ),
        (Or<(With<Predicted>, With<Replicate>)>, With<PlayerMarker>),
    >,
) {
    let tick = timeline.tick();
    for (player_entity, player_id, position, action, controlled_by) in query.iter_mut() {
        let is_server = controlled_by.is_some();
        // NOTE: pressed lets you shoot many bullets, which can be cool
        let cursor_pos = action.axis_pair(&Inputs::Mouse);
        let player_pos = position.0;
        let direction = (cursor_pos - player_pos).normalize_or_zero();

        let angle = direction.y.atan2(direction.x);
        if action.just_pressed(&Inputs::Shoot) {
            // error!(?tick, pos=?player_pos, rot=?angle, "spawn bullet");
            // error!(?tick, pos=?transform.translation.truncate(), rot=?transform.rotation.to_euler(EulerRot::XYZ).2, "spawn bullet");
            // for delta in [-0.2, 0.2] {
            for delta in [0.0] {
                let salt: u64 = if delta < 0.0 { 0 } else { 1 };
                let mut bullet_transform = Transform::from_translation(player_pos.extend(0.1));
                bullet_transform.rotation = Quat::from_rotation_z(angle + delta - FRAC_PI_2);
                let bullet_bundle = (
                    bullet_transform,
                    LinearVelocity(bullet_transform.up().as_vec3().truncate() * BULLET_MOVE_SPEED),
                    RigidBody::Kinematic,
                    BulletMarker {
                        player_entity: player_entity,
                    },
                    // CollisionLayers::new(
                    //     GamePhysicsLayer::PlayerProjectile,
                    // [GamePhysicsLayer::WorldStatic, GamePhysicsLayer::Bot],
                    // ),
                    Name::new("Bullet"),
                );

                // on the server, replicate the bullet
                if is_server {
                    // #[cfg(feature = "server")]
                    commands.spawn((
                        bullet_bundle,
                        // NOTE: the PreSpawned component indicates that the entity will be spawned on both client and server
                        //  but the server will take authority as soon as the client receives the entity
                        //  it does this by matching with the client entity that has the same hash
                        //  The hash is computed automatically in PostUpdate from the entity's components + spawn tick
                        //  unless you set the hash manually before PostUpdate to a value of your choice
                        //
                        // the default hashing algorithm uses the tick and component list. in order to disambiguate
                        // between the two bullets, we add additional information to the hash.
                        // NOTE: if you don't add the salt, the 'left' bullet on the server might get matched with the
                        // 'right' bullet on the client, and vice versa. This is not critical, but it will cause a rollback
                        PreSpawned::default_with_salt(salt),
                        DespawnAfter(Timer::new(Duration::from_secs(5), TimerMode::Once)),
                        Replicate::to_clients(NetworkTarget::All),
                        PredictionTarget::to_clients(NetworkTarget::Single(player_id.0)),
                        InterpolationTarget::to_clients(NetworkTarget::AllExceptSingle(
                            player_id.0,
                        )),
                        controlled_by.unwrap().clone(),
                    ));
                } else {
                    // on the client, just spawn the ball
                    // NOTE: the PreSpawned component indicates that the entity will be spawned on both client and server
                    //  but the server will take authority as soon as the client receives the entity
                    commands.spawn((bullet_bundle, PreSpawned::default_with_salt(salt)));
                }
            }
        }
    }
}

#[derive(Component)]
struct DespawnAfter(pub Timer);

/// Despawn entities after their timer has finished
fn despawn_after(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut DespawnAfter)>,
) {
    for (entity, mut despawn_after) in query.iter_mut() {
        despawn_after.0.tick(time.delta());
        if despawn_after.0.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

pub fn load_resources(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let character_texture = asset_server.load("sprout/Characters/basic_movement.png");
    let sprite_sheet_layout = TextureAtlasLayout::from_grid(UVec2::new(48, 48), 4, 4, None, None);
    let texture_atlas_layout = texture_atlas_layouts.add(sprite_sheet_layout);

    commands.insert_resource(PlayerSpriteSheetResource {
        player_image: character_texture,
        atlas: texture_atlas_layout,
    });
}

pub fn get_player_anim_config() -> PlayerAnimations {
    let character_animation_config = PlayerAnimations::new(
        AnimationConfig {
            first_sprite_index: 0,
            last_sprite_index: 1,
        },
        AnimationConfig {
            first_sprite_index: 4,
            last_sprite_index: 5,
        },
        AnimationConfig {
            first_sprite_index: 8,
            last_sprite_index: 9,
        },
        AnimationConfig {
            first_sprite_index: 12,
            last_sprite_index: 13,
        },
        AnimationConfig {
            first_sprite_index: 2,
            last_sprite_index: 3,
        },
        AnimationConfig {
            first_sprite_index: 6,
            last_sprite_index: 7,
        },
        AnimationConfig {
            first_sprite_index: 10,
            last_sprite_index: 11,
        },
        AnimationConfig {
            first_sprite_index: 14,
            last_sprite_index: 15,
        },
    );

    character_animation_config
}

//walls
#[derive(Bundle)]
pub struct WallBundle {
    physics: StaticPhysicsBundle,
    wall: Wall,
    transform: Transform,
    lag_compensation: LagCompensationHistory,
}

#[derive(Component)]
pub struct Wall {
    pub position: Vec2,
    pub size: Vec2,
}

impl WallBundle {
    pub fn new(center_pos: Vec2, size: Vec2) -> Self {
        Self {
            physics: StaticPhysicsBundle {
                collider: Collider::rectangle(size.x, size.y),
                collider_density: ColliderDensity(1.0),
                rigid_body: RigidBody::Static,
                layers: CollisionLayers::new(
                    GamePhysicsLayer::WorldStatic,
                    GamePhysicsLayer::PlayerRigidBody,
                ),
            },
            wall: Wall {
                position: center_pos,
                size: size,
            },
            transform: Transform::from_xyz(center_pos.x, center_pos.y, 0.0),
            lag_compensation: LagCompensationHistory::default(),
        }
        // let mut app = App::new();
        // app.add_plugins(DefaultPlugins.
        //     set(ImagePlugin::default_nearest()).
        //     set(AssetPlugin {
        //         meta_check: bevy::asset::AssetMetaCheck::Never,
        //         ..default()
        // }).set(LogPlugin {
        //         level: Level::INFO,
        //         filter: "wgpu=error,bevy_render=info,bevy_ecs=warn,bevy_time=warn,naga=warn,bevy_enhanced_input::action::fns=error".to_string(),
        //         ..default()
        //     }
        // ).set(
        //     WindowPlugin {
        //         primary_window: Some(Window {
        //             title: format!("Multiplayer: {}", env!("CARGO_PKG_NAME")),
        //             resolution: (1024, 768).into(),
        //             present_mode: PresentMode::AutoVsync,
        //             // set to true if we want to capture tab etc in wasm
        //             prevent_default_event_handling: true,
        //             ..Default::default()
        //         }),
        //         ..default()
        //     }
        // ));

        // app.insert_resource(WinitSettings::continuous());
        // // #[cfg(feature = "debug")]
        // // app.add_plugins(DebugUIPlugin);

        // app.run();

        // #[cfg(feature = "client")]
        // app.add_plugins(lightyear::prelude::client::ClientPlugins { tick_duration });

        // #[cfg(feature = "server")]
        // app.add_plugins(lightyear::prelude::server::ServerPlugins { tick_duration });

        // #[cfg(any(feature = "gui2d", feature = "gui3d"))]
        // app.add_plugins(RenderPlugin:)
        // ExampleClientRendererPlugin::new(format!("Client {client_id:?}")),
        // app.add_plugins(DefaultPlugins.set(Image)

        // App::new()
        //     .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        //     .add_systems(Startup, (setup_camera, setup_character).chain())
        //     .add_systems(
        //         Update,
        //         (
        //             accumulate_input,
        //             move_character,
        //             process_character_animation,
        //             clear_input,
        //         )
        //             .chain(),
        //     )
        //     .run();
        //
    }
}

pub fn init_walls(mut commands: Commands) {
    info!("Walls spawned");
    commands.spawn(WallBundle::new(
        Vec2 {
            x: WALL_SIZE * 2.0,
            y: WALL_SIZE * 2.0,
        },
        Vec2 {
            x: WALL_SIZE,
            y: WALL_SIZE,
        },
    ));
    commands.spawn(WallBundle::new(
        Vec2 {
            x: -WALL_SIZE * 2.0,
            y: -WALL_SIZE * 2.0,
        },
        Vec2 {
            x: WALL_SIZE,
            y: WALL_SIZE,
        },
    ));
}
