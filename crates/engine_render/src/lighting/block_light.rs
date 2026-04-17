//! Block-based light propagation using BFS.
//!
//! Each block has a light value 0-15. Light propagates from sources
//! (torches, glowing blocks) and from the sky downward.

use std::collections::VecDeque;

use engine_core::coords::LocalPos;
use glam::IVec3;

/// Maximum light value (full brightness).
pub const LIGHT_MAX: u8 = 15;

/// Chunk size for light calculations.
const CHUNK_SIZE: i32 = 16;

/// A light value stored per block (0-15).
pub type LightValue = u8;

/// Stores light values for a single chunk.
#[derive(Debug, Clone)]
pub struct BlockLightMap {
    /// Block light from torches, glowing blocks etc. (0-15 per block)
    block_light: [LightValue; 4096],
    /// Sunlight propagated from sky (0-15 per block)
    sky_light: [LightValue; 4096],
    /// Whether the light map needs recalculation.
    dirty: bool,
}

impl Default for BlockLightMap {
    fn default() -> Self {
        Self::new()
    }
}

impl BlockLightMap {
    /// Create a new empty light map.
    #[must_use]
    pub fn new() -> Self {
        Self {
            block_light: [0; 4096],
            sky_light: [0; 4096],
            dirty: true,
        }
    }

    /// Convert 3D position to flat array index.
    #[inline]
    fn index(x: i32, y: i32, z: i32) -> usize {
        debug_assert!((0..CHUNK_SIZE).contains(&x));
        debug_assert!((0..CHUNK_SIZE).contains(&y));
        debug_assert!((0..CHUNK_SIZE).contains(&z));
        (x + y * CHUNK_SIZE + z * CHUNK_SIZE * CHUNK_SIZE) as usize
    }

    /// Check if position is within chunk bounds.
    #[inline]
    fn in_bounds(x: i32, y: i32, z: i32) -> bool {
        (0..CHUNK_SIZE).contains(&x)
            && (0..CHUNK_SIZE).contains(&y)
            && (0..CHUNK_SIZE).contains(&z)
    }

    /// Get block light at position.
    #[must_use]
    pub fn get_block_light(&self, pos: LocalPos) -> LightValue {
        let p = pos.0;
        self.block_light[Self::index(p.x as i32, p.y as i32, p.z as i32)]
    }

    /// Get sky light at position.
    #[must_use]
    pub fn get_sky_light(&self, pos: LocalPos) -> LightValue {
        let p = pos.0;
        self.sky_light[Self::index(p.x as i32, p.y as i32, p.z as i32)]
    }

    /// Get combined light level (max of block and sky light).
    #[must_use]
    pub fn get_combined_light(&self, pos: LocalPos) -> LightValue {
        self.get_block_light(pos).max(self.get_sky_light(pos))
    }

    /// Set block light at position.
    pub fn set_block_light(&mut self, pos: LocalPos, value: LightValue) {
        let p = pos.0;
        self.block_light[Self::index(p.x as i32, p.y as i32, p.z as i32)] = value.min(LIGHT_MAX);
        self.dirty = true;
    }

    /// Set sky light at position.
    pub fn set_sky_light(&mut self, pos: LocalPos, value: LightValue) {
        let p = pos.0;
        self.sky_light[Self::index(p.x as i32, p.y as i32, p.z as i32)] = value.min(LIGHT_MAX);
        self.dirty = true;
    }

    /// Check if light map needs recalculation.
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Clear dirty flag.
    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    /// Mark as needing recalculation.
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Calculate block light propagation using BFS.
    ///
    /// `light_sources` contains (local_position, light_level) for each light-emitting block.
    /// `is_opaque` returns true if a block at the given position blocks light.
    pub fn propagate_block_light<F>(&mut self, light_sources: &[(LocalPos, LightValue)], is_opaque: F)
    where
        F: Fn(i32, i32, i32) -> bool,
    {
        // Clear existing block light
        self.block_light.fill(0);

        let mut queue: VecDeque<(i32, i32, i32, LightValue)> = VecDeque::new();

        // Initialize queue with light sources
        for (pos, level) in light_sources {
            let p = pos.0;
            let (x, y, z) = (p.x as i32, p.y as i32, p.z as i32);
            self.block_light[Self::index(x, y, z)] = *level;
            queue.push_back((x, y, z, *level));
        }

        // BFS propagation
        Self::bfs_propagate(&mut queue, &mut self.block_light, &is_opaque);

        self.dirty = false;
    }

    /// Calculate sky light propagation from above.
    ///
    /// `is_opaque` returns true if a block blocks light.
    /// `top_sky_light` is the sky light value at the top of the chunk (usually 15 during day).
    pub fn propagate_sky_light<F>(&mut self, top_sky_light: LightValue, is_opaque: F)
    where
        F: Fn(i32, i32, i32) -> bool,
    {
        // Clear existing sky light
        self.sky_light.fill(0);

        let mut queue: VecDeque<(i32, i32, i32, LightValue)> = VecDeque::new();

        // Initialize from top layer
        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let y = CHUNK_SIZE - 1;
                if !is_opaque(x, y, z) {
                    self.sky_light[Self::index(x, y, z)] = top_sky_light;
                    queue.push_back((x, y, z, top_sky_light));
                }
            }
        }

        // Special case: sunlight propagates straight down without diminishing
        for x in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let mut current_light = top_sky_light;
                for y in (0..CHUNK_SIZE).rev() {
                    if is_opaque(x, y, z) {
                        current_light = 0;
                    } else if current_light == top_sky_light {
                        // Direct sunlight column
                        self.sky_light[Self::index(x, y, z)] = current_light;
                    }
                }
            }
        }

        // Reinitialize queue with all sky-lit blocks for horizontal propagation
        queue.clear();
        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    let level = self.sky_light[Self::index(x, y, z)];
                    if level > 1 {
                        queue.push_back((x, y, z, level));
                    }
                }
            }
        }

        // BFS for horizontal spread
        Self::bfs_propagate(&mut queue, &mut self.sky_light, &is_opaque);
    }

    /// BFS light propagation helper.
    fn bfs_propagate<F>(
        queue: &mut VecDeque<(i32, i32, i32, LightValue)>,
        light_data: &mut [LightValue; 4096],
        is_opaque: &F,
    ) where
        F: Fn(i32, i32, i32) -> bool,
    {
        const NEIGHBORS: [(i32, i32, i32); 6] = [
            (-1, 0, 0),
            (1, 0, 0),
            (0, -1, 0),
            (0, 1, 0),
            (0, 0, -1),
            (0, 0, 1),
        ];

        while let Some((x, y, z, level)) = queue.pop_front() {
            if level <= 1 {
                continue;
            }

            let new_level = level - 1;

            for (dx, dy, dz) in NEIGHBORS {
                let (nx, ny, nz) = (x + dx, y + dy, z + dz);

                if !Self::in_bounds(nx, ny, nz) {
                    continue;
                }

                if is_opaque(nx, ny, nz) {
                    continue;
                }

                let idx = Self::index(nx, ny, nz);
                if light_data[idx] < new_level {
                    light_data[idx] = new_level;
                    queue.push_back((nx, ny, nz, new_level));
                }
            }
        }
    }

    /// Handle block change at position.
    ///
    /// Call this when a block is placed or removed to update lighting.
    pub fn on_block_changed(&mut self, pos: LocalPos) {
        // For now, just mark dirty. Full incremental update would be more complex.
        let _ = pos;
        self.dirty = true;
    }

    /// Get light data as a flat array for GPU upload.
    ///
    /// Returns combined light values packed into bytes.
    #[must_use]
    pub fn as_gpu_data(&self) -> Vec<u8> {
        // Pack block and sky light into single byte each, or combine them
        self.block_light
            .iter()
            .zip(self.sky_light.iter())
            .map(|(&block, &sky)| block.max(sky))
            .collect()
    }

    /// Get light data with separate channels for GPU.
    ///
    /// Returns (block_light, sky_light) pairs packed as u16.
    #[must_use]
    pub fn as_gpu_data_dual_channel(&self) -> Vec<u16> {
        self.block_light
            .iter()
            .zip(self.sky_light.iter())
            .map(|(&block, &sky)| u16::from(block) | (u16::from(sky) << 8))
            .collect()
    }
}

/// Neighbor light data for cross-chunk propagation.
#[derive(Debug, Clone, Copy)]
pub struct NeighborLightData {
    /// Light values at the boundary faces.
    pub neg_x: [[LightValue; 16]; 16], // YZ face
    pub pos_x: [[LightValue; 16]; 16],
    pub neg_y: [[LightValue; 16]; 16], // XZ face
    pub pos_y: [[LightValue; 16]; 16],
    pub neg_z: [[LightValue; 16]; 16], // XY face
    pub pos_z: [[LightValue; 16]; 16],
}

impl Default for NeighborLightData {
    fn default() -> Self {
        Self {
            neg_x: [[0; 16]; 16],
            pos_x: [[0; 16]; 16],
            neg_y: [[0; 16]; 16],
            pos_y: [[0; 16]; 16],
            neg_z: [[0; 16]; 16],
            pos_z: [[0; 16]; 16],
        }
    }
}

impl NeighborLightData {
    /// Extract boundary light values from a light map.
    #[must_use]
    pub fn extract_from(map: &BlockLightMap) -> Self {
        let mut data = Self::default();

        for y in 0..16 {
            for z in 0..16 {
                let pos = LocalPos(glam::UVec3::new(0, y as u32, z as u32));
                data.neg_x[y][z] = map.get_combined_light(pos);

                let pos = LocalPos(glam::UVec3::new(15, y as u32, z as u32));
                data.pos_x[y][z] = map.get_combined_light(pos);
            }
        }

        for x in 0..16 {
            for z in 0..16 {
                let pos = LocalPos(glam::UVec3::new(x as u32, 0, z as u32));
                data.neg_y[x][z] = map.get_combined_light(pos);

                let pos = LocalPos(glam::UVec3::new(x as u32, 15, z as u32));
                data.pos_y[x][z] = map.get_combined_light(pos);
            }
        }

        for x in 0..16 {
            for y in 0..16 {
                let pos = LocalPos(glam::UVec3::new(x as u32, y as u32, 0));
                data.neg_z[x][y] = map.get_combined_light(pos);

                let pos = LocalPos(glam::UVec3::new(x as u32, y as u32, 15));
                data.pos_z[x][y] = map.get_combined_light(pos);
            }
        }

        data
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::UVec3;

    #[test]
    fn test_new_light_map() {
        let map = BlockLightMap::new();
        let pos = LocalPos(UVec3::new(8, 8, 8));
        assert_eq!(map.get_block_light(pos), 0);
        assert_eq!(map.get_sky_light(pos), 0);
    }

    #[test]
    fn test_set_get_light() {
        let mut map = BlockLightMap::new();
        let pos = LocalPos(UVec3::new(5, 10, 3));

        map.set_block_light(pos, 12);
        assert_eq!(map.get_block_light(pos), 12);

        map.set_sky_light(pos, 15);
        assert_eq!(map.get_sky_light(pos), 15);
    }

    #[test]
    fn test_combined_light() {
        let mut map = BlockLightMap::new();
        let pos = LocalPos(UVec3::new(0, 0, 0));

        map.set_block_light(pos, 8);
        map.set_sky_light(pos, 5);
        assert_eq!(map.get_combined_light(pos), 8);

        map.set_sky_light(pos, 12);
        assert_eq!(map.get_combined_light(pos), 12);
    }

    #[test]
    fn test_light_clamped_to_max() {
        let mut map = BlockLightMap::new();
        let pos = LocalPos(UVec3::new(0, 0, 0));

        map.set_block_light(pos, 20); // Over max
        assert_eq!(map.get_block_light(pos), LIGHT_MAX);
    }

    #[test]
    fn test_block_light_propagation() {
        let mut map = BlockLightMap::new();
        let source_pos = LocalPos(UVec3::new(8, 8, 8));

        // No opaque blocks
        let is_opaque = |_x: i32, _y: i32, _z: i32| false;

        map.propagate_block_light(&[(source_pos, 15)], is_opaque);

        // Source should be 15
        assert_eq!(map.get_block_light(source_pos), 15);

        // Adjacent should be 14
        let adjacent = LocalPos(UVec3::new(9, 8, 8));
        assert_eq!(map.get_block_light(adjacent), 14);

        // Two away should be 13
        let two_away = LocalPos(UVec3::new(10, 8, 8));
        assert_eq!(map.get_block_light(two_away), 13);
    }

    #[test]
    fn test_opaque_blocks_stop_light() {
        let mut map = BlockLightMap::new();
        let source_pos = LocalPos(UVec3::new(8, 8, 8));

        // Block at (9, 8, 8) is opaque
        let is_opaque = |x: i32, _y: i32, _z: i32| x == 9;

        map.propagate_block_light(&[(source_pos, 15)], is_opaque);

        // Past the opaque block should be darker (light goes around)
        let past_block = LocalPos(UVec3::new(10, 8, 8));
        let light = map.get_block_light(past_block);
        assert!(light < 13, "Light should be blocked, got {}", light);
    }

    #[test]
    fn test_sky_light_propagation() {
        let mut map = BlockLightMap::new();

        // No opaque blocks
        let is_opaque = |_x: i32, _y: i32, _z: i32| false;

        map.propagate_sky_light(15, is_opaque);

        // Top should be 15
        let top = LocalPos(UVec3::new(8, 15, 8));
        assert_eq!(map.get_sky_light(top), 15);

        // Bottom should also be 15 (direct sunlight column)
        let bottom = LocalPos(UVec3::new(8, 0, 8));
        assert_eq!(map.get_sky_light(bottom), 15);
    }

    #[test]
    fn test_dirty_flag() {
        let mut map = BlockLightMap::new();
        assert!(map.is_dirty());

        map.clear_dirty();
        assert!(!map.is_dirty());

        map.set_block_light(LocalPos(UVec3::ZERO), 5);
        assert!(map.is_dirty());
    }
}
