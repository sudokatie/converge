//! Diagnostic color types and channel palettes.

use serde::{Deserialize, Serialize};

use super::channel::{
    AtmosphereChannel, DiagnosticCategory, DiagnosticChannel, SchedulerChannel, StructuralChannel,
};
use crate::environment::{
    AtmosphereLayer, ConduitKind, FieldChannel, FluidKind, HazardKind, SupportKind,
    VectorFieldChannel,
};
use crate::scheduler::Fidelity;

/// Renderer-agnostic RGBA color.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct DiagnosticColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl DiagnosticColor {
    pub const TRANSPARENT: Self = Self::new(0, 0, 0, 0);
    pub const WHITE: Self = Self::new(255, 255, 255, 255);
    pub const BLACK: Self = Self::new(0, 0, 0, 255);
    pub const RED: Self = Self::new(255, 0, 0, 255);
    pub const GREEN: Self = Self::new(0, 255, 0, 255);
    pub const BLUE: Self = Self::new(0, 0, 255, 255);
    pub const YELLOW: Self = Self::new(255, 255, 0, 255);
    pub const CYAN: Self = Self::new(0, 255, 255, 255);
    pub const MAGENTA: Self = Self::new(255, 0, 255, 255);
    pub const ORANGE: Self = Self::new(255, 165, 0, 255);
    pub const GRAY: Self = Self::new(128, 128, 128, 255);

    #[must_use]
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    #[must_use]
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::new(r, g, b, 255)
    }

    #[must_use]
    pub const fn with_alpha(self, a: u8) -> Self {
        Self { a, ..self }
    }

    #[must_use]
    pub const fn to_array(self) -> [u8; 4] {
        [self.r, self.g, self.b, self.a]
    }

    #[must_use]
    pub const fn from_array(arr: [u8; 4]) -> Self {
        Self::new(arr[0], arr[1], arr[2], arr[3])
    }

    #[must_use]
    pub fn to_f32_array(self) -> [f32; 4] {
        [
            f32::from(self.r) / 255.0,
            f32::from(self.g) / 255.0,
            f32::from(self.b) / 255.0,
            f32::from(self.a) / 255.0,
        ]
    }

    #[must_use]
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        let inv_t = 1.0 - t;
        Self::new(
            (f32::from(self.r) * inv_t + f32::from(other.r) * t) as u8,
            (f32::from(self.g) * inv_t + f32::from(other.g) * t) as u8,
            (f32::from(self.b) * inv_t + f32::from(other.b) * t) as u8,
            (f32::from(self.a) * inv_t + f32::from(other.a) * t) as u8,
        )
    }

    #[must_use]
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn intensity_scaled(self, intensity: f32) -> Self {
        let intensity = intensity.clamp(0.0, 1.0);
        Self::new(
            (f32::from(self.r) * intensity) as u8,
            (f32::from(self.g) * intensity) as u8,
            (f32::from(self.b) * intensity) as u8,
            self.a,
        )
    }
}

/// Channel-specific color palette for diagnostic visualization.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChannelPalette {
    scalar_colors: [DiagnosticColor; FieldChannel::COUNT],
    vector_colors: [DiagnosticColor; VectorFieldChannel::COUNT],
    hazard_colors: [DiagnosticColor; HazardKind::COUNT],
    fluid_colors: [DiagnosticColor; FluidKind::COUNT],
    structural_colors: [DiagnosticColor; StructuralChannel::COUNT],
    conduit_colors: [DiagnosticColor; ConduitKind::COUNT],
    atmosphere_colors: [DiagnosticColor; AtmosphereChannel::COUNT],
    scheduler_colors: [DiagnosticColor; SchedulerChannel::COUNT],
    custom_color: DiagnosticColor,
    category_colors: [DiagnosticColor; DiagnosticCategory::COUNT],
}

impl Default for ChannelPalette {
    fn default() -> Self {
        Self {
            scalar_colors: [
                DiagnosticColor::rgb(255, 100, 50),  // Temperature - orange-red
                DiagnosticColor::rgb(100, 200, 255), // Oxygen - light blue
                DiagnosticColor::rgb(180, 180, 220), // Pressure - light purple
                DiagnosticColor::rgb(0, 255, 100),   // Radiation - green glow
                DiagnosticColor::rgb(150, 50, 200),  // Toxicity - purple
                DiagnosticColor::rgb(100, 150, 255), // Humidity - blue
                DiagnosticColor::rgb(80, 0, 80),     // Corruption - dark magenta
                DiagnosticColor::rgb(200, 200, 100), // SporeDensity - yellow-green
            ],
            vector_colors: [
                DiagnosticColor::rgb(200, 220, 255), // Wind - pale blue
                DiagnosticColor::rgb(50, 100, 200),  // WaterCurrent - deep blue
                DiagnosticColor::rgb(220, 180, 220), // PressureGradient - lavender
                DiagnosticColor::rgb(255, 200, 50),  // GravityOverride - gold
                DiagnosticColor::rgb(255, 100, 100), // HazardSpread - red
            ],
            hazard_colors: [
                DiagnosticColor::rgb(255, 100, 0),   // Fire - orange
                DiagnosticColor::rgb(100, 200, 50),  // Infection - sickly green
                DiagnosticColor::rgb(150, 200, 255), // Frost - ice blue
                DiagnosticColor::rgb(50, 50, 80),    // Vacuum - dark blue-gray
                DiagnosticColor::rgb(100, 150, 200), // Flood - water blue
                DiagnosticColor::rgb(100, 0, 100),   // Corruption - dark purple
            ],
            fluid_colors: [
                DiagnosticColor::rgb(50, 100, 200),  // Water - blue
                DiagnosticColor::rgb(200, 200, 200), // Gas - gray
                DiagnosticColor::rgb(139, 90, 43),   // Slurry - brown
                DiagnosticColor::rgb(255, 80, 0),    // Lava - bright orange
            ],
            structural_colors: [
                DiagnosticColor::rgb(150, 150, 150), // SupportKind - gray
                DiagnosticColor::rgb(200, 150, 50),  // Load - amber
                DiagnosticColor::rgb(255, 50, 50),   // Stress - red
                DiagnosticColor::rgb(50, 200, 100),  // Integrity - green
                DiagnosticColor::rgb(100, 100, 200), // SupportDistance - blue
            ],
            conduit_colors: [
                DiagnosticColor::rgb(255, 220, 50),  // Power - yellow
                DiagnosticColor::rgb(255, 100, 50),  // Heat - orange
                DiagnosticColor::rgb(50, 150, 255),  // Fluid - blue
                DiagnosticColor::rgb(100, 255, 200), // Signal - cyan-green
            ],
            atmosphere_colors: [
                DiagnosticColor::rgb(100, 200, 255), // Layer - light blue
                DiagnosticColor::rgb(50, 200, 100),  // SealQuality - green
                DiagnosticColor::rgb(150, 200, 255), // Ventilation - pale blue
                DiagnosticColor::rgb(200, 100, 50),  // Contamination - rust
            ],
            scheduler_colors: [
                DiagnosticColor::rgb(50, 200, 50),   // Fidelity - green
                DiagnosticColor::rgb(255, 200, 50),  // Interest - gold
                DiagnosticColor::rgb(150, 150, 200), // Distance - lavender
                DiagnosticColor::rgb(200, 100, 200), // Priority - magenta
                DiagnosticColor::rgb(100, 200, 200), // Accumulated - cyan
            ],
            custom_color: DiagnosticColor::MAGENTA,
            category_colors: [
                DiagnosticColor::rgb(255, 150, 100), // ScalarField
                DiagnosticColor::rgb(100, 200, 255), // VectorField
                DiagnosticColor::rgb(255, 100, 50),  // Hazard
                DiagnosticColor::rgb(50, 150, 255),  // Fluid
                DiagnosticColor::rgb(180, 180, 180), // Structural
                DiagnosticColor::rgb(255, 220, 100), // Conduit
                DiagnosticColor::rgb(150, 220, 255), // Atmosphere
                DiagnosticColor::rgb(100, 255, 150), // Scheduler
                DiagnosticColor::MAGENTA,            // Custom
            ],
        }
    }
}

impl ChannelPalette {
    #[must_use]
    pub fn channel_color(&self, channel: DiagnosticChannel) -> DiagnosticColor {
        match channel {
            DiagnosticChannel::Scalar(c) => self.scalar_colors[c.as_index()],
            DiagnosticChannel::Vector(c) => self.vector_colors[c.as_index()],
            DiagnosticChannel::Hazard(k) => self.hazard_colors[k.as_index()],
            DiagnosticChannel::Fluid(k) => self.fluid_colors[k.as_index()],
            DiagnosticChannel::Structural(c) => self.structural_colors[c as usize],
            DiagnosticChannel::Conduit(k) => self.conduit_colors[k.as_index()],
            DiagnosticChannel::Atmosphere(c) => self.atmosphere_colors[c as usize],
            DiagnosticChannel::Scheduler(c) => self.scheduler_colors[c as usize],
            DiagnosticChannel::Custom(_) => self.custom_color,
        }
    }

    #[must_use]
    pub fn category_color(&self, category: DiagnosticCategory) -> DiagnosticColor {
        self.category_colors[category.as_index()]
    }

    pub fn set_channel_color(&mut self, channel: DiagnosticChannel, color: DiagnosticColor) {
        match channel {
            DiagnosticChannel::Scalar(c) => self.scalar_colors[c.as_index()] = color,
            DiagnosticChannel::Vector(c) => self.vector_colors[c.as_index()] = color,
            DiagnosticChannel::Hazard(k) => self.hazard_colors[k.as_index()] = color,
            DiagnosticChannel::Fluid(k) => self.fluid_colors[k.as_index()] = color,
            DiagnosticChannel::Structural(c) => self.structural_colors[c as usize] = color,
            DiagnosticChannel::Conduit(k) => self.conduit_colors[k.as_index()] = color,
            DiagnosticChannel::Atmosphere(c) => self.atmosphere_colors[c as usize] = color,
            DiagnosticChannel::Scheduler(c) => self.scheduler_colors[c as usize] = color,
            DiagnosticChannel::Custom(_) => self.custom_color = color,
        }
    }

    #[must_use]
    pub fn fidelity_color(&self, fidelity: Fidelity) -> DiagnosticColor {
        match fidelity {
            Fidelity::Immediate => DiagnosticColor::rgb(50, 255, 50),
            Fidelity::Near => DiagnosticColor::rgb(150, 255, 50),
            Fidelity::Distant => DiagnosticColor::rgb(255, 200, 50),
            Fidelity::Dormant => DiagnosticColor::rgb(150, 150, 150),
        }
    }

    #[must_use]
    pub fn support_kind_color(&self, kind: SupportKind) -> DiagnosticColor {
        match kind {
            SupportKind::None => DiagnosticColor::rgb(50, 50, 50),
            SupportKind::Foundation => DiagnosticColor::rgb(100, 80, 60),
            SupportKind::Column => DiagnosticColor::rgb(150, 150, 180),
            SupportKind::Beam => DiagnosticColor::rgb(180, 150, 100),
            SupportKind::Brace => DiagnosticColor::rgb(120, 120, 160),
            SupportKind::Solid => DiagnosticColor::rgb(200, 100, 100),
            SupportKind::Weak => DiagnosticColor::rgb(180, 80, 80),
        }
    }

    #[must_use]
    pub fn atmosphere_layer_color(&self, layer: AtmosphereLayer) -> DiagnosticColor {
        match layer {
            AtmosphereLayer::Indoor => DiagnosticColor::rgb(150, 200, 150),
            AtmosphereLayer::Outdoor => DiagnosticColor::rgb(150, 200, 255),
            AtmosphereLayer::Exposed => DiagnosticColor::rgb(255, 200, 150),
            AtmosphereLayer::Vacuum => DiagnosticColor::rgb(30, 30, 50),
        }
    }

    #[must_use]
    pub fn heat_gradient(&self, normalized: f32) -> DiagnosticColor {
        let t = normalized.clamp(0.0, 1.0);
        if t < 0.5 {
            DiagnosticColor::rgb(50, 100, 200).lerp(DiagnosticColor::rgb(50, 200, 50), t * 2.0)
        } else {
            DiagnosticColor::rgb(50, 200, 50)
                .lerp(DiagnosticColor::rgb(255, 50, 0), (t - 0.5) * 2.0)
        }
    }

    #[must_use]
    pub fn stress_gradient(&self, normalized: f32) -> DiagnosticColor {
        let t = normalized.clamp(0.0, 1.0);
        DiagnosticColor::rgb(50, 200, 50).lerp(DiagnosticColor::rgb(255, 50, 0), t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_constructors() {
        let c = DiagnosticColor::rgb(100, 150, 200);
        assert_eq!(c.a, 255);
        assert_eq!(c.to_array(), [100, 150, 200, 255]);
    }

    #[test]
    fn test_color_with_alpha() {
        let c = DiagnosticColor::rgb(100, 150, 200).with_alpha(128);
        assert_eq!(c.a, 128);
        assert_eq!(c.r, 100);
    }

    #[test]
    fn test_color_lerp() {
        let a = DiagnosticColor::rgb(0, 0, 0);
        let b = DiagnosticColor::rgb(200, 100, 50);
        let mid = a.lerp(b, 0.5);
        assert_eq!(mid.r, 100);
        assert_eq!(mid.g, 50);
        assert_eq!(mid.b, 25);
    }

    #[test]
    fn test_color_lerp_clamped() {
        let a = DiagnosticColor::rgb(100, 100, 100);
        let b = DiagnosticColor::rgb(200, 200, 200);
        let below = a.lerp(b, -1.0);
        let above = a.lerp(b, 2.0);
        assert_eq!(below, a);
        assert_eq!(above, b);
    }

    #[test]
    fn test_intensity_scaled() {
        let c = DiagnosticColor::rgb(200, 100, 50);
        let half = c.intensity_scaled(0.5);
        assert_eq!(half.r, 100);
        assert_eq!(half.g, 50);
        assert_eq!(half.b, 25);
        assert_eq!(half.a, 255);
    }

    #[test]
    fn test_palette_channel_color() {
        let palette = ChannelPalette::default();
        let temp_color =
            palette.channel_color(DiagnosticChannel::Scalar(FieldChannel::Temperature));
        assert_ne!(temp_color, DiagnosticColor::TRANSPARENT);
    }

    #[test]
    fn test_palette_all_channels_have_colors() {
        let palette = ChannelPalette::default();
        for ch in DiagnosticChannel::all_scalar() {
            let color = palette.channel_color(ch);
            assert_ne!(color.a, 0, "Scalar channel {ch:?} has zero alpha");
        }
        for ch in DiagnosticChannel::all_hazard() {
            let color = palette.channel_color(ch);
            assert_ne!(color.a, 0, "Hazard channel {ch:?} has zero alpha");
        }
    }

    #[test]
    fn test_heat_gradient_bounds() {
        let palette = ChannelPalette::default();
        let cold = palette.heat_gradient(0.0);
        let hot = palette.heat_gradient(1.0);
        assert_ne!(cold, hot);
    }

    #[test]
    fn test_serde_round_trip() {
        let color = DiagnosticColor::rgb(123, 45, 67);
        let json = serde_json::to_string(&color).unwrap();
        let recovered: DiagnosticColor = serde_json::from_str(&json).unwrap();
        assert_eq!(color, recovered);
    }
}
