//! Cinematic camera path and playback.
//!
//! Provides camera paths built from keyframes with interpolation,
//! looping, and playback controls.

use super::easing::EasingFunction;
use super::keyframe::{CameraKeyframe, InterpolatedCamera, compute_keyframe_fingerprint};
use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

/// A cinematic camera path composed of keyframes.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CameraPath {
    /// Path name for identification.
    pub name: String,
    /// Ordered keyframes (by time).
    keyframes: Vec<CameraKeyframe>,
    /// Loop mode.
    pub loop_mode: LoopMode,
    /// Whether path is enabled.
    pub enabled: bool,
}

impl CameraPath {
    /// Create a new empty camera path.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            keyframes: Vec::new(),
            loop_mode: LoopMode::Once,
            enabled: true,
        }
    }

    /// Create a path from keyframes.
    #[must_use]
    pub fn from_keyframes(name: impl Into<String>, keyframes: Vec<CameraKeyframe>) -> Self {
        let mut path = Self::new(name);
        path.keyframes = keyframes;
        path.sort_keyframes();
        path
    }

    /// Add a keyframe to the path.
    pub fn add_keyframe(&mut self, keyframe: CameraKeyframe) {
        self.keyframes.push(keyframe);
        self.sort_keyframes();
    }

    /// Insert a keyframe at a specific index.
    pub fn insert_keyframe(&mut self, index: usize, keyframe: CameraKeyframe) {
        let index = index.min(self.keyframes.len());
        self.keyframes.insert(index, keyframe);
        self.sort_keyframes();
    }

    /// Remove a keyframe by index.
    pub fn remove_keyframe(&mut self, index: usize) -> Option<CameraKeyframe> {
        if index < self.keyframes.len() {
            Some(self.keyframes.remove(index))
        } else {
            None
        }
    }

    /// Get keyframe by index.
    #[must_use]
    pub fn get_keyframe(&self, index: usize) -> Option<&CameraKeyframe> {
        self.keyframes.get(index)
    }

    /// Get mutable keyframe by index.
    pub fn get_keyframe_mut(&mut self, index: usize) -> Option<&mut CameraKeyframe> {
        self.keyframes.get_mut(index)
    }

    /// Get all keyframes.
    #[must_use]
    pub fn keyframes(&self) -> &[CameraKeyframe] {
        &self.keyframes
    }

    /// Number of keyframes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.keyframes.len()
    }

    /// Whether path has no keyframes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.keyframes.is_empty()
    }

    /// Total duration of the path.
    #[must_use]
    pub fn duration(&self) -> f32 {
        self.keyframes.last().map_or(0.0, |kf| kf.time)
    }

    /// Sort keyframes by time.
    fn sort_keyframes(&mut self) {
        self.keyframes.sort_by(|a, b| {
            a.time
                .partial_cmp(&b.time)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    /// Set loop mode.
    #[must_use]
    pub fn with_loop_mode(mut self, mode: LoopMode) -> Self {
        self.loop_mode = mode;
        self
    }

    /// Set enabled state.
    #[must_use]
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Sample the path at a given time.
    #[must_use]
    pub fn sample(&self, time: f32) -> Option<InterpolatedCamera> {
        if self.keyframes.is_empty() {
            return None;
        }

        if self.keyframes.len() == 1 {
            return Some(InterpolatedCamera::from_keyframe(&self.keyframes[0]));
        }

        let effective_time = self.loop_mode.apply(time, self.duration());

        let (from_idx, to_idx) = self.find_keyframe_pair(effective_time);
        let from = &self.keyframes[from_idx];
        let to = &self.keyframes[to_idx];

        if from_idx == to_idx {
            return Some(InterpolatedCamera::from_keyframe(from));
        }

        let segment_duration = to.time - from.time;
        let t = if segment_duration > 0.0 {
            (effective_time - from.time) / segment_duration
        } else {
            0.0
        };

        Some(InterpolatedCamera::interpolate(from, to, t))
    }

    /// Find the keyframe pair surrounding a time.
    fn find_keyframe_pair(&self, time: f32) -> (usize, usize) {
        if time <= self.keyframes[0].time {
            return (0, 0);
        }

        let last_idx = self.keyframes.len() - 1;
        if time >= self.keyframes[last_idx].time {
            return (last_idx, last_idx);
        }

        for i in 0..last_idx {
            if time >= self.keyframes[i].time && time < self.keyframes[i + 1].time {
                return (i, i + 1);
            }
        }

        (last_idx, last_idx)
    }

    /// Validate the path.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self.keyframes.is_empty()
            && self.keyframes.iter().all(CameraKeyframe::is_valid)
            && self.keyframes.windows(2).all(|w| w[0].time <= w[1].time)
    }

    /// Get total path length (approximate arc length).
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn arc_length(&self, samples_per_segment: usize) -> f32 {
        if self.keyframes.len() < 2 {
            return 0.0;
        }

        let mut total = 0.0;
        let samples = samples_per_segment.max(2);

        for window in self.keyframes.windows(2) {
            let from = &window[0];
            let to = &window[1];
            let mut prev_pos = from.position;

            for i in 1..=samples {
                let t = i as f32 / samples as f32;
                let interp = InterpolatedCamera::interpolate(from, to, t);
                total += (interp.position - prev_pos).length();
                prev_pos = interp.position;
            }
        }

        total
    }

    /// Create a preview of the path as sampled positions.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn preview_positions(&self, sample_count: usize) -> Vec<Vec3> {
        let count = sample_count.max(2);
        let duration = self.duration();

        if duration <= 0.0 {
            return self.keyframes.iter().map(|kf| kf.position).collect();
        }

        (0..count)
            .filter_map(|i| {
                let t = (i as f32 / (count - 1) as f32) * duration;
                self.sample(t).map(|c| c.position)
            })
            .collect()
    }
}

/// Loop mode for camera path playback.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum LoopMode {
    /// Play once and stop at end.
    #[default]
    Once = 0,
    /// Loop back to start.
    Loop = 1,
    /// Ping-pong (play forward, then backward).
    PingPong = 2,
    /// Clamp at end (hold last frame).
    Clamp = 3,
}

impl LoopMode {
    /// Apply loop mode to a time value.
    #[must_use]
    pub fn apply(self, time: f32, duration: f32) -> f32 {
        if duration <= 0.0 {
            return 0.0;
        }

        match self {
            LoopMode::Once | LoopMode::Clamp => time.clamp(0.0, duration),
            LoopMode::Loop => {
                let t = time % duration;
                if t < 0.0 { t + duration } else { t }
            }
            LoopMode::PingPong => {
                let t = time % (duration * 2.0);
                let t = if t < 0.0 { t + duration * 2.0 } else { t };
                if t > duration { duration * 2.0 - t } else { t }
            }
        }
    }

    /// Whether this mode can complete (stop playing).
    #[must_use]
    pub const fn can_complete(&self) -> bool {
        matches!(self, LoopMode::Once)
    }
}

/// Camera path playback state.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PathPlayback {
    /// Current playback time.
    pub time: f32,
    /// Playback speed multiplier.
    pub speed: f32,
    /// Whether playback is paused.
    pub paused: bool,
    /// Whether playback has completed (for Once mode).
    pub completed: bool,
    /// Direction (1.0 forward, -1.0 backward).
    pub direction: f32,
}

impl PathPlayback {
    /// Create new playback state.
    #[must_use]
    pub fn new() -> Self {
        Self {
            time: 0.0,
            speed: 1.0,
            paused: false,
            completed: false,
            direction: 1.0,
        }
    }

    /// Create paused playback.
    #[must_use]
    pub fn paused() -> Self {
        Self {
            paused: true,
            ..Self::new()
        }
    }

    /// Set playback speed.
    #[must_use]
    pub fn with_speed(mut self, speed: f32) -> Self {
        self.speed = speed.max(0.0);
        self
    }

    /// Start or resume playback.
    pub fn play(&mut self) {
        self.paused = false;
        if self.completed {
            self.time = 0.0;
            self.completed = false;
        }
    }

    /// Pause playback.
    pub fn pause(&mut self) {
        self.paused = true;
    }

    /// Toggle pause state.
    pub fn toggle_pause(&mut self) {
        if self.paused {
            self.play();
        } else {
            self.pause();
        }
    }

    /// Stop and reset playback.
    pub fn stop(&mut self) {
        self.paused = true;
        self.time = 0.0;
        self.completed = false;
    }

    /// Seek to a specific time.
    pub fn seek(&mut self, time: f32) {
        self.time = time.max(0.0);
        self.completed = false;
    }

    /// Update playback with delta time.
    pub fn update(&mut self, dt: f32, duration: f32, loop_mode: LoopMode) {
        if self.paused || self.completed {
            return;
        }

        self.time += dt * self.speed * self.direction;

        match loop_mode {
            LoopMode::Once => {
                if self.time >= duration {
                    self.time = duration;
                    self.completed = true;
                    self.paused = true;
                }
            }
            LoopMode::Clamp => {
                self.time = self.time.clamp(0.0, duration);
            }
            LoopMode::Loop => {
                while self.time >= duration {
                    self.time -= duration;
                }
                while self.time < 0.0 {
                    self.time += duration;
                }
            }
            LoopMode::PingPong => {
                if self.time >= duration {
                    self.time = duration;
                    self.direction = -1.0;
                } else if self.time <= 0.0 {
                    self.time = 0.0;
                    self.direction = 1.0;
                }
            }
        }
    }

    /// Get normalized progress (0.0 to 1.0).
    #[must_use]
    pub fn progress(&self, duration: f32) -> f32 {
        if duration <= 0.0 {
            0.0
        } else {
            (self.time / duration).clamp(0.0, 1.0)
        }
    }
}

/// Compute a stable fingerprint for a camera path.
#[must_use]
pub fn compute_path_fingerprint(path: &CameraPath) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    path.name.hash(&mut hasher);
    path.keyframes.len().hash(&mut hasher);
    for kf in &path.keyframes {
        compute_keyframe_fingerprint(kf).hash(&mut hasher);
    }
    path.loop_mode.hash(&mut hasher);
    path.enabled.hash(&mut hasher);
    hasher.finish()
}

/// Builder for creating camera paths with common patterns.
pub struct PathBuilder {
    name: String,
    keyframes: Vec<CameraKeyframe>,
    default_easing: EasingFunction,
}

impl PathBuilder {
    /// Create a new path builder.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            keyframes: Vec::new(),
            default_easing: EasingFunction::SmoothStep,
        }
    }

    /// Set default easing for new keyframes.
    #[must_use]
    pub fn with_default_easing(mut self, easing: EasingFunction) -> Self {
        self.default_easing = easing;
        self
    }

    /// Add a position keyframe.
    #[must_use]
    pub fn at(mut self, time: f32, position: Vec3) -> Self {
        let kf =
            CameraKeyframe::new(time, position, Quat::IDENTITY).with_easing(self.default_easing);
        self.keyframes.push(kf);
        self
    }

    /// Add a look-at keyframe.
    #[must_use]
    pub fn look_at(mut self, time: f32, position: Vec3, target: Vec3) -> Self {
        let kf = CameraKeyframe::looking_at(time, position, target, Vec3::Y)
            .with_easing(self.default_easing);
        self.keyframes.push(kf);
        self
    }

    /// Add a full keyframe.
    #[must_use]
    pub fn keyframe(mut self, kf: CameraKeyframe) -> Self {
        self.keyframes.push(kf);
        self
    }

    /// Build the camera path.
    #[must_use]
    pub fn build(self) -> CameraPath {
        CameraPath::from_keyframes(self.name, self.keyframes)
    }
}

/// Create a circular orbit path around a target.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn create_orbit_path(
    name: impl Into<String>,
    center: Vec3,
    radius: f32,
    height: f32,
    duration: f32,
    keyframe_count: usize,
) -> CameraPath {
    let count = keyframe_count.max(4);
    let mut keyframes = Vec::with_capacity(count);

    for i in 0..count {
        let t = i as f32 / count as f32;
        let angle = t * std::f32::consts::TAU;
        let x = center.x + radius * angle.cos();
        let z = center.z + radius * angle.sin();
        let position = Vec3::new(x, center.y + height, z);

        let kf = CameraKeyframe::looking_at(t * duration, position, center, Vec3::Y)
            .with_easing(EasingFunction::Linear);
        keyframes.push(kf);
    }

    CameraPath::from_keyframes(name, keyframes).with_loop_mode(LoopMode::Loop)
}

/// Create a dolly zoom path (Vertigo effect).
#[must_use]
pub fn create_dolly_zoom_path(
    name: impl Into<String>,
    target: Vec3,
    start_distance: f32,
    end_distance: f32,
    start_fov: f32,
    end_fov: f32,
    duration: f32,
) -> CameraPath {
    let direction = Vec3::NEG_Z;

    let start_pos = target - direction * start_distance;
    let end_pos = target - direction * end_distance;

    let keyframes = vec![
        CameraKeyframe::new(0.0, start_pos, Quat::IDENTITY)
            .with_fov(start_fov)
            .with_easing(EasingFunction::Linear),
        CameraKeyframe::new(duration, end_pos, Quat::IDENTITY)
            .with_fov(end_fov)
            .with_easing(EasingFunction::Linear),
    ];

    CameraPath::from_keyframes(name, keyframes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_empty_path() {
        let path = CameraPath::new("empty");
        assert!(path.is_empty());
        assert_relative_eq!(path.duration(), 0.0);
        assert!(path.sample(0.0).is_none());
    }

    #[test]
    fn test_single_keyframe() {
        let mut path = CameraPath::new("single");
        path.add_keyframe(CameraKeyframe::new(0.0, Vec3::X, Quat::IDENTITY));

        let sample = path.sample(0.0).expect("should sample");
        assert_relative_eq!(sample.position.x, 1.0);
    }

    #[test]
    fn test_path_sampling() {
        let path = PathBuilder::new("test")
            .at(0.0, Vec3::ZERO)
            .at(1.0, Vec3::new(10.0, 0.0, 0.0))
            .build();

        let at_half = path.sample(0.5).expect("should sample");
        assert!(at_half.position.x > 0.0 && at_half.position.x < 10.0);

        let at_end = path.sample(1.0).expect("should sample");
        assert_relative_eq!(at_end.position.x, 10.0, epsilon = 0.01);
    }

    #[test]
    fn test_loop_mode_once() {
        let result = LoopMode::Once.apply(1.5, 1.0);
        assert_relative_eq!(result, 1.0);
    }

    #[test]
    fn test_loop_mode_loop() {
        let result = LoopMode::Loop.apply(2.5, 1.0);
        assert_relative_eq!(result, 0.5, epsilon = 0.001);
    }

    #[test]
    fn test_loop_mode_pingpong() {
        let result = LoopMode::PingPong.apply(1.5, 1.0);
        assert_relative_eq!(result, 0.5, epsilon = 0.001);
    }

    #[test]
    fn test_playback_update() {
        let mut playback = PathPlayback::new();
        playback.update(0.5, 1.0, LoopMode::Once);

        assert_relative_eq!(playback.time, 0.5);
        assert!(!playback.completed);

        playback.update(0.6, 1.0, LoopMode::Once);
        assert!(playback.completed);
        assert!(playback.paused);
    }

    #[test]
    fn test_playback_loop() {
        let mut playback = PathPlayback::new();
        playback.update(1.5, 1.0, LoopMode::Loop);

        assert_relative_eq!(playback.time, 0.5, epsilon = 0.001);
        assert!(!playback.completed);
    }

    #[test]
    fn test_playback_controls() {
        let mut playback = PathPlayback::new();

        playback.pause();
        assert!(playback.paused);

        playback.play();
        assert!(!playback.paused);

        playback.stop();
        assert!(playback.paused);
        assert_relative_eq!(playback.time, 0.0);
    }

    #[test]
    fn test_path_fingerprint_determinism() {
        let path = PathBuilder::new("test")
            .at(0.0, Vec3::ZERO)
            .at(1.0, Vec3::X)
            .build();

        let fp1 = compute_path_fingerprint(&path);
        let fp2 = compute_path_fingerprint(&path);

        assert_eq!(fp1, fp2);
    }

    #[test]
    fn test_path_fingerprint_sensitivity() {
        let path1 = PathBuilder::new("test1").at(0.0, Vec3::ZERO).build();
        let path2 = PathBuilder::new("test2").at(0.0, Vec3::ZERO).build();

        assert_ne!(
            compute_path_fingerprint(&path1),
            compute_path_fingerprint(&path2)
        );
    }

    #[test]
    fn test_orbit_path() {
        let path = create_orbit_path("orbit", Vec3::ZERO, 10.0, 5.0, 4.0, 8);

        assert_eq!(path.len(), 8);
        assert_eq!(path.loop_mode, LoopMode::Loop);
        assert!(path.is_valid());
    }

    #[test]
    fn test_dolly_zoom_path() {
        let path = create_dolly_zoom_path("dolly", Vec3::ZERO, 10.0, 5.0, 30.0, 60.0, 2.0);

        assert_eq!(path.len(), 2);
        let start = path.sample(0.0).expect("sample start");
        let end = path.sample(2.0).expect("sample end");

        assert!(start.fov < end.fov);
    }

    #[test]
    fn test_arc_length() {
        let path = PathBuilder::new("straight")
            .at(0.0, Vec3::ZERO)
            .at(1.0, Vec3::new(10.0, 0.0, 0.0))
            .build();

        let length = path.arc_length(10);
        assert_relative_eq!(length, 10.0, epsilon = 0.1);
    }

    #[test]
    fn test_preview_positions() {
        let path = PathBuilder::new("test")
            .at(0.0, Vec3::ZERO)
            .at(1.0, Vec3::X * 10.0)
            .build();

        let preview = path.preview_positions(5);
        assert_eq!(preview.len(), 5);
    }

    #[test]
    fn test_serde_roundtrip() {
        let path = PathBuilder::new("serialized")
            .at(0.0, Vec3::ZERO)
            .at(1.0, Vec3::X)
            .build()
            .with_loop_mode(LoopMode::PingPong);

        let bytes = bincode::serialize(&path).expect("serialize");
        let restored: CameraPath = bincode::deserialize(&bytes).expect("deserialize");

        assert_eq!(path.name, restored.name);
        assert_eq!(path.loop_mode, restored.loop_mode);
        assert_eq!(path.len(), restored.len());
    }
}
