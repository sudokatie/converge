//! Group AI primitives for packs, swarms, schools, patrols, and evacuation.
//!
//! Provides deterministic group coordination for AI agents:
//!
//! - Group identity and membership with roles
//! - Formation behaviors (cohesion, separation, alignment)
//! - Preset group types (pack, swarm, school, patrol)
//! - Patrol routes with waypoints
//! - Evacuation triggers and safe zones
//! - Group-level decisions and events
//! - Cheap summaries for unloaded regions

use crate::behavior::blackboard::Vec3;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Unique identifier for a group.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct GroupId(pub String);

impl GroupId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<T: Into<String>> From<T> for GroupId {
    fn from(s: T) -> Self {
        Self::new(s)
    }
}

impl std::fmt::Display for GroupId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a group member (entity).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MemberId(pub u64);

impl MemberId {
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Role within a group.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum GroupRole {
    /// Standard member.
    #[default]
    Member,
    /// Group leader.
    Leader,
    /// Scout/advance member.
    Scout,
    /// Rear guard/defender.
    Guard,
    /// Flanking member.
    Flanker,
    /// Custom role.
    Custom(u8),
}

impl GroupRole {
    #[must_use]
    pub fn is_leader(&self) -> bool {
        matches!(self, Self::Leader)
    }
}

/// Member info within a group.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GroupMember {
    pub id: MemberId,
    pub role: GroupRole,
    pub position: SerializableVec3,
    pub velocity: SerializableVec3,
    pub joined_tick: u64,
    pub metadata: BTreeMap<String, String>,
}

impl GroupMember {
    #[must_use]
    pub fn new(id: MemberId) -> Self {
        Self {
            id,
            role: GroupRole::Member,
            position: SerializableVec3::default(),
            velocity: SerializableVec3::default(),
            joined_tick: 0,
            metadata: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn with_role(mut self, role: GroupRole) -> Self {
        self.role = role;
        self
    }

    #[must_use]
    pub fn with_position(mut self, pos: Vec3) -> Self {
        self.position = pos.into();
        self
    }

    #[must_use]
    pub fn with_tick(mut self, tick: u64) -> Self {
        self.joined_tick = tick;
        self
    }

    pub fn update_position(&mut self, pos: Vec3) {
        self.position = pos.into();
    }

    pub fn update_velocity(&mut self, vel: Vec3) {
        self.velocity = vel.into();
    }
}

/// Serializable Vec3 wrapper for serde support.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SerializableVec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl SerializableVec3 {
    #[must_use]
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    #[must_use]
    pub fn to_vec3(self) -> Vec3 {
        Vec3::new(self.x, self.y, self.z)
    }
}

impl From<Vec3> for SerializableVec3 {
    fn from(v: Vec3) -> Self {
        Self {
            x: v.x,
            y: v.y,
            z: v.z,
        }
    }
}

impl From<SerializableVec3> for Vec3 {
    fn from(v: SerializableVec3) -> Self {
        Vec3::new(v.x, v.y, v.z)
    }
}

/// Configuration for formation behaviors.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FormationConfig {
    /// Weight for cohesion (move toward group center).
    pub cohesion_weight: f32,
    /// Weight for separation (avoid crowding).
    pub separation_weight: f32,
    /// Weight for alignment (match group velocity).
    pub alignment_weight: f32,
    /// Desired distance from neighbors.
    pub desired_separation: f32,
    /// Perception radius for flocking.
    pub perception_radius: f32,
    /// Maximum steering force.
    pub max_steering: f32,
    /// Maximum speed.
    pub max_speed: f32,
}

impl Default for FormationConfig {
    fn default() -> Self {
        Self {
            cohesion_weight: 1.0,
            separation_weight: 1.5,
            alignment_weight: 1.0,
            desired_separation: 2.0,
            perception_radius: 10.0,
            max_steering: 1.0,
            max_speed: 5.0,
        }
    }
}

impl FormationConfig {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_cohesion(mut self, weight: f32) -> Self {
        self.cohesion_weight = weight.max(0.0);
        self
    }

    #[must_use]
    pub fn with_separation(mut self, weight: f32) -> Self {
        self.separation_weight = weight.max(0.0);
        self
    }

    #[must_use]
    pub fn with_alignment(mut self, weight: f32) -> Self {
        self.alignment_weight = weight.max(0.0);
        self
    }

    #[must_use]
    pub fn with_desired_separation(mut self, dist: f32) -> Self {
        self.desired_separation = dist.max(0.1);
        self
    }

    #[must_use]
    pub fn with_perception_radius(mut self, radius: f32) -> Self {
        self.perception_radius = radius.max(0.1);
        self
    }

    #[must_use]
    pub fn with_max_steering(mut self, max: f32) -> Self {
        self.max_steering = max.max(0.0);
        self
    }

    #[must_use]
    pub fn with_max_speed(mut self, max: f32) -> Self {
        self.max_speed = max.max(0.0);
        self
    }
}

/// Result of flocking behavior calculation.
#[derive(Clone, Copy, Debug, Default)]
pub struct FlockingResult {
    pub cohesion: Vec3,
    pub separation: Vec3,
    pub alignment: Vec3,
    pub combined: Vec3,
    pub neighbor_count: u32,
}

impl FlockingResult {
    #[must_use]
    pub fn has_neighbors(&self) -> bool {
        self.neighbor_count > 0
    }
}

/// Calculate flocking behaviors for a member.
#[must_use]
pub fn calculate_flocking(
    position: Vec3,
    velocity: Vec3,
    neighbors: &[(Vec3, Vec3)],
    config: &FormationConfig,
) -> FlockingResult {
    if neighbors.is_empty() {
        return FlockingResult::default();
    }

    let mut center = Vec3::new(0.0, 0.0, 0.0);
    let mut separation = Vec3::new(0.0, 0.0, 0.0);
    let mut avg_velocity = Vec3::new(0.0, 0.0, 0.0);
    let mut neighbor_count = 0u32;
    let mut separation_count = 0u32;

    for (n_pos, n_vel) in neighbors {
        let dist = position.distance(n_pos);

        if dist < config.perception_radius && dist > 0.0 {
            center.x += n_pos.x;
            center.y += n_pos.y;
            center.z += n_pos.z;
            avg_velocity.x += n_vel.x;
            avg_velocity.y += n_vel.y;
            avg_velocity.z += n_vel.z;
            neighbor_count += 1;

            if dist < config.desired_separation {
                let diff_x = position.x - n_pos.x;
                let diff_y = position.y - n_pos.y;
                let diff_z = position.z - n_pos.z;
                separation.x += diff_x / dist;
                separation.y += diff_y / dist;
                separation.z += diff_z / dist;
                separation_count += 1;
            }
        }
    }

    if neighbor_count == 0 {
        return FlockingResult::default();
    }

    #[expect(clippy::cast_precision_loss, reason = "neighbor count bounded")]
    let n = neighbor_count as f32;

    center.x /= n;
    center.y /= n;
    center.z /= n;
    let cohesion = Vec3::new(
        (center.x - position.x) * config.cohesion_weight,
        (center.y - position.y) * config.cohesion_weight,
        (center.z - position.z) * config.cohesion_weight,
    );

    if separation_count > 0 {
        #[expect(clippy::cast_precision_loss, reason = "separation count bounded")]
        let sc = separation_count as f32;
        separation.x = (separation.x / sc) * config.separation_weight;
        separation.y = (separation.y / sc) * config.separation_weight;
        separation.z = (separation.z / sc) * config.separation_weight;
    }

    avg_velocity.x /= n;
    avg_velocity.y /= n;
    avg_velocity.z /= n;
    let alignment = Vec3::new(
        (avg_velocity.x - velocity.x) * config.alignment_weight,
        (avg_velocity.y - velocity.y) * config.alignment_weight,
        (avg_velocity.z - velocity.z) * config.alignment_weight,
    );

    let combined = Vec3::new(
        cohesion.x + separation.x + alignment.x,
        cohesion.y + separation.y + alignment.y,
        cohesion.z + separation.z + alignment.z,
    );

    let magnitude =
        (combined.x * combined.x + combined.y * combined.y + combined.z * combined.z).sqrt();
    let clamped = if magnitude > config.max_steering && magnitude > 0.0 {
        let factor = config.max_steering / magnitude;
        Vec3::new(
            combined.x * factor,
            combined.y * factor,
            combined.z * factor,
        )
    } else {
        combined
    };

    FlockingResult {
        cohesion,
        separation,
        alignment,
        combined: clamped,
        neighbor_count,
    }
}

/// Preset group behavior type.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GroupPreset {
    /// Wolf pack behavior: tight formation, follows leader.
    Pack,
    /// Swarm behavior: loose formation, many individuals.
    Swarm,
    /// School behavior: tight alignment, fluid movement.
    School,
    /// Patrol behavior: follows waypoints.
    Patrol,
    /// Evacuation behavior: moves to safe zone.
    Evacuation,
    /// Custom behavior.
    #[default]
    Custom,
}

impl GroupPreset {
    #[must_use]
    pub fn default_config(&self) -> FormationConfig {
        match self {
            Self::Pack => FormationConfig::new()
                .with_cohesion(1.5)
                .with_separation(2.0)
                .with_alignment(0.8)
                .with_desired_separation(3.0)
                .with_perception_radius(15.0),
            Self::Swarm => FormationConfig::new()
                .with_cohesion(0.5)
                .with_separation(1.0)
                .with_alignment(0.3)
                .with_desired_separation(1.0)
                .with_perception_radius(8.0),
            Self::School => FormationConfig::new()
                .with_cohesion(1.2)
                .with_separation(1.8)
                .with_alignment(1.5)
                .with_desired_separation(1.5)
                .with_perception_radius(12.0),
            Self::Patrol => FormationConfig::new()
                .with_cohesion(0.8)
                .with_separation(1.5)
                .with_alignment(0.5)
                .with_desired_separation(4.0)
                .with_perception_radius(20.0),
            Self::Evacuation => FormationConfig::new()
                .with_cohesion(0.3)
                .with_separation(2.5)
                .with_alignment(0.2)
                .with_desired_separation(2.0)
                .with_perception_radius(10.0),
            Self::Custom => FormationConfig::default(),
        }
    }
}

/// A waypoint in a patrol route.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Waypoint {
    pub id: u32,
    pub position: SerializableVec3,
    pub wait_ticks: u64,
    pub radius: f32,
    pub metadata: BTreeMap<String, String>,
}

impl Waypoint {
    #[must_use]
    pub fn new(id: u32, position: Vec3) -> Self {
        Self {
            id,
            position: position.into(),
            wait_ticks: 0,
            radius: 2.0,
            metadata: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn with_wait(mut self, ticks: u64) -> Self {
        self.wait_ticks = ticks;
        self
    }

    #[must_use]
    pub fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius.max(0.1);
        self
    }

    #[must_use]
    pub fn is_reached(&self, pos: Vec3) -> bool {
        pos.distance(&self.position.to_vec3()) <= self.radius
    }
}

/// Unique identifier for a patrol route.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PatrolRouteId(pub String);

impl PatrolRouteId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A patrol route with ordered waypoints.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PatrolRoute {
    pub id: PatrolRouteId,
    pub waypoints: Vec<Waypoint>,
    pub loops: bool,
    pub reverse_on_end: bool,
}

impl PatrolRoute {
    #[must_use]
    pub fn new(id: PatrolRouteId) -> Self {
        Self {
            id,
            waypoints: Vec::new(),
            loops: true,
            reverse_on_end: false,
        }
    }

    #[must_use]
    pub fn with_waypoints(mut self, waypoints: Vec<Waypoint>) -> Self {
        self.waypoints = waypoints;
        self
    }

    #[must_use]
    pub fn with_loops(mut self, loops: bool) -> Self {
        self.loops = loops;
        self
    }

    #[must_use]
    pub fn with_reverse(mut self, reverse: bool) -> Self {
        self.reverse_on_end = reverse;
        self
    }

    pub fn add_waypoint(&mut self, waypoint: Waypoint) {
        self.waypoints.push(waypoint);
    }

    #[must_use]
    pub fn get_waypoint(&self, index: usize) -> Option<&Waypoint> {
        self.waypoints.get(index)
    }

    #[must_use]
    pub fn waypoint_count(&self) -> usize {
        self.waypoints.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.waypoints.is_empty()
    }

    #[must_use]
    pub fn next_waypoint_index(&self, current: usize, reverse: bool) -> Option<usize> {
        if self.waypoints.is_empty() {
            return None;
        }

        let next = if reverse {
            current.checked_sub(1)
        } else {
            Some(current + 1).filter(|&i| i < self.waypoints.len())
        };

        match next {
            Some(i) => Some(i),
            None if self.loops => Some(if reverse { self.waypoints.len() - 1 } else { 0 }),
            None => None,
        }
    }
}

/// State of a group's patrol.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PatrolState {
    pub route_id: PatrolRouteId,
    pub current_waypoint: usize,
    pub waiting_since: Option<u64>,
    pub reversing: bool,
    pub completed_loops: u32,
}

impl PatrolState {
    #[must_use]
    pub fn new(route_id: PatrolRouteId) -> Self {
        Self {
            route_id,
            current_waypoint: 0,
            waiting_since: None,
            reversing: false,
            completed_loops: 0,
        }
    }

    pub fn advance(&mut self, route: &PatrolRoute, current_tick: u64) -> bool {
        if let Some(wait_start) = self.waiting_since {
            if let Some(wp) = route.get_waypoint(self.current_waypoint)
                && current_tick.saturating_sub(wait_start) < wp.wait_ticks
            {
                return false;
            }
            self.waiting_since = None;
        }

        if let Some(next) = route.next_waypoint_index(self.current_waypoint, self.reversing) {
            if !self.reversing && next == 0 {
                self.completed_loops += 1;
            }
            self.current_waypoint = next;
            true
        } else if route.reverse_on_end {
            self.reversing = !self.reversing;
            if let Some(next) = route.next_waypoint_index(self.current_waypoint, self.reversing) {
                self.current_waypoint = next;
            }
            true
        } else {
            false
        }
    }

    pub fn start_waiting(&mut self, tick: u64) {
        self.waiting_since = Some(tick);
    }

    #[must_use]
    pub fn is_waiting(&self) -> bool {
        self.waiting_since.is_some()
    }

    pub fn reset(&mut self) {
        self.current_waypoint = 0;
        self.waiting_since = None;
        self.reversing = false;
        self.completed_loops = 0;
    }
}

/// Trigger condition for evacuation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum EvacuationTrigger {
    /// Threat level exceeds threshold.
    ThreatLevel(f32),
    /// Health below threshold.
    HealthBelow(f32),
    /// Explicit signal received.
    Signal(String),
    /// Group size below minimum.
    GroupSizeBelow(u32),
    /// Environmental hazard detected.
    Hazard(String),
    /// Time-based trigger.
    ScheduledTick(u64),
}

impl EvacuationTrigger {
    #[must_use]
    pub fn is_active(&self, context: &EvacuationContext) -> bool {
        match self {
            Self::ThreatLevel(threshold) => context.threat_level >= *threshold,
            Self::HealthBelow(threshold) => context.avg_health < *threshold,
            Self::Signal(sig) => context.signals.contains(sig),
            Self::GroupSizeBelow(min) => context.group_size < *min,
            Self::Hazard(hazard) => context.hazards.contains(hazard),
            Self::ScheduledTick(tick) => context.current_tick >= *tick,
        }
    }
}

/// Context for evaluating evacuation triggers.
#[derive(Clone, Debug, Default)]
pub struct EvacuationContext {
    pub threat_level: f32,
    pub avg_health: f32,
    pub group_size: u32,
    pub current_tick: u64,
    pub signals: BTreeSet<String>,
    pub hazards: BTreeSet<String>,
}

impl EvacuationContext {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_threat(mut self, level: f32) -> Self {
        self.threat_level = level.clamp(0.0, 1.0);
        self
    }

    #[must_use]
    pub fn with_health(mut self, health: f32) -> Self {
        self.avg_health = health.clamp(0.0, 1.0);
        self
    }

    #[must_use]
    pub fn with_size(mut self, size: u32) -> Self {
        self.group_size = size;
        self
    }

    #[must_use]
    pub fn with_tick(mut self, tick: u64) -> Self {
        self.current_tick = tick;
        self
    }

    pub fn add_signal(&mut self, signal: impl Into<String>) {
        self.signals.insert(signal.into());
    }

    pub fn add_hazard(&mut self, hazard: impl Into<String>) {
        self.hazards.insert(hazard.into());
    }
}

/// A safe zone for evacuation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SafeZone {
    pub id: String,
    pub center: SerializableVec3,
    pub radius: f32,
    pub capacity: Option<u32>,
    pub priority: u32,
    pub active: bool,
}

impl SafeZone {
    #[must_use]
    pub fn new(id: impl Into<String>, center: Vec3, radius: f32) -> Self {
        Self {
            id: id.into(),
            center: center.into(),
            radius: radius.max(0.1),
            capacity: None,
            priority: 0,
            active: true,
        }
    }

    #[must_use]
    pub fn with_capacity(mut self, cap: u32) -> Self {
        self.capacity = Some(cap);
        self
    }

    #[must_use]
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    #[must_use]
    pub fn contains(&self, pos: Vec3) -> bool {
        self.active && pos.distance(&self.center.to_vec3()) <= self.radius
    }

    #[must_use]
    pub fn distance_to(&self, pos: Vec3) -> f32 {
        pos.distance(&self.center.to_vec3())
    }
}

/// State of group evacuation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EvacuationState {
    pub active: bool,
    pub target_zone: Option<String>,
    pub started_tick: u64,
    pub members_evacuated: u32,
    pub trigger: Option<String>,
}

impl EvacuationState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            active: false,
            target_zone: None,
            started_tick: 0,
            members_evacuated: 0,
            trigger: None,
        }
    }

    pub fn start(&mut self, zone_id: impl Into<String>, tick: u64, trigger: impl Into<String>) {
        self.active = true;
        self.target_zone = Some(zone_id.into());
        self.started_tick = tick;
        self.members_evacuated = 0;
        self.trigger = Some(trigger.into());
    }

    pub fn complete(&mut self) {
        self.active = false;
    }

    pub fn record_evacuated(&mut self) {
        self.members_evacuated += 1;
    }

    #[must_use]
    pub fn duration(&self, current_tick: u64) -> u64 {
        current_tick.saturating_sub(self.started_tick)
    }
}

impl Default for EvacuationState {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration for evacuation behavior.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EvacuationConfig {
    pub triggers: Vec<EvacuationTrigger>,
    pub safe_zones: Vec<SafeZone>,
    pub max_evacuation_ticks: u64,
    pub regroup_after: bool,
}

impl Default for EvacuationConfig {
    fn default() -> Self {
        Self {
            triggers: Vec::new(),
            safe_zones: Vec::new(),
            max_evacuation_ticks: 1000,
            regroup_after: true,
        }
    }
}

impl EvacuationConfig {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_trigger(&mut self, trigger: EvacuationTrigger) {
        self.triggers.push(trigger);
    }

    pub fn add_safe_zone(&mut self, zone: SafeZone) {
        self.safe_zones.push(zone);
    }

    #[must_use]
    pub fn should_evacuate(&self, context: &EvacuationContext) -> Option<&EvacuationTrigger> {
        self.triggers.iter().find(|t| t.is_active(context))
    }

    #[must_use]
    pub fn nearest_zone(&self, pos: Vec3) -> Option<&SafeZone> {
        self.safe_zones.iter().filter(|z| z.active).min_by(|a, b| {
            let da = a.distance_to(pos);
            let db = b.distance_to(pos);
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        })
    }

    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "zone priority bounded")]
    pub fn best_zone(&self, pos: Vec3) -> Option<&SafeZone> {
        self.safe_zones.iter().filter(|z| z.active).max_by(|a, b| {
            let score_a = a.priority as f32 - a.distance_to(pos) * 0.01;
            let score_b = b.priority as f32 - b.distance_to(pos) * 0.01;
            score_a
                .partial_cmp(&score_b)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    }
}

/// Event type for group-level events.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GroupEventKind {
    /// Group was formed.
    Formed,
    /// Group was disbanded.
    Disbanded,
    /// Member joined.
    MemberJoined(MemberId),
    /// Member left.
    MemberLeft(MemberId),
    /// Leader changed.
    LeaderChanged(Option<MemberId>),
    /// Evacuation started.
    EvacuationStarted,
    /// Evacuation completed.
    EvacuationCompleted,
    /// Patrol completed a loop.
    PatrolLoopCompleted,
    /// Waypoint reached.
    WaypointReached(u32),
    /// Group split.
    Split(GroupId),
    /// Groups merged.
    Merged(GroupId),
    /// Threat detected.
    ThreatDetected,
    /// Formation changed.
    FormationChanged,
}

/// A group event.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GroupEvent {
    pub kind: GroupEventKind,
    pub group_id: GroupId,
    pub tick: u64,
    pub description: Option<String>,
}

impl GroupEvent {
    #[must_use]
    pub fn new(kind: GroupEventKind, group_id: GroupId, tick: u64) -> Self {
        Self {
            kind,
            group_id,
            tick,
            description: None,
        }
    }

    #[must_use]
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

/// Decision type for group-level decisions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum GroupDecision {
    /// Continue current behavior.
    Continue,
    /// Move to position.
    MoveTo(SerializableVec3),
    /// Follow target.
    Follow(MemberId),
    /// Flee from threat.
    Flee(SerializableVec3),
    /// Attack target.
    Attack(MemberId),
    /// Evacuate to zone.
    Evacuate(String),
    /// Hold position.
    Hold,
    /// Regroup at position.
    Regroup(SerializableVec3),
    /// Split into subgroups.
    Split,
    /// Patrol route.
    Patrol(PatrolRouteId),
}

/// Summary of a group for cheap unloaded-region simulation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GroupSummary {
    pub group_id: GroupId,
    pub preset: GroupPreset,
    pub member_count: u32,
    pub leader_id: Option<MemberId>,
    pub center: SerializableVec3,
    pub avg_velocity: SerializableVec3,
    pub spread: f32,
    pub threat_level: f32,
    pub is_evacuating: bool,
    pub current_decision: GroupDecision,
    pub computed_at_tick: u64,
}

impl GroupSummary {
    #[must_use]
    pub fn new(group_id: GroupId, tick: u64) -> Self {
        Self {
            group_id,
            preset: GroupPreset::Custom,
            member_count: 0,
            leader_id: None,
            center: SerializableVec3::default(),
            avg_velocity: SerializableVec3::default(),
            spread: 0.0,
            threat_level: 0.0,
            is_evacuating: false,
            current_decision: GroupDecision::Continue,
            computed_at_tick: tick,
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.member_count == 0
    }

    #[must_use]
    pub fn has_leader(&self) -> bool {
        self.leader_id.is_some()
    }

    #[must_use]
    pub fn is_stale(&self, current_tick: u64, max_staleness: u64) -> bool {
        current_tick.saturating_sub(self.computed_at_tick) > max_staleness
    }

    #[must_use]
    pub fn age(&self, current_tick: u64) -> u64 {
        current_tick.saturating_sub(self.computed_at_tick)
    }
}

/// Snapshot for persistence and offline simulation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GroupSnapshot {
    pub summary: GroupSummary,
    pub formation_config: FormationConfig,
    pub patrol_state: Option<PatrolState>,
    pub evacuation_state: EvacuationState,
    pub pending_events: Vec<GroupEvent>,
    pub snapshot_tick: u64,
}

impl GroupSnapshot {
    #[must_use]
    pub fn new(summary: GroupSummary, tick: u64) -> Self {
        Self {
            summary,
            formation_config: FormationConfig::default(),
            patrol_state: None,
            evacuation_state: EvacuationState::new(),
            pending_events: Vec::new(),
            snapshot_tick: tick,
        }
    }

    #[must_use]
    pub fn with_formation(mut self, config: FormationConfig) -> Self {
        self.formation_config = config;
        self
    }

    #[must_use]
    pub fn with_patrol(mut self, state: PatrolState) -> Self {
        self.patrol_state = Some(state);
        self
    }

    #[must_use]
    pub fn with_evacuation(mut self, state: EvacuationState) -> Self {
        self.evacuation_state = state;
        self
    }

    pub fn add_event(&mut self, event: GroupEvent) {
        self.pending_events.push(event);
    }

    pub fn drain_events(&mut self) -> Vec<GroupEvent> {
        std::mem::take(&mut self.pending_events)
    }
}

/// A complete group with all state.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Group {
    pub id: GroupId,
    pub preset: GroupPreset,
    pub formation_config: FormationConfig,
    members: BTreeMap<MemberId, GroupMember>,
    leader_id: Option<MemberId>,
    pub patrol_state: Option<PatrolState>,
    pub evacuation_state: EvacuationState,
    pub evacuation_config: EvacuationConfig,
    events: Vec<GroupEvent>,
    pub current_decision: GroupDecision,
    pub created_tick: u64,
    pub last_update_tick: u64,
    pub metadata: BTreeMap<String, String>,
}

impl Group {
    #[must_use]
    pub fn new(id: GroupId, tick: u64) -> Self {
        Self {
            id,
            preset: GroupPreset::Custom,
            formation_config: FormationConfig::default(),
            members: BTreeMap::new(),
            leader_id: None,
            patrol_state: None,
            evacuation_state: EvacuationState::new(),
            evacuation_config: EvacuationConfig::new(),
            events: Vec::new(),
            current_decision: GroupDecision::Continue,
            created_tick: tick,
            last_update_tick: tick,
            metadata: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn with_preset(mut self, preset: GroupPreset) -> Self {
        self.preset = preset;
        self.formation_config = preset.default_config();
        self
    }

    #[must_use]
    pub fn with_formation(mut self, config: FormationConfig) -> Self {
        self.formation_config = config;
        self
    }

    #[must_use]
    pub fn with_evacuation_config(mut self, config: EvacuationConfig) -> Self {
        self.evacuation_config = config;
        self
    }

    pub fn add_member(&mut self, member: GroupMember, tick: u64) {
        let id = member.id;
        self.members.insert(id, member);
        self.events.push(GroupEvent::new(
            GroupEventKind::MemberJoined(id),
            self.id.clone(),
            tick,
        ));
        self.last_update_tick = tick;
    }

    pub fn remove_member(&mut self, id: MemberId, tick: u64) -> Option<GroupMember> {
        let member = self.members.remove(&id);
        if member.is_some() {
            if self.leader_id == Some(id) {
                self.leader_id = None;
                self.elect_leader();
            }
            self.events.push(GroupEvent::new(
                GroupEventKind::MemberLeft(id),
                self.id.clone(),
                tick,
            ));
            self.last_update_tick = tick;
        }
        member
    }

    #[must_use]
    pub fn get_member(&self, id: MemberId) -> Option<&GroupMember> {
        self.members.get(&id)
    }

    pub fn get_member_mut(&mut self, id: MemberId) -> Option<&mut GroupMember> {
        self.members.get_mut(&id)
    }

    #[must_use]
    #[expect(clippy::cast_possible_truncation, reason = "member count bounded")]
    pub fn member_count(&self) -> u32 {
        self.members.len() as u32
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }

    pub fn members(&self) -> impl Iterator<Item = &GroupMember> {
        self.members.values()
    }

    pub fn member_ids(&self) -> impl Iterator<Item = MemberId> + '_ {
        self.members.keys().copied()
    }

    pub fn set_leader(&mut self, id: MemberId, tick: u64) -> bool {
        if !self.members.contains_key(&id) {
            return false;
        }

        if let Some(old_leader) = self.leader_id
            && let Some(m) = self.members.get_mut(&old_leader)
        {
            m.role = GroupRole::Member;
        }

        self.leader_id = Some(id);
        if let Some(m) = self.members.get_mut(&id) {
            m.role = GroupRole::Leader;
        }

        self.events.push(GroupEvent::new(
            GroupEventKind::LeaderChanged(Some(id)),
            self.id.clone(),
            tick,
        ));
        self.last_update_tick = tick;
        true
    }

    #[must_use]
    pub fn leader(&self) -> Option<&GroupMember> {
        self.leader_id.and_then(|id| self.members.get(&id))
    }

    #[must_use]
    pub fn leader_id(&self) -> Option<MemberId> {
        self.leader_id
    }

    fn elect_leader(&mut self) {
        self.leader_id = self.members.keys().next().copied();
        if let Some(id) = self.leader_id
            && let Some(m) = self.members.get_mut(&id)
        {
            m.role = GroupRole::Leader;
        }
    }

    #[must_use]
    pub fn center(&self) -> Vec3 {
        if self.members.is_empty() {
            return Vec3::default();
        }

        let mut sum = Vec3::new(0.0, 0.0, 0.0);
        for m in self.members.values() {
            sum.x += m.position.x;
            sum.y += m.position.y;
            sum.z += m.position.z;
        }

        #[expect(clippy::cast_precision_loss, reason = "member count bounded")]
        let n = self.members.len() as f32;
        Vec3::new(sum.x / n, sum.y / n, sum.z / n)
    }

    #[must_use]
    pub fn avg_velocity(&self) -> Vec3 {
        if self.members.is_empty() {
            return Vec3::default();
        }

        let mut sum = Vec3::new(0.0, 0.0, 0.0);
        for m in self.members.values() {
            sum.x += m.velocity.x;
            sum.y += m.velocity.y;
            sum.z += m.velocity.z;
        }

        #[expect(clippy::cast_precision_loss, reason = "member count bounded")]
        let n = self.members.len() as f32;
        Vec3::new(sum.x / n, sum.y / n, sum.z / n)
    }

    #[must_use]
    pub fn spread(&self) -> f32 {
        if self.members.len() < 2 {
            return 0.0;
        }

        let center = self.center();
        let mut max_dist = 0.0f32;
        for m in self.members.values() {
            let dist = m.position.to_vec3().distance(&center);
            max_dist = max_dist.max(dist);
        }
        max_dist
    }

    #[must_use]
    pub fn calculate_flocking(&self, member_id: MemberId) -> FlockingResult {
        let Some(member) = self.members.get(&member_id) else {
            return FlockingResult::default();
        };

        let neighbors: Vec<(Vec3, Vec3)> = self
            .members
            .values()
            .filter(|m| m.id != member_id)
            .map(|m| (m.position.to_vec3(), m.velocity.to_vec3()))
            .collect();

        calculate_flocking(
            member.position.to_vec3(),
            member.velocity.to_vec3(),
            &neighbors,
            &self.formation_config,
        )
    }

    pub fn check_evacuation(&mut self, context: &EvacuationContext, tick: u64) -> bool {
        if self.evacuation_state.active {
            return false;
        }

        if let Some(trigger) = self.evacuation_config.should_evacuate(context) {
            let trigger_desc = format!("{trigger:?}");
            let center = self.center();
            if let Some(zone) = self.evacuation_config.best_zone(center) {
                self.evacuation_state.start(&zone.id, tick, &trigger_desc);
                self.current_decision = GroupDecision::Evacuate(zone.id.clone());
                self.events.push(
                    GroupEvent::new(GroupEventKind::EvacuationStarted, self.id.clone(), tick)
                        .with_description(trigger_desc),
                );
                self.last_update_tick = tick;
                return true;
            }
        }
        false
    }

    pub fn complete_evacuation(&mut self, tick: u64) {
        if self.evacuation_state.active {
            self.evacuation_state.complete();
            self.current_decision = GroupDecision::Continue;
            self.events.push(GroupEvent::new(
                GroupEventKind::EvacuationCompleted,
                self.id.clone(),
                tick,
            ));
            self.last_update_tick = tick;
        }
    }

    pub fn start_patrol(&mut self, route: &PatrolRoute, tick: u64) {
        self.patrol_state = Some(PatrolState::new(route.id.clone()));
        self.current_decision = GroupDecision::Patrol(route.id.clone());
        self.last_update_tick = tick;
    }

    pub fn advance_patrol(&mut self, route: &PatrolRoute, tick: u64) -> bool {
        if let Some(state) = &mut self.patrol_state {
            let old_wp = state.current_waypoint;
            let old_loops = state.completed_loops;

            if state.advance(route, tick) {
                if state.completed_loops > old_loops {
                    self.events.push(GroupEvent::new(
                        GroupEventKind::PatrolLoopCompleted,
                        self.id.clone(),
                        tick,
                    ));
                }
                if state.current_waypoint != old_wp
                    && let Some(wp) = route.get_waypoint(state.current_waypoint)
                {
                    self.events.push(GroupEvent::new(
                        GroupEventKind::WaypointReached(wp.id),
                        self.id.clone(),
                        tick,
                    ));
                }
                self.last_update_tick = tick;
                return true;
            }
        }
        false
    }

    pub fn drain_events(&mut self) -> Vec<GroupEvent> {
        std::mem::take(&mut self.events)
    }

    #[must_use]
    pub fn events(&self) -> &[GroupEvent] {
        &self.events
    }

    #[must_use]
    pub fn summary(&self, tick: u64) -> GroupSummary {
        GroupSummary {
            group_id: self.id.clone(),
            preset: self.preset,
            member_count: self.member_count(),
            leader_id: self.leader_id,
            center: self.center().into(),
            avg_velocity: self.avg_velocity().into(),
            spread: self.spread(),
            threat_level: 0.0,
            is_evacuating: self.evacuation_state.active,
            current_decision: self.current_decision.clone(),
            computed_at_tick: tick,
        }
    }

    #[must_use]
    pub fn snapshot(&self, tick: u64) -> GroupSnapshot {
        GroupSnapshot {
            summary: self.summary(tick),
            formation_config: self.formation_config.clone(),
            patrol_state: self.patrol_state.clone(),
            evacuation_state: self.evacuation_state.clone(),
            pending_events: self.events.clone(),
            snapshot_tick: tick,
        }
    }
}

/// Registry of all groups.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GroupRegistry {
    groups: BTreeMap<GroupId, Group>,
}

impl GroupRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, group: Group) {
        self.groups.insert(group.id.clone(), group);
    }

    pub fn unregister(&mut self, id: &GroupId) -> Option<Group> {
        self.groups.remove(id)
    }

    #[must_use]
    pub fn get(&self, id: &GroupId) -> Option<&Group> {
        self.groups.get(id)
    }

    pub fn get_mut(&mut self, id: &GroupId) -> Option<&mut Group> {
        self.groups.get_mut(id)
    }

    #[must_use]
    pub fn contains(&self, id: &GroupId) -> bool {
        self.groups.contains_key(id)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.groups.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Group> {
        self.groups.values()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Group> {
        self.groups.values_mut()
    }

    pub fn ids(&self) -> impl Iterator<Item = &GroupId> {
        self.groups.keys()
    }

    pub fn by_preset(&self, preset: GroupPreset) -> impl Iterator<Item = &Group> {
        self.groups.values().filter(move |g| g.preset == preset)
    }

    pub fn find_member(&self, member_id: MemberId) -> Option<&GroupId> {
        self.groups
            .iter()
            .find(|(_, g)| g.members.contains_key(&member_id))
            .map(|(id, _)| id)
    }
}

/// Preset factory functions.
pub mod presets {
    use super::{Group, GroupId, GroupPreset};

    #[must_use]
    pub fn pack(id: impl Into<String>, tick: u64) -> Group {
        Group::new(GroupId::new(id), tick).with_preset(GroupPreset::Pack)
    }

    #[must_use]
    pub fn swarm(id: impl Into<String>, tick: u64) -> Group {
        Group::new(GroupId::new(id), tick).with_preset(GroupPreset::Swarm)
    }

    #[must_use]
    pub fn school(id: impl Into<String>, tick: u64) -> Group {
        Group::new(GroupId::new(id), tick).with_preset(GroupPreset::School)
    }

    #[must_use]
    pub fn patrol(id: impl Into<String>, tick: u64) -> Group {
        Group::new(GroupId::new(id), tick).with_preset(GroupPreset::Patrol)
    }

    #[must_use]
    pub fn evacuation(id: impl Into<String>, tick: u64) -> Group {
        Group::new(GroupId::new(id), tick).with_preset(GroupPreset::Evacuation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vec3(x: f32, y: f32, z: f32) -> Vec3 {
        Vec3::new(x, y, z)
    }

    #[test]
    fn test_group_id() {
        let id = GroupId::new("test_group");
        assert_eq!(id.as_str(), "test_group");
        assert_eq!(format!("{id}"), "test_group");

        let id2: GroupId = "other".into();
        assert_eq!(id2.as_str(), "other");
    }

    #[test]
    fn test_member_id() {
        let id = MemberId::new(42);
        assert_eq!(id.0, 42);
    }

    #[test]
    fn test_group_role() {
        assert!(GroupRole::Leader.is_leader());
        assert!(!GroupRole::Member.is_leader());
        assert!(!GroupRole::Scout.is_leader());
    }

    #[test]
    fn test_group_member() {
        let member = GroupMember::new(MemberId::new(1))
            .with_role(GroupRole::Scout)
            .with_position(vec3(1.0, 2.0, 3.0))
            .with_tick(100);

        assert_eq!(member.id.0, 1);
        assert_eq!(member.role, GroupRole::Scout);
        assert!((member.position.x - 1.0).abs() < f32::EPSILON);
        assert_eq!(member.joined_tick, 100);
    }

    #[test]
    fn test_serializable_vec3() {
        let sv = SerializableVec3::new(1.0, 2.0, 3.0);
        let v = sv.to_vec3();
        assert!((v.x - 1.0).abs() < f32::EPSILON);
        assert!((v.y - 2.0).abs() < f32::EPSILON);
        assert!((v.z - 3.0).abs() < f32::EPSILON);

        let sv2: SerializableVec3 = v.into();
        assert_eq!(sv, sv2);
    }

    #[test]
    fn test_formation_config_default() {
        let config = FormationConfig::default();
        assert!((config.cohesion_weight - 1.0).abs() < f32::EPSILON);
        assert!((config.separation_weight - 1.5).abs() < f32::EPSILON);
        assert!((config.alignment_weight - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_formation_config_builder() {
        let config = FormationConfig::new()
            .with_cohesion(2.0)
            .with_separation(3.0)
            .with_alignment(0.5)
            .with_desired_separation(5.0)
            .with_perception_radius(20.0);

        assert!((config.cohesion_weight - 2.0).abs() < f32::EPSILON);
        assert!((config.separation_weight - 3.0).abs() < f32::EPSILON);
        assert!((config.alignment_weight - 0.5).abs() < f32::EPSILON);
        assert!((config.desired_separation - 5.0).abs() < f32::EPSILON);
        assert!((config.perception_radius - 20.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_calculate_flocking_empty() {
        let result = calculate_flocking(
            vec3(0.0, 0.0, 0.0),
            vec3(0.0, 0.0, 0.0),
            &[],
            &FormationConfig::default(),
        );
        assert_eq!(result.neighbor_count, 0);
        assert!(!result.has_neighbors());
    }

    #[test]
    fn test_calculate_flocking_basic() {
        let neighbors = vec![
            (vec3(5.0, 0.0, 0.0), vec3(1.0, 0.0, 0.0)),
            (vec3(-5.0, 0.0, 0.0), vec3(1.0, 0.0, 0.0)),
        ];
        let config = FormationConfig::default();
        let result = calculate_flocking(
            vec3(0.0, 0.0, 0.0),
            vec3(0.0, 0.0, 0.0),
            &neighbors,
            &config,
        );

        assert_eq!(result.neighbor_count, 2);
        assert!(result.has_neighbors());
        assert!(result.alignment.x > 0.0);
    }

    #[test]
    fn test_calculate_flocking_separation() {
        let neighbors = vec![(vec3(1.0, 0.0, 0.0), vec3(0.0, 0.0, 0.0))];
        let config = FormationConfig::default().with_desired_separation(5.0);
        let result = calculate_flocking(
            vec3(0.0, 0.0, 0.0),
            vec3(0.0, 0.0, 0.0),
            &neighbors,
            &config,
        );

        assert!(result.separation.x < 0.0);
    }

    #[test]
    fn test_group_preset_configs() {
        let pack = GroupPreset::Pack.default_config();
        let swarm = GroupPreset::Swarm.default_config();
        let school = GroupPreset::School.default_config();

        assert!(pack.cohesion_weight > swarm.cohesion_weight);
        assert!(school.alignment_weight > pack.alignment_weight);
    }

    #[test]
    fn test_waypoint() {
        let wp = Waypoint::new(1, vec3(10.0, 0.0, 10.0))
            .with_wait(100)
            .with_radius(5.0);

        assert_eq!(wp.id, 1);
        assert_eq!(wp.wait_ticks, 100);
        assert!((wp.radius - 5.0).abs() < f32::EPSILON);
        assert!(wp.is_reached(vec3(12.0, 0.0, 12.0)));
        assert!(!wp.is_reached(vec3(20.0, 0.0, 20.0)));
    }

    #[test]
    fn test_patrol_route() {
        let mut route = PatrolRoute::new(PatrolRouteId::new("route1")).with_loops(true);

        route.add_waypoint(Waypoint::new(0, vec3(0.0, 0.0, 0.0)));
        route.add_waypoint(Waypoint::new(1, vec3(10.0, 0.0, 0.0)));
        route.add_waypoint(Waypoint::new(2, vec3(10.0, 0.0, 10.0)));

        assert_eq!(route.waypoint_count(), 3);
        assert!(!route.is_empty());
        assert_eq!(route.next_waypoint_index(0, false), Some(1));
        assert_eq!(route.next_waypoint_index(2, false), Some(0));
    }

    #[test]
    fn test_patrol_state() {
        let route = PatrolRoute::new(PatrolRouteId::new("route1"))
            .with_waypoints(vec![
                Waypoint::new(0, vec3(0.0, 0.0, 0.0)),
                Waypoint::new(1, vec3(10.0, 0.0, 0.0)),
            ])
            .with_loops(true);

        let mut state = PatrolState::new(route.id.clone());
        assert_eq!(state.current_waypoint, 0);
        assert_eq!(state.completed_loops, 0);

        state.advance(&route, 0);
        assert_eq!(state.current_waypoint, 1);

        state.advance(&route, 0);
        assert_eq!(state.current_waypoint, 0);
        assert_eq!(state.completed_loops, 1);
    }

    #[test]
    fn test_evacuation_trigger_threat() {
        let trigger = EvacuationTrigger::ThreatLevel(0.7);
        let ctx_low = EvacuationContext::new().with_threat(0.5);
        let ctx_high = EvacuationContext::new().with_threat(0.8);

        assert!(!trigger.is_active(&ctx_low));
        assert!(trigger.is_active(&ctx_high));
    }

    #[test]
    fn test_evacuation_trigger_health() {
        let trigger = EvacuationTrigger::HealthBelow(0.3);
        let ctx_healthy = EvacuationContext::new().with_health(0.8);
        let ctx_hurt = EvacuationContext::new().with_health(0.2);

        assert!(!trigger.is_active(&ctx_healthy));
        assert!(trigger.is_active(&ctx_hurt));
    }

    #[test]
    fn test_evacuation_trigger_signal() {
        let trigger = EvacuationTrigger::Signal("retreat".to_string());
        let mut ctx = EvacuationContext::new();
        assert!(!trigger.is_active(&ctx));

        ctx.add_signal("retreat");
        assert!(trigger.is_active(&ctx));
    }

    #[test]
    fn test_safe_zone() {
        let zone = SafeZone::new("bunker", vec3(50.0, 0.0, 50.0), 10.0)
            .with_capacity(20)
            .with_priority(5);

        assert!(zone.contains(vec3(52.0, 0.0, 52.0)));
        assert!(!zone.contains(vec3(100.0, 0.0, 100.0)));
        assert!((zone.distance_to(vec3(50.0, 0.0, 50.0))).abs() < f32::EPSILON);
    }

    #[test]
    fn test_evacuation_config() {
        let mut config = EvacuationConfig::new();
        config.add_trigger(EvacuationTrigger::ThreatLevel(0.8));
        config.add_safe_zone(SafeZone::new("z1", vec3(0.0, 0.0, 0.0), 10.0));

        let ctx = EvacuationContext::new().with_threat(0.9);
        assert!(config.should_evacuate(&ctx).is_some());

        let nearest = config.nearest_zone(vec3(5.0, 0.0, 5.0));
        assert!(nearest.is_some());
    }

    #[test]
    fn test_group_event() {
        let event = GroupEvent::new(GroupEventKind::Formed, GroupId::new("test"), 100)
            .with_description("Group formed");

        assert_eq!(event.tick, 100);
        assert_eq!(event.description, Some("Group formed".to_string()));
    }

    #[test]
    fn test_group_new() {
        let group = Group::new(GroupId::new("pack1"), 0);
        assert_eq!(group.id.as_str(), "pack1");
        assert!(group.is_empty());
        assert_eq!(group.member_count(), 0);
    }

    #[test]
    fn test_group_with_preset() {
        let group = Group::new(GroupId::new("pack1"), 0).with_preset(GroupPreset::Pack);
        assert_eq!(group.preset, GroupPreset::Pack);
        assert!(group.formation_config.cohesion_weight > 1.0);
    }

    #[test]
    fn test_group_add_remove_member() {
        let mut group = Group::new(GroupId::new("g1"), 0);

        let m1 = GroupMember::new(MemberId::new(1));
        let m2 = GroupMember::new(MemberId::new(2));

        group.add_member(m1, 0);
        group.add_member(m2, 0);

        assert_eq!(group.member_count(), 2);
        assert!(group.get_member(MemberId::new(1)).is_some());

        let removed = group.remove_member(MemberId::new(1), 1);
        assert!(removed.is_some());
        assert_eq!(group.member_count(), 1);
    }

    #[test]
    fn test_group_leader() {
        let mut group = Group::new(GroupId::new("g1"), 0);
        group.add_member(GroupMember::new(MemberId::new(1)), 0);
        group.add_member(GroupMember::new(MemberId::new(2)), 0);

        group.set_leader(MemberId::new(1), 1);
        assert_eq!(group.leader_id(), Some(MemberId::new(1)));

        let leader = group.leader().unwrap();
        assert_eq!(leader.role, GroupRole::Leader);
    }

    #[test]
    fn test_group_center_and_velocity() {
        let mut group = Group::new(GroupId::new("g1"), 0);
        group.add_member(
            GroupMember::new(MemberId::new(1)).with_position(vec3(0.0, 0.0, 0.0)),
            0,
        );
        group.add_member(
            GroupMember::new(MemberId::new(2)).with_position(vec3(10.0, 0.0, 0.0)),
            0,
        );

        let center = group.center();
        assert!((center.x - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_group_spread() {
        let mut group = Group::new(GroupId::new("g1"), 0);
        group.add_member(
            GroupMember::new(MemberId::new(1)).with_position(vec3(0.0, 0.0, 0.0)),
            0,
        );
        group.add_member(
            GroupMember::new(MemberId::new(2)).with_position(vec3(10.0, 0.0, 0.0)),
            0,
        );

        let spread = group.spread();
        assert!((spread - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_group_flocking() {
        let mut group = Group::new(GroupId::new("g1"), 0).with_preset(GroupPreset::School);

        group.add_member(
            GroupMember::new(MemberId::new(1)).with_position(vec3(0.0, 0.0, 0.0)),
            0,
        );
        group.add_member(
            GroupMember::new(MemberId::new(2)).with_position(vec3(5.0, 0.0, 0.0)),
            0,
        );

        let result = group.calculate_flocking(MemberId::new(1));
        assert!(result.has_neighbors());
    }

    #[test]
    fn test_group_evacuation() {
        let mut config = EvacuationConfig::new();
        config.add_trigger(EvacuationTrigger::ThreatLevel(0.7));
        config.add_safe_zone(SafeZone::new("bunker", vec3(100.0, 0.0, 100.0), 20.0));

        let mut group = Group::new(GroupId::new("g1"), 0).with_evacuation_config(config);

        group.add_member(
            GroupMember::new(MemberId::new(1)).with_position(vec3(0.0, 0.0, 0.0)),
            0,
        );

        let ctx = EvacuationContext::new().with_threat(0.9);
        let started = group.check_evacuation(&ctx, 100);
        assert!(started);
        assert!(group.evacuation_state.active);
        assert!(matches!(group.current_decision, GroupDecision::Evacuate(_)));
    }

    #[test]
    fn test_group_patrol() {
        let route = PatrolRoute::new(PatrolRouteId::new("r1"))
            .with_waypoints(vec![
                Waypoint::new(0, vec3(0.0, 0.0, 0.0)),
                Waypoint::new(1, vec3(10.0, 0.0, 0.0)),
            ])
            .with_loops(true);

        let mut group = Group::new(GroupId::new("g1"), 0);
        group.start_patrol(&route, 0);

        assert!(group.patrol_state.is_some());
        assert!(matches!(group.current_decision, GroupDecision::Patrol(_)));

        group.advance_patrol(&route, 100);
        assert_eq!(group.patrol_state.as_ref().unwrap().current_waypoint, 1);
    }

    #[test]
    fn test_group_summary() {
        let mut group = Group::new(GroupId::new("g1"), 0).with_preset(GroupPreset::Pack);

        group.add_member(
            GroupMember::new(MemberId::new(1)).with_position(vec3(0.0, 0.0, 0.0)),
            0,
        );

        let summary = group.summary(100);
        assert_eq!(summary.group_id.as_str(), "g1");
        assert_eq!(summary.member_count, 1);
        assert_eq!(summary.preset, GroupPreset::Pack);
        assert_eq!(summary.computed_at_tick, 100);
    }

    #[test]
    fn test_group_snapshot() {
        let mut group = Group::new(GroupId::new("g1"), 0);
        group.add_member(GroupMember::new(MemberId::new(1)), 0);

        let snapshot = group.snapshot(100);
        assert_eq!(snapshot.summary.member_count, 1);
        assert_eq!(snapshot.snapshot_tick, 100);
    }

    #[test]
    fn test_group_events() {
        let mut group = Group::new(GroupId::new("g1"), 0);
        group.add_member(GroupMember::new(MemberId::new(1)), 0);
        group.add_member(GroupMember::new(MemberId::new(2)), 1);
        group.remove_member(MemberId::new(1), 2);

        let events = group.drain_events();
        assert_eq!(events.len(), 3);
        assert!(matches!(events[0].kind, GroupEventKind::MemberJoined(_)));
        assert!(matches!(events[1].kind, GroupEventKind::MemberJoined(_)));
        assert!(matches!(events[2].kind, GroupEventKind::MemberLeft(_)));
    }

    #[test]
    fn test_group_registry() {
        let mut registry = GroupRegistry::new();

        registry.register(Group::new(GroupId::new("g1"), 0).with_preset(GroupPreset::Pack));
        registry.register(Group::new(GroupId::new("g2"), 0).with_preset(GroupPreset::School));
        registry.register(Group::new(GroupId::new("g3"), 0).with_preset(GroupPreset::Pack));

        assert_eq!(registry.len(), 3);
        assert!(registry.contains(&GroupId::new("g1")));

        let packs: Vec<_> = registry.by_preset(GroupPreset::Pack).collect();
        assert_eq!(packs.len(), 2);

        let removed = registry.unregister(&GroupId::new("g1"));
        assert!(removed.is_some());
        assert_eq!(registry.len(), 2);
    }

    #[test]
    fn test_registry_find_member() {
        let mut registry = GroupRegistry::new();

        let mut g1 = Group::new(GroupId::new("g1"), 0);
        g1.add_member(GroupMember::new(MemberId::new(100)), 0);
        registry.register(g1);

        let found = registry.find_member(MemberId::new(100));
        assert_eq!(found, Some(&GroupId::new("g1")));

        let not_found = registry.find_member(MemberId::new(999));
        assert!(not_found.is_none());
    }

    #[test]
    fn test_presets_factory() {
        let pack = presets::pack("pack1", 0);
        assert_eq!(pack.preset, GroupPreset::Pack);

        let swarm = presets::swarm("swarm1", 0);
        assert_eq!(swarm.preset, GroupPreset::Swarm);

        let school = presets::school("school1", 0);
        assert_eq!(school.preset, GroupPreset::School);

        let patrol = presets::patrol("patrol1", 0);
        assert_eq!(patrol.preset, GroupPreset::Patrol);
    }

    #[test]
    fn test_group_summary_staleness() {
        let summary = GroupSummary::new(GroupId::new("g1"), 100);

        assert!(!summary.is_stale(150, 100));
        assert!(summary.is_stale(250, 100));
        assert_eq!(summary.age(150), 50);
    }

    #[test]
    fn test_serde_group_id() {
        let id = GroupId::new("test_group");
        let json = serde_json::to_string(&id).unwrap();
        let restored: GroupId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, restored);
    }

    #[test]
    fn test_serde_member() {
        let member = GroupMember::new(MemberId::new(1))
            .with_role(GroupRole::Scout)
            .with_position(vec3(1.0, 2.0, 3.0));

        let json = serde_json::to_string(&member).unwrap();
        let restored: GroupMember = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.id, member.id);
        assert_eq!(restored.role, member.role);
    }

    #[test]
    fn test_serde_formation_config() {
        let config = FormationConfig::new()
            .with_cohesion(2.0)
            .with_separation(1.5);

        let json = serde_json::to_string(&config).unwrap();
        let restored: FormationConfig = serde_json::from_str(&json).unwrap();

        assert!((restored.cohesion_weight - 2.0).abs() < f32::EPSILON);
        assert!((restored.separation_weight - 1.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_serde_waypoint() {
        let wp = Waypoint::new(1, vec3(10.0, 0.0, 10.0)).with_wait(100);

        let json = serde_json::to_string(&wp).unwrap();
        let restored: Waypoint = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.id, 1);
        assert_eq!(restored.wait_ticks, 100);
    }

    #[test]
    fn test_serde_patrol_route() {
        let route = PatrolRoute::new(PatrolRouteId::new("r1"))
            .with_waypoints(vec![Waypoint::new(0, vec3(0.0, 0.0, 0.0))])
            .with_loops(true);

        let json = serde_json::to_string(&route).unwrap();
        let restored: PatrolRoute = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.id.as_str(), "r1");
        assert!(restored.loops);
        assert_eq!(restored.waypoint_count(), 1);
    }

    #[test]
    fn test_serde_safe_zone() {
        let zone = SafeZone::new("bunker", vec3(50.0, 0.0, 50.0), 10.0).with_capacity(20);

        let json = serde_json::to_string(&zone).unwrap();
        let restored: SafeZone = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.id, "bunker");
        assert_eq!(restored.capacity, Some(20));
    }

    #[test]
    fn test_serde_evacuation_trigger() {
        let trigger = EvacuationTrigger::ThreatLevel(0.8);
        let json = serde_json::to_string(&trigger).unwrap();
        let restored: EvacuationTrigger = serde_json::from_str(&json).unwrap();
        assert_eq!(trigger, restored);
    }

    #[test]
    fn test_serde_group_event() {
        let event = GroupEvent::new(
            GroupEventKind::MemberJoined(MemberId::new(1)),
            GroupId::new("g1"),
            100,
        );

        let json = serde_json::to_string(&event).unwrap();
        let restored: GroupEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.tick, 100);
    }

    #[test]
    fn test_serde_group_decision() {
        let decision = GroupDecision::Evacuate("zone1".to_string());
        let json = serde_json::to_string(&decision).unwrap();
        let restored: GroupDecision = serde_json::from_str(&json).unwrap();
        assert_eq!(decision, restored);
    }

    #[test]
    fn test_serde_group_summary() {
        let summary = GroupSummary::new(GroupId::new("g1"), 100);
        let json = serde_json::to_string(&summary).unwrap();
        let restored: GroupSummary = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.group_id.as_str(), "g1");
        assert_eq!(restored.computed_at_tick, 100);
    }

    #[test]
    fn test_serde_group_snapshot() {
        let summary = GroupSummary::new(GroupId::new("g1"), 100);
        let snapshot = GroupSnapshot::new(summary, 100);

        let json = serde_json::to_string(&snapshot).unwrap();
        let restored: GroupSnapshot = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.snapshot_tick, 100);
    }

    #[test]
    fn test_serde_group() {
        let mut group = Group::new(GroupId::new("g1"), 0).with_preset(GroupPreset::Pack);
        group.add_member(GroupMember::new(MemberId::new(1)), 0);

        let json = serde_json::to_string(&group).unwrap();
        let restored: Group = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.id.as_str(), "g1");
        assert_eq!(restored.preset, GroupPreset::Pack);
        assert_eq!(restored.member_count(), 1);
    }

    #[test]
    fn test_serde_group_registry() {
        let mut registry = GroupRegistry::new();
        registry.register(Group::new(GroupId::new("g1"), 0));
        registry.register(Group::new(GroupId::new("g2"), 0));

        let json = serde_json::to_string(&registry).unwrap();
        let restored: GroupRegistry = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.len(), 2);
        assert!(restored.contains(&GroupId::new("g1")));
    }

    #[test]
    fn test_deterministic_member_order() {
        let mut group = Group::new(GroupId::new("g1"), 0);
        group.add_member(GroupMember::new(MemberId::new(3)), 0);
        group.add_member(GroupMember::new(MemberId::new(1)), 0);
        group.add_member(GroupMember::new(MemberId::new(2)), 0);

        let ids: Vec<_> = group.member_ids().collect();
        assert_eq!(
            ids,
            vec![MemberId::new(1), MemberId::new(2), MemberId::new(3)]
        );
    }

    #[test]
    fn test_deterministic_registry_order() {
        let mut registry = GroupRegistry::new();
        registry.register(Group::new(GroupId::new("z"), 0));
        registry.register(Group::new(GroupId::new("a"), 0));
        registry.register(Group::new(GroupId::new("m"), 0));

        let ids: Vec<_> = registry.ids().map(GroupId::as_str).collect();
        assert_eq!(ids, vec!["a", "m", "z"]);
    }
}
