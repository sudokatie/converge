//! Game server networking using renet.

use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

use renet::transport::{NetcodeServerTransport, ServerAuthentication, ServerConfig};
use renet::{ClientId, ConnectionConfig, RenetServer, ServerEvent};
use thiserror::Error;
use tracing::{info, warn};

use crate::protocol::{ClientMessage, ServerMessage};
use crate::transport::channels;

/// Server networking errors.
#[derive(Debug, Error)]
pub enum ServerError {
    #[error("Failed to bind socket: {0}")]
    BindFailed(#[from] std::io::Error),
    
    #[error("Transport error: {0}")]
    Transport(String),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),
}

/// Events from the server.
#[derive(Debug)]
pub enum ServerNetEvent {
    /// Client connected.
    ClientConnected(ClientId),
    /// Client disconnected.
    ClientDisconnected(ClientId),
}

/// Game server handling multiple client connections.
pub struct GameServer {
    server: RenetServer,
    transport: NetcodeServerTransport,
    last_update: Instant,
    client_names: HashMap<ClientId, String>,
}

impl GameServer {
    /// Create a new game server bound to the specified port.
    ///
    /// # Errors
    /// Returns error if socket binding fails.
    pub fn new(port: u16) -> Result<Self, ServerError> {
        let server_addr: SocketAddr = format!("0.0.0.0:{port}").parse().unwrap();
        let socket = UdpSocket::bind(server_addr)?;
        
        let connection_config = ConnectionConfig {
            available_bytes_per_tick: 60_000,
            server_channels_config: channels::channel_configs(),
            client_channels_config: channels::channel_configs(),
        };
        
        let server = RenetServer::new(connection_config);
        
        // Use current time as protocol ID (simple, not cryptographically secure)
        let protocol_id = 0x4C415454; // "LATT" in hex
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap();
        
        let server_config = ServerConfig {
            current_time,
            max_clients: 10,
            protocol_id,
            public_addresses: vec![server_addr],
            authentication: ServerAuthentication::Unsecure,
        };
        
        let transport = NetcodeServerTransport::new(server_config, socket)
            .map_err(|e| ServerError::Transport(e.to_string()))?;
        
        info!("Game server started on port {port}");
        
        Ok(Self {
            server,
            transport,
            last_update: Instant::now(),
            client_names: HashMap::new(),
        })
    }
    
    /// Update server networking. Call every frame.
    pub fn update(&mut self, _dt: Duration) -> Vec<ServerNetEvent> {
        let now = Instant::now();
        let delta = now.duration_since(self.last_update);
        self.last_update = now;
        
        self.server.update(delta);
        
        if let Err(e) = self.transport.update(delta, &mut self.server) {
            warn!("Transport update error: {e}");
        }
        
        let mut events = Vec::new();
        
        // Process connection events
        while let Some(event) = self.server.get_event() {
            match event {
                ServerEvent::ClientConnected { client_id } => {
                    info!("Client {client_id} connected");
                    self.client_names.insert(client_id, format!("Player{client_id}"));
                    events.push(ServerNetEvent::ClientConnected(client_id));
                }
                ServerEvent::ClientDisconnected { client_id, reason } => {
                    info!("Client {client_id} disconnected: {reason}");
                    self.client_names.remove(&client_id);
                    events.push(ServerNetEvent::ClientDisconnected(client_id));
                }
            }
        }
        
        events
    }
    
    /// Send packets over the network.
    pub fn send_packets(&mut self) {
        self.transport.send_packets(&mut self.server);
    }
    
    /// Broadcast a message to all connected clients.
    ///
    /// # Errors
    /// Returns error if serialization fails.
    pub fn broadcast(&mut self, message: &ServerMessage) -> Result<(), ServerError> {
        let data = bincode::serialize(message)?;
        let channel = message_channel(message);
        
        for client_id in self.server.clients_id() {
            self.server.send_message(client_id, channel, data.clone());
        }
        
        Ok(())
    }
    
    /// Send a message to a specific client.
    ///
    /// # Errors
    /// Returns error if serialization fails.
    pub fn send(&mut self, client_id: ClientId, message: &ServerMessage) -> Result<(), ServerError> {
        let data = bincode::serialize(message)?;
        let channel = message_channel(message);
        self.server.send_message(client_id, channel, data);
        Ok(())
    }
    
    /// Receive all pending messages from clients.
    pub fn receive(&mut self) -> Vec<(ClientId, ClientMessage)> {
        let mut messages = Vec::new();
        
        for client_id in self.server.clients_id() {
            // Check all channels
            for channel in [channels::UNRELIABLE, channels::RELIABLE, channels::CHUNK] {
                while let Some(data) = self.server.receive_message(client_id, channel) {
                    match bincode::deserialize(&data) {
                        Ok(msg) => messages.push((client_id, msg)),
                        Err(e) => warn!("Failed to deserialize client message: {e}"),
                    }
                }
            }
        }
        
        messages
    }
    
    /// Get list of connected client IDs.
    pub fn connected_clients(&self) -> Vec<ClientId> {
        self.server.clients_id().into_iter().collect()
    }
    
    /// Check if a client is connected.
    pub fn is_connected(&self, client_id: ClientId) -> bool {
        self.server.is_connected(client_id)
    }
    
    /// Get client display name.
    pub fn client_name(&self, client_id: ClientId) -> Option<&str> {
        self.client_names.get(&client_id).map(String::as_str)
    }
    
    /// Set client display name.
    pub fn set_client_name(&mut self, client_id: ClientId, name: String) {
        self.client_names.insert(client_id, name);
    }
    
    /// Disconnect a client.
    pub fn disconnect(&mut self, client_id: ClientId) {
        self.server.disconnect(client_id);
    }
}

/// Determine which channel to use for a message.
fn message_channel(message: &ServerMessage) -> u8 {
    match message {
        ServerMessage::Snapshot(_) => channels::UNRELIABLE,
        ServerMessage::EntityUpdate { .. } => channels::UNRELIABLE,
        ServerMessage::ChunkData { .. } => channels::CHUNK,
        _ => channels::RELIABLE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn server_creation() {
        // Use a random high port to avoid conflicts
        let server = GameServer::new(0);
        assert!(server.is_ok());
    }
    
    #[test]
    fn message_channel_routing() {
        use crate::protocol::WorldSnapshot;
        use glam::Vec3;
        
        let snapshot = ServerMessage::Snapshot(WorldSnapshot {
            tick: 0,
            ack_sequence: 0,
            player_position: Vec3::ZERO,
            player_velocity: Vec3::ZERO,
            entities: vec![],
        });
        assert_eq!(message_channel(&snapshot), channels::UNRELIABLE);
        
        let chat = ServerMessage::ChatReceive {
            sender: "test".into(),
            message: "hello".into(),
        };
        assert_eq!(message_channel(&chat), channels::RELIABLE);
    }
}
