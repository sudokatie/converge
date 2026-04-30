//! Structure template definitions.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::id::TemplateId;

/// Kind/category of a structure template.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
#[repr(u8)]
pub enum TemplateKind {
    /// Generic room or chamber.
    #[default]
    Room = 0,
    /// Corridor or passage.
    Corridor = 1,
    /// Junction connecting multiple passages.
    Junction = 2,
    /// Vertical shaft.
    Shaft = 3,
    /// Staircase or ramp.
    Stair = 4,
    /// Doorway or portal.
    Door = 5,
    /// Decorative element.
    Decoration = 6,
    /// Functional element (terminal, machine, etc).
    Functional = 7,
}

impl TemplateKind {
    /// Get the name of this kind.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Room => "room",
            Self::Corridor => "corridor",
            Self::Junction => "junction",
            Self::Shaft => "shaft",
            Self::Stair => "stair",
            Self::Door => "door",
            Self::Decoration => "decoration",
            Self::Functional => "functional",
        }
    }

    /// Get raw value.
    #[must_use]
    pub const fn as_raw(self) -> u8 {
        self as u8
    }

    /// Create from raw value.
    #[must_use]
    pub const fn from_raw(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Room),
            1 => Some(Self::Corridor),
            2 => Some(Self::Junction),
            3 => Some(Self::Shaft),
            4 => Some(Self::Stair),
            5 => Some(Self::Door),
            6 => Some(Self::Decoration),
            7 => Some(Self::Functional),
            _ => None,
        }
    }
}

/// Direction for socket/anchor facing.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
#[repr(u8)]
pub enum Direction {
    /// Positive X.
    #[default]
    East = 0,
    /// Negative X.
    West = 1,
    /// Positive Y (up).
    Up = 2,
    /// Negative Y (down).
    Down = 3,
    /// Positive Z.
    North = 4,
    /// Negative Z.
    South = 5,
}

impl Direction {
    /// Get the opposite direction.
    #[must_use]
    pub const fn opposite(self) -> Self {
        match self {
            Self::East => Self::West,
            Self::West => Self::East,
            Self::Up => Self::Down,
            Self::Down => Self::Up,
            Self::North => Self::South,
            Self::South => Self::North,
        }
    }

    /// Get the direction vector.
    #[must_use]
    pub const fn vector(self) -> [i32; 3] {
        match self {
            Self::East => [1, 0, 0],
            Self::West => [-1, 0, 0],
            Self::Up => [0, 1, 0],
            Self::Down => [0, -1, 0],
            Self::North => [0, 0, 1],
            Self::South => [0, 0, -1],
        }
    }

    /// Get raw value.
    #[must_use]
    pub const fn as_raw(self) -> u8 {
        self as u8
    }
}

/// Socket connector for template attachment.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Socket {
    /// Socket name/identifier.
    pub name: String,
    /// Position relative to template origin.
    pub position: [i32; 3],
    /// Facing direction.
    pub direction: Direction,
    /// Socket type for compatibility checking.
    pub socket_type: String,
}

impl Socket {
    /// Create a new socket.
    #[must_use]
    pub fn new(name: impl Into<String>, position: [i32; 3], direction: Direction) -> Self {
        Self {
            name: name.into(),
            position,
            direction,
            socket_type: "default".to_string(),
        }
    }

    /// Set socket type.
    #[must_use]
    pub fn with_type(mut self, socket_type: impl Into<String>) -> Self {
        self.socket_type = socket_type.into();
        self
    }

    /// Check if this socket can connect to another socket.
    #[must_use]
    pub fn can_connect(&self, other: &Socket) -> bool {
        self.socket_type == other.socket_type && self.direction == other.direction.opposite()
    }
}

/// Anchor point for template positioning.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Anchor {
    /// Anchor name.
    pub name: String,
    /// Position relative to template origin.
    pub position: [i32; 3],
}

impl Anchor {
    /// Create a new anchor.
    #[must_use]
    pub fn new(name: impl Into<String>, position: [i32; 3]) -> Self {
        Self {
            name: name.into(),
            position,
        }
    }
}

/// Axis-aligned bounding box for template footprint.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Bounds {
    /// Minimum corner (inclusive).
    pub min: [i32; 3],
    /// Maximum corner (inclusive).
    pub max: [i32; 3],
}

impl Bounds {
    /// Create new bounds.
    #[must_use]
    pub const fn new(min: [i32; 3], max: [i32; 3]) -> Self {
        Self { min, max }
    }

    /// Create bounds from size (origin at 0,0,0).
    #[must_use]
    pub const fn from_size(width: i32, height: i32, depth: i32) -> Self {
        Self {
            min: [0, 0, 0],
            max: [width - 1, height - 1, depth - 1],
        }
    }

    /// Get width (X dimension).
    #[must_use]
    pub const fn width(&self) -> i32 {
        self.max[0] - self.min[0] + 1
    }

    /// Get height (Y dimension).
    #[must_use]
    pub const fn height(&self) -> i32 {
        self.max[1] - self.min[1] + 1
    }

    /// Get depth (Z dimension).
    #[must_use]
    pub const fn depth(&self) -> i32 {
        self.max[2] - self.min[2] + 1
    }

    /// Get volume.
    #[must_use]
    pub const fn volume(&self) -> i64 {
        self.width() as i64 * self.height() as i64 * self.depth() as i64
    }

    /// Check if a point is inside bounds.
    #[must_use]
    pub const fn contains(&self, point: [i32; 3]) -> bool {
        point[0] >= self.min[0]
            && point[0] <= self.max[0]
            && point[1] >= self.min[1]
            && point[1] <= self.max[1]
            && point[2] >= self.min[2]
            && point[2] <= self.max[2]
    }

    /// Check if bounds overlap.
    #[must_use]
    pub const fn overlaps(&self, other: &Bounds) -> bool {
        self.min[0] <= other.max[0]
            && self.max[0] >= other.min[0]
            && self.min[1] <= other.max[1]
            && self.max[1] >= other.min[1]
            && self.min[2] <= other.max[2]
            && self.max[2] >= other.min[2]
    }

    /// Translate bounds by offset.
    #[must_use]
    pub const fn translate(&self, offset: [i32; 3]) -> Self {
        Self {
            min: [
                self.min[0] + offset[0],
                self.min[1] + offset[1],
                self.min[2] + offset[2],
            ],
            max: [
                self.max[0] + offset[0],
                self.max[1] + offset[1],
                self.max[2] + offset[2],
            ],
        }
    }

    /// Iterate over all cells in bounds.
    pub fn cells(&self) -> impl Iterator<Item = [i32; 3]> + '_ {
        (self.min[0]..=self.max[0]).flat_map(move |x| {
            (self.min[1]..=self.max[1])
                .flat_map(move |y| (self.min[2]..=self.max[2]).map(move |z| [x, y, z]))
        })
    }
}

/// Block type for palette entries.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BlockType {
    /// Block identifier.
    pub id: String,
    /// Block metadata/variant.
    pub metadata: u8,
}

impl BlockType {
    /// Create a new block type.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            metadata: 0,
        }
    }

    /// Set metadata.
    #[must_use]
    pub fn with_metadata(mut self, metadata: u8) -> Self {
        self.metadata = metadata;
        self
    }
}

/// Block palette for template blocks.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockPalette {
    /// Named palette entries.
    pub entries: BTreeMap<String, BlockType>,
}

impl BlockPalette {
    /// Create a new empty palette.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    /// Add an entry.
    pub fn add(&mut self, name: impl Into<String>, block: BlockType) {
        self.entries.insert(name.into(), block);
    }

    /// Get an entry.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&BlockType> {
        self.entries.get(name)
    }

    /// Check if empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get entry count.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }
}

/// Placement rules for template instantiation.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PlacementRules {
    /// Minimum instances allowed.
    pub min_count: u32,
    /// Maximum instances allowed (0 = unlimited).
    pub max_count: u32,
    /// Required clearance around placement.
    pub clearance: i32,
    /// Weight for weighted selection.
    pub weight: f32,
    /// Required tags on parent placement.
    pub required_tags: Vec<String>,
    /// Forbidden tags on parent placement.
    pub forbidden_tags: Vec<String>,
}

impl PlacementRules {
    /// Create default rules.
    #[must_use]
    pub fn new() -> Self {
        Self {
            min_count: 0,
            max_count: 0,
            clearance: 0,
            weight: 1.0,
            required_tags: Vec::new(),
            forbidden_tags: Vec::new(),
        }
    }

    /// Set min/max count.
    #[must_use]
    pub fn with_count(mut self, min: u32, max: u32) -> Self {
        self.min_count = min;
        self.max_count = max;
        self
    }

    /// Set weight.
    #[must_use]
    pub fn with_weight(mut self, weight: f32) -> Self {
        self.weight = weight;
        self
    }

    /// Set clearance.
    #[must_use]
    pub fn with_clearance(mut self, clearance: i32) -> Self {
        self.clearance = clearance;
        self
    }

    /// Add required tag.
    #[must_use]
    pub fn require_tag(mut self, tag: impl Into<String>) -> Self {
        self.required_tags.push(tag.into());
        self
    }

    /// Add forbidden tag.
    #[must_use]
    pub fn forbid_tag(mut self, tag: impl Into<String>) -> Self {
        self.forbidden_tags.push(tag.into());
        self
    }
}

/// A reusable structure template.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StructureTemplate {
    /// Unique identifier.
    pub id: TemplateId,
    /// Display name.
    pub name: String,
    /// Template kind/category.
    pub kind: TemplateKind,
    /// Bounding box footprint.
    pub bounds: Bounds,
    /// Anchor points.
    pub anchors: Vec<Anchor>,
    /// Socket connectors.
    pub sockets: Vec<Socket>,
    /// Block palette.
    pub palette: BlockPalette,
    /// Placement rules.
    pub rules: PlacementRules,
    /// Metadata tags.
    pub tags: Vec<String>,
}

impl StructureTemplate {
    /// Create a new template.
    #[must_use]
    pub fn new(id: TemplateId, name: impl Into<String>, bounds: Bounds) -> Self {
        Self {
            id,
            name: name.into(),
            kind: TemplateKind::Room,
            bounds,
            anchors: Vec::new(),
            sockets: Vec::new(),
            palette: BlockPalette::new(),
            rules: PlacementRules::new(),
            tags: Vec::new(),
        }
    }

    /// Set kind.
    #[must_use]
    pub fn with_kind(mut self, kind: TemplateKind) -> Self {
        self.kind = kind;
        self
    }

    /// Add an anchor.
    #[must_use]
    pub fn with_anchor(mut self, anchor: Anchor) -> Self {
        self.anchors.push(anchor);
        self
    }

    /// Add a socket.
    #[must_use]
    pub fn with_socket(mut self, socket: Socket) -> Self {
        self.sockets.push(socket);
        self
    }

    /// Set palette.
    #[must_use]
    pub fn with_palette(mut self, palette: BlockPalette) -> Self {
        self.palette = palette;
        self
    }

    /// Set rules.
    #[must_use]
    pub fn with_rules(mut self, rules: PlacementRules) -> Self {
        self.rules = rules;
        self
    }

    /// Add a tag.
    #[must_use]
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Check if template has a specific tag.
    #[must_use]
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }

    /// Get socket by name.
    #[must_use]
    pub fn socket(&self, name: &str) -> Option<&Socket> {
        self.sockets.iter().find(|s| s.name == name)
    }

    /// Get anchor by name.
    #[must_use]
    pub fn anchor(&self, name: &str) -> Option<&Anchor> {
        self.anchors.iter().find(|a| a.name == name)
    }

    /// Get all sockets compatible with a given socket.
    pub fn compatible_sockets<'a>(
        &'a self,
        socket: &'a Socket,
    ) -> impl Iterator<Item = &'a Socket> {
        self.sockets.iter().filter(move |s| s.can_connect(socket))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_kind_basics() {
        assert_eq!(TemplateKind::Room.name(), "room");
        assert_eq!(TemplateKind::from_raw(1), Some(TemplateKind::Corridor));
        assert_eq!(TemplateKind::from_raw(99), None);
    }

    #[test]
    fn direction_opposite() {
        assert_eq!(Direction::East.opposite(), Direction::West);
        assert_eq!(Direction::Up.opposite(), Direction::Down);
        assert_eq!(Direction::North.opposite(), Direction::South);
    }

    #[test]
    fn socket_compatibility() {
        let s1 = Socket::new("exit", [5, 0, 0], Direction::East).with_type("corridor");
        let s2 = Socket::new("entry", [0, 0, 0], Direction::West).with_type("corridor");
        let s3 = Socket::new("entry", [0, 0, 0], Direction::West).with_type("shaft");

        assert!(s1.can_connect(&s2));
        assert!(!s1.can_connect(&s3));
    }

    #[test]
    fn bounds_basics() {
        let bounds = Bounds::from_size(10, 5, 8);
        assert_eq!(bounds.width(), 10);
        assert_eq!(bounds.height(), 5);
        assert_eq!(bounds.depth(), 8);
        assert_eq!(bounds.volume(), 400);
    }

    #[test]
    fn bounds_contains() {
        let bounds = Bounds::new([0, 0, 0], [9, 4, 7]);
        assert!(bounds.contains([5, 2, 3]));
        assert!(!bounds.contains([10, 2, 3]));
        assert!(!bounds.contains([-1, 0, 0]));
    }

    #[test]
    fn bounds_overlap() {
        let b1 = Bounds::new([0, 0, 0], [5, 5, 5]);
        let b2 = Bounds::new([3, 3, 3], [8, 8, 8]);
        let b3 = Bounds::new([10, 10, 10], [15, 15, 15]);

        assert!(b1.overlaps(&b2));
        assert!(!b1.overlaps(&b3));
    }

    #[test]
    fn bounds_translate() {
        let bounds = Bounds::new([0, 0, 0], [5, 5, 5]);
        let translated = bounds.translate([10, 20, 30]);
        assert_eq!(translated.min, [10, 20, 30]);
        assert_eq!(translated.max, [15, 25, 35]);
    }

    #[test]
    fn bounds_cells() {
        let bounds = Bounds::new([0, 0, 0], [1, 1, 1]);
        let cells: Vec<_> = bounds.cells().collect();
        assert_eq!(cells.len(), 8);
    }

    #[test]
    fn block_palette() {
        let mut palette = BlockPalette::new();
        palette.add("floor", BlockType::new("stone").with_metadata(1));
        palette.add("wall", BlockType::new("brick"));

        assert_eq!(palette.len(), 2);
        assert_eq!(palette.get("floor").unwrap().id, "stone");
    }

    #[test]
    fn template_creation() {
        let template = StructureTemplate::new(
            TemplateId::new(1),
            "test_room",
            Bounds::from_size(10, 5, 10),
        )
        .with_kind(TemplateKind::Room)
        .with_tag("interior")
        .with_anchor(Anchor::new("center", [5, 0, 5]))
        .with_socket(Socket::new("north", [5, 0, 9], Direction::North));

        assert_eq!(template.name, "test_room");
        assert!(template.has_tag("interior"));
        assert!(template.anchor("center").is_some());
        assert!(template.socket("north").is_some());
    }

    #[test]
    fn serde_roundtrip() {
        let template =
            StructureTemplate::new(TemplateId::new(42), "test", Bounds::from_size(5, 5, 5))
                .with_kind(TemplateKind::Corridor)
                .with_tag("test_tag");

        let json = serde_json::to_string(&template).unwrap();
        let recovered: StructureTemplate = serde_json::from_str(&json).unwrap();
        assert_eq!(template, recovered);
    }
}
