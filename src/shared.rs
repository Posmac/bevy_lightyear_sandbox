use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::LazyLock,
    time::Duration,
};

use aeronet_websocket::rustls::quic::DirectionalKeys;
use avian2d::{
    PhysicsPlugins,
    math::FRAC_PI_2,
    prelude::{Gravity, LinearVelocity, PhysicsTransformPlugin, Position, RigidBody, Rotation},
};
use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use leafwing_input_manager::prelude::ActionState;
use lightyear::{
    avian2d::plugin::{AvianReplicationMode, LightyearAvianPlugin},
    prelude::{
        ControlledBy, Interpolated, InterpolationTarget, LocalTimeline, NetworkTarget, PreSpawned,
        Predicted, PredictionHistory, PredictionTarget, Replicate, Replicated,
    },
};
use noise::{
    Fbm, Perlin,
    utils::{NoiseMap, NoiseMapBuilder, PlaneMapBuilder},
};

use crate::protocol::{
    AnimationConfig, BulletMarker, Inputs, PlayerAnimations, PlayerId, PlayerMarker, PlayerState,
    PlayerStateEnum, ProtocolPlugin,
};

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
pub const PLAYER_SIZE: f32 = 40.0;
pub const BULLET_COLLISION_DISTANCE_CHECK: f32 = 4.0;

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
            replication_mode: AvianReplicationMode::PositionButInterpolateTransform,
            ..Default::default()
        });

        app.add_systems(PreUpdate, despawn_after);
        //debug systems
        app.add_systems(FixedLast, fixed_update_log);

        app.add_systems(
            FixedUpdate,
            (player_movement, player_animation, shoot_bullet).chain(),
        );

        app.add_plugins(
            PhysicsPlugins::default()
                .build()
                .disable::<PhysicsTransformPlugin>(),
        )
        .insert_resource(Gravity(Vec2::ZERO));

        app.add_systems(Startup, load_resources);
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
            &ActionState<Inputs>,
            &PlayerId,
        ),
        (Or<(With<Predicted>, With<Replicate>)>, With<PlayerMarker>),
    >,
) {
    for (position, rotation, action_state, player_id) in player_query.iter_mut() {
        debug!(tick = ?timeline.tick(), action = ?action_state.dual_axis_data(&Inputs::Mouse), "Data in Movement (FixedUpdate)");
        shared_movement_behaviour(position, rotation, action_state);
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
    action: &ActionState<Inputs>,
) {
    const MOVE_SPEED: f32 = 10.0;

    // if let Some(cursor_data) = action.dual_axis_data(&Inputs::Mouse) {
    // } else {
    // }

    if action.pressed(&Inputs::Up) {
        position.y += MOVE_SPEED;
    }
    if action.pressed(&Inputs::Down) {
        position.y -= MOVE_SPEED;
    }
    if action.pressed(&Inputs::Left) {
        position.x -= MOVE_SPEED;
    }
    if action.pressed(&Inputs::Right) {
        position.x += MOVE_SPEED;
    }
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
            &PlayerId,
            &Transform,
            &mut ActionState<Inputs>,
            Option<&ControlledBy>,
        ),
        (Or<(With<Predicted>, With<Replicate>)>, With<PlayerMarker>),
    >,
) {
    let tick = timeline.tick();
    for (id, transform, action, controlled_by) in query.iter_mut() {
        let is_server = controlled_by.is_some();
        // NOTE: pressed lets you shoot many bullets, which can be cool
        let cursor_pos = action.axis_pair(&Inputs::Mouse);
        let player_pos = transform.translation.truncate();
        let direction = (cursor_pos - player_pos).normalize_or_zero();

        let angle = direction.y.atan2(direction.x);
        if action.just_pressed(&Inputs::Shoot) {
            error!(?tick, pos=?transform.translation.truncate(), rot=?transform.rotation.to_euler(EulerRot::XYZ).2, "spawn bullet");
            // for delta in [-0.2, 0.2] {
            for delta in [0.0] {
                let salt: u64 = if delta < 0.0 { 0 } else { 1 };
                let mut bullet_transform = Transform::from_translation(player_pos.extend(0.1));
                bullet_transform.rotation = Quat::from_rotation_z(angle + delta - FRAC_PI_2);
                let bullet_bundle = (
                    bullet_transform,
                    LinearVelocity(bullet_transform.up().as_vec3().truncate() * BULLET_MOVE_SPEED),
                    RigidBody::Kinematic,
                    // store the player who fired the bullet
                    *id,
                    // *color,
                    BulletMarker,
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
                        DespawnAfter(Timer::new(Duration::from_secs(2), TimerMode::Once)),
                        Replicate::to_clients(NetworkTarget::All),
                        PredictionTarget::to_clients(NetworkTarget::Single(id.0)),
                        InterpolationTarget::to_clients(NetworkTarget::AllExceptSingle(id.0)),
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

pub fn shared_world_generator(
    seed: u32,
    world_size: u64,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    #[cfg(all(not(feature = "atlas"), feature = "render"))] array_texture_loader: Res<
        ArrayTextureLoader,
    >,
) {
    #[cfg(feature = "world_generator")]
    {
        //generate world from noise
        // https://www.boristhebrave.com/2013/07/14/tileset-roundup/?q=tutorials/tileset-roundup
        // https://www.boristhebrave.com/permanent/24/06/cr31/stagecast/wang/blob.html
        let noise_map: NoiseMap = generate_world_noise(seed, world_size);
        let terrain_matrix = generate_terrain_matrix(&noise_map, world_size);
        // log_terrain_matrix(&data);
        let tileset_mask: Vec<u8> = generate_terrain_mask(&terrain_matrix, world_size);
        // log_mask_matrix(&tileset_mask, world_size);
        // log_dense_masks(&tileset_mask, world_size as usize);

        fill_tilemap_render(
            world_size,
            commands,
            asset_server,
            &terrain_matrix,
            &tileset_mask,
        );
    }
}

pub fn generate_world_noise(seed: u32, world_size: u64) -> NoiseMap {
    let fbm = Fbm::<Perlin>::new(seed as u32);
    let builder = PlaneMapBuilder::new(fbm)
        .set_size(world_size as usize, world_size as usize)
        .set_x_bounds(-1.0, 1.0)
        .set_y_bounds(-1.0, 1.0)
        .build();
    builder
}

pub fn generate_terrain_matrix(noise_map: &NoiseMap, world_size: u64) -> Vec<Vec<bool>> {
    let mut data = vec![vec![false; world_size as usize]; world_size as usize];

    for (index, noise) in noise_map.iter().enumerate() {
        let row_index = index / world_size as usize;
        let column_index = index % world_size as usize;

        if *noise > 0.0 {
            data[row_index][column_index] = true;
        }
    }
    data
}

pub fn generate_terrain_mask(terrain_matrix: &Vec<Vec<bool>>, world_size: u64) -> Vec<u8> {
    let mut data = vec![0u8; (world_size * world_size) as usize];

    for (i, d) in data.iter_mut().enumerate() {
        let r = i as i32 / world_size as i32;
        let c = i as i32 % world_size as i32;

        //collect near tiles data
        //r-1, c-1 or 0
        //r-1, c or 0
        //r-1, c+1 or 0
        //r, c-1 or 0
        //skip
        //r, c+1 or 0
        //r+1, c-1 or 0
        //r+1, c o r0
        //r+1, c+1 or 0
        let n = get_value_safe(terrain_matrix, r + 1, c); // North (Up)
        let s = get_value_safe(terrain_matrix, r - 1, c); // South (Down)
        let w = get_value_safe(terrain_matrix, r, c - 1); // West (Left)
        let e = get_value_safe(terrain_matrix, r, c + 1); // East (Right)

        let nw = get_value_safe(terrain_matrix, r + 1, c - 1);
        let ne = get_value_safe(terrain_matrix, r + 1, c + 1);
        let sw = get_value_safe(terrain_matrix, r - 1, c - 1);
        let se = get_value_safe(terrain_matrix, r - 1, c + 1);

        let ne_bit = if ne && n && e { 2 } else { 0 };
        let se_bit = if se && s && e { 8 } else { 0 };
        let sw_bit = if sw && s && w { 32 } else { 0 };
        let nw_bit = if nw && n && w { 128 } else { 0 };

        let mask = (if n { 1 } else { 0 })
            + ne_bit
            + (if e { 4 } else { 0 })
            + se_bit
            + (if s { 16 } else { 0 })
            + sw_bit
            + (if w { 64 } else { 0 })
            + nw_bit;

        *d = mask;
    }

    data
}

fn get_value_safe(data: &Vec<Vec<bool>>, x: i32, y: i32) -> bool {
    if x < 0 || y < 0 {
        return false;
    }

    // match
    data.get(y as usize) // Option<&Vec<bool>>
        .and_then(|row| row.get(x as usize)) // Option<&bool>
        .copied() // Option<bool>
        .unwrap_or(false)
    // {
    //     true => 1,
    //     false => 0,
    // }
}

pub fn log_terrain_matrix(data: &Vec<Vec<bool>>) {
    let mut output = String::new();
    output.push_str("\n--- World Map ---\n");

    for row in data {
        for &is_land in row {
            let symbol = if is_land { "██" } else { "  " };
            output.push_str(symbol);
        }
        output.push('\n');
    }

    output.push_str("------------------\n");
    info!("{}", output);
}

pub fn log_mask_matrix(masks: &Vec<u8>, world_size: u64) {
    let size = world_size as usize;
    let mut output = String::new();
    output.push_str("\n--- Tileset Masks (Hex/Dec) ---\n");

    for y in 0..size {
        for x in 0..size {
            let index = y * size + x;
            let mask = masks[index];

            // Выводим в шестнадцатеричном виде (0-F) или десятичном с пробелом
            // {:2} гарантирует, что каждое число займет 2 символа
            output.push_str(&format!("{:2} ", mask));
        }
        output.push('\n');
    }

    output.push_str("-------------------------------\n");
    println!("{}", output); // Или info!("{}", output) для Bevy
}

pub fn log_dense_masks(masks: &[u8], world_size: usize) {
    println!("\n--- 🗺️  Visual World Map (8-bit masks) ---");

    for row in masks.chunks(world_size) {
        let mut line = String::new();
        for &m in row {
            let symbol = match m {
                255 => "██",                   // Полностью заполнен (центр)
                127 | 253 | 223 | 191 => "▓▓", // Почти заполнен
                m if m > 100 => "▒▒",          // Средняя связность
                m if m > 0 => "░░",            // Края и углы
                0 => "  ",                     // Пустота
                _ => "..",
            };
            line.push_str(symbol);
        }
        println!("{}", line);
    }
    println!("--- End of Map ---\n");
}

pub struct TileInfo {
    pub x: u32, // Колонка в тайлсете (0-6)
    pub y: u32, // Строка в тайлсете (0-6)
}

static TILE_MASKS: LazyLock<HashMap<u8, TileInfo>> = LazyLock::new(|| {
    let mut m = HashMap::new();

    // Формат: m.insert(индекс, TileInfo { x, y });
    // Строка 0 water
    m.insert(0, TileInfo { x: 0, y: 0 });
    m.insert(4, TileInfo { x: 1, y: 0 });
    m.insert(92, TileInfo { x: 2, y: 0 });
    m.insert(124, TileInfo { x: 3, y: 0 });
    m.insert(116, TileInfo { x: 4, y: 0 });
    m.insert(80, TileInfo { x: 5, y: 0 });

    // Строка 1
    m.insert(16, TileInfo { x: 0, y: 1 });
    m.insert(20, TileInfo { x: 1, y: 1 });
    m.insert(87, TileInfo { x: 2, y: 1 });
    m.insert(223, TileInfo { x: 3, y: 1 });
    m.insert(241, TileInfo { x: 4, y: 1 });
    m.insert(21, TileInfo { x: 5, y: 1 });
    m.insert(64, TileInfo { x: 6, y: 1 });

    // Строка 2
    m.insert(29, TileInfo { x: 0, y: 2 });
    m.insert(117, TileInfo { x: 1, y: 2 });
    m.insert(85, TileInfo { x: 2, y: 2 });
    m.insert(71, TileInfo { x: 3, y: 2 });
    m.insert(221, TileInfo { x: 4, y: 2 });
    m.insert(125, TileInfo { x: 5, y: 2 });
    m.insert(112, TileInfo { x: 6, y: 2 });

    // Строка 3
    m.insert(31, TileInfo { x: 0, y: 3 });
    m.insert(253, TileInfo { x: 1, y: 3 });
    m.insert(113, TileInfo { x: 2, y: 3 });
    m.insert(28, TileInfo { x: 3, y: 3 });
    m.insert(127, TileInfo { x: 4, y: 3 });
    m.insert(247, TileInfo { x: 5, y: 3 });
    m.insert(209, TileInfo { x: 6, y: 3 });

    // Строка 4
    m.insert(23, TileInfo { x: 0, y: 4 });
    m.insert(199, TileInfo { x: 1, y: 4 });
    m.insert(213, TileInfo { x: 2, y: 4 });
    m.insert(95, TileInfo { x: 3, y: 4 });
    m.insert(255, TileInfo { x: 4, y: 4 });
    m.insert(245, TileInfo { x: 5, y: 4 });
    m.insert(81, TileInfo { x: 6, y: 4 });

    // Строка 5
    m.insert(5, TileInfo { x: 0, y: 5 });
    m.insert(84, TileInfo { x: 1, y: 5 });
    m.insert(93, TileInfo { x: 2, y: 5 });
    m.insert(119, TileInfo { x: 3, y: 5 });
    m.insert(215, TileInfo { x: 4, y: 5 });
    m.insert(193, TileInfo { x: 5, y: 5 });
    m.insert(17, TileInfo { x: 6, y: 5 });

    // Строка 6
    m.insert(1, TileInfo { x: 1, y: 6 });
    m.insert(7, TileInfo { x: 2, y: 6 });
    m.insert(197, TileInfo { x: 3, y: 6 });
    m.insert(69, TileInfo { x: 4, y: 6 });
    m.insert(68, TileInfo { x: 5, y: 6 });
    m.insert(65, TileInfo { x: 6, y: 6 });

    m
});

pub fn fill_tilemap_render(
    world_size: u64,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    terrain_matrix: &Vec<Vec<bool>>,
    tilemap_masks: &Vec<u8>,
    #[cfg(all(not(feature = "atlas"), feature = "render"))] array_texture_loader: Res<
        ArrayTextureLoader,
    >,
) {
    // Массив для масок 0..15 (4 соседа)
    // Индексы соответствуют твоему файлу 11x9
    // let texture_handle: Handle<Image> = asset_server.load("sprout/Tilesets/Hills.png");
    let texture_handle: Handle<Image> = asset_server.load("wangbl.png");

    let map_size = TilemapSize {
        x: world_size as u32,
        y: world_size as u32,
    };

    // Create a tilemap entity a little early.
    // We want this entity early because we need to tell each tile which tilemap entity
    // it is associated with. This is done with the TilemapId component on each tile.
    // Eventually, we will insert the `TilemapBundle` bundle on the entity, which
    // will contain various necessary components, such as `TileStorage`.
    let tilemap_entity = commands.spawn_empty().id();

    // To begin creating the map we will need a `TileStorage` component.
    // This component is a grid of tile entities and is used to help keep track of individual
    // tiles in the world. If you have multiple layers of tiles you would have a tilemap entity
    // per layer, each with their own `TileStorage` component.
    let mut tile_storage = TileStorage::empty(map_size);

    for (i, mask) in tilemap_masks.iter().enumerate() {
        let world_size = world_size as u32;
        let r = i as u32 / world_size; // Ряд (Y)
        let c = i as u32 % world_size; // Колонка (X)

        let tile_pos = TilePos { x: c, y: r };

        let is_land = get_value_safe(terrain_matrix, r as i32, c as i32);

        let mut mask = *mask;
        if !is_land {
            mask = 0u8;
        }

        if let Some(tile_info) = TILE_MASKS.get(&mask) {
            let texture_index = tile_info.y * 7 + tile_info.x;

            let tile_entity = commands
                .spawn(TileBundle {
                    position: tile_pos,
                    texture_index: TileTextureIndex(texture_index as u32),
                    tilemap_id: TilemapId(tilemap_entity),
                    ..Default::default()
                })
                .id();
            tile_storage.set(&tile_pos, tile_entity);
        }
    }

    let tile_size = TilemapTileSize { x: 32.0, y: 32.0 };
    let grid_size = tile_size.into();
    let map_type = TilemapType::default();

    commands.entity(tilemap_entity).insert(TilemapBundle {
        grid_size,
        map_type,
        size: map_size,
        storage: tile_storage,
        texture: TilemapTexture::Single(texture_handle),
        tile_size,
        anchor: TilemapAnchor::Center,
        ..Default::default()
    });

    // Add atlas to array texture loader so it's preprocessed before we need to use it.
    // Only used when the atlas feature is off and we are using array textures.
    #[cfg(all(not(feature = "atlas"), feature = "render"))]
    {
        array_texture_loader.add(TilemapArrayTexture {
            texture: TilemapTexture::Single(asset_server.load("tiles.png")),
            tile_size,
            ..Default::default()
        });
    }
}

//PHYSICS
pub fn fixed_update_log(
    timeline: Res<LocalTimeline>,
    player: Query<(Entity, &Transform), (With<PlayerMarker>, With<PlayerId>)>,
    predicted_bullet: Query<
        (
            Entity,
            &Position,
            &Transform,
            Option<&PredictionHistory<Transform>>,
        ),
        With<BulletMarker>,
    >,
) {
    let tick = timeline.tick();
    for (entity, transform) in player.iter() {
        debug!(
            ?tick,
            ?entity,
            pos = ?transform.translation.truncate(),
            "Player after fixed update"
        );
    }
    for (entity, position, transform, history) in predicted_bullet.iter() {
        info!(
            ?tick,
            ?entity,
            ?position,
            transform = ?transform.translation.truncate(),
            ?history,
            "Bullet after fixed update"
        );
    }
}
