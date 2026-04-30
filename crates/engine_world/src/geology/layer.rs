//! Geological layer and stratum definitions.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::identity::{LayerId, MaterialId, RockType};

/// A boundary between geological layers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LayerBoundary {
    /// Depth of the boundary.
    pub depth: f32,
    /// Thickness of transition zone.
    pub transition_thickness: f32,
    /// Whether this is a conformable boundary.
    pub conformable: bool,
}

impl LayerBoundary {
    #[must_use]
    pub fn new(depth: f32) -> Self {
        Self {
            depth: depth.max(0.0),
            transition_thickness: 1.0,
            conformable: true,
        }
    }

    #[must_use]
    pub fn with_transition(mut self, thickness: f32) -> Self {
        self.transition_thickness = thickness.clamp(0.1, 10.0);
        self
    }

    #[must_use]
    pub fn unconformable(mut self) -> Self {
        self.conformable = false;
        self
    }

    #[must_use]
    pub fn top_depth(&self) -> f32 {
        self.depth
    }

    #[must_use]
    pub fn bottom_depth(&self) -> f32 {
        self.depth + self.transition_thickness
    }

    #[must_use]
    pub fn blend_factor(&self, depth: f32) -> f32 {
        if depth <= self.depth {
            0.0
        } else if depth >= self.depth + self.transition_thickness {
            1.0
        } else {
            (depth - self.depth) / self.transition_thickness
        }
    }
}

/// A single geological stratum (thin layer of consistent material).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Stratum {
    /// Material identifier.
    pub material_id: MaterialId,
    /// Rock type.
    pub rock_type: RockType,
    /// Thickness of this stratum.
    pub thickness: f32,
    /// Porosity (0-1).
    pub porosity: f32,
    /// Permeability coefficient.
    pub permeability: f32,
    /// Base compressive strength.
    pub strength: f32,
}

impl Stratum {
    #[must_use]
    pub fn new(material_id: MaterialId, rock_type: RockType, thickness: f32) -> Self {
        Self {
            material_id,
            rock_type,
            thickness: thickness.max(0.1),
            porosity: 0.1,
            permeability: 0.01,
            strength: rock_type.compressive_strength(),
        }
    }

    #[must_use]
    pub fn with_porosity(mut self, porosity: f32) -> Self {
        self.porosity = porosity.clamp(0.0, 0.5);
        self
    }

    #[must_use]
    pub fn with_permeability(mut self, permeability: f32) -> Self {
        self.permeability = permeability.clamp(0.0, 1.0);
        self
    }

    #[must_use]
    pub fn with_strength(mut self, strength: f32) -> Self {
        self.strength = strength.max(0.0);
        self
    }

    #[must_use]
    pub fn density(&self) -> f32 {
        self.rock_type.base_density() * (1.0 - self.porosity)
    }

    #[must_use]
    pub fn thermal_conductivity(&self) -> f32 {
        self.rock_type.thermal_conductivity() * (1.0 - self.porosity * 0.5)
    }

    #[must_use]
    pub fn effective_strength(&self, depth_pressure: f32) -> f32 {
        self.strength * (1.0 + depth_pressure * 0.01)
    }
}

/// A geological layer composed of one or more strata.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GeologicalLayer {
    /// Layer identifier.
    pub id: LayerId,
    /// Layer name.
    pub name: String,
    /// Upper boundary depth.
    pub top_depth: f32,
    /// Lower boundary depth.
    pub bottom_depth: f32,
    /// Strata within this layer (ordered top to bottom).
    strata: Vec<Stratum>,
    /// Upper boundary definition.
    pub upper_boundary: LayerBoundary,
    /// Lower boundary definition.
    pub lower_boundary: LayerBoundary,
    /// Base temperature at this layer.
    pub base_temperature: f32,
    /// Base pressure at this layer.
    pub base_pressure: f32,
}

impl GeologicalLayer {
    #[must_use]
    pub fn new(id: LayerId, name: impl Into<String>, top_depth: f32, bottom_depth: f32) -> Self {
        let top = top_depth.max(0.0);
        let bottom = bottom_depth.max(top + 1.0);
        Self {
            id,
            name: name.into(),
            top_depth: top,
            bottom_depth: bottom,
            strata: Vec::new(),
            upper_boundary: LayerBoundary::new(top),
            lower_boundary: LayerBoundary::new(bottom),
            base_temperature: 20.0 + top * 0.03,
            base_pressure: top * 0.1,
        }
    }

    #[must_use]
    pub fn with_stratum(mut self, stratum: Stratum) -> Self {
        self.strata.push(stratum);
        self
    }

    pub fn add_stratum(&mut self, stratum: Stratum) {
        self.strata.push(stratum);
    }

    #[must_use]
    pub fn with_upper_boundary(mut self, boundary: LayerBoundary) -> Self {
        self.upper_boundary = boundary;
        self
    }

    #[must_use]
    pub fn with_lower_boundary(mut self, boundary: LayerBoundary) -> Self {
        self.lower_boundary = boundary;
        self
    }

    #[must_use]
    pub fn with_base_temperature(mut self, temp: f32) -> Self {
        self.base_temperature = temp;
        self
    }

    #[must_use]
    pub fn with_base_pressure(mut self, pressure: f32) -> Self {
        self.base_pressure = pressure.max(0.0);
        self
    }

    #[must_use]
    pub fn strata(&self) -> &[Stratum] {
        &self.strata
    }

    #[must_use]
    pub fn thickness(&self) -> f32 {
        self.bottom_depth - self.top_depth
    }

    #[must_use]
    pub fn contains_depth(&self, depth: f32) -> bool {
        depth >= self.top_depth && depth < self.bottom_depth
    }

    #[must_use]
    pub fn relative_depth(&self, absolute_depth: f32) -> f32 {
        (absolute_depth - self.top_depth).clamp(0.0, self.thickness())
    }

    #[must_use]
    pub fn stratum_at_depth(&self, depth: f32) -> Option<&Stratum> {
        if !self.contains_depth(depth) {
            return None;
        }

        let mut current_depth = self.top_depth;
        for stratum in &self.strata {
            if depth < current_depth + stratum.thickness {
                return Some(stratum);
            }
            current_depth += stratum.thickness;
        }

        self.strata.last()
    }

    #[must_use]
    pub fn average_density(&self) -> f32 {
        if self.strata.is_empty() {
            return RockType::default().base_density();
        }

        let total_thickness: f32 = self.strata.iter().map(|s| s.thickness).sum();
        if total_thickness == 0.0 {
            return RockType::default().base_density();
        }

        let weighted_sum: f32 = self.strata.iter().map(|s| s.density() * s.thickness).sum();

        weighted_sum / total_thickness
    }

    #[must_use]
    pub fn average_permeability(&self) -> f32 {
        if self.strata.is_empty() {
            return 0.01;
        }

        let total_thickness: f32 = self.strata.iter().map(|s| s.thickness).sum();
        if total_thickness == 0.0 {
            return 0.01;
        }

        let weighted_sum: f32 = self
            .strata
            .iter()
            .map(|s| s.permeability * s.thickness)
            .sum();

        weighted_sum / total_thickness
    }

    #[must_use]
    pub fn min_strength(&self) -> f32 {
        self.strata
            .iter()
            .map(|s| s.strength)
            .min_by(f32::total_cmp)
            .unwrap_or(100.0)
    }

    #[must_use]
    pub fn temperature_at_depth(&self, depth: f32, gradient: f32) -> f32 {
        let relative = self.relative_depth(depth);
        self.base_temperature + relative * gradient
    }

    #[must_use]
    pub fn pressure_at_depth(&self, depth: f32, coefficient: f32) -> f32 {
        let relative = self.relative_depth(depth);
        self.base_pressure + relative * coefficient
    }

    #[must_use]
    pub fn fingerprint(&self) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&self.id.raw().to_le_bytes());
        hasher.update(&self.top_depth.to_le_bytes());
        hasher.update(&self.bottom_depth.to_le_bytes());
        hasher.update(&(self.strata.len() as u64).to_le_bytes());
        for stratum in &self.strata {
            hasher.update(&stratum.material_id.raw().to_le_bytes());
            hasher.update(&stratum.thickness.to_le_bytes());
        }
        hasher.finalize()
    }
}

/// Collection of geological layers ordered by depth.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LayerStack {
    layers: BTreeMap<LayerId, GeologicalLayer>,
    depth_index: Vec<(f32, LayerId)>,
}

impl LayerStack {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_layer(&mut self, layer: GeologicalLayer) {
        let id = layer.id;
        let top_depth = layer.top_depth;
        self.layers.insert(id, layer);
        self.depth_index.push((top_depth, id));
        self.depth_index.sort_by(|a, b| a.0.total_cmp(&b.0));
    }

    #[must_use]
    pub fn get(&self, id: LayerId) -> Option<&GeologicalLayer> {
        self.layers.get(&id)
    }

    pub fn get_mut(&mut self, id: LayerId) -> Option<&mut GeologicalLayer> {
        self.layers.get_mut(&id)
    }

    #[must_use]
    pub fn layer_at_depth(&self, depth: f32) -> Option<&GeologicalLayer> {
        for (_, layer_id) in self.depth_index.iter().rev() {
            if let Some(layer) = self.layers.get(layer_id)
                && layer.contains_depth(depth)
            {
                return Some(layer);
            }
        }
        None
    }

    pub fn layers(&self) -> impl Iterator<Item = &GeologicalLayer> {
        self.depth_index
            .iter()
            .filter_map(|(_, id)| self.layers.get(id))
    }

    #[must_use]
    pub fn count(&self) -> usize {
        self.layers.len()
    }

    #[must_use]
    pub fn max_depth(&self) -> f32 {
        self.layers
            .values()
            .map(|l| l.bottom_depth)
            .max_by(f32::total_cmp)
            .unwrap_or(0.0)
    }

    #[must_use]
    pub fn fingerprint(&self) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&(self.layers.len() as u64).to_le_bytes());
        for layer in self.layers.values() {
            hasher.update(&layer.fingerprint().to_le_bytes());
        }
        hasher.finalize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_stratum() -> Stratum {
        Stratum::new(MaterialId::new(1), RockType::Igneous, 10.0)
    }

    fn test_layer() -> GeologicalLayer {
        GeologicalLayer::new(LayerId::new(1), "Test Layer", 0.0, 100.0)
            .with_stratum(Stratum::new(
                MaterialId::new(1),
                RockType::Sedimentary,
                30.0,
            ))
            .with_stratum(Stratum::new(MaterialId::new(2), RockType::Igneous, 70.0))
    }

    #[test]
    fn layer_boundary_blend() {
        let boundary = LayerBoundary::new(50.0).with_transition(10.0);

        assert!((boundary.blend_factor(50.0) - 0.0).abs() < f32::EPSILON);
        assert!((boundary.blend_factor(55.0) - 0.5).abs() < f32::EPSILON);
        assert!((boundary.blend_factor(60.0) - 1.0).abs() < f32::EPSILON);
        assert!((boundary.blend_factor(40.0) - 0.0).abs() < f32::EPSILON);
        assert!((boundary.blend_factor(70.0) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn stratum_properties() {
        let stratum = test_stratum().with_porosity(0.2).with_permeability(0.05);

        assert!(stratum.density() < RockType::Igneous.base_density());
        assert!(stratum.thermal_conductivity() > 0.0);
        assert!(stratum.effective_strength(10.0) > stratum.strength);
    }

    #[test]
    fn layer_contains_depth() {
        let layer = test_layer();

        assert!(layer.contains_depth(0.0));
        assert!(layer.contains_depth(50.0));
        assert!(layer.contains_depth(99.9));
        assert!(!layer.contains_depth(100.0));
        assert!(!layer.contains_depth(-1.0));
    }

    #[test]
    fn layer_stratum_at_depth() {
        let layer = test_layer();

        let s0 = layer.stratum_at_depth(10.0).unwrap();
        assert_eq!(s0.rock_type, RockType::Sedimentary);

        let s1 = layer.stratum_at_depth(50.0).unwrap();
        assert_eq!(s1.rock_type, RockType::Igneous);

        assert!(layer.stratum_at_depth(150.0).is_none());
    }

    #[test]
    fn layer_average_properties() {
        let layer = test_layer();

        let density = layer.average_density();
        assert!(density > 0.0);

        let perm = layer.average_permeability();
        assert!(perm > 0.0);

        let strength = layer.min_strength();
        assert!(strength > 0.0);
    }

    #[test]
    fn layer_temperature_pressure() {
        let layer = GeologicalLayer::new(LayerId::new(1), "Deep", 100.0, 200.0)
            .with_base_temperature(50.0)
            .with_base_pressure(10.0);

        let temp = layer.temperature_at_depth(150.0, 0.03);
        assert!(temp > 50.0);

        let pressure = layer.pressure_at_depth(150.0, 0.1);
        assert!(pressure > 10.0);
    }

    #[test]
    fn layer_fingerprint_deterministic() {
        let layer = test_layer();
        let fp1 = layer.fingerprint();
        let fp2 = layer.fingerprint();
        assert_eq!(fp1, fp2);
    }

    #[test]
    fn layer_stack_operations() {
        let mut stack = LayerStack::new();

        stack.add_layer(GeologicalLayer::new(LayerId::new(1), "Surface", 0.0, 50.0));
        stack.add_layer(GeologicalLayer::new(LayerId::new(2), "Middle", 50.0, 150.0));
        stack.add_layer(GeologicalLayer::new(LayerId::new(3), "Deep", 150.0, 300.0));

        assert_eq!(stack.count(), 3);
        assert!((stack.max_depth() - 300.0).abs() < f32::EPSILON);

        let surface = stack.layer_at_depth(25.0).unwrap();
        assert_eq!(surface.id, LayerId::new(1));

        let middle = stack.layer_at_depth(100.0).unwrap();
        assert_eq!(middle.id, LayerId::new(2));

        let deep = stack.layer_at_depth(200.0).unwrap();
        assert_eq!(deep.id, LayerId::new(3));

        assert!(stack.layer_at_depth(500.0).is_none());
    }

    #[test]
    fn layer_stack_fingerprint() {
        let mut stack1 = LayerStack::new();
        let mut stack2 = LayerStack::new();

        stack1.add_layer(test_layer());
        stack2.add_layer(test_layer());

        assert_eq!(stack1.fingerprint(), stack2.fingerprint());
    }

    #[test]
    fn serde_layer_boundary() {
        let boundary = LayerBoundary::new(50.0)
            .with_transition(5.0)
            .unconformable();
        let json = serde_json::to_string(&boundary).unwrap();
        let recovered: LayerBoundary = serde_json::from_str(&json).unwrap();
        assert_eq!(boundary, recovered);
    }

    #[test]
    fn serde_stratum() {
        let stratum = test_stratum().with_porosity(0.15);
        let json = serde_json::to_string(&stratum).unwrap();
        let recovered: Stratum = serde_json::from_str(&json).unwrap();
        assert_eq!(stratum, recovered);
    }

    #[test]
    fn serde_geological_layer() {
        let layer = test_layer();
        let json = serde_json::to_string(&layer).unwrap();
        let recovered: GeologicalLayer = serde_json::from_str(&json).unwrap();
        assert_eq!(layer, recovered);
    }

    #[test]
    fn serde_layer_stack() {
        let mut stack = LayerStack::new();
        stack.add_layer(test_layer());
        stack.add_layer(GeologicalLayer::new(
            LayerId::new(2),
            "Second",
            100.0,
            200.0,
        ));

        let json = serde_json::to_string(&stack).unwrap();
        let recovered: LayerStack = serde_json::from_str(&json).unwrap();
        assert_eq!(stack, recovered);
    }
}
