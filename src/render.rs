use crate::protocol::*;
use bevy::prelude::*;

#[derive(Clone)]
pub struct GameRendererPlugin;

impl Plugin for GameRendererPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init);
        app.add_systems(Update, draw_boxes);
    }
}

fn init(mut commands: Commands) {
    commands.spawn(Camera2d);
}

/// System that draws the boxes of the player positions.
/// The components should be replicated from the server to the client
pub fn draw_boxes(mut gizmos: Gizmos, players: Query<&PlayerPosition>) {
    for position in &players {
        // info!("Position: {}", position.0);
        gizmos.rect_2d(
            Isometry2d::from_translation(position.0),
            Vec2::ONE * 50.0,
            Color::LinearRgba(LinearRgba {
                red: 1.0,
                green: 1.0,
                blue: 1.0,
                alpha: 1.0,
            }),
        );
    }
}
