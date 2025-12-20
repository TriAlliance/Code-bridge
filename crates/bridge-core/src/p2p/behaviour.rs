//! Custom libp2p behaviour combining multiple protocols

use crate::config::Config;
use libp2p::{
    identify, kad, mdns, ping,
    request_response::{self, ProtocolSupport},
    swarm::NetworkBehaviour,
    PeerId,
};
use std::time::Duration;

use super::protocol::TransferProtocol;

/// Combined network behaviour for Code Bridge
#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "BridgeBehaviourEvent")]
pub struct BridgeBehaviour {
    /// Ping protocol for connection liveness
    pub ping: ping::Behaviour,

    /// Identify protocol for peer information exchange
    pub identify: identify::Behaviour,

    /// mDNS for local network discovery
    pub mdns: mdns::tokio::Behaviour,

    /// Kademlia DHT for global discovery
    pub kademlia: kad::Behaviour<kad::store::MemoryStore>,

    /// Request-response for file transfers
    pub transfer: request_response::Behaviour<TransferProtocol>,
}

impl BridgeBehaviour {
    pub fn new(
        keypair: libp2p::identity::Keypair,
        local_peer_id: PeerId,
        config: &Config,
    ) -> Self {
        // Ping behaviour
        let ping = ping::Behaviour::new(ping::Config::new().with_interval(Duration::from_secs(15)));

        // Identify behaviour
        let identify = identify::Behaviour::new(identify::Config::new(
            "/codebridge/1.0.0".to_string(),
            keypair.public(),
        ));

        // mDNS for local discovery
        let mdns = mdns::tokio::Behaviour::new(
            mdns::Config::default(),
            local_peer_id,
        )
        .expect("Failed to create mDNS behaviour");

        // Kademlia DHT
        let store = kad::store::MemoryStore::new(local_peer_id);
        let mut kademlia = kad::Behaviour::new(local_peer_id, store);

        // Add bootstrap peers
        for peer_addr in &config.network.bootstrap_peers {
            if let Ok(addr) = peer_addr.parse() {
                kademlia.add_address(&local_peer_id, addr);
            }
        }

        // Request-response for file transfers
        let transfer = request_response::Behaviour::new(
            [(TransferProtocol, ProtocolSupport::Full)],
            request_response::Config::default(),
        );

        Self {
            ping,
            identify,
            mdns,
            kademlia,
            transfer,
        }
    }
}

/// Events emitted by the combined behaviour
#[derive(Debug)]
pub enum BridgeBehaviourEvent {
    Ping(ping::Event),
    Identify(identify::Event),
    Mdns(mdns::Event),
    Kademlia(kad::Event),
    Transfer(request_response::Event<super::protocol::FileRequest, super::protocol::FileResponse>),
}

impl From<ping::Event> for BridgeBehaviourEvent {
    fn from(event: ping::Event) -> Self {
        BridgeBehaviourEvent::Ping(event)
    }
}

impl From<identify::Event> for BridgeBehaviourEvent {
    fn from(event: identify::Event) -> Self {
        BridgeBehaviourEvent::Identify(event)
    }
}

impl From<mdns::Event> for BridgeBehaviourEvent {
    fn from(event: mdns::Event) -> Self {
        BridgeBehaviourEvent::Mdns(event)
    }
}

impl From<kad::Event> for BridgeBehaviourEvent {
    fn from(event: kad::Event) -> Self {
        BridgeBehaviourEvent::Kademlia(event)
    }
}

impl From<request_response::Event<super::protocol::FileRequest, super::protocol::FileResponse>>
    for BridgeBehaviourEvent
{
    fn from(
        event: request_response::Event<super::protocol::FileRequest, super::protocol::FileResponse>,
    ) -> Self {
        BridgeBehaviourEvent::Transfer(event)
    }
}
