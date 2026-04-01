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

    // #[cfg(feature = "server")]
    {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(
            Duration::from_secs_f64(1.0 / 60.0), // Серверный тик 60 FPS
        )));

        // app.add_plugins(
        //     bevy_app::PanicHandlerPlugin,
        //     bevy::log::LogPlugin::default(),
        //     bevy_app::TaskPoolPlugin,
        //     bevy_diagnostic::FrameCountPlugin,
        //     bevy_time::TimePlugin,
        //     bevy_transform::TransformPlugin,
        //     bevy_diagnostic::DiagnosticsPlugin,
        //     bevy_input::InputPlugin,
        //     bevy_app::TerminalCtrlCHandlerPlugin,
        //     bevy_asset::AssetPlugin,
        //     bevy_scene::ScenePlugin,
        // );
        // pub struct MinimalPlugins {
        //     bevy_app:::TaskPoolPlugin,
        //     bevy_diagnostic:::FrameCountPlugin,
        //     bevy_time:::TimePlugin,
        //     bevy_app:::ScheduleRunnerPlugin,
        //     #[cfg(feature = "bevy_ci_testing")]
        //     bevy_dev_tools::ci_testing:::CiTestingPlugin,
        // }
        //
        app.add_plugins((
            //     bevy_asset::io::web::WebAssetPlugin,
            bevy::app::PanicHandlerPlugin::default(),
            // bevy::app::TaskPoolPlugin::default(),
            // bevy::diagnostic::FrameCountPlugin::default(),
            // bevy::time::TimePlugin::default(),
            bevy::diagnostic::DiagnosticsPlugin::default(),
            bevy::input::InputPlugin::default(),
            bevy::app::TerminalCtrlCHandlerPlugin::default(),
            bevy::log::LogPlugin::default(), // Чтобы видеть инфо в консоли
            bevy::transform::TransformPlugin, // Координаты (нужно для всего)
            bevy::asset::AssetPlugin::default(), // Загрузка данных
            bevy::scene::ScenePlugin,        // Работа со сценами (нужно для Avian)
                                             // bevy::state::app::StatesPlugin,      // Стейты игры
        ));

        // app.add_plugins(
        // DefaultPlugins
        //         .build()
        //         // 1. Отключаем графическое ядро
        //         .disable::<bevy::render::RenderPlugin>()
        //         .disable::<bevy::render::pipelined_rendering::PipelinedRenderingPlugin>()
        //         .disable::<bevy::core_pipeline::CorePipelinePlugin>()
        //         // 2. Отключаем окна и ввод
        //         .disable::<bevy::winit::WinitPlugin>()
        //         .disable::<bevy::window::WindowPlugin>()
        //         // 3. Отключаем высокоуровневую графику (шейдеры!)
        //         // .disable::<bevy::pbr::PbrPlugin>()
        //         .disable::<bevy::sprite::SpritePlugin>()
        //         .disable::<bevy::ui::UiPlugin>()
        //         .disable::<bevy::text::TextPlugin>()
        //         .disable::<bevy::mesh::MeshPlugin>()
        //         .disable::<bevy::image::ImagePlugin>()
        //         // 4. Отключаем звук и прочее
        //         // .disable::<bevy::audio::AudioPlugin>()
        //         .disable::<bevy::gilrs::GilrsPlugin>()
        //         .disable::<bevy::gizmos::GizmoPlugin>(),
        // );

        // ВАЖНО: Поскольку мы отключили WindowPlugin, нам нужно включить ScheduleRunnerPlugin,
        // чтобы сервер работал в цикле, а не ждал событий окна.
        // app.add_plugins(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
        //     1.0 / 60.0,
        // )));
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

        // app.add_plugins(default_plugins);
        app.add_plugins(ServerPlugins {
            tick_duration: Duration::from_secs_f64(1.0 / FIXED_TIMESTEP_HZ),
        });
        // NOTE: the ProtocolPlugin must be added AFTER the Client/Server plugins
        app.add_plugins(SharedPlugin);
        app.add_plugins(GameServerPlugin);
        // app.add_plugins(GameRendererPlugin);
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
