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

// fn clear_input(player_input: Single<&mut AccumulatedInput>) {
//     let mut input = player_input.into_inner();
//     input.movement = Vec2::default();
// }
