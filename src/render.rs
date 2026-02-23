use crate::{protocol::*, shared::PlayerAnimationTimer};
use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::TilemapPlugin;

#[derive(Clone)]
pub struct GameRendererPlugin;

impl Plugin for GameRendererPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(TilemapPlugin);
        app.add_systems(Startup, init);
        app.add_systems(Update, draw);
        app.add_systems(Update, play_animation);
    }
}

fn init(mut commands: Commands) {
    commands.spawn(Camera2d);
}

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

fn play_animation(
    time: Res<Time>,
    mut player: Query<(
        &PlayerState,
        &mut PlayerAnimations,
        &mut PlayerAnimationTimer,
        &mut Sprite,
    )>,
) {
    for (state, mut animations, mut timer, mut sprite) in player.iter_mut() {
        timer.frame_timer.tick(time.delta());
        if let Some(atlas) = &mut sprite.texture_atlas {
            match state.current_state {
                PlayerStateEnum::IdleFront => {
                    animations.current_animation = animations.idle_front;
                }
                PlayerStateEnum::IdleBack => {
                    animations.current_animation = animations.idle_back;
                }
                PlayerStateEnum::IdleLeft => {
                    animations.current_animation = animations.idle_left;
                }
                PlayerStateEnum::IdleRight => {
                    animations.current_animation = animations.idle_right;
                }
                PlayerStateEnum::WalkingFront => {
                    animations.current_animation = animations.move_front;
                }
                PlayerStateEnum::WalkingBack => {
                    animations.current_animation = animations.move_back;
                }
                PlayerStateEnum::WalkingLeft => {
                    animations.current_animation = animations.move_left;
                }
                PlayerStateEnum::WalkingRight => {
                    animations.current_animation = animations.move_right;
                }
            };

            if state.prev_state != state.current_state {
                atlas.index = animations.current_animation.first_sprite_index;
                timer.frame_timer.finish();
            } else {
                if timer.frame_timer.just_finished() {
                    if atlas.index >= animations.current_animation.last_sprite_index
                        || atlas.index < animations.current_animation.first_sprite_index
                    {
                        atlas.index = animations.current_animation.first_sprite_index;
                    } else {
                        atlas.index += 1;
                    }
                }
            }
        }
    }
}
