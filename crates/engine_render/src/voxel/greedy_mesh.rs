//! Greedy meshing algorithm for voxel chunks.

use engine_core::coords::{LocalPos, CHUNK_SIZE};
use engine_world::chunk::{BlockId, BlockRegistry, Chunk, AIR};
use glam::Vec3;

use super::ambient_occlusion::{ao_offsets, calculate_ao};
use super::mesh_builder::{normals, MeshBuilder, Vertex};

/// Neighbor chunk data for meshing.
///
/// Provides access to blocks in adjacent chunks for face culling and AO.
#[derive(Debug)]
pub struct ChunkNeighbors<'a> {
    /// Neighbor in -X direction.
    pub neg_x: Option<&'a Chunk>,
    /// Neighbor in +X direction.
    pub pos_x: Option<&'a Chunk>,
    /// Neighbor in -Y direction.
    pub neg_y: Option<&'a Chunk>,
    /// Neighbor in +Y direction.
    pub pos_y: Option<&'a Chunk>,
    /// Neighbor in -Z direction.
    pub neg_z: Option<&'a Chunk>,
    /// Neighbor in +Z direction.
    pub pos_z: Option<&'a Chunk>,
}

impl<'a> ChunkNeighbors<'a> {
    /// Create neighbors with no adjacent chunks.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            neg_x: None,
            pos_x: None,
            neg_y: None,
            pos_y: None,
            neg_z: None,
            pos_z: None,
        }
    }
}

/// Size of the chunk as usize.
const SIZE: usize = CHUNK_SIZE as usize;

/// Generate a mesh for a chunk using greedy meshing.
///
/// Merges adjacent faces of the same block type to reduce vertex count.
/// Calculates per-vertex ambient occlusion.
#[must_use]
pub fn greedy_mesh(
    chunk: &Chunk,
    neighbors: &ChunkNeighbors,
    registry: &BlockRegistry,
) -> MeshBuilder {
    let mut builder = MeshBuilder::with_capacity(1024, 1536);

    // Skip empty chunks
    if chunk.is_empty() {
        return builder;
    }

    // Process each axis direction
    mesh_axis::<0>(chunk, neighbors, registry, &mut builder); // X axis
    mesh_axis::<1>(chunk, neighbors, registry, &mut builder); // Y axis
    mesh_axis::<2>(chunk, neighbors, registry, &mut builder); // Z axis

    builder
}

/// Mesh faces along a single axis.
///
/// Generic over axis: 0=X, 1=Y, 2=Z.
fn mesh_axis<const AXIS: usize>(
    chunk: &Chunk,
    neighbors: &ChunkNeighbors,
    registry: &BlockRegistry,
    builder: &mut MeshBuilder,
) {
    // Determine the two perpendicular axes
    let (u_axis, v_axis) = match AXIS {
        0 => (2, 1), // X axis: u=Z, v=Y
        1 => (0, 2), // Y axis: u=X, v=Z
        _ => (0, 1), // Z axis: u=X, v=Y
    };

    // Process both directions (+/-) along the axis
    for back_face in [false, true] {
        // Track which faces are already processed in each slice
        let mut mask = [[false; SIZE]; SIZE];

        // Process each slice perpendicular to the axis
        for d in 0..SIZE {
            // Clear mask for new slice
            for row in &mut mask {
                row.fill(false);
            }

            // Build face mask for this slice
            let face_data = build_face_mask::<AXIS>(chunk, neighbors, registry, d, back_face);

            // Greedy merge faces
            for v in 0..SIZE {
                for u in 0..SIZE {
                    if mask[v][u] {
                        continue;
                    }

                    let Some(block) = face_data[v][u] else {
                        continue;
                    };

                    // Find maximum width
                    let mut w = 1;
                    while u + w < SIZE && !mask[v][u + w] && face_data[v][u + w] == Some(block) {
                        w += 1;
                    }

                    // Find maximum height with same width
                    let mut h = 1;
                    'outer: while v + h < SIZE {
                        for du in 0..w {
                            if mask[v + h][u + du] || face_data[v + h][u + du] != Some(block) {
                                break 'outer;
                            }
                        }
                        h += 1;
                    }

                    // Mark as processed
                    for dv in 0..h {
                        for du in 0..w {
                            mask[v + dv][u + du] = true;
                        }
                    }

                    // Emit quad
                    emit_quad::<AXIS>(
                        chunk,
                        neighbors,
                        registry,
                        builder,
                        d,
                        u,
                        v,
                        w,
                        h,
                        block,
                        back_face,
                        u_axis,
                        v_axis,
                    );
                }
            }
        }
    }
}

/// Build a mask of visible faces for a slice.
fn build_face_mask<const AXIS: usize>(
    chunk: &Chunk,
    neighbors: &ChunkNeighbors,
    registry: &BlockRegistry,
    d: usize,
    back_face: bool,
) -> [[Option<BlockId>; SIZE]; SIZE] {
    let mut mask = [[None; SIZE]; SIZE];

    let (u_axis, v_axis) = match AXIS {
        0 => (2, 1),
        1 => (0, 2),
        _ => (0, 1),
    };

    for v in 0..SIZE {
        for u in 0..SIZE {
            // Build position based on axis
            let mut pos = [0usize; 3];
            pos[AXIS] = d;
            pos[u_axis] = u;
            pos[v_axis] = v;

            let block = chunk.get(LocalPos::new(pos[0] as u32, pos[1] as u32, pos[2] as u32));

            // Skip air blocks
            if block == AIR {
                continue;
            }

            // Check if face is visible
            let neighbor_block = get_neighbor_block::<AXIS>(chunk, neighbors, pos, back_face);

            // Face is visible if neighbor is air or transparent
            let block_transparent = registry.is_transparent(block);
            let neighbor_solid = registry.is_solid(neighbor_block);
            let neighbor_transparent = registry.is_transparent(neighbor_block);

            // Solid blocks show faces toward air/transparent
            // Transparent blocks only show faces toward air (not toward other transparent)
            let show_face = if block_transparent {
                neighbor_block == AIR
            } else {
                !neighbor_solid || neighbor_transparent
            };

            if show_face {
                mask[v][u] = Some(block);
            }
        }
    }

    mask
}

/// Get the block in the neighbor direction.
fn get_neighbor_block<const AXIS: usize>(
    chunk: &Chunk,
    neighbors: &ChunkNeighbors,
    pos: [usize; 3],
    back_face: bool,
) -> BlockId {
    let mut neighbor_pos = [pos[0] as i32, pos[1] as i32, pos[2] as i32];

    if back_face {
        neighbor_pos[AXIS] -= 1;
    } else {
        neighbor_pos[AXIS] += 1;
    }

    // Check if in this chunk
    if neighbor_pos[AXIS] >= 0 && neighbor_pos[AXIS] < SIZE as i32 {
        chunk.get(LocalPos::new(
            neighbor_pos[0] as u32,
            neighbor_pos[1] as u32,
            neighbor_pos[2] as u32,
        ))
    } else {
        // Sample from neighbor chunk
        let neighbor_chunk = match (AXIS, back_face) {
            (0, true) => neighbors.neg_x,
            (0, false) => neighbors.pos_x,
            (1, true) => neighbors.neg_y,
            (1, false) => neighbors.pos_y,
            (2, true) => neighbors.neg_z,
            (2, false) => neighbors.pos_z,
            _ => None,
        };

        match neighbor_chunk {
            Some(nc) => {
                let wrapped_pos = [
                    (neighbor_pos[0].rem_euclid(SIZE as i32)) as u32,
                    (neighbor_pos[1].rem_euclid(SIZE as i32)) as u32,
                    (neighbor_pos[2].rem_euclid(SIZE as i32)) as u32,
                ];
                nc.get(LocalPos::new(wrapped_pos[0], wrapped_pos[1], wrapped_pos[2]))
            }
            None => AIR, // No neighbor = air (show face)
        }
    }
}

/// Emit a quad for a merged face region.
#[allow(clippy::too_many_arguments)]
fn emit_quad<const AXIS: usize>(
    chunk: &Chunk,
    neighbors: &ChunkNeighbors,
    registry: &BlockRegistry,
    builder: &mut MeshBuilder,
    d: usize,
    u: usize,
    v: usize,
    w: usize,
    h: usize,
    block: BlockId,
    back_face: bool,
    u_axis: usize,
    v_axis: usize,
) {
    let face_idx = match (AXIS, back_face) {
        (0, true) => 0,  // -X
        (0, false) => 1, // +X
        (1, true) => 2,  // -Y
        (1, false) => 3, // +Y
        (2, true) => 4,  // -Z
        (2, false) => 5, // +Z
        _ => 0,
    };

    // Get texture index for this face (used later for texture atlas)
    let _tex_idx = registry
        .get(block)
        .map(|p| p.texture_indices[face_idx])
        .unwrap_or(0);

    // Normal for this face
    let normal = match (AXIS, back_face) {
        (0, true) => normals::NEG_X,
        (0, false) => normals::POS_X,
        (1, true) => normals::NEG_Y,
        (1, false) => normals::POS_Y,
        (2, true) => normals::NEG_Z,
        (2, false) => normals::POS_Z,
        _ => 0,
    };

    // Calculate corner positions
    let face_offset = if back_face { 0.0 } else { 1.0 };
    let mut corners = [[0.0f32; 3]; 4];

    // Build corner positions
    for (i, corner) in corners.iter_mut().enumerate() {
        let cu = if i == 0 || i == 3 { u } else { u + w };
        let cv = if i < 2 { v } else { v + h };

        corner[AXIS] = d as f32 + face_offset;
        corner[u_axis] = cu as f32;
        corner[v_axis] = cv as f32;
    }

    // Calculate AO for each corner
    let ao = calculate_corner_ao::<AXIS>(chunk, neighbors, registry, d, u, v, w, h, back_face);

    // UVs based on quad size
    let uvs = [
        [0.0, h as f32],
        [w as f32, h as f32],
        [w as f32, 0.0],
        [0.0, 0.0],
    ];

    // Create vertices
    let v0 = Vertex::new(Vec3::from_array(corners[0]), normal, uvs[0], ao[0]);
    let v1 = Vertex::new(Vec3::from_array(corners[1]), normal, uvs[1], ao[1]);
    let v2 = Vertex::new(Vec3::from_array(corners[2]), normal, uvs[2], ao[2]);
    let v3 = Vertex::new(Vec3::from_array(corners[3]), normal, uvs[3], ao[3]);

    // Flip winding for back faces
    if back_face {
        builder.add_quad(v0, v3, v2, v1);
    } else {
        builder.add_quad(v0, v1, v2, v3);
    }
}

/// Calculate AO for all 4 corners of a quad.
#[allow(clippy::too_many_arguments)]
fn calculate_corner_ao<const AXIS: usize>(
    chunk: &Chunk,
    neighbors: &ChunkNeighbors,
    registry: &BlockRegistry,
    d: usize,
    u: usize,
    v: usize,
    w: usize,
    h: usize,
    back_face: bool,
) -> [u8; 4] {
    let (u_axis, v_axis) = match AXIS {
        0 => (2, 1),
        1 => (0, 2),
        _ => (0, 1),
    };

    // Corner positions in (u, v) space
    let corner_uvs = [
        (u, v),         // Bottom-left
        (u + w, v),     // Bottom-right
        (u + w, v + h), // Top-right
        (u, v + h),     // Top-left
    ];

    let offsets = match (AXIS, back_face) {
        (0, true) => &ao_offsets::NEG_X,
        (0, false) => &ao_offsets::POS_X,
        (1, true) => &ao_offsets::NEG_Y,
        (1, false) => &ao_offsets::POS_Y,
        (2, true) => &ao_offsets::NEG_Z,
        (2, false) => &ao_offsets::POS_Z,
        _ => &ao_offsets::POS_X,
    };

    let mut ao = [3u8; 4];

    for (i, &(cu, cv)) in corner_uvs.iter().enumerate() {
        let mut base_pos = [0i32; 3];
        base_pos[AXIS] = d as i32;
        base_pos[u_axis] = cu as i32;
        base_pos[v_axis] = cv as i32;

        let corner_offsets = &offsets[i];

        let check_solid = |offset: [i32; 3]| -> bool {
            let check_pos = [
                base_pos[0] + offset[0],
                base_pos[1] + offset[1],
                base_pos[2] + offset[2],
            ];
            let block = sample_block(chunk, neighbors, check_pos);
            registry.is_solid(block)
        };

        let side1 = check_solid(corner_offsets[0]);
        let side2 = check_solid(corner_offsets[1]);
        let corner = check_solid(corner_offsets[2]);

        ao[i] = calculate_ao(side1, side2, corner);
    }

    ao
}

/// Sample a block at any position, including neighbor chunks.
fn sample_block(chunk: &Chunk, neighbors: &ChunkNeighbors, pos: [i32; 3]) -> BlockId {
    let in_bounds = pos.iter().all(|&c| c >= 0 && c < SIZE as i32);

    if in_bounds {
        chunk.get(LocalPos::new(pos[0] as u32, pos[1] as u32, pos[2] as u32))
    } else {
        // Determine which neighbor
        let neighbor_chunk = if pos[0] < 0 {
            neighbors.neg_x
        } else if pos[0] >= SIZE as i32 {
            neighbors.pos_x
        } else if pos[1] < 0 {
            neighbors.neg_y
        } else if pos[1] >= SIZE as i32 {
            neighbors.pos_y
        } else if pos[2] < 0 {
            neighbors.neg_z
        } else if pos[2] >= SIZE as i32 {
            neighbors.pos_z
        } else {
            None
        };

        match neighbor_chunk {
            Some(nc) => {
                let wrapped = [
                    pos[0].rem_euclid(SIZE as i32) as u32,
                    pos[1].rem_euclid(SIZE as i32) as u32,
                    pos[2].rem_euclid(SIZE as i32) as u32,
                ];
                nc.get(LocalPos::new(wrapped[0], wrapped[1], wrapped[2]))
            }
            None => AIR,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_world::chunk::STONE;

    fn test_registry() -> BlockRegistry {
        BlockRegistry::default()
    }

    #[test]
    fn test_empty_chunk_no_mesh() {
        let chunk = Chunk::new();
        let neighbors = ChunkNeighbors::empty();
        let registry = test_registry();

        let mesh = greedy_mesh(&chunk, &neighbors, &registry);
        assert!(mesh.is_empty());
    }

    #[test]
    fn test_single_block_six_faces() {
        let mut chunk = Chunk::new();
        chunk.set(LocalPos::new(8, 8, 8), STONE);

        let neighbors = ChunkNeighbors::empty();
        let registry = test_registry();

        let mesh = greedy_mesh(&chunk, &neighbors, &registry);

        // Single block = 6 faces = 24 vertices, 36 indices
        assert_eq!(mesh.vertex_count(), 24);
        assert_eq!(mesh.index_count(), 36);
    }

    #[test]
    fn test_two_adjacent_blocks_merge() {
        let mut chunk = Chunk::new();
        chunk.set(LocalPos::new(8, 8, 8), STONE);
        chunk.set(LocalPos::new(9, 8, 8), STONE);

        let neighbors = ChunkNeighbors::empty();
        let registry = test_registry();

        let mesh = greedy_mesh(&chunk, &neighbors, &registry);

        // Two adjacent blocks along X:
        // - Shared face is culled (internal face hidden)
        // - 2 end faces (-X of first, +X of second)
        // - Top/bottom/front/back each merge into 2x1 quads (4 quads)
        // Total: 6 faces = 24 vertices, 36 indices
        assert_eq!(mesh.vertex_count(), 24);
        assert_eq!(mesh.index_count(), 36);
    }

    #[test]
    fn test_ao_calculated() {
        let mut chunk = Chunk::new();
        // Place a corner configuration
        chunk.set(LocalPos::new(0, 0, 0), STONE);
        chunk.set(LocalPos::new(1, 0, 0), STONE);
        chunk.set(LocalPos::new(0, 1, 0), STONE);
        chunk.set(LocalPos::new(0, 0, 1), STONE);

        let neighbors = ChunkNeighbors::empty();
        let registry = test_registry();

        let mesh = greedy_mesh(&chunk, &neighbors, &registry);

        // Verify mesh was created
        assert!(!mesh.is_empty());

        // Check that some vertices have AO < 3 (occluded)
        let has_ao = mesh.vertices().iter().any(|v| v.ao < 3);
        assert!(has_ao, "Should have some occluded vertices");
    }
}
