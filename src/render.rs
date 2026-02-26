use crate::{
    protocol::*,
    shared::{BOT_RADIUS, BULLET_SIZE, GREEN, PlayerAnimationTimer},
};
use avian2d::prelude::{ColliderAabb, Position};
use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::TilemapPlugin;
use lightyear::{
    frame_interpolation::{FrameInterpolate, FrameInterpolationPlugin},
    prelude::{Interpolated, Replicated},
};
use lightyear_avian2d::prelude::AabbEnvelopeHolder;

#[derive(Clone)]
pub struct GameRendererPlugin;

impl Plugin for GameRendererPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(TilemapPlugin);
        app.add_plugins(FrameInterpolationPlugin::<Transform>::default());

        app.add_systems(Startup, init);
        app.add_systems(Update, play_animation);

        #[cfg(feature = "client")]
        app.add_systems(Update, display_score);

        #[cfg(feature = "server")]
        app.add_systems(PostUpdate, draw_aabb_envelope);

        app.add_observer(add_bullet_visuals);
        app.add_observer(add_interpolated_bot_visuals);
    }
}

#[derive(Component)]
struct ScoreText;

#[cfg(feature = "client")]
fn display_score(
    mut score_text: Query<&mut Text, With<ScoreText>>,
    hits: Query<&Score, With<Replicated>>,
) {
    if let Ok(score) = hits.single() {
        if let Ok(mut text) = score_text.single_mut() {
            text.0 = format!("Score: {}", score.0);
        }
    }
}

#[cfg(feature = "server")]
fn draw_aabb_envelope(query: Query<&ColliderAabb, With<AabbEnvelopeHolder>>, mut gizmos: Gizmos) {
    query.iter().for_each(|collider_aabb| {
        gizmos.rect_2d(
            Isometry2d::new(collider_aabb.center(), Rot2::default()),
            collider_aabb.size(),
            Color::LinearRgba(LinearRgba {
                red: 1.0,
                green: 0.0,
                blue: 0.0,
                alpha: 1.0,
            }),
        );
    })
}

fn init(mut commands: Commands) {
    commands.spawn(Camera2d);
    #[cfg(feature = "client")]
    {
        commands.spawn((
            Text::new("Score: 0"),
            TextFont::from_font_size(40.0),
            TextColor(Color::WHITE.with_alpha(0.5)),
            Node {
                align_self: AlignSelf::End,
                ..Default::default()
            },
            ScoreText,
        ));
    }
}

fn add_bullet_visuals(
    trigger: On<Add, BulletMarker>,
    query: Query<Has<Interpolated>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    if let Ok(interpolated) = query.get(trigger.entity) {
        commands.entity(trigger.entity).insert((
            Visibility::default(),
            Mesh2d(meshes.add(Mesh::from(Circle {
                radius: BULLET_SIZE,
            }))),
            MeshMaterial2d(materials.add(ColorMaterial {
                color: Color::WHITE,
                ..Default::default()
            })),
        ));
        if interpolated {
            commands
                .entity(trigger.entity)
                .insert(FrameInterpolate::<Transform>::default());
        }
    }
}

/// System that draws the boxes of the player positions.
/// The components should be replicated from the server to the client
pub fn draw(
    mut gizmos: Gizmos,
    mut players: Query<(&Position, &mut Transform), With<PlayerMarker>>,
) {
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
    mut player_state: Query<(&Children, &PlayerState, &mut PlayerAnimations), With<PlayerMarker>>,
    mut visual_query: Query<(&mut PlayerAnimationTimer, &mut Sprite)>,
) {
    for (children, state, mut animations) in player_state.iter_mut() {
        for child in children.iter() {
            if let Ok((mut timer, mut sprite)) = visual_query.get_mut(child) {
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
    }
}

/// Add visuals to newly spawned bots
fn add_interpolated_bot_visuals(
    trigger: On<Add, BotMarker>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let entity = trigger.entity;
    // add visibility
    commands.entity(entity).insert((
        Visibility::default(),
        Mesh2d(meshes.add(Mesh::from(Circle { radius: BOT_RADIUS }))),
        MeshMaterial2d(materials.add(ColorMaterial {
            color: GREEN.into(),
            ..Default::default()
        })),
    ));
}
