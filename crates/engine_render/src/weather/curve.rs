//! Over-time parameter curves for particle systems.
//!
//! Provides deterministic evaluation of time-varying properties
//! like size, color, and velocity over a particle's lifetime.

use serde::{Deserialize, Serialize};

/// Predefined curve shapes for common use cases.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[repr(u8)]
pub enum CurvePreset {
    /// Constant value.
    #[default]
    Constant = 0,
    /// Linear interpolation from start to end.
    Linear = 1,
    /// Ease in (slow start, fast end).
    EaseIn = 2,
    /// Ease out (fast start, slow end).
    EaseOut = 3,
    /// Ease in and out (smooth S-curve).
    EaseInOut = 4,
    /// Pulse: rise then fall.
    Pulse = 5,
    /// Flash: quick rise, slow fall.
    Flash = 6,
    /// Flicker: random-looking variation.
    Flicker = 7,
}

impl CurvePreset {
    /// All curve presets.
    pub const ALL: [Self; 8] = [
        Self::Constant,
        Self::Linear,
        Self::EaseIn,
        Self::EaseOut,
        Self::EaseInOut,
        Self::Pulse,
        Self::Flash,
        Self::Flicker,
    ];

    /// Evaluate the preset at normalized time t (0.0 to 1.0).
    #[must_use]
    pub fn evaluate(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Self::Constant => 1.0,
            Self::Linear => t,
            Self::EaseIn => t * t,
            Self::EaseOut => 1.0 - (1.0 - t) * (1.0 - t),
            Self::EaseInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
                }
            }
            Self::Pulse => {
                let rise = (t * 2.0).min(1.0);
                let fall = ((1.0 - t) * 2.0).min(1.0);
                rise.min(fall)
            }
            Self::Flash => {
                let rise = (t * 4.0).min(1.0);
                let fall = (1.0 - t).powf(0.5);
                rise.min(fall)
            }
            Self::Flicker => {
                let base = (t * 20.0).sin().abs();
                let envelope = (1.0 - t) * 0.5 + 0.5;
                base * envelope
            }
        }
    }
}

/// A keyframe in a custom curve.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Keyframe {
    /// Time (0.0 to 1.0 normalized).
    pub time: f32,
    /// Value at this time.
    pub value: f32,
    /// Tangent for smooth interpolation (optional).
    pub tangent: f32,
}

impl Keyframe {
    /// Create a keyframe with default tangent.
    #[must_use]
    pub fn new(time: f32, value: f32) -> Self {
        Self {
            time: time.clamp(0.0, 1.0),
            value,
            tangent: 0.0,
        }
    }

    /// Create a keyframe with explicit tangent.
    #[must_use]
    pub fn with_tangent(time: f32, value: f32, tangent: f32) -> Self {
        Self {
            time: time.clamp(0.0, 1.0),
            value,
            tangent,
        }
    }
}

impl Default for Keyframe {
    fn default() -> Self {
        Self::new(0.0, 1.0)
    }
}

/// Over-time curve for animating particle properties.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverTimeCurve {
    /// Start value (at t=0).
    pub start: f32,
    /// End value (at t=1).
    pub end: f32,
    /// Curve shape preset (if using preset).
    pub preset: CurvePreset,
    /// Custom keyframes (empty = use preset).
    pub keyframes: Vec<Keyframe>,
    /// Whether to use custom keyframes or preset.
    pub use_keyframes: bool,
}

impl Default for OverTimeCurve {
    fn default() -> Self {
        Self {
            start: 1.0,
            end: 0.0,
            preset: CurvePreset::Linear,
            keyframes: Vec::new(),
            use_keyframes: false,
        }
    }
}

impl OverTimeCurve {
    /// Create a constant curve (no change over time).
    #[must_use]
    pub fn constant(value: f32) -> Self {
        Self {
            start: value,
            end: value,
            preset: CurvePreset::Constant,
            keyframes: Vec::new(),
            use_keyframes: false,
        }
    }

    /// Create a linear fade from start to end.
    #[must_use]
    pub fn linear(start: f32, end: f32) -> Self {
        Self {
            start,
            end,
            preset: CurvePreset::Linear,
            keyframes: Vec::new(),
            use_keyframes: false,
        }
    }

    /// Create a curve using a preset shape.
    #[must_use]
    pub fn from_preset(start: f32, end: f32, preset: CurvePreset) -> Self {
        Self {
            start,
            end,
            preset,
            keyframes: Vec::new(),
            use_keyframes: false,
        }
    }

    /// Create a curve with custom keyframes.
    #[must_use]
    pub fn from_keyframes(keyframes: Vec<Keyframe>) -> Self {
        let start = keyframes.first().map_or(1.0, |k| k.value);
        let end = keyframes.last().map_or(0.0, |k| k.value);
        Self {
            start,
            end,
            preset: CurvePreset::Linear,
            keyframes,
            use_keyframes: true,
        }
    }

    /// Evaluate the curve at normalized time t (0.0 to 1.0).
    #[must_use]
    pub fn evaluate(&self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);

        if self.use_keyframes && !self.keyframes.is_empty() {
            self.evaluate_keyframes(t)
        } else {
            let curve_t = self.preset.evaluate(t);
            self.start + (self.end - self.start) * curve_t
        }
    }

    fn evaluate_keyframes(&self, t: f32) -> f32 {
        if self.keyframes.is_empty() {
            return self.start;
        }

        if t <= self.keyframes[0].time {
            return self.keyframes[0].value;
        }

        let last_idx = self.keyframes.len() - 1;
        if t >= self.keyframes[last_idx].time {
            return self.keyframes[last_idx].value;
        }

        for i in 0..last_idx {
            let k0 = &self.keyframes[i];
            let k1 = &self.keyframes[i + 1];

            if t >= k0.time && t <= k1.time {
                let local_t = if (k1.time - k0.time).abs() < 0.0001 {
                    0.0
                } else {
                    (t - k0.time) / (k1.time - k0.time)
                };

                return hermite_interp(k0.value, k1.value, k0.tangent, k1.tangent, local_t);
            }
        }

        self.keyframes[last_idx].value
    }

    /// Sample the curve at regular intervals for preview.
    #[must_use]
    pub fn sample(&self, count: usize) -> Vec<f32> {
        if count == 0 {
            return Vec::new();
        }
        if count == 1 {
            return vec![self.evaluate(0.5)];
        }

        (0..count)
            .map(|i| {
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "sample count is small; precision loss acceptable"
                )]
                let t = i as f32 / (count - 1) as f32;
                self.evaluate(t)
            })
            .collect()
    }

    /// Get the minimum and maximum values across the curve.
    #[must_use]
    pub fn bounds(&self) -> (f32, f32) {
        let samples = self.sample(32);
        let min = samples.iter().copied().fold(f32::INFINITY, f32::min);
        let max = samples.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        (min, max)
    }

    /// Add a keyframe (sorts by time).
    pub fn add_keyframe(&mut self, keyframe: Keyframe) {
        self.keyframes.push(keyframe);
        self.keyframes.sort_by(|a, b| {
            a.time
                .partial_cmp(&b.time)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        self.use_keyframes = true;

        if let Some(first) = self.keyframes.first() {
            self.start = first.value;
        }
        if let Some(last) = self.keyframes.last() {
            self.end = last.value;
        }
    }

    /// Clear all keyframes.
    pub fn clear_keyframes(&mut self) {
        self.keyframes.clear();
        self.use_keyframes = false;
    }

    /// Check if the curve is valid.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        if self.use_keyframes {
            for (i, k) in self.keyframes.iter().enumerate() {
                if k.time < 0.0 || k.time > 1.0 {
                    return false;
                }
                if i > 0 && k.time < self.keyframes[i - 1].time {
                    return false;
                }
            }
        }
        true
    }
}

fn hermite_interp(v0: f32, v1: f32, t0: f32, t1: f32, t: f32) -> f32 {
    let t2 = t * t;
    let t3 = t2 * t;

    let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
    let h10 = t3 - 2.0 * t2 + t;
    let h01 = -2.0 * t3 + 3.0 * t2;
    let h11 = t3 - t2;

    h00 * v0 + h10 * t0 + h01 * v1 + h11 * t1
}

/// Color curve for animating particle color over lifetime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorOverTime {
    /// Red channel curve.
    pub r: OverTimeCurve,
    /// Green channel curve.
    pub g: OverTimeCurve,
    /// Blue channel curve.
    pub b: OverTimeCurve,
    /// Alpha channel curve.
    pub a: OverTimeCurve,
}

impl Default for ColorOverTime {
    fn default() -> Self {
        Self {
            r: OverTimeCurve::constant(1.0),
            g: OverTimeCurve::constant(1.0),
            b: OverTimeCurve::constant(1.0),
            a: OverTimeCurve::linear(1.0, 0.0),
        }
    }
}

impl ColorOverTime {
    /// Create a constant color.
    #[must_use]
    pub fn constant(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self {
            r: OverTimeCurve::constant(r),
            g: OverTimeCurve::constant(g),
            b: OverTimeCurve::constant(b),
            a: OverTimeCurve::constant(a),
        }
    }

    /// Create a fade out (alpha goes to 0).
    #[must_use]
    pub fn fade_out(r: f32, g: f32, b: f32) -> Self {
        Self {
            r: OverTimeCurve::constant(r),
            g: OverTimeCurve::constant(g),
            b: OverTimeCurve::constant(b),
            a: OverTimeCurve::linear(1.0, 0.0),
        }
    }

    /// Create color that shifts from one to another.
    #[must_use]
    pub fn gradient(start: (f32, f32, f32, f32), end: (f32, f32, f32, f32)) -> Self {
        Self {
            r: OverTimeCurve::linear(start.0, end.0),
            g: OverTimeCurve::linear(start.1, end.1),
            b: OverTimeCurve::linear(start.2, end.2),
            a: OverTimeCurve::linear(start.3, end.3),
        }
    }

    /// Evaluate at time t.
    #[must_use]
    pub fn evaluate(&self, t: f32) -> (f32, f32, f32, f32) {
        (
            self.r.evaluate(t).clamp(0.0, 1.0),
            self.g.evaluate(t).clamp(0.0, 1.0),
            self.b.evaluate(t).clamp(0.0, 1.0),
            self.a.evaluate(t).clamp(0.0, 1.0),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_preset_constant() {
        assert_relative_eq!(CurvePreset::Constant.evaluate(0.0), 1.0, epsilon = 0.001);
        assert_relative_eq!(CurvePreset::Constant.evaluate(0.5), 1.0, epsilon = 0.001);
        assert_relative_eq!(CurvePreset::Constant.evaluate(1.0), 1.0, epsilon = 0.001);
    }

    #[test]
    fn test_preset_linear() {
        assert_relative_eq!(CurvePreset::Linear.evaluate(0.0), 0.0, epsilon = 0.001);
        assert_relative_eq!(CurvePreset::Linear.evaluate(0.5), 0.5, epsilon = 0.001);
        assert_relative_eq!(CurvePreset::Linear.evaluate(1.0), 1.0, epsilon = 0.001);
    }

    #[test]
    fn test_preset_ease_in() {
        let mid = CurvePreset::EaseIn.evaluate(0.5);
        assert!(mid < 0.5, "ease in should be below linear at midpoint");
    }

    #[test]
    fn test_preset_ease_out() {
        let mid = CurvePreset::EaseOut.evaluate(0.5);
        assert!(mid > 0.5, "ease out should be above linear at midpoint");
    }

    #[test]
    fn test_preset_ease_in_out_endpoints() {
        assert_relative_eq!(CurvePreset::EaseInOut.evaluate(0.0), 0.0, epsilon = 0.001);
        assert_relative_eq!(CurvePreset::EaseInOut.evaluate(1.0), 1.0, epsilon = 0.001);
    }

    #[test]
    fn test_preset_pulse_peak() {
        let peak = CurvePreset::Pulse.evaluate(0.5);
        let start = CurvePreset::Pulse.evaluate(0.0);
        let end = CurvePreset::Pulse.evaluate(1.0);

        assert!(peak > start);
        assert!(peak > end);
    }

    #[test]
    fn test_curve_constant() {
        let curve = OverTimeCurve::constant(0.75);
        assert_relative_eq!(curve.evaluate(0.0), 0.75, epsilon = 0.001);
        assert_relative_eq!(curve.evaluate(0.5), 0.75, epsilon = 0.001);
        assert_relative_eq!(curve.evaluate(1.0), 0.75, epsilon = 0.001);
    }

    #[test]
    fn test_curve_linear() {
        let curve = OverTimeCurve::linear(0.0, 1.0);
        assert_relative_eq!(curve.evaluate(0.0), 0.0, epsilon = 0.001);
        assert_relative_eq!(curve.evaluate(0.5), 0.5, epsilon = 0.001);
        assert_relative_eq!(curve.evaluate(1.0), 1.0, epsilon = 0.001);
    }

    #[test]
    fn test_curve_linear_reversed() {
        let curve = OverTimeCurve::linear(1.0, 0.0);
        assert_relative_eq!(curve.evaluate(0.0), 1.0, epsilon = 0.001);
        assert_relative_eq!(curve.evaluate(1.0), 0.0, epsilon = 0.001);
    }

    #[test]
    fn test_curve_keyframes() {
        let curve = OverTimeCurve::from_keyframes(vec![
            Keyframe::new(0.0, 0.0),
            Keyframe::new(0.5, 1.0),
            Keyframe::new(1.0, 0.5),
        ]);

        assert_relative_eq!(curve.evaluate(0.0), 0.0, epsilon = 0.001);
        assert_relative_eq!(curve.evaluate(1.0), 0.5, epsilon = 0.001);
    }

    #[test]
    fn test_curve_bounds() {
        let curve = OverTimeCurve::linear(0.2, 0.8);
        let (min, max) = curve.bounds();

        assert!(min >= 0.2 - 0.01);
        assert!(max <= 0.8 + 0.01);
    }

    #[test]
    fn test_curve_sample() {
        let curve = OverTimeCurve::linear(0.0, 1.0);
        let samples = curve.sample(5);

        assert_eq!(samples.len(), 5);
        assert_relative_eq!(samples[0], 0.0, epsilon = 0.001);
        assert_relative_eq!(samples[4], 1.0, epsilon = 0.001);
    }

    #[test]
    fn test_curve_add_keyframe() {
        let mut curve = OverTimeCurve::default();
        curve.add_keyframe(Keyframe::new(0.5, 0.5));
        curve.add_keyframe(Keyframe::new(0.0, 1.0));
        curve.add_keyframe(Keyframe::new(1.0, 0.0));

        assert!(curve.use_keyframes);
        assert_eq!(curve.keyframes.len(), 3);
        assert_relative_eq!(curve.keyframes[0].time, 0.0, epsilon = 0.001);
    }

    #[test]
    fn test_curve_is_valid() {
        let valid = OverTimeCurve::linear(0.0, 1.0);
        assert!(valid.is_valid());

        let mut invalid =
            OverTimeCurve::from_keyframes(vec![Keyframe::new(0.5, 0.5), Keyframe::new(0.3, 0.3)]);
        invalid.keyframes[1].time = 0.3;
        assert!(!invalid.is_valid());
    }

    #[test]
    fn test_color_over_time_fade() {
        let color = ColorOverTime::fade_out(1.0, 0.5, 0.0);

        let (r, _g, _b, a) = color.evaluate(0.0);
        assert_relative_eq!(r, 1.0, epsilon = 0.001);
        assert_relative_eq!(a, 1.0, epsilon = 0.001);

        let (_, _, _, a_end) = color.evaluate(1.0);
        assert_relative_eq!(a_end, 0.0, epsilon = 0.001);
    }

    #[test]
    fn test_color_gradient() {
        let color = ColorOverTime::gradient((1.0, 0.0, 0.0, 1.0), (0.0, 0.0, 1.0, 1.0));

        let (r_start, _, b_start, _) = color.evaluate(0.0);
        assert_relative_eq!(r_start, 1.0, epsilon = 0.001);
        assert_relative_eq!(b_start, 0.0, epsilon = 0.001);

        let (r_end, _, b_end, _) = color.evaluate(1.0);
        assert_relative_eq!(r_end, 0.0, epsilon = 0.001);
        assert_relative_eq!(b_end, 1.0, epsilon = 0.001);
    }

    #[test]
    fn test_all_presets_bounded() {
        for preset in CurvePreset::ALL {
            for i in 0..=10 {
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "small values, precision loss acceptable"
                )]
                let t = i as f32 / 10.0;
                let v = preset.evaluate(t);
                assert!(
                    (-0.1..=1.1).contains(&v),
                    "{preset:?} at t={t} produced {v}"
                );
            }
        }
    }

    #[test]
    fn test_hermite_interp_endpoints() {
        let v = hermite_interp(0.0, 1.0, 0.0, 0.0, 0.0);
        assert_relative_eq!(v, 0.0, epsilon = 0.001);

        let v = hermite_interp(0.0, 1.0, 0.0, 0.0, 1.0);
        assert_relative_eq!(v, 1.0, epsilon = 0.001);
    }
}
