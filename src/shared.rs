use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::Duration,
};

use bevy::prelude::*;

use crate::protocol::{
    AnimationConfig, Inputs, PlayerAnimations, PlayerPosition, PlayerState, PlayerStateEnum,
    ProtocolPlugin,
};

pub const FIXED_TIMESTEP_HZ: f64 = 64.0;
pub const SERVER_PORT: u16 = 5888;
/// 0 means that the OS will assign any available port
pub const CLIENT_PORT: u16 = 0;
pub const SERVER_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), SERVER_PORT);
pub const LOCAL_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), CLIENT_PORT);
pub const SEND_INTERVAL: Duration = Duration::from_millis(50);
pub const SHARED_SETTINGS: SharedSettings = SharedSettings {
    protocol_id: 1997,
    private_key: [0; 32],
};

#[derive(Copy, Clone, Debug)]
pub struct SharedSettings {
    /// An id to identify the protocol version
    pub protocol_id: u64,

    /// a 32-byte array to authenticate via the Netcode.io protocol
    pub private_key: [u8; 32],
}

pub struct SharedPlugin;

impl Plugin for SharedPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ProtocolPlugin);
        app.add_systems(Startup, load_resources);
    }
}

#[derive(Debug, Resource, Clone, PartialEq, Reflect)]
pub struct PlayerSpriteSheetResource {
    pub player_image: Handle<Image>,
    pub atlas: Handle<TextureAtlasLayout>,
}

#[derive(Debug, Component, Clone, PartialEq, Reflect)]
pub struct PlayerAnimationTimer {
    pub fps: u8,
    pub frame_timer: Timer,
}

impl PlayerAnimationTimer {
    pub fn new(fps: u8) -> Self {
        Self {
            fps,
            frame_timer: Timer::new(
                Duration::from_secs_f32(1.0 / (fps as f32)),
                TimerMode::Repeating,
            ),
        }
    }
}

// This system defines how we update the player's positions when we receive an input
pub fn shared_movement_behaviour(mut position: Mut<PlayerPosition>, input: &Inputs) {
    const MOVE_SPEED: f32 = 10.0;
    let Inputs::Direction(direction) = input;
    if direction.back {
        position.y += MOVE_SPEED;
    }
    if direction.front {
        position.y -= MOVE_SPEED;
    }
    if direction.left {
        position.x -= MOVE_SPEED;
    }
    if direction.right {
        position.x += MOVE_SPEED;
    }
}

pub fn shared_animation_behaviour(
    mut player_state: Mut<PlayerState>,
    // mut player_animations: Mut<PlayerAnimations>,
    input: &Inputs,
) {
    let Inputs::Direction(direction) = input;

    player_state.prev_state = player_state.current_state.clone();
    if direction.is_none() {
        if player_state.current_state.is_walking() {
            let inverse_state = player_state.current_state.get_opposite_state();
            // let inverse_animation = player_animations.get_anim(&inverse_state);
            player_state.current_state = inverse_state;
            // player_animations.current_animation = inverse_animation;
        }
        return;
    }

    if direction.front {
        player_state.current_state = PlayerStateEnum::WalkingFront;
        // player_animations.current_animation = player_animations.move_front;
    }
    if direction.back {
        player_state.current_state = PlayerStateEnum::WalkingBack;
        // player_animations.current_animation = player_animations.move_back;
    }
    if direction.left {
        player_state.current_state = PlayerStateEnum::WalkingLeft;
        // player_animations.current_animation = player_animations.move_left;
    }
    if direction.right {
        player_state.current_state = PlayerStateEnum::WalkingRight;
        // player_animations.current_animation = player_animations.move_right;
    }
}

pub fn load_resources(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let character_texture = asset_server.load("sprout/Characters/basic_movement.png");
    let sprite_sheet_layout = TextureAtlasLayout::from_grid(UVec2::new(48, 48), 4, 4, None, None);
    let texture_atlas_layout = texture_atlas_layouts.add(sprite_sheet_layout);

    commands.insert_resource(PlayerSpriteSheetResource {
        player_image: character_texture,
        atlas: texture_atlas_layout,
    });
}

pub fn get_player_anim_config() -> PlayerAnimations {
    let character_animation_config = PlayerAnimations::new(
        AnimationConfig {
            first_sprite_index: 0,
            last_sprite_index: 1,
        },
        AnimationConfig {
            first_sprite_index: 4,
            last_sprite_index: 5,
        },
        AnimationConfig {
            first_sprite_index: 8,
            last_sprite_index: 9,
        },
        AnimationConfig {
            first_sprite_index: 12,
            last_sprite_index: 13,
        },
        AnimationConfig {
            first_sprite_index: 2,
            last_sprite_index: 3,
        },
        AnimationConfig {
            first_sprite_index: 6,
            last_sprite_index: 7,
        },
        AnimationConfig {
            first_sprite_index: 10,
            last_sprite_index: 11,
        },
        AnimationConfig {
            first_sprite_index: 14,
            last_sprite_index: 15,
        },
    );

    character_animation_config
}
