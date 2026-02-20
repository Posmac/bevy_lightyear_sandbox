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
    }
}

impl Ease for PlayerPosition {
    fn interpolating_curve_unbounded(start: Self, end: Self) -> impl Curve<Self> {
        FunctionCurve::new(Interval::UNIT, move |t| {
            PlayerPosition(Vec2::lerp(start.0, end.0, t))
        })
    }
}

#[derive(Debug, Component, Serialize, Deserialize, Clone, PartialEq, Reflect, Deref, DerefMut)]
pub struct PlayerPosition(pub Vec2);

#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Eq, Clone, Reflect)]
pub struct Direction {
    pub(crate) up: bool,
    pub(crate) down: bool,
    pub(crate) left: bool,
    pub(crate) right: bool,
}

impl Direction {
    pub(crate) fn is_none(&self) -> bool {
        !self.up && !self.down && !self.left && !self.right
    }
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
