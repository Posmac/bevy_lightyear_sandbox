use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::Duration,
};

use bevy::prelude::*;

use crate::protocol::{Inputs, PlayerPosition, ProtocolPlugin};

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
    }
}

// This system defines how we update the player's positions when we receive an input
pub fn shared_movement_behaviour(mut position: Mut<PlayerPosition>, input: &Inputs) {
    const MOVE_SPEED: f32 = 10.0;
    let Inputs::Direction(direction) = input;
    if direction.up {
        position.y += MOVE_SPEED;
    }
    if direction.down {
        position.y -= MOVE_SPEED;
    }
    if direction.left {
        position.x -= MOVE_SPEED;
    }
    if direction.right {
        position.x += MOVE_SPEED;
    }
}
