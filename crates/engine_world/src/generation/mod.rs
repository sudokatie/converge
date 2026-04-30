//! World generation systems.

mod biome;
mod biome_pipeline;
mod caves;
mod noise;
pub mod structure_grammar;
mod structures;
mod terrain;
pub mod topology;

pub use biome::{Biome, BiomeSelector};
pub use biome_pipeline::{
    BiomeInfluence, BiomePipeline, BiomePipelineConfig, BiomePipelineHook, FeaturePlacement,
    HookConfig, HookPriority, PipelineContext, ResourceDeposit, TerrainLayers,
};
pub use caves::CaveCarver;
pub use noise::TerrainNoise;
pub use structures::{Structure, StructureBlock, should_place_tree, structure_random};
pub use terrain::TerrainGenerator;
pub use topology::{
    CellQuery, CellState, ConfigError as TopologyConfigError,
    FingerprintBuilder as TopologyFingerprintBuilder, HazardType, MissionHook, NodeId, NodeRole,
    PathQuery, PlannerSummary, QueryResult, ResourceType, SegmentId, SegmentKind,
    TopologyAnnotation, TopologyAnnotations, TopologyCell, TopologyChecksum, TopologyConfig,
    TopologyFingerprint, TopologyKind, TopologyNode, TopologyPlanner, TopologySegment,
};
