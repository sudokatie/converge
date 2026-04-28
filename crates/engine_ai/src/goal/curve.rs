//! Utility curves for mapping input values to utility scores.

use serde::{Deserialize, Serialize};

/// Type of utility curve for value transformation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CurveKind {
    /// Linear interpolation.
    Linear {
        slope: f32,
        x_shift: f32,
        y_shift: f32,
    },
    /// Polynomial curve with configurable exponent.
    Polynomial {
        slope: f32,
        exponent: f32,
        x_shift: f32,
        y_shift: f32,
    },
    /// Logistic (sigmoid) curve.
    Logistic { slope: f32, x_shift: f32 },
    /// Logit (inverse sigmoid) curve.
    Logit { slope: f32, y_shift: f32 },
    /// Sine wave curve.
    Sine {
        frequency: f32,
        amplitude: f32,
        x_shift: f32,
        y_shift: f32,
    },
    /// Step function with configurable threshold.
    Step { threshold: f32, low: f32, high: f32 },
    /// Constant value regardless of input.
    Constant(f32),
    /// Custom piecewise linear from points.
    Piecewise(Vec<(f32, f32)>),
}

impl CurveKind {
    /// Evaluate the curve at a given x value.
    #[must_use]
    pub fn evaluate(&self, x: f32) -> f32 {
        match self {
            Self::Linear {
                slope,
                x_shift,
                y_shift,
            } => slope * (x - x_shift) + y_shift,

            Self::Polynomial {
                slope,
                exponent,
                x_shift,
                y_shift,
            } => slope * (x - x_shift).powf(*exponent) + y_shift,

            Self::Logistic { slope, x_shift } => 1.0 / (1.0 + (-slope * (x - x_shift)).exp()),

            Self::Logit { slope, y_shift } => {
                let clamped = x.clamp(0.001, 0.999);
                (clamped / (1.0 - clamped)).ln() * slope + y_shift
            }

            Self::Sine {
                frequency,
                amplitude,
                x_shift,
                y_shift,
            } => ((x - x_shift) * frequency).sin() * amplitude + y_shift,

            Self::Step {
                threshold,
                low,
                high,
            } => {
                if x < *threshold {
                    *low
                } else {
                    *high
                }
            }

            Self::Constant(c) => *c,

            Self::Piecewise(points) => {
                if points.is_empty() {
                    return 0.0;
                }
                if points.len() == 1 {
                    return points[0].1;
                }

                if x <= points[0].0 {
                    return points[0].1;
                }
                if x >= points[points.len() - 1].0 {
                    return points[points.len() - 1].1;
                }

                for window in points.windows(2) {
                    let (x0, y0) = window[0];
                    let (x1, y1) = window[1];
                    if x >= x0 && x <= x1 {
                        let t = if (x1 - x0).abs() < f32::EPSILON {
                            0.0
                        } else {
                            (x - x0) / (x1 - x0)
                        };
                        return y0 + t * (y1 - y0);
                    }
                }

                points[points.len() - 1].1
            }
        }
    }
}

impl Default for CurveKind {
    fn default() -> Self {
        Self::Linear {
            slope: 1.0,
            x_shift: 0.0,
            y_shift: 0.0,
        }
    }
}

/// A utility curve with configurable clamping and inversion.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UtilityCurve {
    /// The curve function.
    pub kind: CurveKind,
    /// Minimum output value (clamp floor).
    pub min_output: f32,
    /// Maximum output value (clamp ceiling).
    pub max_output: f32,
    /// Whether to invert the output (1.0 - y).
    pub invert: bool,
}

impl UtilityCurve {
    /// Create a new utility curve.
    #[must_use]
    pub fn new(kind: CurveKind) -> Self {
        Self {
            kind,
            min_output: 0.0,
            max_output: 1.0,
            invert: false,
        }
    }

    /// Set output clamping range.
    #[must_use]
    pub fn with_clamp(mut self, min: f32, max: f32) -> Self {
        self.min_output = min;
        self.max_output = max;
        self
    }

    /// Set output inversion.
    #[must_use]
    pub fn with_invert(mut self, invert: bool) -> Self {
        self.invert = invert;
        self
    }

    /// Evaluate the curve at a given input.
    #[must_use]
    pub fn evaluate(&self, input: f32) -> f32 {
        let mut y = self.kind.evaluate(input);

        if self.invert {
            y = 1.0 - y;
        }

        y.clamp(self.min_output, self.max_output)
    }

    /// Create a linear curve from 0 to 1.
    #[must_use]
    pub fn linear() -> Self {
        Self::new(CurveKind::Linear {
            slope: 1.0,
            x_shift: 0.0,
            y_shift: 0.0,
        })
    }

    /// Create an inverse linear curve (1 to 0).
    #[must_use]
    pub fn inverse_linear() -> Self {
        Self::new(CurveKind::Linear {
            slope: -1.0,
            x_shift: 0.0,
            y_shift: 1.0,
        })
    }

    /// Create a quadratic curve (slow start, fast finish).
    #[must_use]
    pub fn quadratic() -> Self {
        Self::new(CurveKind::Polynomial {
            slope: 1.0,
            exponent: 2.0,
            x_shift: 0.0,
            y_shift: 0.0,
        })
    }

    /// Create an inverse quadratic curve.
    #[must_use]
    pub fn inverse_quadratic() -> Self {
        Self::new(CurveKind::Polynomial {
            slope: -1.0,
            exponent: 2.0,
            x_shift: 1.0,
            y_shift: 1.0,
        })
    }

    /// Create a sigmoid curve (slow-fast-slow transition).
    #[must_use]
    pub fn sigmoid() -> Self {
        Self::new(CurveKind::Logistic {
            slope: 10.0,
            x_shift: 0.5,
        })
    }

    /// Create a step function at 0.5.
    #[must_use]
    pub fn step() -> Self {
        Self::new(CurveKind::Step {
            threshold: 0.5,
            low: 0.0,
            high: 1.0,
        })
    }

    /// Create a constant curve.
    #[must_use]
    pub fn constant(value: f32) -> Self {
        Self::new(CurveKind::Constant(value))
    }

    /// Create a piecewise curve from points.
    #[must_use]
    pub fn piecewise(mut points: Vec<(f32, f32)>) -> Self {
        points.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        Self::new(CurveKind::Piecewise(points))
    }
}

impl Default for UtilityCurve {
    fn default() -> Self {
        Self::linear()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_curve() {
        let curve = UtilityCurve::linear();

        assert!((curve.evaluate(0.0)).abs() < f32::EPSILON);
        assert!((curve.evaluate(0.5) - 0.5).abs() < f32::EPSILON);
        assert!((curve.evaluate(1.0) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_inverse_linear_curve() {
        let curve = UtilityCurve::inverse_linear();

        assert!((curve.evaluate(0.0) - 1.0).abs() < f32::EPSILON);
        assert!((curve.evaluate(0.5) - 0.5).abs() < f32::EPSILON);
        assert!((curve.evaluate(1.0)).abs() < f32::EPSILON);
    }

    #[test]
    fn test_quadratic_curve() {
        let curve = UtilityCurve::quadratic();

        assert!((curve.evaluate(0.0)).abs() < f32::EPSILON);
        assert!((curve.evaluate(0.5) - 0.25).abs() < f32::EPSILON);
        assert!((curve.evaluate(1.0) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_sigmoid_curve() {
        let curve = UtilityCurve::sigmoid();

        assert!(curve.evaluate(0.0) < 0.1);
        assert!((curve.evaluate(0.5) - 0.5).abs() < 0.01);
        assert!(curve.evaluate(1.0) > 0.9);
    }

    #[test]
    fn test_step_curve() {
        let curve = UtilityCurve::step();

        assert!((curve.evaluate(0.0)).abs() < f32::EPSILON);
        assert!((curve.evaluate(0.49)).abs() < f32::EPSILON);
        assert!((curve.evaluate(0.5) - 1.0).abs() < f32::EPSILON);
        assert!((curve.evaluate(1.0) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_constant_curve() {
        let curve = UtilityCurve::constant(0.75);

        assert!((curve.evaluate(0.0) - 0.75).abs() < f32::EPSILON);
        assert!((curve.evaluate(0.5) - 0.75).abs() < f32::EPSILON);
        assert!((curve.evaluate(1.0) - 0.75).abs() < f32::EPSILON);
    }

    #[test]
    fn test_piecewise_curve() {
        let curve = UtilityCurve::piecewise(vec![(0.0, 0.0), (0.3, 0.8), (1.0, 1.0)]);

        assert!((curve.evaluate(0.0)).abs() < f32::EPSILON);
        assert!((curve.evaluate(0.15) - 0.4).abs() < 0.01);
        assert!((curve.evaluate(0.3) - 0.8).abs() < f32::EPSILON);
        assert!((curve.evaluate(1.0) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_piecewise_extrapolation() {
        let curve = UtilityCurve::piecewise(vec![(0.2, 0.3), (0.8, 0.7)]);

        assert!((curve.evaluate(0.0) - 0.3).abs() < f32::EPSILON);
        assert!((curve.evaluate(1.0) - 0.7).abs() < f32::EPSILON);
    }

    #[test]
    fn test_curve_clamping() {
        let curve = UtilityCurve::new(CurveKind::Linear {
            slope: 2.0,
            x_shift: 0.0,
            y_shift: 0.0,
        })
        .with_clamp(0.0, 1.0);

        assert!((curve.evaluate(0.75) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_curve_inversion() {
        let curve = UtilityCurve::linear().with_invert(true);

        assert!((curve.evaluate(0.0) - 1.0).abs() < f32::EPSILON);
        assert!((curve.evaluate(1.0)).abs() < f32::EPSILON);
    }

    #[test]
    fn test_logit_curve() {
        let curve = UtilityCurve::new(CurveKind::Logit {
            slope: 0.1,
            y_shift: 0.5,
        });

        assert!((curve.evaluate(0.5) - 0.5).abs() < f32::EPSILON);
        assert!(curve.evaluate(0.9) > curve.evaluate(0.5));
        assert!(curve.evaluate(0.1) < curve.evaluate(0.5));
    }

    #[test]
    fn test_sine_curve() {
        let curve = UtilityCurve::new(CurveKind::Sine {
            frequency: std::f32::consts::PI,
            amplitude: 0.5,
            x_shift: 0.0,
            y_shift: 0.5,
        });

        assert!((curve.evaluate(0.0) - 0.5).abs() < 0.01);
        assert!((curve.evaluate(0.5) - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_polynomial_cube() {
        let curve = UtilityCurve::new(CurveKind::Polynomial {
            slope: 1.0,
            exponent: 3.0,
            x_shift: 0.0,
            y_shift: 0.0,
        });

        assert!((curve.evaluate(0.5) - 0.125).abs() < f32::EPSILON);
    }

    #[test]
    fn test_serde_round_trip() {
        let curve = UtilityCurve::piecewise(vec![(0.0, 0.0), (0.5, 0.8), (1.0, 1.0)])
            .with_clamp(0.1, 0.9)
            .with_invert(true);

        let json = serde_json::to_string(&curve).unwrap();
        let restored: UtilityCurve = serde_json::from_str(&json).unwrap();

        assert!((restored.min_output - curve.min_output).abs() < f32::EPSILON);
        assert!((restored.max_output - curve.max_output).abs() < f32::EPSILON);
        assert_eq!(restored.invert, curve.invert);
    }

    #[test]
    fn test_default_curve() {
        let curve = UtilityCurve::default();
        assert!((curve.evaluate(0.5) - 0.5).abs() < f32::EPSILON);
    }
}
