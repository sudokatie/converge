//! Easing functions for smooth camera interpolation.
//!
//! Provides standard easing curves for keyframe-based animation
//! with deterministic, CPU-side evaluation.

use serde::{Deserialize, Serialize};
use std::f32::consts::PI;

/// Easing function type for interpolation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum EasingFunction {
    /// Linear interpolation (no easing).
    #[default]
    Linear = 0,
    /// Ease in (slow start).
    EaseIn = 1,
    /// Ease out (slow end).
    EaseOut = 2,
    /// Ease in and out (slow start and end).
    EaseInOut = 3,
    /// Quadratic ease in.
    QuadIn = 4,
    /// Quadratic ease out.
    QuadOut = 5,
    /// Quadratic ease in/out.
    QuadInOut = 6,
    /// Cubic ease in.
    CubicIn = 7,
    /// Cubic ease out.
    CubicOut = 8,
    /// Cubic ease in/out.
    CubicInOut = 9,
    /// Quartic ease in.
    QuartIn = 10,
    /// Quartic ease out.
    QuartOut = 11,
    /// Quartic ease in/out.
    QuartInOut = 12,
    /// Exponential ease in.
    ExpoIn = 13,
    /// Exponential ease out.
    ExpoOut = 14,
    /// Exponential ease in/out.
    ExpoInOut = 15,
    /// Sinusoidal ease in.
    SineIn = 16,
    /// Sinusoidal ease out.
    SineOut = 17,
    /// Sinusoidal ease in/out.
    SineInOut = 18,
    /// Elastic ease in (spring effect).
    ElasticIn = 19,
    /// Elastic ease out (spring effect).
    ElasticOut = 20,
    /// Back ease in (anticipation).
    BackIn = 21,
    /// Back ease out (overshoot).
    BackOut = 22,
    /// Back ease in/out.
    BackInOut = 23,
    /// Bounce ease out.
    BounceOut = 24,
    /// Step function (instant transition at 0.5).
    Step = 25,
    /// Smooth step (Hermite interpolation).
    SmoothStep = 26,
    /// Smoother step (higher-order Hermite).
    SmootherStep = 27,
}

impl EasingFunction {
    /// All available easing functions.
    pub const ALL: [EasingFunction; 28] = [
        EasingFunction::Linear,
        EasingFunction::EaseIn,
        EasingFunction::EaseOut,
        EasingFunction::EaseInOut,
        EasingFunction::QuadIn,
        EasingFunction::QuadOut,
        EasingFunction::QuadInOut,
        EasingFunction::CubicIn,
        EasingFunction::CubicOut,
        EasingFunction::CubicInOut,
        EasingFunction::QuartIn,
        EasingFunction::QuartOut,
        EasingFunction::QuartInOut,
        EasingFunction::ExpoIn,
        EasingFunction::ExpoOut,
        EasingFunction::ExpoInOut,
        EasingFunction::SineIn,
        EasingFunction::SineOut,
        EasingFunction::SineInOut,
        EasingFunction::ElasticIn,
        EasingFunction::ElasticOut,
        EasingFunction::BackIn,
        EasingFunction::BackOut,
        EasingFunction::BackInOut,
        EasingFunction::BounceOut,
        EasingFunction::Step,
        EasingFunction::SmoothStep,
        EasingFunction::SmootherStep,
    ];

    /// Evaluate the easing function at parameter t (0.0 to 1.0).
    #[must_use]
    pub fn evaluate(self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            EasingFunction::Linear => t,
            EasingFunction::EaseIn | EasingFunction::CubicIn => ease_in_cubic(t),
            EasingFunction::EaseOut | EasingFunction::CubicOut => ease_out_cubic(t),
            EasingFunction::EaseInOut | EasingFunction::CubicInOut => ease_in_out_cubic(t),
            EasingFunction::QuadIn => t * t,
            EasingFunction::QuadOut => 1.0 - (1.0 - t) * (1.0 - t),
            EasingFunction::QuadInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
                }
            }
            EasingFunction::QuartIn => t * t * t * t,
            EasingFunction::QuartOut => 1.0 - (1.0 - t).powi(4),
            EasingFunction::QuartInOut => {
                if t < 0.5 {
                    8.0 * t * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(4) / 2.0
                }
            }
            EasingFunction::ExpoIn => expo_in(t),
            EasingFunction::ExpoOut => expo_out(t),
            EasingFunction::ExpoInOut => expo_in_out(t),
            EasingFunction::SineIn => 1.0 - (t * PI / 2.0).cos(),
            EasingFunction::SineOut => (t * PI / 2.0).sin(),
            EasingFunction::SineInOut => -(PI * t).cos() / 2.0 + 0.5,
            EasingFunction::ElasticIn => elastic_in(t),
            EasingFunction::ElasticOut => elastic_out(t),
            EasingFunction::BackIn => back_in(t),
            EasingFunction::BackOut => back_out(t),
            EasingFunction::BackInOut => back_in_out(t),
            EasingFunction::BounceOut => bounce_out(t),
            EasingFunction::Step => {
                if t < 0.5 {
                    0.0
                } else {
                    1.0
                }
            }
            EasingFunction::SmoothStep => smooth_step(t),
            EasingFunction::SmootherStep => smoother_step(t),
        }
    }

    /// Get the function name for display.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            EasingFunction::Linear => "Linear",
            EasingFunction::EaseIn => "Ease In",
            EasingFunction::EaseOut => "Ease Out",
            EasingFunction::EaseInOut => "Ease In/Out",
            EasingFunction::QuadIn => "Quad In",
            EasingFunction::QuadOut => "Quad Out",
            EasingFunction::QuadInOut => "Quad In/Out",
            EasingFunction::CubicIn => "Cubic In",
            EasingFunction::CubicOut => "Cubic Out",
            EasingFunction::CubicInOut => "Cubic In/Out",
            EasingFunction::QuartIn => "Quart In",
            EasingFunction::QuartOut => "Quart Out",
            EasingFunction::QuartInOut => "Quart In/Out",
            EasingFunction::ExpoIn => "Expo In",
            EasingFunction::ExpoOut => "Expo Out",
            EasingFunction::ExpoInOut => "Expo In/Out",
            EasingFunction::SineIn => "Sine In",
            EasingFunction::SineOut => "Sine Out",
            EasingFunction::SineInOut => "Sine In/Out",
            EasingFunction::ElasticIn => "Elastic In",
            EasingFunction::ElasticOut => "Elastic Out",
            EasingFunction::BackIn => "Back In",
            EasingFunction::BackOut => "Back Out",
            EasingFunction::BackInOut => "Back In/Out",
            EasingFunction::BounceOut => "Bounce Out",
            EasingFunction::Step => "Step",
            EasingFunction::SmoothStep => "Smooth Step",
            EasingFunction::SmootherStep => "Smoother Step",
        }
    }

    /// Whether this easing can overshoot the target value.
    #[must_use]
    pub const fn can_overshoot(&self) -> bool {
        matches!(
            self,
            EasingFunction::ElasticIn
                | EasingFunction::ElasticOut
                | EasingFunction::BackIn
                | EasingFunction::BackOut
                | EasingFunction::BackInOut
                | EasingFunction::BounceOut
        )
    }

    /// Whether this easing is symmetric around t=0.5.
    #[must_use]
    pub const fn is_symmetric(&self) -> bool {
        matches!(
            self,
            EasingFunction::Linear
                | EasingFunction::EaseInOut
                | EasingFunction::QuadInOut
                | EasingFunction::CubicInOut
                | EasingFunction::QuartInOut
                | EasingFunction::ExpoInOut
                | EasingFunction::SineInOut
                | EasingFunction::BackInOut
                | EasingFunction::SmoothStep
                | EasingFunction::SmootherStep
        )
    }
}

fn ease_in_cubic(t: f32) -> f32 {
    t * t * t
}

fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

fn ease_in_out_cubic(t: f32) -> f32 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

fn expo_in(t: f32) -> f32 {
    if t == 0.0 {
        0.0
    } else {
        2.0_f32.powf(10.0 * t - 10.0)
    }
}

fn expo_out(t: f32) -> f32 {
    if (t - 1.0).abs() < f32::EPSILON {
        1.0
    } else {
        1.0 - 2.0_f32.powf(-10.0 * t)
    }
}

fn expo_in_out(t: f32) -> f32 {
    if t.abs() < f32::EPSILON {
        return 0.0;
    }
    if (t - 1.0).abs() < f32::EPSILON {
        return 1.0;
    }
    if t < 0.5 {
        2.0_f32.powf(20.0 * t - 10.0) / 2.0
    } else {
        (2.0 - 2.0_f32.powf(-20.0 * t + 10.0)) / 2.0
    }
}

fn elastic_in(t: f32) -> f32 {
    if t.abs() < f32::EPSILON {
        return 0.0;
    }
    if (t - 1.0).abs() < f32::EPSILON {
        return 1.0;
    }
    let c4 = (2.0 * PI) / 3.0;
    -2.0_f32.powf(10.0 * t - 10.0) * ((t * 10.0 - 10.75) * c4).sin()
}

fn elastic_out(t: f32) -> f32 {
    if t.abs() < f32::EPSILON {
        return 0.0;
    }
    if (t - 1.0).abs() < f32::EPSILON {
        return 1.0;
    }
    let c4 = (2.0 * PI) / 3.0;
    2.0_f32.powf(-10.0 * t) * ((t * 10.0 - 0.75) * c4).sin() + 1.0
}

fn back_in(t: f32) -> f32 {
    const C1: f32 = 1.70158;
    const C3: f32 = C1 + 1.0;
    C3 * t * t * t - C1 * t * t
}

fn back_out(t: f32) -> f32 {
    const C1: f32 = 1.70158;
    const C3: f32 = C1 + 1.0;
    1.0 + C3 * (t - 1.0).powi(3) + C1 * (t - 1.0).powi(2)
}

fn back_in_out(t: f32) -> f32 {
    const C1: f32 = 1.70158;
    const C2: f32 = C1 * 1.525;
    if t < 0.5 {
        ((2.0 * t).powi(2) * ((C2 + 1.0) * 2.0 * t - C2)) / 2.0
    } else {
        let base = (2.0 * t - 2.0).powi(2) * ((C2 + 1.0) * (t * 2.0 - 2.0) + C2);
        f32::midpoint(base, 2.0)
    }
}

fn bounce_out(t: f32) -> f32 {
    const N1: f32 = 7.5625;
    const D1: f32 = 2.75;

    if t < 1.0 / D1 {
        N1 * t * t
    } else if t < 2.0 / D1 {
        let t = t - 1.5 / D1;
        N1 * t * t + 0.75
    } else if t < 2.5 / D1 {
        let t = t - 2.25 / D1;
        N1 * t * t + 0.9375
    } else {
        let t = t - 2.625 / D1;
        N1 * t * t + 0.984_375
    }
}

fn smooth_step(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

fn smoother_step(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

/// Interpolate between two values using an easing function.
#[must_use]
pub fn lerp_eased(a: f32, b: f32, t: f32, easing: EasingFunction) -> f32 {
    let eased_t = easing.evaluate(t);
    a + (b - a) * eased_t
}

/// Sample an easing curve for preview/debugging.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn sample_curve(easing: EasingFunction, samples: usize) -> Vec<f32> {
    let count = samples.max(2);
    (0..count)
        .map(|i| {
            let t = i as f32 / (count - 1) as f32;
            easing.evaluate(t)
        })
        .collect()
}

/// Compute derivative of easing function at t (for velocity calculation).
#[must_use]
pub fn easing_derivative(easing: EasingFunction, t: f32) -> f32 {
    const H: f32 = 0.0001;
    let t = t.clamp(H, 1.0 - H);
    (easing.evaluate(t + H) - easing.evaluate(t - H)) / (2.0 * H)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_linear_easing() {
        assert_relative_eq!(EasingFunction::Linear.evaluate(0.0), 0.0);
        assert_relative_eq!(EasingFunction::Linear.evaluate(0.5), 0.5);
        assert_relative_eq!(EasingFunction::Linear.evaluate(1.0), 1.0);
    }

    #[test]
    fn test_all_easings_bounded_at_endpoints() {
        for easing in EasingFunction::ALL {
            let at_zero = easing.evaluate(0.0);
            let at_one = easing.evaluate(1.0);

            assert!(
                (at_zero - 0.0).abs() < 0.01,
                "{easing:?} at t=0.0: {at_zero}"
            );
            assert!((at_one - 1.0).abs() < 0.01, "{easing:?} at t=1.0: {at_one}");
        }
    }

    #[test]
    fn test_symmetric_easings() {
        for easing in EasingFunction::ALL {
            if easing.is_symmetric() {
                let at_half = easing.evaluate(0.5);
                assert!(
                    (at_half - 0.5).abs() < 0.01,
                    "{easing:?} should be ~0.5 at t=0.5: {at_half}"
                );
            }
        }
    }

    #[test]
    fn test_easing_clamping() {
        for easing in EasingFunction::ALL {
            let below_zero = easing.evaluate(-0.5);
            let above_one = easing.evaluate(1.5);

            assert_relative_eq!(below_zero, easing.evaluate(0.0));
            assert_relative_eq!(above_one, easing.evaluate(1.0));
        }
    }

    #[test]
    fn test_lerp_eased() {
        let result = lerp_eased(10.0, 20.0, 0.5, EasingFunction::Linear);
        assert_relative_eq!(result, 15.0);

        let eased = lerp_eased(0.0, 100.0, 0.5, EasingFunction::SmoothStep);
        assert_relative_eq!(eased, 50.0);
    }

    #[test]
    fn test_sample_curve() {
        let samples = sample_curve(EasingFunction::Linear, 5);
        assert_eq!(samples.len(), 5);
        assert_relative_eq!(samples[0], 0.0);
        assert_relative_eq!(samples[2], 0.5);
        assert_relative_eq!(samples[4], 1.0);
    }

    #[test]
    fn test_smooth_step() {
        let s = smooth_step(0.0);
        assert_relative_eq!(s, 0.0);
        let s = smooth_step(1.0);
        assert_relative_eq!(s, 1.0);
        let s = smooth_step(0.5);
        assert_relative_eq!(s, 0.5);
    }

    #[test]
    fn test_ease_in_slower_at_start() {
        let ease_in_val = EasingFunction::CubicIn.evaluate(0.25);
        let linear_val = EasingFunction::Linear.evaluate(0.25);
        assert!(ease_in_val < linear_val);
    }

    #[test]
    fn test_ease_out_faster_at_start() {
        let ease_out_val = EasingFunction::CubicOut.evaluate(0.25);
        let linear_val = EasingFunction::Linear.evaluate(0.25);
        assert!(ease_out_val > linear_val);
    }

    #[test]
    fn test_overshoot_functions() {
        let back_out = EasingFunction::BackOut.evaluate(0.1);
        assert!(!(0.0..=0.1).contains(&back_out), "BackOut should overshoot");
    }

    #[test]
    fn test_step_function() {
        assert_relative_eq!(EasingFunction::Step.evaluate(0.49), 0.0);
        assert_relative_eq!(EasingFunction::Step.evaluate(0.51), 1.0);
    }

    #[test]
    fn test_derivative_nonzero() {
        let d = easing_derivative(EasingFunction::Linear, 0.5);
        assert!(d > 0.0);

        let d = easing_derivative(EasingFunction::SmoothStep, 0.5);
        assert!(d > 0.0);
    }

    #[test]
    fn test_serde_roundtrip() {
        for easing in EasingFunction::ALL {
            let bytes = bincode::serialize(&easing).expect("serialize");
            let restored: EasingFunction = bincode::deserialize(&bytes).expect("deserialize");
            assert_eq!(easing, restored);
        }
    }
}
