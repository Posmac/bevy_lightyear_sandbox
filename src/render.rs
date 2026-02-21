use crate::protocol::*;
use bevy::prelude::*;

#[derive(Clone)]
pub struct GameRendererPlugin;

impl Plugin for GameRendererPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init);
        app.add_systems(Update, draw);
    }
}

fn init(mut commands: Commands) {
    commands.spawn(Camera2d);
}

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

/// System that draws the boxes of the player positions.
/// The components should be replicated from the server to the client
pub fn draw(mut gizmos: Gizmos, mut players: Query<(&PlayerPosition, &mut Transform)>) {
    for (position, mut trx) in players.iter_mut() {
        gizmos.rect_2d(
            Isometry2d::from_translation(position.0),
            Vec2::ONE * 100.0,
            Color::LinearRgba(LinearRgba {
                red: 1.0,
                green: 0.0,
                blue: 0.0,
                alpha: 1.0,
            }),
        );

        let velocity = Vec3 {
            x: position.0.x,
            y: position.0.y,
            z: 0.0,
        };
        // .normalize_or_zero()
        //     * SPEED
        //     * time.delta_secs();
        trx.translation = velocity;
    }
}
