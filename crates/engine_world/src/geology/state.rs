//! Field state types for geological simulation.

use serde::{Deserialize, Serialize};

/// Pressure field at a geological location.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PressureField {
    /// Lithostatic pressure from overlying rock.
    pub lithostatic: f32,
    /// Tectonic pressure from plate movement.
    pub tectonic: f32,
    /// Fluid pressure from pore water/magma.
    pub fluid: f32,
    /// Effective pressure (total minus pore).
    effective: f32,
}

impl PressureField {
    pub const MIN: f32 = 0.0;
    pub const MAX: f32 = 10000.0;

    #[must_use]
    pub fn new(lithostatic: f32) -> Self {
        let mut field = Self {
            lithostatic: lithostatic.clamp(Self::MIN, Self::MAX),
            tectonic: 0.0,
            fluid: 0.0,
            effective: 0.0,
        };
        field.update_effective();
        field
    }

    #[must_use]
    pub fn with_tectonic(mut self, tectonic: f32) -> Self {
        self.tectonic = tectonic.clamp(Self::MIN, Self::MAX);
        self.update_effective();
        self
    }

    #[must_use]
    pub fn with_fluid(mut self, fluid: f32) -> Self {
        self.fluid = fluid.clamp(Self::MIN, Self::MAX);
        self.update_effective();
        self
    }

    fn update_effective(&mut self) {
        self.effective = (self.lithostatic + self.tectonic - self.fluid).max(0.0);
    }

    pub fn add_tectonic(&mut self, delta: f32) {
        self.tectonic = (self.tectonic + delta).clamp(Self::MIN, Self::MAX);
        self.update_effective();
    }

    pub fn add_fluid(&mut self, delta: f32) {
        self.fluid = (self.fluid + delta).clamp(Self::MIN, Self::MAX);
        self.update_effective();
    }

    #[must_use]
    pub fn total(&self) -> f32 {
        self.lithostatic + self.tectonic + self.fluid
    }

    #[must_use]
    pub fn effective(&self) -> f32 {
        self.effective
    }

    #[must_use]
    pub fn is_overpressured(&self, threshold: f32) -> bool {
        self.fluid > threshold * self.lithostatic
    }

    #[must_use]
    pub fn compression_ratio(&self) -> f32 {
        if self.lithostatic > 0.0 {
            self.effective / self.lithostatic
        } else {
            1.0
        }
    }
}

/// Temperature field at a geological location.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TemperatureField {
    /// Ambient temperature from geothermal gradient.
    pub ambient: f32,
    /// Local anomaly (positive for heat sources like magma).
    pub anomaly: f32,
    /// Heat flux rate (energy transfer per tick).
    pub flux: f32,
}

impl TemperatureField {
    pub const MIN: f32 = -273.15;
    pub const MAX: f32 = 5000.0;

    #[must_use]
    pub fn new(ambient: f32) -> Self {
        Self {
            ambient: ambient.clamp(Self::MIN, Self::MAX),
            anomaly: 0.0,
            flux: 0.0,
        }
    }

    #[must_use]
    pub fn with_anomaly(mut self, anomaly: f32) -> Self {
        self.anomaly = anomaly.clamp(-1000.0, 2000.0);
        self
    }

    #[must_use]
    pub fn with_flux(mut self, flux: f32) -> Self {
        self.flux = flux;
        self
    }

    #[must_use]
    pub fn temperature(&self) -> f32 {
        (self.ambient + self.anomaly).clamp(Self::MIN, Self::MAX)
    }

    pub fn apply_flux(&mut self, dt: f32) {
        self.anomaly = (self.anomaly + self.flux * dt).clamp(-1000.0, 2000.0);
    }

    pub fn diffuse_toward(&mut self, target_temp: f32, diffusivity: f32, dt: f32) {
        let current = self.temperature();
        let delta = (target_temp - current) * diffusivity * dt;
        self.anomaly += delta;
    }

    #[must_use]
    pub fn is_molten(&self, threshold: f32) -> bool {
        self.temperature() >= threshold
    }

    pub fn add_heat(&mut self, amount: f32) {
        self.anomaly = (self.anomaly + amount).clamp(-1000.0, 2000.0);
    }

    pub fn remove_heat(&mut self, amount: f32) {
        self.anomaly = (self.anomaly - amount).clamp(-1000.0, 2000.0);
    }
}

/// Stability field tracking structural integrity.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct StabilityField {
    /// Base structural integrity (0-1).
    pub integrity: f32,
    /// Accumulated damage (0-1).
    pub damage: f32,
    /// Current stress level.
    pub stress: f32,
    /// Fracture density.
    pub fractures: f32,
}

impl StabilityField {
    #[must_use]
    pub fn new(integrity: f32) -> Self {
        Self {
            integrity: integrity.clamp(0.0, 1.0),
            damage: 0.0,
            stress: 0.0,
            fractures: 0.0,
        }
    }

    #[must_use]
    pub fn with_damage(mut self, damage: f32) -> Self {
        self.damage = damage.clamp(0.0, 1.0);
        self
    }

    #[must_use]
    pub fn effective_integrity(&self) -> f32 {
        (self.integrity - self.damage).max(0.0)
    }

    #[must_use]
    pub fn is_compromised(&self) -> bool {
        self.effective_integrity() < 0.3
    }

    #[must_use]
    pub fn is_failed(&self) -> bool {
        self.effective_integrity() <= 0.0
    }

    pub fn apply_stress(&mut self, stress: f32, strength: f32) {
        self.stress = stress.max(0.0);
        if strength > 0.0 && stress > strength {
            let overstress = (stress - strength) / strength;
            self.damage = (self.damage + overstress * 0.01).min(1.0);
            self.fractures = (self.fractures + overstress * 0.001).min(1.0);
        }
    }

    pub fn heal(&mut self, rate: f32) {
        self.damage = (self.damage - rate).max(0.0);
    }

    #[must_use]
    pub fn failure_probability(&self) -> f32 {
        if self.is_failed() {
            return 1.0;
        }

        let integrity_factor = 1.0 - self.effective_integrity();
        let fracture_factor = self.fractures;
        (integrity_factor * 0.7 + fracture_factor * 0.3).clamp(0.0, 1.0)
    }
}

/// Combined geology fields for a location.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GeologyFields {
    /// Pressure state.
    pub pressure: PressureField,
    /// Temperature state.
    pub temperature: TemperatureField,
    /// Stability state.
    pub stability: StabilityField,
    /// Depth below surface.
    pub depth: f32,
    /// Last update tick.
    pub last_tick: u64,
}

impl GeologyFields {
    #[must_use]
    pub fn new(depth: f32) -> Self {
        Self {
            pressure: PressureField::new(depth * 0.1),
            temperature: TemperatureField::new(15.0 + depth * 0.03),
            stability: StabilityField::new(1.0),
            depth: depth.max(0.0),
            last_tick: 0,
        }
    }

    #[must_use]
    pub fn at_depth(depth: f32, pressure_coeff: f32, temp_gradient: f32) -> Self {
        let d = depth.max(0.0);
        Self {
            pressure: PressureField::new(d * pressure_coeff),
            temperature: TemperatureField::new(15.0 + d * temp_gradient),
            stability: StabilityField::new(1.0),
            depth: d,
            last_tick: 0,
        }
    }

    #[must_use]
    pub fn with_pressure(mut self, pressure: PressureField) -> Self {
        self.pressure = pressure;
        self
    }

    #[must_use]
    pub fn with_temperature(mut self, temperature: TemperatureField) -> Self {
        self.temperature = temperature;
        self
    }

    #[must_use]
    pub fn with_stability(mut self, stability: StabilityField) -> Self {
        self.stability = stability;
        self
    }

    pub fn tick(&mut self, current_tick: u64, dt: f32) {
        if current_tick <= self.last_tick {
            return;
        }
        self.last_tick = current_tick;

        self.temperature.apply_flux(dt);

        if self.stability.stress > 0.0 {
            self.stability.stress *= 0.99;
        }
    }

    #[must_use]
    pub fn is_hazardous(&self, magma_threshold: f32) -> bool {
        self.temperature.is_molten(magma_threshold)
            || self.pressure.is_overpressured(0.9)
            || self.stability.is_compromised()
    }

    #[must_use]
    pub fn hazard_level(&self, magma_threshold: f32) -> f32 {
        let temp_hazard = if self.temperature.is_molten(magma_threshold) {
            1.0
        } else {
            (self.temperature.temperature() / magma_threshold).min(1.0)
        };

        let pressure_hazard = if self.pressure.is_overpressured(0.9) {
            1.0
        } else {
            self.pressure.fluid / (self.pressure.lithostatic * 0.9 + 0.01)
        };

        let stability_hazard = self.stability.failure_probability();

        (temp_hazard * 0.4 + pressure_hazard * 0.3 + stability_hazard * 0.3).clamp(0.0, 1.0)
    }

    #[must_use]
    pub fn fingerprint(&self) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&self.depth.to_le_bytes());
        hasher.update(&self.pressure.total().to_le_bytes());
        hasher.update(&self.temperature.temperature().to_le_bytes());
        hasher.update(&self.stability.effective_integrity().to_le_bytes());
        hasher.finalize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pressure_field_effective() {
        let p = PressureField::new(100.0)
            .with_tectonic(20.0)
            .with_fluid(30.0);

        assert!((p.total() - 150.0).abs() < f32::EPSILON);
        assert!((p.effective() - 90.0).abs() < f32::EPSILON);
    }

    #[test]
    fn pressure_field_overpressure() {
        let p = PressureField::new(100.0).with_fluid(95.0);
        assert!(p.is_overpressured(0.9));

        let p2 = PressureField::new(100.0).with_fluid(50.0);
        assert!(!p2.is_overpressured(0.9));
    }

    #[test]
    fn pressure_field_mutations() {
        let mut p = PressureField::new(50.0);
        p.add_tectonic(10.0);
        assert!((p.tectonic - 10.0).abs() < f32::EPSILON);

        p.add_fluid(5.0);
        assert!((p.fluid - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn temperature_field_basic() {
        let t = TemperatureField::new(100.0).with_anomaly(50.0);
        assert!((t.temperature() - 150.0).abs() < f32::EPSILON);
    }

    #[test]
    fn temperature_field_flux() {
        let mut t = TemperatureField::new(100.0).with_flux(10.0);
        t.apply_flux(1.0);
        assert!((t.anomaly - 10.0).abs() < f32::EPSILON);
    }

    #[test]
    fn temperature_field_diffusion() {
        let mut t = TemperatureField::new(100.0);
        t.diffuse_toward(200.0, 0.1, 1.0);
        assert!(t.temperature() > 100.0);
        assert!(t.temperature() < 200.0);
    }

    #[test]
    fn temperature_field_molten() {
        let t = TemperatureField::new(800.0);
        assert!(t.is_molten(700.0));
        assert!(!t.is_molten(900.0));
    }

    #[test]
    fn stability_field_integrity() {
        let s = StabilityField::new(0.8).with_damage(0.2);
        assert!((s.effective_integrity() - 0.6).abs() < f32::EPSILON);
    }

    #[test]
    fn stability_field_compromised() {
        let s = StabilityField::new(0.5).with_damage(0.3);
        assert!(s.is_compromised());

        let s2 = StabilityField::new(1.0).with_damage(0.1);
        assert!(!s2.is_compromised());
    }

    #[test]
    fn stability_field_stress() {
        let mut s = StabilityField::new(1.0);
        s.apply_stress(200.0, 100.0);
        assert!(s.damage > 0.0);
        assert!(s.fractures > 0.0);
    }

    #[test]
    fn stability_field_healing() {
        let mut s = StabilityField::new(1.0).with_damage(0.5);
        s.heal(0.1);
        assert!((s.damage - 0.4).abs() < f32::EPSILON);
    }

    #[test]
    fn geology_fields_at_depth() {
        let f = GeologyFields::at_depth(100.0, 0.1, 0.03);
        assert!((f.depth - 100.0).abs() < f32::EPSILON);
        assert!((f.pressure.lithostatic - 10.0).abs() < f32::EPSILON);
        assert!((f.temperature.ambient - 18.0).abs() < f32::EPSILON);
    }

    #[test]
    fn geology_fields_hazard() {
        let mut f = GeologyFields::new(100.0);
        assert!(!f.is_hazardous(700.0));

        f.temperature = TemperatureField::new(800.0);
        assert!(f.is_hazardous(700.0));
    }

    #[test]
    fn geology_fields_fingerprint_deterministic() {
        let f1 = GeologyFields::at_depth(50.0, 0.1, 0.03);
        let f2 = GeologyFields::at_depth(50.0, 0.1, 0.03);
        assert_eq!(f1.fingerprint(), f2.fingerprint());
    }

    #[test]
    fn serde_pressure_field() {
        let p = PressureField::new(100.0).with_tectonic(20.0);
        let json = serde_json::to_string(&p).unwrap();
        let recovered: PressureField = serde_json::from_str(&json).unwrap();
        assert_eq!(p, recovered);
    }

    #[test]
    fn serde_temperature_field() {
        let t = TemperatureField::new(200.0).with_anomaly(50.0);
        let json = serde_json::to_string(&t).unwrap();
        let recovered: TemperatureField = serde_json::from_str(&json).unwrap();
        assert_eq!(t, recovered);
    }

    #[test]
    fn serde_stability_field() {
        let s = StabilityField::new(0.9).with_damage(0.1);
        let json = serde_json::to_string(&s).unwrap();
        let recovered: StabilityField = serde_json::from_str(&json).unwrap();
        assert_eq!(s, recovered);
    }

    #[test]
    fn serde_geology_fields() {
        let f = GeologyFields::at_depth(200.0, 0.1, 0.03);
        let json = serde_json::to_string(&f).unwrap();
        let recovered: GeologyFields = serde_json::from_str(&json).unwrap();
        assert_eq!(f, recovered);
    }
}
