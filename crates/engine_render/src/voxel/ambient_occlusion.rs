//! Per-vertex ambient occlusion calculation for voxel meshes.

/// Calculate ambient occlusion value for a vertex.
///
/// Uses the standard 3-neighbor approach: check side1, side2, and corner.
/// Returns a value 0-3 where 0 is fully occluded and 3 is no occlusion.
///
/// # Arguments
/// * `side1` - Whether the first side neighbor is solid.
/// * `side2` - Whether the second side neighbor is solid.
/// * `corner` - Whether the corner neighbor is solid.
#[must_use]
pub fn calculate_ao(side1: bool, side2: bool, corner: bool) -> u8 {
    if side1 && side2 {
        // Both sides solid = corner is fully occluded
        0
    } else {
        // Count solid neighbors
        3 - (side1 as u8 + side2 as u8 + corner as u8)
    }
}

/// AO offsets for each corner of a face.
///
/// For each face direction and corner, provides the two side offsets and corner offset
/// to check for occlusion. Order is [side1, side2, corner] for each of 4 corners.
pub mod ao_offsets {
    /// Offsets for -X face (looking from negative X toward positive X).
    /// Corners ordered: bottom-left, bottom-right, top-right, top-left (CCW).
    pub const NEG_X: [[[i32; 3]; 3]; 4] = [
        [[-1, -1, 0], [-1, 0, -1], [-1, -1, -1]], // BL
        [[-1, -1, 0], [-1, 0, 1], [-1, -1, 1]],   // BR
        [[-1, 1, 0], [-1, 0, 1], [-1, 1, 1]],     // TR
        [[-1, 1, 0], [-1, 0, -1], [-1, 1, -1]],   // TL
    ];

    /// Offsets for +X face.
    pub const POS_X: [[[i32; 3]; 3]; 4] = [
        [[1, -1, 0], [1, 0, 1], [1, -1, 1]],   // BL
        [[1, -1, 0], [1, 0, -1], [1, -1, -1]], // BR
        [[1, 1, 0], [1, 0, -1], [1, 1, -1]],   // TR
        [[1, 1, 0], [1, 0, 1], [1, 1, 1]],     // TL
    ];

    /// Offsets for -Y face (bottom).
    pub const NEG_Y: [[[i32; 3]; 3]; 4] = [
        [[0, -1, -1], [-1, -1, 0], [-1, -1, -1]], // BL
        [[0, -1, -1], [1, -1, 0], [1, -1, -1]],   // BR
        [[0, -1, 1], [1, -1, 0], [1, -1, 1]],     // TR
        [[0, -1, 1], [-1, -1, 0], [-1, -1, 1]],   // TL
    ];

    /// Offsets for +Y face (top).
    pub const POS_Y: [[[i32; 3]; 3]; 4] = [
        [[0, 1, 1], [-1, 1, 0], [-1, 1, 1]],   // BL
        [[0, 1, 1], [1, 1, 0], [1, 1, 1]],     // BR
        [[0, 1, -1], [1, 1, 0], [1, 1, -1]],   // TR
        [[0, 1, -1], [-1, 1, 0], [-1, 1, -1]], // TL
    ];

    /// Offsets for -Z face.
    pub const NEG_Z: [[[i32; 3]; 3]; 4] = [
        [[1, -1, -1], [0, -1, -1], [1, 0, -1]],   // BL
        [[-1, -1, -1], [0, -1, -1], [-1, 0, -1]], // BR
        [[-1, 1, -1], [0, 1, -1], [-1, 0, -1]],   // TR
        [[1, 1, -1], [0, 1, -1], [1, 0, -1]],     // TL
    ];

    /// Offsets for +Z face.
    pub const POS_Z: [[[i32; 3]; 3]; 4] = [
        [[-1, -1, 1], [0, -1, 1], [-1, 0, 1]], // BL
        [[1, -1, 1], [0, -1, 1], [1, 0, 1]],   // BR
        [[1, 1, 1], [0, 1, 1], [1, 0, 1]],     // TR
        [[-1, 1, 1], [0, 1, 1], [-1, 0, 1]],   // TL
    ];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ao_no_occlusion() {
        assert_eq!(calculate_ao(false, false, false), 3);
    }

    #[test]
    fn test_ao_full_occlusion() {
        assert_eq!(calculate_ao(true, true, false), 0);
        assert_eq!(calculate_ao(true, true, true), 0);
    }

    #[test]
    fn test_ao_partial_occlusion() {
        assert_eq!(calculate_ao(true, false, false), 2);
        assert_eq!(calculate_ao(false, true, false), 2);
        assert_eq!(calculate_ao(false, false, true), 2);
        assert_eq!(calculate_ao(true, false, true), 1);
        assert_eq!(calculate_ao(false, true, true), 1);
    }
}
