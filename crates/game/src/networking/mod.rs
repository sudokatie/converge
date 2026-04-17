//! Client-side networking for multiplayer.

pub mod commands;

use std::time::Duration;

use engine_network::{
    ClientMessage, GameClient, ServerMessage, WorldSnapshot,
    protocol::InputState,
};
use glam::Vec3;
use tracing::{debug, info, warn};

/// Client network state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NetworkState {
    /// Disconnected from server.
    Disconnected,
    /// Attempting to connect.
    Connecting,
    /// Connected and playing.
    Connected,
}

/// Server info received on connect.
#[derive(Clone, Debug)]
pub struct ServerInfo {
    /// Our assigned player ID.
    pub player_id: u64,
    /// Server tick rate.
    pub tick_rate: u32,
    /// World seed.
    pub seed: u64,
}

/// Network client for game.
pub struct NetworkClient {
    /// Underlying network client.
    client: Option<GameClient>,
    /// Current state.
    state: NetworkState,
    /// Server info (after welcome).
    server_info: Option<ServerInfo>,
    /// Current input sequence.
    input_sequence: u32,
    /// Last received snapshot.
    last_snapshot: Option<WorldSnapshot>,
    /// Pending events to process.
    pending_events: Vec<NetworkEvent>,
}

/// Events from the network.
#[derive(Clone, Debug)]
pub enum NetworkEvent {
    /// Connected to server.
    Connected(ServerInfo),
    /// Disconnected from server.
    Disconnected,
    /// Player joined.
    PlayerJoined { id: u64, name: String },
    /// Player left.
    PlayerLeft { id: u64 },
    /// Chat message received.
    ChatReceived { sender: String, message: String },
    /// Block changed in world.
    BlockChanged { pos: (i32, i32, i32), block: u16 },
}

impl NetworkClient {
    /// Create a new network client (not connected).
    pub fn new() -> Self {
        Self {
            client: None,
            state: NetworkState::Disconnected,
            server_info: None,
            input_sequence: 0,
            last_snapshot: None,
            pending_events: Vec::new(),
        }
    }
    
    /// Connect to a server.
    pub fn connect(&mut self, address: &str) -> Result<(), String> {
        match GameClient::connect(address) {
            Ok(client) => {
                self.client = Some(client);
                self.state = NetworkState::Connecting;
                info!("Connecting to {address}");
                Ok(())
            }
            Err(e) => {
                warn!("Failed to connect: {e}");
                Err(e.to_string())
            }
        }
    }
    
    /// Disconnect from server.
    pub fn disconnect(&mut self) {
        if let Some(ref mut client) = self.client {
            client.disconnect();
        }
        self.client = None;
        self.state = NetworkState::Disconnected;
        self.server_info = None;
        self.last_snapshot = None;
        self.pending_events.push(NetworkEvent::Disconnected);
    }
    
    /// Update networking. Call every frame.
    pub fn update(&mut self, dt: Duration) {
        let Some(ref mut client) = self.client else {
            return;
        };
        
        client.update(dt);
        
        // Update state
        if client.is_connected() {
            if self.state == NetworkState::Connecting {
                // Wait for welcome message to confirm connection
            }
        } else if !client.is_connecting() {
            if self.state != NetworkState::Disconnected {
                self.state = NetworkState::Disconnected;
                self.pending_events.push(NetworkEvent::Disconnected);
            }
        }
        
        // Collect messages first to avoid borrow issues
        let messages: Vec<_> = client.receive();
        
        // Send pending packets
        client.send_packets();
        
        // Process received messages
        for message in messages {
            self.process_message(message);
        }
    }
    
    /// Process a message from the server.
    fn process_message(&mut self, message: ServerMessage) {
        match message {
            ServerMessage::Welcome { player_id, tick_rate, seed } => {
                let info = ServerInfo { player_id, tick_rate, seed };
                self.server_info = Some(info.clone());
                self.state = NetworkState::Connected;
                self.pending_events.push(NetworkEvent::Connected(info));
                info!("Connected to server, player_id: {player_id}");
            }
            
            ServerMessage::Snapshot(snapshot) => {
                self.last_snapshot = Some(snapshot);
            }
            
            ServerMessage::PlayerJoined { id, name } => {
                self.pending_events.push(NetworkEvent::PlayerJoined { id, name });
            }
            
            ServerMessage::PlayerLeft { id } => {
                self.pending_events.push(NetworkEvent::PlayerLeft { id });
            }
            
            ServerMessage::ChatReceive { sender, message } => {
                self.pending_events.push(NetworkEvent::ChatReceived { sender, message });
            }
            
            ServerMessage::BlockChange { pos, block } => {
                self.pending_events.push(NetworkEvent::BlockChanged {
                    pos: (pos.0.x, pos.0.y, pos.0.z),
                    block,
                });
            }
            
            ServerMessage::EntitySpawn { .. } |
            ServerMessage::EntityDespawn { .. } |
            ServerMessage::EntityUpdate { .. } |
            ServerMessage::ChunkData { .. } => {
                // Handled elsewhere
                debug!("Received entity/chunk message");
            }
        }
    }
    
    /// Send player input to server.
    pub fn send_input(
        &mut self,
        movement: Vec3,
        jump: bool,
        sprint: bool,
        yaw: f32,
        pitch: f32,
    ) {
        let Some(ref mut client) = self.client else {
            return;
        };
        
        if !client.is_connected() {
            return;
        }
        
        self.input_sequence = self.input_sequence.wrapping_add(1);
        
        let input = InputState {
            movement,
            jump,
            sprint,
            yaw,
            pitch,
            sequence: self.input_sequence,
        };
        
        if let Err(e) = client.send(&ClientMessage::Input(input)) {
            debug!("Failed to send input: {e}");
        }
    }
    
    /// Send chat message.
    pub fn send_chat(&mut self, message: String) {
        let Some(ref mut client) = self.client else {
            return;
        };
        
        if let Err(e) = client.send(&ClientMessage::ChatSend { message }) {
            warn!("Failed to send chat: {e}");
        }
    }
    
    /// Request a chunk from the server.
    pub fn request_chunk(&mut self, x: i32, y: i32, z: i32) {
        use engine_core::coords::ChunkPos;
        use glam::IVec3;
        
        let Some(ref mut client) = self.client else {
            return;
        };
        
        let pos = ChunkPos(IVec3::new(x, y, z));
        if let Err(e) = client.send(&ClientMessage::ChunkRequest { pos }) {
            debug!("Failed to request chunk: {e}");
        }
    }
    
    /// Get current network state.
    pub fn state(&self) -> NetworkState {
        self.state
    }
    
    /// Check if connected.
    pub fn is_connected(&self) -> bool {
        self.state == NetworkState::Connected
    }
    
    /// Get server info (if connected).
    pub fn server_info(&self) -> Option<&ServerInfo> {
        self.server_info.as_ref()
    }
    
    /// Get last received snapshot.
    pub fn last_snapshot(&self) -> Option<&WorldSnapshot> {
        self.last_snapshot.as_ref()
    }
    
    /// Get and clear pending events.
    pub fn drain_events(&mut self) -> Vec<NetworkEvent> {
        std::mem::take(&mut self.pending_events)
    }
    
    /// Get RTT in milliseconds.
    pub fn rtt_ms(&self) -> f64 {
        self.client.as_ref().map_or(0.0, |c| c.rtt_ms())
    }
    
    /// Get packet loss percentage.
    pub fn packet_loss(&self) -> f64 {
        self.client.as_ref().map_or(0.0, |c| c.packet_loss())
    }
}

impl Default for NetworkClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn network_client_initial_state() {
        let client = NetworkClient::new();
        assert_eq!(client.state(), NetworkState::Disconnected);
        assert!(!client.is_connected());
        assert!(client.server_info().is_none());
    }
    
    #[test]
    fn network_state_enum() {
        assert_ne!(NetworkState::Connected, NetworkState::Disconnected);
        assert_ne!(NetworkState::Connecting, NetworkState::Connected);
    }
}
