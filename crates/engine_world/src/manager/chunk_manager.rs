//! Chunk management with loading, unloading, and state tracking.

use crate::chunk::{BlockId, Chunk, ChunkState};
use crate::generation::TerrainGenerator;
use crate::manager::{LoadingQueue, NeighborTracker};
use crossbeam_channel::{Receiver, Sender};
use engine_core::coords::{ChunkPos, WorldPos, CHUNK_SIZE};
use glam::{IVec3, Vec3};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tracing::{debug, trace};

/// Maximum chunks to generate per update.
const MAX_GENERATE_PER_UPDATE: usize = 4;

/// Entry in the chunk map containing chunk data and state.
#[derive(Debug)]
pub struct ChunkEntry {
    /// The chunk data.
    pub chunk: Chunk,
    /// Current state of this chunk.
    pub state: ChunkState,
}

/// Result from a generation job.
struct GenerationResult {
    pos: ChunkPos,
    chunk: Chunk,
}

/// Manages chunk loading, unloading, and state transitions.
pub struct ChunkManager {
    /// All loaded chunks.
    chunks: HashMap<ChunkPos, ChunkEntry>,
    /// Ready chunk positions for quick lookup.
    ready_chunks: HashSet<ChunkPos>,
    /// Terrain generator (shared for parallel generation).
    generator: Arc<TerrainGenerator>,
    /// View distance in chunks.
    view_distance: i32,
    /// Unload distance (hysteresis).
    unload_distance: i32,
    /// Queue for loading chunks in spiral order.
    loading_queue: LoadingQueue,
    /// Tracks neighbor dependencies for meshing.
    neighbor_tracker: NeighborTracker,
    /// Current player chunk position.
    player_chunk: ChunkPos,
    /// Channel for receiving generated chunks.
    generation_rx: Receiver<GenerationResult>,
    /// Channel for sending generation jobs.
    generation_tx: Sender<GenerationResult>,
    /// Chunks currently being generated.
    generating: HashSet<ChunkPos>,
}

impl ChunkManager {
    /// Create a new chunk manager.
    ///
    /// # Arguments
    /// * `seed` - World seed for terrain generation.
    /// * `view_distance` - View distance in chunks.
    #[must_use]
    pub fn new(seed: u64, view_distance: i32) -> Self {
        let (tx, rx) = crossbeam_channel::unbounded();

        Self {
            chunks: HashMap::new(),
            ready_chunks: HashSet::new(),
            generator: Arc::new(TerrainGenerator::new(seed)),
            view_distance,
            unload_distance: view_distance + 2,
            loading_queue: LoadingQueue::new(),
            neighbor_tracker: NeighborTracker::new(),
            player_chunk: ChunkPos(IVec3::new(i32::MAX, i32::MAX, i32::MAX)),
            generation_rx: rx,
            generation_tx: tx,
        generating: HashSet::new(),
        }
    }

    /// Update chunk loading based on player position.
    ///
    /// Call this every frame with the player's world position.
    pub fn update(&mut self, player_pos: Vec3) {
        let new_chunk = WorldPos(IVec3::new(
            player_pos.x.floor() as i32,
            player_pos.y.floor() as i32,
            player_pos.z.floor() as i32,
        ))
        .to_chunk_pos();

        // Rebuild loading queue if player moved to new chunk
        if new_chunk != self.player_chunk {
            debug!(
                "Player moved to chunk {:?} from {:?}",
                new_chunk, self.player_chunk
            );
            self.player_chunk = new_chunk;
            self.rebuild_loading_queue();
            self.unload_distant_chunks();
        }

        // Process completed generations
        self.process_generation_results();

        // Start new generation jobs
        self.start_generation_jobs();

        // Update chunks waiting for neighbors
        self.process_neighbor_dependencies();
    }

    /// Get an immutable reference to a chunk.
    #[must_use]
    pub fn get_chunk(&self, pos: ChunkPos) -> Option<&Chunk> {
        self.chunks.get(&pos).map(|e| &e.chunk)
    }

    /// Get a mutable reference to a chunk.
    pub fn get_chunk_mut(&mut self, pos: ChunkPos) -> Option<&mut Chunk> {
        self.chunks.get_mut(&pos).map(|e| &mut e.chunk)
    }

    /// Get the state of a chunk.
    #[must_use]
    pub fn get_state(&self, pos: ChunkPos) -> Option<ChunkState> {
        self.chunks.get(&pos).map(|e| e.state)
    }

    /// Set a block in the world, marking the chunk as dirty.
    ///
    /// Returns `true` if the block was set successfully.
    pub fn set_block(&mut self, pos: WorldPos, block: BlockId) -> bool {
        let chunk_pos = pos.to_chunk_pos();
        let local_pos = pos.to_local_pos();

        if let Some(entry) = self.chunks.get_mut(&chunk_pos) {
            entry.chunk.set(local_pos, block);

            // Mark as dirty if it was ready
            if entry.state == ChunkState::Ready {
                entry.state = ChunkState::Dirty;
                self.ready_chunks.remove(&chunk_pos);
                debug!("Chunk {:?} marked dirty", chunk_pos);
            }

            // Check if we modified a border block - mark neighbors dirty too
            let local = local_pos.0;
            let border_neighbors = [
                (local.x == 0, IVec3::new(-1, 0, 0)),
                (local.x == CHUNK_SIZE as u32 - 1, IVec3::new(1, 0, 0)),
                (local.y == 0, IVec3::new(0, -1, 0)),
                (local.y == CHUNK_SIZE as u32 - 1, IVec3::new(0, 1, 0)),
                (local.z == 0, IVec3::new(0, 0, -1)),
                (local.z == CHUNK_SIZE as u32 - 1, IVec3::new(0, 0, 1)),
            ];

            for (is_border, offset) in border_neighbors {
                if is_border {
                    let neighbor_pos = ChunkPos(chunk_pos.0 + offset);
                    if let Some(neighbor) = self.chunks.get_mut(&neighbor_pos) {
                        if neighbor.state == ChunkState::Ready {
                            neighbor.state = ChunkState::Dirty;
                            self.ready_chunks.remove(&neighbor_pos);
                            debug!("Neighbor chunk {:?} marked dirty", neighbor_pos);
                        }
                    }
                }
            }

            true
        } else {
            false
        }
    }

    /// Iterate over all ready chunks.
    pub fn iter_ready(&self) -> impl Iterator<Item = (ChunkPos, &Chunk)> {
        self.ready_chunks.iter().filter_map(|&pos| {
            self.chunks.get(&pos).map(|entry| (pos, &entry.chunk))
        })
    }

    /// Iterate over all dirty chunks that need remeshing.
    pub fn iter_dirty(&self) -> impl Iterator<Item = (ChunkPos, &Chunk)> {
        self.chunks.iter().filter_map(|(pos, entry)| {
            if entry.state == ChunkState::Dirty {
                Some((*pos, &entry.chunk))
            } else {
                None
            }
        })
    }

    /// Mark a chunk as ready after meshing is complete.
    pub fn mark_ready(&mut self, pos: ChunkPos) {
        if let Some(entry) = self.chunks.get_mut(&pos) {
            entry.state = ChunkState::Ready;
            self.ready_chunks.insert(pos);
            trace!("Chunk {:?} marked ready", pos);
        }
    }

    /// Mark a chunk as meshing (in progress).
    pub fn mark_meshing(&mut self, pos: ChunkPos) {
        if let Some(entry) = self.chunks.get_mut(&pos) {
            entry.state = ChunkState::Meshing;
            trace!("Chunk {:?} marked meshing", pos);
        }
    }

    /// Get chunks that are ready for meshing (generated + all neighbors ready).
    pub fn chunks_ready_for_meshing(&self) -> Vec<ChunkPos> {
        self.chunks
            .iter()
            .filter(|(_, entry)| entry.state == ChunkState::Generated)
            .filter(|(pos, _)| !self.neighbor_tracker.is_waiting(pos))
            .map(|(pos, _)| *pos)
            .collect()
    }

    /// Number of loaded chunks.
    #[must_use]
    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    /// Number of ready chunks.
    #[must_use]
    pub fn ready_count(&self) -> usize {
        self.ready_chunks.len()
    }

    /// Rebuild the loading queue based on player position.
    fn rebuild_loading_queue(&mut self) {
        self.loading_queue.rebuild(self.player_chunk, self.view_distance);
        debug!(
            "Rebuilt loading queue: {} chunks to consider",
            self.loading_queue.len()
        );
    }

    /// Unload chunks that are too far from the player.
    fn unload_distant_chunks(&mut self) {
        let center = self.player_chunk.0;
        let max_dist_sq = self.unload_distance * self.unload_distance;

        let to_unload: Vec<ChunkPos> = self
            .chunks
            .keys()
            .filter(|pos| {
                let diff = pos.0 - center;
                diff.x * diff.x + diff.y * diff.y + diff.z * diff.z > max_dist_sq
            })
            .copied()
            .collect();

        for pos in to_unload {
            self.unload_chunk(pos);
        }
    }

    /// Unload a single chunk.
    fn unload_chunk(&mut self, pos: ChunkPos) {
        self.chunks.remove(&pos);
        self.ready_chunks.remove(&pos);
        self.generating.remove(&pos);
        self.neighbor_tracker.remove(pos);
        debug!("Unloaded chunk {:?}", pos);
    }

    /// Process completed generation results.
    fn process_generation_results(&mut self) {
        while let Ok(result) = self.generation_rx.try_recv() {
            self.generating.remove(&result.pos);

            // Check if chunk is still needed (player might have moved)
            let center = self.player_chunk.0;
            let diff = result.pos.0 - center;
            let dist_sq = diff.x * diff.x + diff.y * diff.y + diff.z * diff.z;

            if dist_sq > self.unload_distance * self.unload_distance {
                trace!("Discarding generated chunk {:?} (too far)", result.pos);
                continue;
            }

            self.chunks.insert(
                result.pos,
                ChunkEntry {
                    chunk: result.chunk,
                    state: ChunkState::Generated,
                },
            );

            // Check if neighbors are waiting for this chunk
            let unblocked = self.neighbor_tracker.chunk_ready(result.pos);
            for pos in unblocked {
                debug!("Chunk {:?} unblocked by {:?}", pos, result.pos);
            }

            trace!("Chunk {:?} generated", result.pos);
        }
    }

    /// Start generation jobs for queued chunks.
    fn start_generation_jobs(&mut self) {
        let mut jobs_started = 0;

        while jobs_started < MAX_GENERATE_PER_UPDATE && !self.loading_queue.is_empty() {
            let Some(pos) = self.loading_queue.pop() else {
                break;
            };

            // Skip if already loaded or generating
            if self.chunks.contains_key(&pos) || self.generating.contains(&pos) {
                continue;
            }

            // Mark as generating
            self.generating.insert(pos);

            // Spawn generation task
            let generator = Arc::clone(&self.generator);
            let tx = self.generation_tx.clone();

            rayon::spawn(move || {
                let chunk = generator.generate(pos);
                let _ = tx.send(GenerationResult { pos, chunk });
            });

            jobs_started += 1;
            trace!("Started generation for chunk {:?}", pos);
        }
    }

    /// Process chunks waiting for neighbor dependencies.
    fn process_neighbor_dependencies(&mut self) {
        // Find all Generated chunks and check their neighbor status
        let generated: Vec<ChunkPos> = self
            .chunks
            .iter()
            .filter(|(_, entry)| entry.state == ChunkState::Generated)
            .map(|(pos, _)| *pos)
            .collect();

        let ready_set: HashSet<ChunkPos> = self
            .chunks
            .iter()
            .filter(|(_, entry)| {
                matches!(
                    entry.state,
                    ChunkState::Generated | ChunkState::Ready | ChunkState::Dirty
                )
            })
            .map(|(pos, _)| *pos)
            .collect();

        for pos in generated {
            if !self.neighbor_tracker.is_waiting(&pos) {
                // Start tracking if not already
                self.neighbor_tracker.start_waiting(pos, &ready_set);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_manager_empty() {
        let manager = ChunkManager::new(12345, 4);
        assert_eq!(manager.chunk_count(), 0);
        assert_eq!(manager.ready_count(), 0);
    }

    #[test]
    fn update_starts_generation() {
        let mut manager = ChunkManager::new(12345, 2);
        manager.update(Vec3::ZERO);

        // Give rayon time to process
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Process results
        manager.update(Vec3::ZERO);

        assert!(manager.chunk_count() > 0, "Should have loaded some chunks");
    }

    #[test]
    fn set_block_marks_dirty() {
        let mut manager = ChunkManager::new(12345, 2);

        // Generate and ready a chunk
        manager.update(Vec3::ZERO);
        std::thread::sleep(std::time::Duration::from_millis(200));
        manager.update(Vec3::ZERO);

        let center = ChunkPos(IVec3::ZERO);
        if manager.chunks.contains_key(&center) {
            manager.mark_ready(center);
            assert_eq!(manager.get_state(center), Some(ChunkState::Ready));

            // Set a block
            let world_pos = WorldPos(IVec3::new(8, 8, 8));
            manager.set_block(world_pos, BlockId(1));

            assert_eq!(manager.get_state(center), Some(ChunkState::Dirty));
        }
    }

    #[test]
    fn chunks_unload_with_hysteresis() {
        let mut manager = ChunkManager::new(12345, 2);

        // Generate chunks at origin
        manager.update(Vec3::ZERO);
        std::thread::sleep(std::time::Duration::from_millis(200));
        manager.update(Vec3::ZERO);

        let initial_count = manager.chunk_count();

        // Move player far away
        manager.update(Vec3::new(1000.0, 0.0, 1000.0));

        // Old chunks should be unloaded
        assert!(
            manager.chunk_count() < initial_count || initial_count == 0,
            "Chunks should be unloaded when player moves far"
        );
    }
}
