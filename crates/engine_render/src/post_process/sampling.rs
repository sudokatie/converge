//! Camera sampling utilities for post-processing.
//!
//! Provides deterministic sampling helpers for evaluating post-processing
//! effects relative to camera position and view frustum.

use glam::{Mat4, Vec3, Vec4};
use std::f32::consts::TAU;

const HASH_MUL_A: u32 = 0x85eb_ca6b;
const HASH_MUL_B: u32 = 0xc2b2_ae35;

/// Camera state for post-processing evaluation.
#[derive(Debug, Clone, Copy)]
pub struct PostCameraState {
    /// Camera world position.
    pub position: Vec3,
    /// Camera forward direction.
    pub forward: Vec3,
    /// Camera up direction.
    pub up: Vec3,
    /// Camera right direction.
    pub right: Vec3,
    /// View matrix.
    pub view: Mat4,
    /// Projection matrix.
    pub projection: Mat4,
    /// Near plane distance.
    pub near: f32,
    /// Far plane distance.
    pub far: f32,
    /// Vertical field of view in radians.
    pub fov: f32,
    /// Aspect ratio (width / height).
    pub aspect: f32,
}

impl PostCameraState {
    /// Create a new camera state.
    #[must_use]
    pub fn new(
        position: Vec3,
        forward: Vec3,
        up: Vec3,
        fov: f32,
        aspect: f32,
        near: f32,
        far: f32,
    ) -> Self {
        let forward = forward.normalize();
        let right = forward.cross(up).normalize();
        let up = right.cross(forward).normalize();

        let view = Mat4::look_to_rh(position, forward, up);
        let projection = Mat4::perspective_rh(fov, aspect, near, far);

        Self {
            position,
            forward,
            up,
            right,
            view,
            projection,
            near,
            far,
            fov,
            aspect,
        }
    }

    /// Create from view and projection matrices.
    #[must_use]
    pub fn from_matrices(view: Mat4, projection: Mat4, near: f32, far: f32) -> Self {
        let view_inv = view.inverse();
        let position = view_inv.w_axis.truncate();
        let forward = -view_inv.z_axis.truncate().normalize();
        let up = view_inv.y_axis.truncate().normalize();
        let right = view_inv.x_axis.truncate().normalize();

        Self {
            position,
            forward,
            up,
            right,
            view,
            projection,
            near,
            far,
            fov: 1.0,
            aspect: 1.0,
        }
    }

    /// Get the view-projection matrix.
    #[must_use]
    pub fn view_projection(&self) -> Mat4 {
        self.projection * self.view
    }

    /// Transform world position to clip space.
    #[must_use]
    pub fn world_to_clip(&self, world_pos: Vec3) -> Vec4 {
        self.view_projection() * world_pos.extend(1.0)
    }

    /// Transform world position to normalized device coordinates.
    #[must_use]
    pub fn world_to_ndc(&self, world_pos: Vec3) -> Vec3 {
        let clip = self.world_to_clip(world_pos);
        if clip.w.abs() < 0.0001 {
            return Vec3::ZERO;
        }
        Vec3::new(clip.x / clip.w, clip.y / clip.w, clip.z / clip.w)
    }

    /// Transform world position to screen UV (0-1 range).
    #[must_use]
    pub fn world_to_uv(&self, world_pos: Vec3) -> (f32, f32) {
        let ndc = self.world_to_ndc(world_pos);
        ((ndc.x + 1.0) * 0.5, (1.0 - ndc.y) * 0.5)
    }

    /// Get linear depth (0 = near, 1 = far) for a world position.
    #[must_use]
    pub fn linear_depth(&self, world_pos: Vec3) -> f32 {
        let view_pos = self.view * world_pos.extend(1.0);
        let z = -view_pos.z;
        if z <= self.near {
            0.0
        } else if z >= self.far {
            1.0
        } else {
            (z - self.near) / (self.far - self.near)
        }
    }

    /// Check if a world position is in front of the camera.
    #[must_use]
    pub fn is_in_front(&self, world_pos: Vec3) -> bool {
        (world_pos - self.position).dot(self.forward) > 0.0
    }

    /// Check if a world position is within the view frustum (approximate).
    #[must_use]
    pub fn is_visible(&self, world_pos: Vec3) -> bool {
        let ndc = self.world_to_ndc(world_pos);
        ndc.x >= -1.0
            && ndc.x <= 1.0
            && ndc.y >= -1.0
            && ndc.y <= 1.0
            && ndc.z >= 0.0
            && ndc.z <= 1.0
    }

    /// Get distance from camera to a world position.
    #[must_use]
    pub fn distance_to(&self, world_pos: Vec3) -> f32 {
        (world_pos - self.position).length()
    }

    /// Get view-aligned distance (depth) to a world position.
    #[must_use]
    pub fn depth_to(&self, world_pos: Vec3) -> f32 {
        (world_pos - self.position).dot(self.forward)
    }
}

impl Default for PostCameraState {
    fn default() -> Self {
        Self::new(
            Vec3::ZERO,
            Vec3::NEG_Z,
            Vec3::Y,
            std::f32::consts::FRAC_PI_4,
            16.0 / 9.0,
            0.1,
            1000.0,
        )
    }
}

/// Deterministic sampler for post-processing effects.
#[derive(Debug, Clone, Copy)]
pub struct PostSampler {
    /// Seed for deterministic variation.
    pub seed: u32,
    /// Current time for animated effects.
    pub time: f32,
    /// Jitter offset for TAA-style sampling.
    pub jitter: (f32, f32),
}

impl PostSampler {
    /// Create a new sampler.
    #[must_use]
    pub fn new(seed: u32) -> Self {
        Self {
            seed,
            time: 0.0,
            jitter: (0.0, 0.0),
        }
    }

    /// Set time.
    #[must_use]
    pub fn with_time(mut self, time: f32) -> Self {
        self.time = time;
        self
    }

    /// Set jitter offset.
    #[must_use]
    pub fn with_jitter(mut self, jitter_x: f32, jitter_y: f32) -> Self {
        self.jitter = (jitter_x, jitter_y);
        self
    }

    /// Sample noise at screen UV coordinates.
    #[must_use]
    #[allow(clippy::many_single_char_names)]
    pub fn sample_screen_noise(&self, u: f32, v: f32) -> f32 {
        let x = (u + self.jitter.0) * 100.0;
        let y = (v + self.jitter.1) * 100.0;
        let t = self.time * 0.1;
        self.hash_2d(x + t, y + t * 0.7)
    }

    /// Sample film grain pattern.
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "frame index from time is small and positive"
    )]
    pub fn sample_grain(&self, u: f32, v: f32, intensity: f32) -> f32 {
        let frame_seed = self.seed.wrapping_add((self.time * 24.0) as u32);
        let noise = Self::hash_with_seed(u * 200.0, v * 200.0, frame_seed);
        (noise - 0.5) * intensity * 2.0
    }

    /// Sample vignette factor at screen UV.
    #[must_use]
    pub fn sample_vignette(&self, u: f32, v: f32, radius: f32, strength: f32) -> f32 {
        let dx = u - 0.5;
        let dy = v - 0.5;
        let dist = (dx * dx + dy * dy).sqrt() * 2.0;
        if dist <= radius {
            1.0
        } else {
            let falloff = (dist - radius) / (1.0 - radius);
            (1.0 - falloff * strength).max(0.0)
        }
    }

    /// Sample chromatic aberration offset.
    #[must_use]
    pub fn sample_chromatic_offset(&self, u: f32, v: f32, strength: f32) -> (f32, f32, f32) {
        let dx = u - 0.5;
        let dy = v - 0.5;
        let dist = (dx * dx + dy * dy).sqrt();
        let offset = dist * strength;
        (-offset, 0.0, offset)
    }

    /// Sample depth of field circle of confusion.
    #[must_use]
    pub fn sample_coc(&self, depth: f32, focus_dist: f32, aperture: f32) -> f32 {
        let diff = (depth - focus_dist).abs();
        (diff * aperture).min(1.0)
    }

    /// Generate deterministic sample positions for blur kernel.
    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "sample count is small")]
    pub fn blur_samples(&self, sample_count: u32) -> Vec<(f32, f32)> {
        let mut samples = Vec::with_capacity(sample_count as usize);
        let golden_angle = TAU / (1.0 + (5.0_f32).sqrt()) * 0.5;

        for i in 0..sample_count {
            let r = (i as f32 + 0.5) / sample_count as f32;
            let theta = i as f32 * golden_angle;
            samples.push((r * theta.cos(), r * theta.sin()));
        }
        samples
    }

    fn hash_2d(&self, x: f32, y: f32) -> f32 {
        Self::hash_with_seed(x, y, self.seed)
    }

    fn hash_with_seed(x: f32, y: f32, seed: u32) -> f32 {
        let xi = x.to_bits();
        let yi = y.to_bits();
        let mut n = seed.wrapping_add(xi).wrapping_add(yi.wrapping_mul(57));
        n = n.wrapping_mul(HASH_MUL_A);
        n ^= n >> 13;
        n = n.wrapping_mul(HASH_MUL_B);
        n ^= n >> 16;
        u32_to_unit(n)
    }
}

impl Default for PostSampler {
    fn default() -> Self {
        Self::new(0)
    }
}

#[expect(
    clippy::cast_precision_loss,
    reason = "masked to 23 bits; fits in f32 mantissa"
)]
fn u32_to_unit(n: u32) -> f32 {
    (n & 0x7F_FFFF) as f32 / 0x7F_FFFF_u32 as f32
}

/// Hash a position to a deterministic value (0-1).
#[must_use]
pub fn position_hash(x: f32, y: f32, z: f32, seed: u32) -> f32 {
    let xi = x.to_bits();
    let yi = y.to_bits();
    let zi = z.to_bits();
    let mut n = seed
        .wrapping_add(xi)
        .wrapping_add(yi.wrapping_mul(57))
        .wrapping_add(zi.wrapping_mul(113));
    n = n.wrapping_mul(HASH_MUL_A);
    n ^= n >> 13;
    n = n.wrapping_mul(HASH_MUL_B);
    n ^= n >> 16;
    u32_to_unit(n)
}

/// Compute temporal jitter for TAA-style sampling.
#[must_use]
#[expect(clippy::cast_precision_loss, reason = "halton base is small")]
pub fn halton_jitter(frame: u32, base: u32) -> f32 {
    let mut f = 1.0;
    let mut result = 0.0;
    let mut i = frame;

    while i > 0 {
        f /= base as f32;
        result += f * (i % base) as f32;
        i /= base;
    }

    result - 0.5
}

/// Generate Halton sequence jitter for frame.
#[must_use]
pub fn frame_jitter(frame: u32) -> (f32, f32) {
    (halton_jitter(frame, 2), halton_jitter(frame, 3))
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_camera_state_creation() {
        let camera = PostCameraState::new(
            Vec3::new(0.0, 5.0, 10.0),
            Vec3::NEG_Z,
            Vec3::Y,
            1.0,
            16.0 / 9.0,
            0.1,
            1000.0,
        );

        assert_relative_eq!(camera.position.y, 5.0, epsilon = 0.001);
        assert_relative_eq!(camera.forward.z, -1.0, epsilon = 0.001);
    }

    #[test]
    fn test_camera_is_in_front() {
        let camera = PostCameraState::default();

        assert!(camera.is_in_front(Vec3::new(0.0, 0.0, -10.0)));
        assert!(!camera.is_in_front(Vec3::new(0.0, 0.0, 10.0)));
    }

    #[test]
    fn test_camera_distance_to() {
        let camera = PostCameraState::default();
        let dist = camera.distance_to(Vec3::new(3.0, 4.0, 0.0));
        assert_relative_eq!(dist, 5.0, epsilon = 0.001);
    }

    #[test]
    fn test_camera_depth_to() {
        let camera = PostCameraState::default();
        let depth = camera.depth_to(Vec3::new(0.0, 0.0, -10.0));
        assert_relative_eq!(depth, 10.0, epsilon = 0.001);
    }

    #[test]
    fn test_camera_linear_depth() {
        let camera = PostCameraState::new(Vec3::ZERO, Vec3::NEG_Z, Vec3::Y, 1.0, 1.0, 1.0, 100.0);

        assert_relative_eq!(
            camera.linear_depth(Vec3::new(0.0, 0.0, -1.0)),
            0.0,
            epsilon = 0.001
        );
        assert_relative_eq!(
            camera.linear_depth(Vec3::new(0.0, 0.0, -100.0)),
            1.0,
            epsilon = 0.001
        );
    }

    #[test]
    fn test_sampler_determinism() {
        let sampler = PostSampler::new(42);
        let v1 = sampler.sample_screen_noise(0.5, 0.5);
        let v2 = sampler.sample_screen_noise(0.5, 0.5);
        assert_relative_eq!(v1, v2, epsilon = 0.0001);
    }

    #[test]
    fn test_sampler_seed_variation() {
        let sampler1 = PostSampler::new(42);
        let sampler2 = PostSampler::new(123);
        let v1 = sampler1.sample_screen_noise(0.5, 0.5);
        let v2 = sampler2.sample_screen_noise(0.5, 0.5);
        assert!((v1 - v2).abs() > 0.01);
    }

    #[test]
    fn test_sampler_grain_range() {
        let sampler = PostSampler::new(0);
        let coords: [(f32, f32); 8] = [
            (0.0, 0.0),
            (0.1, 0.13),
            (0.25, 0.33),
            (0.5, 0.65),
            (0.75, 0.78),
            (0.9, 0.91),
            (0.99, 0.87),
            (0.42, 0.55),
        ];
        for (u, v) in coords {
            let grain = sampler.sample_grain(u, v, 1.0);
            assert!((-1.0..=1.0).contains(&grain));
        }
    }

    #[test]
    fn test_sampler_vignette_center() {
        let sampler = PostSampler::new(0);
        let vignette = sampler.sample_vignette(0.5, 0.5, 0.5, 0.5);
        assert_relative_eq!(vignette, 1.0, epsilon = 0.001);
    }

    #[test]
    fn test_sampler_vignette_edge() {
        let sampler = PostSampler::new(0);
        let vignette = sampler.sample_vignette(0.0, 0.0, 0.5, 0.5);
        assert!(vignette < 1.0);
        assert!(vignette >= 0.0);
    }

    #[test]
    fn test_sampler_chromatic_offset() {
        let sampler = PostSampler::new(0);
        let (r, g, b) = sampler.sample_chromatic_offset(0.0, 0.5, 0.1);
        assert!(r < 0.0);
        assert_relative_eq!(g, 0.0, epsilon = 0.001);
        assert!(b > 0.0);
    }

    #[test]
    fn test_sampler_coc() {
        let sampler = PostSampler::new(0);
        let coc_focus = sampler.sample_coc(10.0, 10.0, 0.1);
        let coc_blur = sampler.sample_coc(50.0, 10.0, 0.1);

        assert_relative_eq!(coc_focus, 0.0, epsilon = 0.001);
        assert!(coc_blur > 0.0);
    }

    #[test]
    fn test_sampler_blur_samples() {
        let sampler = PostSampler::new(0);
        let pts = sampler.blur_samples(8);

        assert_eq!(pts.len(), 8);
        for (x, y) in &pts {
            let r = (x * x + y * y).sqrt();
            assert!(r <= 1.0);
        }
    }

    #[test]
    fn test_position_hash_determinism() {
        let h1 = position_hash(1.5, 2.5, 3.5, 42);
        let h2 = position_hash(1.5, 2.5, 3.5, 42);
        assert_relative_eq!(h1, h2, epsilon = 0.0001);
    }

    #[test]
    fn test_position_hash_range() {
        let coords: [(f32, f32, f32, u32); 8] = [
            (0.0, 0.0, 0.0, 0),
            (1.5, 2.7, 3.9, 10),
            (5.0, 8.5, 11.5, 50),
            (9.9, 16.8, 22.8, 99),
            (0.5, 1.0, 1.5, 5),
            (3.3, 5.6, 7.6, 33),
            (7.7, 13.1, 17.7, 77),
            (4.2, 7.1, 9.7, 42),
        ];
        for (x, y, z, seed) in coords {
            let h = position_hash(x, y, z, seed);
            assert!((0.0..=1.0).contains(&h));
        }
    }

    #[test]
    fn test_halton_jitter_range() {
        let frames: [u32; 8] = [0, 1, 7, 15, 31, 42, 55, 63];
        for frame in frames {
            let jx = halton_jitter(frame, 2);
            let jy = halton_jitter(frame, 3);
            assert!((-0.5..0.5).contains(&jx));
            assert!((-0.5..0.5).contains(&jy));
        }
    }

    #[test]
    fn test_frame_jitter() {
        let (jx, jy) = frame_jitter(0);
        assert_relative_eq!(jx, -0.5, epsilon = 0.001);
        assert_relative_eq!(jy, -0.5, epsilon = 0.001);

        let (horiz, vert) = frame_jitter(1);
        assert!((horiz - jx).abs() > 0.1 || (vert - jy).abs() > 0.1);
    }
}
