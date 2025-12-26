//! P2P networking using libp2p
//!
//! Provides peer discovery (mDNS + DHT), file transfer,
//! and encrypted communications using QUIC.

mod behaviour;
mod protocol;

use crate::{config::Config, BridgeError, Result};
use futures::StreamExt;
use libp2p::{
    identify, mdns, noise, ping, swarm::SwarmEvent, tcp,
    yamux, Multiaddr, PeerId, Swarm, SwarmBuilder,
};
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

pub use behaviour::BridgeBehaviour;
pub use protocol::{FileRequest, FileResponse, TransferProtocol};

/// Events emitted by the peer network
#[derive(Debug, Clone)]
pub enum NetworkEvent {
    /// A new peer was discovered
    PeerDiscovered { peer_id: PeerId, addresses: Vec<Multiaddr> },

    /// A peer disconnected
    PeerDisconnected { peer_id: PeerId },

    /// Received a file request from a peer
    FileRequested { peer_id: PeerId, request: FileRequest },

    /// File transfer completed
    TransferComplete { peer_id: PeerId, hash: String },

    /// Network is ready
    Ready { local_peer_id: PeerId, listening_on: Vec<Multiaddr> },
}

/// Peer information
#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub peer_id: PeerId,
    pub addresses: Vec<Multiaddr>,
    pub device_name: Option<String>,
    pub last_seen: std::time::Instant,
}

/// Main P2P network handler
pub struct PeerNetwork {
    swarm: Swarm<BridgeBehaviour>,
    local_peer_id: PeerId,
    peers: HashMap<PeerId, PeerInfo>,
    event_tx: mpsc::Sender<NetworkEvent>,
    event_rx: mpsc::Receiver<NetworkEvent>,
    config: Config,
}

impl PeerNetwork {
    /// Create a new peer network
    pub async fn new(config: &Config) -> Result<Self> {
        let (event_tx, event_rx) = mpsc::channel(256);

        // Build the swarm with TCP transport
        let config_clone = config.clone();
        let swarm = SwarmBuilder::with_new_identity()
            .with_tokio()
            .with_tcp(
                tcp::Config::default(),
                noise::Config::new,
                yamux::Config::default,
            )
            .map_err(|e| BridgeError::Network(e.to_string()))?
            .with_behaviour(|key| {
                let local_peer_id = key.public().to_peer_id();
                Ok(BridgeBehaviour::new(key.clone(), local_peer_id, &config_clone))
            })
            .map_err(|e| BridgeError::Network(format!("{:?}", e)))?
            .with_swarm_config(|cfg| {
                cfg.with_idle_connection_timeout(Duration::from_secs(60))
            })
            .build();

        let local_peer_id = *swarm.local_peer_id();
        info!("Local peer ID: {}", local_peer_id);

        Ok(Self {
            swarm,
            local_peer_id,
            peers: HashMap::new(),
            event_tx,
            event_rx,
            config: config.clone(),
        })
    }

    /// Get the local peer ID
    pub fn local_peer_id(&self) -> &PeerId {
        &self.local_peer_id
    }

    /// Get connected peers
    pub fn peers(&self) -> &HashMap<PeerId, PeerInfo> {
        &self.peers
    }

    /// Start the network (listen on configured ports)
    pub async fn start(&mut self) -> Result<()> {
        // Listen on QUIC
        if self.config.network.enable_quic {
            let quic_addr: Multiaddr = format!(
                "/ip4/0.0.0.0/udp/{}/quic-v1",
                self.config.network.listen_port
            )
            .parse()
            .map_err(|e: libp2p::multiaddr::Error| BridgeError::Network(e.to_string()))?;

            self.swarm
                .listen_on(quic_addr.clone())
                .map_err(|e| BridgeError::Network(e.to_string()))?;

            info!("Listening on QUIC: {}", quic_addr);
        }

        // Listen on TCP (fallback)
        if self.config.network.enable_tcp {
            let tcp_addr: Multiaddr = format!(
                "/ip4/0.0.0.0/tcp/{}",
                self.config.network.listen_port
            )
            .parse()
            .map_err(|e: libp2p::multiaddr::Error| BridgeError::Network(e.to_string()))?;

            self.swarm
                .listen_on(tcp_addr.clone())
                .map_err(|e| BridgeError::Network(e.to_string()))?;

            info!("Listening on TCP: {}", tcp_addr);
        }

        Ok(())
    }

    /// Stop the network
    pub async fn stop(&mut self) -> Result<()> {
        info!("Stopping P2P network");
        Ok(())
    }

    /// Poll for network events (call this in your event loop)
    pub async fn poll(&mut self) -> Option<NetworkEvent> {
        tokio::select! {
            event = self.swarm.select_next_some() => {
                self.handle_swarm_event(event).await
            }
            event = self.event_rx.recv() => {
                event
            }
        }
    }

    /// Handle a swarm event
    async fn handle_swarm_event(&mut self, event: SwarmEvent<behaviour::BridgeBehaviourEvent>) -> Option<NetworkEvent> {
        match event {
            SwarmEvent::NewListenAddr { address, .. } => {
                info!("Listening on {}", address);
                None
            }

            SwarmEvent::Behaviour(behaviour::BridgeBehaviourEvent::Mdns(mdns::Event::Discovered(peers))) => {
                for (peer_id, addr) in peers {
                    info!("mDNS discovered peer: {} at {}", peer_id, addr);
                    self.swarm.dial(addr.clone()).ok();

                    let info = self.peers.entry(peer_id).or_insert_with(|| PeerInfo {
                        peer_id,
                        addresses: vec![],
                        device_name: None,
                        last_seen: std::time::Instant::now(),
                    });
                    info.addresses.push(addr.clone());
                    info.last_seen = std::time::Instant::now();

                    return Some(NetworkEvent::PeerDiscovered {
                        peer_id,
                        addresses: vec![addr],
                    });
                }
                None
            }

            SwarmEvent::Behaviour(behaviour::BridgeBehaviourEvent::Mdns(mdns::Event::Expired(peers))) => {
                for (peer_id, _addr) in peers {
                    debug!("mDNS peer expired: {}", peer_id);
                }
                None
            }

            SwarmEvent::Behaviour(behaviour::BridgeBehaviourEvent::Identify(identify::Event::Received { peer_id, info, .. })) => {
                debug!("Identified peer {}: {:?}", peer_id, info.agent_version);

                if let Some(peer_info) = self.peers.get_mut(&peer_id) {
                    peer_info.device_name = Some(info.agent_version.clone());
                    peer_info.last_seen = std::time::Instant::now();
                }
                None
            }

            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                info!("Connected to peer: {}", peer_id);
                None
            }

            SwarmEvent::ConnectionClosed { peer_id, .. } => {
                info!("Disconnected from peer: {}", peer_id);
                self.peers.remove(&peer_id);
                Some(NetworkEvent::PeerDisconnected { peer_id })
            }

            SwarmEvent::Behaviour(behaviour::BridgeBehaviourEvent::Ping(ping::Event { peer, result, .. })) => {
                match result {
                    Ok(rtt) => debug!("Ping to {}: {:?}", peer, rtt),
                    Err(e) => warn!("Ping to {} failed: {}", peer, e),
                }
                None
            }

            _ => None,
        }
    }

    /// Connect to a peer by address
    pub fn dial(&mut self, addr: Multiaddr) -> Result<()> {
        self.swarm
            .dial(addr)
            .map_err(|e| BridgeError::Network(e.to_string()))?;
        Ok(())
    }

    /// Connect to a peer by peer ID
    pub fn dial_peer(&mut self, peer_id: PeerId) -> Result<()> {
        if let Some(info) = self.peers.get(&peer_id) {
            if let Some(addr) = info.addresses.first() {
                return self.dial(addr.clone());
            }
        }
        Err(BridgeError::Network(format!(
            "No known address for peer {}",
            peer_id
        )))
    }

    /// Send a file to a peer
    pub async fn send_file(&mut self, _peer_id: PeerId, _hash: &str, _data: Vec<u8>) -> Result<()> {
        // TODO: Implement file transfer protocol
        Ok(())
    }

    /// Request a file from a peer
    pub async fn request_file(&mut self, _peer_id: PeerId, _hash: &str) -> Result<Vec<u8>> {
        // TODO: Implement file request protocol
        Err(BridgeError::Network("Not implemented".to_string()))
    }
}
