//! Concrete automation primitive types with typed configurations.
//!
//! Provides deterministic, serde-covered implementations for common automation
//! devices: pumps, valves, relays, sensors, timers, and logic gates.

use serde::{Deserialize, Serialize};

use super::signal::SignalValue;

// -----------------------------------------------------------------------------
// Logic Gate Primitives
// -----------------------------------------------------------------------------

/// Logic gate operation type.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum GateOp {
    /// Logical AND: output true when all inputs are true.
    #[default]
    And = 0,
    /// Logical OR: output true when any input is true.
    Or = 1,
    /// Logical NOT: invert the first input.
    Not = 2,
    /// Logical XOR: output true when inputs differ.
    Xor = 3,
    /// Logical NAND: inverted AND.
    Nand = 4,
    /// Logical NOR: inverted OR.
    Nor = 5,
    /// Logical XNOR: inverted XOR (equivalence).
    Xnor = 6,
}

impl GateOp {
    /// Evaluate the gate operation on two boolean inputs.
    #[must_use]
    pub const fn evaluate(self, a: bool, b: bool) -> bool {
        match self {
            Self::And => a && b,
            Self::Or => a || b,
            Self::Not => !a,
            Self::Xor => a ^ b,
            Self::Nand => !(a && b),
            Self::Nor => !(a || b),
            Self::Xnor => a == b,
        }
    }

    /// Evaluate the gate operation on signal values.
    #[must_use]
    pub fn evaluate_signals(self, a: SignalValue, b: SignalValue) -> SignalValue {
        SignalValue::Boolean(self.evaluate(a.is_truthy(), b.is_truthy()))
    }

    /// Get display name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::And => "AND",
            Self::Or => "OR",
            Self::Not => "NOT",
            Self::Xor => "XOR",
            Self::Nand => "NAND",
            Self::Nor => "NOR",
            Self::Xnor => "XNOR",
        }
    }

    /// Number of inputs required.
    #[must_use]
    pub const fn input_count(self) -> u8 {
        match self {
            Self::Not => 1,
            _ => 2,
        }
    }

    /// Convert from raw u8.
    #[must_use]
    pub const fn from_raw(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::And),
            1 => Some(Self::Or),
            2 => Some(Self::Not),
            3 => Some(Self::Xor),
            4 => Some(Self::Nand),
            5 => Some(Self::Nor),
            6 => Some(Self::Xnor),
            _ => None,
        }
    }
}

/// Configuration for a logic gate device.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GateConfig {
    /// Gate operation.
    pub op: GateOp,
    /// Invert output (additional NOT).
    pub invert_output: bool,
}

impl GateConfig {
    /// Create an AND gate.
    #[must_use]
    pub const fn and() -> Self {
        Self {
            op: GateOp::And,
            invert_output: false,
        }
    }

    /// Create an OR gate.
    #[must_use]
    pub const fn or() -> Self {
        Self {
            op: GateOp::Or,
            invert_output: false,
        }
    }

    /// Create a NOT gate.
    #[must_use]
    pub const fn not() -> Self {
        Self {
            op: GateOp::Not,
            invert_output: false,
        }
    }

    /// Create a XOR gate.
    #[must_use]
    pub const fn xor() -> Self {
        Self {
            op: GateOp::Xor,
            invert_output: false,
        }
    }

    /// Create a NAND gate.
    #[must_use]
    pub const fn nand() -> Self {
        Self {
            op: GateOp::Nand,
            invert_output: false,
        }
    }

    /// Create a NOR gate.
    #[must_use]
    pub const fn nor() -> Self {
        Self {
            op: GateOp::Nor,
            invert_output: false,
        }
    }

    /// Create a XNOR gate.
    #[must_use]
    pub const fn xnor() -> Self {
        Self {
            op: GateOp::Xnor,
            invert_output: false,
        }
    }

    /// Evaluate gate output.
    #[must_use]
    pub fn evaluate(self, a: SignalValue, b: SignalValue) -> SignalValue {
        let result = self.op.evaluate_signals(a, b);
        if self.invert_output {
            SignalValue::Boolean(!result.is_truthy())
        } else {
            result
        }
    }

    /// Compute fingerprint for state comparison.
    #[must_use]
    pub const fn fingerprint(self) -> u32 {
        let mut h: u32 = 0x9e37_79b9;
        h = h.wrapping_add(self.op as u32);
        h = h.wrapping_mul(0x85eb_ca6b);
        h = h.wrapping_add(self.invert_output as u32);
        h
    }
}

// -----------------------------------------------------------------------------
// Timer Primitives
// -----------------------------------------------------------------------------

/// Timer operating mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum TimerMode {
    /// Periodic pulse: outputs true every N ticks.
    #[default]
    Periodic = 0,
    /// One-shot delay: outputs true once after N ticks, then stops.
    OneShot = 1,
    /// Retriggerable delay: resets on input, outputs after N ticks of no input.
    Retriggerable = 2,
    /// Pulse width: outputs true for N ticks after input goes high.
    PulseWidth = 3,
}

impl TimerMode {
    /// Get display name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Periodic => "periodic",
            Self::OneShot => "one-shot",
            Self::Retriggerable => "retriggerable",
            Self::PulseWidth => "pulse-width",
        }
    }
}

/// Configuration for a timer device.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TimerConfig {
    /// Timer mode.
    pub mode: TimerMode,
    /// Interval in ticks.
    pub interval: u32,
    /// Duty cycle for periodic mode (0-255 maps to 0-100%).
    pub duty_cycle: u8,
    /// Initial delay before first pulse.
    pub initial_delay: u32,
}

impl Default for TimerConfig {
    fn default() -> Self {
        Self {
            mode: TimerMode::Periodic,
            interval: 20,
            duty_cycle: 128,
            initial_delay: 0,
        }
    }
}

impl TimerConfig {
    /// Create a periodic timer.
    #[must_use]
    pub const fn periodic(interval: u32) -> Self {
        Self {
            mode: TimerMode::Periodic,
            interval,
            duty_cycle: 128,
            initial_delay: 0,
        }
    }

    /// Create a one-shot timer.
    #[must_use]
    pub const fn one_shot(delay: u32) -> Self {
        Self {
            mode: TimerMode::OneShot,
            interval: delay,
            duty_cycle: 255,
            initial_delay: 0,
        }
    }

    /// Create a retriggerable timer.
    #[must_use]
    pub const fn retriggerable(timeout: u32) -> Self {
        Self {
            mode: TimerMode::Retriggerable,
            interval: timeout,
            duty_cycle: 255,
            initial_delay: 0,
        }
    }

    /// Create a pulse width timer.
    #[must_use]
    pub const fn pulse_width(width: u32) -> Self {
        Self {
            mode: TimerMode::PulseWidth,
            interval: width,
            duty_cycle: 255,
            initial_delay: 0,
        }
    }

    /// Set duty cycle (0-100).
    #[must_use]
    #[expect(clippy::cast_possible_truncation)]
    pub const fn with_duty(mut self, percent: u8) -> Self {
        self.duty_cycle = if percent > 100 {
            255
        } else {
            (percent as u16 * 255 / 100) as u8
        };
        self
    }

    /// Set initial delay.
    #[must_use]
    pub const fn with_initial_delay(mut self, delay: u32) -> Self {
        self.initial_delay = delay;
        self
    }

    /// Compute fingerprint for state comparison.
    #[must_use]
    pub const fn fingerprint(self) -> u32 {
        let mut h: u32 = 0x9e37_79b9;
        h = h.wrapping_add(self.mode as u32);
        h = h.wrapping_mul(0x85eb_ca6b);
        h = h.wrapping_add(self.interval);
        h = h.wrapping_mul(0x85eb_ca6b);
        h = h.wrapping_add(self.duty_cycle as u32);
        h = h.wrapping_mul(0x85eb_ca6b);
        h = h.wrapping_add(self.initial_delay);
        h
    }
}

/// Runtime state for a timer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TimerState {
    /// Current counter value.
    pub counter: u32,
    /// Whether the timer has fired (for one-shot).
    pub fired: bool,
    /// Whether the timer is currently active.
    pub active: bool,
    /// Last input state (for edge detection).
    pub last_input: bool,
}

impl TimerState {
    /// Reset timer state.
    pub fn reset(&mut self) {
        self.counter = 0;
        self.fired = false;
        self.active = false;
        self.last_input = false;
    }

    /// Tick the timer and return the output signal.
    #[must_use]
    pub fn tick(&mut self, config: &TimerConfig, input: SignalValue) -> SignalValue {
        let input_high = input.is_truthy();
        let rising_edge = input_high && !self.last_input;
        self.last_input = input_high;

        match config.mode {
            TimerMode::Periodic => self.tick_periodic(config),
            TimerMode::OneShot => self.tick_one_shot(config, rising_edge),
            TimerMode::Retriggerable => self.tick_retriggerable(config, input_high),
            TimerMode::PulseWidth => self.tick_pulse_width(config, rising_edge),
        }
    }

    fn tick_periodic(&mut self, config: &TimerConfig) -> SignalValue {
        if config.interval == 0 {
            return SignalValue::Boolean(false);
        }

        if self.counter < config.initial_delay {
            self.counter += 1;
            return SignalValue::Boolean(false);
        }

        let cycle_pos = (self.counter - config.initial_delay) % config.interval;
        self.counter += 1;

        let threshold = u32::from(config.duty_cycle) * config.interval / 255;
        SignalValue::Boolean(cycle_pos < threshold)
    }

    fn tick_one_shot(&mut self, config: &TimerConfig, trigger: bool) -> SignalValue {
        if trigger && !self.fired {
            self.active = true;
            self.counter = 0;
        }

        if self.active {
            self.counter += 1;
            if self.counter >= config.interval {
                self.fired = true;
                self.active = false;
                return SignalValue::Boolean(true);
            }
        }

        SignalValue::Boolean(false)
    }

    fn tick_retriggerable(&mut self, config: &TimerConfig, input_high: bool) -> SignalValue {
        if input_high {
            self.counter = 0;
            self.active = false;
            return SignalValue::Boolean(false);
        }

        self.counter += 1;
        if self.counter >= config.interval {
            SignalValue::Boolean(true)
        } else {
            SignalValue::Boolean(false)
        }
    }

    fn tick_pulse_width(&mut self, config: &TimerConfig, trigger: bool) -> SignalValue {
        if trigger {
            self.active = true;
            self.counter = 0;
        }

        if self.active {
            self.counter += 1;
            if self.counter > config.interval {
                self.active = false;
                return SignalValue::Boolean(false);
            }
            return SignalValue::Boolean(true);
        }

        SignalValue::Boolean(false)
    }

    /// Compute fingerprint for state comparison.
    #[must_use]
    pub const fn fingerprint(self) -> u32 {
        let mut h: u32 = 0x9e37_79b9;
        h = h.wrapping_add(self.counter);
        h = h.wrapping_mul(0x85eb_ca6b);
        h = h.wrapping_add(self.fired as u32);
        h = h.wrapping_mul(0x85eb_ca6b);
        h = h.wrapping_add(self.active as u32);
        h = h.wrapping_mul(0x85eb_ca6b);
        h = h.wrapping_add(self.last_input as u32);
        h
    }
}

// -----------------------------------------------------------------------------
// Sensor Primitives
// -----------------------------------------------------------------------------

/// Sensor measurement type.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum SensorType {
    /// Pressure sensor (outputs float).
    #[default]
    Pressure = 0,
    /// Temperature sensor (outputs float).
    Temperature = 1,
    /// Level/fill sensor (outputs float 0-1).
    Level = 2,
    /// Flow rate sensor (outputs float).
    FlowRate = 3,
    /// Binary proximity/presence sensor (outputs bool).
    Proximity = 4,
    /// Analog value sensor (outputs float).
    Analog = 5,
    /// Digital counter sensor (outputs int).
    Counter = 6,
}

impl SensorType {
    /// Get display name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Pressure => "pressure",
            Self::Temperature => "temperature",
            Self::Level => "level",
            Self::FlowRate => "flow-rate",
            Self::Proximity => "proximity",
            Self::Analog => "analog",
            Self::Counter => "counter",
        }
    }

    /// Check if this sensor outputs boolean values.
    #[must_use]
    pub const fn is_boolean(self) -> bool {
        matches!(self, Self::Proximity)
    }

    /// Check if this sensor outputs integer values.
    #[must_use]
    pub const fn is_integer(self) -> bool {
        matches!(self, Self::Counter)
    }
}

/// Configuration for a sensor device.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SensorConfig {
    /// Sensor type.
    pub sensor_type: SensorType,
    /// Minimum value for scaling.
    pub min_value: f32,
    /// Maximum value for scaling.
    pub max_value: f32,
    /// Threshold for comparison output.
    pub threshold: f32,
    /// Hysteresis for threshold (prevents oscillation).
    pub hysteresis: f32,
    /// Sample rate divisor (1 = every tick, 2 = every other tick).
    pub sample_divisor: u8,
}

impl Default for SensorConfig {
    fn default() -> Self {
        Self {
            sensor_type: SensorType::Analog,
            min_value: 0.0,
            max_value: 1.0,
            threshold: 0.5,
            hysteresis: 0.05,
            sample_divisor: 1,
        }
    }
}

impl SensorConfig {
    /// Create a pressure sensor.
    #[must_use]
    pub fn pressure(min: f32, max: f32) -> Self {
        Self {
            sensor_type: SensorType::Pressure,
            min_value: min,
            max_value: max,
            ..Self::default()
        }
    }

    /// Create a temperature sensor.
    #[must_use]
    pub fn temperature(min: f32, max: f32) -> Self {
        Self {
            sensor_type: SensorType::Temperature,
            min_value: min,
            max_value: max,
            ..Self::default()
        }
    }

    /// Create a level sensor.
    #[must_use]
    pub fn level() -> Self {
        Self {
            sensor_type: SensorType::Level,
            min_value: 0.0,
            max_value: 1.0,
            ..Self::default()
        }
    }

    /// Create a flow rate sensor.
    #[must_use]
    pub fn flow_rate(max: f32) -> Self {
        Self {
            sensor_type: SensorType::FlowRate,
            min_value: 0.0,
            max_value: max,
            ..Self::default()
        }
    }

    /// Create a proximity sensor.
    #[must_use]
    pub fn proximity() -> Self {
        Self {
            sensor_type: SensorType::Proximity,
            min_value: 0.0,
            max_value: 1.0,
            threshold: 0.5,
            ..Self::default()
        }
    }

    /// Create a counter sensor.
    #[must_use]
    pub fn counter() -> Self {
        Self {
            sensor_type: SensorType::Counter,
            min_value: 0.0,
            max_value: f32::MAX,
            ..Self::default()
        }
    }

    /// Set threshold with hysteresis.
    #[must_use]
    pub fn with_threshold(mut self, threshold: f32, hysteresis: f32) -> Self {
        self.threshold = threshold;
        self.hysteresis = hysteresis;
        self
    }

    /// Normalize a raw value to the sensor's range.
    #[must_use]
    pub fn normalize(&self, raw: f32) -> f32 {
        if (self.max_value - self.min_value).abs() < f32::EPSILON {
            return 0.0;
        }
        ((raw - self.min_value) / (self.max_value - self.min_value)).clamp(0.0, 1.0)
    }

    /// Compute fingerprint for state comparison.
    #[must_use]
    pub fn fingerprint(&self) -> u32 {
        let mut h: u32 = 0x9e37_79b9;
        h = h.wrapping_add(self.sensor_type as u32);
        h = h.wrapping_mul(0x85eb_ca6b);
        h = h.wrapping_add(self.min_value.to_bits());
        h = h.wrapping_mul(0x85eb_ca6b);
        h = h.wrapping_add(self.max_value.to_bits());
        h = h.wrapping_mul(0x85eb_ca6b);
        h = h.wrapping_add(self.threshold.to_bits());
        h
    }
}

/// Runtime state for a sensor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SensorState {
    /// Current raw reading.
    pub raw_value: f32,
    /// Last threshold comparison result (for hysteresis).
    pub above_threshold: bool,
    /// Sample counter for rate limiting.
    pub sample_counter: u8,
    /// Accumulated count for counter sensors.
    pub count: i32,
}

impl SensorState {
    /// Reset sensor state.
    pub fn reset(&mut self) {
        self.raw_value = 0.0;
        self.above_threshold = false;
        self.sample_counter = 0;
        self.count = 0;
    }

    /// Update the sensor with a new raw reading.
    #[must_use]
    pub fn update(&mut self, config: &SensorConfig, raw: f32) -> SignalValue {
        self.sample_counter = self.sample_counter.wrapping_add(1);
        if config.sample_divisor > 1 && !self.sample_counter.is_multiple_of(config.sample_divisor) {
            return self.current_output(config);
        }

        self.raw_value = raw;

        match config.sensor_type {
            SensorType::Proximity => self.update_threshold(config, raw),
            SensorType::Counter => {
                if raw > config.threshold && !self.above_threshold {
                    self.count = self.count.saturating_add(1);
                }
                self.above_threshold = raw > config.threshold;
                SignalValue::Integer(self.count)
            }
            _ => SignalValue::Float(config.normalize(raw)),
        }
    }

    fn update_threshold(&mut self, config: &SensorConfig, raw: f32) -> SignalValue {
        let normalized = config.normalize(raw);
        let threshold_high = config.threshold + config.hysteresis;
        let threshold_low = config.threshold - config.hysteresis;

        if self.above_threshold {
            if normalized < threshold_low {
                self.above_threshold = false;
            }
        } else if normalized > threshold_high {
            self.above_threshold = true;
        }

        SignalValue::Boolean(self.above_threshold)
    }

    /// Get current output without updating.
    #[must_use]
    pub fn current_output(&self, config: &SensorConfig) -> SignalValue {
        match config.sensor_type {
            SensorType::Proximity => SignalValue::Boolean(self.above_threshold),
            SensorType::Counter => SignalValue::Integer(self.count),
            _ => SignalValue::Float(config.normalize(self.raw_value)),
        }
    }

    /// Compute fingerprint for state comparison.
    #[must_use]
    pub fn fingerprint(&self) -> u32 {
        let mut h: u32 = 0x9e37_79b9;
        h = h.wrapping_add(self.raw_value.to_bits());
        h = h.wrapping_mul(0x85eb_ca6b);
        h = h.wrapping_add(u32::from(self.above_threshold));
        h = h.wrapping_mul(0x85eb_ca6b);
        h = h.wrapping_add(self.count.cast_unsigned());
        h
    }
}

// -----------------------------------------------------------------------------
// Relay Primitives
// -----------------------------------------------------------------------------

/// Relay operating mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum RelayMode {
    /// Pass-through: output equals input.
    #[default]
    PassThrough = 0,
    /// Normally open: output is off unless control is high.
    NormallyOpen = 1,
    /// Normally closed: output is on unless control is low.
    NormallyClosed = 2,
    /// Latching: control toggles output state.
    Latching = 3,
}

impl RelayMode {
    /// Get display name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::PassThrough => "pass-through",
            Self::NormallyOpen => "normally-open",
            Self::NormallyClosed => "normally-closed",
            Self::Latching => "latching",
        }
    }
}

/// Configuration for a relay device.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RelayConfig {
    /// Relay mode.
    pub mode: RelayMode,
    /// Propagation delay in ticks.
    pub delay: u8,
    /// Whether to invert the control signal.
    pub invert_control: bool,
}

impl RelayConfig {
    /// Create a pass-through relay.
    #[must_use]
    pub const fn pass_through() -> Self {
        Self {
            mode: RelayMode::PassThrough,
            delay: 0,
            invert_control: false,
        }
    }

    /// Create a normally-open relay.
    #[must_use]
    pub const fn normally_open() -> Self {
        Self {
            mode: RelayMode::NormallyOpen,
            delay: 0,
            invert_control: false,
        }
    }

    /// Create a normally-closed relay.
    #[must_use]
    pub const fn normally_closed() -> Self {
        Self {
            mode: RelayMode::NormallyClosed,
            delay: 0,
            invert_control: false,
        }
    }

    /// Create a latching relay.
    #[must_use]
    pub const fn latching() -> Self {
        Self {
            mode: RelayMode::Latching,
            delay: 0,
            invert_control: false,
        }
    }

    /// Set propagation delay.
    #[must_use]
    pub const fn with_delay(mut self, delay: u8) -> Self {
        self.delay = delay;
        self
    }

    /// Compute fingerprint for state comparison.
    #[must_use]
    pub const fn fingerprint(self) -> u32 {
        let mut h: u32 = 0x9e37_79b9;
        h = h.wrapping_add(self.mode as u32);
        h = h.wrapping_mul(0x85eb_ca6b);
        h = h.wrapping_add(self.delay as u32);
        h = h.wrapping_mul(0x85eb_ca6b);
        h = h.wrapping_add(self.invert_control as u32);
        h
    }
}

/// Runtime state for a relay.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RelayState {
    /// Current output state.
    pub output: SignalValue,
    /// Latched state for latching relays.
    pub latched: bool,
    /// Last control input (for edge detection).
    pub last_control: bool,
    /// Delay buffer for delayed relays.
    pub delay_counter: u8,
    /// Pending output value.
    pub pending_output: SignalValue,
}

impl RelayState {
    /// Reset relay state.
    pub fn reset(&mut self) {
        self.output = SignalValue::None;
        self.latched = false;
        self.last_control = false;
        self.delay_counter = 0;
        self.pending_output = SignalValue::None;
    }

    /// Tick the relay and return the output signal.
    #[must_use]
    pub fn tick(
        &mut self,
        config: &RelayConfig,
        input: SignalValue,
        control: SignalValue,
    ) -> SignalValue {
        let ctrl = control.is_truthy() ^ config.invert_control;
        let rising_edge = ctrl && !self.last_control;
        self.last_control = ctrl;

        let new_output = match config.mode {
            RelayMode::PassThrough => input,
            RelayMode::NormallyOpen => {
                if ctrl {
                    input
                } else {
                    SignalValue::None
                }
            }
            RelayMode::NormallyClosed => {
                if ctrl {
                    SignalValue::None
                } else {
                    input
                }
            }
            RelayMode::Latching => {
                if rising_edge {
                    self.latched = !self.latched;
                }
                if self.latched {
                    input
                } else {
                    SignalValue::None
                }
            }
        };

        if config.delay == 0 {
            self.output = new_output;
        } else {
            if self.delay_counter == 0 {
                self.pending_output = new_output;
                self.delay_counter = config.delay;
            }
            self.delay_counter = self.delay_counter.saturating_sub(1);
            if self.delay_counter == 0 {
                self.output = self.pending_output;
            }
        }

        self.output
    }

    /// Compute fingerprint for state comparison.
    #[must_use]
    pub fn fingerprint(&self) -> u32 {
        let mut h: u32 = 0x9e37_79b9;
        h = h.wrapping_add(u32::from(self.latched));
        h = h.wrapping_mul(0x85eb_ca6b);
        h = h.wrapping_add(u32::from(self.last_control));
        h = h.wrapping_mul(0x85eb_ca6b);
        h = h.wrapping_add(u32::from(self.delay_counter));
        h
    }
}

// -----------------------------------------------------------------------------
// Valve Primitives
// -----------------------------------------------------------------------------

/// Valve operating mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum ValveMode {
    /// Binary on/off valve.
    #[default]
    Binary = 0,
    /// Proportional valve (0-100% flow).
    Proportional = 1,
    /// Three-way diverter valve.
    Diverter = 2,
    /// Check valve (one-way flow).
    Check = 3,
}

impl ValveMode {
    /// Get display name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Binary => "binary",
            Self::Proportional => "proportional",
            Self::Diverter => "diverter",
            Self::Check => "check",
        }
    }
}

/// Configuration for a valve device.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ValveConfig {
    /// Valve mode.
    pub mode: ValveMode,
    /// Maximum flow rate (units per tick).
    pub max_flow: f32,
    /// Actuation time in ticks (time to fully open/close).
    pub actuation_time: u8,
    /// Default open percentage (0-255).
    pub default_position: u8,
    /// Fail-safe position when control is lost (0-255).
    pub fail_safe_position: u8,
}

impl Default for ValveConfig {
    fn default() -> Self {
        Self {
            mode: ValveMode::Binary,
            max_flow: 1.0,
            actuation_time: 1,
            default_position: 0,
            fail_safe_position: 0,
        }
    }
}

impl ValveConfig {
    /// Create a binary valve.
    #[must_use]
    pub fn binary(max_flow: f32) -> Self {
        Self {
            mode: ValveMode::Binary,
            max_flow,
            ..Self::default()
        }
    }

    /// Create a proportional valve.
    #[must_use]
    pub fn proportional(max_flow: f32) -> Self {
        Self {
            mode: ValveMode::Proportional,
            max_flow,
            ..Self::default()
        }
    }

    /// Create a diverter valve.
    #[must_use]
    pub fn diverter(max_flow: f32) -> Self {
        Self {
            mode: ValveMode::Diverter,
            max_flow,
            ..Self::default()
        }
    }

    /// Create a check valve.
    #[must_use]
    pub fn check(max_flow: f32) -> Self {
        Self {
            mode: ValveMode::Check,
            max_flow,
            default_position: 255,
            ..Self::default()
        }
    }

    /// Set actuation time.
    #[must_use]
    pub const fn with_actuation_time(mut self, ticks: u8) -> Self {
        self.actuation_time = ticks;
        self
    }

    /// Set fail-safe position.
    #[must_use]
    pub const fn with_fail_safe(mut self, position: u8) -> Self {
        self.fail_safe_position = position;
        self
    }

    /// Compute fingerprint for state comparison.
    #[must_use]
    pub fn fingerprint(&self) -> u32 {
        let mut h: u32 = 0x9e37_79b9;
        h = h.wrapping_add(self.mode as u32);
        h = h.wrapping_mul(0x85eb_ca6b);
        h = h.wrapping_add(self.max_flow.to_bits());
        h = h.wrapping_mul(0x85eb_ca6b);
        h = h.wrapping_add(u32::from(self.actuation_time));
        h
    }
}

/// Runtime state for a valve.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ValveState {
    /// Current position (0-255, 0=closed, 255=fully open).
    pub position: u8,
    /// Target position.
    pub target_position: u8,
    /// Current flow rate.
    pub current_flow: f32,
    /// Diverter output selection (for diverter valves).
    pub diverter_output: u8,
}

impl ValveState {
    /// Reset valve state.
    pub fn reset(&mut self, config: &ValveConfig) {
        self.position = config.default_position;
        self.target_position = config.default_position;
        self.current_flow = 0.0;
        self.diverter_output = 0;
    }

    /// Tick the valve and return the output signal (flow rate).
    #[must_use]
    pub fn tick(
        &mut self,
        config: &ValveConfig,
        control: SignalValue,
        input_pressure: f32,
    ) -> SignalValue {
        self.update_target(*config, control);

        if config.actuation_time > 1 {
            let step = 255_u16.div_ceil(u16::from(config.actuation_time));
            let step_u8 = step.min(255) as u8;
            if self.position < self.target_position {
                self.position = self.position.saturating_add(step_u8);
                if self.position > self.target_position {
                    self.position = self.target_position;
                }
            } else if self.position > self.target_position {
                self.position = self.position.saturating_sub(step_u8);
                if self.position < self.target_position {
                    self.position = self.target_position;
                }
            }
        } else {
            self.position = self.target_position;
        }

        self.current_flow = self.compute_flow(*config, input_pressure);

        match config.mode {
            ValveMode::Diverter => SignalValue::Integer(i32::from(self.diverter_output)),
            _ => SignalValue::Float(self.current_flow),
        }
    }

    fn update_target(&mut self, config: ValveConfig, control: SignalValue) {
        match config.mode {
            ValveMode::Binary => {
                self.target_position = if control.is_truthy() { 255 } else { 0 };
            }
            ValveMode::Proportional => {
                let ratio = control.to_float().clamp(0.0, 1.0);
                #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let pos = (ratio * 255.0) as u8;
                self.target_position = pos;
            }
            ValveMode::Diverter => {
                self.target_position = 255;
                #[expect(clippy::cast_sign_loss)]
                let output = control.to_int().clamp(0, 2) as u8;
                self.diverter_output = output;
            }
            ValveMode::Check => {
                self.target_position = 255;
            }
        }
    }

    fn compute_flow(self, config: ValveConfig, input_pressure: f32) -> f32 {
        if config.mode == ValveMode::Check && input_pressure < 0.0 {
            return 0.0;
        }

        let opening = f32::from(self.position) / 255.0;
        opening * config.max_flow * input_pressure.abs().min(1.0)
    }

    /// Check if valve is fully open.
    #[must_use]
    pub const fn is_open(&self) -> bool {
        self.position == 255
    }

    /// Check if valve is fully closed.
    #[must_use]
    pub const fn is_closed(&self) -> bool {
        self.position == 0
    }

    /// Get position as percentage (0.0-1.0).
    #[must_use]
    pub fn position_ratio(&self) -> f32 {
        f32::from(self.position) / 255.0
    }

    /// Compute fingerprint for state comparison.
    #[must_use]
    pub fn fingerprint(&self) -> u32 {
        let mut h: u32 = 0x9e37_79b9;
        h = h.wrapping_add(u32::from(self.position));
        h = h.wrapping_mul(0x85eb_ca6b);
        h = h.wrapping_add(u32::from(self.target_position));
        h = h.wrapping_mul(0x85eb_ca6b);
        h = h.wrapping_add(self.current_flow.to_bits());
        h
    }
}

// -----------------------------------------------------------------------------
// Pump Primitives
// -----------------------------------------------------------------------------

/// Pump operating mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum PumpMode {
    /// On/off pump.
    #[default]
    Binary = 0,
    /// Variable speed pump.
    Variable = 1,
    /// Positive displacement pump (fixed flow per cycle).
    Displacement = 2,
}

impl PumpMode {
    /// Get display name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Binary => "binary",
            Self::Variable => "variable",
            Self::Displacement => "displacement",
        }
    }
}

/// Configuration for a pump device.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PumpConfig {
    /// Pump mode.
    pub mode: PumpMode,
    /// Maximum flow rate (units per tick).
    pub max_flow: f32,
    /// Maximum pressure head.
    pub max_pressure: f32,
    /// Power consumption at full speed.
    pub power_consumption: f32,
    /// Ramp-up time in ticks.
    pub ramp_time: u8,
    /// Minimum speed percentage (0-255).
    pub min_speed: u8,
}

impl Default for PumpConfig {
    fn default() -> Self {
        Self {
            mode: PumpMode::Binary,
            max_flow: 1.0,
            max_pressure: 1.0,
            power_consumption: 1.0,
            ramp_time: 1,
            min_speed: 0,
        }
    }
}

impl PumpConfig {
    /// Create a binary pump.
    #[must_use]
    pub fn binary(max_flow: f32) -> Self {
        Self {
            mode: PumpMode::Binary,
            max_flow,
            ..Self::default()
        }
    }

    /// Create a variable speed pump.
    #[must_use]
    pub fn variable(max_flow: f32) -> Self {
        Self {
            mode: PumpMode::Variable,
            max_flow,
            ..Self::default()
        }
    }

    /// Create a displacement pump.
    #[must_use]
    pub fn displacement(flow_per_cycle: f32) -> Self {
        Self {
            mode: PumpMode::Displacement,
            max_flow: flow_per_cycle,
            ..Self::default()
        }
    }

    /// Set ramp time.
    #[must_use]
    pub const fn with_ramp_time(mut self, ticks: u8) -> Self {
        self.ramp_time = ticks;
        self
    }

    /// Set power consumption.
    #[must_use]
    pub fn with_power(mut self, power: f32) -> Self {
        self.power_consumption = power;
        self
    }

    /// Compute fingerprint for state comparison.
    #[must_use]
    pub fn fingerprint(&self) -> u32 {
        let mut h: u32 = 0x9e37_79b9;
        h = h.wrapping_add(self.mode as u32);
        h = h.wrapping_mul(0x85eb_ca6b);
        h = h.wrapping_add(self.max_flow.to_bits());
        h = h.wrapping_mul(0x85eb_ca6b);
        h = h.wrapping_add(self.max_pressure.to_bits());
        h
    }
}

/// Runtime state for a pump.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PumpState {
    /// Current speed (0-255).
    pub speed: u8,
    /// Target speed.
    pub target_speed: u8,
    /// Current flow rate.
    pub current_flow: f32,
    /// Current power draw.
    pub power_draw: f32,
    /// Whether pump is running.
    pub running: bool,
    /// Fault condition.
    pub fault: bool,
}

impl PumpState {
    /// Reset pump state.
    pub fn reset(&mut self) {
        self.speed = 0;
        self.target_speed = 0;
        self.current_flow = 0.0;
        self.power_draw = 0.0;
        self.running = false;
        self.fault = false;
    }

    /// Tick the pump and return the output signal (flow rate).
    #[must_use]
    pub fn tick(
        &mut self,
        config: &PumpConfig,
        control: SignalValue,
        back_pressure: f32,
    ) -> SignalValue {
        if self.fault {
            self.running = false;
            self.speed = 0;
            self.current_flow = 0.0;
            self.power_draw = 0.0;
            return SignalValue::Float(0.0);
        }

        self.update_target(config, control);

        if config.ramp_time > 1 {
            let step = 255_u16.div_ceil(u16::from(config.ramp_time));
            let step_u8 = step.min(255) as u8;
            if self.speed < self.target_speed {
                self.speed = self.speed.saturating_add(step_u8);
                if self.speed > self.target_speed {
                    self.speed = self.target_speed;
                }
            } else if self.speed > self.target_speed {
                self.speed = self.speed.saturating_sub(step_u8);
                if self.speed < self.target_speed {
                    self.speed = self.target_speed;
                }
            }
        } else {
            self.speed = self.target_speed;
        }

        self.running = self.speed > 0;
        self.current_flow = self.compute_flow(config, back_pressure);
        self.power_draw = self.compute_power(config);

        SignalValue::Float(self.current_flow)
    }

    fn update_target(&mut self, config: &PumpConfig, control: SignalValue) {
        match config.mode {
            PumpMode::Binary => {
                self.target_speed = if control.is_truthy() { 255 } else { 0 };
            }
            PumpMode::Variable | PumpMode::Displacement => {
                let ratio = control.to_float().clamp(0.0, 1.0);
                let min = f32::from(config.min_speed) / 255.0;
                if ratio > 0.0 {
                    let scaled = min + (1.0 - min) * ratio;
                    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    let speed = (scaled * 255.0) as u8;
                    self.target_speed = speed;
                } else {
                    self.target_speed = 0;
                }
            }
        }
    }

    fn compute_flow(&self, config: &PumpConfig, back_pressure: f32) -> f32 {
        if self.speed == 0 {
            return 0.0;
        }

        let speed_ratio = f32::from(self.speed) / 255.0;

        if config.mode == PumpMode::Displacement {
            speed_ratio * config.max_flow
        } else {
            let pressure_factor = (1.0 - back_pressure / config.max_pressure).max(0.0);
            speed_ratio * config.max_flow * pressure_factor
        }
    }

    fn compute_power(&self, config: &PumpConfig) -> f32 {
        let speed_ratio = f32::from(self.speed) / 255.0;
        speed_ratio * speed_ratio * config.power_consumption
    }

    /// Check if pump is at full speed.
    #[must_use]
    pub const fn is_at_full_speed(&self) -> bool {
        self.speed == 255
    }

    /// Get speed as percentage (0.0-1.0).
    #[must_use]
    pub fn speed_ratio(&self) -> f32 {
        f32::from(self.speed) / 255.0
    }

    /// Set fault condition.
    pub fn set_fault(&mut self, fault: bool) {
        self.fault = fault;
    }

    /// Compute fingerprint for state comparison.
    #[must_use]
    pub fn fingerprint(&self) -> u32 {
        let mut h: u32 = 0x9e37_79b9;
        h = h.wrapping_add(u32::from(self.speed));
        h = h.wrapping_mul(0x85eb_ca6b);
        h = h.wrapping_add(u32::from(self.running));
        h = h.wrapping_mul(0x85eb_ca6b);
        h = h.wrapping_add(u32::from(self.fault));
        h = h.wrapping_mul(0x85eb_ca6b);
        h = h.wrapping_add(self.current_flow.to_bits());
        h
    }
}

// -----------------------------------------------------------------------------
// Primitive Container
// -----------------------------------------------------------------------------

/// Combined primitive configuration for storage in `DeviceConfig`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum PrimitiveConfig {
    /// No specialized primitive.
    #[default]
    None,
    /// Logic gate.
    Gate(GateConfig),
    /// Timer.
    Timer(TimerConfig),
    /// Sensor.
    Sensor(SensorConfig),
    /// Relay.
    Relay(RelayConfig),
    /// Valve.
    Valve(ValveConfig),
    /// Pump.
    Pump(PumpConfig),
}

impl PrimitiveConfig {
    /// Compute fingerprint for state comparison.
    #[must_use]
    pub fn fingerprint(&self) -> u32 {
        match self {
            Self::None => 0,
            Self::Gate(c) => c.fingerprint(),
            Self::Timer(c) => c.fingerprint(),
            Self::Sensor(c) => c.fingerprint(),
            Self::Relay(c) => c.fingerprint(),
            Self::Valve(c) => c.fingerprint(),
            Self::Pump(c) => c.fingerprint(),
        }
    }

    /// Get type name.
    #[must_use]
    pub const fn type_name(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Gate(_) => "gate",
            Self::Timer(_) => "timer",
            Self::Sensor(_) => "sensor",
            Self::Relay(_) => "relay",
            Self::Valve(_) => "valve",
            Self::Pump(_) => "pump",
        }
    }
}

/// Combined primitive state for runtime storage.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum PrimitiveState {
    /// No state.
    #[default]
    None,
    /// Timer state.
    Timer(TimerState),
    /// Sensor state.
    Sensor(SensorState),
    /// Relay state.
    Relay(RelayState),
    /// Valve state.
    Valve(ValveState),
    /// Pump state.
    Pump(PumpState),
}

impl PrimitiveState {
    /// Reset to default state.
    pub fn reset(&mut self) {
        match self {
            Self::None => {}
            Self::Timer(s) => s.reset(),
            Self::Sensor(s) => s.reset(),
            Self::Relay(s) => s.reset(),
            Self::Valve(s) => s.reset(&ValveConfig::default()),
            Self::Pump(s) => s.reset(),
        }
    }

    /// Compute fingerprint for state comparison.
    #[must_use]
    pub fn fingerprint(&self) -> u32 {
        match self {
            Self::None => 0,
            Self::Timer(s) => s.fingerprint(),
            Self::Sensor(s) => s.fingerprint(),
            Self::Relay(s) => s.fingerprint(),
            Self::Valve(s) => s.fingerprint(),
            Self::Pump(s) => s.fingerprint(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // Gate Tests
    // -------------------------------------------------------------------------

    #[test]
    fn gate_and_evaluation() {
        let gate = GateConfig::and();
        assert_eq!(
            gate.evaluate(SignalValue::ON, SignalValue::ON),
            SignalValue::Boolean(true)
        );
        assert_eq!(
            gate.evaluate(SignalValue::ON, SignalValue::OFF),
            SignalValue::Boolean(false)
        );
        assert_eq!(
            gate.evaluate(SignalValue::OFF, SignalValue::ON),
            SignalValue::Boolean(false)
        );
        assert_eq!(
            gate.evaluate(SignalValue::OFF, SignalValue::OFF),
            SignalValue::Boolean(false)
        );
    }

    #[test]
    fn gate_or_evaluation() {
        let gate = GateConfig::or();
        assert_eq!(
            gate.evaluate(SignalValue::ON, SignalValue::ON),
            SignalValue::Boolean(true)
        );
        assert_eq!(
            gate.evaluate(SignalValue::ON, SignalValue::OFF),
            SignalValue::Boolean(true)
        );
        assert_eq!(
            gate.evaluate(SignalValue::OFF, SignalValue::ON),
            SignalValue::Boolean(true)
        );
        assert_eq!(
            gate.evaluate(SignalValue::OFF, SignalValue::OFF),
            SignalValue::Boolean(false)
        );
    }

    #[test]
    fn gate_not_evaluation() {
        let gate = GateConfig::not();
        assert_eq!(
            gate.evaluate(SignalValue::ON, SignalValue::None),
            SignalValue::Boolean(false)
        );
        assert_eq!(
            gate.evaluate(SignalValue::OFF, SignalValue::None),
            SignalValue::Boolean(true)
        );
    }

    #[test]
    fn gate_xor_evaluation() {
        let gate = GateConfig::xor();
        assert_eq!(
            gate.evaluate(SignalValue::ON, SignalValue::ON),
            SignalValue::Boolean(false)
        );
        assert_eq!(
            gate.evaluate(SignalValue::ON, SignalValue::OFF),
            SignalValue::Boolean(true)
        );
    }

    #[test]
    fn gate_nand_evaluation() {
        let gate = GateConfig::nand();
        assert_eq!(
            gate.evaluate(SignalValue::ON, SignalValue::ON),
            SignalValue::Boolean(false)
        );
        assert_eq!(
            gate.evaluate(SignalValue::ON, SignalValue::OFF),
            SignalValue::Boolean(true)
        );
    }

    #[test]
    fn gate_invert_output() {
        let mut gate = GateConfig::and();
        gate.invert_output = true;
        assert_eq!(
            gate.evaluate(SignalValue::ON, SignalValue::ON),
            SignalValue::Boolean(false)
        );
    }

    #[test]
    fn gate_fingerprint_deterministic() {
        let g1 = GateConfig::and();
        let g2 = GateConfig::and();
        assert_eq!(g1.fingerprint(), g2.fingerprint());

        let g3 = GateConfig::or();
        assert_ne!(g1.fingerprint(), g3.fingerprint());
    }

    // -------------------------------------------------------------------------
    // Timer Tests
    // -------------------------------------------------------------------------

    #[test]
    fn timer_periodic() {
        let config = TimerConfig::periodic(4);
        let mut state = TimerState::default();

        let outputs: Vec<bool> = (0..8)
            .map(|_| state.tick(&config, SignalValue::None).is_truthy())
            .collect();

        assert_eq!(
            outputs,
            vec![true, true, false, false, true, true, false, false]
        );
    }

    #[test]
    fn timer_one_shot() {
        let config = TimerConfig::one_shot(3);
        let mut state = TimerState::default();

        assert!(!state.tick(&config, SignalValue::OFF).is_truthy());
        assert!(!state.tick(&config, SignalValue::ON).is_truthy());
        assert!(!state.tick(&config, SignalValue::OFF).is_truthy());
        assert!(state.tick(&config, SignalValue::OFF).is_truthy());
        assert!(!state.tick(&config, SignalValue::OFF).is_truthy());
        assert!(!state.tick(&config, SignalValue::ON).is_truthy());
    }

    #[test]
    fn timer_pulse_width() {
        let config = TimerConfig::pulse_width(3);
        let mut state = TimerState::default();

        assert!(!state.tick(&config, SignalValue::OFF).is_truthy());
        assert!(state.tick(&config, SignalValue::ON).is_truthy());
        assert!(state.tick(&config, SignalValue::OFF).is_truthy());
        assert!(state.tick(&config, SignalValue::OFF).is_truthy());
        assert!(!state.tick(&config, SignalValue::OFF).is_truthy());
    }

    #[test]
    fn timer_retriggerable() {
        let config = TimerConfig::retriggerable(3);
        let mut state = TimerState::default();

        assert!(!state.tick(&config, SignalValue::ON).is_truthy());
        assert!(!state.tick(&config, SignalValue::OFF).is_truthy());
        assert!(!state.tick(&config, SignalValue::ON).is_truthy());
        assert!(!state.tick(&config, SignalValue::OFF).is_truthy());
        assert!(!state.tick(&config, SignalValue::OFF).is_truthy());
        assert!(state.tick(&config, SignalValue::OFF).is_truthy());
    }

    #[test]
    fn timer_fingerprint_deterministic() {
        let c1 = TimerConfig::periodic(10);
        let c2 = TimerConfig::periodic(10);
        assert_eq!(c1.fingerprint(), c2.fingerprint());

        let c3 = TimerConfig::periodic(20);
        assert_ne!(c1.fingerprint(), c3.fingerprint());
    }

    // -------------------------------------------------------------------------
    // Sensor Tests
    // -------------------------------------------------------------------------

    #[test]
    fn sensor_analog_normalization() {
        let config = SensorConfig::pressure(0.0, 100.0);
        let mut state = SensorState::default();

        let output = state.update(&config, 50.0);
        assert!((output.to_float() - 0.5).abs() < f32::EPSILON);

        let output = state.update(&config, 0.0);
        assert!((output.to_float() - 0.0).abs() < f32::EPSILON);

        let output = state.update(&config, 100.0);
        assert!((output.to_float() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn sensor_proximity_hysteresis() {
        let config = SensorConfig::proximity().with_threshold(0.5, 0.1);
        let mut state = SensorState::default();

        assert!(!state.update(&config, 0.3).is_truthy());
        assert!(!state.update(&config, 0.55).is_truthy());
        assert!(state.update(&config, 0.65).is_truthy());
        assert!(state.update(&config, 0.45).is_truthy());
        assert!(!state.update(&config, 0.35).is_truthy());
    }

    #[test]
    fn sensor_counter() {
        let config = SensorConfig::counter();
        let mut state = SensorState::default();

        let _ = state.update(&config, 0.0);
        assert_eq!(state.count, 0);

        let _ = state.update(&config, 1.0);
        assert_eq!(state.count, 1);

        let _ = state.update(&config, 1.0);
        assert_eq!(state.count, 1);

        let _ = state.update(&config, 0.0);
        let _ = state.update(&config, 1.0);
        assert_eq!(state.count, 2);
    }

    #[test]
    fn sensor_fingerprint_deterministic() {
        let c1 = SensorConfig::pressure(0.0, 100.0);
        let c2 = SensorConfig::pressure(0.0, 100.0);
        assert_eq!(c1.fingerprint(), c2.fingerprint());
    }

    // -------------------------------------------------------------------------
    // Relay Tests
    // -------------------------------------------------------------------------

    #[test]
    fn relay_pass_through() {
        let config = RelayConfig::pass_through();
        let mut state = RelayState::default();

        let output = state.tick(&config, SignalValue::Integer(42), SignalValue::None);
        assert_eq!(output, SignalValue::Integer(42));
    }

    #[test]
    fn relay_normally_open() {
        let config = RelayConfig::normally_open();
        let mut state = RelayState::default();

        let output = state.tick(&config, SignalValue::Integer(42), SignalValue::OFF);
        assert_eq!(output, SignalValue::None);

        let output = state.tick(&config, SignalValue::Integer(42), SignalValue::ON);
        assert_eq!(output, SignalValue::Integer(42));
    }

    #[test]
    fn relay_normally_closed() {
        let config = RelayConfig::normally_closed();
        let mut state = RelayState::default();

        let output = state.tick(&config, SignalValue::Integer(42), SignalValue::OFF);
        assert_eq!(output, SignalValue::Integer(42));

        let output = state.tick(&config, SignalValue::Integer(42), SignalValue::ON);
        assert_eq!(output, SignalValue::None);
    }

    #[test]
    fn relay_latching() {
        let config = RelayConfig::latching();
        let mut state = RelayState::default();

        let output = state.tick(&config, SignalValue::Integer(42), SignalValue::OFF);
        assert_eq!(output, SignalValue::None);

        let output = state.tick(&config, SignalValue::Integer(42), SignalValue::ON);
        assert_eq!(output, SignalValue::Integer(42));

        let output = state.tick(&config, SignalValue::Integer(42), SignalValue::OFF);
        assert_eq!(output, SignalValue::Integer(42));

        let output = state.tick(&config, SignalValue::Integer(42), SignalValue::ON);
        assert_eq!(output, SignalValue::None);
    }

    #[test]
    fn relay_fingerprint_deterministic() {
        let c1 = RelayConfig::normally_open();
        let c2 = RelayConfig::normally_open();
        assert_eq!(c1.fingerprint(), c2.fingerprint());
    }

    // -------------------------------------------------------------------------
    // Valve Tests
    // -------------------------------------------------------------------------

    #[test]
    fn valve_binary() {
        let config = ValveConfig::binary(10.0);
        let mut state = ValveState::default();

        let output = state.tick(&config, SignalValue::OFF, 1.0);
        assert!(state.is_closed());
        assert!((output.to_float() - 0.0).abs() < f32::EPSILON);

        let output = state.tick(&config, SignalValue::ON, 1.0);
        assert!(state.is_open());
        assert!((output.to_float() - 10.0).abs() < f32::EPSILON);
    }

    #[test]
    fn valve_proportional() {
        let config = ValveConfig::proportional(10.0);
        let mut state = ValveState::default();

        let output = state.tick(&config, SignalValue::Float(0.5), 1.0);
        assert!((state.position_ratio() - 0.5).abs() < 0.01);
        assert!((output.to_float() - 5.0).abs() < 0.1);
    }

    #[test]
    fn valve_check() {
        let config = ValveConfig::check(10.0);
        let mut state = ValveState::default();

        let output = state.tick(&config, SignalValue::None, 1.0);
        assert!((output.to_float() - 10.0).abs() < f32::EPSILON);

        let output = state.tick(&config, SignalValue::None, -1.0);
        assert!((output.to_float() - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn valve_actuation_time() {
        let config = ValveConfig::binary(10.0).with_actuation_time(4);
        let mut state = ValveState::default();

        let _ = state.tick(&config, SignalValue::ON, 1.0);
        assert!(!state.is_open());

        let _ = state.tick(&config, SignalValue::ON, 1.0);
        let _ = state.tick(&config, SignalValue::ON, 1.0);
        let _ = state.tick(&config, SignalValue::ON, 1.0);
        assert!(state.is_open());
    }

    #[test]
    fn valve_fingerprint_deterministic() {
        let c1 = ValveConfig::binary(10.0);
        let c2 = ValveConfig::binary(10.0);
        assert_eq!(c1.fingerprint(), c2.fingerprint());
    }

    // -------------------------------------------------------------------------
    // Pump Tests
    // -------------------------------------------------------------------------

    #[test]
    fn pump_binary() {
        let config = PumpConfig::binary(10.0);
        let mut state = PumpState::default();

        let output = state.tick(&config, SignalValue::OFF, 0.0);
        assert!(!state.running);
        assert!((output.to_float() - 0.0).abs() < f32::EPSILON);

        let output = state.tick(&config, SignalValue::ON, 0.0);
        assert!(state.running);
        assert!((output.to_float() - 10.0).abs() < f32::EPSILON);
    }

    #[test]
    fn pump_variable() {
        let config = PumpConfig::variable(10.0);
        let mut state = PumpState::default();

        let output = state.tick(&config, SignalValue::Float(0.5), 0.0);
        assert!(state.running);
        assert!((output.to_float() - 5.0).abs() < 0.1);
    }

    #[test]
    fn pump_back_pressure() {
        let config = PumpConfig::binary(10.0);
        let mut state = PumpState::default();

        let output = state.tick(&config, SignalValue::ON, 0.5);
        assert!((output.to_float() - 5.0).abs() < f32::EPSILON);

        let output = state.tick(&config, SignalValue::ON, 1.0);
        assert!((output.to_float() - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn pump_fault() {
        let config = PumpConfig::binary(10.0);
        let mut state = PumpState::default();

        state.set_fault(true);
        let output = state.tick(&config, SignalValue::ON, 0.0);
        assert!(!state.running);
        assert!((output.to_float() - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn pump_ramp_time() {
        let config = PumpConfig::binary(10.0).with_ramp_time(4);
        let mut state = PumpState::default();

        let _ = state.tick(&config, SignalValue::ON, 0.0);
        assert!(!state.is_at_full_speed());

        let _ = state.tick(&config, SignalValue::ON, 0.0);
        let _ = state.tick(&config, SignalValue::ON, 0.0);
        let _ = state.tick(&config, SignalValue::ON, 0.0);
        assert!(state.is_at_full_speed());
    }

    #[test]
    fn pump_power_consumption() {
        let config = PumpConfig::binary(10.0).with_power(100.0);
        let mut state = PumpState::default();

        let _ = state.tick(&config, SignalValue::OFF, 0.0);
        assert!((state.power_draw - 0.0).abs() < f32::EPSILON);

        let _ = state.tick(&config, SignalValue::ON, 0.0);
        assert!((state.power_draw - 100.0).abs() < f32::EPSILON);
    }

    #[test]
    fn pump_fingerprint_deterministic() {
        let c1 = PumpConfig::binary(10.0);
        let c2 = PumpConfig::binary(10.0);
        assert_eq!(c1.fingerprint(), c2.fingerprint());
    }

    // -------------------------------------------------------------------------
    // Serde Tests
    // -------------------------------------------------------------------------

    #[test]
    fn serde_gate_config() {
        let config = GateConfig::nand();
        let json = serde_json::to_string(&config).unwrap();
        let recovered: GateConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, recovered);
    }

    #[test]
    fn serde_timer_config() {
        let config = TimerConfig::periodic(20).with_duty(75);
        let json = serde_json::to_string(&config).unwrap();
        let recovered: TimerConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, recovered);
    }

    #[test]
    fn serde_sensor_config() {
        let config = SensorConfig::pressure(0.0, 100.0);
        let json = serde_json::to_string(&config).unwrap();
        let recovered: SensorConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, recovered);
    }

    #[test]
    fn serde_primitive_config() {
        let config = PrimitiveConfig::Pump(PumpConfig::variable(10.0));
        let json = serde_json::to_string(&config).unwrap();
        let recovered: PrimitiveConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, recovered);
    }

    #[test]
    fn serde_primitive_state() {
        let mut state = PrimitiveState::Timer(TimerState::default());
        if let PrimitiveState::Timer(ref mut t) = state {
            t.counter = 42;
            t.active = true;
        }
        let json = serde_json::to_string(&state).unwrap();
        let recovered: PrimitiveState = serde_json::from_str(&json).unwrap();
        assert_eq!(state, recovered);
    }
}
