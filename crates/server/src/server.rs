//! Game server with fixed tick rate simulation.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use anyhow::Result;
use engine_network::{
    ClientId, ClientMessage, EntityKind, GameServer, ServerMessage, WorldSnapshot,
};
use glam::{Quat, Vec3};
use tracing::{debug, info, warn};

/// Server tick rate in Hz.
pub const TICK_RATE: u32 = 20;

/// Duration of one tick.
pub const TICK_DURATION: Duration = Duration::from_millis(1000 / TICK_RATE as u64);

/// Connected player state.
#[derive(Debug)]
pub struct PlayerState {
    /// Player's entity ID.
    pub entity_id: u64,
    /// Player's position.
    pub position: Vec3,
    /// Player's velocity.
    pub velocity: Vec3,
    /// Player's rotation (yaw, pitch).
    pub yaw: f32,
    pub pitch: f32,
    /// Last acknowledged input sequence.
    pub last_ack_sequence: u32,
    /// Display name.
    pub name: String,
}

impl PlayerState {
    fn new(entity_id: u64, spawn_pos: Vec3, name: String) -> Self {
        Self {
            entity_id,
            position: spawn_pos,
            velocity: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            last_ack_sequence: 0,
            name,
        }
    }
}

/// Game server managing the world simulation.
pub struct LatticeServer {
    /// Network transport.
    network: GameServer,
    /// Connected players.
    players: HashMap<ClientId, PlayerState>,
    /// Current server tick.
    tick: u64,
    /// Next entity ID.
    next_entity_id: u64,
    /// World seed.
    seed: u64,
    /// Spawn position.
    spawn_pos: Vec3,
    /// Running state.
    running: bool,
}

impl LatticeServer {
    /// Create a new game server.
    ///
    /// # Errors
    /// Returns error if network binding fails.
    pub fn new(port: u16, seed: u64) -> Result<Self> {
        let network = GameServer::new(port)?;
        
        Ok(Self {
            network,
            players: HashMap::new(),
            tick: 0,
            next_entity_id: 1,
            seed,
            spawn_pos: Vec3::new(0.0, 64.0, 0.0),
            running: true,
        })
    }
    
    /// Run the server game loop.
    pub fn run(&mut self) -> Result<()> {
        info!("Server running at {} Hz", TICK_RATE);
        
        let mut last_tick = Instant::now();
        let mut accumulator = Duration::ZERO;
        
        while self.running {
            let now = Instant::now();
            let frame_time = now.duration_since(last_tick);
            last_tick = now;
            
            // Cap frame time to prevent spiral of death
            let frame_time = frame_time.min(Duration::from_millis(250));
            accumulator += frame_time;
            
            // Fixed timestep simulation
            let mut ticks_this_frame = 0;
            while accumulator >= TICK_DURATION && ticks_this_frame < 4 {
                self.tick();
                accumulator -= TICK_DURATION;
                ticks_this_frame += 1;
            }
            
            // Sleep to avoid busy-waiting
            if accumulator < TICK_DURATION {
                let sleep_time = TICK_DURATION - accumulator;
                std::thread::sleep(sleep_time.min(Duration::from_millis(5)));
            }
        }
        
        info!("Server shutting down");
        Ok(())
    }
    
    /// Run a single server tick.
    fn tick(&mut self) {
        self.tick += 1;
        
        // Update network
        let events = self.network.update(TICK_DURATION);
        
        // Handle connection events
        for event in events {
            match event {
                engine_network::transport::server::ServerNetEvent::ClientConnected(client_id) => {
                    self.on_client_connected(client_id);
                }
                engine_network::transport::server::ServerNetEvent::ClientDisconnected(client_id) => {
                    self.on_client_disconnected(client_id);
                }
            }
        }
        
        // Process client messages
        let messages = self.network.receive();
        for (client_id, message) in messages {
            self.process_message(client_id, message);
        }
        
        // Simulate world (placeholder - would integrate with engine_world)
        self.simulate();
        
        // Broadcast snapshots to all clients
        self.broadcast_snapshots();
        
        // Send packets
        self.network.send_packets();
    }
    
    /// Handle client connection.
    fn on_client_connected(&mut self, client_id: ClientId) {
        let entity_id = self.next_entity_id;
        self.next_entity_id += 1;
        
        let name = self.network.client_name(client_id)
            .unwrap_or("Unknown")
            .to_string();
        
        let player = PlayerState::new(entity_id, self.spawn_pos, name.clone());
        self.players.insert(client_id, player);
        
        // Send welcome message
        let welcome = ServerMessage::Welcome {
            player_id: entity_id,
            tick_rate: TICK_RATE,
            seed: self.seed,
        };
        if let Err(e) = self.network.send(client_id, &welcome) {
            warn!("Failed to send welcome: {e}");
        }
        
        // Notify others
        let join = ServerMessage::PlayerJoined {
            id: entity_id,
            name,
        };
        let _ = self.network.broadcast(&join);
        
        info!("Player {} joined (entity {})", client_id, entity_id);
    }
    
    /// Handle client disconnection.
    fn on_client_disconnected(&mut self, client_id: ClientId) {
        if let Some(player) = self.players.remove(&client_id) {
            let leave = ServerMessage::PlayerLeft {
                id: player.entity_id,
            };
            let _ = self.network.broadcast(&leave);
            info!("Player {} left", client_id);
        }
    }
    
    /// Process a message from a client.
    fn process_message(&mut self, client_id: ClientId, message: ClientMessage) {
        match message {
            ClientMessage::Input(input) => {
                if let Some(player) = self.players.get_mut(&client_id) {
                    // Apply input to player
                    player.yaw = input.yaw;
                    player.pitch = input.pitch;
                    player.last_ack_sequence = input.sequence;
                    
                    // Simple movement (would integrate with physics)
                    let forward = Vec3::new(
                        input.yaw.sin(),
                        0.0,
                        -input.yaw.cos(),
                    );
                    let right = Vec3::new(
                        -input.yaw.cos(),
                        0.0,
                        -input.yaw.sin(),
                    );
                    
                    let mut move_dir = Vec3::ZERO;
                    move_dir += forward * input.movement.z;
                    move_dir += right * input.movement.x;
                    
                    let speed = if input.sprint { 7.5 } else { 5.0 };
                    if move_dir.length_squared() > 0.0 {
                        move_dir = move_dir.normalize() * speed;
                    }
                    
                    player.velocity = Vec3::new(move_dir.x, player.velocity.y, move_dir.z);
                    
                    // Apply gravity
                    player.velocity.y -= 20.0 * (TICK_DURATION.as_secs_f32());
                    
                    // Simple ground check at y=64
                    if player.position.y <= 64.0 {
                        player.position.y = 64.0;
                        player.velocity.y = 0.0;
                        
                        if input.jump {
                            player.velocity.y = 8.0;
                        }
                    }
                    
                    // Update position
                    player.position += player.velocity * TICK_DURATION.as_secs_f32();
                }
            }
            
            ClientMessage::BlockPlace { pos, block } => {
                // TODO: Integrate with world
                let change = ServerMessage::BlockChange { pos, block };
                let _ = self.network.broadcast(&change);
            }
            
            ClientMessage::BlockBreak { pos } => {
                // TODO: Integrate with world
                let change = ServerMessage::BlockChange { pos, block: 0 };
                let _ = self.network.broadcast(&change);
            }
            
            ClientMessage::ChatSend { message } => {
                if let Some(player) = self.players.get(&client_id) {
                    let chat = ServerMessage::ChatReceive {
                        sender: player.name.clone(),
                        message,
                    };
                    let _ = self.network.broadcast(&chat);
                }
            }
            
            ClientMessage::ChunkRequest { pos } => {
                // TODO: Integrate with world
                debug!("Chunk request from {}: {:?}", client_id, pos);
            }
        }
    }
    
    /// Simulate world (placeholder).
    fn simulate(&mut self) {
        // Would update creatures, physics, etc.
    }
    
    /// Broadcast world snapshots to all clients.
    fn broadcast_snapshots(&mut self) {
        // Build entity list from other players
        let player_snapshots: Vec<_> = self.players.iter()
            .map(|(_, player)| {
                engine_network::protocol::server_message::EntitySnapshot {
                    id: player.entity_id,
                    kind: EntityKind::Player,
                    position: player.position,
                    rotation: Quat::from_rotation_y(player.yaw),
                    velocity: player.velocity,
                    health: Some(20.0),
                }
            })
            .collect();
        
        // Send personalized snapshot to each client
        for (&client_id, player) in &self.players {
            // Filter out self from entities
            let entities: Vec<_> = player_snapshots.iter()
                .filter(|e| e.id != player.entity_id)
                .cloned()
                .collect();
            
            let snapshot = WorldSnapshot {
                tick: self.tick,
                ack_sequence: player.last_ack_sequence,
                player_position: player.position,
                player_velocity: player.velocity,
                entities,
            };
            
            let msg = ServerMessage::Snapshot(snapshot);
            if let Err(e) = self.network.send(client_id, &msg) {
                debug!("Failed to send snapshot to {}: {}", client_id, e);
            }
        }
    }
    
    /// Stop the server.
    pub fn shutdown(&mut self) {
        self.running = false;
    }
    
    /// Get current tick number.
    pub fn current_tick(&self) -> u64 {
        self.tick
    }
    
    /// Get number of connected players.
    pub fn player_count(&self) -> usize {
        self.players.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn server_creation() {
        let server = LatticeServer::new(0, 12345);
        assert!(server.is_ok());
    }
    
    #[test]
    fn tick_duration_correct() {
        // 20 Hz = 50ms per tick
        assert_eq!(TICK_DURATION, Duration::from_millis(50));
    }
    
    #[test]
    fn player_state_initialization() {
        let player = PlayerState::new(1, Vec3::new(0.0, 64.0, 0.0), "Test".into());
        assert_eq!(player.entity_id, 1);
        assert_eq!(player.position.y, 64.0);
        assert_eq!(player.velocity, Vec3::ZERO);
    }
}
