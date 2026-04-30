//! Fault line simulation with stress accumulation and earthquake generation.

use serde::{Deserialize, Serialize};

use super::config::FaultConfig;
use super::identity::FeatureId;

/// Type of geological fault.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
#[repr(u8)]
pub enum FaultType {
    /// Normal fault (extensional).
    #[default]
    Normal = 0,
    /// Reverse/thrust fault (compressional).
    Reverse = 1,
    /// Strike-slip fault (lateral movement).
    StrikeSlip = 2,
    /// Oblique fault (combination).
    Oblique = 3,
    /// Transform fault (plate boundary).
    Transform = 4,
}

impl FaultType {
    pub const ALL: [FaultType; 5] = [
        Self::Normal,
        Self::Reverse,
        Self::StrikeSlip,
        Self::Oblique,
        Self::Transform,
    ];

    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Reverse => "reverse",
            Self::StrikeSlip => "strike_slip",
            Self::Oblique => "oblique",
            Self::Transform => "transform",
        }
    }

    #[must_use]
    pub const fn stress_multiplier(&self) -> f32 {
        match self {
            Self::Normal => 0.8,
            Self::Reverse => 1.2,
            Self::StrikeSlip => 1.0,
            Self::Oblique => 1.1,
            Self::Transform => 1.5,
        }
    }

    #[must_use]
    pub const fn slip_direction(&self) -> (f32, f32, f32) {
        match self {
            Self::Normal => (0.0, 0.0, -1.0),
            Self::Reverse => (0.0, 0.0, 1.0),
            Self::StrikeSlip | Self::Transform => (1.0, 0.0, 0.0),
            Self::Oblique => (0.7, 0.0, 0.7),
        }
    }

    #[must_use]
    pub const fn typical_dip(&self) -> f32 {
        match self {
            Self::Normal => 60.0,
            Self::Reverse => 30.0,
            Self::StrikeSlip | Self::Transform => 90.0,
            Self::Oblique => 45.0,
        }
    }

    #[must_use]
    pub const fn from_raw(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Normal),
            1 => Some(Self::Reverse),
            2 => Some(Self::StrikeSlip),
            3 => Some(Self::Oblique),
            4 => Some(Self::Transform),
            _ => None,
        }
    }
}

/// State of slip activity on a fault.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum SlipState {
    /// Fault is locked, stress accumulating.
    #[default]
    Locked = 0,
    /// Minor creep occurring.
    Creeping = 1,
    /// Minor slip event (small earthquake).
    MinorSlip = 2,
    /// Major slip event (significant earthquake).
    MajorSlip = 3,
    /// Post-slip recovery period.
    Recovering = 4,
}

impl SlipState {
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Locked => "locked",
            Self::Creeping => "creeping",
            Self::MinorSlip => "minor_slip",
            Self::MajorSlip => "major_slip",
            Self::Recovering => "recovering",
        }
    }

    #[must_use]
    pub const fn is_slipping(&self) -> bool {
        matches!(self, Self::MinorSlip | Self::MajorSlip)
    }

    #[must_use]
    pub const fn is_active(&self) -> bool {
        !matches!(self, Self::Locked)
    }
}

/// Tracks stress accumulation on a fault.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct StressAccumulator {
    /// Current accumulated stress.
    pub current: f32,
    /// Maximum recorded stress.
    pub peak: f32,
    /// Total stress released through slips.
    pub total_released: f32,
    /// Number of slip events.
    pub slip_count: u32,
}

impl StressAccumulator {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_initial_stress(mut self, stress: f32) -> Self {
        self.current = stress.max(0.0);
        self.peak = self.current;
        self
    }

    pub fn accumulate(&mut self, amount: f32) {
        self.current = (self.current + amount).max(0.0);
        if self.current > self.peak {
            self.peak = self.current;
        }
    }

    pub fn release(&mut self, factor: f32) -> f32 {
        let released = self.current * factor.clamp(0.0, 1.0);
        self.current -= released;
        self.total_released += released;
        self.slip_count += 1;
        released
    }

    pub fn decay(&mut self, rate: f32) {
        self.current = (self.current * (1.0 - rate)).max(0.0);
    }

    #[must_use]
    pub fn stress_ratio(&self, threshold: f32) -> f32 {
        if threshold > 0.0 {
            self.current / threshold
        } else {
            0.0
        }
    }
}

/// A geological fault line.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FaultLine {
    /// Unique identifier.
    pub id: FeatureId,
    /// Fault type.
    pub fault_type: FaultType,
    /// Start position (x, y, z).
    pub start: (f32, f32, f32),
    /// End position (x, y, z).
    pub end: (f32, f32, f32),
    /// Depth extent below start/end points.
    pub depth_extent: f32,
    /// Dip angle in degrees.
    pub dip: f32,
    /// Strike direction in degrees from north.
    pub strike: f32,
    /// Stress accumulator.
    stress: StressAccumulator,
    /// Current slip state.
    state: SlipState,
    /// Friction coefficient (higher = more stress before slip).
    pub friction: f32,
    /// Recovery ticks remaining after slip.
    recovery_ticks: u32,
    /// Last tick processed.
    last_tick: u64,
}

impl FaultLine {
    #[must_use]
    pub fn new(
        id: FeatureId,
        fault_type: FaultType,
        start: (f32, f32, f32),
        end: (f32, f32, f32),
    ) -> Self {
        Self {
            id,
            fault_type,
            start,
            end,
            depth_extent: 50.0,
            dip: fault_type.typical_dip(),
            strike: 0.0,
            stress: StressAccumulator::new(),
            state: SlipState::Locked,
            friction: 0.6,
            recovery_ticks: 0,
            last_tick: 0,
        }
    }

    #[must_use]
    pub fn with_depth_extent(mut self, extent: f32) -> Self {
        self.depth_extent = extent.max(1.0);
        self
    }

    #[must_use]
    pub fn with_dip(mut self, dip: f32) -> Self {
        self.dip = dip.clamp(0.0, 90.0);
        self
    }

    #[must_use]
    pub fn with_strike(mut self, strike: f32) -> Self {
        self.strike = strike % 360.0;
        self
    }

    #[must_use]
    pub fn with_friction(mut self, friction: f32) -> Self {
        self.friction = friction.clamp(0.1, 1.0);
        self
    }

    #[must_use]
    pub fn with_initial_stress(mut self, stress: f32) -> Self {
        self.stress = StressAccumulator::new().with_initial_stress(stress);
        self
    }

    #[must_use]
    pub fn stress(&self) -> &StressAccumulator {
        &self.stress
    }

    #[must_use]
    pub fn state(&self) -> SlipState {
        self.state
    }

    #[must_use]
    pub fn length(&self) -> f32 {
        let dx = self.end.0 - self.start.0;
        let dy = self.end.1 - self.start.1;
        let dz = self.end.2 - self.start.2;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    #[must_use]
    pub fn area(&self) -> f32 {
        self.length() * self.depth_extent
    }

    #[must_use]
    pub fn center(&self) -> (f32, f32, f32) {
        (
            f32::midpoint(self.start.0, self.end.0),
            f32::midpoint(self.start.1, self.end.1),
            f32::midpoint(self.start.2, self.end.2),
        )
    }

    #[must_use]
    pub fn effective_stress_rate(&self, base_rate: f32) -> f32 {
        base_rate * self.fault_type.stress_multiplier() * self.friction
    }

    pub fn tick(&mut self, config: &FaultConfig, current_tick: u64) -> Option<QuakeEvent> {
        if current_tick <= self.last_tick {
            return None;
        }
        self.last_tick = current_tick;

        if self.recovery_ticks > 0 {
            self.recovery_ticks -= 1;
            if self.recovery_ticks == 0 {
                self.state = SlipState::Locked;
            }
            return None;
        }

        let stress_rate = self.effective_stress_rate(config.stress_rate);
        self.stress.accumulate(stress_rate);

        self.stress.decay(config.stress_decay_rate);

        if self.stress.current >= config.major_slip_threshold {
            return Some(self.trigger_slip(config, current_tick, true));
        }

        if self.stress.current >= config.minor_slip_threshold {
            if self.state != SlipState::Creeping {
                self.state = SlipState::Creeping;
            }

            let slip_chance = (self.stress.current - config.minor_slip_threshold)
                / (config.major_slip_threshold - config.minor_slip_threshold);

            if slip_chance > 0.5 {
                return Some(self.trigger_slip(config, current_tick, false));
            }
        }

        None
    }

    fn trigger_slip(&mut self, config: &FaultConfig, tick: u64, major: bool) -> QuakeEvent {
        let released = self.stress.release(config.slip_release_factor);
        let moment = config.seismic_moment(released, if major { 1.0 } else { 0.3 });
        let magnitude = config.magnitude_from_moment(moment);

        self.state = if major {
            SlipState::MajorSlip
        } else {
            SlipState::MinorSlip
        };
        self.recovery_ticks = if major { 100 } else { 20 };

        QuakeEvent {
            fault_id: self.id,
            tick,
            magnitude,
            epicenter: self.center(),
            depth: f32::midpoint(self.start.2, self.end.2) + self.depth_extent / 2.0,
            slip_amount: released * 0.01,
            fault_type: self.fault_type,
            aftershock_potential: config.aftershock_probability * magnitude / 5.0,
        }
    }

    #[must_use]
    pub fn distance_to(&self, point: (f32, f32, f32)) -> f32 {
        let center = self.center();
        let dx = point.0 - center.0;
        let dy = point.1 - center.1;
        let dz = point.2 - center.2;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    #[must_use]
    pub fn projected_stress(&self, ticks_ahead: u64, config: &FaultConfig) -> f32 {
        let rate = self.effective_stress_rate(config.stress_rate);
        let decay = config.stress_decay_rate;

        #[allow(clippy::cast_precision_loss)]
        let projection =
            self.stress.current + (rate - self.stress.current * decay) * ticks_ahead as f32;
        projection.max(0.0)
    }

    #[must_use]
    pub fn fingerprint(&self) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&self.id.raw().to_le_bytes());
        hasher.update(&self.stress.current.to_le_bytes());
        hasher.update(&[self.state as u8]);
        hasher.update(&self.recovery_ticks.to_le_bytes());
        hasher.finalize()
    }
}

/// An earthquake event generated by fault slip.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QuakeEvent {
    /// Source fault.
    pub fault_id: FeatureId,
    /// Tick when event occurred.
    pub tick: u64,
    /// Magnitude (Richter scale approximation).
    pub magnitude: f32,
    /// Epicenter position.
    pub epicenter: (f32, f32, f32),
    /// Depth of focus.
    pub depth: f32,
    /// Amount of slip (meters).
    pub slip_amount: f32,
    /// Type of fault that slipped.
    pub fault_type: FaultType,
    /// Probability of aftershocks.
    pub aftershock_potential: f32,
}

impl QuakeEvent {
    #[must_use]
    pub fn intensity_at_distance(&self, distance: f32) -> f32 {
        if distance < 0.1 {
            return self.magnitude;
        }

        let attenuation = 1.0 / (1.0 + distance * 0.1);
        (self.magnitude * attenuation).max(0.0)
    }

    #[must_use]
    pub fn is_significant(&self) -> bool {
        self.magnitude >= 4.0
    }

    #[must_use]
    pub fn is_major(&self) -> bool {
        self.magnitude >= 6.0
    }

    #[must_use]
    pub fn affected_radius(&self) -> f32 {
        (10.0_f32).powf(self.magnitude * 0.5)
    }

    fn sort_key(&self) -> (u64, FeatureId) {
        (self.tick, self.fault_id)
    }
}

impl PartialOrd for QuakeEvent {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for QuakeEvent {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.sort_key().cmp(&other.sort_key())
    }
}

impl Eq for QuakeEvent {}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_fault() -> FaultLine {
        FaultLine::new(
            FeatureId::new(1),
            FaultType::Normal,
            (0.0, 0.0, 10.0),
            (100.0, 0.0, 10.0),
        )
    }

    #[test]
    fn fault_type_properties() {
        assert_eq!(FaultType::Normal.name(), "normal");
        assert!(FaultType::Transform.stress_multiplier() > FaultType::Normal.stress_multiplier());
        assert!((FaultType::StrikeSlip.typical_dip() - 90.0).abs() < f32::EPSILON);
    }

    #[test]
    fn fault_type_from_raw() {
        for (i, ft) in FaultType::ALL.iter().enumerate() {
            #[allow(clippy::cast_possible_truncation)]
            let idx = i as u8;
            assert_eq!(FaultType::from_raw(idx), Some(*ft));
        }
        assert_eq!(FaultType::from_raw(10), None);
    }

    #[test]
    fn slip_state_properties() {
        assert!(SlipState::MajorSlip.is_slipping());
        assert!(SlipState::MinorSlip.is_slipping());
        assert!(!SlipState::Locked.is_slipping());
        assert!(SlipState::Creeping.is_active());
    }

    #[test]
    fn stress_accumulator_operations() {
        let mut acc = StressAccumulator::new().with_initial_stress(10.0);
        assert!((acc.current - 10.0).abs() < f32::EPSILON);

        acc.accumulate(5.0);
        assert!((acc.current - 15.0).abs() < f32::EPSILON);
        assert!((acc.peak - 15.0).abs() < f32::EPSILON);

        let released = acc.release(0.5);
        assert!((released - 7.5).abs() < f32::EPSILON);
        assert!((acc.current - 7.5).abs() < f32::EPSILON);
        assert_eq!(acc.slip_count, 1);
    }

    #[test]
    fn stress_accumulator_decay() {
        let mut acc = StressAccumulator::new().with_initial_stress(100.0);
        acc.decay(0.1);
        assert!((acc.current - 90.0).abs() < f32::EPSILON);
    }

    #[test]
    fn fault_line_geometry() {
        let fault = test_fault();
        assert!((fault.length() - 100.0).abs() < f32::EPSILON);
        assert!(fault.area() > 0.0);

        let center = fault.center();
        assert!((center.0 - 50.0).abs() < f32::EPSILON);
    }

    #[test]
    fn fault_line_stress_rate() {
        let fault = test_fault().with_friction(0.8);
        let config = FaultConfig::new();
        let rate = fault.effective_stress_rate(config.stress_rate);
        assert!(rate > 0.0);
    }

    #[test]
    fn fault_line_distance() {
        let fault = test_fault();
        let dist = fault.distance_to((50.0, 100.0, 10.0));
        assert!((dist - 100.0).abs() < f32::EPSILON);
    }

    #[test]
    fn fault_line_projection() {
        let fault = test_fault().with_initial_stress(1.0);
        let config = FaultConfig::new();
        let projected = fault.projected_stress(100, &config);
        assert!(projected >= fault.stress().current);
    }

    #[test]
    fn quake_event_intensity() {
        let event = QuakeEvent {
            fault_id: FeatureId::new(1),
            tick: 100,
            magnitude: 5.0,
            epicenter: (0.0, 0.0, 0.0),
            depth: 10.0,
            slip_amount: 0.5,
            fault_type: FaultType::Normal,
            aftershock_potential: 0.2,
        };

        let intensity_close = event.intensity_at_distance(1.0);
        let intensity_far = event.intensity_at_distance(100.0);
        assert!(intensity_close > intensity_far);
    }

    #[test]
    fn quake_event_significance() {
        let minor = QuakeEvent {
            fault_id: FeatureId::new(1),
            tick: 100,
            magnitude: 3.5,
            epicenter: (0.0, 0.0, 0.0),
            depth: 10.0,
            slip_amount: 0.1,
            fault_type: FaultType::Normal,
            aftershock_potential: 0.1,
        };

        let major = QuakeEvent {
            fault_id: FeatureId::new(2),
            tick: 100,
            magnitude: 6.5,
            epicenter: (0.0, 0.0, 0.0),
            depth: 10.0,
            slip_amount: 2.0,
            fault_type: FaultType::Transform,
            aftershock_potential: 0.5,
        };

        assert!(!minor.is_significant());
        assert!(major.is_significant());
        assert!(major.is_major());
    }

    #[test]
    fn quake_event_ordering() {
        let e1 = QuakeEvent {
            fault_id: FeatureId::new(1),
            tick: 100,
            magnitude: 4.0,
            epicenter: (0.0, 0.0, 0.0),
            depth: 10.0,
            slip_amount: 0.3,
            fault_type: FaultType::Normal,
            aftershock_potential: 0.2,
        };

        let e2 = QuakeEvent {
            fault_id: FeatureId::new(2),
            tick: 100,
            magnitude: 5.0,
            epicenter: (0.0, 0.0, 0.0),
            depth: 10.0,
            slip_amount: 0.5,
            fault_type: FaultType::Reverse,
            aftershock_potential: 0.3,
        };

        assert!(e1 < e2);
    }

    #[test]
    fn fingerprint_determinism() {
        let f1 = test_fault().with_initial_stress(50.0);
        let f2 = test_fault().with_initial_stress(50.0);
        assert_eq!(f1.fingerprint(), f2.fingerprint());
    }

    #[test]
    fn serde_fault_type() {
        let ft = FaultType::Transform;
        let json = serde_json::to_string(&ft).unwrap();
        let recovered: FaultType = serde_json::from_str(&json).unwrap();
        assert_eq!(ft, recovered);
    }

    #[test]
    fn serde_slip_state() {
        let state = SlipState::Creeping;
        let json = serde_json::to_string(&state).unwrap();
        let recovered: SlipState = serde_json::from_str(&json).unwrap();
        assert_eq!(state, recovered);
    }

    #[test]
    fn serde_stress_accumulator() {
        let acc = StressAccumulator::new().with_initial_stress(75.0);
        let json = serde_json::to_string(&acc).unwrap();
        let recovered: StressAccumulator = serde_json::from_str(&json).unwrap();
        assert_eq!(acc, recovered);
    }

    #[test]
    fn serde_fault_line() {
        let fault = test_fault().with_friction(0.7).with_initial_stress(30.0);
        let json = serde_json::to_string(&fault).unwrap();
        let recovered: FaultLine = serde_json::from_str(&json).unwrap();
        assert_eq!(fault.id, recovered.id);
        assert!((fault.friction - recovered.friction).abs() < f32::EPSILON);
    }

    #[test]
    fn serde_quake_event() {
        let event = QuakeEvent {
            fault_id: FeatureId::new(42),
            tick: 500,
            magnitude: 5.5,
            epicenter: (100.0, 200.0, 10.0),
            depth: 15.0,
            slip_amount: 0.8,
            fault_type: FaultType::StrikeSlip,
            aftershock_potential: 0.4,
        };
        let json = serde_json::to_string(&event).unwrap();
        let recovered: QuakeEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, recovered);
    }
}
