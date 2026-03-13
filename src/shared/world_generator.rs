use std::{collections::HashMap, sync::LazyLock};

use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use noise::{
    Fbm, Perlin,
    utils::{NoiseMap, NoiseMapBuilder, PlaneMapBuilder},
};

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
