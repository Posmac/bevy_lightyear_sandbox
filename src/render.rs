use crate::{
    // client::ClientId,
    protocol::*,
    shared::constants::{
        BOT_RADIUS, BULLET_SIZE, HEALTH_BAR_SIZE, ITEM_RADIUS, PLAYER_SIZE, PlayerAnimationTimer,
        Wall,
    },
};
use avian2d::prelude::{ColliderAabb, PhysicsDebugPlugin, Position, RigidBody, Rotation};
use bevy::{
    color::palettes::css::{BLUE, GREEN, RED},
    prelude::*,
};
use bevy_ecs_tilemap::prelude::TilemapPlugin;
#[cfg(feature = "client")]
use lightyear::prelude::Predicted;
use lightyear::{
    frame_interpolation::{FrameInterpolate, FrameInterpolationPlugin},
    prelude::{Interpolated, InterpolationSystems, Replicated, RollbackSystems},
};
use lightyear_avian2d::prelude::AabbEnvelopeHolder;

#[derive(Clone)]
pub struct GameRendererPlugin;

impl Plugin for GameRendererPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(TilemapPlugin);
        app.add_plugins(FrameInterpolationPlugin::<Position>::default());
        app.add_plugins(FrameInterpolationPlugin::<Rotation>::default());
        app.add_plugins(PhysicsDebugPlugin::default());

        app.add_systems(Startup, init);

        #[cfg(feature = "client")]
        app.add_systems(PostUpdate, display_score);
        app.add_systems(PostUpdate, play_animation);
        #[cfg(feature = "server")]
        app.add_systems(PostUpdate, draw_aabb_envelope);
        app.add_systems(
            PostUpdate,
            draw_walls
                .after(InterpolationSystems::Interpolate)
                .after(RollbackSystems::VisualCorrection),
        );
        app.add_systems(PostUpdate, check_net_pos);

        app.add_systems(Update, update_health_bar);
        app.add_observer(add_health_bar_visuals);
        app.add_observer(add_bullet_visuals);
        // app.add_observer(add_interpolated_bot_visuals);
        // app.add_observer(add_interpolated_item_visuals);
    }
}

#[derive(Component)]
struct ScoreText;

#[cfg(feature = "client")]
fn display_score(
    mut score_text: Query<&mut Text, With<ScoreText>>,
    hits: Query<&Score, With<Predicted>>,
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

// fn add_bullet_visuals(
//     trigger: On<Add, BulletMarker>,
//     query: Query<Has<Interpolated>>,
//     mut commands: Commands,
//     mut meshes: ResMut<Assets<Mesh>>,
//     mut materials: ResMut<Assets<ColorMaterial>>,
// ) {
//     if let Ok(interpolated) = query.get(trigger.entity) {
//         commands.entity(trigger.entity).insert((
//             Visibility::default(),
//             Mesh2d(meshes.add(Mesh::from(Circle {
//                 radius: BULLET_SIZE,
//             }))),
//             MeshMaterial2d(materials.add(ColorMaterial {
//                 color: Color::WHITE,
//                 ..Default::default()
//             })),
//             RigidBody::Kinematic,
//         ));
//         if interpolated {
//             commands.entity(trigger.entity).insert((
//                 FrameInterpolate::<Position>::default(),
//                 FrameInterpolate::<Rotation>::default(),
//             ));
//         }
//     }
// }

pub fn add_health_bar_visuals(
    trigger: On<Add, HealthComponent>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.entity(trigger.entity).with_children(|parent| {
        parent.spawn((
            Mesh2d(meshes.add(Rectangle::from_size(HEALTH_BAR_SIZE))),
            MeshMaterial2d(materials.add(Color::BLACK)),
            Transform::from_xyz(0.0, PLAYER_SIZE * 1.05, 1.0),
        ));

        parent.spawn((
            HealthBarMarker,
            Mesh2d(meshes.add(Rectangle::from_size(HEALTH_BAR_SIZE))),
            MeshMaterial2d(materials.add(ColorMaterial {
                color: Color::Srgba(Srgba {
                    red: 0.0,
                    green: 1.0,
                    blue: 0.0,
                    alpha: 1.0,
                }),
                ..Default::default()
            })),
            Transform::from_xyz(0.0, PLAYER_SIZE * 1.05, 1.1),
        ));
    });
}

pub fn update_health_bar(
    mut health_bar_query: Query<(&mut Transform, &ChildOf), With<HealthBarMarker>>,
    parent_query: Query<&HealthComponent>,
) {
    for (mut transform, parent) in health_bar_query.iter_mut() {
        if let Ok(health) = parent_query.get(parent.0) {
            let health_pct = health.current_health as f32 / health.max_health as f32;
            let health_pct = health_pct.clamp(0.0, 1.0);

            transform.scale.x = health_pct;

            let offset = (1.0 - health_pct) * (HEALTH_BAR_SIZE.x / 2.0);
            transform.translation.x = -offset;
        }
    }
}

fn add_bullet_visuals(
    trigger: On<Add, BulletMarker>,
    query: Query<(&Position, Has<Interpolated>)>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    if let Ok((pos, interpolated)) = query.get(trigger.entity) {
        commands.entity(trigger.entity).insert((
            // Создаем Transform сразу в нужной позиции,
            // иначе один кадр пуля будет в 0,0,0
            Transform::from_translation(pos.0.extend(0.1)),
            Visibility::default(),
            Mesh2d(meshes.add(Mesh::from(Circle {
                radius: BULLET_SIZE,
            }))),
            MeshMaterial2d(materials.add(ColorMaterial {
                color: Color::WHITE,
                ..Default::default()
            })),
            RigidBody::Kinematic,
        ));

        if interpolated {
            commands.entity(trigger.entity).insert((
                FrameInterpolate::<Position>::default(),
                FrameInterpolate::<Rotation>::default(),
            ));
        }
    }
}

// /// System that draws the boxes of the player positions.
// /// The components should be replicated from the server to the client
// pub fn draw(
//     mut gizmos: Gizmos,
//     mut players: Query<(&Position, &mut Transform), With<PlayerMarker>>,
// ) {
//     for (position, mut trx) in players.iter_mut() {
//         gizmos.rect_2d(
//             Isometry2d::from_translation(position.0),
//             Vec2::ONE * 100.0,
//             Color::LinearRgba(LinearRgba {
//                 red: 1.0,
//                 green: 0.0,
//                 blue: 0.0,
//                 alpha: 1.0,
//             }),
//         );

//         let velocity = Vec3 {
//             x: position.0.x,
//             y: position.0.y,
//             z: 0.0,
//         };
//         // .normalize_or_zero()
//         //     * SPEED
//         //     * time.delta_secs();
//         trx.translation = velocity;
//     }
// }

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
// fn add_interpolated_bot_visuals(
//     trigger: On<Add, BotMarker>,
//     mut commands: Commands,
//     mut meshes: ResMut<Assets<Mesh>>,
//     mut materials: ResMut<Assets<ColorMaterial>>,
//     client_id: Option<Res<ClientId>>,
// ) {
//     let entity = trigger.entity;
//     if client_id.is_some() {
//         commands.entity(entity).insert((
//             Transform::from_xyz(200.0, 10.0, 0.0),
//             GlobalTransform::default(),
//             InheritedVisibility::default(),
//         ));
//     }

//     // add visibility
//     commands.entity(entity).with_children(|parent| {
//         parent.spawn((
//             Mesh2d(meshes.add(Mesh::from(Circle { radius: BOT_RADIUS }))),
//             MeshMaterial2d(materials.add(ColorMaterial {
//                 color: GREEN.into(),
//                 ..Default::default()
//             })),
//             Transform::default(),
//             GlobalTransform::default(),
//             InheritedVisibility::default(),
//         ));
//     });
// }

fn check_net_pos(q: Query<&Position, With<ItemMarker>>) {
    // for p in q.iter() {
    //     println!("Network Position: {:?}", p.0);
    // }
}

// pub fn add_interpolated_item_visuals(
//     trigger: On<Add, ItemMarker>,
//     mut commands: Commands,
//     mut meshes: ResMut<Assets<Mesh>>,
//     mut materials: ResMut<Assets<ColorMaterial>>,
//     client_id: Option<Res<ClientId>>,
//     query: Query<&Position, With<ItemMarker>>,
// ) {
//     let entity = trigger.entity;
//     let initial_pos = query.get(entity).map(|p| p.0).unwrap_or(Vec2::ZERO);

//     info!("Client: Visual spawned for item at {:?}", initial_pos);
//     if client_id.is_some() {
//         commands.entity(entity).insert((
//             Transform::from_xyz(initial_pos.x, initial_pos.y, 0.0),
//             GlobalTransform::default(),
//             InheritedVisibility::default(),
//         ));
//     }

//     // add visibility
//     commands.entity(entity).with_children(|parent| {
//         parent.spawn((
//             Mesh2d(meshes.add(Mesh::from(Circle {
//                 radius: ITEM_RADIUS,
//             }))),
//             MeshMaterial2d(materials.add(ColorMaterial {
//                 color: RED.into(),
//                 ..Default::default()
//             })),
//             Transform::default(),
//             GlobalTransform::default(),
//             InheritedVisibility::default(),
//         ));
//     });
// }

fn draw_walls(mut gizmos: Gizmos, walls: Query<&Wall, ()>) {
    for wall in &walls {
        gizmos.rect_2d(Isometry2d::from_translation(wall.position), wall.size, BLUE);
    }
}
