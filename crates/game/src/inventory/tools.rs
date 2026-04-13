//! Tool effectiveness system for mining and harvesting.
//!
//! Determines mining speed and whether blocks drop items based on
//! the tool being used.

use engine_world::chunk::BlockId;
use serde::{Deserialize, Serialize};

use super::registry::ToolType;

/// Block hardness category for tool effectiveness.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlockHardness {
    /// Instant break (flowers, torches).
    Instant,
    /// Soft blocks (dirt, sand, leaves).
    Soft,
    /// Wood blocks (logs, planks).
    Wood,
    /// Stone blocks (stone, cobblestone, ores).
    Stone,
    /// Metal blocks (iron, gold).
    Metal,
    /// Unbreakable (bedrock).
    Unbreakable,
}

impl BlockHardness {
    /// Get base break time in seconds (with bare hands).
    #[must_use]
    pub fn base_break_time(&self) -> f32 {
        match self {
            BlockHardness::Instant => 0.0,
            BlockHardness::Soft => 0.5,
            BlockHardness::Wood => 2.0,
            BlockHardness::Stone => 7.5,
            BlockHardness::Metal => 15.0,
            BlockHardness::Unbreakable => f32::INFINITY,
        }
    }

    /// Get the preferred tool type for this hardness.
    #[must_use]
    pub fn preferred_tool(&self) -> Option<ToolType> {
        match self {
            BlockHardness::Instant => None,
            BlockHardness::Soft => Some(ToolType::Shovel),
            BlockHardness::Wood => Some(ToolType::Axe),
            BlockHardness::Stone | BlockHardness::Metal => Some(ToolType::Pickaxe),
            BlockHardness::Unbreakable => None,
        }
    }

    /// Check if this block requires a tool to drop items.
    #[must_use]
    pub fn requires_tool(&self) -> bool {
        matches!(self, BlockHardness::Stone | BlockHardness::Metal)
    }
}

/// Block properties for tool interactions.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockToolProperties {
    /// Block hardness category.
    pub hardness: BlockHardness,
    /// Minimum tool tier required to drop items (0 = wood, 1 = stone, 2 = iron, 3 = diamond).
    pub min_tier: u8,
    /// Custom break time override (if set, overrides hardness-based time).
    pub custom_break_time: Option<f32>,
}

impl Default for BlockToolProperties {
    fn default() -> Self {
        Self {
            hardness: BlockHardness::Soft,
            min_tier: 0,
            custom_break_time: None,
        }
    }
}

impl BlockToolProperties {
    /// Create properties for an instant-break block.
    #[must_use]
    pub fn instant() -> Self {
        Self {
            hardness: BlockHardness::Instant,
            min_tier: 0,
            custom_break_time: None,
        }
    }

    /// Create properties for a soft block (dirt, sand).
    #[must_use]
    pub fn soft() -> Self {
        Self {
            hardness: BlockHardness::Soft,
            min_tier: 0,
            custom_break_time: None,
        }
    }

    /// Create properties for a wood block.
    #[must_use]
    pub fn wood() -> Self {
        Self {
            hardness: BlockHardness::Wood,
            min_tier: 0,
            custom_break_time: None,
        }
    }

    /// Create properties for a stone block.
    #[must_use]
    pub fn stone() -> Self {
        Self {
            hardness: BlockHardness::Stone,
            min_tier: 0,
            custom_break_time: None,
        }
    }

    /// Create properties for an ore block with tier requirement.
    #[must_use]
    pub fn ore(min_tier: u8) -> Self {
        Self {
            hardness: BlockHardness::Stone,
            min_tier,
            custom_break_time: None,
        }
    }

    /// Create unbreakable block properties.
    #[must_use]
    pub fn unbreakable() -> Self {
        Self {
            hardness: BlockHardness::Unbreakable,
            min_tier: 0,
            custom_break_time: None,
        }
    }

    /// Set custom break time.
    #[must_use]
    pub fn with_break_time(mut self, time: f32) -> Self {
        self.custom_break_time = Some(time);
        self
    }
}

/// Tool tier for harvest level checking.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ToolTier {
    /// Hand (no tool).
    Hand = 0,
    /// Wooden tools.
    Wood = 1,
    /// Stone tools.
    Stone = 2,
    /// Iron tools.
    Iron = 3,
    /// Diamond tools.
    Diamond = 4,
}

impl ToolTier {
    /// Get tier from a numeric value.
    #[must_use]
    pub fn from_level(level: u8) -> Self {
        match level {
            0 => ToolTier::Hand,
            1 => ToolTier::Wood,
            2 => ToolTier::Stone,
            3 => ToolTier::Iron,
            _ => ToolTier::Diamond,
        }
    }

    /// Get the base mining speed multiplier for this tier.
    #[must_use]
    pub fn speed_multiplier(&self) -> f32 {
        match self {
            ToolTier::Hand => 1.0,
            ToolTier::Wood => 2.0,
            ToolTier::Stone => 4.0,
            ToolTier::Iron => 6.0,
            ToolTier::Diamond => 8.0,
        }
    }
}

/// Calculate mining speed for a tool against a block.
///
/// Returns the mining speed multiplier (higher = faster mining).
#[must_use]
pub fn calculate_mining_speed(
    tool_type: Option<ToolType>,
    tool_tier: ToolTier,
    tool_speed: f32,
    block_props: &BlockToolProperties,
) -> f32 {
    // Unbreakable blocks can't be mined
    if block_props.hardness == BlockHardness::Unbreakable {
        return 0.0;
    }

    // Check if tool is effective
    let preferred = block_props.hardness.preferred_tool();
    let is_effective = tool_type.is_some() && tool_type == preferred;

    if is_effective {
        // Effective tool: use tool's speed multiplied by tier bonus
        tool_speed * tool_tier.speed_multiplier()
    } else if tool_type.is_some() {
        // Wrong tool type: no bonus, slight speed increase
        tool_speed * 0.5
    } else {
        // No tool (hand): base speed
        1.0
    }
}

/// Calculate break time for a block.
///
/// Returns time in seconds to break the block.
#[must_use]
pub fn calculate_break_time(
    tool_type: Option<ToolType>,
    tool_tier: ToolTier,
    tool_speed: f32,
    block_props: &BlockToolProperties,
) -> f32 {
    let base_time = block_props
        .custom_break_time
        .unwrap_or_else(|| block_props.hardness.base_break_time());

    if base_time == f32::INFINITY {
        return f32::INFINITY;
    }

    if base_time == 0.0 {
        return 0.0;
    }

    let speed = calculate_mining_speed(tool_type, tool_tier, tool_speed, block_props);
    if speed <= 0.0 {
        return f32::INFINITY;
    }

    base_time / speed
}

/// Check if a block will drop items when broken with given tool.
#[must_use]
pub fn will_drop_items(
    tool_type: Option<ToolType>,
    tool_tier: ToolTier,
    block_props: &BlockToolProperties,
) -> bool {
    // Blocks that don't require tools always drop
    if !block_props.hardness.requires_tool() {
        return true;
    }

    // Must have correct tool type
    let preferred = block_props.hardness.preferred_tool();
    if tool_type != preferred {
        return false;
    }

    // Must meet tier requirement
    (tool_tier as u8) >= block_props.min_tier
}

/// Get default block properties for common block IDs.
#[must_use]
pub fn default_block_properties(block_id: BlockId) -> BlockToolProperties {
    match block_id.0 {
        0 => BlockToolProperties::instant(),           // Air
        1 => BlockToolProperties::stone(),             // Stone
        2 => BlockToolProperties::soft(),              // Dirt
        3 => BlockToolProperties::soft(),              // Grass
        4 => BlockToolProperties::soft(),              // Sand
        5 => BlockToolProperties::soft().with_break_time(0.1), // Water (instant-ish)
        6 => BlockToolProperties::wood(),              // Oak Log
        7 => BlockToolProperties::instant(),           // Oak Leaves
        8 => BlockToolProperties::wood(),              // Birch Log
        9 => BlockToolProperties::instant(),           // Birch Leaves
        10 => BlockToolProperties::instant(),          // Cactus
        11 => BlockToolProperties::wood(),             // Oak Planks
        12 => BlockToolProperties::stone(),            // Cobblestone
        13 => BlockToolProperties::ore(1),             // Coal Ore (wood pickaxe)
        14 => BlockToolProperties::ore(2),             // Iron Ore (stone pickaxe)
        15 => BlockToolProperties::ore(3),             // Gold Ore (iron pickaxe)
        16 => BlockToolProperties::ore(3),             // Diamond Ore (iron pickaxe)
        255 => BlockToolProperties::unbreakable(),     // Bedrock
        _ => BlockToolProperties::soft(),              // Default to soft
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_hardness_break_time() {
        assert_eq!(BlockHardness::Instant.base_break_time(), 0.0);
        assert!(BlockHardness::Soft.base_break_time() < BlockHardness::Stone.base_break_time());
        assert_eq!(BlockHardness::Unbreakable.base_break_time(), f32::INFINITY);
    }

    #[test]
    fn test_preferred_tools() {
        assert_eq!(BlockHardness::Soft.preferred_tool(), Some(ToolType::Shovel));
        assert_eq!(BlockHardness::Wood.preferred_tool(), Some(ToolType::Axe));
        assert_eq!(BlockHardness::Stone.preferred_tool(), Some(ToolType::Pickaxe));
    }

    #[test]
    fn test_tool_tier_ordering() {
        assert!(ToolTier::Wood > ToolTier::Hand);
        assert!(ToolTier::Diamond > ToolTier::Iron);
    }

    #[test]
    fn test_mining_speed_with_correct_tool() {
        let props = BlockToolProperties::stone();

        // Correct tool (pickaxe) is fast
        let speed_pickaxe = calculate_mining_speed(
            Some(ToolType::Pickaxe),
            ToolTier::Stone,
            4.0,
            &props,
        );

        // Wrong tool (axe) is slower
        let speed_axe = calculate_mining_speed(
            Some(ToolType::Axe),
            ToolTier::Stone,
            4.0,
            &props,
        );

        assert!(speed_pickaxe > speed_axe);
    }

    #[test]
    fn test_mining_speed_hand() {
        let props = BlockToolProperties::soft();

        let speed = calculate_mining_speed(None, ToolTier::Hand, 1.0, &props);
        assert_eq!(speed, 1.0);
    }

    #[test]
    fn test_break_time_calculation() {
        let props = BlockToolProperties::stone();

        // Hand: slow
        let time_hand = calculate_break_time(None, ToolTier::Hand, 1.0, &props);

        // Iron pickaxe: fast
        let time_iron = calculate_break_time(
            Some(ToolType::Pickaxe),
            ToolTier::Iron,
            6.0,
            &props,
        );

        assert!(time_iron < time_hand);
    }

    #[test]
    fn test_unbreakable_block() {
        let props = BlockToolProperties::unbreakable();

        let speed = calculate_mining_speed(Some(ToolType::Pickaxe), ToolTier::Diamond, 8.0, &props);
        assert_eq!(speed, 0.0);

        let time = calculate_break_time(Some(ToolType::Pickaxe), ToolTier::Diamond, 8.0, &props);
        assert_eq!(time, f32::INFINITY);
    }

    #[test]
    fn test_will_drop_items_soft_block() {
        let props = BlockToolProperties::soft();

        // Soft blocks always drop
        assert!(will_drop_items(None, ToolTier::Hand, &props));
        assert!(will_drop_items(Some(ToolType::Shovel), ToolTier::Wood, &props));
    }

    #[test]
    fn test_will_drop_items_stone_needs_pickaxe() {
        let props = BlockToolProperties::stone();

        // Stone needs pickaxe
        assert!(!will_drop_items(None, ToolTier::Hand, &props));
        assert!(!will_drop_items(Some(ToolType::Axe), ToolTier::Iron, &props));
        assert!(will_drop_items(Some(ToolType::Pickaxe), ToolTier::Wood, &props));
    }

    #[test]
    fn test_will_drop_items_ore_tier() {
        let iron_ore = BlockToolProperties::ore(2); // Needs stone pickaxe

        // Wood pickaxe too low tier
        assert!(!will_drop_items(Some(ToolType::Pickaxe), ToolTier::Wood, &iron_ore));

        // Stone pickaxe works
        assert!(will_drop_items(Some(ToolType::Pickaxe), ToolTier::Stone, &iron_ore));

        // Iron pickaxe also works
        assert!(will_drop_items(Some(ToolType::Pickaxe), ToolTier::Iron, &iron_ore));
    }

    #[test]
    fn test_default_block_properties() {
        let stone = default_block_properties(BlockId(1));
        assert_eq!(stone.hardness, BlockHardness::Stone);

        let dirt = default_block_properties(BlockId(2));
        assert_eq!(dirt.hardness, BlockHardness::Soft);

        let bedrock = default_block_properties(BlockId(255));
        assert_eq!(bedrock.hardness, BlockHardness::Unbreakable);
    }

    #[test]
    fn test_custom_break_time() {
        let props = BlockToolProperties::soft().with_break_time(0.1);

        let time = calculate_break_time(None, ToolTier::Hand, 1.0, &props);
        assert!((time - 0.1).abs() < 0.01);
    }
}
