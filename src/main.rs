use std::time::Duration;

use bevy::app::ScheduleRunnerPlugin;
use bevy::log::*;
use bevy::winit::WinitPlugin;
use bevy::{prelude::*, window::PresentMode};
use bevy_ecs_tilemap::helpers::hex_grid::neighbors::HexDirection;
use clap::{Parser, Subcommand};
use lightyear::prelude::{client::ClientPlugins, server::ServerPlugins, *};

#[cfg(feature = "client")]
use crate::client::GameClientPlugin;

use crate::render::GameRendererPlugin;
#[cfg(feature = "server")]
use crate::server::GameServerPlugin;

use crate::shared::constants::{FIXED_TIMESTEP_HZ, SharedPlugin};

#[cfg(feature = "client")]
pub mod client;
#[cfg(feature = "server")]
pub mod server;

pub mod protocol;
pub mod render;
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
    // let cli = Cli::parse();

    println!("HUI");

    #[cfg(feature = "server")]
    {
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
            .disable::<WinitPlugin>();

        app.add_plugins(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
            1.0 / 60.0,
        )));
        // .set(WindowPlugin {
        //     primary_window: Some(Window {
        //         title: format!("Multiplayer: {}", env!("CARGO_PKG_NAME")),
        //         resolution: (1024, 768).into(),
        //         present_mode: PresentMode::AutoVsync,
        //         prevent_default_event_handling: true,
        //         ..default()
        //     }),
        //     ..default()
        // });

        app.add_plugins(default_plugins);
        app.add_plugins(ServerPlugins {
            tick_duration: Duration::from_secs_f64(1.0 / FIXED_TIMESTEP_HZ),
        });
        // NOTE: the ProtocolPlugin must be added AFTER the Client/Server plugins
        app.add_plugins(SharedPlugin);
        app.add_plugins(GameServerPlugin);
        app.add_plugins(GameRendererPlugin);
        app.run();
    }

    {
        #[cfg(feature = "client")]
        {
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
                        title: format!("Multiplayer: {}", env!("CARGO_PKG_NAME")),
                        resolution: (1024, 768).into(),
                        present_mode: PresentMode::AutoVsync,
                        prevent_default_event_handling: true,
                        ..default()
                    }),
                    ..default()
                });
            app.add_plugins(default_plugins);
            // add lightyear plugins
            app.add_plugins(ClientPlugins {
                tick_duration: Duration::from_secs_f64(1.0 / FIXED_TIMESTEP_HZ),
            });

            // NOTE: the ProtocolPlugin must be added AFTER the Client/Server plugins,
            app.add_plugins(SharedPlugin);
            // add client-specific plugins
            app.add_plugins(GameClientPlugin {
                client_id: rand::random::<u8>() as u64,
            });
            app.add_plugins(GameRendererPlugin);
            app.run();
        }
    }

    #[cfg(feature = "dev")]
    {
        app.add_plugins(bevy_inspector_egui::bevy_egui::EguiPlugin::default());
        app.add_plugins(bevy_inspector_egui::quick::WorldInspectorPlugin::new());
    }
}
