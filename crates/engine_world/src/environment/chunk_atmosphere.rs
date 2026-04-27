//! Per-chunk atmosphere storage.

use engine_core::coords::LocalPos;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{AtmosphereCell, AtmosphereLayer, FieldChannel};
use crate::chunk::CHUNK_VOLUME;

/// Per-chunk atmosphere state storage.
///
/// Stores one [`AtmosphereCell`] per voxel within the chunk.
/// Lazily allocates storage; unallocated chunks return outdoor defaults.
#[derive(Clone)]
pub struct ChunkAtmosphere {
    cells: Option<Box<[AtmosphereCell; CHUNK_VOLUME]>>,
}

impl std::fmt::Debug for ChunkAtmosphere {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.cells {
            Some(cells) => {
                let mut counts = [0usize; AtmosphereLayer::COUNT];
                for cell in cells.iter() {
                    counts[cell.layer().as_index()] += 1;
                }
                f.debug_struct("ChunkAtmosphere")
                    .field("indoor", &counts[0])
                    .field("outdoor", &counts[1])
                    .field("exposed", &counts[2])
                    .field("vacuum", &counts[3])
                    .finish()
            }
            None => f
                .debug_struct("ChunkAtmosphere")
                .field("cells", &"unallocated")
                .finish(),
        }
    }
}

impl Serialize for ChunkAtmosphere {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.cells
            .as_ref()
            .map(|b| b.as_slice())
            .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ChunkAtmosphere {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt: Option<Vec<AtmosphereCell>> = Option::deserialize(deserializer)?;
        match opt {
            None => Ok(Self::new()),
            Some(vec) => {
                if vec.len() != CHUNK_VOLUME {
                    return Err(serde::de::Error::invalid_length(
                        vec.len(),
                        &"4096 AtmosphereCell values",
                    ));
                }
                let mut cells = Box::new([AtmosphereCell::outdoor(); CHUNK_VOLUME]);
                cells.copy_from_slice(&vec);
                Ok(Self { cells: Some(cells) })
            }
        }
    }
}

impl Default for ChunkAtmosphere {
    fn default() -> Self {
        Self::new()
    }
}

impl ChunkAtmosphere {
    /// Create new unallocated chunk atmosphere.
    ///
    /// All positions return outdoor atmosphere until explicitly set.
    #[must_use]
    pub const fn new() -> Self {
        Self { cells: None }
    }

    /// Create chunk atmosphere with all cells set to outdoor.
    #[must_use]
    pub fn with_outdoor() -> Self {
        Self {
            cells: Some(Box::new([AtmosphereCell::outdoor(); CHUNK_VOLUME])),
        }
    }

    /// Create chunk atmosphere filled with a specific cell state.
    #[must_use]
    pub fn filled(cell: AtmosphereCell) -> Self {
        Self {
            cells: Some(Box::new([cell; CHUNK_VOLUME])),
        }
    }

    /// Check if storage has been allocated.
    #[must_use]
    pub const fn is_allocated(&self) -> bool {
        self.cells.is_some()
    }

    /// Get the atmosphere cell at a position.
    ///
    /// Returns outdoor default if not allocated.
    #[must_use]
    pub fn get(&self, pos: LocalPos) -> AtmosphereCell {
        match &self.cells {
            Some(cells) => cells[pos.to_index()],
            None => AtmosphereCell::outdoor(),
        }
    }

    /// Get the atmosphere layer at a position.
    #[must_use]
    pub fn layer(&self, pos: LocalPos) -> AtmosphereLayer {
        self.get(pos).layer()
    }

    /// Set the atmosphere cell at a position.
    ///
    /// Allocates storage if not present.
    pub fn set(&mut self, pos: LocalPos, cell: AtmosphereCell) {
        self.ensure_allocated();
        if let Some(cells) = &mut self.cells {
            cells[pos.to_index()] = cell;
        }
    }

    /// Set just the layer at a position, preserving other cell properties.
    pub fn set_layer(&mut self, pos: LocalPos, layer: AtmosphereLayer) {
        self.ensure_allocated();
        if let Some(cells) = &mut self.cells {
            cells[pos.to_index()].set_layer(layer);
        }
    }

    /// Get mutable reference to a cell, allocating if needed.
    pub fn get_mut(&mut self, pos: LocalPos) -> &mut AtmosphereCell {
        self.ensure_allocated();
        &mut self.cells.as_mut().expect("just allocated")[pos.to_index()]
    }

    /// Ensure storage is allocated with outdoor defaults.
    fn ensure_allocated(&mut self) {
        if self.cells.is_none() {
            self.cells = Some(Box::new([AtmosphereCell::outdoor(); CHUNK_VOLUME]));
        }
    }

    /// Get direct read-only access to cells if allocated.
    #[must_use]
    pub fn cells(&self) -> Option<&[AtmosphereCell; CHUNK_VOLUME]> {
        self.cells.as_ref().map(|b| b.as_ref())
    }

    /// Get mutable access to cells, allocating if needed.
    pub fn cells_mut(&mut self) -> &mut [AtmosphereCell; CHUNK_VOLUME] {
        self.ensure_allocated();
        self.cells.as_mut().expect("just allocated")
    }

    /// Fill all cells with a value, allocating if needed.
    pub fn fill(&mut self, cell: AtmosphereCell) {
        self.ensure_allocated();
        if let Some(cells) = &mut self.cells {
            cells.fill(cell);
        }
    }

    /// Clear storage, deallocating back to unallocated state.
    pub fn clear(&mut self) {
        self.cells = None;
    }

    /// Count cells by layer type.
    #[must_use]
    pub fn count_by_layer(&self) -> [usize; AtmosphereLayer::COUNT] {
        let mut counts = [0usize; AtmosphereLayer::COUNT];
        match &self.cells {
            Some(cells) => {
                for cell in cells.iter() {
                    counts[cell.layer().as_index()] += 1;
                }
            }
            None => {
                counts[AtmosphereLayer::Outdoor.as_index()] = CHUNK_VOLUME;
            }
        }
        counts
    }

    /// Check if all cells have a specific layer.
    #[must_use]
    pub fn is_uniform(&self, layer: AtmosphereLayer) -> bool {
        match &self.cells {
            Some(cells) => cells.iter().all(|c| c.layer() == layer),
            None => layer == AtmosphereLayer::Outdoor,
        }
    }

    /// Sample atmosphere with trilinear interpolation for continuous properties.
    ///
    /// Returns interpolated seal quality, ventilation, and contamination.
    /// Layer is taken from the nearest cell (no interpolation for categorical).
    #[must_use]
    pub fn sample(&self, x: f32, y: f32, z: f32) -> AtmosphereSample {
        let cells = match &self.cells {
            Some(c) => c,
            None => return AtmosphereSample::outdoor(),
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

        let c000 = &cells[LocalPos::new(x0, y0, z0).to_index()];
        let c001 = &cells[LocalPos::new(x0, y0, z1).to_index()];
        let c010 = &cells[LocalPos::new(x0, y1, z0).to_index()];
        let c011 = &cells[LocalPos::new(x0, y1, z1).to_index()];
        let c100 = &cells[LocalPos::new(x1, y0, z0).to_index()];
        let c101 = &cells[LocalPos::new(x1, y0, z1).to_index()];
        let c110 = &cells[LocalPos::new(x1, y1, z0).to_index()];
        let c111 = &cells[LocalPos::new(x1, y1, z1).to_index()];

        let interp = |f: fn(&AtmosphereCell) -> f32| -> f32 {
            let v000 = f(c000);
            let v001 = f(c001);
            let v010 = f(c010);
            let v011 = f(c011);
            let v100 = f(c100);
            let v101 = f(c101);
            let v110 = f(c110);
            let v111 = f(c111);

            let v00 = v000 * (1.0 - fx) + v100 * fx;
            let v01 = v001 * (1.0 - fx) + v101 * fx;
            let v10 = v010 * (1.0 - fx) + v110 * fx;
            let v11 = v011 * (1.0 - fx) + v111 * fx;

            let v0 = v00 * (1.0 - fy) + v10 * fy;
            let v1 = v01 * (1.0 - fy) + v11 * fy;

            v0 * (1.0 - fz) + v1 * fz
        };

        let nearest_idx =
            LocalPos::new(x.round() as u32, y.round() as u32, z.round() as u32).to_index();

        AtmosphereSample {
            layer: cells[nearest_idx].layer(),
            seal_quality: interp(AtmosphereCell::seal_quality),
            ventilation: interp(AtmosphereCell::ventilation),
            contamination: interp(AtmosphereCell::contamination),
        }
    }

    /// Apply environmental consequences to scalar fields based on atmosphere.
    ///
    /// Modifies the provided field values for oxygen and pressure based on
    /// the atmosphere layer at each cell.
    pub fn apply_to_fields(&self, fields: &mut super::ChunkFields) {
        match &self.cells {
            Some(cells) => {
                for (idx, cell) in cells.iter().enumerate() {
                    let pos = LocalPos::from_index(idx);
                    let layer = cell.layer();

                    fields.set(FieldChannel::Oxygen, pos, layer.default_oxygen());
                    fields.set(FieldChannel::Pressure, pos, layer.default_pressure());
                }
            }
            None => {
                // Outdoor defaults already match field defaults for O2/pressure
            }
        }
    }
}

/// Sampled atmosphere values with interpolation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AtmosphereSample {
    /// Nearest layer (categorical, not interpolated).
    pub layer: AtmosphereLayer,
    /// Interpolated seal quality.
    pub seal_quality: f32,
    /// Interpolated ventilation rate.
    pub ventilation: f32,
    /// Interpolated contamination level.
    pub contamination: f32,
}

impl AtmosphereSample {
    /// Create a standard outdoor sample.
    #[must_use]
    pub const fn outdoor() -> Self {
        Self {
            layer: AtmosphereLayer::Outdoor,
            seal_quality: 0.0,
            ventilation: 1.0,
            contamination: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_is_unallocated() {
        let atmo = ChunkAtmosphere::new();
        assert!(!atmo.is_allocated());
    }

    #[test]
    fn unallocated_returns_outdoor() {
        let atmo = ChunkAtmosphere::new();
        let cell = atmo.get(LocalPos::new(8, 8, 8));
        assert_eq!(cell.layer(), AtmosphereLayer::Outdoor);
    }

    #[test]
    fn set_allocates() {
        let mut atmo = ChunkAtmosphere::new();
        atmo.set(LocalPos::new(0, 0, 0), AtmosphereCell::indoor_sealed());
        assert!(atmo.is_allocated());
    }

    #[test]
    fn set_and_get() {
        let mut atmo = ChunkAtmosphere::new();
        let pos = LocalPos::new(5, 10, 3);
        let cell = AtmosphereCell::indoor(0.7);

        atmo.set(pos, cell);

        assert_eq!(atmo.get(pos), cell);
    }

    #[test]
    fn set_layer() {
        let mut atmo = ChunkAtmosphere::new();
        let pos = LocalPos::new(0, 0, 0);

        atmo.set_layer(pos, AtmosphereLayer::Indoor);
        assert_eq!(atmo.layer(pos), AtmosphereLayer::Indoor);
    }

    #[test]
    fn get_mut_allocates() {
        let mut atmo = ChunkAtmosphere::new();
        let pos = LocalPos::new(0, 0, 0);

        atmo.get_mut(pos).set_contamination(0.5);

        assert!(atmo.is_allocated());
        assert_eq!(atmo.get(pos).contamination(), 0.5);
    }

    #[test]
    fn with_outdoor() {
        let atmo = ChunkAtmosphere::with_outdoor();
        assert!(atmo.is_allocated());
        assert!(atmo.is_uniform(AtmosphereLayer::Outdoor));
    }

    #[test]
    fn filled() {
        let atmo = ChunkAtmosphere::filled(AtmosphereCell::vacuum());
        assert!(atmo.is_allocated());
        assert!(atmo.is_uniform(AtmosphereLayer::Vacuum));
    }

    #[test]
    fn fill() {
        let mut atmo = ChunkAtmosphere::new();
        atmo.fill(AtmosphereCell::indoor_sealed());
        assert!(atmo.is_allocated());
        assert!(atmo.is_uniform(AtmosphereLayer::Indoor));
    }

    #[test]
    fn clear() {
        let mut atmo = ChunkAtmosphere::with_outdoor();
        assert!(atmo.is_allocated());

        atmo.clear();
        assert!(!atmo.is_allocated());
    }

    #[test]
    fn count_by_layer_unallocated() {
        let atmo = ChunkAtmosphere::new();
        let counts = atmo.count_by_layer();

        assert_eq!(counts[AtmosphereLayer::Outdoor.as_index()], CHUNK_VOLUME);
        assert_eq!(counts[AtmosphereLayer::Indoor.as_index()], 0);
    }

    #[test]
    fn count_by_layer_mixed() {
        let mut atmo = ChunkAtmosphere::with_outdoor();

        for z in 0..8 {
            for y in 0..16 {
                for x in 0..16 {
                    atmo.set_layer(LocalPos::new(x, y, z), AtmosphereLayer::Indoor);
                }
            }
        }

        let counts = atmo.count_by_layer();
        assert_eq!(counts[AtmosphereLayer::Indoor.as_index()], 8 * 16 * 16);
        assert_eq!(counts[AtmosphereLayer::Outdoor.as_index()], 8 * 16 * 16);
    }

    #[test]
    fn is_uniform_true() {
        let atmo = ChunkAtmosphere::filled(AtmosphereCell::vacuum());
        assert!(atmo.is_uniform(AtmosphereLayer::Vacuum));
        assert!(!atmo.is_uniform(AtmosphereLayer::Outdoor));
    }

    #[test]
    fn is_uniform_unallocated() {
        let atmo = ChunkAtmosphere::new();
        assert!(atmo.is_uniform(AtmosphereLayer::Outdoor));
        assert!(!atmo.is_uniform(AtmosphereLayer::Indoor));
    }

    #[test]
    fn sample_unallocated() {
        let atmo = ChunkAtmosphere::new();
        let sample = atmo.sample(8.0, 8.0, 8.0);

        assert_eq!(sample.layer, AtmosphereLayer::Outdoor);
        assert_eq!(sample.seal_quality, 0.0);
        assert_eq!(sample.ventilation, 1.0);
    }

    #[test]
    fn sample_interpolates_contamination() {
        let mut atmo = ChunkAtmosphere::with_outdoor();

        atmo.get_mut(LocalPos::new(0, 0, 0)).set_contamination(0.0);
        atmo.get_mut(LocalPos::new(1, 0, 0)).set_contamination(1.0);

        let sample = atmo.sample(0.5, 0.0, 0.0);
        assert!((sample.contamination - 0.5).abs() < 0.1);
    }

    #[test]
    fn sample_nearest_layer() {
        let mut atmo = ChunkAtmosphere::with_outdoor();

        atmo.set_layer(LocalPos::new(0, 0, 0), AtmosphereLayer::Indoor);
        atmo.set_layer(LocalPos::new(1, 0, 0), AtmosphereLayer::Vacuum);

        let sample_near_0 = atmo.sample(0.3, 0.0, 0.0);
        assert_eq!(sample_near_0.layer, AtmosphereLayer::Indoor);

        let sample_near_1 = atmo.sample(0.7, 0.0, 0.0);
        assert_eq!(sample_near_1.layer, AtmosphereLayer::Vacuum);
    }

    #[test]
    fn cells_access() {
        let atmo = ChunkAtmosphere::new();
        assert!(atmo.cells().is_none());

        let atmo2 = ChunkAtmosphere::with_outdoor();
        assert!(atmo2.cells().is_some());
    }

    #[test]
    fn cells_mut_allocates() {
        let mut atmo = ChunkAtmosphere::new();
        let cells = atmo.cells_mut();
        assert_eq!(cells.len(), CHUNK_VOLUME);
    }

    #[test]
    fn serde_round_trip_unallocated() {
        let atmo = ChunkAtmosphere::new();
        let json = serde_json::to_string(&atmo).unwrap();
        let recovered: ChunkAtmosphere = serde_json::from_str(&json).unwrap();

        assert!(!recovered.is_allocated());
    }

    #[test]
    fn serde_round_trip_allocated() {
        let mut atmo = ChunkAtmosphere::with_outdoor();
        atmo.set(LocalPos::new(0, 0, 0), AtmosphereCell::indoor(0.8));
        atmo.set(LocalPos::new(15, 15, 15), AtmosphereCell::vacuum());

        let json = serde_json::to_string(&atmo).unwrap();
        let recovered: ChunkAtmosphere = serde_json::from_str(&json).unwrap();

        assert!(recovered.is_allocated());
        assert_eq!(
            recovered.get(LocalPos::new(0, 0, 0)).layer(),
            AtmosphereLayer::Indoor
        );
        assert_eq!(
            recovered.get(LocalPos::new(15, 15, 15)).layer(),
            AtmosphereLayer::Vacuum
        );
    }

    #[test]
    fn debug_format() {
        let atmo = ChunkAtmosphere::new();
        let debug = format!("{:?}", atmo);
        assert!(debug.contains("unallocated"));

        let atmo2 = ChunkAtmosphere::with_outdoor();
        let debug2 = format!("{:?}", atmo2);
        assert!(debug2.contains("outdoor"));
    }
}
