//! Curved-world navigation and pathfinding for spherical and interior surfaces.
//!
//! Provides deterministic navigation planning on non-flat geometries:
//!
//! - Surface types: spherical planetary exteriors, interior sphere surfaces (Dyson spheres, caves)
//! - Curved surface positions with latitude/longitude or angular coordinates
//! - Tangent space and local neighborhood computation
//! - Geodesic distance calculations and path costs
//! - Integration with movement domains and agent capabilities
//! - Pathfinding using geodesic-aware A*
//! - Summaries, projections, and fingerprints for offline/deterministic verification

use crate::navigation::{AgentCapabilities, MovementDomain};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};
use std::f32::consts::PI;
use std::fmt;
use std::hash::{Hash, Hasher};

/// Type of curved surface geometry.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum SurfaceGeometry {
    /// Exterior of a sphere (standard planet).
    #[default]
    SphericalExterior,
    /// Interior of a sphere (Dyson sphere, hollow world).
    SphericalInterior,
}

impl SurfaceGeometry {
    /// Returns whether gravity points toward the surface center.
    #[must_use]
    pub fn gravity_inward(&self) -> bool {
        matches!(self, Self::SphericalExterior)
    }

    /// Returns the local "up" direction sign relative to center.
    #[must_use]
    pub fn up_sign(&self) -> f32 {
        match self {
            Self::SphericalExterior => 1.0,
            Self::SphericalInterior => -1.0,
        }
    }
}

/// Configuration for a curved surface world.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CurvedSurfaceConfig {
    /// Surface geometry type.
    pub geometry: SurfaceGeometry,
    /// Radius of the sphere in world units.
    pub radius: f32,
    /// Center position of the sphere in world space.
    pub center_x: f32,
    pub center_y: f32,
    pub center_z: f32,
    /// Grid resolution in latitude divisions (for discretization).
    pub lat_divisions: u32,
    /// Grid resolution in longitude divisions.
    pub lon_divisions: u32,
    /// Maximum elevation deviation from sphere surface.
    pub max_elevation: f32,
    /// Surface identifier.
    pub surface_id: CurvedSurfaceId,
}

impl Default for CurvedSurfaceConfig {
    fn default() -> Self {
        Self {
            geometry: SurfaceGeometry::SphericalExterior,
            radius: 1000.0,
            center_x: 0.0,
            center_y: 0.0,
            center_z: 0.0,
            lat_divisions: 180,
            lon_divisions: 360,
            max_elevation: 100.0,
            surface_id: CurvedSurfaceId::new("default"),
        }
    }
}

impl CurvedSurfaceConfig {
    /// Create a new spherical exterior surface config.
    #[must_use]
    pub fn spherical_exterior(radius: f32, id: impl Into<String>) -> Self {
        Self {
            geometry: SurfaceGeometry::SphericalExterior,
            radius: radius.max(1.0),
            surface_id: CurvedSurfaceId::new(id),
            ..Default::default()
        }
    }

    /// Create a new spherical interior surface config.
    #[must_use]
    pub fn spherical_interior(radius: f32, id: impl Into<String>) -> Self {
        Self {
            geometry: SurfaceGeometry::SphericalInterior,
            radius: radius.max(1.0),
            surface_id: CurvedSurfaceId::new(id),
            ..Default::default()
        }
    }

    /// Set sphere center.
    #[must_use]
    pub fn with_center(mut self, x: f32, y: f32, z: f32) -> Self {
        self.center_x = x;
        self.center_y = y;
        self.center_z = z;
        self
    }

    /// Set grid resolution.
    #[must_use]
    pub fn with_resolution(mut self, lat: u32, lon: u32) -> Self {
        self.lat_divisions = lat.max(4);
        self.lon_divisions = lon.max(4);
        self
    }

    /// Set maximum elevation.
    #[must_use]
    pub fn with_max_elevation(mut self, elev: f32) -> Self {
        self.max_elevation = elev.max(0.0);
        self
    }

    /// Get latitude step size in radians.
    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "grid divisions fit in f32")]
    pub fn lat_step(&self) -> f32 {
        PI / self.lat_divisions as f32
    }

    /// Get longitude step size in radians.
    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "grid divisions fit in f32")]
    pub fn lon_step(&self) -> f32 {
        (2.0 * PI) / self.lon_divisions as f32
    }
}

/// Unique identifier for a curved surface.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CurvedSurfaceId(pub String);

impl CurvedSurfaceId {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CurvedSurfaceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Position on a curved surface using spherical coordinates.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CurvedPosition {
    /// Latitude in radians (-PI/2 to PI/2, south to north pole).
    pub latitude: f32,
    /// Longitude in radians (0 to 2*PI).
    pub longitude: f32,
    /// Elevation above/below the nominal surface radius.
    pub elevation: f32,
}

impl CurvedPosition {
    /// Create a new curved position.
    #[must_use]
    pub fn new(latitude: f32, longitude: f32, elevation: f32) -> Self {
        Self {
            latitude: latitude.clamp(-PI / 2.0, PI / 2.0),
            longitude: longitude.rem_euclid(2.0 * PI),
            elevation,
        }
    }

    /// Create a position at the equator with given longitude.
    #[must_use]
    pub fn equator(longitude: f32) -> Self {
        Self::new(0.0, longitude, 0.0)
    }

    /// Create position at north pole.
    #[must_use]
    pub fn north_pole() -> Self {
        Self::new(PI / 2.0, 0.0, 0.0)
    }

    /// Create position at south pole.
    #[must_use]
    pub fn south_pole() -> Self {
        Self::new(-PI / 2.0, 0.0, 0.0)
    }

    /// Convert to Cartesian coordinates relative to sphere center.
    #[must_use]
    pub fn to_cartesian(&self, config: &CurvedSurfaceConfig) -> (f32, f32, f32) {
        let r = config.radius + self.elevation * config.geometry.up_sign();
        let cos_lat = self.latitude.cos();
        let x = config.center_x + r * cos_lat * self.longitude.cos();
        let y = config.center_y + r * self.latitude.sin();
        let z = config.center_z + r * cos_lat * self.longitude.sin();
        (x, y, z)
    }

    /// Create from Cartesian coordinates.
    #[must_use]
    pub fn from_cartesian(x: f32, y: f32, z: f32, config: &CurvedSurfaceConfig) -> Self {
        let dx = x - config.center_x;
        let dy = y - config.center_y;
        let dz = z - config.center_z;
        let r = (dx * dx + dy * dy + dz * dz).sqrt();

        if r < 0.0001 {
            return Self::new(0.0, 0.0, -config.radius);
        }

        let latitude = (dy / r).clamp(-1.0, 1.0).asin();
        let longitude = dz.atan2(dx).rem_euclid(2.0 * PI);
        let elevation = (r - config.radius) * config.geometry.up_sign();

        Self::new(latitude, longitude, elevation)
    }

    /// Convert to discrete grid cell.
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "grid indices are bounded"
    )]
    pub fn to_grid_cell(&self, config: &CurvedSurfaceConfig) -> CurvedGridCell {
        let lat_normalized = (self.latitude + PI / 2.0) / PI;
        let lon_normalized = self.longitude / (2.0 * PI);

        let lat_idx = (lat_normalized * config.lat_divisions as f32).floor() as u32;
        let lon_idx = (lon_normalized * config.lon_divisions as f32).floor() as u32;

        CurvedGridCell {
            lat_idx: lat_idx.min(config.lat_divisions - 1),
            lon_idx: lon_idx % config.lon_divisions,
            surface_id: config.surface_id.clone(),
        }
    }

    /// Calculate great-circle (geodesic) distance to another position.
    #[must_use]
    pub fn geodesic_distance(&self, other: &CurvedPosition, config: &CurvedSurfaceConfig) -> f32 {
        let d_lat = other.latitude - self.latitude;
        let d_lon = other.longitude - self.longitude;

        let a = (d_lat / 2.0).sin().powi(2)
            + self.latitude.cos() * other.latitude.cos() * (d_lon / 2.0).sin().powi(2);
        let c = 2.0 * a.sqrt().clamp(0.0, 1.0).asin();

        let avg_elevation = f32::midpoint(self.elevation, other.elevation);
        (config.radius + avg_elevation) * c
    }

    /// Calculate straight-line (chord) distance.
    #[must_use]
    pub fn chord_distance(&self, other: &CurvedPosition, config: &CurvedSurfaceConfig) -> f32 {
        let (x1, y1, z1) = self.to_cartesian(config);
        let (x2, y2, z2) = other.to_cartesian(config);
        let dx = x2 - x1;
        let dy = y2 - y1;
        let dz = z2 - z1;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    /// Get the tangent space basis vectors (east, north, up).
    #[must_use]
    pub fn tangent_basis(&self, config: &CurvedSurfaceConfig) -> TangentBasis {
        let cos_lat = self.latitude.cos();
        let sin_lat = self.latitude.sin();
        let cos_lon = self.longitude.cos();
        let sin_lon = self.longitude.sin();

        let up_sign = config.geometry.up_sign();

        let east = (-sin_lon, 0.0, cos_lon);
        let north = (-sin_lat * cos_lon, cos_lat, -sin_lat * sin_lon);
        let up = (
            up_sign * cos_lat * cos_lon,
            up_sign * sin_lat,
            up_sign * cos_lat * sin_lon,
        );

        TangentBasis { east, north, up }
    }

    /// Interpolate between two positions along the great circle.
    #[must_use]
    pub fn slerp(&self, other: &CurvedPosition, factor: f32) -> Self {
        let factor = factor.clamp(0.0, 1.0);

        let lat1 = self.latitude;
        let lon1 = self.longitude;
        let lat2 = other.latitude;
        let lon2 = other.longitude;

        let d_lon = lon2 - lon1;
        let cos_lat1 = lat1.cos();
        let cos_lat2 = lat2.cos();
        let sin_lat1 = lat1.sin();
        let sin_lat2 = lat2.sin();

        let sigma = (sin_lat1 * sin_lat2 + cos_lat1 * cos_lat2 * d_lon.cos())
            .clamp(-1.0, 1.0)
            .acos();

        if sigma.abs() < 0.0001 {
            return *self;
        }

        let weight_self = ((1.0 - factor) * sigma).sin() / sigma.sin();
        let weight_other = (factor * sigma).sin() / sigma.sin();

        let interp_x = weight_self * cos_lat1 * lon1.cos() + weight_other * cos_lat2 * lon2.cos();
        let interp_y = weight_self * sin_lat1 + weight_other * sin_lat2;
        let interp_z = weight_self * cos_lat1 * lon1.sin() + weight_other * cos_lat2 * lon2.sin();

        let lat = interp_y.clamp(-1.0, 1.0).asin();
        let lon = interp_z.atan2(interp_x).rem_euclid(2.0 * PI);
        let elev = self.elevation + factor * (other.elevation - self.elevation);

        Self::new(lat, lon, elev)
    }

    /// Check if position is near a pole.
    #[must_use]
    pub fn is_near_pole(&self, threshold: f32) -> bool {
        self.latitude.abs() > (PI / 2.0 - threshold)
    }

    /// Calculate bearing to another position (0 = north, PI/2 = east).
    #[must_use]
    pub fn bearing_to(&self, other: &CurvedPosition) -> f32 {
        let d_lon = other.longitude - self.longitude;
        let y = d_lon.sin() * other.latitude.cos();
        let x = self.latitude.cos() * other.latitude.sin()
            - self.latitude.sin() * other.latitude.cos() * d_lon.cos();
        y.atan2(x).rem_euclid(2.0 * PI)
    }

    /// Move in a given bearing by a distance.
    #[must_use]
    pub fn move_along_bearing(&self, bearing: f32, distance: f32, radius: f32) -> Self {
        let angular_dist = distance / radius;

        let lat2 = (self.latitude.sin() * angular_dist.cos()
            + self.latitude.cos() * angular_dist.sin() * bearing.cos())
        .clamp(-1.0, 1.0)
        .asin();

        let lon2 = self.longitude
            + (bearing.sin() * angular_dist.sin() * self.latitude.cos())
                .atan2(angular_dist.cos() - self.latitude.sin() * lat2.sin());

        Self::new(lat2, lon2.rem_euclid(2.0 * PI), self.elevation)
    }

    /// Compute checksum for deterministic verification.
    #[must_use]
    #[expect(clippy::cast_possible_truncation, reason = "scaled values fit in u32")]
    pub fn checksum(&self) -> u32 {
        let lat_bits = ((self.latitude * 10000.0) as i32).unsigned_abs();
        let lon_bits = ((self.longitude * 10000.0) as i32).unsigned_abs();
        let elev_bits = ((self.elevation * 100.0) as i32).unsigned_abs();
        crc32fast::hash(
            &[
                lat_bits.to_le_bytes(),
                lon_bits.to_le_bytes(),
                elev_bits.to_le_bytes(),
            ]
            .concat(),
        )
    }
}

/// Tangent space basis vectors at a surface point.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct TangentBasis {
    /// East direction (longitude increasing).
    pub east: (f32, f32, f32),
    /// North direction (latitude increasing).
    pub north: (f32, f32, f32),
    /// Surface normal (up from surface).
    pub up: (f32, f32, f32),
}

impl TangentBasis {
    /// Project a world-space direction onto the tangent plane.
    #[must_use]
    pub fn project_to_tangent(&self, dx: f32, dy: f32, dz: f32) -> (f32, f32) {
        let east_component = dx * self.east.0 + dy * self.east.1 + dz * self.east.2;
        let north_component = dx * self.north.0 + dy * self.north.1 + dz * self.north.2;
        (east_component, north_component)
    }

    /// Convert tangent-space direction to world-space.
    #[must_use]
    pub fn tangent_to_world(&self, east: f32, north: f32) -> (f32, f32, f32) {
        (
            east * self.east.0 + north * self.north.0,
            east * self.east.1 + north * self.north.1,
            east * self.east.2 + north * self.north.2,
        )
    }
}

/// Discrete grid cell on the curved surface.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CurvedGridCell {
    /// Latitude index (0 = south pole, max = north pole).
    pub lat_idx: u32,
    /// Longitude index (0 to lon_divisions-1).
    pub lon_idx: u32,
    /// Surface this cell belongs to.
    pub surface_id: CurvedSurfaceId,
}

impl CurvedGridCell {
    /// Create a new grid cell.
    #[must_use]
    pub fn new(lat_idx: u32, lon_idx: u32, surface_id: CurvedSurfaceId) -> Self {
        Self {
            lat_idx,
            lon_idx,
            surface_id,
        }
    }

    /// Convert to continuous position (cell center).
    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "grid indices fit in f32")]
    pub fn to_position(&self, config: &CurvedSurfaceConfig) -> CurvedPosition {
        let lat = -PI / 2.0 + (self.lat_idx as f32 + 0.5) * PI / config.lat_divisions as f32;
        let lon = (self.lon_idx as f32 + 0.5) * 2.0 * PI / config.lon_divisions as f32;
        CurvedPosition::new(lat, lon, 0.0)
    }

    /// Get neighboring cells on the grid.
    #[must_use]
    pub fn neighbors(&self, config: &CurvedSurfaceConfig) -> Vec<CurvedGridCell> {
        let mut result = Vec::with_capacity(8);

        let north_lat = self.lat_idx + 1;
        let south_lat = self.lat_idx.saturating_sub(1);
        let east_lon = (self.lon_idx + 1) % config.lon_divisions;
        let west_lon = (self.lon_idx + config.lon_divisions - 1) % config.lon_divisions;

        if self.lat_idx > 0 {
            result.push(CurvedGridCell::new(
                south_lat,
                self.lon_idx,
                self.surface_id.clone(),
            ));
            result.push(CurvedGridCell::new(
                south_lat,
                east_lon,
                self.surface_id.clone(),
            ));
            result.push(CurvedGridCell::new(
                south_lat,
                west_lon,
                self.surface_id.clone(),
            ));
        }

        result.push(CurvedGridCell::new(
            self.lat_idx,
            east_lon,
            self.surface_id.clone(),
        ));
        result.push(CurvedGridCell::new(
            self.lat_idx,
            west_lon,
            self.surface_id.clone(),
        ));

        if north_lat < config.lat_divisions {
            result.push(CurvedGridCell::new(
                north_lat,
                self.lon_idx,
                self.surface_id.clone(),
            ));
            result.push(CurvedGridCell::new(
                north_lat,
                east_lon,
                self.surface_id.clone(),
            ));
            result.push(CurvedGridCell::new(
                north_lat,
                west_lon,
                self.surface_id.clone(),
            ));
        }

        result
    }

    /// Get cardinal (non-diagonal) neighbors only.
    #[must_use]
    pub fn cardinal_neighbors(&self, config: &CurvedSurfaceConfig) -> Vec<CurvedGridCell> {
        let mut result = Vec::with_capacity(4);

        if self.lat_idx > 0 {
            result.push(CurvedGridCell::new(
                self.lat_idx - 1,
                self.lon_idx,
                self.surface_id.clone(),
            ));
        }

        if self.lat_idx + 1 < config.lat_divisions {
            result.push(CurvedGridCell::new(
                self.lat_idx + 1,
                self.lon_idx,
                self.surface_id.clone(),
            ));
        }

        result.push(CurvedGridCell::new(
            self.lat_idx,
            (self.lon_idx + 1) % config.lon_divisions,
            self.surface_id.clone(),
        ));

        result.push(CurvedGridCell::new(
            self.lat_idx,
            (self.lon_idx + config.lon_divisions - 1) % config.lon_divisions,
            self.surface_id.clone(),
        ));

        result
    }

    /// Check if this cell is at a pole.
    #[must_use]
    pub fn is_at_pole(&self, config: &CurvedSurfaceConfig) -> bool {
        self.lat_idx == 0 || self.lat_idx >= config.lat_divisions - 1
    }

    /// Calculate geodesic distance to another cell.
    #[must_use]
    pub fn geodesic_distance(&self, other: &CurvedGridCell, config: &CurvedSurfaceConfig) -> f32 {
        let pos1 = self.to_position(config);
        let pos2 = other.to_position(config);
        pos1.geodesic_distance(&pos2, config)
    }
}

/// Annotation for a curved surface node.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CurvedNodeAnnotation {
    /// Cell location.
    pub cell: CurvedGridCell,
    /// Movement domain at this cell.
    pub domain: MovementDomain,
    /// Alternative domains available.
    pub alt_domains: BTreeSet<MovementDomain>,
    /// Local elevation adjustment.
    pub elevation: f32,
    /// Whether cell is passable.
    pub passable: bool,
    /// Terrain difficulty multiplier.
    pub difficulty: f32,
    /// Optional region tag.
    pub region_tag: Option<String>,
}

impl Default for CurvedNodeAnnotation {
    fn default() -> Self {
        Self {
            cell: CurvedGridCell::new(0, 0, CurvedSurfaceId::default()),
            domain: MovementDomain::Walking,
            alt_domains: BTreeSet::new(),
            passable: true,
            elevation: 0.0,
            difficulty: 1.0,
            region_tag: None,
        }
    }
}

impl CurvedNodeAnnotation {
    /// Create a new annotation.
    #[must_use]
    pub fn new(cell: CurvedGridCell, domain: MovementDomain) -> Self {
        Self {
            cell,
            domain,
            ..Default::default()
        }
    }

    /// Set passability.
    #[must_use]
    pub fn passable(mut self, passable: bool) -> Self {
        self.passable = passable;
        self
    }

    /// Set elevation.
    #[must_use]
    pub fn with_elevation(mut self, elevation: f32) -> Self {
        self.elevation = elevation;
        self
    }

    /// Set difficulty.
    #[must_use]
    pub fn with_difficulty(mut self, difficulty: f32) -> Self {
        self.difficulty = difficulty.max(0.1);
        self
    }

    /// Add alternative domain.
    #[must_use]
    pub fn with_alt_domain(mut self, domain: MovementDomain) -> Self {
        self.alt_domains.insert(domain);
        self
    }

    /// Set region tag.
    #[must_use]
    pub fn in_region(mut self, tag: impl Into<String>) -> Self {
        self.region_tag = Some(tag.into());
        self
    }

    /// Check if agent can use this node.
    #[must_use]
    pub fn is_usable_by(&self, caps: &AgentCapabilities) -> bool {
        if !self.passable {
            return false;
        }
        if caps.can_use_domain(self.domain) {
            return true;
        }
        self.alt_domains.iter().any(|d| caps.can_use_domain(*d))
    }

    /// Get movement cost for an agent.
    #[must_use]
    pub fn cost_for(&self, caps: &AgentCapabilities) -> f32 {
        let domain_cost = caps.cost_for_domain(self.domain);
        if domain_cost.is_impassable() {
            return f32::MAX;
        }
        self.difficulty * domain_cost.multiplier
    }
}

/// Result of curved-world pathfinding.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CurvedPathResult {
    /// Path found.
    Found(CurvedPath),
    /// Partial path found.
    Partial(CurvedPath),
    /// No path exists.
    NotFound(CurvedPathFailure),
    /// Search exceeded limits.
    LimitExceeded(CurvedPathLimitExceeded),
}

impl CurvedPathResult {
    /// Check if path was found.
    #[must_use]
    pub fn is_found(&self) -> bool {
        matches!(self, Self::Found(_))
    }

    /// Check if any path exists.
    #[must_use]
    pub fn has_path(&self) -> bool {
        matches!(self, Self::Found(_) | Self::Partial(_))
    }

    /// Get the path if available.
    #[must_use]
    pub fn path(&self) -> Option<&CurvedPath> {
        match self {
            Self::Found(p) | Self::Partial(p) => Some(p),
            _ => None,
        }
    }

    /// Extract the path.
    #[must_use]
    pub fn into_path(self) -> Option<CurvedPath> {
        match self {
            Self::Found(p) | Self::Partial(p) => Some(p),
            _ => None,
        }
    }
}

/// Reason for path failure.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CurvedPathFailure {
    /// Start cell is invalid.
    InvalidStart,
    /// Goal cell is invalid.
    InvalidGoal,
    /// No path connects start to goal.
    NoPath,
    /// Capability mismatch.
    CapabilityMismatch,
    /// Different surfaces.
    SurfaceMismatch,
}

/// Path limit exceeded info.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CurvedPathLimitExceeded {
    /// Which limit was hit.
    pub limit_type: CurvedPathLimitType,
    /// Partial path if any.
    pub partial_path: Option<CurvedPath>,
}

/// Type of limit exceeded.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CurvedPathLimitType {
    /// Iteration limit.
    Iterations,
    /// Cost limit.
    Cost,
    /// Length limit.
    Length,
}

/// A path on a curved surface.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CurvedPath {
    /// Waypoints along the path.
    pub waypoints: Vec<CurvedWaypoint>,
    /// Total geodesic distance.
    pub total_distance: f32,
    /// Total traversal cost.
    pub total_cost: f32,
    /// Surface this path is on.
    pub surface_id: CurvedSurfaceId,
    /// Whether path is partial.
    pub is_partial: bool,
    /// Tick when path was computed.
    pub computed_tick: u64,
}

impl CurvedPath {
    /// Create a new empty path.
    #[must_use]
    pub fn new(surface_id: CurvedSurfaceId, tick: u64) -> Self {
        Self {
            waypoints: Vec::new(),
            total_distance: 0.0,
            total_cost: 0.0,
            surface_id,
            is_partial: false,
            computed_tick: tick,
        }
    }

    /// Add a waypoint.
    pub fn add_waypoint(&mut self, waypoint: CurvedWaypoint) {
        self.waypoints.push(waypoint);
    }

    /// Get path length in waypoints.
    #[must_use]
    pub fn len(&self) -> usize {
        self.waypoints.len()
    }

    /// Check if path is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.waypoints.is_empty()
    }

    /// Get start position.
    #[must_use]
    pub fn start(&self) -> Option<&CurvedPosition> {
        self.waypoints.first().map(|w| &w.position)
    }

    /// Get goal position.
    #[must_use]
    pub fn goal(&self) -> Option<&CurvedPosition> {
        self.waypoints.last().map(|w| &w.position)
    }

    /// Mark as partial.
    #[must_use]
    pub fn as_partial(mut self) -> Self {
        self.is_partial = true;
        self
    }

    /// Compute checksum.
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "checksum values fit in bounds"
    )]
    pub fn checksum(&self) -> u32 {
        let mut data = Vec::new();
        data.extend_from_slice(&(self.waypoints.len() as u32).to_le_bytes());
        data.extend_from_slice(&self.total_distance.to_le_bytes());
        data.extend_from_slice(&self.total_cost.to_le_bytes());
        for wp in &self.waypoints {
            data.extend_from_slice(&wp.position.checksum().to_le_bytes());
        }
        crc32fast::hash(&data)
    }
}

/// A waypoint on a curved path.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CurvedWaypoint {
    /// Position.
    pub position: CurvedPosition,
    /// Grid cell.
    pub cell: CurvedGridCell,
    /// Movement domain.
    pub domain: MovementDomain,
    /// Cumulative cost.
    pub cumulative_cost: f32,
    /// Cumulative distance.
    pub cumulative_distance: f32,
    /// Bearing to next waypoint (if any).
    pub next_bearing: Option<f32>,
}

impl CurvedWaypoint {
    /// Create a new waypoint.
    #[must_use]
    pub fn new(position: CurvedPosition, cell: CurvedGridCell) -> Self {
        Self {
            position,
            cell,
            domain: MovementDomain::Walking,
            cumulative_cost: 0.0,
            cumulative_distance: 0.0,
            next_bearing: None,
        }
    }

    /// Set domain.
    #[must_use]
    pub fn with_domain(mut self, domain: MovementDomain) -> Self {
        self.domain = domain;
        self
    }

    /// Set cumulative cost.
    #[must_use]
    pub fn with_cost(mut self, cost: f32) -> Self {
        self.cumulative_cost = cost;
        self
    }

    /// Set cumulative distance.
    #[must_use]
    pub fn with_distance(mut self, dist: f32) -> Self {
        self.cumulative_distance = dist;
        self
    }

    /// Set next bearing.
    #[must_use]
    pub fn with_bearing(mut self, bearing: f32) -> Self {
        self.next_bearing = Some(bearing);
        self
    }
}

/// Configuration for curved-world pathfinding.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CurvedPathfindingConfig {
    /// Maximum iterations.
    pub max_iterations: u32,
    /// Maximum path cost.
    pub max_cost: f32,
    /// Maximum path length in cells.
    pub max_length: u32,
    /// Allow diagonal movement.
    pub allow_diagonal: bool,
    /// Heuristic weight.
    pub heuristic_weight: f32,
    /// Allow partial paths.
    pub allow_partial: bool,
}

impl Default for CurvedPathfindingConfig {
    fn default() -> Self {
        Self {
            max_iterations: 50000,
            max_cost: f32::MAX,
            max_length: 10000,
            allow_diagonal: true,
            heuristic_weight: 1.0,
            allow_partial: false,
        }
    }
}

/// Curved-world pathfinder using geodesic-aware A*.
pub struct CurvedPathfinder {
    surface_config: CurvedSurfaceConfig,
    pathfinding_config: CurvedPathfindingConfig,
    annotations: BTreeMap<(u32, u32), CurvedNodeAnnotation>,
}

impl CurvedPathfinder {
    /// Create a new curved-world pathfinder.
    #[must_use]
    pub fn new(surface_config: CurvedSurfaceConfig) -> Self {
        Self {
            surface_config,
            pathfinding_config: CurvedPathfindingConfig::default(),
            annotations: BTreeMap::new(),
        }
    }

    /// Set pathfinding config.
    #[must_use]
    pub fn with_pathfinding_config(mut self, config: CurvedPathfindingConfig) -> Self {
        self.pathfinding_config = config;
        self
    }

    /// Get surface config.
    #[must_use]
    pub fn surface_config(&self) -> &CurvedSurfaceConfig {
        &self.surface_config
    }

    /// Get pathfinding config.
    #[must_use]
    pub fn pathfinding_config(&self) -> &CurvedPathfindingConfig {
        &self.pathfinding_config
    }

    /// Set node annotation.
    pub fn set_annotation(&mut self, lat_idx: u32, lon_idx: u32, annotation: CurvedNodeAnnotation) {
        self.annotations.insert((lat_idx, lon_idx), annotation);
    }

    /// Get node annotation.
    #[must_use]
    pub fn get_annotation(&self, lat_idx: u32, lon_idx: u32) -> Option<&CurvedNodeAnnotation> {
        self.annotations.get(&(lat_idx, lon_idx))
    }

    /// Remove annotation.
    pub fn remove_annotation(
        &mut self,
        lat_idx: u32,
        lon_idx: u32,
    ) -> Option<CurvedNodeAnnotation> {
        self.annotations.remove(&(lat_idx, lon_idx))
    }

    /// Check if cell is passable.
    #[must_use]
    pub fn is_passable(&self, cell: &CurvedGridCell, caps: &AgentCapabilities) -> bool {
        if let Some(ann) = self.annotations.get(&(cell.lat_idx, cell.lon_idx)) {
            ann.is_usable_by(caps)
        } else {
            caps.can_use_domain(MovementDomain::Walking)
        }
    }

    /// Get movement cost between cells.
    #[must_use]
    pub fn movement_cost(
        &self,
        from: &CurvedGridCell,
        to: &CurvedGridCell,
        caps: &AgentCapabilities,
    ) -> f32 {
        let base_distance = from.geodesic_distance(to, &self.surface_config);

        let to_cost = if let Some(ann) = self.annotations.get(&(to.lat_idx, to.lon_idx)) {
            ann.cost_for(caps)
        } else {
            caps.cost_for_domain(MovementDomain::Walking).multiplier
        };

        base_distance * to_cost
    }

    /// Find path between positions.
    #[must_use]
    pub fn find_path(
        &self,
        start: CurvedPosition,
        goal: CurvedPosition,
        caps: &AgentCapabilities,
        tick: u64,
    ) -> CurvedPathResult {
        let start_cell = start.to_grid_cell(&self.surface_config);
        let goal_cell = goal.to_grid_cell(&self.surface_config);

        if start_cell.surface_id != goal_cell.surface_id {
            return CurvedPathResult::NotFound(CurvedPathFailure::SurfaceMismatch);
        }

        if !self.is_passable(&start_cell, caps) {
            return CurvedPathResult::NotFound(CurvedPathFailure::InvalidStart);
        }

        if !self.is_passable(&goal_cell, caps) {
            return CurvedPathResult::NotFound(CurvedPathFailure::InvalidGoal);
        }

        if start_cell == goal_cell {
            let mut path = CurvedPath::new(self.surface_config.surface_id.clone(), tick);
            path.add_waypoint(CurvedWaypoint::new(start, start_cell));
            return CurvedPathResult::Found(path);
        }

        self.astar_search(start, goal, &start_cell, &goal_cell, caps, tick)
    }

    fn astar_search(
        &self,
        _start_pos: CurvedPosition,
        _goal_pos: CurvedPosition,
        start_cell: &CurvedGridCell,
        goal_cell: &CurvedGridCell,
        caps: &AgentCapabilities,
        tick: u64,
    ) -> CurvedPathResult {
        let mut open_set = BinaryHeap::new();
        let mut closed_set = BTreeSet::new();
        let mut came_from: BTreeMap<(u32, u32), (u32, u32)> = BTreeMap::new();
        let mut g_score: BTreeMap<(u32, u32), f32> = BTreeMap::new();

        let start_key = (start_cell.lat_idx, start_cell.lon_idx);
        let goal_key = (goal_cell.lat_idx, goal_cell.lon_idx);

        g_score.insert(start_key, 0.0);

        let h_start = self.heuristic(start_cell, goal_cell);
        open_set.push(CurvedOpenNode {
            lat_idx: start_cell.lat_idx,
            lon_idx: start_cell.lon_idx,
            f_score: h_start,
        });

        let mut iterations = 0;
        let mut best_node: Option<(u32, u32)> = None;
        let mut best_h = f32::MAX;

        while let Some(current) = open_set.pop() {
            iterations += 1;
            if iterations > self.pathfinding_config.max_iterations {
                let search_state = SearchState {
                    came_from: &came_from,
                    g_score: &g_score,
                    start_cell,
                };
                return self.handle_limit_exceeded(
                    CurvedPathLimitType::Iterations,
                    best_node,
                    &search_state,
                    tick,
                );
            }

            let current_key = (current.lat_idx, current.lon_idx);

            if current_key == goal_key {
                return CurvedPathResult::Found(self.reconstruct_path(
                    current_key,
                    &came_from,
                    &g_score,
                    start_cell,
                    tick,
                ));
            }

            if closed_set.contains(&current_key) {
                continue;
            }
            closed_set.insert(current_key);

            let current_g = g_score.get(&current_key).copied().unwrap_or(f32::MAX);
            let current_cell = CurvedGridCell::new(
                current.lat_idx,
                current.lon_idx,
                self.surface_config.surface_id.clone(),
            );

            let neighbors = if self.pathfinding_config.allow_diagonal {
                current_cell.neighbors(&self.surface_config)
            } else {
                current_cell.cardinal_neighbors(&self.surface_config)
            };

            for neighbor in neighbors {
                let neighbor_key = (neighbor.lat_idx, neighbor.lon_idx);

                if closed_set.contains(&neighbor_key) {
                    continue;
                }

                if !self.is_passable(&neighbor, caps) {
                    continue;
                }

                let move_cost = self.movement_cost(&current_cell, &neighbor, caps);
                let tentative_g = current_g + move_cost;

                #[expect(clippy::cast_precision_loss, reason = "max_length fits in f32")]
                if tentative_g > self.pathfinding_config.max_cost
                    || tentative_g
                        > self.pathfinding_config.max_length as f32
                            * self.surface_config.lat_step()
                            * self.surface_config.radius
                {
                    continue;
                }

                let neighbor_g = g_score.get(&neighbor_key).copied().unwrap_or(f32::MAX);

                if tentative_g < neighbor_g {
                    came_from.insert(neighbor_key, current_key);
                    g_score.insert(neighbor_key, tentative_g);

                    let h = self.heuristic(&neighbor, goal_cell);
                    if h < best_h {
                        best_h = h;
                        best_node = Some(neighbor_key);
                    }

                    let f_score = tentative_g + h * self.pathfinding_config.heuristic_weight;
                    open_set.push(CurvedOpenNode {
                        lat_idx: neighbor.lat_idx,
                        lon_idx: neighbor.lon_idx,
                        f_score,
                    });
                }
            }
        }

        if self.pathfinding_config.allow_partial
            && let Some(best) = best_node
        {
            let path = self.reconstruct_path(best, &came_from, &g_score, start_cell, tick);
            return CurvedPathResult::Partial(path.as_partial());
        }

        CurvedPathResult::NotFound(CurvedPathFailure::NoPath)
    }

    fn heuristic(&self, from: &CurvedGridCell, to: &CurvedGridCell) -> f32 {
        from.geodesic_distance(to, &self.surface_config)
    }

    fn handle_limit_exceeded(
        &self,
        limit_type: CurvedPathLimitType,
        best_node: Option<(u32, u32)>,
        search_state: &SearchState<'_>,
        tick: u64,
    ) -> CurvedPathResult {
        let partial_path = if self.pathfinding_config.allow_partial {
            best_node.map(|node| {
                self.reconstruct_path(
                    node,
                    search_state.came_from,
                    search_state.g_score,
                    search_state.start_cell,
                    tick,
                )
                .as_partial()
            })
        } else {
            None
        };

        CurvedPathResult::LimitExceeded(CurvedPathLimitExceeded {
            limit_type,
            partial_path,
        })
    }

    fn reconstruct_path(
        &self,
        goal_key: (u32, u32),
        came_from: &BTreeMap<(u32, u32), (u32, u32)>,
        g_score: &BTreeMap<(u32, u32), f32>,
        start_cell: &CurvedGridCell,
        tick: u64,
    ) -> CurvedPath {
        let mut path_keys = vec![goal_key];
        let mut current = goal_key;

        while let Some(&prev) = came_from.get(&current) {
            path_keys.push(prev);
            current = prev;
        }

        path_keys.reverse();

        let mut path = CurvedPath::new(self.surface_config.surface_id.clone(), tick);
        let mut cumulative_distance = 0.0;

        for (i, &key) in path_keys.iter().enumerate() {
            let cell = CurvedGridCell::new(key.0, key.1, start_cell.surface_id.clone());
            let position = cell.to_position(&self.surface_config);

            let domain = self
                .annotations
                .get(&key)
                .map_or(MovementDomain::Walking, |ann| ann.domain);

            let cumulative_cost = g_score.get(&key).copied().unwrap_or(0.0);

            if i > 0 {
                let prev_key = path_keys[i - 1];
                let prev_cell =
                    CurvedGridCell::new(prev_key.0, prev_key.1, start_cell.surface_id.clone());
                cumulative_distance += cell.geodesic_distance(&prev_cell, &self.surface_config);
            }

            let next_bearing = if i + 1 < path_keys.len() {
                let next_key = path_keys[i + 1];
                let next_cell =
                    CurvedGridCell::new(next_key.0, next_key.1, start_cell.surface_id.clone());
                let next_pos = next_cell.to_position(&self.surface_config);
                Some(position.bearing_to(&next_pos))
            } else {
                None
            };

            let mut waypoint = CurvedWaypoint::new(position, cell)
                .with_domain(domain)
                .with_cost(cumulative_cost)
                .with_distance(cumulative_distance);

            if let Some(bearing) = next_bearing {
                waypoint = waypoint.with_bearing(bearing);
            }

            path.add_waypoint(waypoint);
        }

        path.total_cost = g_score.get(&goal_key).copied().unwrap_or(0.0);
        path.total_distance = cumulative_distance;

        path
    }

    /// Get annotation count.
    #[must_use]
    pub fn annotation_count(&self) -> usize {
        self.annotations.len()
    }

    /// Iterate over annotations.
    pub fn annotations(&self) -> impl Iterator<Item = &CurvedNodeAnnotation> {
        self.annotations.values()
    }

    /// Compute surface summary.
    #[must_use]
    pub fn summary(&self) -> CurvedSurfaceSummary {
        let mut domains = BTreeSet::new();
        let mut passable_count = 0;
        let mut impassable_count = 0;

        for ann in self.annotations.values() {
            domains.insert(ann.domain);
            for d in &ann.alt_domains {
                domains.insert(*d);
            }
            if ann.passable {
                passable_count += 1;
            } else {
                impassable_count += 1;
            }
        }

        CurvedSurfaceSummary {
            surface_id: self.surface_config.surface_id.clone(),
            geometry: self.surface_config.geometry,
            radius: self.surface_config.radius,
            lat_divisions: self.surface_config.lat_divisions,
            lon_divisions: self.surface_config.lon_divisions,
            annotation_count: self.annotations.len(),
            passable_count,
            impassable_count,
            available_domains: domains,
        }
    }

    /// Compute fingerprint.
    #[must_use]
    pub fn fingerprint(&self, tick: u64) -> CurvedWorldFingerprint {
        CurvedWorldFingerprint::from_pathfinder(self, tick)
    }
}

struct SearchState<'a> {
    came_from: &'a BTreeMap<(u32, u32), (u32, u32)>,
    g_score: &'a BTreeMap<(u32, u32), f32>,
    start_cell: &'a CurvedGridCell,
}

struct CurvedOpenNode {
    lat_idx: u32,
    lon_idx: u32,
    f_score: f32,
}

impl PartialEq for CurvedOpenNode {
    fn eq(&self, other: &Self) -> bool {
        self.lat_idx == other.lat_idx && self.lon_idx == other.lon_idx
    }
}

impl Eq for CurvedOpenNode {}

impl Ord for CurvedOpenNode {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other
            .f_score
            .partial_cmp(&self.f_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    }
}

impl PartialOrd for CurvedOpenNode {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Summary of a curved surface.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CurvedSurfaceSummary {
    /// Surface identifier.
    pub surface_id: CurvedSurfaceId,
    /// Geometry type.
    pub geometry: SurfaceGeometry,
    /// Radius.
    pub radius: f32,
    /// Latitude divisions.
    pub lat_divisions: u32,
    /// Longitude divisions.
    pub lon_divisions: u32,
    /// Total annotation count.
    pub annotation_count: usize,
    /// Passable cell count.
    pub passable_count: usize,
    /// Impassable cell count.
    pub impassable_count: usize,
    /// Available movement domains.
    pub available_domains: BTreeSet<MovementDomain>,
}

impl CurvedSurfaceSummary {
    /// Total cell count.
    #[must_use]
    pub fn total_cells(&self) -> usize {
        (self.lat_divisions as usize) * (self.lon_divisions as usize)
    }

    /// Surface area.
    #[must_use]
    pub fn surface_area(&self) -> f32 {
        4.0 * PI * self.radius * self.radius
    }
}

/// Projection for estimating future curved-world state.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CurvedWorldProjection {
    /// Surface being projected.
    pub surface_id: CurvedSurfaceId,
    /// Tick of projection.
    pub projection_tick: u64,
    /// Estimated connectivity changes.
    pub connectivity_changes: Vec<CurvedConnectivityChange>,
    /// Estimated passability changes.
    pub passability_changes: Vec<CurvedPassabilityChange>,
}

/// Predicted connectivity change.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CurvedConnectivityChange {
    /// Affected region.
    pub region_tag: String,
    /// Change type.
    pub change_type: ConnectivityChangeType,
    /// Estimated tick.
    pub estimated_tick: u64,
}

/// Type of connectivity change.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectivityChangeType {
    /// Region becoming disconnected.
    Disconnection,
    /// Region becoming connected.
    Connection,
    /// Domain becoming available.
    DomainAdded(MovementDomain),
    /// Domain becoming unavailable.
    DomainRemoved(MovementDomain),
}

/// Predicted passability change.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CurvedPassabilityChange {
    /// Affected cell.
    pub cell: CurvedGridCell,
    /// New passability state.
    pub new_passable: bool,
    /// Estimated tick.
    pub estimated_tick: u64,
}

/// Deterministic fingerprint for curved-world state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CurvedWorldFingerprint(pub u32);

impl CurvedWorldFingerprint {
    #[must_use]
    pub fn raw(self) -> u32 {
        self.0
    }

    #[must_use]
    pub fn matches(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    #[must_use]
    pub fn from_pathfinder(pathfinder: &CurvedPathfinder, tick: u64) -> Self {
        let mut hasher = StableHasher::new();

        tick.hash(&mut hasher);
        pathfinder.surface_config.surface_id.0.hash(&mut hasher);

        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "radius scaled for hashing"
        )]
        let radius_bits = (pathfinder.surface_config.radius * 100.0) as u32;
        radius_bits.hash(&mut hasher);

        pathfinder.surface_config.lat_divisions.hash(&mut hasher);
        pathfinder.surface_config.lon_divisions.hash(&mut hasher);
        pathfinder.annotations.len().hash(&mut hasher);

        for ((lat, lon), ann) in &pathfinder.annotations {
            lat.hash(&mut hasher);
            lon.hash(&mut hasher);
            ann.passable.hash(&mut hasher);
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "difficulty scaled for hashing"
            )]
            let diff_bits = (ann.difficulty * 1000.0) as u32;
            diff_bits.hash(&mut hasher);
        }

        Self(hasher.finish_u32())
    }
}

impl fmt::Display for CurvedWorldFingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "curved:{:08x}", self.0)
    }
}

struct StableHasher {
    state: u64,
}

impl StableHasher {
    fn new() -> Self {
        Self {
            state: 0xcbf2_9ce4_8422_2325,
        }
    }

    #[expect(
        clippy::cast_possible_truncation,
        reason = "intentional truncation for u32 hash"
    )]
    fn finish_u32(&self) -> u32 {
        (self.state ^ (self.state >> 32)) as u32
    }
}

impl Hasher for StableHasher {
    fn finish(&self) -> u64 {
        self.state
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.state ^= u64::from(*byte);
            self.state = self.state.wrapping_mul(0x0100_0000_01b3);
        }
    }
}

/// Snapshot of curved-world state for serialization.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CurvedWorldSnapshot {
    /// Surface configuration.
    pub surface_config: CurvedSurfaceConfig,
    /// Pathfinding configuration.
    pub pathfinding_config: CurvedPathfindingConfig,
    /// All annotations.
    pub annotations: Vec<CurvedNodeAnnotation>,
    /// Snapshot tick.
    pub tick: u64,
    /// Fingerprint at snapshot time.
    pub fingerprint: CurvedWorldFingerprint,
}

impl CurvedWorldSnapshot {
    /// Create snapshot from pathfinder.
    #[must_use]
    pub fn from_pathfinder(pathfinder: &CurvedPathfinder, tick: u64) -> Self {
        Self {
            surface_config: pathfinder.surface_config.clone(),
            pathfinding_config: pathfinder.pathfinding_config.clone(),
            annotations: pathfinder.annotations.values().cloned().collect(),
            tick,
            fingerprint: pathfinder.fingerprint(tick),
        }
    }

    /// Restore pathfinder from snapshot.
    #[must_use]
    pub fn to_pathfinder(&self) -> CurvedPathfinder {
        let mut pathfinder = CurvedPathfinder::new(self.surface_config.clone())
            .with_pathfinding_config(self.pathfinding_config.clone());

        for ann in &self.annotations {
            pathfinder.set_annotation(ann.cell.lat_idx, ann.cell.lon_idx, ann.clone());
        }

        pathfinder
    }

    /// Compute checksum.
    #[must_use]
    pub fn checksum(&self) -> u32 {
        self.fingerprint.raw()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_config() -> CurvedSurfaceConfig {
        CurvedSurfaceConfig::spherical_exterior(1000.0, "test_planet").with_resolution(36, 72)
    }

    #[test]
    fn test_surface_geometry() {
        assert!(SurfaceGeometry::SphericalExterior.gravity_inward());
        assert!(!SurfaceGeometry::SphericalInterior.gravity_inward());
        assert!((SurfaceGeometry::SphericalExterior.up_sign() - 1.0).abs() < f32::EPSILON);
        assert!((SurfaceGeometry::SphericalInterior.up_sign() + 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_curved_surface_config() {
        let config = make_test_config();
        assert_eq!(config.geometry, SurfaceGeometry::SphericalExterior);
        assert!((config.radius - 1000.0).abs() < f32::EPSILON);
        assert_eq!(config.lat_divisions, 36);
        assert_eq!(config.lon_divisions, 72);
    }

    #[test]
    fn test_curved_position_new() {
        let pos = CurvedPosition::new(0.5, 1.0, 10.0);
        assert!((pos.latitude - 0.5).abs() < f32::EPSILON);
        assert!((pos.longitude - 1.0).abs() < f32::EPSILON);
        assert!((pos.elevation - 10.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_curved_position_clamping() {
        let pos = CurvedPosition::new(10.0, -1.0, 0.0);
        assert!((pos.latitude - PI / 2.0).abs() < f32::EPSILON);
        assert!(pos.longitude >= 0.0 && pos.longitude < 2.0 * PI);
    }

    #[test]
    fn test_curved_position_poles() {
        let north = CurvedPosition::north_pole();
        let south = CurvedPosition::south_pole();
        assert!((north.latitude - PI / 2.0).abs() < f32::EPSILON);
        assert!((south.latitude + PI / 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_curved_position_cartesian_roundtrip() {
        let config = make_test_config();
        let original = CurvedPosition::new(0.3, 1.2, 50.0);
        let (x, y, z) = original.to_cartesian(&config);
        let restored = CurvedPosition::from_cartesian(x, y, z, &config);

        assert!((original.latitude - restored.latitude).abs() < 0.001);
        assert!((original.longitude - restored.longitude).abs() < 0.001);
        assert!((original.elevation - restored.elevation).abs() < 0.1);
    }

    #[test]
    fn test_curved_position_geodesic_distance() {
        let config = make_test_config();
        let pos1 = CurvedPosition::equator(0.0);
        let pos2 = CurvedPosition::equator(PI);

        let dist = pos1.geodesic_distance(&pos2, &config);
        let expected = PI * config.radius;
        assert!((dist - expected).abs() < 1.0);
    }

    #[test]
    fn test_curved_position_geodesic_distance_same_point() {
        let config = make_test_config();
        let pos = CurvedPosition::new(0.5, 1.0, 0.0);
        let dist = pos.geodesic_distance(&pos, &config);
        assert!(dist.abs() < 0.001);
    }

    #[test]
    fn test_curved_position_slerp() {
        let pos1 = CurvedPosition::equator(0.0);
        let pos2 = CurvedPosition::equator(PI / 2.0);

        let mid = pos1.slerp(&pos2, 0.5);
        assert!((mid.latitude).abs() < 0.001);
        assert!((mid.longitude - PI / 4.0).abs() < 0.01);
    }

    #[test]
    fn test_curved_position_bearing() {
        let pos1 = CurvedPosition::equator(0.0);
        let pos2 = CurvedPosition::north_pole();

        let bearing = pos1.bearing_to(&pos2);
        assert!(bearing.abs() < 0.01);
    }

    #[test]
    fn test_curved_grid_cell_to_position() {
        let config = make_test_config();
        let cell = CurvedGridCell::new(18, 36, CurvedSurfaceId::new("test"));
        let pos = cell.to_position(&config);

        assert!(pos.latitude.abs() < 0.1);
        assert!((pos.longitude - PI).abs() < 0.1);
    }

    #[test]
    fn test_curved_grid_cell_neighbors() {
        let config = make_test_config();
        let cell = CurvedGridCell::new(18, 36, CurvedSurfaceId::new("test"));
        let neighbors = cell.neighbors(&config);

        assert_eq!(neighbors.len(), 8);
    }

    #[test]
    fn test_curved_grid_cell_neighbors_at_pole() {
        let config = make_test_config();
        let cell = CurvedGridCell::new(0, 10, CurvedSurfaceId::new("test"));
        let neighbors = cell.neighbors(&config);

        assert!(neighbors.len() < 8);
    }

    #[test]
    fn test_curved_grid_cell_cardinal_neighbors() {
        let config = make_test_config();
        let cell = CurvedGridCell::new(18, 36, CurvedSurfaceId::new("test"));
        let neighbors = cell.cardinal_neighbors(&config);

        assert_eq!(neighbors.len(), 4);
    }

    #[test]
    fn test_curved_node_annotation() {
        let cell = CurvedGridCell::new(10, 20, CurvedSurfaceId::new("test"));
        let ann = CurvedNodeAnnotation::new(cell, MovementDomain::Walking)
            .with_difficulty(1.5)
            .with_elevation(10.0)
            .passable(true);

        assert!(ann.passable);
        assert!((ann.difficulty - 1.5).abs() < f32::EPSILON);
        assert!((ann.elevation - 10.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_curved_pathfinder_new() {
        let config = make_test_config();
        let pathfinder = CurvedPathfinder::new(config);

        assert_eq!(pathfinder.annotation_count(), 0);
    }

    #[test]
    fn test_curved_pathfinder_annotations() {
        let config = make_test_config();
        let mut pathfinder = CurvedPathfinder::new(config);

        let cell = CurvedGridCell::new(10, 20, CurvedSurfaceId::new("test_planet"));
        let ann = CurvedNodeAnnotation::new(cell.clone(), MovementDomain::Walking);
        pathfinder.set_annotation(10, 20, ann);

        assert_eq!(pathfinder.annotation_count(), 1);
        assert!(pathfinder.get_annotation(10, 20).is_some());

        pathfinder.remove_annotation(10, 20);
        assert_eq!(pathfinder.annotation_count(), 0);
    }

    #[test]
    fn test_curved_pathfinder_find_path_same_cell() {
        let config = make_test_config();
        let pathfinder = CurvedPathfinder::new(config);
        let caps = AgentCapabilities::default();

        let pos = CurvedPosition::new(0.5, 1.0, 0.0);
        let result = pathfinder.find_path(pos, pos, &caps, 0);

        assert!(result.is_found());
        let path = result.path().unwrap();
        assert_eq!(path.len(), 1);
    }

    #[test]
    fn test_curved_pathfinder_find_path_short() {
        let config = make_test_config();
        let pathfinder = CurvedPathfinder::new(config.clone());
        let caps = AgentCapabilities::default();

        let start = CurvedPosition::new(0.0, 0.0, 0.0);
        let goal = CurvedPosition::new(0.0, config.lon_step() * 2.0, 0.0);

        let result = pathfinder.find_path(start, goal, &caps, 0);

        assert!(result.has_path());
    }

    #[test]
    fn test_curved_pathfinder_blocked_start() {
        let config = make_test_config();
        let mut pathfinder = CurvedPathfinder::new(config);

        let start_pos = CurvedPosition::new(0.0, 0.0, 0.0);
        let start_cell = start_pos.to_grid_cell(pathfinder.surface_config());

        let ann = CurvedNodeAnnotation::new(start_cell, MovementDomain::Walking).passable(false);
        pathfinder.set_annotation(ann.cell.lat_idx, ann.cell.lon_idx, ann);

        let caps = AgentCapabilities::default();
        let goal = CurvedPosition::new(0.5, 0.5, 0.0);
        let result = pathfinder.find_path(start_pos, goal, &caps, 0);

        assert!(matches!(
            result,
            CurvedPathResult::NotFound(CurvedPathFailure::InvalidStart)
        ));
    }

    #[test]
    fn test_curved_path_checksum() {
        let mut path = CurvedPath::new(CurvedSurfaceId::new("test"), 0);
        let cell = CurvedGridCell::new(10, 20, CurvedSurfaceId::new("test"));
        path.add_waypoint(CurvedWaypoint::new(
            CurvedPosition::new(0.1, 0.2, 0.0),
            cell,
        ));
        path.total_cost = 100.0;
        path.total_distance = 500.0;

        let checksum = path.checksum();
        assert_ne!(checksum, 0);

        let checksum2 = path.checksum();
        assert_eq!(checksum, checksum2);
    }

    #[test]
    fn test_curved_surface_summary() {
        let config = make_test_config();
        let mut pathfinder = CurvedPathfinder::new(config);

        let cell1 = CurvedGridCell::new(10, 20, CurvedSurfaceId::new("test_planet"));
        let ann1 = CurvedNodeAnnotation::new(cell1, MovementDomain::Walking);
        pathfinder.set_annotation(10, 20, ann1);

        let cell2 = CurvedGridCell::new(11, 21, CurvedSurfaceId::new("test_planet"));
        let ann2 = CurvedNodeAnnotation::new(cell2, MovementDomain::Swimming).passable(false);
        pathfinder.set_annotation(11, 21, ann2);

        let summary = pathfinder.summary();

        assert_eq!(summary.annotation_count, 2);
        assert_eq!(summary.passable_count, 1);
        assert_eq!(summary.impassable_count, 1);
        assert!(summary.available_domains.contains(&MovementDomain::Walking));
        assert!(
            summary
                .available_domains
                .contains(&MovementDomain::Swimming)
        );
    }

    #[test]
    fn test_curved_world_fingerprint() {
        let config = make_test_config();
        let pathfinder1 = CurvedPathfinder::new(config.clone());
        let pathfinder2 = CurvedPathfinder::new(config);

        let fp1 = pathfinder1.fingerprint(0);
        let fp2 = pathfinder2.fingerprint(0);

        assert!(fp1.matches(&fp2));
    }

    #[test]
    fn test_curved_world_fingerprint_changes_with_state() {
        let config = make_test_config();
        let mut pathfinder = CurvedPathfinder::new(config);

        let fp_empty = pathfinder.fingerprint(0);

        let cell = CurvedGridCell::new(10, 20, CurvedSurfaceId::new("test_planet"));
        let ann = CurvedNodeAnnotation::new(cell, MovementDomain::Walking);
        pathfinder.set_annotation(10, 20, ann);

        let fp_with_ann = pathfinder.fingerprint(0);

        assert!(!fp_empty.matches(&fp_with_ann));
    }

    #[test]
    fn test_curved_world_fingerprint_changes_with_tick() {
        let config = make_test_config();
        let pathfinder = CurvedPathfinder::new(config);

        let fp0 = pathfinder.fingerprint(0);
        let fp1 = pathfinder.fingerprint(1);

        assert!(!fp0.matches(&fp1));
    }

    #[test]
    fn test_curved_world_fingerprint_display() {
        let fp = CurvedWorldFingerprint(0xdead_beef);
        assert_eq!(format!("{fp}"), "curved:deadbeef");
    }

    #[test]
    fn test_curved_world_snapshot_roundtrip() {
        let config = make_test_config();
        let mut pathfinder = CurvedPathfinder::new(config);

        let cell = CurvedGridCell::new(10, 20, CurvedSurfaceId::new("test_planet"));
        let ann = CurvedNodeAnnotation::new(cell, MovementDomain::Walking).with_difficulty(1.5);
        pathfinder.set_annotation(10, 20, ann);

        let snapshot = CurvedWorldSnapshot::from_pathfinder(&pathfinder, 100);
        let restored = snapshot.to_pathfinder();

        let fp_original = pathfinder.fingerprint(100);
        let fp_restored = restored.fingerprint(100);

        assert!(fp_original.matches(&fp_restored));
        assert_eq!(pathfinder.annotation_count(), restored.annotation_count());
    }

    #[test]
    fn test_curved_world_snapshot_serde() {
        let config = make_test_config();
        let pathfinder = CurvedPathfinder::new(config);
        let snapshot = CurvedWorldSnapshot::from_pathfinder(&pathfinder, 0);

        let json = serde_json::to_string(&snapshot).unwrap();
        let restored: CurvedWorldSnapshot = serde_json::from_str(&json).unwrap();

        assert!(snapshot.fingerprint.matches(&restored.fingerprint));
    }

    #[test]
    fn test_curved_position_serde() {
        let pos = CurvedPosition::new(0.5, 1.2, 10.0);
        let json = serde_json::to_string(&pos).unwrap();
        let restored: CurvedPosition = serde_json::from_str(&json).unwrap();

        assert!((pos.latitude - restored.latitude).abs() < f32::EPSILON);
        assert!((pos.longitude - restored.longitude).abs() < f32::EPSILON);
        assert!((pos.elevation - restored.elevation).abs() < f32::EPSILON);
    }

    #[test]
    fn test_curved_grid_cell_serde() {
        let cell = CurvedGridCell::new(10, 20, CurvedSurfaceId::new("planet_x"));
        let json = serde_json::to_string(&cell).unwrap();
        let restored: CurvedGridCell = serde_json::from_str(&json).unwrap();

        assert_eq!(cell, restored);
    }

    #[test]
    fn test_curved_path_serde() {
        let mut path = CurvedPath::new(CurvedSurfaceId::new("test"), 100);
        let cell = CurvedGridCell::new(5, 10, CurvedSurfaceId::new("test"));
        path.add_waypoint(CurvedWaypoint::new(
            CurvedPosition::new(0.1, 0.2, 0.0),
            cell,
        ));
        path.total_cost = 50.0;
        path.total_distance = 200.0;

        let json = serde_json::to_string(&path).unwrap();
        let restored: CurvedPath = serde_json::from_str(&json).unwrap();

        assert_eq!(path.len(), restored.len());
        assert!((path.total_cost - restored.total_cost).abs() < f32::EPSILON);
    }

    #[test]
    fn test_curved_surface_config_serde() {
        let config = make_test_config();
        let json = serde_json::to_string(&config).unwrap();
        let restored: CurvedSurfaceConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(config.geometry, restored.geometry);
        assert!((config.radius - restored.radius).abs() < f32::EPSILON);
    }

    #[test]
    fn test_bincode_serialization() {
        let config = make_test_config();
        let mut pathfinder = CurvedPathfinder::new(config);

        let cell = CurvedGridCell::new(15, 30, CurvedSurfaceId::new("test_planet"));
        let ann = CurvedNodeAnnotation::new(cell, MovementDomain::Walking);
        pathfinder.set_annotation(15, 30, ann);

        let snapshot = CurvedWorldSnapshot::from_pathfinder(&pathfinder, 42);

        let encoded = bincode::serialize(&snapshot).unwrap();
        let decoded: CurvedWorldSnapshot = bincode::deserialize(&encoded).unwrap();

        assert!(snapshot.fingerprint.matches(&decoded.fingerprint));
        assert_eq!(snapshot.tick, decoded.tick);
    }

    #[test]
    fn test_tangent_basis() {
        let config = make_test_config();
        let pos = CurvedPosition::equator(0.0);
        let basis = pos.tangent_basis(&config);

        let (ex, ey, ez) = basis.east;
        let east_len = (ex * ex + ey * ey + ez * ez).sqrt();
        assert!((east_len - 1.0).abs() < 0.001);

        let (nx, ny, nz) = basis.north;
        let north_len = (nx * nx + ny * ny + nz * nz).sqrt();
        assert!((north_len - 1.0).abs() < 0.001);

        let dot = ex * nx + ey * ny + ez * nz;
        assert!(dot.abs() < 0.001);
    }

    #[test]
    fn test_interior_surface() {
        let config =
            CurvedSurfaceConfig::spherical_interior(500.0, "dyson").with_resolution(18, 36);

        assert_eq!(config.geometry, SurfaceGeometry::SphericalInterior);
        assert!((config.radius - 500.0).abs() < f32::EPSILON);

        let pos = CurvedPosition::new(0.0, 0.0, 0.0);
        let (x, y, z) = pos.to_cartesian(&config);

        let dist_from_center = (x * x + y * y + z * z).sqrt();
        assert!((dist_from_center - 500.0).abs() < 1.0);
    }

    #[test]
    fn test_position_checksum_determinism() {
        let pos1 = CurvedPosition::new(0.12345, 1.23456, 10.5);
        let pos2 = CurvedPosition::new(0.12345, 1.23456, 10.5);

        assert_eq!(pos1.checksum(), pos2.checksum());

        let pos3 = CurvedPosition::new(0.12345, 2.0, 10.5);
        assert_ne!(pos1.checksum(), pos3.checksum());
    }

    #[test]
    fn test_path_result_variants() {
        let path = CurvedPath::new(CurvedSurfaceId::new("test"), 0);
        let found = CurvedPathResult::Found(path.clone());
        let partial = CurvedPathResult::Partial(path);
        let not_found = CurvedPathResult::NotFound(CurvedPathFailure::NoPath);

        assert!(found.is_found());
        assert!(found.has_path());
        assert!(!partial.is_found());
        assert!(partial.has_path());
        assert!(!not_found.is_found());
        assert!(!not_found.has_path());
    }

    #[test]
    fn test_move_along_bearing() {
        let pos = CurvedPosition::equator(0.0);
        let radius = 1000.0;
        let distance = 100.0;

        let north = pos.move_along_bearing(0.0, distance, radius);
        assert!(north.latitude > pos.latitude);

        let east = pos.move_along_bearing(PI / 2.0, distance, radius);
        assert!(east.longitude > pos.longitude);
    }

    #[test]
    fn test_is_near_pole() {
        let north = CurvedPosition::north_pole();
        let equator = CurvedPosition::equator(0.0);

        assert!(north.is_near_pole(0.1));
        assert!(!equator.is_near_pole(0.1));
    }
}
