//! Status effect icon display for the HUD.
//!
//! Renders active status effects (poison, regeneration, speed, etc.)
//! as small icons with duration bars on the HUD.

use egui::{Color32, Pos2, Rect, Rounding, Shape};

/// A status effect type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StatusEffectKind {
    /// Poison damage over time.
    Poison,
    /// Wither damage over time.
    Wither,
    /// Health regeneration.
    Regeneration,
    /// Movement speed boost.
    Speed,
    /// Movement slowdown.
    Slowness,
    /// Jump boost.
    JumpBoost,
    /// Damage resistance.
    Resistance,
    /// Fire resistance.
    FireResistance,
    /// Night vision.
    NightVision,
    /// Water breathing.
    WaterBreathing,
    /// Invisibility.
    Invisibility,
    /// Blindness.
    Blindness,
    /// Hunger (increased food drain).
    Hunger,
    /// Weakness (reduced attack damage).
    Weakness,
    /// Absorption (extra health).
    Absorption,
    /// Glowing (visible through walls).
    Glowing,
    /// Levitation (float upward).
    Levitation,
}

impl StatusEffectKind {
    /// Get the display color for this effect.
    #[must_use]
    pub fn color(&self) -> Color32 {
        match self {
            StatusEffectKind::Poison => Color32::from_rgb(100, 180, 50),
            StatusEffectKind::Wither => Color32::from_rgb(80, 50, 80),
            StatusEffectKind::Regeneration => Color32::from_rgb(255, 170, 100),
            StatusEffectKind::Speed => Color32::from_rgb(150, 200, 255),
            StatusEffectKind::Slowness => Color32::from_rgb(100, 100, 150),
            StatusEffectKind::JumpBoost => Color32::from_rgb(100, 255, 200),
            StatusEffectKind::Resistance => Color32::from_rgb(150, 150, 200),
            StatusEffectKind::FireResistance => Color32::from_rgb(255, 150, 50),
            StatusEffectKind::NightVision => Color32::from_rgb(50, 100, 200),
            StatusEffectKind::WaterBreathing => Color32::from_rgb(50, 150, 255),
            StatusEffectKind::Invisibility => Color32::from_rgb(200, 200, 200),
            StatusEffectKind::Blindness => Color32::from_rgb(50, 50, 50),
            StatusEffectKind::Hunger => Color32::from_rgb(150, 100, 50),
            StatusEffectKind::Weakness => Color32::from_rgb(100, 80, 80),
            StatusEffectKind::Absorption => Color32::from_rgb(255, 220, 50),
            StatusEffectKind::Glowing => Color32::from_rgb(200, 255, 150),
            StatusEffectKind::Levitation => Color32::from_rgb(200, 150, 255),
        }
    }

    /// Get the short label (1-3 chars) for this effect.
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            StatusEffectKind::Poison => "PSN",
            StatusEffectKind::Wither => "WTR",
            StatusEffectKind::Regeneration => "REG",
            StatusEffectKind::Speed => "SPD",
            StatusEffectKind::Slowness => "SLW",
            StatusEffectKind::JumpBoost => "JMP",
            StatusEffectKind::Resistance => "RES",
            StatusEffectKind::FireResistance => "FIR",
            StatusEffectKind::NightVision => "NV",
            StatusEffectKind::WaterBreathing => "WBR",
            StatusEffectKind::Invisibility => "INV",
            StatusEffectKind::Blindness => "BLD",
            StatusEffectKind::Hunger => "HNG",
            StatusEffectKind::Weakness => "WEA",
            StatusEffectKind::Absorption => "ABS",
            StatusEffectKind::Glowing => "GLO",
            StatusEffectKind::Levitation => "LEV",
        }
    }

    /// Whether this is a beneficial effect.
    #[must_use]
    pub fn is_beneficial(&self) -> bool {
        matches!(
            self,
            StatusEffectKind::Regeneration
                | StatusEffectKind::Speed
                | StatusEffectKind::JumpBoost
                | StatusEffectKind::Resistance
                | StatusEffectKind::FireResistance
                | StatusEffectKind::NightVision
                | StatusEffectKind::WaterBreathing
                | StatusEffectKind::Invisibility
                | StatusEffectKind::Absorption
                | StatusEffectKind::Glowing
                | StatusEffectKind::Levitation
        )
    }

    /// All status effect kinds.
    #[must_use]
    pub fn all() -> &'static [StatusEffectKind] {
        &[
            StatusEffectKind::Poison,
            StatusEffectKind::Wither,
            StatusEffectKind::Regeneration,
            StatusEffectKind::Speed,
            StatusEffectKind::Slowness,
            StatusEffectKind::JumpBoost,
            StatusEffectKind::Resistance,
            StatusEffectKind::FireResistance,
            StatusEffectKind::NightVision,
            StatusEffectKind::WaterBreathing,
            StatusEffectKind::Invisibility,
            StatusEffectKind::Blindness,
            StatusEffectKind::Hunger,
            StatusEffectKind::Weakness,
            StatusEffectKind::Absorption,
            StatusEffectKind::Glowing,
            StatusEffectKind::Levitation,
        ]
    }
}

/// An active status effect with duration.
#[derive(Debug, Clone)]
pub struct ActiveStatusEffect {
    /// The effect kind.
    pub kind: StatusEffectKind,
    /// Remaining duration in seconds (None = permanent).
    pub remaining_secs: Option<f32>,
    /// Total duration in seconds.
    pub total_secs: f32,
    /// Amplifier level (0 = base, 1 = II, 2 = III, etc.).
    pub amplifier: u8,
}

impl ActiveStatusEffect {
    /// Create a new timed status effect.
    #[must_use]
    pub fn new(kind: StatusEffectKind, duration_secs: f32) -> Self {
        Self {
            kind,
            remaining_secs: Some(duration_secs),
            total_secs: duration_secs,
            amplifier: 0,
        }
    }

    /// Create a permanent status effect.
    #[must_use]
    pub fn permanent(kind: StatusEffectKind) -> Self {
        Self {
            kind,
            remaining_secs: None,
            total_secs: 0.0,
            amplifier: 0,
        }
    }

    /// Set the amplifier level.
    #[must_use]
    pub fn with_amplifier(mut self, level: u8) -> Self {
        self.amplifier = level;
        self
    }

    /// Get the progress fraction (0.0 to 1.0).
    #[must_use]
    pub fn progress(&self) -> f32 {
        match self.remaining_secs {
            Some(remaining) if self.total_secs > 0.0 => {
                (remaining / self.total_secs).clamp(0.0, 1.0)
            }
            _ => 1.0,
        }
    }

    /// Tick the effect, reducing remaining time.
    ///
    /// Returns true if the effect has expired.
    pub fn tick(&mut self, dt: f32) -> bool {
        if let Some(remaining) = &mut self.remaining_secs {
            *remaining -= dt;
            if *remaining <= 0.0 {
                return true;
            }
        }
        false
    }
}

/// Icon size for status effect display.
pub const ICON_SIZE: f32 = 24.0;

/// Draw status effect icons at the given position.
///
/// Returns the shapes to render.
pub fn draw_status_effects(
    position: Pos2,
    effects: &[ActiveStatusEffect],
) -> Vec<Shape> {
    let mut shapes = Vec::new();
    let padding = 2.0;

    for (i, effect) in effects.iter().enumerate() {
        let x = position.x + i as f32 * (ICON_SIZE + padding);
        let y = position.y;

        // Background
        let bg_rect = Rect::from_min_size(Pos2::new(x, y), egui::vec2(ICON_SIZE, ICON_SIZE));
        let bg_color = if effect.kind.is_beneficial() {
            Color32::from_black_alpha(150)
        } else {
            Color32::from_rgba_unmultiplied(150, 0, 0, 150)
        };
        shapes.push(Shape::rect_filled(bg_rect, Rounding::same(3.0), bg_color));

        // Duration bar
        if let Some(_remaining) = effect.remaining_secs {
            let progress = effect.progress();
            let bar_height = 3.0;
            let bar_width = ICON_SIZE * progress;
            let bar_rect = Rect::from_min_size(
                Pos2::new(x, y + ICON_SIZE - bar_height),
                egui::vec2(bar_width, bar_height),
            );
            shapes.push(Shape::rect_filled(bar_rect, Rounding::same(1.0), effect.kind.color()));
        }

        // Label
        // (In a full implementation, this would render text via the egui painter)
    }

    shapes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_effect_colors() {
        // Just verify colors are non-transparent
        for kind in StatusEffectKind::all() {
            let color = kind.color();
            assert_ne!(color, Color32::TRANSPARENT, "{:?} should have a color", kind);
        }
    }

    #[test]
    fn test_effect_labels() {
        assert_eq!(StatusEffectKind::Poison.label(), "PSN");
        assert_eq!(StatusEffectKind::Regeneration.label(), "REG");
        assert_eq!(StatusEffectKind::Speed.label(), "SPD");
    }

    #[test]
    fn test_beneficial_effects() {
        assert!(StatusEffectKind::Regeneration.is_beneficial());
        assert!(StatusEffectKind::Speed.is_beneficial());
        assert!(!StatusEffectKind::Poison.is_beneficial());
        assert!(!StatusEffectKind::Wither.is_beneficial());
    }

    #[test]
    fn test_all_effects_count() {
        assert_eq!(StatusEffectKind::all().len(), 17);
    }

    #[test]
    fn test_timed_effect_progress() {
        let effect = ActiveStatusEffect::new(StatusEffectKind::Speed, 10.0);
        assert!((effect.progress() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_permanent_effect_progress() {
        let effect = ActiveStatusEffect::permanent(StatusEffectKind::NightVision);
        assert!((effect.progress() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_effect_tick_expiry() {
        let mut effect = ActiveStatusEffect::new(StatusEffectKind::Poison, 0.5);
        assert!(!effect.tick(0.3));
        assert!(effect.tick(0.3)); // Expired
    }

    #[test]
    fn test_permanent_never_expires() {
        let mut effect = ActiveStatusEffect::permanent(StatusEffectKind::FireResistance);
        assert!(!effect.tick(100.0));
    }

    #[test]
    fn test_amplifier() {
        let effect = ActiveStatusEffect::new(StatusEffectKind::Speed, 10.0).with_amplifier(2);
        assert_eq!(effect.amplifier, 2);
    }

    #[test]
    fn test_draw_empty_effects() {
        let shapes = draw_status_effects(Pos2::new(0.0, 0.0), &[]);
        assert!(shapes.is_empty());
    }

    #[test]
    fn test_draw_with_effects() {
        let effects = vec![
            ActiveStatusEffect::new(StatusEffectKind::Speed, 10.0),
            ActiveStatusEffect::new(StatusEffectKind::Poison, 5.0),
        ];
        let shapes = draw_status_effects(Pos2::new(0.0, 0.0), &effects);
        assert!(!shapes.is_empty());
    }

    #[test]
    fn test_effect_progress_after_tick() {
        let mut effect = ActiveStatusEffect::new(StatusEffectKind::Regeneration, 10.0);
        effect.tick(5.0);
        assert!((effect.progress() - 0.5).abs() < 0.01);
    }
}
