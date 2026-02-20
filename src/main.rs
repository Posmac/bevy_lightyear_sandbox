use std::time::Duration;

use bevy::log::*;
use bevy::render::RenderPlugin;
use bevy::{prelude::*, window::PresentMode};
use clap::{Parser, Subcommand};
use lightyear::prelude::{client::ClientPlugins, server::ServerPlugins, *};

use crate::client::GameClientPlugin;
use crate::render::GameRendererPlugin;
use crate::server::GameServerPlugin;
use crate::shared::{FIXED_TIMESTEP_HZ, SharedPlugin};

// #[cfg(feature = "client")]
pub mod client;
pub mod protocol;
pub mod render;
// #[cfg(feature = "server")]
pub mod server;
pub mod shared;

#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub mode: Mode,
}

#[derive(Subcommand, Debug)]
pub enum Mode {
    Client {
        #[arg(short, long, default_value = None)]
        client_id: Option<u64>,
    },

    Server,
}

fn main() {
    let cli = Cli::parse();
    let mut app = App::new();
    let default_plugins = DefaultPlugins
        .set(ImagePlugin::default_nearest())
        .set(AssetPlugin {
            meta_check: bevy::asset::AssetMetaCheck::Never,
            ..default()
        })
        .set(LogPlugin {
            level: Level::INFO,
            // filter: "wgpu=error,bevy_render=info,bevy_ecs=warn,bevy_time=warn,naga=warn,bevy_enhanced_input::action::fns=error".to_string(),
            ..default()
        })
        .set(WindowPlugin {
            primary_window: Some(Window {
                title: format!("Multiplayer: {} {:#?}", env!("CARGO_PKG_NAME"), cli.mode),
                resolution: (1024, 768).into(),
                present_mode: PresentMode::AutoVsync,
                prevent_default_event_handling: true,
                ..default()
            }),
            ..default()
        });

    match cli.mode {
        Mode::Client { client_id } => {
            app.add_plugins(default_plugins);
            // add lightyear plugins
            app.add_plugins(ClientPlugins {
                tick_duration: Duration::from_secs_f64(1.0 / FIXED_TIMESTEP_HZ),
            });

            // NOTE: the ProtocolPlugin must be added AFTER the Client/Server plugins,
            app.add_plugins(SharedPlugin);
            // add client-specific plugins
            app.add_plugins(GameClientPlugin {
                client_id: client_id.expect("Client id is NONE!"),
            });
            app.add_plugins(GameRendererPlugin);
        }
        Mode::Server => {
            app.add_plugins(default_plugins);
            app.add_plugins(ServerPlugins {
                tick_duration: Duration::from_secs_f64(1.0 / FIXED_TIMESTEP_HZ),
            });
            // NOTE: the ProtocolPlugin must be added AFTER the Client/Server plugins
            app.add_plugins(SharedPlugin);
            app.add_plugins(GameServerPlugin);
            app.add_plugins(GameRendererPlugin);
        }
    }

    app.add_plugins(bevy_inspector_egui::bevy_egui::EguiPlugin::default());
    app.add_plugins(bevy_inspector_egui::quick::WorldInspectorPlugin::new());

    app.run();

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
}

// #[derive(Copy, Clone, Debug)]
// struct AnimationConfig {
//     first_sprite_index: usize,
//     last_sprite_index: usize,
// }

// #[derive(Debug, Component, Clone, Copy, PartialEq)]
// enum MovementDirection {
//     Back,
//     Left,
//     Right,
//     Front,
// }

// impl Default for MovementDirection {
//     fn default() -> Self {
//         MovementDirection::Front
//     }
// }

// #[derive(Component)]
// struct CharacterAnimations {
//     current_animation: AnimationConfig,

//     idle_front: AnimationConfig,
//     idle_back: AnimationConfig,
//     idle_left: AnimationConfig,
//     idle_right: AnimationConfig,

//     move_front: AnimationConfig,
//     move_back: AnimationConfig,
//     move_left: AnimationConfig,
//     move_right: AnimationConfig,

//     fps: u8,
//     frame_timer: Timer,
// }

// impl CharacterAnimations {
//     fn new(
//         idle_front: AnimationConfig,
//         idle_back: AnimationConfig,
//         idle_left: AnimationConfig,
//         idle_right: AnimationConfig,

//         move_front: AnimationConfig,
//         move_back: AnimationConfig,
//         move_left: AnimationConfig,
//         move_right: AnimationConfig,

//         fps: u8,
//     ) -> Self {
//         let character_animation_config = CharacterAnimations {
//             current_animation: idle_front,
//             idle_front,
//             idle_back,
//             idle_left,
//             idle_right,
//             move_front,
//             move_back,
//             move_left,
//             move_right,
//             fps: fps,
//             frame_timer: Timer::new(
//                 Duration::from_secs_f32(1.0 / (fps as f32)),
//                 TimerMode::Repeating,
//             ),
//         };
//         character_animation_config
//     }
// }

// #[derive(Debug, Component, Clone, Copy, PartialEq, Default)]
// struct AccumulatedInput {
//     // The player's movement input (WASD).
//     movement: Vec2,
//     movement_direction: MovementDirection,
//     prev_movement_direction: MovementDirection,
//     is_new_direction: bool,
//     is_zero: bool,
//     // Other input that could make sense would be e.g.
//     // boost: bool
// }

// /// A vector representing the player's velocity in the physics simulation.
// #[derive(Debug, Component, Clone, Copy, PartialEq, Default, Deref, DerefMut)]
// struct Velocity(Vec2);

// fn setup_camera(mut commands: Commands) {
//     commands.spawn(Camera2d);
// }

// fn setup_character(
//     mut commands: Commands,
//     asset_server: Res<AssetServer>,
//     mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
// ) {
//     let character_texture = asset_server.load("sprout/Characters/basic_movement.png");
//     let sprite_sheet_layout = TextureAtlasLayout::from_grid(UVec2::new(48, 48), 4, 4, None, None);
//     let texture_atlas_layout = texture_atlas_layouts.add(sprite_sheet_layout);

//     let character_animation_config = CharacterAnimations::new(
//         AnimationConfig {
//             first_sprite_index: 0,
//             last_sprite_index: 1,
//         },
//         AnimationConfig {
//             first_sprite_index: 4,
//             last_sprite_index: 5,
//         },
//         AnimationConfig {
//             first_sprite_index: 8,
//             last_sprite_index: 9,
//         },
//         AnimationConfig {
//             first_sprite_index: 12,
//             last_sprite_index: 13,
//         },
//         AnimationConfig {
//             first_sprite_index: 2,
//             last_sprite_index: 3,
//         },
//         AnimationConfig {
//             first_sprite_index: 6,
//             last_sprite_index: 7,
//         },
//         AnimationConfig {
//             first_sprite_index: 10,
//             last_sprite_index: 11,
//         },
//         AnimationConfig {
//             first_sprite_index: 14,
//             last_sprite_index: 15,
//         },
//         2, //fps
//     );

//     commands.spawn((
//         Sprite::from_atlas_image(
//             character_texture,
//             TextureAtlas {
//                 layout: texture_atlas_layout,
//                 index: 0,
//             },
//         ),
//         Transform::from_scale(Vec3::splat(6.0)),
//         character_animation_config,
//         AccumulatedInput::default(),
//     ));
// }

// fn process_character_animation(
//     time: Res<Time>,
//     player: Single<(&mut AccumulatedInput, &mut CharacterAnimations, &mut Sprite)>,
// ) {
//     let (input, mut animations, mut sprite) = player.into_inner();

//     match input.movement_direction {
//         MovementDirection::Front => match input.is_zero {
//             true => {
//                 animations.current_animation = animations.idle_front;
//             }
//             false => {
//                 animations.current_animation = animations.move_front;
//             }
//         },
//         MovementDirection::Left => match input.is_zero {
//             true => {
//                 animations.current_animation = animations.idle_left;
//             }
//             false => {
//                 animations.current_animation = animations.move_left;
//             }
//         },
//         MovementDirection::Right => match input.is_zero {
//             true => {
//                 animations.current_animation = animations.idle_right;
//             }
//             false => {
//                 animations.current_animation = animations.move_right;
//             }
//         },
//         MovementDirection::Back => match input.is_zero {
//             true => {
//                 animations.current_animation = animations.idle_back;
//             }
//             false => {
//                 animations.current_animation = animations.move_back;
//             }
//         },
//     };

//     animations.frame_timer.tick(time.delta());
//     if let Some(atlas) = &mut sprite.texture_atlas {
//         if input.is_new_direction {
//             atlas.index = animations.current_animation.first_sprite_index;
//         } else if animations.frame_timer.just_finished() {
//             if atlas.index >= animations.current_animation.last_sprite_index
//                 || atlas.index < animations.current_animation.first_sprite_index
//             {
//                 atlas.index = animations.current_animation.first_sprite_index;
//             } else {
//                 atlas.index += 1;
//             }
//         }
//     }
// }

// fn accumulate_input(
//     keyboard_input: Res<ButtonInput<KeyCode>>,
//     player: Single<&mut AccumulatedInput>,
//     // camera: Single<&Transform, With<Camera>>,
// ) {
//     let mut input = player.into_inner();
//     // Reset the input to zero before reading the new input. As mentioned above, we can only do this
//     // because this is continuously pressed by the user. Do not reset e.g. whether the user wants to boost.
//     input.movement = Vec2::ZERO;
//     input.is_zero = true;
//     input.prev_movement_direction = input.movement_direction;
//     if keyboard_input.pressed(KeyCode::KeyW) || keyboard_input.pressed(KeyCode::ArrowUp) {
//         input.movement.y += 1.0;
//         input.movement_direction = MovementDirection::Back;
//         input.is_zero = false;
//     }
//     if keyboard_input.pressed(KeyCode::KeyS) || keyboard_input.pressed(KeyCode::ArrowDown) {
//         input.movement.y -= 1.0;
//         input.movement_direction = MovementDirection::Front;
//         input.is_zero = false;
//     }
//     if keyboard_input.pressed(KeyCode::KeyA) || keyboard_input.pressed(KeyCode::ArrowLeft) {
//         input.movement.x -= 1.0;
//         input.movement_direction = MovementDirection::Left;
//         input.is_zero = false;
//     }
//     if keyboard_input.pressed(KeyCode::KeyD) || keyboard_input.pressed(KeyCode::ArrowRight) {
//         input.movement.x += 1.0;
//         input.movement_direction = MovementDirection::Right;
//         input.is_zero = false;
//     }

//     input.is_new_direction = input.prev_movement_direction != input.movement_direction;
// }

// fn move_character(player: Single<(&AccumulatedInput, &mut Transform)>, time: Res<Time>) {
//     /// Since Bevy's 3D renderer assumes SI units, this has the unit of meters per second.
//     /// Note that about 1.5 is the average walking speed of a human.
//     const SPEED: f32 = 400.0;
//     let (input, mut transform) = player.into_inner();

//     let velocity = Vec3 {
//         x: input.movement.x,
//         y: input.movement.y,
//         z: 0.0,
//     }
//     .normalize_or_zero()
//         * SPEED
//         * time.delta_secs();
//     transform.translation += velocity;
// }

// fn clear_input(player_input: Single<&mut AccumulatedInput>) {
//     let mut input = player_input.into_inner();
//     input.movement = Vec2::default();
// }
