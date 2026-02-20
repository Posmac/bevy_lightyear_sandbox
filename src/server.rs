use bevy::prelude::*;
use lightyear::{
    netcode::{NetcodeServer, prelude::server},
    prelude::{
        Connected, ControlledBy, InterpolationTarget, LinkOf, LocalAddr, LocalTimeline,
        NetworkTarget, PredictionTarget, RemoteId, Replicate, ReplicationSender, SendUpdatesMode,
        input::native::ActionState,
        server::{ClientOf, ServerUdpIo, Start},
    },
};

use crate::{
    protocol::{Inputs, PlayerPosition},
    shared::{SEND_INTERVAL, SERVER_ADDR, SHARED_SETTINGS, shared_movement_behaviour},
};

pub struct GameServerPlugin;

impl Plugin for GameServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, startup);
        app.add_systems(FixedUpdate, movement);
        app.add_observer(on_link);
        app.add_observer(on_connected);

        // app.add_systems(Update, debug_server_replicate);
    }
}

// fn debug_server_replicate(query: Query<(Entity, &Replicate)>) {
//     for (e, _) in query.iter() {
//         info!("SERVER: Entity {:?} marked for replication", e);
//     }
// }

fn on_link(trigger: On<Add, LinkOf>, mut commands: Commands) {
    info!(
        "Incoming UDP packet → spawned LinkOf entity: {:?}",
        trigger.entity
    );
    commands.entity(trigger.entity).insert((
        ReplicationSender::new(SEND_INTERVAL, SendUpdatesMode::SinceLastAck, false),
        Name::from("Client"),
    ));
}

fn on_connected(
    trigger: On<Add, Connected>,
    query: Query<&RemoteId, With<ClientOf>>,
    mut commands: Commands,
) {
    info!(
        "Handshake complete → client fully connected: {:?}",
        trigger.entity
    );

    let Ok(client_id) = query.get(trigger.entity) else {
        return;
    };

    let client_id = client_id.0;

    let entity = commands
        .spawn((
            PlayerPosition(Vec2::ZERO),
            Replicate::to_clients(NetworkTarget::All),
            PredictionTarget::to_clients(NetworkTarget::Single(client_id)),
            InterpolationTarget::to_clients(NetworkTarget::AllExceptSingle(client_id)),
            ControlledBy {
                owner: trigger.entity,
                lifetime: Default::default(),
            },
            // ActionState::<Inputs>::default(),
        ))
        .id();

    info!(
        "Created player entity {:?} for client {:?}",
        entity, client_id
    );
}

pub fn startup(mut commands: Commands) {
    info!("Server created");
    let server = commands
        .spawn((
            NetcodeServer::new(server::NetcodeConfig {
                protocol_id: SHARED_SETTINGS.protocol_id,
                private_key: SHARED_SETTINGS.private_key,
                ..Default::default()
            }),
            LocalAddr(SERVER_ADDR),
            ServerUdpIo::default(),
        ))
        .id();
    commands.trigger(Start { entity: server });
}

/// Read client inputs and move players in server therefore giving a basis for other clients
fn movement(
    timeline: Res<LocalTimeline>,
    mut position_query: Query<(&mut PlayerPosition, &ActionState<Inputs>)>,
) {
    let tick = timeline.tick();
    for (position, inputs) in position_query.iter_mut() {
        trace!(?tick, ?position, ?inputs, "server");
        shared_movement_behaviour(position, inputs);
    }
}
