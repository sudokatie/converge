//! Per-cell atmosphere state storage.

use serde::{Deserialize, Serialize};

use super::AtmosphereLayer;

/// Atmosphere state for a single cell.
///
/// Combines the categorical layer classification with continuous
/// properties that affect environmental simulation.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct AtmosphereCell {
    /// The categorical atmosphere layer.
    layer: AtmosphereLayer,

    /// Seal quality (0.0 = fully permeable, 1.0 = perfectly sealed).
    /// Only meaningful for Indoor cells; affects leak rates.
    seal_quality: f32,

    /// Ventilation rate (0.0 = stagnant, 1.0 = maximum exchange).
    /// Affects how quickly atmosphere equilibrates with neighbors.
    ventilation: f32,

    /// Contamination level (0.0 = clean, 1.0 = heavily contaminated).
    /// Generic pollution/contamination separate from specific hazards.
    contamination: f32,
}

impl Default for AtmosphereCell {
    fn default() -> Self {
        Self::outdoor()
    }
}

impl AtmosphereCell {
    /// Create a new atmosphere cell with explicit values.
    #[must_use]
    pub fn new(
        layer: AtmosphereLayer,
        seal_quality: f32,
        ventilation: f32,
        contamination: f32,
    ) -> Self {
        Self {
            layer,
            seal_quality: seal_quality.clamp(0.0, 1.0),
            ventilation: ventilation.clamp(0.0, 1.0),
            contamination: contamination.clamp(0.0, 1.0),
        }
    }

    /// Create a standard outdoor cell.
    #[must_use]
    pub const fn outdoor() -> Self {
        Self {
            layer: AtmosphereLayer::Outdoor,
            seal_quality: 0.0,
            ventilation: 1.0,
            contamination: 0.0,
        }
    }

    /// Create a standard indoor cell with given seal quality.
    #[must_use]
    pub fn indoor(seal_quality: f32) -> Self {
        Self {
            layer: AtmosphereLayer::Indoor,
            seal_quality: seal_quality.clamp(0.0, 1.0),
            ventilation: 0.3,
            contamination: 0.0,
        }
    }

    /// Create a well-sealed indoor cell.
    #[must_use]
    pub const fn indoor_sealed() -> Self {
        Self {
            layer: AtmosphereLayer::Indoor,
            seal_quality: 1.0,
            ventilation: 0.2,
            contamination: 0.0,
        }
    }

    /// Create a standard exposed cell.
    #[must_use]
    pub const fn exposed() -> Self {
        Self {
            layer: AtmosphereLayer::Exposed,
            seal_quality: 0.0,
            ventilation: 0.7,
            contamination: 0.0,
        }
    }

    /// Create a vacuum cell.
    #[must_use]
    pub const fn vacuum() -> Self {
        Self {
            layer: AtmosphereLayer::Vacuum,
            seal_quality: 0.0,
            ventilation: 0.0,
            contamination: 0.0,
        }
    }

    /// Get the atmosphere layer.
    #[must_use]
    pub const fn layer(&self) -> AtmosphereLayer {
        self.layer
    }

    /// Get the seal quality.
    #[must_use]
    pub const fn seal_quality(&self) -> f32 {
        self.seal_quality
    }

    /// Get the ventilation rate.
    #[must_use]
    pub const fn ventilation(&self) -> f32 {
        self.ventilation
    }

    /// Get the contamination level.
    #[must_use]
    pub const fn contamination(&self) -> f32 {
        self.contamination
    }

    /// Set the atmosphere layer.
    pub fn set_layer(&mut self, layer: AtmosphereLayer) {
        self.layer = layer;
    }

    /// Set the seal quality (clamped to 0.0-1.0).
    pub fn set_seal_quality(&mut self, quality: f32) {
        self.seal_quality = quality.clamp(0.0, 1.0);
    }

    /// Set the ventilation rate (clamped to 0.0-1.0).
    pub fn set_ventilation(&mut self, rate: f32) {
        self.ventilation = rate.clamp(0.0, 1.0);
    }

    /// Set the contamination level (clamped to 0.0-1.0).
    pub fn set_contamination(&mut self, level: f32) {
        self.contamination = level.clamp(0.0, 1.0);
    }

    /// Add contamination (result clamped to 0.0-1.0).
    pub fn add_contamination(&mut self, delta: f32) {
        self.contamination = (self.contamination + delta).clamp(0.0, 1.0);
    }

    /// Check if atmosphere can freely flow into this cell.
    #[must_use]
    pub fn is_permeable(&self) -> bool {
        !self.layer.sealed() || self.seal_quality < 0.9
    }

    /// Check if this is a well-sealed indoor space.
    #[must_use]
    pub fn is_sealed_indoor(&self) -> bool {
        self.layer == AtmosphereLayer::Indoor && self.seal_quality >= 0.9
    }

    /// Calculate effective air exchange rate with outside atmosphere.
    /// Higher = more exchange with external conditions.
    #[must_use]
    pub fn air_exchange_rate(&self) -> f32 {
        match self.layer {
            AtmosphereLayer::Indoor => (1.0 - self.seal_quality) * self.ventilation,
            AtmosphereLayer::Outdoor => 1.0,
            AtmosphereLayer::Exposed => 0.5 + 0.5 * self.ventilation,
            AtmosphereLayer::Vacuum => 0.0,
        }
    }

    /// Calculate effective temperature (blending regulated and external).
    /// Returns a blend factor: 0.0 = external temp, 1.0 = regulated temp.
    #[must_use]
    pub fn temperature_blend(&self) -> f32 {
        let base_regulation = self.layer.temperature_regulation();
        match self.layer {
            AtmosphereLayer::Indoor => base_regulation * self.seal_quality,
            _ => base_regulation,
        }
    }

    /// Calculate effective radiation exposure considering seal quality.
    #[must_use]
    pub fn effective_radiation_exposure(&self) -> f32 {
        let base = self.layer.radiation_exposure();
        match self.layer {
            AtmosphereLayer::Indoor => base * (1.0 - self.seal_quality * 0.5),
            _ => base,
        }
    }

    /// Check if this cell should derive oxygen from its layer defaults.
    #[must_use]
    pub fn uses_layer_defaults(&self) -> bool {
        matches!(
            self.layer,
            AtmosphereLayer::Outdoor | AtmosphereLayer::Exposed | AtmosphereLayer::Vacuum
        )
    }
}

#[cfg(test)]
#[allow(
    clippy::float_cmp,
    reason = "tests check exact constructor return values"
)]
mod tests {
    use super::*;

    #[test]
    fn default_is_outdoor() {
        let cell = AtmosphereCell::default();
        assert_eq!(cell.layer(), AtmosphereLayer::Outdoor);
    }

    #[test]
    fn outdoor_properties() {
        let cell = AtmosphereCell::outdoor();
        assert_eq!(cell.layer(), AtmosphereLayer::Outdoor);
        assert!(cell.seal_quality().abs() < f32::EPSILON);
        assert!((cell.ventilation() - 1.0).abs() < f32::EPSILON);
        assert!(cell.contamination().abs() < f32::EPSILON);
    }

    #[test]
    fn indoor_sealed_properties() {
        let cell = AtmosphereCell::indoor_sealed();
        assert_eq!(cell.layer(), AtmosphereLayer::Indoor);
        assert!((cell.seal_quality() - 1.0).abs() < f32::EPSILON);
        assert!(cell.is_sealed_indoor());
    }

    #[test]
    fn indoor_with_seal() {
        let cell = AtmosphereCell::indoor(0.8);
        assert_eq!(cell.layer(), AtmosphereLayer::Indoor);
        assert!((cell.seal_quality() - 0.8).abs() < f32::EPSILON);
        assert!(!cell.is_sealed_indoor()); // < 0.9
    }

    #[test]
    fn exposed_properties() {
        let cell = AtmosphereCell::exposed();
        assert_eq!(cell.layer(), AtmosphereLayer::Exposed);
        assert!(cell.is_permeable());
    }

    #[test]
    fn vacuum_properties() {
        let cell = AtmosphereCell::vacuum();
        assert_eq!(cell.layer(), AtmosphereLayer::Vacuum);
        assert_eq!(cell.air_exchange_rate(), 0.0);
    }

    #[test]
    fn new_clamps_values() {
        let cell = AtmosphereCell::new(AtmosphereLayer::Indoor, -0.5, 1.5, 2.0);
        assert_eq!(cell.seal_quality(), 0.0);
        assert_eq!(cell.ventilation(), 1.0);
        assert_eq!(cell.contamination(), 1.0);
    }

    #[test]
    fn setters_clamp() {
        let mut cell = AtmosphereCell::outdoor();

        cell.set_seal_quality(1.5);
        assert_eq!(cell.seal_quality(), 1.0);

        cell.set_ventilation(-0.5);
        assert_eq!(cell.ventilation(), 0.0);

        cell.set_contamination(2.0);
        assert_eq!(cell.contamination(), 1.0);
    }

    #[test]
    fn add_contamination_clamps() {
        let mut cell = AtmosphereCell::outdoor();

        cell.add_contamination(0.5);
        assert_eq!(cell.contamination(), 0.5);

        cell.add_contamination(0.7);
        assert_eq!(cell.contamination(), 1.0);

        cell.add_contamination(-2.0);
        assert_eq!(cell.contamination(), 0.0);
    }

    #[test]
    fn permeable_logic() {
        let sealed = AtmosphereCell::indoor_sealed();
        assert!(!sealed.is_permeable());

        let leaky = AtmosphereCell::indoor(0.5);
        assert!(leaky.is_permeable());

        let outdoor = AtmosphereCell::outdoor();
        assert!(outdoor.is_permeable());
    }

    #[test]
    fn air_exchange_rate_values() {
        let outdoor = AtmosphereCell::outdoor();
        assert_eq!(outdoor.air_exchange_rate(), 1.0);

        let sealed = AtmosphereCell::indoor_sealed();
        assert!(sealed.air_exchange_rate() < 0.1);

        let exposed = AtmosphereCell::exposed();
        assert!(exposed.air_exchange_rate() > 0.5);

        let vacuum = AtmosphereCell::vacuum();
        assert_eq!(vacuum.air_exchange_rate(), 0.0);
    }

    #[test]
    fn temperature_blend_values() {
        let outdoor = AtmosphereCell::outdoor();
        assert_eq!(outdoor.temperature_blend(), 0.0);

        let sealed = AtmosphereCell::indoor_sealed();
        assert_eq!(sealed.temperature_blend(), 1.0);

        let leaky = AtmosphereCell::indoor(0.5);
        assert_eq!(leaky.temperature_blend(), 0.5);
    }

    #[test]
    fn effective_radiation_exposure_values() {
        let outdoor = AtmosphereCell::outdoor();
        let outdoor_rad = outdoor.effective_radiation_exposure();

        let indoor = AtmosphereCell::indoor_sealed();
        let indoor_rad = indoor.effective_radiation_exposure();

        assert!(indoor_rad < outdoor_rad);

        let vacuum = AtmosphereCell::vacuum();
        assert_eq!(vacuum.effective_radiation_exposure(), 1.0);
    }

    #[test]
    fn uses_layer_defaults_logic() {
        assert!(!AtmosphereCell::indoor_sealed().uses_layer_defaults());
        assert!(AtmosphereCell::outdoor().uses_layer_defaults());
        assert!(AtmosphereCell::exposed().uses_layer_defaults());
        assert!(AtmosphereCell::vacuum().uses_layer_defaults());
    }

    #[test]
    fn serde_round_trip() {
        let cells = [
            AtmosphereCell::outdoor(),
            AtmosphereCell::indoor_sealed(),
            AtmosphereCell::exposed(),
            AtmosphereCell::vacuum(),
            AtmosphereCell::new(AtmosphereLayer::Indoor, 0.7, 0.5, 0.3),
        ];

        for cell in cells {
            let json = serde_json::to_string(&cell).unwrap();
            let recovered: AtmosphereCell = serde_json::from_str(&json).unwrap();
            assert_eq!(recovered, cell);
        }
    }
}
