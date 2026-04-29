//! Diagnostic sample types for bounded field/state queries.

use glam::Vec3;
use serde::{Deserialize, Serialize};

use super::channel::DiagnosticChannel;
use super::color::DiagnosticColor;
use super::fingerprint::DiagnosticFingerprint;

/// Scalar value with optional normalized intensity.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScalarValue {
    pub raw: f32,
    pub normalized: f32,
}

impl ScalarValue {
    pub const ZERO: Self = Self {
        raw: 0.0,
        normalized: 0.0,
    };

    #[must_use]
    pub const fn new(raw: f32, normalized: f32) -> Self {
        Self { raw, normalized }
    }

    #[must_use]
    pub fn from_range(value: f32, min: f32, max: f32) -> Self {
        let normalized = if (max - min).abs() < f32::EPSILON {
            0.0
        } else {
            ((value - min) / (max - min)).clamp(0.0, 1.0)
        };
        Self {
            raw: value,
            normalized,
        }
    }
}

impl Default for ScalarValue {
    fn default() -> Self {
        Self::ZERO
    }
}

/// Vector value with magnitude and direction.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct VectorValue {
    pub direction: [f32; 3],
    pub magnitude: f32,
    pub normalized_magnitude: f32,
}

impl VectorValue {
    pub const ZERO: Self = Self {
        direction: [0.0, 0.0, 0.0],
        magnitude: 0.0,
        normalized_magnitude: 0.0,
    };

    #[must_use]
    pub fn new(vec: Vec3, max_magnitude: Option<f32>) -> Self {
        let magnitude = vec.length();
        let direction = if magnitude > f32::EPSILON {
            (vec / magnitude).to_array()
        } else {
            [0.0, 0.0, 0.0]
        };
        let normalized_magnitude = max_magnitude.map_or_else(
            || magnitude.min(1.0),
            |max| (magnitude / max).clamp(0.0, 1.0),
        );
        Self {
            direction,
            magnitude,
            normalized_magnitude,
        }
    }

    #[must_use]
    pub fn to_vec3(&self) -> Vec3 {
        Vec3::from_array(self.direction) * self.magnitude
    }
}

impl Default for VectorValue {
    fn default() -> Self {
        Self::ZERO
    }
}

/// A single sampled cell with position, channel, value, and color.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SampleCell {
    pub world_pos: [i32; 3],
    pub local_pos: [u8; 3],
    pub chunk_pos: [i32; 3],
    pub channel: DiagnosticChannel,
    pub scalar: Option<ScalarValue>,
    pub vector: Option<VectorValue>,
    pub color: DiagnosticColor,
    pub intensity: f32,
    pub label: Option<String>,
}

impl SampleCell {
    #[must_use]
    pub fn scalar(
        world_pos: [i32; 3],
        local_pos: [u8; 3],
        chunk_pos: [i32; 3],
        channel: DiagnosticChannel,
        value: ScalarValue,
        color: DiagnosticColor,
    ) -> Self {
        Self {
            world_pos,
            local_pos,
            chunk_pos,
            channel,
            scalar: Some(value),
            vector: None,
            color,
            intensity: value.normalized,
            label: None,
        }
    }

    #[must_use]
    pub fn vector(
        world_pos: [i32; 3],
        local_pos: [u8; 3],
        chunk_pos: [i32; 3],
        channel: DiagnosticChannel,
        value: VectorValue,
        color: DiagnosticColor,
    ) -> Self {
        Self {
            world_pos,
            local_pos,
            chunk_pos,
            channel,
            scalar: None,
            vector: Some(value),
            color,
            intensity: value.normalized_magnitude,
            label: None,
        }
    }

    #[must_use]
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    #[must_use]
    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity.clamp(0.0, 1.0);
        self
    }

    #[must_use]
    pub fn is_active(&self) -> bool {
        self.intensity > 0.001
    }

    fn sort_key(&self) -> (i32, i32, i32, DiagnosticChannel) {
        (
            self.world_pos[0],
            self.world_pos[1],
            self.world_pos[2],
            self.channel,
        )
    }
}

/// Bounded collection of sample cells with stable ordering.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SampleGrid {
    cells: Vec<SampleCell>,
    bounds_min: [i32; 3],
    bounds_max: [i32; 3],
    max_cells: usize,
    sorted: bool,
}

impl SampleGrid {
    pub const DEFAULT_MAX_CELLS: usize = 8192;

    #[must_use]
    pub fn new() -> Self {
        Self {
            cells: Vec::new(),
            bounds_min: [i32::MAX, i32::MAX, i32::MAX],
            bounds_max: [i32::MIN, i32::MIN, i32::MIN],
            max_cells: Self::DEFAULT_MAX_CELLS,
            sorted: true,
        }
    }

    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            cells: Vec::with_capacity(capacity),
            bounds_min: [i32::MAX, i32::MAX, i32::MAX],
            bounds_max: [i32::MIN, i32::MIN, i32::MIN],
            max_cells: Self::DEFAULT_MAX_CELLS,
            sorted: true,
        }
    }

    #[must_use]
    pub fn with_max_cells(mut self, max: usize) -> Self {
        self.max_cells = max;
        self
    }

    pub fn push(&mut self, cell: SampleCell) {
        if self.cells.len() >= self.max_cells {
            return;
        }
        self.update_bounds(&cell);
        self.cells.push(cell);
        self.sorted = false;
    }

    pub fn extend(&mut self, cells: impl IntoIterator<Item = SampleCell>) {
        for cell in cells {
            if self.cells.len() >= self.max_cells {
                break;
            }
            self.update_bounds(&cell);
            self.cells.push(cell);
        }
        self.sorted = false;
    }

    fn update_bounds(&mut self, cell: &SampleCell) {
        for i in 0..3 {
            self.bounds_min[i] = self.bounds_min[i].min(cell.world_pos[i]);
            self.bounds_max[i] = self.bounds_max[i].max(cell.world_pos[i]);
        }
    }

    pub fn ensure_sorted(&mut self) {
        if !self.sorted {
            self.cells.sort_by_key(SampleCell::sort_key);
            self.sorted = true;
        }
    }

    #[must_use]
    pub fn cells(&self) -> &[SampleCell] {
        &self.cells
    }

    #[must_use]
    pub fn into_cells(self) -> Vec<SampleCell> {
        self.cells
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.cells.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    #[must_use]
    pub fn is_full(&self) -> bool {
        self.cells.len() >= self.max_cells
    }

    #[must_use]
    pub fn bounds_min(&self) -> [i32; 3] {
        self.bounds_min
    }

    #[must_use]
    pub fn bounds_max(&self) -> [i32; 3] {
        self.bounds_max
    }

    #[must_use]
    pub fn active_count(&self) -> usize {
        self.cells.iter().filter(|c| c.is_active()).count()
    }

    pub fn filter_by_channel(&mut self, channel: DiagnosticChannel) {
        self.cells.retain(|c| c.channel == channel);
        self.sorted = false;
    }

    pub fn filter_by_intensity(&mut self, min_intensity: f32) {
        self.cells.retain(|c| c.intensity >= min_intensity);
    }

    pub fn clear(&mut self) {
        self.cells.clear();
        self.bounds_min = [i32::MAX, i32::MAX, i32::MAX];
        self.bounds_max = [i32::MIN, i32::MIN, i32::MIN];
        self.sorted = true;
    }

    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn fingerprint(&self) -> DiagnosticFingerprint {
        let mut fp = DiagnosticFingerprint::new();
        fp.update_usize(self.cells.len());
        for cell in &self.cells {
            fp.update_i32_array(&cell.world_pos);
            fp.update_u8(cell.channel.category().as_index() as u8);
            if let Some(ref scalar) = cell.scalar {
                fp.update_f32(scalar.raw);
            }
            if let Some(ref vector) = cell.vector {
                fp.update_f32(vector.magnitude);
            }
        }
        fp
    }

    pub fn iter(&self) -> impl Iterator<Item = &SampleCell> {
        self.cells.iter()
    }

    pub fn iter_active(&self) -> impl Iterator<Item = &SampleCell> {
        self.cells.iter().filter(|c| c.is_active())
    }

    pub fn iter_by_channel(&self, channel: DiagnosticChannel) -> impl Iterator<Item = &SampleCell> {
        self.cells.iter().filter(move |c| c.channel == channel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::FieldChannel;

    const EPSILON: f32 = 0.001;

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < EPSILON
    }

    #[test]
    fn test_scalar_value_from_range() {
        let v = ScalarValue::from_range(50.0, 0.0, 100.0);
        assert!(approx_eq(v.normalized, 0.5));
        assert!(approx_eq(v.raw, 50.0));
    }

    #[test]
    fn test_scalar_value_clamps() {
        let v = ScalarValue::from_range(150.0, 0.0, 100.0);
        assert!(approx_eq(v.normalized, 1.0));
    }

    #[test]
    fn test_scalar_value_zero_range() {
        let v = ScalarValue::from_range(50.0, 50.0, 50.0);
        assert!(approx_eq(v.normalized, 0.0));
    }

    #[test]
    fn test_vector_value_normalization() {
        let v = VectorValue::new(Vec3::new(3.0, 4.0, 0.0), Some(10.0));
        assert!(approx_eq(v.magnitude, 5.0));
        assert!(approx_eq(v.normalized_magnitude, 0.5));
    }

    #[test]
    fn test_vector_value_direction() {
        let v = VectorValue::new(Vec3::new(10.0, 0.0, 0.0), None);
        assert!(approx_eq(v.direction[0], 1.0));
        assert!(approx_eq(v.direction[1], 0.0));
    }

    #[test]
    fn test_sample_grid_bounds() {
        let mut grid = SampleGrid::new();
        grid.push(SampleCell::scalar(
            [0, 0, 0],
            [0, 0, 0],
            [0, 0, 0],
            DiagnosticChannel::Scalar(FieldChannel::Temperature),
            ScalarValue::new(20.0, 0.5),
            DiagnosticColor::RED,
        ));
        grid.push(SampleCell::scalar(
            [10, -5, 3],
            [0, 0, 0],
            [0, 0, 0],
            DiagnosticChannel::Scalar(FieldChannel::Temperature),
            ScalarValue::new(30.0, 0.7),
            DiagnosticColor::RED,
        ));
        assert_eq!(grid.bounds_min(), [0, -5, 0]);
        assert_eq!(grid.bounds_max(), [10, 0, 3]);
    }

    #[test]
    fn test_sample_grid_max_cells() {
        let mut grid = SampleGrid::new().with_max_cells(2);
        for i in 0..5 {
            grid.push(SampleCell::scalar(
                [i, 0, 0],
                [0, 0, 0],
                [0, 0, 0],
                DiagnosticChannel::Scalar(FieldChannel::Temperature),
                ScalarValue::new(20.0, 0.5),
                DiagnosticColor::RED,
            ));
        }
        assert_eq!(grid.len(), 2);
        assert!(grid.is_full());
    }

    #[test]
    fn test_sample_grid_deterministic_ordering() {
        let cells = vec![
            SampleCell::scalar(
                [5, 0, 0],
                [0, 0, 0],
                [0, 0, 0],
                DiagnosticChannel::Scalar(FieldChannel::Temperature),
                ScalarValue::new(20.0, 0.5),
                DiagnosticColor::RED,
            ),
            SampleCell::scalar(
                [0, 0, 0],
                [0, 0, 0],
                [0, 0, 0],
                DiagnosticChannel::Scalar(FieldChannel::Oxygen),
                ScalarValue::new(1.0, 1.0),
                DiagnosticColor::BLUE,
            ),
            SampleCell::scalar(
                [0, 0, 0],
                [0, 0, 0],
                [0, 0, 0],
                DiagnosticChannel::Scalar(FieldChannel::Temperature),
                ScalarValue::new(25.0, 0.6),
                DiagnosticColor::RED,
            ),
        ];

        let mut grid1 = SampleGrid::new();
        grid1.extend(cells.clone());
        grid1.ensure_sorted();

        let mut grid2 = SampleGrid::new();
        grid2.extend(cells.into_iter().rev());
        grid2.ensure_sorted();

        assert_eq!(grid1.fingerprint(), grid2.fingerprint());
    }

    #[test]
    fn test_sample_grid_filter_by_intensity() {
        let mut grid = SampleGrid::new();
        grid.push(SampleCell::scalar(
            [0, 0, 0],
            [0, 0, 0],
            [0, 0, 0],
            DiagnosticChannel::Scalar(FieldChannel::Temperature),
            ScalarValue::new(20.0, 0.1),
            DiagnosticColor::RED,
        ));
        grid.push(SampleCell::scalar(
            [1, 0, 0],
            [0, 0, 0],
            [0, 0, 0],
            DiagnosticChannel::Scalar(FieldChannel::Temperature),
            ScalarValue::new(80.0, 0.8),
            DiagnosticColor::RED,
        ));
        grid.filter_by_intensity(0.5);
        assert_eq!(grid.len(), 1);
    }

    #[test]
    fn test_serde_round_trip() {
        let mut grid = SampleGrid::new();
        grid.push(SampleCell::scalar(
            [1, 2, 3],
            [4, 5, 6],
            [0, 0, 0],
            DiagnosticChannel::Scalar(FieldChannel::Radiation),
            ScalarValue::new(0.5, 0.5),
            DiagnosticColor::GREEN,
        ));
        let json = serde_json::to_string(&grid).unwrap();
        let recovered: SampleGrid = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered.cells()[0].world_pos, [1, 2, 3]);
    }
}
