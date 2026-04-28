//! Multi-domain navigation abstraction for voxel, swimming, climbing, flying, zero-G, and dynamic worlds.
//!
//! Provides deterministic navigation planning across movement domains:
//!
//! - Movement domains: walking, swimming, climbing, flying, zero-G, dynamic frames
//! - Agent capabilities specifying which domains and costs an agent supports
//! - Node/edge annotations for domain-specific traversal requirements
//! - Route requests with domain preferences and constraints
//! - Route results with plans, steering hints, and replan triggers
//! - Dynamic frame support: moving platforms, inherited velocity, reference frames
//! - Unloaded/stale region summaries for offline simulation handoff
//! - Integration with existing A* pathfinding and steering behaviors

use crate::pathfinding::astar::{AStarConfig, GridPos, PathResult, Walkable};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Movement domain identifier.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum MovementDomain {
    /// Standard voxel-based ground walking.
    #[default]
    Walking,
    /// Swimming through liquid volumes.
    Swimming,
    /// Climbing vertical surfaces or ladders.
    Climbing,
    /// Flying through open air spaces.
    Flying,
    /// Zero-G maneuvering in weightless environments.
    ZeroG,
    /// Dynamic frame movement (on moving platforms, vehicles, etc.).
    DynamicFrame,
}

impl MovementDomain {
    /// Returns whether this domain requires continuous position updates.
    #[must_use]
    pub fn is_continuous(&self) -> bool {
        matches!(self, Self::Flying | Self::Swimming | Self::ZeroG)
    }

    /// Returns whether this domain is affected by gravity.
    #[must_use]
    pub fn has_gravity(&self) -> bool {
        matches!(self, Self::Walking | Self::Climbing | Self::Swimming)
    }

    /// Returns default movement speed multiplier for this domain.
    #[must_use]
    pub fn default_speed_multiplier(&self) -> f32 {
        match self {
            Self::Walking | Self::DynamicFrame => 1.0,
            Self::Swimming => 0.6,
            Self::Climbing => 0.4,
            Self::Flying => 1.5,
            Self::ZeroG => 0.8,
        }
    }
}

/// Cost modifier for traversing a specific domain.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct DomainCost {
    /// Base cost multiplier (1.0 = normal).
    pub multiplier: f32,
    /// Energy cost per unit distance.
    pub energy_per_unit: f32,
    /// Risk factor (0.0 = safe, 1.0 = dangerous).
    pub risk: f32,
}

impl Default for DomainCost {
    fn default() -> Self {
        Self {
            multiplier: 1.0,
            energy_per_unit: 1.0,
            risk: 0.0,
        }
    }
}

impl DomainCost {
    /// Create a new domain cost.
    #[must_use]
    pub fn new(multiplier: f32, energy: f32, risk: f32) -> Self {
        Self {
            multiplier: multiplier.max(0.1),
            energy_per_unit: energy.max(0.0),
            risk: risk.clamp(0.0, 1.0),
        }
    }

    /// Create an impassable domain cost.
    #[must_use]
    pub fn impassable() -> Self {
        Self {
            multiplier: f32::MAX,
            energy_per_unit: f32::MAX,
            risk: 1.0,
        }
    }

    /// Check if this domain is effectively impassable.
    #[must_use]
    pub fn is_impassable(&self) -> bool {
        self.multiplier >= 1000.0 || self.energy_per_unit >= 1000.0
    }

    /// Calculate total cost for a distance.
    #[must_use]
    pub fn total_cost(&self, distance: f32) -> f32 {
        distance * self.multiplier + self.energy_per_unit * distance
    }
}

/// Agent navigation capabilities across domains.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentCapabilities {
    /// Unique capability set identifier.
    pub id: AgentCapabilityId,
    /// Costs for each supported domain.
    pub domain_costs: BTreeMap<MovementDomain, DomainCost>,
    /// Maximum speed in each domain.
    pub domain_speeds: BTreeMap<MovementDomain, f32>,
    /// Agent size (bounding radius) for collision.
    pub size: f32,
    /// Maximum jump height (for walking/climbing transitions).
    pub max_jump_height: f32,
    /// Maximum fall distance.
    pub max_fall_distance: f32,
    /// Whether agent can cling to walls.
    pub can_wall_cling: bool,
    /// Whether agent can hover in place (flying/zero-G).
    pub can_hover: bool,
    /// Preferred domains in priority order.
    pub preferred_domains: Vec<MovementDomain>,
}

/// Unique identifier for an agent capability set.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AgentCapabilityId(pub String);

impl AgentCapabilityId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for AgentCapabilities {
    fn default() -> Self {
        let mut domain_costs = BTreeMap::new();
        domain_costs.insert(MovementDomain::Walking, DomainCost::default());

        let mut domain_speeds = BTreeMap::new();
        domain_speeds.insert(MovementDomain::Walking, 5.0);

        Self {
            id: AgentCapabilityId::new("default"),
            domain_costs,
            domain_speeds,
            size: 0.5,
            max_jump_height: 1.0,
            max_fall_distance: 3.0,
            can_wall_cling: false,
            can_hover: false,
            preferred_domains: vec![MovementDomain::Walking],
        }
    }
}

impl AgentCapabilities {
    /// Create new agent capabilities.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: AgentCapabilityId::new(id),
            ..Default::default()
        }
    }

    /// Add support for a movement domain.
    #[must_use]
    pub fn with_domain(mut self, domain: MovementDomain, cost: DomainCost, speed: f32) -> Self {
        self.domain_costs.insert(domain, cost);
        self.domain_speeds.insert(domain, speed.max(0.1));
        self
    }

    /// Set agent size.
    #[must_use]
    pub fn with_size(mut self, size: f32) -> Self {
        self.size = size.max(0.1);
        self
    }

    /// Set jump and fall limits.
    #[must_use]
    pub fn with_vertical_limits(mut self, jump: f32, fall: f32) -> Self {
        self.max_jump_height = jump.max(0.0);
        self.max_fall_distance = fall.max(0.0);
        self
    }

    /// Enable wall clinging.
    #[must_use]
    pub fn with_wall_cling(mut self) -> Self {
        self.can_wall_cling = true;
        self
    }

    /// Enable hovering.
    #[must_use]
    pub fn with_hover(mut self) -> Self {
        self.can_hover = true;
        self
    }

    /// Set preferred domains.
    #[must_use]
    pub fn with_preferences(mut self, domains: Vec<MovementDomain>) -> Self {
        self.preferred_domains = domains;
        self
    }

    /// Check if agent can use a domain.
    #[must_use]
    pub fn can_use_domain(&self, domain: MovementDomain) -> bool {
        self.domain_costs
            .get(&domain)
            .is_some_and(|c| !c.is_impassable())
    }

    /// Get cost for a domain (returns impassable if not supported).
    #[must_use]
    pub fn cost_for_domain(&self, domain: MovementDomain) -> DomainCost {
        self.domain_costs
            .get(&domain)
            .copied()
            .unwrap_or_else(DomainCost::impassable)
    }

    /// Get speed for a domain.
    #[must_use]
    pub fn speed_for_domain(&self, domain: MovementDomain) -> f32 {
        self.domain_speeds.get(&domain).copied().unwrap_or(0.0)
    }

    /// Get all supported domains.
    pub fn supported_domains(&self) -> impl Iterator<Item = MovementDomain> + '_ {
        self.domain_costs
            .iter()
            .filter(|(_, c)| !c.is_impassable())
            .map(|(d, _)| *d)
    }
}

/// Preset agent capability configurations.
pub mod capability_presets {
    use super::{AgentCapabilities, DomainCost, MovementDomain};

    /// Ground-based humanoid.
    #[must_use]
    pub fn humanoid() -> AgentCapabilities {
        AgentCapabilities::new("humanoid")
            .with_domain(MovementDomain::Walking, DomainCost::default(), 5.0)
            .with_domain(
                MovementDomain::Swimming,
                DomainCost::new(1.5, 2.0, 0.1),
                2.0,
            )
            .with_domain(
                MovementDomain::Climbing,
                DomainCost::new(2.0, 3.0, 0.2),
                1.5,
            )
            .with_vertical_limits(1.5, 4.0)
            .with_preferences(vec![
                MovementDomain::Walking,
                MovementDomain::Climbing,
                MovementDomain::Swimming,
            ])
    }

    /// Flying creature.
    #[must_use]
    pub fn flying() -> AgentCapabilities {
        AgentCapabilities::new("flying")
            .with_domain(MovementDomain::Walking, DomainCost::new(1.5, 1.0, 0.0), 3.0)
            .with_domain(MovementDomain::Flying, DomainCost::new(0.8, 0.5, 0.0), 10.0)
            .with_hover()
            .with_vertical_limits(0.0, f32::MAX)
            .with_preferences(vec![MovementDomain::Flying, MovementDomain::Walking])
    }

    /// Aquatic creature.
    #[must_use]
    pub fn aquatic() -> AgentCapabilities {
        AgentCapabilities::new("aquatic")
            .with_domain(
                MovementDomain::Swimming,
                DomainCost::new(0.5, 0.3, 0.0),
                8.0,
            )
            .with_domain(MovementDomain::Walking, DomainCost::new(3.0, 5.0, 0.3), 1.0)
            .with_vertical_limits(0.0, 1.0)
            .with_preferences(vec![MovementDomain::Swimming, MovementDomain::Walking])
    }

    /// Spider/wall-crawler.
    #[must_use]
    pub fn crawler() -> AgentCapabilities {
        AgentCapabilities::new("crawler")
            .with_domain(MovementDomain::Walking, DomainCost::default(), 4.0)
            .with_domain(
                MovementDomain::Climbing,
                DomainCost::new(0.8, 0.5, 0.0),
                4.0,
            )
            .with_wall_cling()
            .with_vertical_limits(2.0, 10.0)
            .with_preferences(vec![MovementDomain::Walking, MovementDomain::Climbing])
    }

    /// Zero-G maneuvering unit.
    #[must_use]
    pub fn zero_g() -> AgentCapabilities {
        AgentCapabilities::new("zero_g")
            .with_domain(MovementDomain::ZeroG, DomainCost::new(1.0, 0.2, 0.0), 6.0)
            .with_domain(MovementDomain::Walking, DomainCost::new(0.8, 1.0, 0.0), 4.0)
            .with_hover()
            .with_vertical_limits(f32::MAX, f32::MAX)
            .with_preferences(vec![MovementDomain::ZeroG, MovementDomain::Walking])
    }
}

/// Annotation for a navigation node.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct NodeAnnotation {
    /// Position in world space.
    pub position: NavPosition,
    /// Primary movement domain for this node.
    pub domain: MovementDomain,
    /// Alternative domains available at this node.
    pub alt_domains: BTreeSet<MovementDomain>,
    /// Whether this is a domain transition point.
    pub is_transition: bool,
    /// Dynamic frame this node belongs to (if any).
    pub frame_id: Option<FrameId>,
    /// Surface type for traversal.
    pub surface: SurfaceType,
    /// Region this node belongs to.
    pub region_id: Option<NavRegionId>,
    /// Custom flags.
    pub flags: u32,
}

impl NodeAnnotation {
    /// Create a new node annotation.
    #[must_use]
    pub fn new(position: NavPosition, domain: MovementDomain) -> Self {
        Self {
            position,
            domain,
            ..Default::default()
        }
    }

    /// Add an alternative domain.
    #[must_use]
    pub fn with_alt_domain(mut self, domain: MovementDomain) -> Self {
        self.alt_domains.insert(domain);
        self
    }

    /// Mark as transition point.
    #[must_use]
    pub fn as_transition(mut self) -> Self {
        self.is_transition = true;
        self
    }

    /// Set dynamic frame.
    #[must_use]
    pub fn on_frame(mut self, frame: FrameId) -> Self {
        self.frame_id = Some(frame);
        self
    }

    /// Set surface type.
    #[must_use]
    pub fn with_surface(mut self, surface: SurfaceType) -> Self {
        self.surface = surface;
        self
    }

    /// Set region.
    #[must_use]
    pub fn in_region(mut self, region: NavRegionId) -> Self {
        self.region_id = Some(region);
        self
    }

    /// Check if agent can use this node.
    #[must_use]
    pub fn is_usable_by(&self, caps: &AgentCapabilities) -> bool {
        if caps.can_use_domain(self.domain) {
            return true;
        }
        self.alt_domains.iter().any(|d| caps.can_use_domain(*d))
    }

    /// Get best domain for agent.
    #[must_use]
    pub fn best_domain_for(&self, caps: &AgentCapabilities) -> Option<MovementDomain> {
        for pref in &caps.preferred_domains {
            if *pref == self.domain && caps.can_use_domain(self.domain) {
                return Some(self.domain);
            }
            if self.alt_domains.contains(pref) && caps.can_use_domain(*pref) {
                return Some(*pref);
            }
        }
        if caps.can_use_domain(self.domain) {
            return Some(self.domain);
        }
        self.alt_domains
            .iter()
            .find(|d| caps.can_use_domain(**d))
            .copied()
    }
}

/// Surface type affecting traversal.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SurfaceType {
    /// Normal solid ground.
    #[default]
    Solid,
    /// Slippery surface (ice, wet).
    Slippery,
    /// Rough/slow terrain.
    Rough,
    /// Liquid surface (water top).
    Liquid,
    /// Ladder or climbable.
    Climbable,
    /// Moving platform.
    Moving,
    /// Dangerous/damaging.
    Hazardous,
    /// Passable air/void.
    Air,
}

impl SurfaceType {
    /// Get friction coefficient for this surface.
    #[must_use]
    pub fn friction(&self) -> f32 {
        match self {
            Self::Solid | Self::Moving | Self::Hazardous => 1.0,
            Self::Slippery => 0.2,
            Self::Rough => 1.5,
            Self::Liquid => 0.5,
            Self::Climbable => 0.8,
            Self::Air => 0.0,
        }
    }

    /// Get speed modifier for this surface.
    #[must_use]
    pub fn speed_modifier(&self) -> f32 {
        match self {
            Self::Solid | Self::Moving | Self::Air => 1.0,
            Self::Slippery => 1.2,
            Self::Rough => 0.6,
            Self::Liquid | Self::Hazardous => 0.8,
            Self::Climbable => 0.5,
        }
    }
}

/// Navigation position with optional frame reference.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct NavPosition {
    /// X coordinate.
    pub x: f32,
    /// Y coordinate (vertical).
    pub y: f32,
    /// Z coordinate.
    pub z: f32,
    /// Frame-local flag (true if coordinates are frame-relative).
    pub frame_local: bool,
}

impl NavPosition {
    /// Create a world-space position.
    #[must_use]
    pub fn world(x: f32, y: f32, z: f32) -> Self {
        Self {
            x,
            y,
            z,
            frame_local: false,
        }
    }

    /// Create a frame-local position.
    #[must_use]
    pub fn local(x: f32, y: f32, z: f32) -> Self {
        Self {
            x,
            y,
            z,
            frame_local: true,
        }
    }

    /// Convert to grid position.
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "intentional grid conversion"
    )]
    pub fn to_grid(&self) -> GridPos {
        GridPos::new(
            self.x.floor() as i32,
            self.y.floor() as i32,
            self.z.floor() as i32,
        )
    }

    /// Distance to another position.
    #[must_use]
    pub fn distance(&self, other: &NavPosition) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    /// Distance squared.
    #[must_use]
    pub fn distance_squared(&self, other: &NavPosition) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        dx * dx + dy * dy + dz * dz
    }
}

/// Annotation for a navigation edge.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EdgeAnnotation {
    /// Movement domain required for this edge.
    pub domain: MovementDomain,
    /// Base traversal cost.
    pub base_cost: f32,
    /// Additional cost modifiers.
    pub cost_modifiers: Vec<CostModifier>,
    /// Whether edge is bidirectional.
    pub bidirectional: bool,
    /// Transition type (if connecting different domains).
    pub transition: Option<DomainTransition>,
    /// Edge crosses frame boundary.
    pub frame_crossing: Option<FrameCrossing>,
    /// Required capabilities to use this edge.
    pub required_caps: EdgeRequirements,
}

impl Default for EdgeAnnotation {
    fn default() -> Self {
        Self {
            domain: MovementDomain::Walking,
            base_cost: 1.0,
            cost_modifiers: Vec::new(),
            bidirectional: true,
            transition: None,
            frame_crossing: None,
            required_caps: EdgeRequirements::default(),
        }
    }
}

impl EdgeAnnotation {
    /// Create a new edge annotation.
    #[must_use]
    pub fn new(domain: MovementDomain, cost: f32) -> Self {
        Self {
            domain,
            base_cost: cost.max(0.1),
            ..Default::default()
        }
    }

    /// Make edge one-way.
    #[must_use]
    pub fn one_way(mut self) -> Self {
        self.bidirectional = false;
        self
    }

    /// Add a cost modifier.
    #[must_use]
    pub fn with_modifier(mut self, modifier: CostModifier) -> Self {
        self.cost_modifiers.push(modifier);
        self
    }

    /// Set domain transition.
    #[must_use]
    pub fn with_transition(mut self, from: MovementDomain, to: MovementDomain) -> Self {
        self.transition = Some(DomainTransition { from, to });
        self
    }

    /// Set frame crossing.
    #[must_use]
    pub fn crossing_frames(mut self, from: Option<FrameId>, to: Option<FrameId>) -> Self {
        self.frame_crossing = Some(FrameCrossing {
            from_frame: from,
            to_frame: to,
        });
        self
    }

    /// Set requirements.
    #[must_use]
    pub fn with_requirements(mut self, reqs: EdgeRequirements) -> Self {
        self.required_caps = reqs;
        self
    }

    /// Calculate total cost for an agent.
    #[must_use]
    pub fn cost_for(&self, caps: &AgentCapabilities) -> f32 {
        let domain_cost = caps.cost_for_domain(self.domain);
        if domain_cost.is_impassable() {
            return f32::MAX;
        }

        let mut cost = self.base_cost * domain_cost.multiplier;

        for modifier in &self.cost_modifiers {
            cost = modifier.apply(cost, caps);
        }

        if let Some(transition) = &self.transition {
            cost += transition.cost(caps);
        }

        cost
    }

    /// Check if agent can traverse this edge.
    #[must_use]
    pub fn is_traversable_by(&self, caps: &AgentCapabilities) -> bool {
        if !caps.can_use_domain(self.domain) {
            return false;
        }
        self.required_caps.satisfied_by(caps)
    }
}

/// Cost modifier for edges.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CostModifier {
    /// Flat cost addition.
    Flat(f32),
    /// Multiplicative modifier.
    Multiply(f32),
    /// Conditional modifier based on capability.
    Conditional { has_hover: bool, modifier: f32 },
    /// Risk-based modifier.
    Risk(f32),
}

impl CostModifier {
    /// Apply modifier to base cost.
    #[must_use]
    pub fn apply(&self, cost: f32, caps: &AgentCapabilities) -> f32 {
        match self {
            Self::Flat(add) => cost + add,
            Self::Multiply(mul) => cost * mul,
            Self::Conditional {
                has_hover,
                modifier,
            } => {
                if caps.can_hover == *has_hover {
                    cost * modifier
                } else {
                    cost
                }
            }
            Self::Risk(risk) => cost * (1.0 + risk),
        }
    }
}

/// Domain transition descriptor.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct DomainTransition {
    /// Source domain.
    pub from: MovementDomain,
    /// Target domain.
    pub to: MovementDomain,
}

impl DomainTransition {
    /// Calculate transition cost for an agent.
    #[must_use]
    pub fn cost(&self, caps: &AgentCapabilities) -> f32 {
        let from_cost = caps.cost_for_domain(self.from);
        let to_cost = caps.cost_for_domain(self.to);

        if from_cost.is_impassable() || to_cost.is_impassable() {
            return f32::MAX;
        }

        (from_cost.multiplier + to_cost.multiplier) * 0.5
    }
}

/// Frame crossing descriptor.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FrameCrossing {
    /// Source frame (None = world frame).
    pub from_frame: Option<FrameId>,
    /// Destination frame (None = world frame).
    pub to_frame: Option<FrameId>,
}

/// Requirements to traverse an edge.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EdgeRequirements {
    /// Minimum jump height required.
    pub min_jump: Option<f32>,
    /// Minimum fall tolerance required.
    pub min_fall: Option<f32>,
    /// Requires wall cling capability.
    pub needs_wall_cling: bool,
    /// Requires hover capability.
    pub needs_hover: bool,
    /// Maximum agent size allowed.
    pub max_size: Option<f32>,
}

impl EdgeRequirements {
    /// Check if capabilities satisfy requirements.
    #[must_use]
    pub fn satisfied_by(&self, caps: &AgentCapabilities) -> bool {
        if let Some(min_jump) = self.min_jump
            && caps.max_jump_height < min_jump
        {
            return false;
        }
        if let Some(min_fall) = self.min_fall
            && caps.max_fall_distance < min_fall
        {
            return false;
        }
        if self.needs_wall_cling && !caps.can_wall_cling {
            return false;
        }
        if self.needs_hover && !caps.can_hover {
            return false;
        }
        if let Some(max_size) = self.max_size
            && caps.size > max_size
        {
            return false;
        }
        true
    }
}

/// Unique identifier for a dynamic reference frame.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct FrameId(pub String);

impl FrameId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for FrameId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Dynamic reference frame for moving platforms/worlds.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DynamicFrame {
    /// Frame identifier.
    pub id: FrameId,
    /// Current world-space position of frame origin.
    pub position: NavPosition,
    /// Current velocity.
    pub velocity: FrameVelocity,
    /// Frame parent (for nested frames).
    pub parent: Option<FrameId>,
    /// Whether frame is currently active.
    pub active: bool,
    /// Last update tick.
    pub last_update_tick: u64,
    /// Metadata.
    pub metadata: BTreeMap<String, String>,
}

impl DynamicFrame {
    /// Create a new dynamic frame.
    #[must_use]
    pub fn new(id: FrameId, position: NavPosition) -> Self {
        Self {
            id,
            position,
            velocity: FrameVelocity::default(),
            parent: None,
            active: true,
            last_update_tick: 0,
            metadata: BTreeMap::new(),
        }
    }

    /// Set velocity.
    #[must_use]
    pub fn with_velocity(mut self, velocity: FrameVelocity) -> Self {
        self.velocity = velocity;
        self
    }

    /// Set parent frame.
    #[must_use]
    pub fn with_parent(mut self, parent: FrameId) -> Self {
        self.parent = Some(parent);
        self
    }

    /// Update frame state.
    pub fn update(&mut self, position: NavPosition, velocity: FrameVelocity, tick: u64) {
        self.position = position;
        self.velocity = velocity;
        self.last_update_tick = tick;
    }

    /// Convert local position to world position.
    #[must_use]
    pub fn local_to_world(&self, local: NavPosition) -> NavPosition {
        NavPosition::world(
            self.position.x + local.x,
            self.position.y + local.y,
            self.position.z + local.z,
        )
    }

    /// Convert world position to local position.
    #[must_use]
    pub fn world_to_local(&self, world: NavPosition) -> NavPosition {
        NavPosition::local(
            world.x - self.position.x,
            world.y - self.position.y,
            world.z - self.position.z,
        )
    }
}

/// Velocity of a dynamic frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FrameVelocity {
    /// Linear velocity components.
    pub linear_x: f32,
    pub linear_y: f32,
    pub linear_z: f32,
    /// Angular velocity (rotation around Y axis).
    pub angular_y: f32,
}

impl FrameVelocity {
    /// Create linear velocity.
    #[must_use]
    pub fn linear(x: f32, y: f32, z: f32) -> Self {
        Self {
            linear_x: x,
            linear_y: y,
            linear_z: z,
            angular_y: 0.0,
        }
    }

    /// Add angular velocity.
    #[must_use]
    pub fn with_rotation(mut self, angular: f32) -> Self {
        self.angular_y = angular;
        self
    }

    /// Get velocity magnitude.
    #[must_use]
    pub fn speed(&self) -> f32 {
        (self.linear_x * self.linear_x
            + self.linear_y * self.linear_y
            + self.linear_z * self.linear_z)
            .sqrt()
    }

    /// Check if frame is moving.
    #[must_use]
    pub fn is_moving(&self) -> bool {
        self.speed() > 0.001 || self.angular_y.abs() > 0.001
    }
}

/// Unique identifier for a navigation region.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct NavRegionId(pub String);

impl NavRegionId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Route request for navigation planning.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RouteRequest {
    /// Request identifier.
    pub id: RouteRequestId,
    /// Start position.
    pub start: NavPosition,
    /// Goal position.
    pub goal: NavPosition,
    /// Agent capabilities.
    pub capabilities: AgentCapabilities,
    /// Preferred domains (overrides capability defaults).
    pub domain_preferences: Option<Vec<MovementDomain>>,
    /// Maximum route cost.
    pub max_cost: Option<f32>,
    /// Maximum route length.
    pub max_length: Option<u32>,
    /// Domains to avoid.
    pub avoid_domains: BTreeSet<MovementDomain>,
    /// Frames to avoid.
    pub avoid_frames: BTreeSet<FrameId>,
    /// Regions to avoid.
    pub avoid_regions: BTreeSet<NavRegionId>,
    /// Allow partial paths.
    pub allow_partial: bool,
    /// Request tick.
    pub requested_tick: u64,
}

/// Unique identifier for a route request.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RouteRequestId(pub u64);

impl RouteRequestId {
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

impl RouteRequest {
    /// Create a new route request.
    #[must_use]
    pub fn new(
        id: u64,
        start: NavPosition,
        goal: NavPosition,
        capabilities: AgentCapabilities,
        tick: u64,
    ) -> Self {
        Self {
            id: RouteRequestId::new(id),
            start,
            goal,
            capabilities,
            domain_preferences: None,
            max_cost: None,
            max_length: None,
            avoid_domains: BTreeSet::new(),
            avoid_frames: BTreeSet::new(),
            avoid_regions: BTreeSet::new(),
            allow_partial: false,
            requested_tick: tick,
        }
    }

    /// Set domain preferences.
    #[must_use]
    pub fn with_preferences(mut self, domains: Vec<MovementDomain>) -> Self {
        self.domain_preferences = Some(domains);
        self
    }

    /// Set maximum cost.
    #[must_use]
    pub fn with_max_cost(mut self, cost: f32) -> Self {
        self.max_cost = Some(cost);
        self
    }

    /// Set maximum length.
    #[must_use]
    pub fn with_max_length(mut self, length: u32) -> Self {
        self.max_length = Some(length);
        self
    }

    /// Add domain to avoid.
    pub fn avoid_domain(&mut self, domain: MovementDomain) {
        self.avoid_domains.insert(domain);
    }

    /// Add frame to avoid.
    pub fn avoid_frame(&mut self, frame: FrameId) {
        self.avoid_frames.insert(frame);
    }

    /// Add region to avoid.
    pub fn avoid_region(&mut self, region: NavRegionId) {
        self.avoid_regions.insert(region);
    }

    /// Allow partial paths.
    #[must_use]
    pub fn allowing_partial(mut self) -> Self {
        self.allow_partial = true;
        self
    }

    /// Get effective domain preferences.
    #[must_use]
    pub fn effective_preferences(&self) -> &[MovementDomain] {
        self.domain_preferences
            .as_deref()
            .unwrap_or(&self.capabilities.preferred_domains)
    }
}

/// Result of a route planning operation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RouteResult {
    /// Route found successfully.
    Found(RoutePlan),
    /// Partial route found (didn't reach goal).
    Partial(RoutePlan),
    /// No route exists.
    NotFound(RouteFailure),
    /// Planning exceeded limits.
    LimitExceeded(RouteLimitExceeded),
}

impl RouteResult {
    /// Check if route was found.
    #[must_use]
    pub fn is_found(&self) -> bool {
        matches!(self, Self::Found(_))
    }

    /// Check if any path was found (including partial).
    #[must_use]
    pub fn has_path(&self) -> bool {
        matches!(self, Self::Found(_) | Self::Partial(_))
    }

    /// Get the plan if available.
    #[must_use]
    pub fn plan(&self) -> Option<&RoutePlan> {
        match self {
            Self::Found(plan) | Self::Partial(plan) => Some(plan),
            _ => None,
        }
    }

    /// Extract the plan.
    #[must_use]
    pub fn into_plan(self) -> Option<RoutePlan> {
        match self {
            Self::Found(plan) | Self::Partial(plan) => Some(plan),
            _ => None,
        }
    }
}

/// Reason for route planning failure.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RouteFailure {
    /// Start position is unreachable.
    InvalidStart,
    /// Goal position is unreachable.
    InvalidGoal,
    /// No path connects start to goal.
    NoPath,
    /// Required capabilities not available.
    CapabilityMismatch,
    /// All paths go through avoided areas.
    AllPathsAvoided,
    /// Region data not loaded.
    RegionNotLoaded(NavRegionId),
}

/// Which limit was exceeded during planning.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RouteLimitExceeded {
    /// Which limit was hit.
    pub limit_type: RouteLimitType,
    /// Partial plan if any.
    pub partial_plan: Option<RoutePlan>,
}

/// Type of limit exceeded.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RouteLimitType {
    /// Iteration/search limit.
    Iterations,
    /// Cost limit.
    Cost,
    /// Length limit.
    Length,
}

/// A complete route plan.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoutePlan {
    /// Request this plan fulfills.
    pub request_id: RouteRequestId,
    /// Ordered waypoints.
    pub waypoints: Vec<RouteWaypoint>,
    /// Total estimated cost.
    pub total_cost: f32,
    /// Estimated travel time (ticks).
    pub estimated_ticks: u64,
    /// Domains used in this route.
    pub domains_used: BTreeSet<MovementDomain>,
    /// Frames traversed.
    pub frames_traversed: BTreeSet<FrameId>,
    /// Regions traversed.
    pub regions_traversed: BTreeSet<NavRegionId>,
    /// Whether this is a partial plan.
    pub is_partial: bool,
    /// Tick when plan was created.
    pub created_tick: u64,
    /// Tick when plan expires.
    pub expires_tick: Option<u64>,
}

impl RoutePlan {
    /// Create a new route plan.
    #[must_use]
    pub fn new(request_id: RouteRequestId, tick: u64) -> Self {
        Self {
            request_id,
            waypoints: Vec::new(),
            total_cost: 0.0,
            estimated_ticks: 0,
            domains_used: BTreeSet::new(),
            frames_traversed: BTreeSet::new(),
            regions_traversed: BTreeSet::new(),
            is_partial: false,
            created_tick: tick,
            expires_tick: None,
        }
    }

    /// Add a waypoint.
    pub fn add_waypoint(&mut self, waypoint: RouteWaypoint) {
        self.domains_used.insert(waypoint.domain);
        if let Some(frame) = &waypoint.frame_id {
            self.frames_traversed.insert(frame.clone());
        }
        if let Some(region) = &waypoint.region_id {
            self.regions_traversed.insert(region.clone());
        }
        self.waypoints.push(waypoint);
    }

    /// Get waypoint count.
    #[must_use]
    pub fn waypoint_count(&self) -> usize {
        self.waypoints.len()
    }

    /// Check if plan is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.waypoints.is_empty()
    }

    /// Get start position.
    #[must_use]
    pub fn start(&self) -> Option<&NavPosition> {
        self.waypoints.first().map(|w| &w.position)
    }

    /// Get goal position.
    #[must_use]
    pub fn goal(&self) -> Option<&NavPosition> {
        self.waypoints.last().map(|w| &w.position)
    }

    /// Check if plan is expired.
    #[must_use]
    pub fn is_expired(&self, current_tick: u64) -> bool {
        self.expires_tick.is_some_and(|exp| current_tick > exp)
    }

    /// Set expiration.
    #[must_use]
    pub fn expires_at(mut self, tick: u64) -> Self {
        self.expires_tick = Some(tick);
        self
    }

    /// Mark as partial.
    #[must_use]
    pub fn as_partial(mut self) -> Self {
        self.is_partial = true;
        self
    }
}

/// A waypoint in a route plan.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RouteWaypoint {
    /// Position of this waypoint.
    pub position: NavPosition,
    /// Movement domain at this waypoint.
    pub domain: MovementDomain,
    /// Dynamic frame (if any).
    pub frame_id: Option<FrameId>,
    /// Region.
    pub region_id: Option<NavRegionId>,
    /// Cost to reach this waypoint from start.
    pub cumulative_cost: f32,
    /// Steering hint for approaching this waypoint.
    pub steering_hint: Option<SteeringHint>,
    /// Whether this is a domain transition point.
    pub is_transition: bool,
}

impl RouteWaypoint {
    /// Create a new waypoint.
    #[must_use]
    pub fn new(position: NavPosition, domain: MovementDomain) -> Self {
        Self {
            position,
            domain,
            frame_id: None,
            region_id: None,
            cumulative_cost: 0.0,
            steering_hint: None,
            is_transition: false,
        }
    }

    /// Set frame.
    #[must_use]
    pub fn on_frame(mut self, frame: FrameId) -> Self {
        self.frame_id = Some(frame);
        self
    }

    /// Set region.
    #[must_use]
    pub fn in_region(mut self, region: NavRegionId) -> Self {
        self.region_id = Some(region);
        self
    }

    /// Set cumulative cost.
    #[must_use]
    pub fn with_cost(mut self, cost: f32) -> Self {
        self.cumulative_cost = cost;
        self
    }

    /// Set steering hint.
    #[must_use]
    pub fn with_steering(mut self, hint: SteeringHint) -> Self {
        self.steering_hint = Some(hint);
        self
    }

    /// Mark as transition.
    #[must_use]
    pub fn as_transition(mut self) -> Self {
        self.is_transition = true;
        self
    }
}

/// Steering hint for a waypoint.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct SteeringHint {
    /// Suggested approach direction.
    pub direction: SteeringDirection,
    /// Suggested speed (0.0-1.0 of max).
    pub speed_factor: f32,
    /// Whether to use precise steering.
    pub precise: bool,
    /// Whether to anticipate next waypoint.
    pub anticipate: bool,
}

impl Default for SteeringHint {
    fn default() -> Self {
        Self {
            direction: SteeringDirection::Forward,
            speed_factor: 1.0,
            precise: false,
            anticipate: true,
        }
    }
}

/// Steering approach direction.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SteeringDirection {
    /// Approach directly.
    #[default]
    Forward,
    /// Strafe left while approaching.
    StrafeLeft,
    /// Strafe right while approaching.
    StrafeRight,
    /// Move backward toward waypoint.
    Backward,
    /// Ascend while approaching.
    Ascend,
    /// Descend while approaching.
    Descend,
}

/// Reason for route invalidation requiring replan.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReplanReason {
    /// Route has expired.
    Expired,
    /// Agent deviated too far from route.
    Deviation,
    /// Obstacle appeared on route.
    ObstacleDetected,
    /// Dynamic frame moved unexpectedly.
    FrameMoved(FrameId),
    /// Frame became inactive.
    FrameDeactivated(FrameId),
    /// Region became stale/unloaded.
    RegionStale(NavRegionId),
    /// Domain transition failed.
    TransitionFailed(MovementDomain),
    /// Goal changed or moved.
    GoalChanged,
    /// Higher priority goal available.
    GoalPreempted,
    /// External invalidation request.
    ExternalRequest,
}

/// Summary of navigation state for unloaded regions.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NavRegionSummary {
    /// Region identifier.
    pub region_id: NavRegionId,
    /// Available domains in this region.
    pub available_domains: BTreeSet<MovementDomain>,
    /// Active dynamic frames.
    pub active_frames: BTreeSet<FrameId>,
    /// Connectivity to adjacent regions.
    pub connections: BTreeMap<NavRegionId, RegionConnection>,
    /// Approximate center position.
    pub center: NavPosition,
    /// Approximate bounding radius.
    pub radius: f32,
    /// Whether region data is stale.
    pub is_stale: bool,
    /// Last update tick.
    pub last_update_tick: u64,
    /// Staleness age (ticks since update).
    pub staleness_age: u64,
}

impl NavRegionSummary {
    /// Create a new region summary.
    #[must_use]
    pub fn new(region_id: NavRegionId, center: NavPosition, tick: u64) -> Self {
        Self {
            region_id,
            available_domains: BTreeSet::new(),
            active_frames: BTreeSet::new(),
            connections: BTreeMap::new(),
            center,
            radius: 0.0,
            is_stale: false,
            last_update_tick: tick,
            staleness_age: 0,
        }
    }

    /// Add available domain.
    pub fn add_domain(&mut self, domain: MovementDomain) {
        self.available_domains.insert(domain);
    }

    /// Add active frame.
    pub fn add_frame(&mut self, frame: FrameId) {
        self.active_frames.insert(frame);
    }

    /// Add connection to another region.
    pub fn add_connection(&mut self, region: NavRegionId, connection: RegionConnection) {
        self.connections.insert(region, connection);
    }

    /// Check if agent can traverse this region.
    #[must_use]
    pub fn is_traversable_by(&self, caps: &AgentCapabilities) -> bool {
        self.available_domains
            .iter()
            .any(|d| caps.can_use_domain(*d))
    }

    /// Update staleness.
    pub fn update_staleness(&mut self, current_tick: u64, max_staleness: u64) {
        self.staleness_age = current_tick.saturating_sub(self.last_update_tick);
        self.is_stale = self.staleness_age > max_staleness;
    }

    /// Refresh the summary.
    pub fn refresh(&mut self, tick: u64) {
        self.last_update_tick = tick;
        self.staleness_age = 0;
        self.is_stale = false;
    }
}

/// Connection between navigation regions.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegionConnection {
    /// Domain required to cross.
    pub domain: MovementDomain,
    /// Estimated traversal cost.
    pub cost: f32,
    /// Whether connection is bidirectional.
    pub bidirectional: bool,
    /// Frame crossing (if any).
    pub frame_crossing: Option<FrameCrossing>,
}

impl RegionConnection {
    /// Create a new connection.
    #[must_use]
    pub fn new(domain: MovementDomain, cost: f32) -> Self {
        Self {
            domain,
            cost,
            bidirectional: true,
            frame_crossing: None,
        }
    }

    /// Make one-way.
    #[must_use]
    pub fn one_way(mut self) -> Self {
        self.bidirectional = false;
        self
    }

    /// Set frame crossing.
    #[must_use]
    pub fn with_frame_crossing(mut self, crossing: FrameCrossing) -> Self {
        self.frame_crossing = Some(crossing);
        self
    }
}

/// Multi-domain walkable world adapter for A* integration.
pub struct MultiDomainWorld<'a> {
    capabilities: &'a AgentCapabilities,
    node_annotations: &'a BTreeMap<GridPos, NodeAnnotation>,
    edge_annotations: &'a BTreeMap<(GridPos, GridPos), EdgeAnnotation>,
    avoid_domains: &'a BTreeSet<MovementDomain>,
    avoid_frames: &'a BTreeSet<FrameId>,
    avoid_regions: &'a BTreeSet<NavRegionId>,
}

impl<'a> MultiDomainWorld<'a> {
    /// Create a new multi-domain world adapter.
    #[must_use]
    pub fn new(
        capabilities: &'a AgentCapabilities,
        node_annotations: &'a BTreeMap<GridPos, NodeAnnotation>,
        edge_annotations: &'a BTreeMap<(GridPos, GridPos), EdgeAnnotation>,
        avoid_domains: &'a BTreeSet<MovementDomain>,
        avoid_frames: &'a BTreeSet<FrameId>,
        avoid_regions: &'a BTreeSet<NavRegionId>,
    ) -> Self {
        Self {
            capabilities,
            node_annotations,
            edge_annotations,
            avoid_domains,
            avoid_frames,
            avoid_regions,
        }
    }

    fn is_node_allowed(&self, annotation: &NodeAnnotation) -> bool {
        if self.avoid_domains.contains(&annotation.domain) {
            return false;
        }
        if let Some(frame) = &annotation.frame_id
            && self.avoid_frames.contains(frame)
        {
            return false;
        }
        if let Some(region) = &annotation.region_id
            && self.avoid_regions.contains(region)
        {
            return false;
        }
        true
    }
}

impl Walkable for MultiDomainWorld<'_> {
    fn is_walkable(&self, pos: &GridPos) -> bool {
        if let Some(annotation) = self.node_annotations.get(pos) {
            if !self.is_node_allowed(annotation) {
                return false;
            }
            annotation.is_usable_by(self.capabilities)
        } else {
            self.capabilities.can_use_domain(MovementDomain::Walking)
        }
    }

    fn movement_cost(&self, from: &GridPos, to: &GridPos) -> f32 {
        if let Some(edge) = self.edge_annotations.get(&(*from, *to)) {
            if !edge.is_traversable_by(self.capabilities) {
                return f32::MAX;
            }
            edge.cost_for(self.capabilities)
        } else if let Some(to_node) = self.node_annotations.get(to) {
            if let Some(domain) = to_node.best_domain_for(self.capabilities) {
                let cost = self.capabilities.cost_for_domain(domain);
                cost.total_cost(1.0)
            } else {
                f32::MAX
            }
        } else {
            let cost = self.capabilities.cost_for_domain(MovementDomain::Walking);
            cost.total_cost(1.0)
        }
    }
}

/// Navigator for multi-domain pathfinding.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Navigator {
    /// Node annotations.
    node_annotations: BTreeMap<GridPos, NodeAnnotation>,
    /// Edge annotations.
    edge_annotations: BTreeMap<(GridPos, GridPos), EdgeAnnotation>,
    /// Dynamic frames.
    frames: BTreeMap<FrameId, DynamicFrame>,
    /// Region summaries.
    region_summaries: BTreeMap<NavRegionId, NavRegionSummary>,
    /// Next request ID.
    next_request_id: u64,
    /// Current tick.
    current_tick: u64,
}

impl Navigator {
    /// Create a new navigator.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set current tick.
    pub fn set_tick(&mut self, tick: u64) {
        self.current_tick = tick;
    }

    /// Get current tick.
    #[must_use]
    pub fn tick(&self) -> u64 {
        self.current_tick
    }

    /// Add or update a node annotation.
    pub fn set_node(&mut self, pos: GridPos, annotation: NodeAnnotation) {
        self.node_annotations.insert(pos, annotation);
    }

    /// Remove a node annotation.
    pub fn remove_node(&mut self, pos: &GridPos) -> Option<NodeAnnotation> {
        self.node_annotations.remove(pos)
    }

    /// Get a node annotation.
    #[must_use]
    pub fn get_node(&self, pos: &GridPos) -> Option<&NodeAnnotation> {
        self.node_annotations.get(pos)
    }

    /// Add or update an edge annotation.
    pub fn set_edge(&mut self, from: GridPos, to: GridPos, annotation: EdgeAnnotation) {
        self.edge_annotations.insert((from, to), annotation.clone());
        if annotation.bidirectional {
            let mut reverse = annotation;
            reverse.bidirectional = true;
            if let Some(transition) = &mut reverse.transition {
                std::mem::swap(&mut transition.from, &mut transition.to);
            }
            self.edge_annotations.insert((to, from), reverse);
        }
    }

    /// Remove an edge annotation.
    pub fn remove_edge(&mut self, from: &GridPos, to: &GridPos) {
        self.edge_annotations.remove(&(*from, *to));
        self.edge_annotations.remove(&(*to, *from));
    }

    /// Get an edge annotation.
    #[must_use]
    pub fn get_edge(&self, from: &GridPos, to: &GridPos) -> Option<&EdgeAnnotation> {
        self.edge_annotations.get(&(*from, *to))
    }

    /// Register a dynamic frame.
    pub fn register_frame(&mut self, frame: DynamicFrame) {
        self.frames.insert(frame.id.clone(), frame);
    }

    /// Unregister a dynamic frame.
    pub fn unregister_frame(&mut self, id: &FrameId) -> Option<DynamicFrame> {
        self.frames.remove(id)
    }

    /// Get a dynamic frame.
    #[must_use]
    pub fn get_frame(&self, id: &FrameId) -> Option<&DynamicFrame> {
        self.frames.get(id)
    }

    /// Get mutable dynamic frame.
    pub fn get_frame_mut(&mut self, id: &FrameId) -> Option<&mut DynamicFrame> {
        self.frames.get_mut(id)
    }

    /// Update a frame's state.
    pub fn update_frame(&mut self, id: &FrameId, position: NavPosition, velocity: FrameVelocity) {
        if let Some(frame) = self.frames.get_mut(id) {
            frame.update(position, velocity, self.current_tick);
        }
    }

    /// Iterate over frames.
    pub fn frames(&self) -> impl Iterator<Item = &DynamicFrame> {
        self.frames.values()
    }

    /// Register a region summary.
    pub fn register_region(&mut self, summary: NavRegionSummary) {
        self.region_summaries
            .insert(summary.region_id.clone(), summary);
    }

    /// Unregister a region.
    pub fn unregister_region(&mut self, id: &NavRegionId) -> Option<NavRegionSummary> {
        self.region_summaries.remove(id)
    }

    /// Get a region summary.
    #[must_use]
    pub fn get_region(&self, id: &NavRegionId) -> Option<&NavRegionSummary> {
        self.region_summaries.get(id)
    }

    /// Update region staleness.
    pub fn update_region_staleness(&mut self, max_staleness: u64) {
        for summary in self.region_summaries.values_mut() {
            summary.update_staleness(self.current_tick, max_staleness);
        }
    }

    /// Get stale regions.
    pub fn stale_regions(&self) -> impl Iterator<Item = &NavRegionId> {
        self.region_summaries
            .iter()
            .filter(|(_, s)| s.is_stale)
            .map(|(id, _)| id)
    }

    /// Create a route request.
    #[must_use]
    pub fn create_request(
        &mut self,
        start: NavPosition,
        goal: NavPosition,
        capabilities: AgentCapabilities,
    ) -> RouteRequest {
        let id = self.next_request_id;
        self.next_request_id += 1;
        RouteRequest::new(id, start, goal, capabilities, self.current_tick)
    }

    /// Plan a route.
    #[must_use]
    pub fn plan_route(&self, request: &RouteRequest) -> RouteResult {
        let astar_config = self.create_astar_config(request);
        let astar = crate::pathfinding::astar::AStar::new(astar_config);

        let world = MultiDomainWorld::new(
            &request.capabilities,
            &self.node_annotations,
            &self.edge_annotations,
            &request.avoid_domains,
            &request.avoid_frames,
            &request.avoid_regions,
        );

        let start_grid = request.start.to_grid();
        let goal_grid = request.goal.to_grid();

        let path_result = astar.find_path(start_grid, goal_grid, &world);

        match path_result {
            PathResult::Found(grid_path) => {
                let plan = self.build_plan(request, &grid_path);
                RouteResult::Found(plan)
            }
            PathResult::NotFound => RouteResult::NotFound(RouteFailure::NoPath),
            PathResult::InvalidEndpoint => {
                if world.is_walkable(&start_grid) {
                    RouteResult::NotFound(RouteFailure::InvalidGoal)
                } else {
                    RouteResult::NotFound(RouteFailure::InvalidStart)
                }
            }
            PathResult::IterationLimit => RouteResult::LimitExceeded(RouteLimitExceeded {
                limit_type: RouteLimitType::Iterations,
                partial_plan: None,
            }),
            PathResult::PathTooLong => RouteResult::LimitExceeded(RouteLimitExceeded {
                limit_type: RouteLimitType::Length,
                partial_plan: None,
            }),
        }
    }

    #[expect(clippy::unused_self, reason = "method for API consistency")]
    fn create_astar_config(&self, request: &RouteRequest) -> AStarConfig {
        let mut config = AStarConfig::default();

        if let Some(max_len) = request.max_length {
            config.max_path_length = max_len;
        }

        let can_fly = request.capabilities.can_use_domain(MovementDomain::Flying);
        let can_swim = request
            .capabilities
            .can_use_domain(MovementDomain::Swimming);
        let can_climb = request
            .capabilities
            .can_use_domain(MovementDomain::Climbing);

        config.allow_vertical = can_fly || can_swim || can_climb;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "jump/fall heights are bounded to reasonable i32 values"
        )]
        {
            config.max_jump_height = request.capabilities.max_jump_height as i32;
            config.max_fall_distance = request.capabilities.max_fall_distance as i32;
        }

        config
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "grid positions are bounded to reasonable f32 values"
    )]
    fn build_plan(&self, request: &RouteRequest, grid_path: &[GridPos]) -> RoutePlan {
        let mut plan = RoutePlan::new(request.id.clone(), self.current_tick);
        let mut cumulative_cost = 0.0;

        for (i, &grid_pos) in grid_path.iter().enumerate() {
            let position = NavPosition::world(
                grid_pos.x as f32 + 0.5,
                grid_pos.y as f32,
                grid_pos.z as f32 + 0.5,
            );

            let (domain, frame_id, region_id, is_transition) =
                if let Some(annotation) = self.node_annotations.get(&grid_pos) {
                    let domain = annotation
                        .best_domain_for(&request.capabilities)
                        .unwrap_or(MovementDomain::Walking);
                    (
                        domain,
                        annotation.frame_id.clone(),
                        annotation.region_id.clone(),
                        annotation.is_transition,
                    )
                } else {
                    (MovementDomain::Walking, None, None, false)
                };

            if i > 0 {
                let prev = grid_path[i - 1];
                if let Some(edge) = self.edge_annotations.get(&(prev, grid_pos)) {
                    cumulative_cost += edge.cost_for(&request.capabilities);
                } else {
                    let cost = request.capabilities.cost_for_domain(domain);
                    cumulative_cost += cost.total_cost(1.0);
                }
            }

            let steering_hint = self.compute_steering_hint(grid_path, i, &request.capabilities);

            let mut waypoint = RouteWaypoint::new(position, domain).with_cost(cumulative_cost);

            if let Some(frame) = frame_id {
                waypoint = waypoint.on_frame(frame);
            }
            if let Some(region) = region_id {
                waypoint = waypoint.in_region(region);
            }
            if let Some(hint) = steering_hint {
                waypoint = waypoint.with_steering(hint);
            }
            if is_transition {
                waypoint = waypoint.as_transition();
            }

            plan.add_waypoint(waypoint);
        }

        plan.total_cost = cumulative_cost;
        plan.estimated_ticks = self.estimate_travel_ticks(&plan, &request.capabilities);

        plan
    }

    fn compute_steering_hint(
        &self,
        path: &[GridPos],
        index: usize,
        caps: &AgentCapabilities,
    ) -> Option<SteeringHint> {
        if path.len() < 2 {
            return None;
        }

        let current = path[index];
        let domain = self
            .node_annotations
            .get(&current)
            .and_then(|a| a.best_domain_for(caps))
            .unwrap_or(MovementDomain::Walking);

        let mut hint = SteeringHint::default();

        if index > 0 && index < path.len() - 1 {
            let prev = path[index - 1];
            let next = path[index + 1];

            let dy_in = current.y - prev.y;
            let dy_out = next.y - current.y;

            if dy_out > 0 {
                hint.direction = SteeringDirection::Ascend;
            } else if dy_out < 0 {
                hint.direction = SteeringDirection::Descend;
            }

            if (dy_in != 0) != (dy_out != 0) {
                hint.precise = true;
            }
        }

        hint.speed_factor = domain.default_speed_multiplier();

        if let Some(annotation) = self.node_annotations.get(&current) {
            hint.speed_factor *= annotation.surface.speed_modifier();
            if annotation.is_transition {
                hint.precise = true;
                hint.anticipate = false;
            }
        }

        Some(hint)
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "tick estimation bounds are reasonable"
    )]
    fn estimate_travel_ticks(&self, plan: &RoutePlan, caps: &AgentCapabilities) -> u64 {
        if plan.waypoints.len() < 2 {
            return 0;
        }

        let mut total_ticks = 0u64;

        for window in plan.waypoints.windows(2) {
            let from = &window[0];
            let to = &window[1];

            let distance = from.position.distance(&to.position);
            let speed = caps.speed_for_domain(to.domain);
            let surface_mod = self
                .node_annotations
                .get(&to.position.to_grid())
                .map_or(1.0, |a| a.surface.speed_modifier());

            let effective_speed = (speed * surface_mod).max(0.1);
            let ticks = (distance / effective_speed * 20.0) as u64;
            total_ticks += ticks.max(1);
        }

        total_ticks
    }

    /// Check if a plan needs replanning.
    #[must_use]
    pub fn check_replan(
        &self,
        plan: &RoutePlan,
        current_pos: &NavPosition,
    ) -> Option<ReplanReason> {
        if plan.is_expired(self.current_tick) {
            return Some(ReplanReason::Expired);
        }

        for frame_id in &plan.frames_traversed {
            if let Some(frame) = self.frames.get(frame_id) {
                if !frame.active {
                    return Some(ReplanReason::FrameDeactivated(frame_id.clone()));
                }
            } else {
                return Some(ReplanReason::FrameDeactivated(frame_id.clone()));
            }
        }

        for region_id in &plan.regions_traversed {
            if let Some(summary) = self.region_summaries.get(region_id)
                && summary.is_stale
            {
                return Some(ReplanReason::RegionStale(region_id.clone()));
            }
        }

        if let Some(start) = plan.start() {
            let deviation = current_pos.distance(start);
            if deviation > 10.0 {
                return Some(ReplanReason::Deviation);
            }
        }

        None
    }

    /// Get node count.
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.node_annotations.len()
    }

    /// Get edge count.
    #[must_use]
    pub fn edge_count(&self) -> usize {
        self.edge_annotations.len()
    }

    /// Get frame count.
    #[must_use]
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    /// Get region count.
    #[must_use]
    pub fn region_count(&self) -> usize {
        self.region_summaries.len()
    }
}

#[cfg(test)]
#[expect(clippy::cast_precision_loss, reason = "test values fit in f32")]
mod tests {
    use super::*;

    #[test]
    fn test_movement_domain() {
        assert!(MovementDomain::Flying.is_continuous());
        assert!(!MovementDomain::Walking.is_continuous());
        assert!(MovementDomain::Walking.has_gravity());
        assert!(!MovementDomain::ZeroG.has_gravity());
    }

    #[test]
    fn test_domain_cost() {
        let cost = DomainCost::new(1.5, 2.0, 0.3);
        assert!((cost.multiplier - 1.5).abs() < f32::EPSILON);
        assert!(!cost.is_impassable());

        let impassable = DomainCost::impassable();
        assert!(impassable.is_impassable());
    }

    #[test]
    fn test_agent_capabilities() {
        let caps = AgentCapabilities::new("test")
            .with_domain(MovementDomain::Walking, DomainCost::default(), 5.0)
            .with_domain(
                MovementDomain::Swimming,
                DomainCost::new(1.5, 2.0, 0.1),
                3.0,
            )
            .with_vertical_limits(2.0, 5.0);

        assert!(caps.can_use_domain(MovementDomain::Walking));
        assert!(caps.can_use_domain(MovementDomain::Swimming));
        assert!(!caps.can_use_domain(MovementDomain::Flying));

        assert!((caps.speed_for_domain(MovementDomain::Walking) - 5.0).abs() < f32::EPSILON);
        assert!((caps.speed_for_domain(MovementDomain::Flying)).abs() < f32::EPSILON);
    }

    #[test]
    fn test_capability_presets() {
        let humanoid = capability_presets::humanoid();
        assert!(humanoid.can_use_domain(MovementDomain::Walking));
        assert!(humanoid.can_use_domain(MovementDomain::Swimming));
        assert!(humanoid.can_use_domain(MovementDomain::Climbing));
        assert!(!humanoid.can_use_domain(MovementDomain::Flying));

        let flying = capability_presets::flying();
        assert!(flying.can_use_domain(MovementDomain::Flying));
        assert!(flying.can_hover);

        let crawler = capability_presets::crawler();
        assert!(crawler.can_wall_cling);
    }

    #[test]
    fn test_node_annotation() {
        let caps = capability_presets::humanoid();
        let annotation =
            NodeAnnotation::new(NavPosition::world(0.0, 0.0, 0.0), MovementDomain::Walking)
                .with_alt_domain(MovementDomain::Swimming)
                .as_transition();

        assert!(annotation.is_usable_by(&caps));
        assert_eq!(
            annotation.best_domain_for(&caps),
            Some(MovementDomain::Walking)
        );
        assert!(annotation.is_transition);
    }

    #[test]
    fn test_edge_annotation() {
        let caps = capability_presets::humanoid();
        let edge = EdgeAnnotation::new(MovementDomain::Walking, 1.0)
            .with_modifier(CostModifier::Flat(0.5))
            .with_requirements(EdgeRequirements {
                min_jump: Some(1.0),
                ..Default::default()
            });

        assert!(edge.is_traversable_by(&caps));
        let cost = edge.cost_for(&caps);
        assert!(cost > 1.0);
    }

    #[test]
    fn test_edge_requirements() {
        let caps = capability_presets::humanoid();

        let basic_reqs = EdgeRequirements::default();
        assert!(basic_reqs.satisfied_by(&caps));

        let jump_reqs = EdgeRequirements {
            min_jump: Some(10.0),
            ..Default::default()
        };
        assert!(!jump_reqs.satisfied_by(&caps));

        let hover_reqs = EdgeRequirements {
            needs_hover: true,
            ..Default::default()
        };
        assert!(!hover_reqs.satisfied_by(&caps));
    }

    #[test]
    fn test_nav_position() {
        let world = NavPosition::world(1.5, 2.0, 3.5);
        let local = NavPosition::local(1.0, 2.0, 3.0);

        assert!(!world.frame_local);
        assert!(local.frame_local);

        let grid = world.to_grid();
        assert_eq!(grid.x, 1);
        assert_eq!(grid.y, 2);
        assert_eq!(grid.z, 3);
    }

    #[test]
    fn test_dynamic_frame() {
        let mut frame = DynamicFrame::new(
            FrameId::new("platform1"),
            NavPosition::world(10.0, 0.0, 10.0),
        )
        .with_velocity(FrameVelocity::linear(1.0, 0.0, 0.0));

        assert!(frame.velocity.is_moving());

        let local = NavPosition::local(5.0, 0.0, 5.0);
        let world = frame.local_to_world(local);
        assert!((world.x - 15.0).abs() < f32::EPSILON);
        assert!((world.z - 15.0).abs() < f32::EPSILON);

        frame.update(
            NavPosition::world(20.0, 0.0, 10.0),
            FrameVelocity::linear(0.0, 0.0, 0.0),
            100,
        );
        assert_eq!(frame.last_update_tick, 100);
    }

    #[test]
    fn test_route_request() {
        let caps = capability_presets::humanoid();
        let mut request = RouteRequest::new(
            1,
            NavPosition::world(0.0, 0.0, 0.0),
            NavPosition::world(10.0, 0.0, 10.0),
            caps,
            0,
        )
        .with_max_cost(100.0)
        .with_max_length(50);

        request.avoid_domain(MovementDomain::Swimming);
        request.avoid_frame(FrameId::new("dangerous_platform"));

        assert!(request.avoid_domains.contains(&MovementDomain::Swimming));
        assert!(
            request
                .avoid_frames
                .contains(&FrameId::new("dangerous_platform"))
        );
    }

    #[test]
    fn test_route_result() {
        let plan = RoutePlan::new(RouteRequestId::new(1), 0);
        let found = RouteResult::Found(plan.clone());
        let partial = RouteResult::Partial(plan);
        let not_found = RouteResult::NotFound(RouteFailure::NoPath);

        assert!(found.is_found());
        assert!(found.has_path());
        assert!(!partial.is_found());
        assert!(partial.has_path());
        assert!(!not_found.is_found());
        assert!(!not_found.has_path());
    }

    #[test]
    fn test_route_plan() {
        let mut plan = RoutePlan::new(RouteRequestId::new(1), 0);

        plan.add_waypoint(
            RouteWaypoint::new(NavPosition::world(0.0, 0.0, 0.0), MovementDomain::Walking)
                .on_frame(FrameId::new("f1"))
                .in_region(NavRegionId::new("r1")),
        );
        plan.add_waypoint(RouteWaypoint::new(
            NavPosition::world(5.0, 0.0, 5.0),
            MovementDomain::Swimming,
        ));

        assert_eq!(plan.waypoint_count(), 2);
        assert!(plan.domains_used.contains(&MovementDomain::Walking));
        assert!(plan.domains_used.contains(&MovementDomain::Swimming));
        assert!(plan.frames_traversed.contains(&FrameId::new("f1")));
        assert!(plan.regions_traversed.contains(&NavRegionId::new("r1")));
    }

    #[test]
    fn test_route_plan_expiration() {
        let plan = RoutePlan::new(RouteRequestId::new(1), 0).expires_at(100);

        assert!(!plan.is_expired(50));
        assert!(plan.is_expired(150));
    }

    #[test]
    fn test_steering_hint() {
        let hint = SteeringHint {
            direction: SteeringDirection::Ascend,
            speed_factor: 0.5,
            precise: true,
            anticipate: false,
        };

        assert_eq!(hint.direction, SteeringDirection::Ascend);
        assert!(hint.precise);
        assert!(!hint.anticipate);
    }

    #[test]
    fn test_region_summary() {
        let mut summary = NavRegionSummary::new(
            NavRegionId::new("region1"),
            NavPosition::world(50.0, 0.0, 50.0),
            0,
        );

        summary.add_domain(MovementDomain::Walking);
        summary.add_domain(MovementDomain::Swimming);
        summary.add_frame(FrameId::new("platform1"));
        summary.add_connection(
            NavRegionId::new("region2"),
            RegionConnection::new(MovementDomain::Walking, 10.0),
        );

        let caps = capability_presets::humanoid();
        assert!(summary.is_traversable_by(&caps));

        summary.update_staleness(200, 100);
        assert!(summary.is_stale);

        summary.refresh(200);
        assert!(!summary.is_stale);
    }

    #[test]
    fn test_navigator_basic() {
        let mut nav = Navigator::new();
        nav.set_tick(100);

        nav.set_node(
            GridPos::new(0, 0, 0),
            NodeAnnotation::new(NavPosition::world(0.5, 0.0, 0.5), MovementDomain::Walking),
        );
        nav.set_node(
            GridPos::new(1, 0, 0),
            NodeAnnotation::new(NavPosition::world(1.5, 0.0, 0.5), MovementDomain::Walking),
        );
        nav.set_edge(
            GridPos::new(0, 0, 0),
            GridPos::new(1, 0, 0),
            EdgeAnnotation::new(MovementDomain::Walking, 1.0),
        );

        assert_eq!(nav.node_count(), 2);
        assert_eq!(nav.edge_count(), 2);
        assert!(nav.get_node(&GridPos::new(0, 0, 0)).is_some());
        assert!(
            nav.get_edge(&GridPos::new(0, 0, 0), &GridPos::new(1, 0, 0))
                .is_some()
        );
    }

    #[test]
    fn test_navigator_frames() {
        let mut nav = Navigator::new();

        let frame = DynamicFrame::new(
            FrameId::new("platform1"),
            NavPosition::world(10.0, 0.0, 10.0),
        );
        nav.register_frame(frame);

        assert_eq!(nav.frame_count(), 1);
        assert!(nav.get_frame(&FrameId::new("platform1")).is_some());

        nav.update_frame(
            &FrameId::new("platform1"),
            NavPosition::world(20.0, 0.0, 10.0),
            FrameVelocity::linear(1.0, 0.0, 0.0),
        );

        let frame = nav.get_frame(&FrameId::new("platform1")).unwrap();
        assert!((frame.position.x - 20.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_navigator_regions() {
        let mut nav = Navigator::new();
        nav.set_tick(100);

        let summary =
            NavRegionSummary::new(NavRegionId::new("r1"), NavPosition::world(0.0, 0.0, 0.0), 0);
        nav.register_region(summary);

        nav.update_region_staleness(50);

        let stale: Vec<_> = nav.stale_regions().collect();
        assert_eq!(stale.len(), 1);
    }

    #[test]
    fn test_navigator_plan_route() {
        let mut nav = Navigator::new();
        nav.set_tick(0);

        for x in 0..5 {
            nav.set_node(
                GridPos::new(x, 0, 0),
                NodeAnnotation::new(
                    NavPosition::world(x as f32 + 0.5, 0.0, 0.5),
                    MovementDomain::Walking,
                ),
            );
        }

        for x in 0..4 {
            nav.set_edge(
                GridPos::new(x, 0, 0),
                GridPos::new(x + 1, 0, 0),
                EdgeAnnotation::new(MovementDomain::Walking, 1.0),
            );
        }

        let caps = capability_presets::humanoid();
        let request = nav.create_request(
            NavPosition::world(0.5, 0.0, 0.5),
            NavPosition::world(4.5, 0.0, 0.5),
            caps,
        );

        let result = nav.plan_route(&request);
        assert!(result.is_found());

        let plan = result.plan().unwrap();
        assert!(plan.waypoint_count() >= 2);
        assert!(plan.domains_used.contains(&MovementDomain::Walking));
    }

    #[test]
    fn test_navigator_check_replan() {
        let mut nav = Navigator::new();
        nav.set_tick(0);

        let mut plan = RoutePlan::new(RouteRequestId::new(1), 0).expires_at(100);
        plan.add_waypoint(RouteWaypoint::new(
            NavPosition::world(0.0, 0.0, 0.0),
            MovementDomain::Walking,
        ));
        plan.frames_traversed.insert(FrameId::new("platform1"));

        let current_pos = NavPosition::world(0.0, 0.0, 0.0);
        let reason = nav.check_replan(&plan, &current_pos);
        assert!(matches!(reason, Some(ReplanReason::FrameDeactivated(_))));

        nav.register_frame(DynamicFrame::new(
            FrameId::new("platform1"),
            NavPosition::world(0.0, 0.0, 0.0),
        ));

        nav.set_tick(200);
        let reason = nav.check_replan(&plan, &current_pos);
        assert!(matches!(reason, Some(ReplanReason::Expired)));
    }

    #[test]
    fn test_multi_domain_world() {
        let caps = capability_presets::humanoid();
        let mut nodes = BTreeMap::new();
        nodes.insert(
            GridPos::new(0, 0, 0),
            NodeAnnotation::new(NavPosition::world(0.5, 0.0, 0.5), MovementDomain::Walking),
        );
        nodes.insert(
            GridPos::new(1, 0, 0),
            NodeAnnotation::new(NavPosition::world(1.5, 0.0, 0.5), MovementDomain::Swimming),
        );
        nodes.insert(
            GridPos::new(2, 0, 0),
            NodeAnnotation::new(NavPosition::world(2.5, 0.0, 0.5), MovementDomain::Flying),
        );

        let edges = BTreeMap::new();
        let avoid_domains = BTreeSet::new();
        let avoid_frames = BTreeSet::new();
        let avoid_regions = BTreeSet::new();

        let world = MultiDomainWorld::new(
            &caps,
            &nodes,
            &edges,
            &avoid_domains,
            &avoid_frames,
            &avoid_regions,
        );

        assert!(world.is_walkable(&GridPos::new(0, 0, 0)));
        assert!(world.is_walkable(&GridPos::new(1, 0, 0)));
        assert!(!world.is_walkable(&GridPos::new(2, 0, 0)));
    }

    #[test]
    fn test_serde_movement_domain() {
        let domain = MovementDomain::Swimming;
        let json = serde_json::to_string(&domain).unwrap();
        let restored: MovementDomain = serde_json::from_str(&json).unwrap();
        assert_eq!(domain, restored);
    }

    #[test]
    fn test_serde_domain_cost() {
        let cost = DomainCost::new(1.5, 2.0, 0.3);
        let json = serde_json::to_string(&cost).unwrap();
        let restored: DomainCost = serde_json::from_str(&json).unwrap();
        assert!((cost.multiplier - restored.multiplier).abs() < f32::EPSILON);
    }

    #[test]
    fn test_serde_agent_capabilities() {
        let caps = capability_presets::humanoid();
        let json = serde_json::to_string(&caps).unwrap();
        let restored: AgentCapabilities = serde_json::from_str(&json).unwrap();
        assert_eq!(caps.id, restored.id);
        assert!(restored.can_use_domain(MovementDomain::Walking));
    }

    #[test]
    fn test_serde_node_annotation() {
        let annotation =
            NodeAnnotation::new(NavPosition::world(1.0, 2.0, 3.0), MovementDomain::Walking)
                .with_alt_domain(MovementDomain::Swimming)
                .on_frame(FrameId::new("f1"))
                .as_transition();

        let json = serde_json::to_string(&annotation).unwrap();
        let restored: NodeAnnotation = serde_json::from_str(&json).unwrap();
        assert_eq!(annotation.domain, restored.domain);
        assert!(restored.is_transition);
    }

    #[test]
    fn test_serde_edge_annotation() {
        let edge = EdgeAnnotation::new(MovementDomain::Walking, 1.5)
            .one_way()
            .with_transition(MovementDomain::Walking, MovementDomain::Swimming);

        let json = serde_json::to_string(&edge).unwrap();
        let restored: EdgeAnnotation = serde_json::from_str(&json).unwrap();
        assert!(!restored.bidirectional);
        assert!(restored.transition.is_some());
    }

    #[test]
    fn test_serde_dynamic_frame() {
        let frame = DynamicFrame::new(
            FrameId::new("platform1"),
            NavPosition::world(10.0, 0.0, 10.0),
        )
        .with_velocity(FrameVelocity::linear(1.0, 0.0, 0.0));

        let json = serde_json::to_string(&frame).unwrap();
        let restored: DynamicFrame = serde_json::from_str(&json).unwrap();
        assert_eq!(frame.id, restored.id);
    }

    #[test]
    fn test_serde_route_request() {
        let caps = capability_presets::humanoid();
        let request = RouteRequest::new(
            1,
            NavPosition::world(0.0, 0.0, 0.0),
            NavPosition::world(10.0, 0.0, 10.0),
            caps,
            0,
        );

        let json = serde_json::to_string(&request).unwrap();
        let restored: RouteRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(request.id, restored.id);
    }

    #[test]
    fn test_serde_route_plan() {
        let mut plan = RoutePlan::new(RouteRequestId::new(1), 0);
        plan.add_waypoint(RouteWaypoint::new(
            NavPosition::world(0.0, 0.0, 0.0),
            MovementDomain::Walking,
        ));

        let json = serde_json::to_string(&plan).unwrap();
        let restored: RoutePlan = serde_json::from_str(&json).unwrap();
        assert_eq!(plan.request_id, restored.request_id);
        assert_eq!(plan.waypoint_count(), restored.waypoint_count());
    }

    #[test]
    fn test_serde_region_summary() {
        let mut summary = NavRegionSummary::new(
            NavRegionId::new("r1"),
            NavPosition::world(50.0, 0.0, 50.0),
            100,
        );
        summary.add_domain(MovementDomain::Walking);

        let json = serde_json::to_string(&summary).unwrap();
        let restored: NavRegionSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(summary.region_id, restored.region_id);
    }

    #[test]
    fn test_serde_navigator() {
        let mut nav = Navigator::new();
        nav.set_tick(100);

        let frame = DynamicFrame::new(FrameId::new("f1"), NavPosition::world(10.0, 0.0, 10.0));
        nav.register_frame(frame);

        let summary =
            NavRegionSummary::new(NavRegionId::new("r1"), NavPosition::world(0.0, 0.0, 0.0), 0);
        nav.register_region(summary);

        let json = serde_json::to_string(&nav).unwrap();
        let restored: Navigator = serde_json::from_str(&json).unwrap();
        assert_eq!(nav.frame_count(), restored.frame_count());
        assert_eq!(nav.region_count(), restored.region_count());
        assert_eq!(nav.tick(), restored.tick());
    }

    #[test]
    fn test_deterministic_ordering() {
        let mut caps = AgentCapabilities::new("test");
        caps.domain_costs
            .insert(MovementDomain::Flying, DomainCost::default());
        caps.domain_costs
            .insert(MovementDomain::Walking, DomainCost::default());
        caps.domain_costs
            .insert(MovementDomain::Swimming, DomainCost::default());

        let domains: Vec<_> = caps.supported_domains().collect();
        assert_eq!(domains[0], MovementDomain::Walking);
        assert_eq!(domains[1], MovementDomain::Swimming);
        assert_eq!(domains[2], MovementDomain::Flying);
    }
}
