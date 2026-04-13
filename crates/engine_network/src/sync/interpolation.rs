//! State interpolation for smooth rendering between snapshots.
//!
//! Buffers the last 2 snapshots and interpolates entity positions
//! to produce smooth movement despite network jitter.

use std::collections::VecDeque;

use glam::{Quat, Vec3};

use crate::protocol::server_message::EntitySnapshot;
use crate::WorldSnapshot;

/// Default interpolation delay in milliseconds.
/// At 20 Hz tick rate, this is 2 ticks worth of buffer.
pub const DEFAULT_INTERPOLATION_DELAY_MS: u64 = 100;

/// Interpolated entity state for rendering.
#[derive(Clone, Debug)]
pub struct InterpolatedEntity {
    /// Entity network ID.
    pub id: u64,
    /// Interpolated position.
    pub position: Vec3,
    /// Interpolated rotation.
    pub rotation: Quat,
    /// Interpolated velocity (for prediction).
    pub velocity: Vec3,
}

/// Interpolated world state for rendering.
#[derive(Clone, Debug)]
pub struct InterpolatedState {
    /// Render time (between snapshot times).
    pub render_time: f64,
    /// Player's interpolated position.
    pub player_position: Vec3,
    /// Player's interpolated velocity.
    pub player_velocity: Vec3,
    /// Interpolated entities.
    pub entities: Vec<InterpolatedEntity>,
}

/// Buffered snapshot with timestamp.
#[derive(Clone, Debug)]
struct TimestampedSnapshot {
    /// Time when received (monotonic, seconds).
    time: f64,
    /// The snapshot data.
    snapshot: WorldSnapshot,
}

/// Buffer for snapshot interpolation.
pub struct InterpolationBuffer {
    /// Buffered snapshots (newest at back).
    snapshots: VecDeque<TimestampedSnapshot>,
    /// Interpolation delay in seconds.
    delay: f64,
    /// Current monotonic time.
    current_time: f64,
    /// Server tick rate (for time calculations).
    tick_rate: u32,
}

impl InterpolationBuffer {
    /// Create a new interpolation buffer.
    pub fn new(tick_rate: u32) -> Self {
        Self {
            snapshots: VecDeque::with_capacity(4),
            delay: DEFAULT_INTERPOLATION_DELAY_MS as f64 / 1000.0,
            current_time: 0.0,
            tick_rate,
        }
    }
    
    /// Set interpolation delay in milliseconds.
    pub fn set_delay_ms(&mut self, delay_ms: u64) {
        self.delay = delay_ms as f64 / 1000.0;
    }
    
    /// Push a new snapshot into the buffer.
    pub fn push(&mut self, snapshot: WorldSnapshot) {
        let timestamped = TimestampedSnapshot {
            time: self.current_time,
            snapshot,
        };
        
        self.snapshots.push_back(timestamped);
        
        // Keep at most 4 snapshots
        while self.snapshots.len() > 4 {
            self.snapshots.pop_front();
        }
    }
    
    /// Advance time and get interpolated state.
    pub fn update(&mut self, dt: f64) -> Option<InterpolatedState> {
        self.current_time += dt;
        
        // Render time is current time minus delay
        let render_time = self.current_time - self.delay;
        
        // Need at least 2 snapshots to interpolate
        if self.snapshots.len() < 2 {
            // Use latest snapshot if available
            return self.snapshots.back().map(|ts| {
                InterpolatedState {
                    render_time,
                    player_position: ts.snapshot.player_position,
                    player_velocity: ts.snapshot.player_velocity,
                    entities: ts.snapshot.entities.iter()
                        .map(|e| InterpolatedEntity {
                            id: e.id,
                            position: e.position,
                            rotation: e.rotation,
                            velocity: e.velocity,
                        })
                        .collect(),
                }
            });
        }
        
        // Find the two snapshots to interpolate between
        let (from, to, alpha) = self.find_interpolation_pair(render_time)?;
        
        // Interpolate player
        let player_position = from.snapshot.player_position
            .lerp(to.snapshot.player_position, alpha);
        let player_velocity = from.snapshot.player_velocity
            .lerp(to.snapshot.player_velocity, alpha);
        
        // Interpolate entities
        let entities = interpolate_entities(&from.snapshot.entities, &to.snapshot.entities, alpha);
        
        Some(InterpolatedState {
            render_time,
            player_position,
            player_velocity,
            entities,
        })
    }
    
    /// Find the two snapshots to interpolate between.
    fn find_interpolation_pair(&self, render_time: f64) -> Option<(&TimestampedSnapshot, &TimestampedSnapshot, f32)> {
        // Find first snapshot after render_time
        for i in 1..self.snapshots.len() {
            let from = &self.snapshots[i - 1];
            let to = &self.snapshots[i];
            
            if from.time <= render_time && to.time >= render_time {
                let duration = to.time - from.time;
                let alpha = if duration > 0.0 {
                    ((render_time - from.time) / duration) as f32
                } else {
                    0.0
                };
                return Some((from, to, alpha.clamp(0.0, 1.0)));
            }
        }
        
        // If render time is before all snapshots, use oldest
        if render_time < self.snapshots.front()?.time {
            let oldest = self.snapshots.front()?;
            return Some((oldest, oldest, 0.0));
        }
        
        // If render time is after all snapshots, extrapolate from latest
        let latest = self.snapshots.back()?;
        Some((latest, latest, 0.0))
    }
    
    /// Get the latest server tick number.
    pub fn latest_tick(&self) -> Option<u64> {
        self.snapshots.back().map(|ts| ts.snapshot.tick)
    }
    
    /// Clear all buffered snapshots.
    pub fn clear(&mut self) {
        self.snapshots.clear();
        self.current_time = 0.0;
    }
}

/// Interpolate entity lists between two snapshots.
fn interpolate_entities(
    from: &[EntitySnapshot],
    to: &[EntitySnapshot],
    alpha: f32,
) -> Vec<InterpolatedEntity> {
    // Build map from entity ID to 'to' snapshot
    use std::collections::HashMap;
    let to_map: HashMap<u64, &EntitySnapshot> = to.iter()
        .map(|e| (e.id, e))
        .collect();
    
    let mut result = Vec::with_capacity(to.len());
    
    // Interpolate entities that exist in both snapshots
    for from_entity in from {
        if let Some(to_entity) = to_map.get(&from_entity.id) {
            result.push(InterpolatedEntity {
                id: from_entity.id,
                position: from_entity.position.lerp(to_entity.position, alpha),
                rotation: from_entity.rotation.slerp(to_entity.rotation, alpha),
                velocity: from_entity.velocity.lerp(to_entity.velocity, alpha),
            });
        }
    }
    
    // Add entities that only exist in 'to' (newly spawned)
    for to_entity in to {
        if !from.iter().any(|e| e.id == to_entity.id) {
            result.push(InterpolatedEntity {
                id: to_entity.id,
                position: to_entity.position,
                rotation: to_entity.rotation,
                velocity: to_entity.velocity,
            });
        }
    }
    
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::EntityKind;
    
    fn make_snapshot(tick: u64, player_pos: Vec3) -> WorldSnapshot {
        WorldSnapshot {
            tick,
            ack_sequence: 0,
            player_position: player_pos,
            player_velocity: Vec3::ZERO,
            entities: vec![],
        }
    }
    
    #[test]
    fn buffer_creation() {
        let buffer = InterpolationBuffer::new(20);
        assert!(buffer.snapshots.is_empty());
        assert_eq!(buffer.tick_rate, 20);
    }
    
    #[test]
    fn push_snapshots() {
        let mut buffer = InterpolationBuffer::new(20);
        
        buffer.push(make_snapshot(1, Vec3::ZERO));
        assert_eq!(buffer.snapshots.len(), 1);
        
        buffer.push(make_snapshot(2, Vec3::X));
        assert_eq!(buffer.snapshots.len(), 2);
    }
    
    #[test]
    fn buffer_size_limited() {
        let mut buffer = InterpolationBuffer::new(20);
        
        for i in 0..10 {
            buffer.push(make_snapshot(i, Vec3::ZERO));
        }
        
        assert_eq!(buffer.snapshots.len(), 4);
    }
    
    #[test]
    fn interpolation_with_two_snapshots() {
        let mut buffer = InterpolationBuffer::new(20);
        buffer.set_delay_ms(0); // No delay for testing
        
        buffer.push(make_snapshot(1, Vec3::ZERO));
        buffer.current_time = 0.1;
        buffer.push(make_snapshot(2, Vec3::new(10.0, 0.0, 0.0)));
        
        // At render_time = 0.05 (halfway), should interpolate
        buffer.current_time = 0.05;
        let state = buffer.update(0.0).unwrap();
        
        // Position should be between 0 and 10
        assert!(state.player_position.x >= 0.0);
        assert!(state.player_position.x <= 10.0);
    }
    
    #[test]
    fn entity_interpolation() {
        let from = vec![
            EntitySnapshot {
                id: 1,
                kind: EntityKind::Player,
                position: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                velocity: Vec3::ZERO,
                health: Some(20.0),
            },
        ];
        
        let to = vec![
            EntitySnapshot {
                id: 1,
                kind: EntityKind::Player,
                position: Vec3::new(10.0, 0.0, 0.0),
                rotation: Quat::IDENTITY,
                velocity: Vec3::ZERO,
                health: Some(20.0),
            },
        ];
        
        let result = interpolate_entities(&from, &to, 0.5);
        assert_eq!(result.len(), 1);
        assert!((result[0].position.x - 5.0).abs() < 0.001);
    }
    
    #[test]
    fn new_entity_appears() {
        let from = vec![];
        let to = vec![
            EntitySnapshot {
                id: 1,
                kind: EntityKind::Pig,
                position: Vec3::new(5.0, 0.0, 0.0),
                rotation: Quat::IDENTITY,
                velocity: Vec3::ZERO,
                health: Some(10.0),
            },
        ];
        
        let result = interpolate_entities(&from, &to, 0.5);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, 1);
    }
}
