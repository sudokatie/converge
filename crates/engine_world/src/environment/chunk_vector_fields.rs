//! Per-chunk vector environmental field storage.

use engine_core::coords::LocalPos;
use glam::Vec3;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::VectorFieldChannel;
use crate::chunk::CHUNK_VOLUME;

/// Storage for a single vector environmental field channel within a chunk.
///
/// Stores one Vec3 value per cell, matching the chunk's 16x16x16 grid.
#[derive(Clone)]
pub struct VectorChannelData {
    values: Box<[Vec3; CHUNK_VOLUME]>,
}

impl Serialize for VectorChannelData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let flat: Vec<[f32; 3]> = self.values.iter().map(|v| [v.x, v.y, v.z]).collect();
        flat.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for VectorChannelData {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let flat: Vec<[f32; 3]> = Vec::deserialize(deserializer)?;

        if flat.len() != CHUNK_VOLUME {
            return Err(serde::de::Error::invalid_length(
                flat.len(),
                &"4096 Vec3 values",
            ));
        }

        let mut values = Box::new([Vec3::ZERO; CHUNK_VOLUME]);
        for (i, arr) in flat.into_iter().enumerate() {
            values[i] = Vec3::new(arr[0], arr[1], arr[2]);
        }

        Ok(Self { values })
    }
}

impl std::fmt::Debug for VectorChannelData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let non_zero = self
            .values
            .iter()
            .filter(|v| v.length_squared() > 0.0)
            .count();
        f.debug_struct("VectorChannelData")
            .field("non_zero_count", &non_zero)
            .finish_non_exhaustive()
    }
}

impl VectorChannelData {
    /// Create new channel data filled with a default value.
    #[must_use]
    pub fn new(default: Vec3) -> Self {
        Self {
            values: Box::new([default; CHUNK_VOLUME]),
        }
    }

    /// Get the value at a local position.
    #[must_use]
    pub fn get(&self, pos: LocalPos) -> Vec3 {
        self.values[pos.to_index()]
    }

    /// Set the value at a local position.
    pub fn set(&mut self, pos: LocalPos, value: Vec3) {
        self.values[pos.to_index()] = value;
    }

    /// Add to the value at a local position.
    pub fn add(&mut self, pos: LocalPos, delta: Vec3) {
        self.values[pos.to_index()] += delta;
    }

    /// Get direct access to raw values.
    #[must_use]
    pub fn values(&self) -> &[Vec3; CHUNK_VOLUME] {
        &self.values
    }

    /// Get mutable access to raw values.
    pub fn values_mut(&mut self) -> &mut [Vec3; CHUNK_VOLUME] {
        &mut self.values
    }

    /// Fill all cells with a value.
    pub fn fill(&mut self, value: Vec3) {
        self.values.fill(value);
    }

    /// Apply a function to all values.
    pub fn map_in_place(&mut self, f: impl Fn(Vec3) -> Vec3) {
        for v in self.values.iter_mut() {
            *v = f(*v);
        }
    }

    /// Clamp all values to a maximum magnitude.
    pub fn clamp_magnitude(&mut self, max_magnitude: f32) {
        for v in self.values.iter_mut() {
            let len = v.length();
            if len > max_magnitude {
                *v = *v * (max_magnitude / len);
            }
        }
    }

    /// Normalize all non-zero vectors.
    pub fn normalize_all(&mut self) {
        for v in self.values.iter_mut() {
            if v.length_squared() > 0.0 {
                *v = v.normalize();
            }
        }
    }
}

/// Vector environmental field storage for an entire chunk.
///
/// Supports multiple vector field channels, each storing per-cell Vec3 values.
/// Channels are lazily allocated when first written to.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChunkVectorFields {
    channels: [Option<VectorChannelData>; VectorFieldChannel::COUNT],
}

impl ChunkVectorFields {
    /// Create new chunk vector fields with no allocated channels.
    ///
    /// Channels will return their default values until written to.
    #[must_use]
    pub fn new() -> Self {
        Self {
            channels: std::array::from_fn(|_| None),
        }
    }

    /// Create chunk vector fields with all channels pre-allocated with defaults.
    #[must_use]
    pub fn with_defaults() -> Self {
        Self {
            channels: std::array::from_fn(|i| {
                let channel = VectorFieldChannel::from_index(i).expect("valid index");
                Some(VectorChannelData::new(channel.default_value()))
            }),
        }
    }

    /// Check if a channel has been allocated.
    #[must_use]
    pub fn has_vector_channel(&self, channel: VectorFieldChannel) -> bool {
        self.channels[channel.as_index()].is_some()
    }

    /// Get a value from a channel at a position.
    ///
    /// Returns the channel's default value if not allocated.
    #[must_use]
    pub fn get(&self, channel: VectorFieldChannel, pos: LocalPos) -> Vec3 {
        match &self.channels[channel.as_index()] {
            Some(data) => data.get(pos),
            None => channel.default_value(),
        }
    }

    /// Set a value in a channel at a position.
    ///
    /// Allocates the channel with default values if not present.
    pub fn set(&mut self, channel: VectorFieldChannel, pos: LocalPos, value: Vec3) {
        let data = self.ensure_channel(channel);
        data.set(pos, value);
    }

    /// Add to a value in a channel at a position.
    ///
    /// Allocates the channel with default values if not present.
    pub fn add(&mut self, channel: VectorFieldChannel, pos: LocalPos, delta: Vec3) {
        let data = self.ensure_channel(channel);
        data.add(pos, delta);
    }

    /// Get read-only access to a channel's data.
    #[must_use]
    pub fn channel(&self, channel: VectorFieldChannel) -> Option<&VectorChannelData> {
        self.channels[channel.as_index()].as_ref()
    }

    /// Get mutable access to a channel's data.
    ///
    /// Allocates the channel if not present.
    pub fn channel_mut(&mut self, channel: VectorFieldChannel) -> &mut VectorChannelData {
        self.ensure_channel(channel)
    }

    /// Ensure a channel is allocated, returning mutable access.
    fn ensure_channel(&mut self, channel: VectorFieldChannel) -> &mut VectorChannelData {
        let idx = channel.as_index();
        if self.channels[idx].is_none() {
            self.channels[idx] = Some(VectorChannelData::new(channel.default_value()));
        }
        self.channels[idx].as_mut().expect("just allocated")
    }

    /// Clamp a channel's values to its maximum magnitude.
    pub fn clamp_channel(&mut self, channel: VectorFieldChannel) {
        if let Some(max_mag) = channel.max_magnitude() {
            if let Some(data) = &mut self.channels[channel.as_index()] {
                data.clamp_magnitude(max_mag);
            }
        }
    }

    /// Clamp all allocated channels to their valid ranges.
    pub fn clamp_all(&mut self) {
        for channel in VectorFieldChannel::ALL {
            self.clamp_channel(channel);
        }
    }

    /// Clear a channel, deallocating its storage.
    pub fn clear_channel(&mut self, channel: VectorFieldChannel) {
        self.channels[channel.as_index()] = None;
    }

    /// Sample a value with trilinear interpolation.
    ///
    /// The position is in local float coordinates [0, 16).
    /// Values outside the chunk are clamped to edges.
    #[must_use]
    pub fn sample(&self, channel: VectorFieldChannel, x: f32, y: f32, z: f32) -> Vec3 {
        let data = match &self.channels[channel.as_index()] {
            Some(d) => d,
            None => return channel.default_value(),
        };

        let x = x.clamp(0.0, 15.999);
        let y = y.clamp(0.0, 15.999);
        let z = z.clamp(0.0, 15.999);

        let x0 = x.floor() as u32;
        let y0 = y.floor() as u32;
        let z0 = z.floor() as u32;

        let x1 = (x0 + 1).min(15);
        let y1 = (y0 + 1).min(15);
        let z1 = (z0 + 1).min(15);

        let fx = x.fract();
        let fy = y.fract();
        let fz = z.fract();

        let c000 = data.get(LocalPos::new(x0, y0, z0));
        let c001 = data.get(LocalPos::new(x0, y0, z1));
        let c010 = data.get(LocalPos::new(x0, y1, z0));
        let c011 = data.get(LocalPos::new(x0, y1, z1));
        let c100 = data.get(LocalPos::new(x1, y0, z0));
        let c101 = data.get(LocalPos::new(x1, y0, z1));
        let c110 = data.get(LocalPos::new(x1, y1, z0));
        let c111 = data.get(LocalPos::new(x1, y1, z1));

        let c00 = c000 * (1.0 - fx) + c100 * fx;
        let c01 = c001 * (1.0 - fx) + c101 * fx;
        let c10 = c010 * (1.0 - fx) + c110 * fx;
        let c11 = c011 * (1.0 - fx) + c111 * fx;

        let c0 = c00 * (1.0 - fy) + c10 * fy;
        let c1 = c01 * (1.0 - fy) + c11 * fy;

        c0 * (1.0 - fz) + c1 * fz
    }

    /// Count allocated channels.
    #[must_use]
    pub fn allocated_count(&self) -> usize {
        self.channels.iter().filter(|c| c.is_some()).count()
    }

    /// Check if all channels are unallocated.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.channels.iter().all(|c| c.is_none())
    }
}

impl Default for ChunkVectorFields {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_vector_channel_data_new() {
        let data = VectorChannelData::new(Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(data.get(LocalPos::new(0, 0, 0)), Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(
            data.get(LocalPos::new(15, 15, 15)),
            Vec3::new(1.0, 2.0, 3.0)
        );
    }

    #[test]
    fn test_vector_channel_data_set_get() {
        let mut data = VectorChannelData::new(Vec3::ZERO);
        let pos = LocalPos::new(5, 10, 3);
        data.set(pos, Vec3::new(1.0, -2.0, 3.5));
        assert_eq!(data.get(pos), Vec3::new(1.0, -2.0, 3.5));
    }

    #[test]
    fn test_vector_channel_data_add() {
        let mut data = VectorChannelData::new(Vec3::new(1.0, 1.0, 1.0));
        let pos = LocalPos::new(0, 0, 0);
        data.add(pos, Vec3::new(0.5, -0.5, 1.0));
        assert_eq!(data.get(pos), Vec3::new(1.5, 0.5, 2.0));
    }

    #[test]
    fn test_vector_channel_data_clamp_magnitude() {
        let mut data = VectorChannelData::new(Vec3::ZERO);
        data.set(LocalPos::new(0, 0, 0), Vec3::new(10.0, 0.0, 0.0));
        data.set(LocalPos::new(1, 0, 0), Vec3::new(0.5, 0.0, 0.0));
        data.clamp_magnitude(2.0);

        let v0 = data.get(LocalPos::new(0, 0, 0));
        assert_relative_eq!(v0.length(), 2.0, epsilon = 0.001);

        let v1 = data.get(LocalPos::new(1, 0, 0));
        assert_relative_eq!(v1.length(), 0.5, epsilon = 0.001);
    }

    #[test]
    fn test_chunk_vector_fields_default_values() {
        let fields = ChunkVectorFields::new();
        let pos = LocalPos::new(8, 8, 8);
        assert_eq!(fields.get(VectorFieldChannel::Wind, pos), Vec3::ZERO);
        assert_eq!(
            fields.get(VectorFieldChannel::GravityOverride, pos),
            Vec3::new(0.0, -9.81, 0.0)
        );
    }

    #[test]
    fn test_chunk_vector_fields_lazy_allocation() {
        let mut fields = ChunkVectorFields::new();
        assert!(!fields.has_vector_channel(VectorFieldChannel::Wind));
        assert!(fields.is_empty());

        fields.set(VectorFieldChannel::Wind, LocalPos::new(0, 0, 0), Vec3::X);
        assert!(fields.has_vector_channel(VectorFieldChannel::Wind));
        assert!(!fields.is_empty());
        assert_eq!(fields.allocated_count(), 1);
    }

    #[test]
    fn test_chunk_vector_fields_set_get() {
        let mut fields = ChunkVectorFields::new();
        let pos = LocalPos::new(5, 10, 3);

        fields.set(
            VectorFieldChannel::WaterCurrent,
            pos,
            Vec3::new(0.0, -1.0, 0.5),
        );
        assert_eq!(
            fields.get(VectorFieldChannel::WaterCurrent, pos),
            Vec3::new(0.0, -1.0, 0.5)
        );
    }

    #[test]
    fn test_chunk_vector_fields_add() {
        let mut fields = ChunkVectorFields::new();
        let pos = LocalPos::new(0, 0, 0);

        fields.add(VectorFieldChannel::Wind, pos, Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(
            fields.get(VectorFieldChannel::Wind, pos),
            Vec3::new(1.0, 0.0, 0.0)
        );
    }

    #[test]
    fn test_chunk_vector_fields_clamp_channel() {
        let mut fields = ChunkVectorFields::new();
        let pos = LocalPos::new(0, 0, 0);

        fields.set(VectorFieldChannel::Wind, pos, Vec3::new(200.0, 0.0, 0.0));
        fields.clamp_channel(VectorFieldChannel::Wind);

        let v = fields.get(VectorFieldChannel::Wind, pos);
        assert_relative_eq!(v.length(), 100.0, epsilon = 0.001);
    }

    #[test]
    fn test_chunk_vector_fields_clear_channel() {
        let mut fields = ChunkVectorFields::new();
        fields.set(
            VectorFieldChannel::HazardSpread,
            LocalPos::new(0, 0, 0),
            Vec3::X,
        );
        assert!(fields.has_vector_channel(VectorFieldChannel::HazardSpread));

        fields.clear_channel(VectorFieldChannel::HazardSpread);
        assert!(!fields.has_vector_channel(VectorFieldChannel::HazardSpread));
    }

    #[test]
    fn test_chunk_vector_fields_sample_unallocated() {
        let fields = ChunkVectorFields::new();
        let value = fields.sample(VectorFieldChannel::Wind, 8.5, 8.5, 8.5);
        assert_eq!(value, VectorFieldChannel::Wind.default_value());
    }

    #[test]
    fn test_chunk_vector_fields_sample_interpolation() {
        let mut fields = ChunkVectorFields::new();

        fields.set(VectorFieldChannel::Wind, LocalPos::new(0, 0, 0), Vec3::ZERO);
        fields.set(
            VectorFieldChannel::Wind,
            LocalPos::new(1, 0, 0),
            Vec3::new(2.0, 0.0, 0.0),
        );

        let mid = fields.sample(VectorFieldChannel::Wind, 0.5, 0.0, 0.0);
        assert_relative_eq!(mid.x, 1.0, epsilon = 0.01);
        assert_relative_eq!(mid.y, 0.0, epsilon = 0.01);
        assert_relative_eq!(mid.z, 0.0, epsilon = 0.01);
    }

    #[test]
    fn test_chunk_vector_fields_with_defaults() {
        let fields = ChunkVectorFields::with_defaults();
        assert_eq!(fields.allocated_count(), VectorFieldChannel::COUNT);
        for channel in VectorFieldChannel::ALL {
            assert!(fields.has_vector_channel(channel));
        }
    }

    #[test]
    fn test_serialization_length_validation() {
        let data = VectorChannelData::new(Vec3::ONE);
        let serialized = serde_json::to_string(&data).unwrap();
        let deserialized: VectorChannelData = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.get(LocalPos::new(0, 0, 0)), Vec3::ONE);

        let bad_json = "[[1.0,2.0,3.0],[4.0,5.0,6.0]]";
        let result: Result<VectorChannelData, _> = serde_json::from_str(bad_json);
        assert!(result.is_err());
    }
}
