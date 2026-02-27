use avian2d::dynamics::solver::xpbd::XpbdConstraint;
use avian2d::prelude::*;
use bevy::ecs::entity::{EntityMapper, MapEntities};
use bevy::prelude::{Deref, DerefMut};
use bevy::transform::components::Transform;
use bevy::{app::Plugin, ecs::component::Component, reflect::Reflect};
use bevy_ecs::bundle::Bundle;
use leafwing_input_manager::Actionlike;
use lightyear::input::config::InputConfig;
use lightyear::prelude::input::leafwing::InputPlugin;
use lightyear::prelude::*;

use serde::{Deserialize, Serialize};

use crate::shared::{BOT_RADIUS, PLAYER_SIZE};

pub struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        // app.add_plugins(input::native::InputPlugin::<Inputs>::default());

        app.add_plugins(InputPlugin::<Inputs> {
            config: InputConfig::<Inputs> {
                lag_compensation: true,
                rebroadcast_inputs: true,
                ..Default::default()
            },
        });

        //physics parameters
        app.register_component::<Position>()
            .add_prediction()
            .add_linear_interpolation()
            .add_should_rollback(position_should_rollback)
            .enable_correction()
            .add_linear_correction_fn();

        app.register_component::<Rotation>()
            .add_prediction()
            .add_linear_interpolation()
            .add_should_rollback(rotation_should_rollback)
            .enable_correction()
            .add_linear_correction_fn();

        // app.register_component::<RigidBody>();

        // NOTE: interpolation/correction is only needed for components that are visually displayed!
        // we still need prediction to be able to correctly predict the physics on the client
        app.register_component::<LinearVelocity>().add_prediction();

        app.register_component::<AngularVelocity>().add_prediction();

        //other params
        app.register_component::<PlayerState>();
        app.register_component::<PlayerAnimations>();
        app.register_component::<WorldConfig>();

        app.register_component::<PlayerId>();
        app.register_component::<Score>();

        app.register_component::<PlayerMarker>();
        app.register_component::<BulletMarker>();

        //bots
        app.register_component::<BotMarker>();
    }
}

// impl Ease for PlayerPosition {
//     fn interpolating_curve_unbounded(start: Self, end: Self) -> impl Curve<Self> {
//         FunctionCurve::new(Interval::UNIT, move |t| {
//             PlayerPosition(Vec2::lerp(start.0, end.0, t))
//         })
//     }
// }

//Position component for player
// #[derive(
//     Debug, Component, Serialize, Deserialize, Clone, PartialEq, Reflect, Deref, DerefMut, Default,
// )]
// pub struct PlayerPosition(pub Vec2);

// #[derive(Debug, Serialize, Deserialize, Default, PartialEq, Eq, Clone, Copy, Hash, Reflect)]
// pub enum Direction {}

#[derive(Debug, Serialize, Deserialize, Default, PartialEq, Eq, Clone, Copy, Hash, Reflect)]
pub enum Inputs {
    Up,
    #[default]
    Down,
    Left,
    Right,
    Mouse,
    Shoot,
}

impl Actionlike for Inputs {
    fn input_control_kind(&self) -> leafwing_input_manager::InputControlKind {
        match self {
            Inputs::Mouse => leafwing_input_manager::InputControlKind::DualAxis,
            _ => leafwing_input_manager::InputControlKind::Button,
        }
    }
}

// impl Default for Inputs {
//     fn default() -> Self {
//         Self::Direction(Direction::default())
//     }
// }

// impl Direction {
//     pub fn is_none(&self) -> bool {
//         !self.front && !self.back && !self.left && !self.right
//     }
// }

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
    pub seed: u32,
    pub world_size: u64,
}

//new
#[derive(
    Debug, Component, Serialize, Deserialize, Clone, Copy, PartialEq, Reflect, Deref, DerefMut,
)]
pub struct Score(pub usize);

#[derive(Component, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Reflect)]
pub struct PlayerId(pub PeerId);

#[derive(Debug, Component, Serialize, Deserialize, Clone, Copy, PartialEq, Reflect)]
pub struct BulletMarker;

#[derive(Debug, Component, Serialize, Deserialize, Clone, Copy, PartialEq, Reflect)]
pub struct PlayerMarker;

#[derive(Debug, Component, Serialize, Deserialize, Clone, Copy, PartialEq, Reflect)]
pub struct BotMarker;

fn position_should_rollback(this: &Position, that: &Position) -> bool {
    (this.0 - that.0).length() >= 0.01
}

fn rotation_should_rollback(this: &Rotation, that: &Rotation) -> bool {
    this.angle_between(*that) >= 0.01
}

#[derive(Bundle)]
pub struct PhysicsBundle {
    pub collider: Collider,
    pub collider_density: ColliderDensity,
    pub rigid_body: RigidBody,
    pub restitution: Restitution,
    pub constraint: LockedAxes,
    pub dumping: LinearDamping,
}

impl PhysicsBundle {
    pub(crate) fn player() -> Self {
        Self {
            collider: Collider::rectangle(PLAYER_SIZE, PLAYER_SIZE),
            collider_density: ColliderDensity(1.0),
            rigid_body: RigidBody::Dynamic,
            restitution: Restitution::new(0.0),
            constraint: LockedAxes::new().lock_rotation(),
            dumping: LinearDamping(1.0),
        }
    }
}
