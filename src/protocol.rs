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
        // app.register_component::<MovementDirection>();
        app.register_component::<PlayerState>();
        app.register_component::<PlayerAnimations>();
        app.register_component::<WorldConfig>();
    }
}

impl Ease for PlayerPosition {
    fn interpolating_curve_unbounded(start: Self, end: Self) -> impl Curve<Self> {
        FunctionCurve::new(Interval::UNIT, move |t| {
            PlayerPosition(Vec2::lerp(start.0, end.0, t))
        })
    }
}

//Position component for player
#[derive(
    Debug, Component, Serialize, Deserialize, Clone, PartialEq, Reflect, Deref, DerefMut, Default,
)]
pub struct PlayerPosition(pub Vec2);

#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Eq, Clone, Reflect)]
pub struct Direction {
    pub front: bool,
    pub back: bool,
    pub left: bool,
    pub right: bool,
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

impl Direction {
    pub fn is_none(&self) -> bool {
        !self.front && !self.back && !self.left && !self.right
    }
}

impl MapEntities for Inputs {
    fn map_entities<M: EntityMapper>(&mut self, entity_mapper: &mut M) {}
}

//Player state component
#[derive(Debug, Component, Serialize, Deserialize, Clone, PartialEq, Reflect, Default)]
pub struct PlayerState {
    pub current_state: PlayerStateEnum,
    pub prev_state: PlayerStateEnum,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Reflect, Default)]
pub enum PlayerStateEnum {
    #[default]
    IdleFront,
    IdleBack,
    IdleLeft,
    IdleRight,
    WalkingFront,
    WalkingBack,
    WalkingLeft,
    WalkingRight,
}

impl PlayerStateEnum {
    pub fn is_idle(&self) -> bool {
        match self {
            PlayerStateEnum::IdleFront => true,
            PlayerStateEnum::IdleBack => true,
            PlayerStateEnum::IdleLeft => true,
            PlayerStateEnum::IdleRight => true,
            _ => false,
        }
    }

    pub fn is_walking(&self) -> bool {
        !self.is_idle()
    }

    pub fn get_opposite_state(&self) -> Self {
        match self {
            PlayerStateEnum::IdleFront => PlayerStateEnum::WalkingFront,
            PlayerStateEnum::IdleBack => PlayerStateEnum::WalkingBack,
            PlayerStateEnum::IdleLeft => PlayerStateEnum::WalkingLeft,
            PlayerStateEnum::IdleRight => PlayerStateEnum::WalkingRight,
            PlayerStateEnum::WalkingFront => PlayerStateEnum::IdleFront,
            PlayerStateEnum::WalkingBack => PlayerStateEnum::IdleBack,
            PlayerStateEnum::WalkingLeft => PlayerStateEnum::IdleLeft,
            PlayerStateEnum::WalkingRight => PlayerStateEnum::IdleRight,
        }
    }
}

//Animations component
#[derive(Debug, Component, Serialize, Deserialize, Clone, PartialEq, Reflect, Default)]
pub struct PlayerAnimations {
    pub current_animation: AnimationConfig,

    pub idle_front: AnimationConfig,
    pub idle_back: AnimationConfig,
    pub idle_left: AnimationConfig,
    pub idle_right: AnimationConfig,

    pub move_front: AnimationConfig,
    pub move_back: AnimationConfig,
    pub move_left: AnimationConfig,
    pub move_right: AnimationConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Reflect, Default)]
pub struct AnimationConfig {
    pub first_sprite_index: usize,
    pub last_sprite_index: usize,
}

impl PlayerAnimations {
    pub fn new(
        idle_front: AnimationConfig,
        idle_back: AnimationConfig,
        idle_left: AnimationConfig,
        idle_right: AnimationConfig,

        move_front: AnimationConfig,
        move_back: AnimationConfig,
        move_left: AnimationConfig,
        move_right: AnimationConfig,
    ) -> Self {
        let character_animation_config = PlayerAnimations {
            current_animation: idle_front,
            idle_front,
            idle_back,
            idle_left,
            idle_right,
            move_front,
            move_back,
            move_left,
            move_right,
        };
        character_animation_config
    }

    pub fn get_anim(&self, player_state: &PlayerStateEnum) -> AnimationConfig {
        match player_state {
            PlayerStateEnum::IdleFront => self.idle_front,
            PlayerStateEnum::IdleBack => self.idle_back,
            PlayerStateEnum::IdleLeft => self.idle_left,
            PlayerStateEnum::IdleRight => self.idle_right,
            PlayerStateEnum::WalkingFront => self.move_front,
            PlayerStateEnum::WalkingBack => self.move_back,
            PlayerStateEnum::WalkingLeft => self.move_left,
            PlayerStateEnum::WalkingRight => self.move_right,
        }
    }
}

//world generator
#[derive(Debug, Component, Serialize, Deserialize, Clone, PartialEq, Reflect, Default)]
pub struct WorldConfig {
    pub world_size: u64,
    pub seed: u32,
}
