//! Material-aware reverb zones and regions.
//!
//! Provides CPU-side primitives for defining acoustic reverb zones with
//! material-aware presets, priority-based blending, and deterministic sampling.

use glam::Vec3;
use serde::{Deserialize, Serialize};

use crate::occlusion::AcousticMaterial;

/// Unique identifier for a reverb zone.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReverbZoneId(pub u32);

impl ReverbZoneId {
    /// Create a new zone ID.
    #[must_use]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Get the raw ID value.
    #[must_use]
    pub const fn value(&self) -> u32 {
        self.0
    }
}

/// Reverb preset based on acoustic environment.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum ReverbPreset {
    /// No reverb (outdoor, anechoic).
    #[default]
    None = 0,
    /// Small room (closet, bathroom).
    SmallRoom = 1,
    /// Medium room (bedroom, office).
    MediumRoom = 2,
    /// Large room (hall, warehouse).
    LargeRoom = 3,
    /// Cave/cavern.
    Cave = 4,
    /// Underwater.
    Underwater = 5,
    /// Stone corridor/tunnel.
    StoneCorridor = 6,
    /// Wooden interior.
    WoodInterior = 7,
    /// Metal enclosure.
    MetalEnclosure = 8,
    /// Cathedral/large stone.
    Cathedral = 9,
    /// Forest (outdoor with absorption).
    Forest = 10,
    /// Canyon/cliffs.
    Canyon = 11,
}

impl ReverbPreset {
    /// All preset variants.
    pub const ALL: [ReverbPreset; 12] = [
        ReverbPreset::None,
        ReverbPreset::SmallRoom,
        ReverbPreset::MediumRoom,
        ReverbPreset::LargeRoom,
        ReverbPreset::Cave,
        ReverbPreset::Underwater,
        ReverbPreset::StoneCorridor,
        ReverbPreset::WoodInterior,
        ReverbPreset::MetalEnclosure,
        ReverbPreset::Cathedral,
        ReverbPreset::Forest,
        ReverbPreset::Canyon,
    ];

    /// Get the default configuration for this preset.
    #[must_use]
    pub fn config(&self) -> ReverbConfig {
        match self {
            ReverbPreset::None => ReverbConfig::NONE,
            ReverbPreset::SmallRoom => ReverbConfig::SMALL_ROOM,
            ReverbPreset::MediumRoom => ReverbConfig::MEDIUM_ROOM,
            ReverbPreset::LargeRoom => ReverbConfig::LARGE_ROOM,
            ReverbPreset::Cave => ReverbConfig::CAVE,
            ReverbPreset::Underwater => ReverbConfig::UNDERWATER,
            ReverbPreset::StoneCorridor => ReverbConfig::STONE_CORRIDOR,
            ReverbPreset::WoodInterior => ReverbConfig::WOOD_INTERIOR,
            ReverbPreset::MetalEnclosure => ReverbConfig::METAL_ENCLOSURE,
            ReverbPreset::Cathedral => ReverbConfig::CATHEDRAL,
            ReverbPreset::Forest => ReverbConfig::FOREST,
            ReverbPreset::Canyon => ReverbConfig::CANYON,
        }
    }

    /// Get the dominant material for this preset.
    #[must_use]
    pub fn dominant_material(&self) -> AcousticMaterial {
        match self {
            ReverbPreset::None | ReverbPreset::Forest | ReverbPreset::Canyon => {
                AcousticMaterial::Air
            }
            ReverbPreset::SmallRoom
            | ReverbPreset::MediumRoom
            | ReverbPreset::LargeRoom
            | ReverbPreset::Cave
            | ReverbPreset::StoneCorridor
            | ReverbPreset::Cathedral => AcousticMaterial::Stone,
            ReverbPreset::Underwater => AcousticMaterial::Liquid,
            ReverbPreset::WoodInterior => AcousticMaterial::Wood,
            ReverbPreset::MetalEnclosure => AcousticMaterial::Metal,
        }
    }
}

/// Reverb configuration parameters.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReverbConfig {
    /// Wet/dry mix (0.0 = dry, 1.0 = wet).
    pub wet_dry_mix: f32,
    /// Decay time in seconds.
    pub decay_time: f32,
    /// High-frequency damping (0.0 = none, 1.0 = full).
    pub damping: f32,
    /// Room size factor (0.0 = tiny, 1.0 = huge).
    pub room_size: f32,
    /// Early reflection density (0.0 = sparse, 1.0 = dense).
    pub density: f32,
    /// Pre-delay in milliseconds.
    pub pre_delay_ms: f32,
    /// Diffusion (0.0 = discrete echoes, 1.0 = smooth).
    pub diffusion: f32,
    /// Low-frequency rolloff (Hz).
    pub low_cutoff: f32,
    /// High-frequency rolloff (Hz).
    pub high_cutoff: f32,
    /// Color tint from dominant material.
    pub color: Vec3,
}

impl ReverbConfig {
    /// No reverb.
    pub const NONE: ReverbConfig = ReverbConfig {
        wet_dry_mix: 0.0,
        decay_time: 0.0,
        damping: 0.0,
        room_size: 0.0,
        density: 0.0,
        pre_delay_ms: 0.0,
        diffusion: 0.0,
        low_cutoff: 20.0,
        high_cutoff: 20000.0,
        color: Vec3::ONE,
    };

    /// Small room.
    pub const SMALL_ROOM: ReverbConfig = ReverbConfig {
        wet_dry_mix: 0.3,
        decay_time: 0.4,
        damping: 0.6,
        room_size: 0.15,
        density: 0.7,
        pre_delay_ms: 5.0,
        diffusion: 0.8,
        low_cutoff: 80.0,
        high_cutoff: 12000.0,
        color: Vec3::new(0.9, 0.85, 0.8),
    };

    /// Medium room.
    pub const MEDIUM_ROOM: ReverbConfig = ReverbConfig {
        wet_dry_mix: 0.4,
        decay_time: 0.8,
        damping: 0.5,
        room_size: 0.35,
        density: 0.6,
        pre_delay_ms: 12.0,
        diffusion: 0.75,
        low_cutoff: 60.0,
        high_cutoff: 14000.0,
        color: Vec3::new(0.9, 0.9, 0.85),
    };

    /// Large room.
    pub const LARGE_ROOM: ReverbConfig = ReverbConfig {
        wet_dry_mix: 0.5,
        decay_time: 1.5,
        damping: 0.4,
        room_size: 0.6,
        density: 0.5,
        pre_delay_ms: 25.0,
        diffusion: 0.7,
        low_cutoff: 40.0,
        high_cutoff: 15000.0,
        color: Vec3::new(0.95, 0.95, 0.9),
    };

    /// Cave.
    pub const CAVE: ReverbConfig = ReverbConfig {
        wet_dry_mix: 0.65,
        decay_time: 2.5,
        damping: 0.3,
        room_size: 0.75,
        density: 0.4,
        pre_delay_ms: 40.0,
        diffusion: 0.6,
        low_cutoff: 30.0,
        high_cutoff: 10000.0,
        color: Vec3::new(0.7, 0.75, 0.8),
    };

    /// Underwater.
    pub const UNDERWATER: ReverbConfig = ReverbConfig {
        wet_dry_mix: 0.8,
        decay_time: 1.2,
        damping: 0.85,
        room_size: 0.5,
        density: 0.9,
        pre_delay_ms: 8.0,
        diffusion: 0.95,
        low_cutoff: 100.0,
        high_cutoff: 4000.0,
        color: Vec3::new(0.5, 0.6, 0.9),
    };

    /// Stone corridor.
    pub const STONE_CORRIDOR: ReverbConfig = ReverbConfig {
        wet_dry_mix: 0.55,
        decay_time: 1.8,
        damping: 0.35,
        room_size: 0.4,
        density: 0.55,
        pre_delay_ms: 15.0,
        diffusion: 0.65,
        low_cutoff: 50.0,
        high_cutoff: 11000.0,
        color: Vec3::new(0.75, 0.7, 0.65),
    };

    /// Wood interior.
    pub const WOOD_INTERIOR: ReverbConfig = ReverbConfig {
        wet_dry_mix: 0.35,
        decay_time: 0.6,
        damping: 0.55,
        room_size: 0.3,
        density: 0.65,
        pre_delay_ms: 8.0,
        diffusion: 0.8,
        low_cutoff: 70.0,
        high_cutoff: 13000.0,
        color: Vec3::new(0.9, 0.75, 0.5),
    };

    /// Metal enclosure.
    pub const METAL_ENCLOSURE: ReverbConfig = ReverbConfig {
        wet_dry_mix: 0.6,
        decay_time: 2.0,
        damping: 0.15,
        room_size: 0.45,
        density: 0.75,
        pre_delay_ms: 3.0,
        diffusion: 0.5,
        low_cutoff: 100.0,
        high_cutoff: 16000.0,
        color: Vec3::new(0.6, 0.7, 0.9),
    };

    /// Cathedral.
    pub const CATHEDRAL: ReverbConfig = ReverbConfig {
        wet_dry_mix: 0.7,
        decay_time: 4.0,
        damping: 0.25,
        room_size: 0.9,
        density: 0.35,
        pre_delay_ms: 60.0,
        diffusion: 0.55,
        low_cutoff: 25.0,
        high_cutoff: 12000.0,
        color: Vec3::new(0.85, 0.8, 0.75),
    };

    /// Forest.
    pub const FOREST: ReverbConfig = ReverbConfig {
        wet_dry_mix: 0.2,
        decay_time: 0.5,
        damping: 0.8,
        room_size: 0.8,
        density: 0.25,
        pre_delay_ms: 30.0,
        diffusion: 0.4,
        low_cutoff: 50.0,
        high_cutoff: 8000.0,
        color: Vec3::new(0.7, 0.8, 0.6),
    };

    /// Canyon.
    pub const CANYON: ReverbConfig = ReverbConfig {
        wet_dry_mix: 0.5,
        decay_time: 3.0,
        damping: 0.2,
        room_size: 0.95,
        density: 0.2,
        pre_delay_ms: 80.0,
        diffusion: 0.3,
        low_cutoff: 30.0,
        high_cutoff: 14000.0,
        color: Vec3::new(0.9, 0.85, 0.75),
    };

    /// Create a custom configuration.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        wet_dry_mix: f32,
        decay_time: f32,
        damping: f32,
        room_size: f32,
        density: f32,
        pre_delay_ms: f32,
        diffusion: f32,
        low_cutoff: f32,
        high_cutoff: f32,
        color: Vec3,
    ) -> Self {
        Self {
            wet_dry_mix,
            decay_time,
            damping,
            room_size,
            density,
            pre_delay_ms,
            diffusion,
            low_cutoff,
            high_cutoff,
            color,
        }
    }

    /// Validate configuration parameters.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        (0.0..=1.0).contains(&self.wet_dry_mix)
            && self.decay_time >= 0.0
            && (0.0..=1.0).contains(&self.damping)
            && (0.0..=1.0).contains(&self.room_size)
            && (0.0..=1.0).contains(&self.density)
            && self.pre_delay_ms >= 0.0
            && (0.0..=1.0).contains(&self.diffusion)
            && self.low_cutoff > 0.0
            && self.high_cutoff > self.low_cutoff
    }

    /// Clamp all parameters to valid ranges.
    #[must_use]
    pub fn clamped(&self) -> Self {
        Self {
            wet_dry_mix: self.wet_dry_mix.clamp(0.0, 1.0),
            decay_time: self.decay_time.max(0.0),
            damping: self.damping.clamp(0.0, 1.0),
            room_size: self.room_size.clamp(0.0, 1.0),
            density: self.density.clamp(0.0, 1.0),
            pre_delay_ms: self.pre_delay_ms.max(0.0),
            diffusion: self.diffusion.clamp(0.0, 1.0),
            low_cutoff: self.low_cutoff.max(1.0),
            high_cutoff: self.high_cutoff.max(self.low_cutoff + 1.0),
            color: self.color,
        }
    }

    /// Blend two configurations by weight.
    #[must_use]
    pub fn blend(&self, other: &Self, weight: f32) -> Self {
        let w = weight.clamp(0.0, 1.0);
        let inv_w = 1.0 - w;
        Self {
            wet_dry_mix: self.wet_dry_mix * inv_w + other.wet_dry_mix * w,
            decay_time: self.decay_time * inv_w + other.decay_time * w,
            damping: self.damping * inv_w + other.damping * w,
            room_size: self.room_size * inv_w + other.room_size * w,
            density: self.density * inv_w + other.density * w,
            pre_delay_ms: self.pre_delay_ms * inv_w + other.pre_delay_ms * w,
            diffusion: self.diffusion * inv_w + other.diffusion * w,
            low_cutoff: self.low_cutoff * inv_w + other.low_cutoff * w,
            high_cutoff: self.high_cutoff * inv_w + other.high_cutoff * w,
            color: self.color * inv_w + other.color * w,
        }
    }

    /// Apply material coloration to this config.
    #[must_use]
    pub fn with_material(&self, material: AcousticMaterial) -> Self {
        let profile = material.profile();
        Self {
            color: profile.reverb_color,
            damping: (self.damping + profile.absorption * 0.3).min(1.0),
            ..*self
        }
    }
}

impl Default for ReverbConfig {
    fn default() -> Self {
        Self::NONE
    }
}

/// Shape of a reverb zone.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum ZoneShape {
    /// Axis-aligned box.
    Box { half_extents: Vec3 },
    /// Sphere.
    Sphere { radius: f32 },
    /// Infinite (global fallback).
    #[default]
    Global,
}

impl ZoneShape {
    /// Create a box shape.
    #[must_use]
    pub const fn box_shape(half_extents: Vec3) -> Self {
        ZoneShape::Box { half_extents }
    }

    /// Create a sphere shape.
    #[must_use]
    pub const fn sphere(radius: f32) -> Self {
        ZoneShape::Sphere { radius }
    }

    /// Check if a point is inside this shape (relative to shape center).
    #[must_use]
    pub fn contains(&self, local_pos: Vec3) -> bool {
        match self {
            ZoneShape::Box { half_extents } => {
                local_pos.x.abs() <= half_extents.x
                    && local_pos.y.abs() <= half_extents.y
                    && local_pos.z.abs() <= half_extents.z
            }
            ZoneShape::Sphere { radius } => local_pos.length_squared() <= radius * radius,
            ZoneShape::Global => true,
        }
    }

    /// Get blend factor for position (1.0 at center, 0.0 at edge).
    #[must_use]
    pub fn blend_factor(&self, local_pos: Vec3, falloff: f32) -> f32 {
        let falloff = falloff.clamp(0.0, 1.0);
        match self {
            ZoneShape::Box { half_extents } => {
                if !self.contains(local_pos) {
                    return 0.0;
                }
                let norm_x = if half_extents.x > 0.0 {
                    local_pos.x.abs() / half_extents.x
                } else {
                    0.0
                };
                let norm_y = if half_extents.y > 0.0 {
                    local_pos.y.abs() / half_extents.y
                } else {
                    0.0
                };
                let norm_z = if half_extents.z > 0.0 {
                    local_pos.z.abs() / half_extents.z
                } else {
                    0.0
                };
                let max_norm = norm_x.max(norm_y).max(norm_z);
                let inner_edge = 1.0 - falloff;
                if max_norm <= inner_edge {
                    1.0
                } else {
                    1.0 - (max_norm - inner_edge) / falloff
                }
            }
            ZoneShape::Sphere { radius } => {
                let dist = local_pos.length();
                if dist > *radius {
                    return 0.0;
                }
                let norm_dist = dist / radius;
                let inner_edge = 1.0 - falloff;
                if norm_dist <= inner_edge {
                    1.0
                } else {
                    1.0 - (norm_dist - inner_edge) / falloff
                }
            }
            ZoneShape::Global => 1.0,
        }
    }
}

/// A reverb zone with spatial bounds and acoustic properties.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReverbZone {
    /// Unique identifier.
    pub id: ReverbZoneId,
    /// Zone center in world space.
    pub center: Vec3,
    /// Zone shape.
    pub shape: ZoneShape,
    /// Reverb configuration.
    pub config: ReverbConfig,
    /// Priority (higher overrides lower).
    pub priority: i32,
    /// Blend falloff (0.0 = hard edge, 1.0 = full fade).
    pub falloff: f32,
    /// Whether this zone is active.
    pub active: bool,
}

impl ReverbZone {
    /// Create a new reverb zone.
    #[must_use]
    pub fn new(id: ReverbZoneId, center: Vec3, shape: ZoneShape, config: ReverbConfig) -> Self {
        Self {
            id,
            center,
            shape,
            config,
            priority: 0,
            falloff: 0.3,
            active: true,
        }
    }

    /// Create from a preset.
    #[must_use]
    pub fn from_preset(
        id: ReverbZoneId,
        center: Vec3,
        shape: ZoneShape,
        preset: ReverbPreset,
    ) -> Self {
        Self::new(id, center, shape, preset.config())
    }

    /// Builder: set priority.
    #[must_use]
    pub const fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Builder: set falloff.
    #[must_use]
    pub fn with_falloff(mut self, falloff: f32) -> Self {
        self.falloff = falloff.clamp(0.0, 1.0);
        self
    }

    /// Builder: set active.
    #[must_use]
    pub const fn with_active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// Check if a world position is inside this zone.
    #[must_use]
    pub fn contains(&self, world_pos: Vec3) -> bool {
        self.active && self.shape.contains(world_pos - self.center)
    }

    /// Get blend factor for a world position.
    #[must_use]
    pub fn blend_factor(&self, world_pos: Vec3) -> f32 {
        if !self.active {
            return 0.0;
        }
        self.shape
            .blend_factor(world_pos - self.center, self.falloff)
    }
}

impl Default for ReverbZone {
    fn default() -> Self {
        Self::new(
            ReverbZoneId::default(),
            Vec3::ZERO,
            ZoneShape::Global,
            ReverbConfig::NONE,
        )
    }
}

/// Result of sampling reverb at a listener position.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReverbSample {
    /// Blended reverb configuration.
    pub config: ReverbConfig,
    /// Contributing zones with their weights.
    pub contributions: Vec<(ReverbZoneId, f32)>,
    /// Total blend weight (sum of contributions).
    pub total_weight: f32,
    /// Fingerprint of this sample.
    pub fingerprint: u64,
}

impl ReverbSample {
    /// Check if there's any reverb effect.
    #[must_use]
    pub fn has_reverb(&self) -> bool {
        self.config.wet_dry_mix > 0.001
    }

    /// Get the dominant zone (highest contribution).
    #[must_use]
    pub fn dominant_zone(&self) -> Option<ReverbZoneId> {
        self.contributions
            .iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(id, _)| *id)
    }
}

impl Default for ReverbSample {
    fn default() -> Self {
        Self {
            config: ReverbConfig::NONE,
            contributions: Vec::new(),
            total_weight: 0.0,
            fingerprint: 0,
        }
    }
}

/// Registry of reverb zones.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ReverbZoneRegistry {
    zones: Vec<ReverbZone>,
    next_id: u32,
}

impl ReverbZoneRegistry {
    /// Create an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a zone and return its ID.
    pub fn add(&mut self, mut zone: ReverbZone) -> ReverbZoneId {
        let id = ReverbZoneId::new(self.next_id);
        self.next_id += 1;
        zone.id = id;
        self.zones.push(zone);
        id
    }

    /// Add a zone with a specific ID.
    pub fn add_with_id(&mut self, zone: ReverbZone) {
        self.next_id = self.next_id.max(zone.id.value() + 1);
        self.zones.push(zone);
    }

    /// Get a zone by ID.
    #[must_use]
    pub fn get(&self, id: ReverbZoneId) -> Option<&ReverbZone> {
        self.zones.iter().find(|z| z.id == id)
    }

    /// Get a mutable zone by ID.
    pub fn get_mut(&mut self, id: ReverbZoneId) -> Option<&mut ReverbZone> {
        self.zones.iter_mut().find(|z| z.id == id)
    }

    /// Remove a zone by ID.
    pub fn remove(&mut self, id: ReverbZoneId) -> Option<ReverbZone> {
        self.zones
            .iter()
            .position(|z| z.id == id)
            .map(|i| self.zones.remove(i))
    }

    /// Get all zones.
    #[must_use]
    pub fn zones(&self) -> &[ReverbZone] {
        &self.zones
    }

    /// Get active zones containing a position.
    #[must_use]
    pub fn zones_at(&self, pos: Vec3) -> Vec<&ReverbZone> {
        self.zones.iter().filter(|z| z.contains(pos)).collect()
    }

    /// Number of zones.
    #[must_use]
    pub fn len(&self) -> usize {
        self.zones.len()
    }

    /// Check if empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.zones.is_empty()
    }

    /// Clear all zones.
    pub fn clear(&mut self) {
        self.zones.clear();
    }

    /// Sort zones by priority (highest first).
    pub fn sort_by_priority(&mut self) {
        self.zones.sort_by(|a, b| b.priority.cmp(&a.priority));
    }
}

/// Sample reverb at a listener position from a set of zones.
#[must_use]
pub fn sample_reverb(zones: &[ReverbZone], listener_pos: Vec3) -> ReverbSample {
    let mut contributions: Vec<(ReverbZoneId, f32, &ReverbConfig)> = Vec::new();

    for zone in zones {
        let weight = zone.blend_factor(listener_pos);
        if weight > 0.001 {
            contributions.push((zone.id, weight, &zone.config));
        }
    }

    if contributions.is_empty() {
        return ReverbSample::default();
    }

    contributions.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let total_weight: f32 = contributions.iter().map(|(_, w, _)| w).sum();

    let mut blended = ReverbConfig::NONE;
    for (_, weight, config) in &contributions {
        let norm_weight = weight / total_weight;
        blended = blended.blend(config, norm_weight / (1.0 - norm_weight + norm_weight));
    }

    if contributions.len() > 1 {
        blended = ReverbConfig::NONE;
        for (_, weight, config) in &contributions {
            let norm_weight = weight / total_weight;
            blended.wet_dry_mix += config.wet_dry_mix * norm_weight;
            blended.decay_time += config.decay_time * norm_weight;
            blended.damping += config.damping * norm_weight;
            blended.room_size += config.room_size * norm_weight;
            blended.density += config.density * norm_weight;
            blended.pre_delay_ms += config.pre_delay_ms * norm_weight;
            blended.diffusion += config.diffusion * norm_weight;
            blended.low_cutoff += config.low_cutoff * norm_weight;
            blended.high_cutoff += config.high_cutoff * norm_weight;
            blended.color += config.color * norm_weight;
        }
    } else {
        blended = *contributions[0].2;
    }

    let contrib_ids: Vec<(ReverbZoneId, f32)> =
        contributions.iter().map(|(id, w, _)| (*id, *w)).collect();
    let fingerprint = compute_sample_fingerprint(&blended, &contrib_ids);

    ReverbSample {
        config: blended,
        contributions: contrib_ids,
        total_weight,
        fingerprint,
    }
}

/// Sample reverb with priority-based blending (higher priority zones override).
#[must_use]
pub fn sample_reverb_priority(
    zones: &[ReverbZone],
    listener_pos: Vec3,
    max_zones: usize,
) -> ReverbSample {
    let mut active_zones: Vec<(&ReverbZone, f32)> = zones
        .iter()
        .filter_map(|z| {
            let weight = z.blend_factor(listener_pos);
            if weight > 0.001 {
                Some((z, weight))
            } else {
                None
            }
        })
        .collect();

    active_zones.sort_by(|a, b| b.0.priority.cmp(&a.0.priority));
    active_zones.truncate(max_zones);

    if active_zones.is_empty() {
        return ReverbSample::default();
    }

    let sorted_zones: Vec<ReverbZone> = active_zones.iter().map(|(z, _)| (*z).clone()).collect();
    sample_reverb(&sorted_zones, listener_pos)
}

/// Compute fingerprint for a reverb sample.
fn compute_sample_fingerprint(config: &ReverbConfig, contributions: &[(ReverbZoneId, f32)]) -> u64 {
    let mut hasher = crc32fast::Hasher::new();
    hasher.update(&config.wet_dry_mix.to_le_bytes());
    hasher.update(&config.decay_time.to_le_bytes());
    hasher.update(&config.damping.to_le_bytes());
    hasher.update(&config.room_size.to_le_bytes());
    hasher.update(&config.density.to_le_bytes());
    #[allow(clippy::cast_possible_truncation)]
    let count = contributions.len() as u32;
    hasher.update(&count.to_le_bytes());
    for (id, weight) in contributions {
        hasher.update(&id.0.to_le_bytes());
        hasher.update(&weight.to_le_bytes());
    }
    u64::from(hasher.finalize())
}

/// Compute fingerprint for a reverb zone.
#[must_use]
pub fn compute_zone_fingerprint(zone: &ReverbZone) -> u64 {
    let mut hasher = crc32fast::Hasher::new();
    hasher.update(&zone.id.0.to_le_bytes());
    hasher.update(&zone.center.x.to_le_bytes());
    hasher.update(&zone.center.y.to_le_bytes());
    hasher.update(&zone.center.z.to_le_bytes());
    hasher.update(&zone.priority.to_le_bytes());
    hasher.update(&zone.falloff.to_le_bytes());
    hasher.update(&[u8::from(zone.active)]);
    hasher.update(&zone.config.wet_dry_mix.to_le_bytes());
    hasher.update(&zone.config.decay_time.to_le_bytes());
    u64::from(hasher.finalize())
}

/// Compute fingerprint for a registry.
#[must_use]
pub fn compute_registry_fingerprint(registry: &ReverbZoneRegistry) -> u64 {
    let mut hasher = crc32fast::Hasher::new();
    #[allow(clippy::cast_possible_truncation)]
    let count = registry.zones.len() as u32;
    hasher.update(&count.to_le_bytes());
    for zone in &registry.zones {
        let zone_fp = compute_zone_fingerprint(zone);
        hasher.update(&zone_fp.to_le_bytes());
    }
    u64::from(hasher.finalize())
}

/// Serialize registry to bincode.
///
/// # Errors
///
/// Returns error if serialization fails.
pub fn serialize_registry(registry: &ReverbZoneRegistry) -> Result<Vec<u8>, bincode::Error> {
    bincode::serialize(registry)
}

/// Deserialize registry from bincode.
///
/// # Errors
///
/// Returns error if deserialization fails.
pub fn deserialize_registry(bytes: &[u8]) -> Result<ReverbZoneRegistry, bincode::Error> {
    bincode::deserialize(bytes)
}

/// Serialize a single zone to bincode.
///
/// # Errors
///
/// Returns error if serialization fails.
pub fn serialize_zone(zone: &ReverbZone) -> Result<Vec<u8>, bincode::Error> {
    bincode::serialize(zone)
}

/// Deserialize a zone from bincode.
///
/// # Errors
///
/// Returns error if deserialization fails.
pub fn deserialize_zone(bytes: &[u8]) -> Result<ReverbZone, bincode::Error> {
    bincode::deserialize(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_preset_all_variants() {
        const EXPECTED: [u8; 12] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
        assert_eq!(ReverbPreset::ALL.len(), 12);
        for (preset, expected) in ReverbPreset::ALL.iter().zip(EXPECTED.iter()) {
            assert_eq!(*preset as u8, *expected);
        }
    }

    #[test]
    fn test_preset_configs_valid() {
        for preset in ReverbPreset::ALL {
            let config = preset.config();
            assert!(config.is_valid(), "{preset:?} config should be valid");
        }
    }

    #[test]
    fn test_config_blend() {
        let a = ReverbConfig::SMALL_ROOM;
        let b = ReverbConfig::LARGE_ROOM;
        let blended = a.blend(&b, 0.5);

        assert_relative_eq!(
            blended.wet_dry_mix,
            f32::midpoint(a.wet_dry_mix, b.wet_dry_mix),
            epsilon = 0.01
        );
        assert_relative_eq!(
            blended.decay_time,
            f32::midpoint(a.decay_time, b.decay_time),
            epsilon = 0.01
        );
    }

    #[test]
    fn test_config_clamp() {
        let invalid = ReverbConfig::new(
            -0.5,
            -1.0,
            1.5,
            2.0,
            -0.1,
            -10.0,
            1.5,
            0.0,
            100.0,
            Vec3::ONE,
        );
        let clamped = invalid.clamped();
        assert!(clamped.is_valid());
    }

    #[test]
    fn test_zone_shape_box_contains() {
        let shape = ZoneShape::box_shape(Vec3::new(5.0, 5.0, 5.0));
        assert!(shape.contains(Vec3::ZERO));
        assert!(shape.contains(Vec3::new(4.0, 4.0, 4.0)));
        assert!(!shape.contains(Vec3::new(6.0, 0.0, 0.0)));
    }

    #[test]
    fn test_zone_shape_sphere_contains() {
        let shape = ZoneShape::sphere(10.0);
        assert!(shape.contains(Vec3::ZERO));
        assert!(shape.contains(Vec3::new(5.0, 5.0, 0.0)));
        assert!(!shape.contains(Vec3::new(10.0, 10.0, 0.0)));
    }

    #[test]
    fn test_zone_shape_global() {
        let shape = ZoneShape::Global;
        assert!(shape.contains(Vec3::new(1000.0, 1000.0, 1000.0)));
        assert_relative_eq!(shape.blend_factor(Vec3::splat(999.0), 0.5), 1.0);
    }

    #[test]
    fn test_zone_blend_factor() {
        let shape = ZoneShape::sphere(10.0);
        assert_relative_eq!(shape.blend_factor(Vec3::ZERO, 0.3), 1.0, epsilon = 0.01);
        assert!(shape.blend_factor(Vec3::new(9.0, 0.0, 0.0), 0.3) < 1.0);
        assert_relative_eq!(
            shape.blend_factor(Vec3::new(11.0, 0.0, 0.0), 0.3),
            0.0,
            epsilon = 0.001
        );
    }

    #[test]
    fn test_zone_contains_world_pos() {
        let zone = ReverbZone::new(
            ReverbZoneId::new(1),
            Vec3::new(100.0, 0.0, 0.0),
            ZoneShape::sphere(5.0),
            ReverbConfig::CAVE,
        );

        assert!(zone.contains(Vec3::new(100.0, 0.0, 0.0)));
        assert!(zone.contains(Vec3::new(103.0, 0.0, 0.0)));
        assert!(!zone.contains(Vec3::new(110.0, 0.0, 0.0)));
    }

    #[test]
    fn test_zone_inactive_not_contains() {
        let zone = ReverbZone::new(
            ReverbZoneId::new(1),
            Vec3::ZERO,
            ZoneShape::Global,
            ReverbConfig::CAVE,
        )
        .with_active(false);

        assert!(!zone.contains(Vec3::ZERO));
        assert_relative_eq!(zone.blend_factor(Vec3::ZERO), 0.0);
    }

    #[test]
    fn test_registry_add_remove() {
        let mut registry = ReverbZoneRegistry::new();
        let id = registry.add(ReverbZone::from_preset(
            ReverbZoneId::default(),
            Vec3::ZERO,
            ZoneShape::sphere(10.0),
            ReverbPreset::Cave,
        ));

        assert_eq!(registry.len(), 1);
        assert!(registry.get(id).is_some());

        registry.remove(id);
        assert!(registry.is_empty());
    }

    #[test]
    fn test_registry_zones_at() {
        let mut registry = ReverbZoneRegistry::new();
        registry.add(ReverbZone::new(
            ReverbZoneId::default(),
            Vec3::ZERO,
            ZoneShape::sphere(10.0),
            ReverbConfig::CAVE,
        ));
        registry.add(ReverbZone::new(
            ReverbZoneId::default(),
            Vec3::new(100.0, 0.0, 0.0),
            ZoneShape::sphere(5.0),
            ReverbConfig::SMALL_ROOM,
        ));

        let at_origin = registry.zones_at(Vec3::ZERO);
        assert_eq!(at_origin.len(), 1);

        let at_far = registry.zones_at(Vec3::new(100.0, 0.0, 0.0));
        assert_eq!(at_far.len(), 1);
    }

    #[test]
    fn test_sample_single_zone() {
        let zones = vec![ReverbZone::new(
            ReverbZoneId::new(1),
            Vec3::ZERO,
            ZoneShape::sphere(10.0),
            ReverbConfig::CAVE,
        )];

        let sample = sample_reverb(&zones, Vec3::ZERO);

        assert!(sample.has_reverb());
        assert_eq!(sample.contributions.len(), 1);
        assert_relative_eq!(
            sample.config.wet_dry_mix,
            ReverbConfig::CAVE.wet_dry_mix,
            epsilon = 0.01
        );
    }

    #[test]
    fn test_sample_blended_zones() {
        let zones = vec![
            ReverbZone::new(
                ReverbZoneId::new(1),
                Vec3::ZERO,
                ZoneShape::sphere(10.0),
                ReverbConfig::CAVE,
            ),
            ReverbZone::new(
                ReverbZoneId::new(2),
                Vec3::ZERO,
                ZoneShape::sphere(10.0),
                ReverbConfig::SMALL_ROOM,
            ),
        ];

        let sample = sample_reverb(&zones, Vec3::ZERO);

        assert_eq!(sample.contributions.len(), 2);
        let expected_mix = f32::midpoint(
            ReverbConfig::CAVE.wet_dry_mix,
            ReverbConfig::SMALL_ROOM.wet_dry_mix,
        );
        assert_relative_eq!(sample.config.wet_dry_mix, expected_mix, epsilon = 0.05);
    }

    #[test]
    fn test_sample_empty_zones() {
        let sample = sample_reverb(&[], Vec3::ZERO);
        assert!(!sample.has_reverb());
        assert!(sample.contributions.is_empty());
    }

    #[test]
    fn test_sample_outside_all_zones() {
        let zones = vec![ReverbZone::new(
            ReverbZoneId::new(1),
            Vec3::ZERO,
            ZoneShape::sphere(5.0),
            ReverbConfig::CAVE,
        )];

        let sample = sample_reverb(&zones, Vec3::new(100.0, 0.0, 0.0));
        assert!(!sample.has_reverb());
    }

    #[test]
    fn test_sample_priority() {
        let zones = vec![
            ReverbZone::new(
                ReverbZoneId::new(1),
                Vec3::ZERO,
                ZoneShape::Global,
                ReverbConfig::CAVE,
            )
            .with_priority(0),
            ReverbZone::new(
                ReverbZoneId::new(2),
                Vec3::ZERO,
                ZoneShape::Global,
                ReverbConfig::SMALL_ROOM,
            )
            .with_priority(10),
        ];

        let sample = sample_reverb_priority(&zones, Vec3::ZERO, 1);
        assert_eq!(sample.contributions.len(), 1);
        assert_eq!(sample.dominant_zone(), Some(ReverbZoneId::new(2)));
    }

    #[test]
    fn test_fingerprint_deterministic() {
        let zone = ReverbZone::new(
            ReverbZoneId::new(1),
            Vec3::new(10.0, 20.0, 30.0),
            ZoneShape::sphere(5.0),
            ReverbConfig::CATHEDRAL,
        );

        let fp1 = compute_zone_fingerprint(&zone);
        let fp2 = compute_zone_fingerprint(&zone);
        assert_eq!(fp1, fp2);
    }

    #[test]
    fn test_fingerprint_sensitive() {
        let zone1 = ReverbZone::new(
            ReverbZoneId::new(1),
            Vec3::ZERO,
            ZoneShape::sphere(5.0),
            ReverbConfig::CAVE,
        );
        let zone2 = ReverbZone::new(
            ReverbZoneId::new(1),
            Vec3::ZERO,
            ZoneShape::sphere(5.0),
            ReverbConfig::CATHEDRAL,
        );

        assert_ne!(
            compute_zone_fingerprint(&zone1),
            compute_zone_fingerprint(&zone2)
        );
    }

    #[test]
    fn test_registry_fingerprint() {
        let mut registry = ReverbZoneRegistry::new();
        registry.add(ReverbZone::from_preset(
            ReverbZoneId::default(),
            Vec3::ZERO,
            ZoneShape::sphere(10.0),
            ReverbPreset::Cave,
        ));

        let fp1 = compute_registry_fingerprint(&registry);
        let fp2 = compute_registry_fingerprint(&registry);
        assert_eq!(fp1, fp2);
    }

    #[test]
    fn test_bincode_zone_roundtrip() {
        let zone = ReverbZone::new(
            ReverbZoneId::new(42),
            Vec3::new(1.0, 2.0, 3.0),
            ZoneShape::box_shape(Vec3::new(5.0, 10.0, 15.0)),
            ReverbConfig::UNDERWATER,
        )
        .with_priority(5)
        .with_falloff(0.25);

        let bytes = serialize_zone(&zone).expect("serialize");
        let recovered = deserialize_zone(&bytes).expect("deserialize");

        assert_eq!(recovered.id, zone.id);
        assert_eq!(recovered.center, zone.center);
        assert_eq!(recovered.priority, zone.priority);
        assert_relative_eq!(recovered.falloff, zone.falloff, epsilon = 0.001);
    }

    #[test]
    fn test_bincode_registry_roundtrip() {
        let mut registry = ReverbZoneRegistry::new();
        registry.add(ReverbZone::from_preset(
            ReverbZoneId::default(),
            Vec3::new(10.0, 0.0, 0.0),
            ZoneShape::sphere(5.0),
            ReverbPreset::Cave,
        ));
        registry.add(ReverbZone::from_preset(
            ReverbZoneId::default(),
            Vec3::new(-10.0, 0.0, 0.0),
            ZoneShape::box_shape(Vec3::splat(3.0)),
            ReverbPreset::MetalEnclosure,
        ));

        let bytes = serialize_registry(&registry).expect("serialize");
        let recovered = deserialize_registry(&bytes).expect("deserialize");

        assert_eq!(recovered.len(), registry.len());
    }

    #[test]
    fn test_serialization_preserves_fingerprint() {
        let mut registry = ReverbZoneRegistry::new();
        registry.add(ReverbZone::from_preset(
            ReverbZoneId::default(),
            Vec3::ZERO,
            ZoneShape::sphere(10.0),
            ReverbPreset::Cathedral,
        ));

        let fp_before = compute_registry_fingerprint(&registry);
        let bytes = serialize_registry(&registry).expect("serialize");
        let recovered = deserialize_registry(&bytes).expect("deserialize");
        let fp_after = compute_registry_fingerprint(&recovered);

        assert_eq!(fp_before, fp_after);
    }

    #[test]
    fn test_config_with_material() {
        let config = ReverbConfig::CAVE.with_material(AcousticMaterial::Metal);
        let metal_profile = AcousticMaterial::Metal.profile();
        assert_eq!(config.color, metal_profile.reverb_color);
    }

    #[test]
    fn test_sort_by_priority() {
        let mut registry = ReverbZoneRegistry::new();
        registry.add(ReverbZone::default().with_priority(5));
        registry.add(ReverbZone::default().with_priority(10));
        registry.add(ReverbZone::default().with_priority(1));

        registry.sort_by_priority();

        assert_eq!(registry.zones()[0].priority, 10);
        assert_eq!(registry.zones()[1].priority, 5);
        assert_eq!(registry.zones()[2].priority, 1);
    }

    #[test]
    fn test_dominant_zone() {
        let zones = vec![
            ReverbZone::new(
                ReverbZoneId::new(1),
                Vec3::ZERO,
                ZoneShape::sphere(10.0),
                ReverbConfig::CAVE,
            ),
            ReverbZone::new(
                ReverbZoneId::new(2),
                Vec3::new(8.0, 0.0, 0.0),
                ZoneShape::sphere(5.0),
                ReverbConfig::SMALL_ROOM,
            ),
        ];

        let sample = sample_reverb(&zones, Vec3::ZERO);
        assert_eq!(sample.dominant_zone(), Some(ReverbZoneId::new(1)));
    }
}
