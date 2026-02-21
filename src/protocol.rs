use bevy::ecs::entity::{EntityMapper, MapEntities};
use bevy::math::Curve;
use bevy::math::curve::{Ease, FunctionCurve, Interval};
use bevy::prelude::{Deref, DerefMut};
use bevy::{app::Plugin, ecs::component::Component, math::Vec2, reflect::Reflect};
use lightyear::prelude::*;
use serde::{Deserialize, Serialize};

pub struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_plugins(input::native::InputPlugin::<Inputs>::default());
        app.register_component::<PlayerPosition>()
            .add_prediction()
            .add_linear_interpolation();
        app.register_component::<MovementDirection>();
        app.register_component::<PlayerState>();
    }
}

impl Ease for PlayerPosition {
    fn interpolating_curve_unbounded(start: Self, end: Self) -> impl Curve<Self> {
        FunctionCurve::new(Interval::UNIT, move |t| {
            PlayerPosition(Vec2::lerp(start.0, end.0, t))
        })
    }
}

#[derive(
    Debug, Component, Serialize, Deserialize, Clone, PartialEq, Reflect, Deref, DerefMut, Default,
)]
pub struct PlayerPosition(pub Vec2);

#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Eq, Clone, Reflect)]
pub struct Direction {
    pub(crate) up: bool,
    pub(crate) down: bool,
    pub(crate) left: bool,
    pub(crate) right: bool,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Reflect)]
pub enum Inputs {
    Direction(Direction),
}

impl Default for Inputs {
    fn default() -> Self {
        Self::Direction(Direction::default())
    }
}

impl MapEntities for Inputs {
    fn map_entities<M: EntityMapper>(&mut self, entity_mapper: &mut M) {}
}

#[derive(Debug, Component, Serialize, Deserialize, Clone, PartialEq, Reflect, Default)]
pub enum MovementDirection {
    Back,
    Left,
    Right,
    #[default]
    Front,
}

#[derive(Debug, Component, Serialize, Deserialize, Clone, PartialEq, Reflect, Default)]
pub enum PlayerState {
    #[default]
    Idle,
    Walking,
}

// #[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Reflect)]
// pub struct AnimationConfig {
//     first_sprite_index: usize,
//     last_sprite_index: usize,
// }

// #[derive(Debug, Component, Serialize, Deserialize, Clone, PartialEq, Reflect)]
// pub struct PlayerAnimations {
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
//     // frame_timer: Timer,
// }

// impl PlayerAnimations {
//     pub fn new(
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
//         let character_animation_config = PlayerAnimations {
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
//             // frame_timer: Timer::new(
//             //     Duration::from_secs_f32(1.0 / (fps as f32)),
//             //     TimerMode::Repeating,
//             // ),
//         };
//         character_animation_config
//     }
// }
