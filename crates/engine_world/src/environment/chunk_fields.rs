//! Per-chunk environmental field storage.

use engine_core::coords::LocalPos;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::FieldChannel;
use crate::chunk::CHUNK_VOLUME;

/// Storage for a single environmental field channel within a chunk.
///
/// Stores one f32 value per cell, matching the chunk's 16x16x16 grid.
#[derive(Clone)]
pub struct ChannelData {
    values: Box<[f32; CHUNK_VOLUME]>,
}

impl Serialize for ChannelData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.values.as_slice().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ChannelData {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let values_vec: Vec<f32> = Vec::deserialize(deserializer)?;

        if values_vec.len() != CHUNK_VOLUME {
            return Err(serde::de::Error::invalid_length(
                values_vec.len(),
                &"4096 f32 values",
            ));
        }

        let mut values = Box::new([0.0_f32; CHUNK_VOLUME]);
        values.copy_from_slice(&values_vec);

        Ok(Self { values })
    }
}

impl std::fmt::Debug for ChannelData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let non_default = self.values.iter().filter(|&&v| v != 0.0).count();
        f.debug_struct("ChannelData")
            .field("non_default_count", &non_default)
            .finish_non_exhaustive()
    }
}

impl ChannelData {
    /// Create new channel data filled with a default value.
    #[must_use]
    pub fn new(default: f32) -> Self {
        Self {
            values: Box::new([default; CHUNK_VOLUME]),
        }
    }

    /// Get the value at a local position.
    #[must_use]
    pub fn get(&self, pos: LocalPos) -> f32 {
        self.values[pos.to_index()]
    }

    /// Set the value at a local position.
    pub fn set(&mut self, pos: LocalPos, value: f32) {
        self.values[pos.to_index()] = value;
    }

    /// Add to the value at a local position.
    pub fn add(&mut self, pos: LocalPos, delta: f32) {
        self.values[pos.to_index()] += delta;
    }

    /// Get direct access to raw values.
    #[must_use]
    pub fn values(&self) -> &[f32; CHUNK_VOLUME] {
        &self.values
    }

    /// Get mutable access to raw values.
    pub fn values_mut(&mut self) -> &mut [f32; CHUNK_VOLUME] {
        &mut self.values
    }

    /// Fill all cells with a value.
    pub fn fill(&mut self, value: f32) {
        self.values.fill(value);
    }

    /// Apply a function to all values.
    pub fn map_in_place(&mut self, f: impl Fn(f32) -> f32) {
        for v in self.values.iter_mut() {
            *v = f(*v);
        }
    }

    /// Clamp all values to a range.
    pub fn clamp(&mut self, min: f32, max: f32) {
        for v in self.values.iter_mut() {
            *v = v.clamp(min, max);
        }
    }
}

/// Environmental field storage for an entire chunk.
///
/// Supports multiple field channels, each storing per-cell scalar values.
/// Channels are lazily allocated when first written to.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChunkFields {
    channels: [Option<ChannelData>; FieldChannel::COUNT],
}

impl ChunkFields {
    /// Create new chunk fields with no allocated channels.
    ///
    /// Channels will return their default values until written to.
    #[must_use]
    pub fn new() -> Self {
        Self {
            channels: std::array::from_fn(|_| None),
        }
    }

    /// Create chunk fields with all channels pre-allocated with defaults.
    #[must_use]
    pub fn with_defaults() -> Self {
        Self {
            channels: std::array::from_fn(|i| {
                let channel = FieldChannel::from_index(i).expect("valid index");
                Some(ChannelData::new(channel.default_value()))
            }),
        }
    }

    /// Check if a channel has been allocated.
    #[must_use]
    pub fn has_channel(&self, channel: FieldChannel) -> bool {
        self.channels[channel.as_index()].is_some()
    }

    /// Get a value from a channel at a position.
    ///
    /// Returns the channel's default value if not allocated.
    #[must_use]
    pub fn get(&self, channel: FieldChannel, pos: LocalPos) -> f32 {
        match &self.channels[channel.as_index()] {
            Some(data) => data.get(pos),
            None => channel.default_value(),
        }
    }

    /// Set a value in a channel at a position.
    ///
    /// Allocates the channel with default values if not present.
    pub fn set(&mut self, channel: FieldChannel, pos: LocalPos, value: f32) {
        let data = self.ensure_channel(channel);
        data.set(pos, value);
    }

    /// Add to a value in a channel at a position.
    ///
    /// Allocates the channel with default values if not present.
    pub fn add(&mut self, channel: FieldChannel, pos: LocalPos, delta: f32) {
        let data = self.ensure_channel(channel);
        data.add(pos, delta);
    }

    /// Get read-only access to a channel's data.
    #[must_use]
    pub fn channel(&self, channel: FieldChannel) -> Option<&ChannelData> {
        self.channels[channel.as_index()].as_ref()
    }

    /// Get mutable access to a channel's data.
    ///
    /// Allocates the channel if not present.
    pub fn channel_mut(&mut self, channel: FieldChannel) -> &mut ChannelData {
        self.ensure_channel(channel)
    }

    /// Ensure a channel is allocated, returning mutable access.
    fn ensure_channel(&mut self, channel: FieldChannel) -> &mut ChannelData {
        let idx = channel.as_index();
        if self.channels[idx].is_none() {
            self.channels[idx] = Some(ChannelData::new(channel.default_value()));
        }
        self.channels[idx].as_mut().expect("just allocated")
    }

    /// Clamp a channel's values to its valid range.
    pub fn clamp_channel(&mut self, channel: FieldChannel) {
        if let Some(data) = &mut self.channels[channel.as_index()] {
            let min = channel.min_value();
            let max = channel.max_value().unwrap_or(f32::MAX);
            data.clamp(min, max);
        }
    }

    /// Clamp all allocated channels to their valid ranges.
    pub fn clamp_all(&mut self) {
        for channel in FieldChannel::ALL {
            self.clamp_channel(channel);
        }
    }

    /// Clear a channel, deallocating its storage.
    pub fn clear_channel(&mut self, channel: FieldChannel) {
        self.channels[channel.as_index()] = None;
    }

    /// Sample a value with trilinear interpolation.
    ///
    /// The position is in local float coordinates [0, 16).
    /// Values outside the chunk are clamped to edges.
    #[must_use]
    pub fn sample(&self, channel: FieldChannel, x: f32, y: f32, z: f32) -> f32 {
        let data = match &self.channels[channel.as_index()] {
            Some(d) => d,
            None => return channel.default_value(),
        };

        // Clamp to valid range
        let x = x.clamp(0.0, 15.999);
        let y = y.clamp(0.0, 15.999);
        let z = z.clamp(0.0, 15.999);

        // Integer coordinates and fractions
        let x0 = x.floor() as u32;
        let y0 = y.floor() as u32;
        let z0 = z.floor() as u32;

        let x1 = (x0 + 1).min(15);
        let y1 = (y0 + 1).min(15);
        let z1 = (z0 + 1).min(15);

        let fx = x.fract();
        let fy = y.fract();
        let fz = z.fract();

        // Sample 8 corners
        let c000 = data.get(LocalPos::new(x0, y0, z0));
        let c001 = data.get(LocalPos::new(x0, y0, z1));
        let c010 = data.get(LocalPos::new(x0, y1, z0));
        let c011 = data.get(LocalPos::new(x0, y1, z1));
        let c100 = data.get(LocalPos::new(x1, y0, z0));
        let c101 = data.get(LocalPos::new(x1, y0, z1));
        let c110 = data.get(LocalPos::new(x1, y1, z0));
        let c111 = data.get(LocalPos::new(x1, y1, z1));

        // Trilinear interpolation
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

impl Default for ChunkFields {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_data_new() {
        let data = ChannelData::new(5.0);
        assert_eq!(data.get(LocalPos::new(0, 0, 0)), 5.0);
        assert_eq!(data.get(LocalPos::new(15, 15, 15)), 5.0);
    }

    #[test]
    fn test_channel_data_set_get() {
        let mut data = ChannelData::new(0.0);
        let pos = LocalPos::new(5, 10, 3);
        data.set(pos, 42.5);
        assert_eq!(data.get(pos), 42.5);
    }

    #[test]
    fn test_channel_data_add() {
        let mut data = ChannelData::new(10.0);
        let pos = LocalPos::new(0, 0, 0);
        data.add(pos, 5.0);
        assert_eq!(data.get(pos), 15.0);
        data.add(pos, -3.0);
        assert_eq!(data.get(pos), 12.0);
    }

    #[test]
    fn test_channel_data_clamp() {
        let mut data = ChannelData::new(0.0);
        data.set(LocalPos::new(0, 0, 0), -5.0);
        data.set(LocalPos::new(1, 0, 0), 1.5);
        data.clamp(0.0, 1.0);
        assert_eq!(data.get(LocalPos::new(0, 0, 0)), 0.0);
        assert_eq!(data.get(LocalPos::new(1, 0, 0)), 1.0);
    }

    #[test]
    fn test_chunk_fields_default_values() {
        let fields = ChunkFields::new();
        let pos = LocalPos::new(8, 8, 8);
        assert_eq!(fields.get(FieldChannel::Temperature, pos), 20.0);
        assert_eq!(fields.get(FieldChannel::Oxygen, pos), 1.0);
        assert_eq!(fields.get(FieldChannel::Radiation, pos), 0.0);
    }

    #[test]
    fn test_chunk_fields_lazy_allocation() {
        let mut fields = ChunkFields::new();
        assert!(!fields.has_channel(FieldChannel::Temperature));
        assert!(fields.is_empty());

        fields.set(FieldChannel::Temperature, LocalPos::new(0, 0, 0), 30.0);
        assert!(fields.has_channel(FieldChannel::Temperature));
        assert!(!fields.is_empty());
        assert_eq!(fields.allocated_count(), 1);
    }

    #[test]
    fn test_chunk_fields_set_get() {
        let mut fields = ChunkFields::new();
        let pos = LocalPos::new(5, 10, 3);

        fields.set(FieldChannel::Radiation, pos, 0.75);
        assert_eq!(fields.get(FieldChannel::Radiation, pos), 0.75);
    }

    #[test]
    fn test_chunk_fields_add() {
        let mut fields = ChunkFields::new();
        let pos = LocalPos::new(0, 0, 0);

        // First add allocates with default
        fields.add(FieldChannel::Temperature, pos, 5.0);
        assert_eq!(fields.get(FieldChannel::Temperature, pos), 25.0); // 20 + 5
    }

    #[test]
    fn test_chunk_fields_clamp_channel() {
        let mut fields = ChunkFields::new();
        let pos = LocalPos::new(0, 0, 0);

        fields.set(FieldChannel::Oxygen, pos, 1.5);
        fields.clamp_channel(FieldChannel::Oxygen);
        assert_eq!(fields.get(FieldChannel::Oxygen, pos), 1.0);
    }

    #[test]
    fn test_chunk_fields_clear_channel() {
        let mut fields = ChunkFields::new();
        fields.set(FieldChannel::Corruption, LocalPos::new(0, 0, 0), 0.5);
        assert!(fields.has_channel(FieldChannel::Corruption));

        fields.clear_channel(FieldChannel::Corruption);
        assert!(!fields.has_channel(FieldChannel::Corruption));
    }

    #[test]
    fn test_chunk_fields_sample_unallocated() {
        let fields = ChunkFields::new();
        let value = fields.sample(FieldChannel::Temperature, 8.5, 8.5, 8.5);
        assert_eq!(value, FieldChannel::Temperature.default_value());
    }

    #[test]
    fn test_chunk_fields_sample_interpolation() {
        let mut fields = ChunkFields::new();

        // Set two adjacent cells
        fields.set(FieldChannel::Humidity, LocalPos::new(0, 0, 0), 0.0);
        fields.set(FieldChannel::Humidity, LocalPos::new(1, 0, 0), 1.0);

        // Sample at midpoint
        let mid = fields.sample(FieldChannel::Humidity, 0.5, 0.0, 0.0);
        assert!((mid - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_chunk_fields_with_defaults() {
        let fields = ChunkFields::with_defaults();
        assert_eq!(fields.allocated_count(), FieldChannel::COUNT);
        for channel in FieldChannel::ALL {
            assert!(fields.has_channel(channel));
        }
    }
}
