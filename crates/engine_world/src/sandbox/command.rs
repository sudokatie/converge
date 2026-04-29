//! Spawn commands for the scenario sandbox.

use engine_core::coords::WorldPos;
use glam::Vec3;
use serde::{Deserialize, Serialize};

use crate::environment::{FieldChannel, FluidKind, HazardKind, VectorFieldChannel};

/// Kinds of entities that can be spawned in the sandbox.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SpawnKind {
    /// Spawn a hazard with given kind and intensity.
    Hazard { kind: HazardKind, intensity: f32 },

    /// Set a scalar field value.
    ScalarField { channel: FieldChannel, value: f32 },

    /// Set a vector field value.
    VectorField {
        channel: VectorFieldChannel,
        value: Vec3,
    },

    /// Spawn fluid with given kind and volume.
    Fluid {
        kind: FluidKind,
        volume: f32,
        pressure: f32,
        temperature: f32,
    },

    /// Add structural load at position.
    StructuralLoad { load: f32 },
}

/// Command to spawn something in the sandbox.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpawnCommand {
    /// World position to spawn at.
    pub pos: WorldPos,
    /// What to spawn.
    pub kind: SpawnKind,
}

impl SpawnCommand {
    /// Create a hazard spawn command.
    #[must_use]
    pub fn hazard(pos: WorldPos, kind: HazardKind, intensity: f32) -> Self {
        Self {
            pos,
            kind: SpawnKind::Hazard {
                kind,
                intensity: intensity.clamp(0.0, 1.0),
            },
        }
    }

    /// Create a scalar field spawn command.
    #[must_use]
    pub fn scalar_field(pos: WorldPos, channel: FieldChannel, value: f32) -> Self {
        Self {
            pos,
            kind: SpawnKind::ScalarField { channel, value },
        }
    }

    /// Create a vector field spawn command.
    #[must_use]
    pub fn vector_field(pos: WorldPos, channel: VectorFieldChannel, value: Vec3) -> Self {
        Self {
            pos,
            kind: SpawnKind::VectorField { channel, value },
        }
    }

    /// Create a fluid spawn command.
    #[must_use]
    pub fn fluid(pos: WorldPos, kind: FluidKind, volume: f32) -> Self {
        Self {
            pos,
            kind: SpawnKind::Fluid {
                kind,
                volume: volume.clamp(0.0, 1.0),
                pressure: 1.0,
                temperature: 20.0,
            },
        }
    }

    /// Create a fluid spawn command with full parameters.
    #[must_use]
    pub fn fluid_full(
        pos: WorldPos,
        kind: FluidKind,
        volume: f32,
        pressure: f32,
        temperature: f32,
    ) -> Self {
        Self {
            pos,
            kind: SpawnKind::Fluid {
                kind,
                volume,
                pressure,
                temperature,
            },
        }
    }

    /// Create a structural load spawn command.
    #[must_use]
    pub fn structural_load(pos: WorldPos, load: f32) -> Self {
        Self {
            pos,
            kind: SpawnKind::StructuralLoad { load },
        }
    }
}

/// Result of executing a spawn command.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandResult {
    /// Whether the command succeeded.
    pub success: bool,
    /// Error message if failed.
    pub error: Option<String>,
    /// Tick when command was executed.
    pub tick: u64,
}

impl CommandResult {
    /// Create a successful result.
    #[must_use]
    pub fn ok(tick: u64) -> Self {
        Self {
            success: true,
            error: None,
            tick,
        }
    }

    /// Create a failed result.
    #[must_use]
    pub fn err(tick: u64, message: impl Into<String>) -> Self {
        Self {
            success: false,
            error: Some(message.into()),
            tick,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hazard_command_clamps_intensity() {
        let cmd = SpawnCommand::hazard(WorldPos::new(0, 0, 0), HazardKind::Fire, 1.5);
        match cmd.kind {
            SpawnKind::Hazard { intensity, .. } => assert!((intensity - 1.0).abs() < 0.001),
            _ => panic!("wrong kind"),
        }

        let cmd2 = SpawnCommand::hazard(WorldPos::new(0, 0, 0), HazardKind::Fire, -0.5);
        match cmd2.kind {
            SpawnKind::Hazard { intensity, .. } => assert!(intensity.abs() < 0.001),
            _ => panic!("wrong kind"),
        }
    }

    #[test]
    fn command_serde_round_trip() {
        let cmd = SpawnCommand::hazard(WorldPos::new(1, 2, 3), HazardKind::Frost, 0.75);

        let json = serde_json::to_string(&cmd).unwrap();
        let recovered: SpawnCommand = serde_json::from_str(&json).unwrap();

        assert_eq!(cmd, recovered);
    }

    #[test]
    fn fluid_command_defaults() {
        let cmd = SpawnCommand::fluid(WorldPos::new(0, 0, 0), FluidKind::Water, 0.5);
        match cmd.kind {
            SpawnKind::Fluid {
                pressure,
                temperature,
                ..
            } => {
                assert!((pressure - 1.0).abs() < 0.001);
                assert!((temperature - 20.0).abs() < 0.001);
            }
            _ => panic!("wrong kind"),
        }
    }

    #[test]
    fn result_accessors() {
        let ok = CommandResult::ok(100);
        assert!(ok.success);
        assert!(ok.error.is_none());
        assert_eq!(ok.tick, 100);

        let err = CommandResult::err(50, "test error");
        assert!(!err.success);
        assert_eq!(err.error.as_deref(), Some("test error"));
    }
}
