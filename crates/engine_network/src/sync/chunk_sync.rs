//! Chunk synchronization for multiplayer.
//!
//! Manages chunk requests from clients and prioritized chunk delivery
//! from the server.

use engine_core::coords::ChunkPos;
use glam::Vec3;
use std::collections::{HashMap, HashSet, VecDeque};

/// Maximum pending chunk requests per client.
const MAX_PENDING_REQUESTS: usize = 64;

/// Maximum chunks to send per tick.
const MAX_CHUNKS_PER_TICK: usize = 4;

/// Priority levels for chunk requests.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ChunkPriority {
    /// Critical - player is standing in or near this chunk.
    Critical = 0,
    /// High - chunk is within close view distance.
    High = 1,
    /// Normal - chunk is within view distance.
    Normal = 2,
    /// Low - chunk is at edge of view distance.
    Low = 3,
}

impl ChunkPriority {
    /// Determine priority based on distance from player.
    #[must_use]
    pub fn from_distance(distance_squared: f32, view_distance: i32) -> Self {
        let vd = view_distance as f32;
        let critical_dist = 2.0 * 2.0; // Within 2 chunks
        let high_dist = (vd * 0.3) * (vd * 0.3);
        let normal_dist = (vd * 0.7) * (vd * 0.7);

        if distance_squared <= critical_dist {
            ChunkPriority::Critical
        } else if distance_squared <= high_dist {
            ChunkPriority::High
        } else if distance_squared <= normal_dist {
            ChunkPriority::Normal
        } else {
            ChunkPriority::Low
        }
    }
}

/// A chunk request from a client.
#[derive(Clone, Debug)]
pub struct ChunkRequest {
    /// Chunk position requested.
    pub pos: ChunkPos,
    /// Priority of this request.
    pub priority: ChunkPriority,
    /// Distance squared from player (for sorting within priority).
    pub distance_sq: f32,
}

impl ChunkRequest {
    /// Create a new chunk request.
    #[must_use]
    pub fn new(pos: ChunkPos, player_pos: Vec3, view_distance: i32) -> Self {
        let chunk_center = Vec3::new(
            (pos.x() * 16 + 8) as f32,
            (pos.y() * 16 + 8) as f32,
            (pos.z() * 16 + 8) as f32,
        );
        let distance_sq = player_pos.distance_squared(chunk_center);
        let priority = ChunkPriority::from_distance(distance_sq, view_distance);

        Self {
            pos,
            priority,
            distance_sq,
        }
    }
}

/// Client-side chunk request manager.
///
/// Tracks which chunks have been requested and received.
#[derive(Debug, Default)]
pub struct ClientChunkSync {
    /// Chunks we've requested but not received.
    pending: HashSet<ChunkPos>,
    /// Chunks we have loaded.
    loaded: HashSet<ChunkPos>,
    /// Current view distance.
    view_distance: i32,
}

impl ClientChunkSync {
    /// Create a new client chunk sync manager.
    #[must_use]
    pub fn new(view_distance: i32) -> Self {
        Self {
            pending: HashSet::new(),
            loaded: HashSet::new(),
            view_distance,
        }
    }

    /// Update based on player position, returning chunks to request.
    ///
    /// Returns a list of chunk positions to request from the server.
    pub fn update(&mut self, player_pos: Vec3) -> Vec<ChunkRequest> {
        let player_chunk = ChunkPos::new(
            (player_pos.x / 16.0).floor() as i32,
            (player_pos.y / 16.0).floor() as i32,
            (player_pos.z / 16.0).floor() as i32,
        );

        let mut requests = Vec::new();

        // Find chunks we need
        for dx in -self.view_distance..=self.view_distance {
            for dy in -self.view_distance..=self.view_distance {
                for dz in -self.view_distance..=self.view_distance {
                    let pos = ChunkPos::new(
                        player_chunk.x() + dx,
                        player_chunk.y() + dy,
                        player_chunk.z() + dz,
                    );

                    // Skip if already loaded or pending
                    if self.loaded.contains(&pos) || self.pending.contains(&pos) {
                        continue;
                    }

                    // Skip if too many pending
                    if self.pending.len() >= MAX_PENDING_REQUESTS {
                        break;
                    }

                    let request = ChunkRequest::new(pos, player_pos, self.view_distance);
                    requests.push(request);
                    self.pending.insert(pos);
                }
            }
        }

        // Sort by priority (critical first), then by distance
        requests.sort_by(|a, b| {
            a.priority
                .cmp(&b.priority)
                .then_with(|| a.distance_sq.partial_cmp(&b.distance_sq).unwrap())
        });

        requests
    }

    /// Mark a chunk as received.
    pub fn chunk_received(&mut self, pos: ChunkPos) {
        self.pending.remove(&pos);
        self.loaded.insert(pos);
    }

    /// Unload chunks far from player.
    ///
    /// Returns positions of chunks that were unloaded.
    pub fn unload_distant(&mut self, player_pos: Vec3) -> Vec<ChunkPos> {
        let player_chunk = ChunkPos::new(
            (player_pos.x / 16.0).floor() as i32,
            (player_pos.y / 16.0).floor() as i32,
            (player_pos.z / 16.0).floor() as i32,
        );

        // Unload at view_distance + 2 for hysteresis
        let unload_dist = self.view_distance + 2;
        let unload_dist_sq = (unload_dist * unload_dist) as i32;

        let mut to_unload = Vec::new();
        self.loaded.retain(|&pos| {
            let dx = pos.x() - player_chunk.x();
            let dy = pos.y() - player_chunk.y();
            let dz = pos.z() - player_chunk.z();
            let dist_sq = dx * dx + dy * dy + dz * dz;

            if dist_sq > unload_dist_sq {
                to_unload.push(pos);
                false
            } else {
                true
            }
        });

        // Also remove from pending
        self.pending.retain(|&pos| {
            let dx = pos.x() - player_chunk.x();
            let dy = pos.y() - player_chunk.y();
            let dz = pos.z() - player_chunk.z();
            let dist_sq = dx * dx + dy * dy + dz * dz;
            dist_sq <= unload_dist_sq
        });

        to_unload
    }

    /// Check if a chunk is loaded.
    #[must_use]
    pub fn is_loaded(&self, pos: ChunkPos) -> bool {
        self.loaded.contains(&pos)
    }

    /// Check if a chunk request is pending.
    #[must_use]
    pub fn is_pending(&self, pos: ChunkPos) -> bool {
        self.pending.contains(&pos)
    }

    /// Number of loaded chunks.
    #[must_use]
    pub fn loaded_count(&self) -> usize {
        self.loaded.len()
    }

    /// Number of pending requests.
    #[must_use]
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }
}

/// Server-side chunk delivery manager.
///
/// Manages chunk requests from multiple clients and delivers
/// chunks in priority order.
#[derive(Debug)]
pub struct ServerChunkSync {
    /// Pending requests per client.
    client_requests: HashMap<u64, VecDeque<ChunkRequest>>,
    /// Maximum chunks to send per tick per client.
    max_per_tick: usize,
}

impl Default for ServerChunkSync {
    fn default() -> Self {
        Self::new()
    }
}

impl ServerChunkSync {
    /// Create a new server chunk sync manager.
    #[must_use]
    pub fn new() -> Self {
        Self {
            client_requests: HashMap::new(),
            max_per_tick: MAX_CHUNKS_PER_TICK,
        }
    }

    /// Register a chunk request from a client.
    pub fn add_request(&mut self, client_id: u64, request: ChunkRequest) {
        let queue = self.client_requests.entry(client_id).or_default();

        // Don't add duplicates
        if queue.iter().any(|r| r.pos == request.pos) {
            return;
        }

        // Insert in priority order
        let insert_idx = queue
            .iter()
            .position(|r| {
                request.priority < r.priority
                    || (request.priority == r.priority && request.distance_sq < r.distance_sq)
            })
            .unwrap_or(queue.len());

        queue.insert(insert_idx, request);

        // Limit queue size
        while queue.len() > MAX_PENDING_REQUESTS {
            queue.pop_back();
        }
    }

    /// Get the next chunks to send for a client.
    ///
    /// Returns up to `max_per_tick` chunk positions to send.
    pub fn next_chunks(&mut self, client_id: u64) -> Vec<ChunkPos> {
        let queue = match self.client_requests.get_mut(&client_id) {
            Some(q) => q,
            None => return Vec::new(),
        };

        let count = self.max_per_tick.min(queue.len());
        let mut chunks = Vec::with_capacity(count);

        for _ in 0..count {
            if let Some(request) = queue.pop_front() {
                chunks.push(request.pos);
            }
        }

        chunks
    }

    /// Remove a client's pending requests.
    pub fn remove_client(&mut self, client_id: u64) {
        self.client_requests.remove(&client_id);
    }

    /// Get pending request count for a client.
    #[must_use]
    pub fn pending_count(&self, client_id: u64) -> usize {
        self.client_requests
            .get(&client_id)
            .map(VecDeque::len)
            .unwrap_or(0)
    }

    /// Set maximum chunks to send per tick.
    pub fn set_max_per_tick(&mut self, max: usize) {
        self.max_per_tick = max;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_from_distance() {
        let view_dist = 8;

        // Critical - within 2 chunks
        assert_eq!(
            ChunkPriority::from_distance(1.0, view_dist),
            ChunkPriority::Critical
        );

        // High - within 30% of view distance
        assert_eq!(
            ChunkPriority::from_distance(5.0, view_dist),
            ChunkPriority::High
        );

        // Normal - within 70%
        assert_eq!(
            ChunkPriority::from_distance(20.0, view_dist),
            ChunkPriority::Normal
        );

        // Low - beyond 70%
        assert_eq!(
            ChunkPriority::from_distance(50.0, view_dist),
            ChunkPriority::Low
        );
    }

    #[test]
    fn test_client_chunk_sync_update() {
        let mut sync = ClientChunkSync::new(2);
        let player_pos = Vec3::new(0.0, 64.0, 0.0);

        let requests = sync.update(player_pos);

        // Should request chunks within view distance
        assert!(!requests.is_empty());

        // Should have marked them as pending
        assert!(sync.pending_count() > 0);
    }

    #[test]
    fn test_client_chunk_received() {
        let mut sync = ClientChunkSync::new(2);
        let pos = ChunkPos::new(0, 0, 0);

        sync.pending.insert(pos);
        sync.chunk_received(pos);

        assert!(!sync.is_pending(pos));
        assert!(sync.is_loaded(pos));
    }

    #[test]
    fn test_client_unload_distant() {
        let mut sync = ClientChunkSync::new(2);

        // Load some chunks
        sync.loaded.insert(ChunkPos::new(0, 0, 0));
        sync.loaded.insert(ChunkPos::new(100, 0, 0)); // Far away

        let player_pos = Vec3::new(0.0, 0.0, 0.0);
        let unloaded = sync.unload_distant(player_pos);

        assert!(unloaded.contains(&ChunkPos::new(100, 0, 0)));
        assert!(!sync.is_loaded(ChunkPos::new(100, 0, 0)));
        assert!(sync.is_loaded(ChunkPos::new(0, 0, 0)));
    }

    #[test]
    fn test_server_chunk_sync_add_request() {
        let mut sync = ServerChunkSync::new();

        let request = ChunkRequest {
            pos: ChunkPos::new(0, 0, 0),
            priority: ChunkPriority::Normal,
            distance_sq: 10.0,
        };

        sync.add_request(1, request);

        assert_eq!(sync.pending_count(1), 1);
    }

    #[test]
    fn test_server_chunk_sync_priority_order() {
        let mut sync = ServerChunkSync::new();

        // Add low priority first
        sync.add_request(
            1,
            ChunkRequest {
                pos: ChunkPos::new(0, 0, 0),
                priority: ChunkPriority::Low,
                distance_sq: 10.0,
            },
        );

        // Add critical priority second
        sync.add_request(
            1,
            ChunkRequest {
                pos: ChunkPos::new(1, 0, 0),
                priority: ChunkPriority::Critical,
                distance_sq: 1.0,
            },
        );

        // Critical should come out first
        let chunks = sync.next_chunks(1);
        assert_eq!(chunks[0], ChunkPos::new(1, 0, 0));
    }

    #[test]
    fn test_server_chunk_sync_max_per_tick() {
        let mut sync = ServerChunkSync::new();
        sync.set_max_per_tick(2);

        // Add 5 requests
        for i in 0..5 {
            sync.add_request(
                1,
                ChunkRequest {
                    pos: ChunkPos::new(i, 0, 0),
                    priority: ChunkPriority::Normal,
                    distance_sq: i as f32,
                },
            );
        }

        // Should only get 2
        let chunks = sync.next_chunks(1);
        assert_eq!(chunks.len(), 2);

        // 3 should remain
        assert_eq!(sync.pending_count(1), 3);
    }

    #[test]
    fn test_server_remove_client() {
        let mut sync = ServerChunkSync::new();

        sync.add_request(
            1,
            ChunkRequest {
                pos: ChunkPos::new(0, 0, 0),
                priority: ChunkPriority::Normal,
                distance_sq: 10.0,
            },
        );

        sync.remove_client(1);

        assert_eq!(sync.pending_count(1), 0);
    }
}
