//! Signal types for automation network communication.

use serde::{Deserialize, Serialize};

/// A signal value transmitted between automation devices.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum SignalValue {
    /// No signal / off state.
    #[default]
    None,
    /// Boolean on/off signal.
    Boolean(bool),
    /// Integer signal (e.g., item counts, redstone-like strength).
    Integer(i32),
    /// Floating-point signal (e.g., power levels, percentages).
    Float(f32),
    /// Packed RGBA color signal.
    Color(u32),
}

impl SignalValue {
    /// The zero/off signal.
    pub const ZERO: Self = Self::None;

    /// Boolean true signal.
    pub const ON: Self = Self::Boolean(true);

    /// Boolean false signal.
    pub const OFF: Self = Self::Boolean(false);

    /// Check if this signal is truthy (non-zero, non-none).
    #[must_use]
    pub fn is_truthy(&self) -> bool {
        match self {
            Self::None => false,
            Self::Boolean(b) => *b,
            Self::Integer(i) => *i != 0,
            Self::Float(f) => *f != 0.0,
            Self::Color(c) => *c != 0,
        }
    }

    /// Check if this signal is falsy.
    #[must_use]
    pub fn is_falsy(&self) -> bool {
        !self.is_truthy()
    }

    /// Convert to boolean.
    #[must_use]
    pub fn to_bool(&self) -> bool {
        self.is_truthy()
    }

    /// Convert to integer (clamped/truncated).
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "intentional lossy conversion"
    )]
    pub fn to_int(&self) -> i32 {
        match self {
            Self::None => 0,
            Self::Boolean(b) => i32::from(*b),
            Self::Integer(i) => *i,
            Self::Float(f) => *f as i32,
            Self::Color(c) => (*c).cast_signed(),
        }
    }

    /// Convert to float.
    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "intentional lossy conversion")]
    pub fn to_float(&self) -> f32 {
        match self {
            Self::None => 0.0,
            Self::Boolean(b) => {
                if *b {
                    1.0
                } else {
                    0.0
                }
            }
            Self::Integer(i) => *i as f32,
            Self::Float(f) => *f,
            Self::Color(c) => *c as f32,
        }
    }

    /// Get the type discriminant for ordering.
    #[must_use]
    fn discriminant(self) -> u8 {
        match self {
            Self::None => 0,
            Self::Boolean(_) => 1,
            Self::Integer(_) => 2,
            Self::Float(_) => 3,
            Self::Color(_) => 4,
        }
    }

    /// Feed this value into a checksum builder.
    pub fn feed_checksum(&self, hasher: &mut crate::ChecksumBuilder) {
        hasher.feed_u32(u32::from((*self).discriminant()));
        match self {
            Self::None => {}
            Self::Boolean(b) => {
                hasher.feed_u32(u32::from(*b));
            }
            Self::Integer(i) => {
                hasher.feed_i32(*i);
            }
            Self::Float(f) => {
                hasher.feed_f32(*f);
            }
            Self::Color(c) => {
                hasher.feed_u32(*c);
            }
        }
    }
}

impl From<bool> for SignalValue {
    fn from(b: bool) -> Self {
        Self::Boolean(b)
    }
}

impl From<i32> for SignalValue {
    fn from(i: i32) -> Self {
        Self::Integer(i)
    }
}

impl From<f32> for SignalValue {
    fn from(f: f32) -> Self {
        Self::Float(f)
    }
}

/// A signal port identifier on a device.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PortId(pub u8);

impl PortId {
    /// Primary input port.
    pub const INPUT_0: Self = Self(0);
    /// Secondary input port.
    pub const INPUT_1: Self = Self(1);
    /// Primary output port.
    pub const OUTPUT_0: Self = Self(128);
    /// Secondary output port.
    pub const OUTPUT_1: Self = Self(129);

    /// Check if this is an input port (id < 128).
    #[must_use]
    pub const fn is_input(&self) -> bool {
        self.0 < 128
    }

    /// Check if this is an output port (id >= 128).
    #[must_use]
    pub const fn is_output(&self) -> bool {
        self.0 >= 128
    }

    /// Convert port ID to array index (0-3 for inputs, 4-7 for outputs).
    #[must_use]
    pub const fn index(&self) -> usize {
        if self.0 < 128 {
            self.0 as usize
        } else {
            (self.0 - 128 + 4) as usize
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signal_value_truthy() {
        assert!(!SignalValue::None.is_truthy());
        assert!(!SignalValue::Boolean(false).is_truthy());
        assert!(SignalValue::Boolean(true).is_truthy());
        assert!(!SignalValue::Integer(0).is_truthy());
        assert!(SignalValue::Integer(1).is_truthy());
        assert!(SignalValue::Integer(-1).is_truthy());
        assert!(!SignalValue::Float(0.0).is_truthy());
        assert!(SignalValue::Float(0.5).is_truthy());
    }

    #[test]
    fn signal_value_conversions() {
        assert_eq!(SignalValue::Boolean(true).to_int(), 1);
        assert!((SignalValue::Integer(42).to_float() - 42.0).abs() < f32::EPSILON);
        assert!(SignalValue::Float(1.5).to_bool());
    }

    #[test]
    fn port_id_classification() {
        assert!(PortId::INPUT_0.is_input());
        assert!(PortId::INPUT_1.is_input());
        assert!(PortId::OUTPUT_0.is_output());
        assert!(PortId::OUTPUT_1.is_output());
    }

    #[test]
    fn serde_roundtrip() {
        let values = [
            SignalValue::None,
            SignalValue::Boolean(true),
            SignalValue::Integer(42),
            SignalValue::Float(1.5),
            SignalValue::Color(0xFF00_FF00),
        ];

        for val in &values {
            let json = serde_json::to_string(val).unwrap();
            let recovered: SignalValue = serde_json::from_str(&json).unwrap();
            assert_eq!(*val, recovered);
        }
    }
}
