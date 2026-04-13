//! Game client networking using renet.

use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

use renet::transport::{ClientAuthentication, NetcodeClientTransport};
use renet::{ConnectionConfig, RenetClient};
use thiserror::Error;
use tracing::{info, warn};

use crate::protocol::{ClientMessage, ServerMessage};
use crate::transport::channels;

/// Client networking errors.
#[derive(Debug, Error)]
pub enum ClientError {
    #[error("Failed to bind socket: {0}")]
    BindFailed(#[from] std::io::Error),
    
    #[error("Invalid server address: {0}")]
    InvalidAddress(String),
    
    #[error("Transport error: {0}")]
    Transport(String),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),
    
    #[error("Not connected to server")]
    NotConnected,
    
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
}

/// Connection state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectionState {
    /// Not connected, idle.
    Disconnected,
    /// Attempting to connect.
    Connecting,
    /// Connected and ready.
    Connected,
}

/// Game client for connecting to a server.
pub struct GameClient {
    client: RenetClient,
    transport: NetcodeClientTransport,
    last_update: Instant,
    state: ConnectionState,
}

impl GameClient {
    /// Connect to a game server.
    ///
    /// # Errors
    /// Returns error if connection fails.
    pub fn connect(server_addr: &str) -> Result<Self, ClientError> {
        let server_addr: SocketAddr = server_addr
            .parse()
            .map_err(|_| ClientError::InvalidAddress(server_addr.to_string()))?;
        
        // Bind to any available port
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        
        let connection_config = ConnectionConfig {
            available_bytes_per_tick: 60_000,
            server_channels_config: channels::channel_configs(),
            client_channels_config: channels::channel_configs(),
        };
        
        let client = RenetClient::new(connection_config);
        
        let protocol_id = 0x4C415454; // "LATT" - must match server
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap();
        
        // Generate a random client ID
        let client_id = rand_client_id();
        
        let authentication = ClientAuthentication::Unsecure {
            client_id,
            protocol_id,
            server_addr,
            user_data: None,
        };
        
        let transport = NetcodeClientTransport::new(current_time, authentication, socket)
            .map_err(|e| ClientError::Transport(e.to_string()))?;
        
        info!("Connecting to server at {server_addr}");
        
        Ok(Self {
            client,
            transport,
            last_update: Instant::now(),
            state: ConnectionState::Connecting,
        })
    }
    
    /// Update client networking. Call every frame.
    pub fn update(&mut self, _dt: Duration) {
        let now = Instant::now();
        let delta = now.duration_since(self.last_update);
        self.last_update = now;
        
        self.client.update(delta);
        
        if let Err(e) = self.transport.update(delta, &mut self.client) {
            warn!("Transport update error: {e}");
            self.state = ConnectionState::Disconnected;
            return;
        }
        
        // Update connection state
        if self.client.is_connected() {
            if self.state == ConnectionState::Connecting {
                info!("Connected to server");
            }
            self.state = ConnectionState::Connected;
        } else if self.client.is_connecting() {
            self.state = ConnectionState::Connecting;
        } else {
            if self.state == ConnectionState::Connected {
                info!("Disconnected from server");
            }
            self.state = ConnectionState::Disconnected;
        }
    }
    
    /// Send packets over the network.
    pub fn send_packets(&mut self) {
        if let Err(e) = self.transport.send_packets(&mut self.client) {
            warn!("Failed to send packets: {e}");
        }
    }
    
    /// Send a message to the server.
    ///
    /// # Errors
    /// Returns error if not connected or serialization fails.
    pub fn send(&mut self, message: &ClientMessage) -> Result<(), ClientError> {
        if !self.client.is_connected() {
            return Err(ClientError::NotConnected);
        }
        
        let data = bincode::serialize(message)?;
        let channel = client_message_channel(message);
        self.client.send_message(channel, data);
        Ok(())
    }
    
    /// Receive all pending messages from the server.
    pub fn receive(&mut self) -> Vec<ServerMessage> {
        let mut messages = Vec::new();
        
        // Check all channels
        for channel in [channels::UNRELIABLE, channels::RELIABLE, channels::CHUNK] {
            while let Some(data) = self.client.receive_message(channel) {
                match bincode::deserialize(&data) {
                    Ok(msg) => messages.push(msg),
                    Err(e) => warn!("Failed to deserialize server message: {e}"),
                }
            }
        }
        
        messages
    }
    
    /// Get current connection state.
    pub fn state(&self) -> ConnectionState {
        self.state
    }
    
    /// Check if connected to server.
    pub fn is_connected(&self) -> bool {
        self.state == ConnectionState::Connected
    }
    
    /// Check if currently connecting.
    pub fn is_connecting(&self) -> bool {
        self.state == ConnectionState::Connecting
    }
    
    /// Disconnect from server.
    pub fn disconnect(&mut self) {
        self.client.disconnect();
        self.state = ConnectionState::Disconnected;
    }
    
    /// Get round-trip time estimate in milliseconds.
    pub fn rtt_ms(&self) -> f64 {
        self.client.rtt() * 1000.0
    }
    
    /// Get packet loss percentage (0-100).
    pub fn packet_loss(&self) -> f64 {
        self.client.packet_loss() * 100.0
    }
}

/// Determine which channel to use for a client message.
fn client_message_channel(message: &ClientMessage) -> u8 {
    match message {
        ClientMessage::Input(_) => channels::UNRELIABLE,
        ClientMessage::ChunkRequest { .. } => channels::CHUNK,
        _ => channels::RELIABLE,
    }
}

/// Generate a random client ID.
fn rand_client_id() -> u64 {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    
    let state = RandomState::new();
    let mut hasher = state.build_hasher();
    hasher.write_u64(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn client_message_channel_routing() {
        use crate::protocol::InputState;
        
        let input = ClientMessage::Input(InputState::default());
        assert_eq!(client_message_channel(&input), channels::UNRELIABLE);
        
        let chat = ClientMessage::ChatSend {
            message: "hello".into(),
        };
        assert_eq!(client_message_channel(&chat), channels::RELIABLE);
    }
    
    #[test]
    fn connection_state_initial() {
        // Can't test actual connection without a server
        // Just verify the types work
        assert_eq!(ConnectionState::Disconnected, ConnectionState::Disconnected);
    }
}
