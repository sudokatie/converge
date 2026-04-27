//! Material property database for environmental simulation.
//!
//! Provides physical properties for blocks/materials used by hazard propagation,
//! thermal simulation, and structural systems.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::chunk::BlockId;

/// Unique material identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MaterialId(pub u16);

impl MaterialId {
    /// Get the raw ID value.
    #[must_use]
    pub const fn raw(self) -> u16 {
        self.0
    }
}

/// Physical properties of a material.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MaterialProperties {
    /// Display name for the material.
    pub name: String,

    /// Resistance to burning (0.0 = highly flammable, 1.0 = fireproof).
    burn_resistance: f32,

    /// Thermal conductivity (0.0 = insulator, 1.0 = perfect conductor).
    thermal_conductivity: f32,

    /// Airtightness (0.0 = porous, 1.0 = airtight).
    airtightness: f32,

    /// Buoyancy factor (0.0 = sinks, 0.5 = neutral, 1.0 = floats).
    buoyancy: f32,

    /// Brittleness (0.0 = ductile, 1.0 = shatters easily).
    brittleness: f32,
}

impl MaterialProperties {
    /// Create new material properties with validation.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        burn_resistance: f32,
        thermal_conductivity: f32,
        airtightness: f32,
        buoyancy: f32,
        brittleness: f32,
    ) -> Self {
        Self {
            name: name.into(),
            burn_resistance: burn_resistance.clamp(0.0, 1.0),
            thermal_conductivity: thermal_conductivity.clamp(0.0, 1.0),
            airtightness: airtightness.clamp(0.0, 1.0),
            buoyancy: buoyancy.clamp(0.0, 1.0),
            brittleness: brittleness.clamp(0.0, 1.0),
        }
    }

    /// Get burn resistance (0.0 = highly flammable, 1.0 = fireproof).
    #[must_use]
    pub const fn burn_resistance(&self) -> f32 {
        self.burn_resistance
    }

    /// Get thermal conductivity (0.0 = insulator, 1.0 = perfect conductor).
    #[must_use]
    pub const fn thermal_conductivity(&self) -> f32 {
        self.thermal_conductivity
    }

    /// Get airtightness (0.0 = porous, 1.0 = airtight).
    #[must_use]
    pub const fn airtightness(&self) -> f32 {
        self.airtightness
    }

    /// Get buoyancy factor (0.0 = sinks, 0.5 = neutral, 1.0 = floats).
    #[must_use]
    pub const fn buoyancy(&self) -> f32 {
        self.buoyancy
    }

    /// Get brittleness (0.0 = ductile, 1.0 = shatters easily).
    #[must_use]
    pub const fn brittleness(&self) -> f32 {
        self.brittleness
    }

    /// Check if this material is flammable (burn resistance < 0.5).
    #[must_use]
    pub fn is_flammable(&self) -> bool {
        self.burn_resistance < 0.5
    }

    /// Check if this material is a thermal insulator (conductivity < 0.3).
    #[must_use]
    pub fn is_insulator(&self) -> bool {
        self.thermal_conductivity < 0.3
    }

    /// Check if this material is airtight (airtightness >= 0.9).
    #[must_use]
    pub fn is_airtight(&self) -> bool {
        self.airtightness >= 0.9
    }

    /// Check if this material floats in water (buoyancy > 0.5).
    #[must_use]
    pub fn floats(&self) -> bool {
        self.buoyancy > 0.5
    }

    /// Check if this material is brittle (brittleness >= 0.7).
    #[must_use]
    pub fn is_brittle(&self) -> bool {
        self.brittleness >= 0.7
    }
}

/// Predefined material categories with typical property values.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MaterialCategory {
    /// Air or vacuum - no material.
    Air,
    /// Stone, rock, minerals.
    Stone,
    /// Soil, dirt, clay.
    Earth,
    /// Wood, plant matter.
    Organic,
    /// Sand, gravel, loose materials.
    Granular,
    /// Water, liquids.
    Liquid,
    /// Metal ores and refined metals.
    Metal,
    /// Glass, crystal, ice.
    Glass,
}

impl MaterialCategory {
    /// Get default material properties for this category.
    #[must_use]
    pub fn default_properties(self) -> MaterialProperties {
        match self {
            MaterialCategory::Air => MaterialProperties::new(
                "Air", 1.0, // fireproof
                0.1, // poor conductor
                0.0, // not airtight
                0.5, // neutral
                0.0, // not brittle
            ),
            MaterialCategory::Stone => MaterialProperties::new(
                "Stone", 1.0,  // fireproof
                0.6,  // moderate conductor
                0.95, // mostly airtight
                0.1,  // sinks
                0.4,  // somewhat brittle
            ),
            MaterialCategory::Earth => MaterialProperties::new(
                "Earth", 0.8, // mostly fire resistant
                0.3, // poor conductor
                0.6, // somewhat porous
                0.2, // sinks
                0.1, // not brittle
            ),
            MaterialCategory::Organic => MaterialProperties::new(
                "Organic", 0.2, // flammable
                0.2, // poor conductor
                0.3, // porous
                0.6, // can float
                0.2, // not brittle
            ),
            MaterialCategory::Granular => MaterialProperties::new(
                "Granular", 0.9,  // mostly fire resistant
                0.4,  // moderate conductor
                0.2,  // very porous
                0.15, // sinks
                0.05, // not brittle (flows)
            ),
            MaterialCategory::Liquid => MaterialProperties::new(
                "Liquid", 1.0, // fireproof
                0.5, // moderate conductor
                0.0, // not airtight (flows)
                0.5, // neutral
                0.0, // not brittle
            ),
            MaterialCategory::Metal => MaterialProperties::new(
                "Metal", 1.0,  // fireproof
                0.9,  // excellent conductor
                1.0,  // airtight
                0.05, // sinks
                0.2,  // not brittle
            ),
            MaterialCategory::Glass => MaterialProperties::new(
                "Glass", 1.0, // fireproof
                0.7, // good conductor
                1.0, // airtight
                0.3, // sinks
                0.9, // very brittle
            ),
        }
    }
}

/// Error type for material registry operations.
#[derive(Debug, Error)]
pub enum MaterialRegistryError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Parse error: {0}")]
    Parse(#[from] ron::de::SpannedError),
    #[error("Unknown material ID: {0}")]
    UnknownMaterial(u16),
    #[error("Material already registered: {0}")]
    AlreadyRegistered(u16),
}

/// Registry mapping blocks to material properties.
#[derive(Debug, Default)]
pub struct MaterialRegistry {
    /// Material ID to properties.
    materials: HashMap<MaterialId, MaterialProperties>,
    /// Block ID to material ID mapping.
    block_materials: HashMap<BlockId, MaterialId>,
}

impl MaterialRegistry {
    /// Create an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            materials: HashMap::new(),
            block_materials: HashMap::new(),
        }
    }

    /// Create a registry with default materials for built-in blocks.
    #[must_use]
    pub fn with_defaults() -> Self {
        use crate::chunk::{AIR, DIRT, GRASS, SAND, STONE, WATER};

        let mut registry = Self::new();

        // Material 0: Air
        let air_mat = MaterialId(0);
        registry.register(air_mat, MaterialCategory::Air.default_properties());
        registry.bind_block(AIR, air_mat);

        // Material 1: Stone
        let stone_mat = MaterialId(1);
        registry.register(stone_mat, MaterialCategory::Stone.default_properties());
        registry.bind_block(STONE, stone_mat);

        // Material 2: Earth (for dirt)
        let earth_mat = MaterialId(2);
        registry.register(earth_mat, MaterialCategory::Earth.default_properties());
        registry.bind_block(DIRT, earth_mat);

        // Material 3: Organic (for grass - uses grass-top as primary)
        let grass_mat = MaterialId(3);
        registry.register(
            grass_mat,
            MaterialProperties::new(
                "Grass", 0.3, // flammable surface
                0.25, 0.5, 0.4, 0.15,
            ),
        );
        registry.bind_block(GRASS, grass_mat);

        // Material 4: Granular (for sand)
        let sand_mat = MaterialId(4);
        registry.register(sand_mat, MaterialCategory::Granular.default_properties());
        registry.bind_block(SAND, sand_mat);

        // Material 5: Liquid (for water)
        let water_mat = MaterialId(5);
        registry.register(water_mat, MaterialCategory::Liquid.default_properties());
        registry.bind_block(WATER, water_mat);

        registry
    }

    /// Load material definitions from a RON file.
    ///
    /// # Errors
    /// Returns an error if the file cannot be read or parsed.
    pub fn load(path: &Path) -> Result<Self, MaterialRegistryError> {
        let contents = fs::read_to_string(path)?;

        #[derive(Deserialize)]
        struct MaterialFile {
            materials: HashMap<u16, MaterialProperties>,
            block_bindings: HashMap<u16, u16>,
        }

        let file: MaterialFile = ron::from_str(&contents)?;

        let mut registry = Self::new();
        for (id, props) in file.materials {
            registry.materials.insert(MaterialId(id), props);
        }
        for (block_id, mat_id) in file.block_bindings {
            registry
                .block_materials
                .insert(BlockId(block_id), MaterialId(mat_id));
        }

        Ok(registry)
    }

    /// Register a new material.
    pub fn register(&mut self, id: MaterialId, properties: MaterialProperties) {
        self.materials.insert(id, properties);
    }

    /// Bind a block ID to a material ID.
    pub fn bind_block(&mut self, block_id: BlockId, material_id: MaterialId) {
        self.block_materials.insert(block_id, material_id);
    }

    /// Get material properties by material ID.
    #[must_use]
    pub fn get(&self, id: MaterialId) -> Option<&MaterialProperties> {
        self.materials.get(&id)
    }

    /// Get material ID for a block.
    #[must_use]
    pub fn block_material(&self, block_id: BlockId) -> Option<MaterialId> {
        self.block_materials.get(&block_id).copied()
    }

    /// Get material properties for a block.
    #[must_use]
    pub fn block_properties(&self, block_id: BlockId) -> Option<&MaterialProperties> {
        self.block_material(block_id)
            .and_then(|mat_id| self.get(mat_id))
    }

    /// Get burn resistance for a block (returns 1.0 for unknown blocks).
    #[must_use]
    pub fn burn_resistance(&self, block_id: BlockId) -> f32 {
        self.block_properties(block_id)
            .map_or(1.0, |p| p.burn_resistance())
    }

    /// Get thermal conductivity for a block (returns 0.5 for unknown blocks).
    #[must_use]
    pub fn thermal_conductivity(&self, block_id: BlockId) -> f32 {
        self.block_properties(block_id)
            .map_or(0.5, |p| p.thermal_conductivity())
    }

    /// Get airtightness for a block (returns 0.0 for unknown blocks).
    #[must_use]
    pub fn airtightness(&self, block_id: BlockId) -> f32 {
        self.block_properties(block_id)
            .map_or(0.0, |p| p.airtightness())
    }

    /// Get buoyancy for a block (returns 0.5 for unknown blocks).
    #[must_use]
    pub fn buoyancy(&self, block_id: BlockId) -> f32 {
        self.block_properties(block_id)
            .map_or(0.5, |p| p.buoyancy())
    }

    /// Get brittleness for a block (returns 0.0 for unknown blocks).
    #[must_use]
    pub fn brittleness(&self, block_id: BlockId) -> f32 {
        self.block_properties(block_id)
            .map_or(0.0, |p| p.brittleness())
    }

    /// Check if a block is flammable.
    #[must_use]
    pub fn is_flammable(&self, block_id: BlockId) -> bool {
        self.block_properties(block_id)
            .map_or(false, MaterialProperties::is_flammable)
    }

    /// Check if a block is airtight.
    #[must_use]
    pub fn is_airtight(&self, block_id: BlockId) -> bool {
        self.block_properties(block_id)
            .map_or(false, MaterialProperties::is_airtight)
    }

    /// Get the number of registered materials.
    #[must_use]
    pub fn material_count(&self) -> usize {
        self.materials.len()
    }

    /// Get the number of block bindings.
    #[must_use]
    pub fn binding_count(&self) -> usize {
        self.block_materials.len()
    }

    /// Iterate over all materials.
    pub fn iter(&self) -> impl Iterator<Item = (MaterialId, &MaterialProperties)> {
        self.materials.iter().map(|(&id, props)| (id, props))
    }
}

/// Convert material properties to hazard resistance for propagation.
pub mod hazard_integration {
    use super::*;
    use crate::environment::Resistance;

    /// Get fire spread resistance from material properties.
    #[must_use]
    pub fn fire_resistance(props: &MaterialProperties) -> Resistance {
        Resistance::new(props.burn_resistance())
    }

    /// Get frost spread resistance from material properties (based on thermal conductivity).
    #[must_use]
    pub fn frost_resistance(props: &MaterialProperties) -> Resistance {
        // High conductivity = low resistance to frost spread
        Resistance::new(1.0 - props.thermal_conductivity())
    }

    /// Get vacuum spread resistance from material properties (based on airtightness).
    #[must_use]
    pub fn vacuum_resistance(props: &MaterialProperties) -> Resistance {
        Resistance::new(props.airtightness())
    }

    /// Get flood spread resistance from material properties.
    #[must_use]
    pub fn flood_resistance(props: &MaterialProperties) -> Resistance {
        // Airtight materials also block water
        Resistance::new(props.airtightness())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::{AIR, DIRT, GRASS, SAND, STONE, WATER};
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn material_properties_clamping() {
        let props = MaterialProperties::new("Test", -0.5, 1.5, 0.5, -1.0, 2.0);
        assert_eq!(props.burn_resistance(), 0.0);
        assert_eq!(props.thermal_conductivity(), 1.0);
        assert_eq!(props.airtightness(), 0.5);
        assert_eq!(props.buoyancy(), 0.0);
        assert_eq!(props.brittleness(), 1.0);
    }

    #[test]
    fn material_properties_getters() {
        let props = MaterialProperties::new("Test", 0.3, 0.4, 0.5, 0.6, 0.7);
        assert_eq!(props.burn_resistance(), 0.3);
        assert_eq!(props.thermal_conductivity(), 0.4);
        assert_eq!(props.airtightness(), 0.5);
        assert_eq!(props.buoyancy(), 0.6);
        assert_eq!(props.brittleness(), 0.7);
    }

    #[test]
    fn material_properties_predicates() {
        // Flammable: burn_resistance < 0.5
        assert!(MaterialProperties::new("Wood", 0.2, 0.5, 0.5, 0.5, 0.5).is_flammable());
        assert!(!MaterialProperties::new("Stone", 0.8, 0.5, 0.5, 0.5, 0.5).is_flammable());

        // Insulator: thermal_conductivity < 0.3
        assert!(MaterialProperties::new("Foam", 0.5, 0.1, 0.5, 0.5, 0.5).is_insulator());
        assert!(!MaterialProperties::new("Metal", 0.5, 0.9, 0.5, 0.5, 0.5).is_insulator());

        // Airtight: airtightness >= 0.9
        assert!(MaterialProperties::new("Metal", 0.5, 0.5, 0.95, 0.5, 0.5).is_airtight());
        assert!(!MaterialProperties::new("Mesh", 0.5, 0.5, 0.5, 0.5, 0.5).is_airtight());

        // Floats: buoyancy > 0.5
        assert!(MaterialProperties::new("Wood", 0.5, 0.5, 0.5, 0.7, 0.5).floats());
        assert!(!MaterialProperties::new("Stone", 0.5, 0.5, 0.5, 0.3, 0.5).floats());

        // Brittle: brittleness >= 0.7
        assert!(MaterialProperties::new("Glass", 0.5, 0.5, 0.5, 0.5, 0.9).is_brittle());
        assert!(!MaterialProperties::new("Metal", 0.5, 0.5, 0.5, 0.5, 0.3).is_brittle());
    }

    #[test]
    fn material_category_defaults() {
        let stone = MaterialCategory::Stone.default_properties();
        assert_eq!(stone.burn_resistance(), 1.0);
        assert!(!stone.is_flammable());

        let organic = MaterialCategory::Organic.default_properties();
        assert!(organic.is_flammable());
        assert!(organic.floats());

        let glass = MaterialCategory::Glass.default_properties();
        assert!(glass.is_brittle());
        assert!(glass.is_airtight());

        let metal = MaterialCategory::Metal.default_properties();
        assert!(metal.is_airtight());
        assert!(!metal.is_brittle());
    }

    #[test]
    fn registry_with_defaults() {
        let registry = MaterialRegistry::with_defaults();
        assert_eq!(registry.material_count(), 6);
        assert_eq!(registry.binding_count(), 6);

        // Check block bindings
        assert!(registry.block_material(AIR).is_some());
        assert!(registry.block_material(STONE).is_some());
        assert!(registry.block_material(DIRT).is_some());
        assert!(registry.block_material(GRASS).is_some());
        assert!(registry.block_material(SAND).is_some());
        assert!(registry.block_material(WATER).is_some());
    }

    #[test]
    fn registry_block_properties() {
        let registry = MaterialRegistry::with_defaults();

        // Stone should be fireproof
        assert_eq!(registry.burn_resistance(STONE), 1.0);
        assert!(!registry.is_flammable(STONE));

        // Water should be fireproof
        assert_eq!(registry.burn_resistance(WATER), 1.0);

        // Grass should be flammable
        assert!(registry.is_flammable(GRASS));

        // Stone should be mostly airtight
        assert!(registry.airtightness(STONE) > 0.9);
    }

    #[test]
    fn registry_unknown_block_defaults() {
        let registry = MaterialRegistry::with_defaults();
        let unknown = BlockId(999);

        // Unknown blocks should return safe defaults
        assert_eq!(registry.burn_resistance(unknown), 1.0);
        assert_eq!(registry.thermal_conductivity(unknown), 0.5);
        assert_eq!(registry.airtightness(unknown), 0.0);
        assert_eq!(registry.buoyancy(unknown), 0.5);
        assert_eq!(registry.brittleness(unknown), 0.0);
        assert!(!registry.is_flammable(unknown));
        assert!(!registry.is_airtight(unknown));
    }

    #[test]
    fn registry_register_and_bind() {
        let mut registry = MaterialRegistry::new();

        let mat_id = MaterialId(100);
        let block_id = BlockId(50);

        registry.register(
            mat_id,
            MaterialProperties::new("Custom", 0.1, 0.2, 0.3, 0.4, 0.5),
        );
        registry.bind_block(block_id, mat_id);

        assert_eq!(registry.material_count(), 1);
        assert_eq!(registry.binding_count(), 1);
        assert_eq!(registry.block_material(block_id), Some(mat_id));
        assert_eq!(registry.burn_resistance(block_id), 0.1);
    }

    #[test]
    fn registry_iterate_materials() {
        let registry = MaterialRegistry::with_defaults();
        let all: Vec<_> = registry.iter().collect();
        assert_eq!(all.len(), 6);
    }

    #[test]
    fn load_from_ron() {
        let ron_content = r#"(
            materials: {
                0: (
                    name: "TestAir",
                    burn_resistance: 1.0,
                    thermal_conductivity: 0.1,
                    airtightness: 0.0,
                    buoyancy: 0.5,
                    brittleness: 0.0,
                ),
                1: (
                    name: "TestStone",
                    burn_resistance: 1.0,
                    thermal_conductivity: 0.6,
                    airtightness: 0.95,
                    buoyancy: 0.1,
                    brittleness: 0.4,
                ),
            },
            block_bindings: {
                0: 0,
                1: 1,
            },
        )"#;

        let mut temp = NamedTempFile::new().unwrap();
        temp.write_all(ron_content.as_bytes()).unwrap();

        let registry = MaterialRegistry::load(temp.path()).unwrap();
        assert_eq!(registry.material_count(), 2);
        assert_eq!(registry.binding_count(), 2);

        let air_props = registry.get(MaterialId(0)).unwrap();
        assert_eq!(air_props.name, "TestAir");

        let stone_props = registry.get(MaterialId(1)).unwrap();
        assert_eq!(stone_props.name, "TestStone");
    }

    #[test]
    fn serde_round_trip() {
        let props = MaterialProperties::new("Test", 0.3, 0.4, 0.5, 0.6, 0.7);
        let json = serde_json::to_string(&props).unwrap();
        let recovered: MaterialProperties = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, props);
    }

    #[test]
    fn material_id_serde() {
        let id = MaterialId(42);
        let json = serde_json::to_string(&id).unwrap();
        let recovered: MaterialId = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered, id);
    }

    // Hazard integration tests
    mod hazard_integration_tests {
        use super::*;
        use crate::environment::materials::hazard_integration::*;

        #[test]
        fn fire_resistance_from_burn_resistance() {
            let flammable = MaterialProperties::new("Wood", 0.2, 0.5, 0.5, 0.5, 0.5);
            let fireproof = MaterialProperties::new("Stone", 1.0, 0.5, 0.5, 0.5, 0.5);

            let r1 = fire_resistance(&flammable);
            let r2 = fire_resistance(&fireproof);

            assert!((r1.factor() - 0.2).abs() < 0.001);
            assert!((r2.factor() - 1.0).abs() < 0.001);
            assert!(!r1.blocks());
            assert!(r2.blocks());
        }

        #[test]
        fn frost_resistance_from_conductivity() {
            let conductor = MaterialProperties::new("Metal", 0.5, 0.9, 0.5, 0.5, 0.5);
            let insulator = MaterialProperties::new("Wood", 0.5, 0.1, 0.5, 0.5, 0.5);

            let r1 = frost_resistance(&conductor);
            let r2 = frost_resistance(&insulator);

            // High conductivity = low resistance to frost
            assert!((r1.factor() - 0.1).abs() < 0.001);
            // Low conductivity = high resistance to frost
            assert!((r2.factor() - 0.9).abs() < 0.001);
        }

        #[test]
        fn vacuum_resistance_from_airtightness() {
            let airtight = MaterialProperties::new("Metal", 0.5, 0.5, 1.0, 0.5, 0.5);
            let porous = MaterialProperties::new("Mesh", 0.5, 0.5, 0.2, 0.5, 0.5);

            let r1 = vacuum_resistance(&airtight);
            let r2 = vacuum_resistance(&porous);

            assert!(r1.blocks());
            assert!(!r2.blocks());
            assert!((r2.factor() - 0.2).abs() < 0.001);
        }

        #[test]
        fn flood_resistance_from_airtightness() {
            let solid = MaterialProperties::new("Stone", 0.5, 0.5, 0.95, 0.5, 0.5);
            let porous = MaterialProperties::new("Gravel", 0.5, 0.5, 0.2, 0.5, 0.5);

            let r1 = flood_resistance(&solid);
            let r2 = flood_resistance(&porous);

            assert!((r1.factor() - 0.95).abs() < 0.001);
            assert!((r2.factor() - 0.2).abs() < 0.001);
        }
    }

    #[test]
    fn thermal_conductivity_values() {
        let registry = MaterialRegistry::with_defaults();

        // Air should be a poor conductor
        assert!(registry.thermal_conductivity(AIR) < 0.3);

        // Stone should be a moderate conductor
        let stone_cond = registry.thermal_conductivity(STONE);
        assert!(stone_cond > 0.4 && stone_cond < 0.8);
    }

    #[test]
    fn buoyancy_values() {
        let registry = MaterialRegistry::with_defaults();

        // Stone should sink
        assert!(registry.buoyancy(STONE) < 0.3);

        // Water is neutral
        assert!((registry.buoyancy(WATER) - 0.5).abs() < 0.1);
    }

    #[test]
    fn brittleness_values() {
        let registry = MaterialRegistry::with_defaults();

        // Stone has some brittleness
        assert!(registry.brittleness(STONE) > 0.0);

        // Dirt is not brittle
        assert!(registry.brittleness(DIRT) < 0.3);
    }
}
