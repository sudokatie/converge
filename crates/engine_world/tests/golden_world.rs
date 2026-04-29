//! Golden-world regression tests for terrain, hazards, and persistence invariants.
//!
//! These tests verify deterministic behavior across runs with stable fingerprints.
//! If a test fails, it indicates a simulation behavior change that should be
//! reviewed for intentionality before updating the expected values.
//!
//! # Test Categories
//!
//! - **Terrain**: Deterministic terrain generation with height/block fingerprints
//! - **Hazard**: Hazard simulation progression stability
//! - **Persistence**: Round-trip serialization and mutation journal invariants
//! - **Scenario**: Combined multi-system golden cases

use std::collections::HashMap;

use engine_core::coords::{ChunkPos, LocalPos, WorldPos};
use engine_world::chunk::{AIR, BlockId, Chunk, DIRT, GRASS, STONE};
use engine_world::generation::TerrainGenerator;
use engine_world::persistence::{
    ChunkDelta, MutationJournal, MutationReason, MutationSource, Region,
};
use engine_world::replay::{ChecksumBuilder, StepChecksum};
use engine_world::sandbox::{SandboxConfig, ScenarioSandbox, SpawnCommand};
use engine_world::{HazardKind, HazardSimulator};
use glam::IVec2;

mod terrain {
    use super::*;

    const GOLDEN_SEED: u64 = 0xDEAD_BEEF_CAFE;

    fn terrain_checksum(generator: &TerrainGenerator, positions: &[ChunkPos]) -> StepChecksum {
        let mut builder = ChecksumBuilder::new();
        for &pos in positions {
            let chunk = generator.generate(pos);
            builder.feed_i32(pos.x());
            builder.feed_i32(pos.y());
            builder.feed_i32(pos.z());
            builder.feed_u32(chunk.non_air_count());
            #[expect(clippy::cast_possible_truncation, reason = "test data fits in u32")]
            for (local, block) in chunk.iter_non_air() {
                builder.feed_u32(local.to_index() as u32);
                builder.feed_u32(u32::from(block.raw()));
            }
        }
        builder.build()
    }

    #[test]
    fn deterministic_terrain_same_seed() {
        let generator1 = TerrainGenerator::new(GOLDEN_SEED);
        let generator2 = TerrainGenerator::new(GOLDEN_SEED);

        let positions = [
            ChunkPos::new(0, 4, 0),
            ChunkPos::new(5, 4, 5),
            ChunkPos::new(-10, 3, 7),
        ];

        for &pos in &positions {
            let chunk1 = generator1.generate(pos);
            let chunk2 = generator2.generate(pos);
            assert_eq!(
                chunk1.non_air_count(),
                chunk2.non_air_count(),
                "chunk at {pos:?} differs"
            );
            for (p1, b1) in chunk1.iter() {
                assert_eq!(b1, chunk2.get(p1), "block mismatch at {pos:?}/{p1:?}");
            }
        }
    }

    #[test]
    fn terrain_golden_fingerprint() {
        let generator = TerrainGenerator::new(GOLDEN_SEED);
        let positions = [
            ChunkPos::new(0, 4, 0),
            ChunkPos::new(1, 4, 0),
            ChunkPos::new(0, 4, 1),
            ChunkPos::new(-1, 4, -1),
        ];

        let checksum = terrain_checksum(&generator, &positions);
        assert_eq!(
            checksum.value(),
            0x39BE_9DD4,
            "terrain fingerprint changed - review generation algorithm"
        );
    }

    #[test]
    fn height_map_stability() {
        let generator = TerrainGenerator::new(42);

        let expected_heights = [
            ((0, 0), 80),
            ((100, 100), 80),
            ((-50, 75), 79),
            ((1000, -1000), 80),
        ];

        for ((x, z), expected) in expected_heights {
            let h = generator.height_at(x, z);
            assert_eq!(
                h, expected,
                "height at ({x}, {z}) changed from {expected} to {h}"
            );
        }
    }

    #[test]
    fn surface_layer_structure() {
        let generator = TerrainGenerator::new(GOLDEN_SEED);
        let surface_chunk = generator.generate(ChunkPos::new(0, 5, 0));

        let mut grass_count = 0;
        let mut dirt_count = 0;
        let mut stone_count = 0;

        for (_, block) in surface_chunk.iter_non_air() {
            match block {
                GRASS => grass_count += 1,
                DIRT => dirt_count += 1,
                STONE => stone_count += 1,
                _ => {}
            }
        }

        assert!(grass_count > 0, "surface should have grass");
        assert!(
            dirt_count >= grass_count,
            "should have more dirt than grass"
        );
        assert!(stone_count > 0, "surface should have some stone");
        assert_eq!(
            grass_count + dirt_count + stone_count,
            surface_chunk.non_air_count(),
            "all non-air blocks should be grass, dirt, or stone"
        );
    }

    #[test]
    fn underground_chunk_solid() {
        let generator = TerrainGenerator::new(GOLDEN_SEED);
        let deep_chunk = generator.generate(ChunkPos::new(0, 0, 0));

        assert!(
            deep_chunk.non_air_count() > 4000,
            "deep underground should be mostly solid, got {}",
            deep_chunk.non_air_count()
        );

        let all_stone = deep_chunk.iter_non_air().all(|(_, b)| b == STONE);
        assert!(all_stone, "deep underground should be all stone");
    }

    #[test]
    fn sky_chunk_empty() {
        let generator = TerrainGenerator::new(GOLDEN_SEED);
        let sky_chunk = generator.generate(ChunkPos::new(0, 20, 0));

        assert!(sky_chunk.is_empty(), "high sky should be empty");
    }
}

mod hazard {
    use super::*;

    const GOLDEN_SEED: u64 = 0xCAFE_BABE;

    #[test]
    fn fire_spread_deterministic() {
        let run_simulation = |seed: u64| -> StepChecksum {
            let mut sandbox = ScenarioSandbox::new(seed);
            sandbox.execute(SpawnCommand::hazard(
                WorldPos::new(8, 8, 8),
                HazardKind::Fire,
                1.0,
            ));

            for _ in 0..20 {
                sandbox.step(0.1);
            }

            sandbox.snapshot().checksum
        };

        let cs1 = run_simulation(GOLDEN_SEED);
        let cs2 = run_simulation(GOLDEN_SEED);
        assert_eq!(cs1, cs2, "simulation should be deterministic");
    }

    #[test]
    fn fire_progression_golden() {
        let mut sandbox = ScenarioSandbox::new(GOLDEN_SEED);
        sandbox.execute(SpawnCommand::hazard(
            WorldPos::new(8, 8, 8),
            HazardKind::Fire,
            1.0,
        ));

        let mut checksums = Vec::new();
        for _ in 0..10 {
            let result = sandbox.step(0.1);
            checksums.push(result.overall_checksum.value());
        }

        let expected = [
            0x07B8_C394,
            0x5FA6_6ABC,
            0xDE83_0F9B,
            0xEF9B_38EC,
            0x6EBE_5DCB,
            0x36A0_F4E3,
            0xB785_91C4,
            0x5490_9A0D,
            0xD5B5_FF2A,
            0x8DAB_5602,
        ];

        assert_eq!(
            checksums, expected,
            "fire progression fingerprint changed - review hazard simulation"
        );
    }

    #[test]
    fn multi_hazard_interaction() {
        let mut sandbox = ScenarioSandbox::new(GOLDEN_SEED);

        sandbox.execute(SpawnCommand::hazard(
            WorldPos::new(4, 8, 8),
            HazardKind::Fire,
            0.8,
        ));
        sandbox.execute(SpawnCommand::hazard(
            WorldPos::new(12, 8, 8),
            HazardKind::Frost,
            0.8,
        ));

        for _ in 0..15 {
            sandbox.step(0.1);
        }

        let state = sandbox.state();
        assert!(
            state.hazard_count(HazardKind::Fire) > 0,
            "fire should persist"
        );
        assert!(
            state.hazard_count(HazardKind::Frost) > 0,
            "frost should persist"
        );

        let checksum = sandbox.snapshot().checksum;
        assert_eq!(
            checksum.value(),
            0x1B3E_CB3B,
            "multi-hazard interaction fingerprint changed"
        );
    }

    #[test]
    fn hazard_decay_stability() {
        let mut sandbox = ScenarioSandbox::new(GOLDEN_SEED);
        sandbox.execute(SpawnCommand::hazard(
            WorldPos::new(8, 8, 8),
            HazardKind::Infection,
            0.5,
        ));

        for _ in 0..50 {
            sandbox.step(0.2);
        }

        let state = sandbox.state();
        let checksum = sandbox.snapshot().checksum;

        assert!(
            state.hazard_count(HazardKind::Infection) < 100,
            "infection should decay over time"
        );
        assert_eq!(
            checksum.value(),
            0xD132_8C33,
            "infection decay fingerprint changed"
        );
    }

    #[test]
    fn boundary_propagation_deterministic() {
        let mut sandbox = ScenarioSandbox::new(GOLDEN_SEED);
        sandbox.execute(SpawnCommand::hazard(
            WorldPos::new(15, 8, 8),
            HazardKind::Fire,
            1.0,
        ));
        sandbox.execute(SpawnCommand::hazard(
            WorldPos::new(16, 8, 8),
            HazardKind::Fire,
            0.0,
        ));

        for _ in 0..30 {
            sandbox.step(0.1);
        }

        let checksum = sandbox.snapshot().checksum;
        assert_eq!(
            checksum.value(),
            0x52B0_E90C,
            "boundary propagation fingerprint changed"
        );
    }

    #[test]
    fn replay_verification() {
        let config = SandboxConfig {
            record_history: true,
            ..SandboxConfig::new(GOLDEN_SEED)
        };
        let mut sandbox1 = ScenarioSandbox::with_config(config.clone());

        sandbox1.execute(SpawnCommand::hazard(
            WorldPos::new(8, 8, 8),
            HazardKind::Fire,
            1.0,
        ));
        for _ in 0..10 {
            sandbox1.step(0.1);
        }

        let history = sandbox1.history().to_vec();
        let final_checksum = sandbox1.snapshot().checksum;

        let mut sandbox2 = ScenarioSandbox::with_config(config);
        sandbox2.execute(SpawnCommand::hazard(
            WorldPos::new(8, 8, 8),
            HazardKind::Fire,
            1.0,
        ));
        let results = sandbox2.replay(&history);

        for (tick, matched) in &results {
            assert!(matched, "checksum mismatch at tick {tick}");
        }

        assert_eq!(
            sandbox2.snapshot().checksum,
            final_checksum,
            "final state should match"
        );
    }
}

mod persistence {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn chunk_serde_roundtrip() {
        let mut chunk = Chunk::new();
        chunk.set(LocalPos::new(0, 0, 0), STONE);
        chunk.set(LocalPos::new(5, 5, 5), DIRT);
        chunk.set(LocalPos::new(15, 15, 15), GRASS);

        let serialized = bincode::serialize(&chunk).expect("serialize");
        let deserialized: Chunk = bincode::deserialize(&serialized).expect("deserialize");

        assert_eq!(chunk.non_air_count(), deserialized.non_air_count());
        for (pos, block) in chunk.iter() {
            assert_eq!(block, deserialized.get(pos), "mismatch at {pos:?}");
        }
    }

    #[test]
    fn chunk_delta_roundtrip() {
        let mut delta = ChunkDelta::new();
        delta.set(LocalPos::new(0, 0, 0), STONE);
        delta.set(LocalPos::new(7, 7, 7), DIRT);
        delta.set(LocalPos::new(15, 0, 15), BlockId(100));

        let serialized = bincode::serialize(&delta).expect("serialize");
        let deserialized: ChunkDelta = bincode::deserialize(&serialized).expect("deserialize");

        for (pos, block) in delta.iter() {
            assert_eq!(
                Some(block),
                deserialized.get(pos),
                "delta mismatch at {pos:?}"
            );
        }
    }

    #[test]
    fn mutation_journal_checksum_stable() {
        let mut journal = MutationJournal::at_tick(100);

        journal.append(
            ChunkPos::new(0, 0, 0),
            LocalPos::new(5, 5, 5),
            AIR,
            STONE,
            MutationSource::Player,
            MutationReason::Place,
        );

        journal.advance_tick(101);
        journal.append(
            ChunkPos::new(0, 0, 0),
            LocalPos::new(6, 5, 5),
            AIR,
            DIRT,
            MutationSource::Environment,
            MutationReason::Growth,
        );

        journal.advance_tick(102);
        journal.append(
            ChunkPos::new(1, 0, 0),
            LocalPos::new(0, 0, 0),
            STONE,
            AIR,
            MutationSource::Player,
            MutationReason::Destroy,
        );

        let checksum = journal.checksum();
        assert_eq!(
            checksum, 0x1896_254B,
            "mutation journal checksum changed - review journaling"
        );
    }

    #[test]
    #[expect(
        clippy::cast_sign_loss,
        reason = "test indices are small positive values"
    )]
    fn mutation_journal_serde_roundtrip() {
        let mut journal = MutationJournal::at_tick(50);

        for i in 0..10_i32 {
            journal.append(
                ChunkPos::new(i, 0, 0),
                LocalPos::new(i as u32 % 16, 0, 0),
                AIR,
                STONE,
                MutationSource::Player,
                MutationReason::Place,
            );
            if i % 3 == 0 {
                journal.advance_tick(50 + i as u64 + 1);
            }
        }

        let original_checksum = journal.checksum();
        let serialized = bincode::serialize(&journal).expect("serialize");
        let deserialized: MutationJournal = bincode::deserialize(&serialized).expect("deserialize");

        assert_eq!(journal.len(), deserialized.len());
        assert_eq!(original_checksum, deserialized.checksum());
    }

    #[test]
    fn journal_snapshot_invariants() {
        let mut journal = MutationJournal::at_tick(100);
        let chunk_pos = ChunkPos::new(0, 0, 0);

        journal.append(
            chunk_pos,
            LocalPos::new(5, 5, 5),
            AIR,
            STONE,
            MutationSource::Player,
            MutationReason::Place,
        );
        journal.advance_tick(101);
        journal.append(
            chunk_pos,
            LocalPos::new(6, 5, 5),
            AIR,
            DIRT,
            MutationSource::Player,
            MutationReason::Place,
        );
        journal.advance_tick(102);
        journal.append(
            chunk_pos,
            LocalPos::new(7, 5, 5),
            AIR,
            GRASS,
            MutationSource::Player,
            MutationReason::Place,
        );

        let mut bases = HashMap::new();
        bases.insert(chunk_pos, Chunk::new());

        let snapshot = journal.snapshot(100, &bases);
        assert_eq!(snapshot.base_tick, 100);
        assert_eq!(snapshot.pending_mutations.len(), 2);

        let delta = snapshot.chunk_deltas.get(&chunk_pos).unwrap();
        assert_eq!(delta.get(LocalPos::new(5, 5, 5)), Some(STONE));
        assert_eq!(delta.get(LocalPos::new(6, 5, 5)), None);
    }

    #[test]
    fn region_roundtrip() {
        let temp_dir = TempDir::new().expect("temp dir");
        let region_path = temp_dir.path().join("r.0.0.lreg");

        let mut chunk = Chunk::new();
        chunk.set(LocalPos::new(0, 0, 0), STONE);
        chunk.set(LocalPos::new(8, 8, 8), DIRT);

        {
            let mut region = Region::open(&region_path).expect("create region");
            region
                .save_chunk(IVec2::new(0, 0), &chunk)
                .expect("write chunk");
        }

        {
            let mut region = Region::open(&region_path).expect("reopen region");
            let loaded = region
                .load_chunk(IVec2::new(0, 0))
                .expect("read chunk")
                .expect("chunk exists");

            assert_eq!(chunk.non_air_count(), loaded.non_air_count());
            assert_eq!(
                chunk.get(LocalPos::new(0, 0, 0)),
                loaded.get(LocalPos::new(0, 0, 0))
            );
            assert_eq!(
                chunk.get(LocalPos::new(8, 8, 8)),
                loaded.get(LocalPos::new(8, 8, 8))
            );
        }
    }

    #[test]
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        reason = "test data is small"
    )]
    fn region_multiple_chunks() {
        let temp_dir = TempDir::new().expect("temp dir");
        let region_path = temp_dir.path().join("r.0.0.lreg");

        let chunks: Vec<_> = (0..4_u32)
            .map(|i| {
                let mut c = Chunk::new();
                c.set(LocalPos::new(i, i, i), BlockId(i as u16 + 1));
                c
            })
            .collect();

        {
            let mut region = Region::open(&region_path).expect("create region");
            for (i, chunk) in chunks.iter().enumerate() {
                region
                    .save_chunk(IVec2::new(i as i32 % 2, i as i32 / 2), chunk)
                    .expect("write chunk");
            }
        }

        {
            let mut region = Region::open(&region_path).expect("reopen region");
            for (i, original) in chunks.iter().enumerate() {
                let loaded = region
                    .load_chunk(IVec2::new(i as i32 % 2, i as i32 / 2))
                    .expect("read")
                    .expect("exists");
                assert_eq!(
                    original.non_air_count(),
                    loaded.non_air_count(),
                    "chunk {i} count mismatch"
                );
            }
        }
    }
}

mod scenario {
    use super::*;

    const GOLDEN_SEED: u64 = 0x1234_5678_9ABC_DEF0;

    #[test]
    fn terrain_with_hazard_overlay() {
        let generator = TerrainGenerator::new(GOLDEN_SEED);
        let mut sandbox = ScenarioSandbox::new(GOLDEN_SEED);

        let terrain_positions = [
            ChunkPos::new(0, 4, 0),
            ChunkPos::new(1, 4, 0),
            ChunkPos::new(0, 4, 1),
        ];

        let mut terrain_checksum = ChecksumBuilder::new();
        for &pos in &terrain_positions {
            let chunk = generator.generate(pos);
            terrain_checksum.feed_u32(chunk.non_air_count());
        }
        let terrain_fp = terrain_checksum.build();

        sandbox.execute(SpawnCommand::hazard(
            WorldPos::new(8, 70, 8),
            HazardKind::Fire,
            0.9,
        ));
        sandbox.execute(SpawnCommand::hazard(
            WorldPos::new(24, 70, 8),
            HazardKind::Frost,
            0.7,
        ));

        for _ in 0..25 {
            sandbox.step(0.1);
        }

        let hazard_fp = sandbox.snapshot().checksum;

        let combined = terrain_fp.combine(&hazard_fp);
        assert_eq!(
            combined.value(),
            0x7B4F_5BDD,
            "terrain+hazard combined fingerprint changed"
        );
    }

    #[test]
    fn full_world_cycle_golden() {
        const CHUNK_POSITIONS: [ChunkPos; 4] = [
            ChunkPos::new(0, 4, 0),
            ChunkPos::new(1, 4, 0),
            ChunkPos::new(0, 4, 1),
            ChunkPos::new(1, 4, 1),
        ];

        let generator = TerrainGenerator::new(GOLDEN_SEED);
        let mut sandbox = ScenarioSandbox::new(GOLDEN_SEED);
        let mut journal = MutationJournal::at_tick(0);

        let chunks: HashMap<ChunkPos, Chunk> = CHUNK_POSITIONS
            .into_iter()
            .map(|pos| (pos, generator.generate(pos)))
            .collect();

        let mut combined_checksum = ChecksumBuilder::new();

        for pos in CHUNK_POSITIONS {
            let chunk = &chunks[&pos];
            combined_checksum.feed_i32(pos.x());
            combined_checksum.feed_i32(pos.y());
            combined_checksum.feed_i32(pos.z());
            combined_checksum.feed_u32(chunk.non_air_count());
        }

        sandbox.execute(SpawnCommand::hazard(
            WorldPos::new(8, 70, 8),
            HazardKind::Fire,
            1.0,
        ));
        sandbox.execute(SpawnCommand::hazard(
            WorldPos::new(20, 70, 20),
            HazardKind::Corruption,
            0.6,
        ));

        for tick in 0..30 {
            let result = sandbox.step(0.1);
            journal.advance_tick(tick + 1);

            if result.had_changes {
                #[expect(clippy::cast_possible_truncation, reason = "tick < 30")]
                journal.append(
                    ChunkPos::new(0, 4, 0),
                    LocalPos::new(8, 8, 8),
                    AIR,
                    BlockId(tick as u16 + 10),
                    MutationSource::Environment,
                    MutationReason::Decay,
                );
            }
        }

        combined_checksum.feed_u32(sandbox.snapshot().checksum.value());
        combined_checksum.feed_u32(journal.checksum());

        let final_fp = combined_checksum.build();
        assert_eq!(
            final_fp.value(),
            0x61CD_3E96,
            "full world cycle fingerprint changed - review all systems"
        );
    }

    #[test]
    fn multi_chunk_hazard_simulation() {
        let mut simulator = HazardSimulator::at_tick(0);
        let mut chunks = HashMap::new();

        for x in -1..=1 {
            for z in -1..=1 {
                let mut hazards = engine_world::ChunkHazards::new();
                if x == 0 && z == 0 {
                    hazards.activate(HazardKind::Fire, LocalPos::new(8, 8, 8), 1.0);
                }
                chunks.insert(ChunkPos::new(x, 0, z), hazards);
            }
        }

        let mut tick_checksums = Vec::new();
        for _ in 0..20 {
            let result = simulator.simulate_tick(&mut chunks, &(), 0.1);
            tick_checksums.push(result.overall_checksum.value());
        }

        let snapshot = simulator.snapshot(&chunks);
        assert!(snapshot.total_active() > 0, "should have active hazards");

        let mut final_builder = ChecksumBuilder::new();
        for cs in &tick_checksums {
            final_builder.feed_u32(*cs);
        }
        #[expect(clippy::cast_possible_truncation, reason = "test data is small")]
        final_builder.feed_u32(snapshot.total_active() as u32);

        assert_eq!(
            final_builder.build().value(),
            0xAA52_6CF4,
            "multi-chunk hazard simulation fingerprint changed"
        );
    }

    #[test]
    fn persistence_after_simulation() {
        let mut sandbox = ScenarioSandbox::new(GOLDEN_SEED);
        let mut journal = MutationJournal::at_tick(0);

        sandbox.execute(SpawnCommand::hazard(
            WorldPos::new(8, 8, 8),
            HazardKind::Fire,
            1.0,
        ));

        for tick in 0..10 {
            let result = sandbox.step(0.1);
            journal.advance_tick(tick + 1);

            if result.stats.spread_count > 0 {
                #[expect(clippy::cast_possible_truncation, reason = "tick < 10")]
                journal.append(
                    ChunkPos::new(0, 0, 0),
                    LocalPos::new(tick as u32 % 16, 0, 0),
                    AIR,
                    STONE,
                    MutationSource::Environment,
                    MutationReason::Growth,
                );
            }
        }

        let journal_serialized = bincode::serialize(&journal).expect("serialize journal");
        let journal_restored: MutationJournal =
            bincode::deserialize(&journal_serialized).expect("deserialize journal");

        assert_eq!(journal.checksum(), journal_restored.checksum());

        let sandbox_snapshot = sandbox.snapshot();
        let snapshot_serialized =
            bincode::serialize(&sandbox_snapshot).expect("serialize snapshot");
        let _snapshot_restored: engine_world::sandbox::SandboxSnapshot =
            bincode::deserialize(&snapshot_serialized).expect("deserialize snapshot");
    }
}
