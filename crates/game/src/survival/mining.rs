//! Mining mechanics for block destruction.
//!
//! Implements spec 6.4.3: mining time based on block hardness and tool type,
//! tool effectiveness multipliers, drop items on destroy, durability loss.

use engine_world::chunk::BlockId;

use crate::inventory::{ItemCategory, ItemDef, ToolType};

/// Base mining speed without any tool (hand mining).
pub const BASE_MINING_SPEED: f32 = 1.0;

/// Mining delay between starting and completing a mine action (seconds).
/// This is the base time modified by hardness and tool speed.
pub const BASE_MINE_TIME_SECS: f32 = 1.5;

/// Result of a mining calculation.
#[derive(Debug, Clone)]
pub struct MiningResult {
    /// Time in seconds to mine the block.
    pub time_secs: f32,
    /// Whether the tool is effective against this block.
    pub is_effective: bool,
    /// Whether the tool should take durability damage.
    pub should_damage_tool: bool,
}

/// Calculate mining time for a block given the tool used.
///
/// Mining time = base_time * hardness / tool_mining_speed
/// If no tool or wrong tool type, mining speed is 1.0 (very slow).
/// If tool is effective, mining speed is the tool's mining_speed value.
#[must_use]
pub fn calculate_mining_time(
    block_hardness: f32,
    tool: Option<&ItemDef>,
) -> MiningResult {
    let (speed, is_effective) = effective_mining_speed(block_hardness, tool);

    let time_secs = if block_hardness <= 0.0 {
        0.0 // Instant for zero-hardness blocks (like tall grass)
    } else {
        BASE_MINE_TIME_SECS * block_hardness / speed
    };

    MiningResult {
        time_secs,
        is_effective,
        should_damage_tool: is_effective && tool.is_some(),
    }
}

/// Determine the effective mining speed and whether the tool is effective.
///
/// Tool effectiveness rules:
/// - Pickaxe: effective on stone, ores, and hard blocks (hardness >= 1.5)
/// - Axe: effective on wood and wooden blocks (hardness < 1.5, not dirt/sand)
/// - Shovel: effective on dirt, sand, gravel (soft blocks, hardness < 1.0)
/// - Hoe/Sword: not effective for mining (combat tools)
/// - No tool: base speed, never effective
fn effective_mining_speed(
    block_hardness: f32,
    tool: Option<&ItemDef>,
) -> (f32, bool) {
    let Some(item) = tool else {
        return (BASE_MINING_SPEED, false);
    };

    if item.category != ItemCategory::Tool {
        return (BASE_MINING_SPEED, false);
    }

    let Some(tool_type) = item.tool_type else {
        return (BASE_MINING_SPEED, false);
    };

    match tool_type {
        ToolType::Pickaxe => {
            // Pickaxe is effective on hard blocks (stone, ores)
            if block_hardness >= 1.5 {
                (item.mining_speed, true)
            } else {
                // Can still mine but not effective bonus
                (BASE_MINING_SPEED, false)
            }
        }
        ToolType::Axe => {
            // Axe is effective on medium blocks (wood, planks)
            if block_hardness > 0.0 && block_hardness < 1.5 {
                (item.mining_speed, true)
            } else {
                (BASE_MINING_SPEED, false)
            }
        }
        ToolType::Shovel => {
            // Shovel is effective on soft blocks (dirt, sand, gravel)
            if block_hardness > 0.0 && block_hardness < 1.0 {
                (item.mining_speed, true)
            } else {
                (BASE_MINING_SPEED, false)
            }
        }
        ToolType::Hoe | ToolType::Sword => {
            // Hoe and sword are not mining tools
            (BASE_MINING_SPEED, false)
        }
    }
}

/// Mining progress tracker for continuous mining.
#[derive(Debug, Clone, Default)]
pub struct MiningProgress {
    /// Block being mined.
    pub target_block: Option<BlockPos>,
    /// Total time needed to mine the current block.
    pub total_time: f32,
    /// Time elapsed so far.
    pub elapsed: f32,
    /// Whether mining is complete.
    pub complete: bool,
}

/// Position of a block being mined (simplified).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl MiningProgress {
    /// Create a new mining progress tracker.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Start mining a new block.
    pub fn start(&mut self, block: BlockPos, total_time: f32) {
        self.target_block = Some(block);
        self.total_time = total_time;
        self.elapsed = 0.0;
        self.complete = false;
    }

    /// Advance mining by delta time.
    ///
    /// Returns true if the block is now fully mined.
    pub fn advance(&mut self, dt: f32) -> bool {
        if self.complete || self.target_block.is_none() {
            return false;
        }

        self.elapsed += dt;
        if self.elapsed >= self.total_time {
            self.complete = true;
            return true;
        }
        false
    }

    /// Get progress as a fraction (0.0 to 1.0).
    #[must_use]
    pub fn fraction(&self) -> f32 {
        if self.target_block.is_none() {
            return 0.0;
        }
        if self.total_time <= 0.0 {
            return 1.0;
        }
        (self.elapsed / self.total_time).clamp(0.0, 1.0)
    }

    /// Cancel the current mining operation.
    pub fn cancel(&mut self) {
        self.target_block = None;
        self.elapsed = 0.0;
        self.total_time = 0.0;
        self.complete = false;
    }

    /// Check if currently mining.
    #[must_use]
    pub fn is_mining(&self) -> bool {
        self.target_block.is_some() && !self.complete
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tool(tool_type: ToolType, mining_speed: f32) -> ItemDef {
        ItemDef {
            id: 1,
            name: String::from("Test Tool"),
            stack_size: 1,
            category: ItemCategory::Tool,
            tool_type: Some(tool_type),
            durability: Some(100),
            block_id: None,
            damage: 0.0,
            mining_speed,
            food_value: 0.0,
            saturation_value: 0.0,
        }
    }

    #[test]
    fn test_no_tool_slow_mining() {
        let result = calculate_mining_time(2.0, None);
        assert!(!result.is_effective);
        assert!(result.time_secs > 0.0);
        assert!(!result.should_damage_tool);
    }

    #[test]
    fn test_pickaxe_effective_on_stone() {
        let pickaxe = make_tool(ToolType::Pickaxe, 4.0);
        let result = calculate_mining_time(3.0, Some(&pickaxe));
        assert!(result.is_effective, "Pickaxe should be effective on hard blocks");
        assert!(result.should_damage_tool);
        // Time = 1.5 * 3.0 / 4.0 = 1.125
        assert!((result.time_secs - 1.125).abs() < 0.001);
    }

    #[test]
    fn test_pickaxe_not_effective_on_dirt() {
        let pickaxe = make_tool(ToolType::Pickaxe, 4.0);
        let result = calculate_mining_time(0.5, Some(&pickaxe)); // Soft block
        assert!(!result.is_effective, "Pickaxe should not be effective on soft blocks");
    }

    #[test]
    fn test_shovel_effective_on_dirt() {
        let shovel = make_tool(ToolType::Shovel, 2.0);
        let result = calculate_mining_time(0.5, Some(&shovel));
        assert!(result.is_effective, "Shovel should be effective on soft blocks");
        assert!(result.should_damage_tool);
    }

    #[test]
    fn test_axe_effective_on_wood() {
        let axe = make_tool(ToolType::Axe, 3.0);
        let result = calculate_mining_time(1.0, Some(&axe));
        assert!(result.is_effective, "Axe should be effective on medium blocks");
    }

    #[test]
    fn test_sword_not_mining_tool() {
        let sword = make_tool(ToolType::Sword, 1.5);
        let result = calculate_mining_time(3.0, Some(&sword));
        assert!(!result.is_effective, "Sword should not be effective for mining");
        assert!(!result.should_damage_tool);
    }

    #[test]
    fn test_zero_hardness_instant() {
        let result = calculate_mining_time(0.0, None);
        assert_eq!(result.time_secs, 0.0, "Zero hardness should be instant");
    }

    #[test]
    fn test_tool_speed_scales_linearly() {
        let pick1 = make_tool(ToolType::Pickaxe, 2.0);
        let pick2 = make_tool(ToolType::Pickaxe, 4.0);

        let result1 = calculate_mining_time(3.0, Some(&pick1));
        let result2 = calculate_mining_time(3.0, Some(&pick2));

        // Double speed = half time
        assert!(
            (result1.time_secs - result2.time_secs * 2.0).abs() < 0.01,
            "Double mining speed should halve time"
        );
    }

    #[test]
    fn test_mining_progress_completion() {
        let mut progress = MiningProgress::new();
        let block = BlockPos { x: 0, y: 0, z: 0 };

        progress.start(block, 2.0);
        assert!(progress.is_mining());
        assert_eq!(progress.fraction(), 0.0);

        // Advance half way
        assert!(!progress.advance(1.0));
        assert!((progress.fraction() - 0.5).abs() < 0.001);

        // Complete
        assert!(progress.advance(1.0));
        assert!(progress.complete);
        assert!(!progress.is_mining());
        assert!((progress.fraction() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_mining_progress_cancel() {
        let mut progress = MiningProgress::new();
        progress.start(BlockPos { x: 1, y: 2, z: 3 }, 5.0);
        progress.advance(2.0);

        progress.cancel();
        assert!(!progress.is_mining());
        assert_eq!(progress.fraction(), 0.0);
    }

    #[test]
    fn test_mining_progress_no_advance_when_complete() {
        let mut progress = MiningProgress::new();
        progress.start(BlockPos { x: 0, y: 0, z: 0 }, 1.0);
        progress.advance(1.0); // Completes

        let result = progress.advance(0.5); // Should not advance past complete
        assert!(!result);
    }
}
