//! Client-side movement prediction with server reconciliation.
//!
//! The client predicts its own movement locally to hide latency.
//! When the server sends authoritative state, the client reconciles
//! by replaying any unacknowledged inputs from the correction point.

use glam::Vec3;

use crate::protocol::InputState;

/// Maximum number of unacknowledged inputs to buffer.
const MAX_INPUT_BUFFER: usize = 128;

/// Tolerance for position mismatch before reconciling (meters).
const RECONCILE_THRESHOLD: f32 = 0.01;

/// A recorded input for potential replay.
#[derive(Clone, Debug)]
pub struct InputRecord {
    /// The input state that was sent.
    pub input: InputState,
    /// Delta time when this input was applied.
    pub dt: f32,
    /// Predicted position after applying this input.
    pub predicted_position: Vec3,
    /// Predicted velocity after applying this input.
    pub predicted_velocity: Vec3,
    /// Whether player was on ground after this input.
    pub on_ground: bool,
}

/// Server-authoritative state for reconciliation.
#[derive(Clone, Debug)]
pub struct AuthoritativeState {
    /// Last input sequence the server processed.
    pub last_sequence: u32,
    /// Server-authoritative position.
    pub position: Vec3,
    /// Server-authoritative velocity.
    pub velocity: Vec3,
    /// Server-authoritative ground state.
    pub on_ground: bool,
}

/// Predicted client state.
#[derive(Clone, Debug, Default)]
pub struct PredictedState {
    /// Current predicted position.
    pub position: Vec3,
    /// Current predicted velocity.
    pub velocity: Vec3,
    /// Current predicted ground state.
    pub on_ground: bool,
}

/// Client-side movement predictor.
///
/// Stores a history of inputs and can reconcile with server state.
#[derive(Debug)]
pub struct MovementPredictor {
    /// Buffer of unacknowledged inputs.
    input_buffer: Vec<InputRecord>,
    /// Current sequence number for outgoing inputs.
    current_sequence: u32,
    /// Last acknowledged sequence from server (None = nothing acked yet).
    last_ack_sequence: Option<u32>,
    /// Current predicted state.
    state: PredictedState,
    /// Number of reconciliations performed.
    reconcile_count: u64,
}

impl Default for MovementPredictor {
    fn default() -> Self {
        Self::new()
    }
}

impl MovementPredictor {
    /// Create a new movement predictor.
    #[must_use]
    pub fn new() -> Self {
        Self {
            input_buffer: Vec::with_capacity(MAX_INPUT_BUFFER),
            current_sequence: 0,
            last_ack_sequence: None,
            state: PredictedState::default(),
            reconcile_count: 0,
        }
    }

    /// Create a predictor with initial state.
    #[must_use]
    pub fn with_state(position: Vec3, velocity: Vec3, on_ground: bool) -> Self {
        Self {
            input_buffer: Vec::with_capacity(MAX_INPUT_BUFFER),
            current_sequence: 0,
            last_ack_sequence: None,
            state: PredictedState {
                position,
                velocity,
                on_ground,
            },
            reconcile_count: 0,
        }
    }

    /// Get the next sequence number for an outgoing input.
    #[must_use]
    pub fn next_sequence(&mut self) -> u32 {
        let seq = self.current_sequence;
        self.current_sequence = self.current_sequence.wrapping_add(1);
        seq
    }

    /// Current sequence number (for creating inputs).
    #[must_use]
    pub fn current_sequence(&self) -> u32 {
        self.current_sequence
    }

    /// Record a predicted input and its result.
    ///
    /// Call this after applying input locally.
    pub fn record_input(&mut self, input: InputState, dt: f32, state: &PredictedState) {
        let record = InputRecord {
            input,
            dt,
            predicted_position: state.position,
            predicted_velocity: state.velocity,
            on_ground: state.on_ground,
        };

        self.input_buffer.push(record);

        // Prevent unbounded growth
        if self.input_buffer.len() > MAX_INPUT_BUFFER {
            self.input_buffer.remove(0);
        }

        // Update our state
        self.state = state.clone();
    }

    /// Get current predicted state.
    #[must_use]
    pub fn state(&self) -> &PredictedState {
        &self.state
    }

    /// Get mutable reference to predicted state.
    pub fn state_mut(&mut self) -> &mut PredictedState {
        &mut self.state
    }

    /// Reconcile with server-authoritative state.
    ///
    /// Returns inputs that need to be replayed (if any misprediction occurred).
    /// The caller should apply these inputs to the authoritative state using
    /// their physics simulation.
    ///
    /// # Returns
    /// `Some(inputs)` if reconciliation is needed, `None` if prediction was accurate.
    pub fn reconcile(&mut self, server_state: &AuthoritativeState) -> Option<Vec<InputRecord>> {
        // Ignore old acks (sequence wrapped or out of order)
        if let Some(last_ack) = self.last_ack_sequence
            && !is_sequence_newer(server_state.last_sequence, last_ack)
        {
            return None;
        }

        self.last_ack_sequence = Some(server_state.last_sequence);

        // Remove acknowledged inputs (keep only those newer than last_sequence)
        self.input_buffer.retain(|record| {
            is_sequence_newer(record.input.sequence, server_state.last_sequence)
        });

        // Check if we need to reconcile
        if self.input_buffer.is_empty() {
            // No pending inputs - just accept server state
            self.state.position = server_state.position;
            self.state.velocity = server_state.velocity;
            self.state.on_ground = server_state.on_ground;
            return None;
        }

        // Find position mismatch - compare server state to what we predicted
        // at the point when the first pending input was applied
        let first_pending = &self.input_buffer[0];
        let position_error = (server_state.position - first_pending.predicted_position).length();

        if position_error < RECONCILE_THRESHOLD {
            // Prediction was accurate enough
            return None;
        }

        // Misprediction detected - need to replay
        self.reconcile_count += 1;

        // Reset state to server authoritative
        self.state.position = server_state.position;
        self.state.velocity = server_state.velocity;
        self.state.on_ground = server_state.on_ground;

        // Return inputs to replay
        Some(self.input_buffer.clone())
    }

    /// Apply reconciliation result after replaying inputs.
    ///
    /// Call this after replaying the inputs returned by `reconcile()`.
    pub fn apply_replay_result(&mut self, final_state: PredictedState) {
        self.state = final_state;

        // Update predicted positions in buffer
        // (In a full implementation, we'd update each record's predicted state
        // during replay, but for simplicity we just update the final state)
    }

    /// Number of unacknowledged inputs in buffer.
    #[must_use]
    pub fn pending_input_count(&self) -> usize {
        self.input_buffer.len()
    }

    /// Number of reconciliations performed (for debugging).
    #[must_use]
    pub fn reconcile_count(&self) -> u64 {
        self.reconcile_count
    }

    /// Clear all state (e.g., on disconnect).
    pub fn reset(&mut self) {
        self.input_buffer.clear();
        self.current_sequence = 0;
        self.last_ack_sequence = None;
        self.state = PredictedState::default();
    }
}

/// Check if sequence `a` is newer than sequence `b`, handling wrapping.
fn is_sequence_newer(a: u32, b: u32) -> bool {
    // Handle sequence number wrapping using signed comparison
    let diff = a.wrapping_sub(b) as i32;
    diff > 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    fn make_input(seq: u32, movement: Vec3) -> InputState {
        InputState {
            movement,
            jump: false,
            sprint: false,
            yaw: 0.0,
            pitch: 0.0,
            sequence: seq,
        }
    }

    #[test]
    fn test_sequence_increment() {
        let mut predictor = MovementPredictor::new();

        assert_eq!(predictor.next_sequence(), 0);
        assert_eq!(predictor.next_sequence(), 1);
        assert_eq!(predictor.next_sequence(), 2);
        assert_eq!(predictor.current_sequence(), 3);
    }

    #[test]
    fn test_record_input() {
        let mut predictor = MovementPredictor::new();
        let seq = predictor.next_sequence();
        let input = make_input(seq, Vec3::new(0.0, 0.0, 1.0));

        let state = PredictedState {
            position: Vec3::new(0.0, 0.0, 1.0),
            velocity: Vec3::new(0.0, 0.0, 5.0),
            on_ground: true,
        };

        predictor.record_input(input, 0.016, &state);

        assert_eq!(predictor.pending_input_count(), 1);
        assert_eq!(predictor.state().position, Vec3::new(0.0, 0.0, 1.0));
    }

    #[test]
    fn test_reconcile_accurate_prediction() {
        let mut predictor = MovementPredictor::new();

        // Record some inputs
        let seq = predictor.next_sequence();
        let input = make_input(seq, Vec3::new(0.0, 0.0, 1.0));
        let state = PredictedState {
            position: Vec3::new(0.0, 0.0, 1.0),
            velocity: Vec3::new(0.0, 0.0, 5.0),
            on_ground: true,
        };
        predictor.record_input(input, 0.016, &state);

        // Server confirms prediction was accurate
        let server_state = AuthoritativeState {
            last_sequence: seq,
            position: Vec3::new(0.0, 0.0, 1.0),
            velocity: Vec3::new(0.0, 0.0, 5.0),
            on_ground: true,
        };

        let replay = predictor.reconcile(&server_state);
        assert!(replay.is_none(), "Should not need replay for accurate prediction");
        assert_eq!(predictor.pending_input_count(), 0);
    }

    #[test]
    fn test_reconcile_misprediction() {
        let mut predictor = MovementPredictor::new();

        // Record input with predicted position
        let seq = predictor.next_sequence();
        let input = make_input(seq, Vec3::new(0.0, 0.0, 1.0));
        let predicted_state = PredictedState {
            position: Vec3::new(0.0, 0.0, 1.0),
            velocity: Vec3::new(0.0, 0.0, 5.0),
            on_ground: true,
        };
        predictor.record_input(input, 0.016, &predicted_state);

        // Record another input
        let seq2 = predictor.next_sequence();
        let input2 = make_input(seq2, Vec3::new(0.0, 0.0, 1.0));
        let predicted_state2 = PredictedState {
            position: Vec3::new(0.0, 0.0, 2.0),
            velocity: Vec3::new(0.0, 0.0, 5.0),
            on_ground: true,
        };
        predictor.record_input(input2, 0.016, &predicted_state2);

        // Server says we were wrong (hit a wall at z=0.5)
        let server_state = AuthoritativeState {
            last_sequence: 0,
            position: Vec3::new(0.0, 0.0, 0.5), // Server corrected position
            velocity: Vec3::ZERO,
            on_ground: true,
        };

        let replay = predictor.reconcile(&server_state);
        assert!(replay.is_some(), "Should need replay after misprediction");

        let inputs_to_replay = replay.unwrap();
        assert_eq!(inputs_to_replay.len(), 1, "Should replay unacked input (seq 1)");
        assert_eq!(inputs_to_replay[0].input.sequence, 1);

        // State should be reset to server authoritative
        assert_eq!(predictor.state().position, Vec3::new(0.0, 0.0, 0.5));
    }

    #[test]
    fn test_sequence_wrapping() {
        // Test that sequence comparison handles wrapping
        assert!(is_sequence_newer(1, 0));
        assert!(is_sequence_newer(100, 50));
        assert!(!is_sequence_newer(50, 100));

        // Wrapping case: 0 is newer than u32::MAX
        assert!(is_sequence_newer(0, u32::MAX));
        assert!(is_sequence_newer(1, u32::MAX - 1));
    }

    #[test]
    fn test_buffer_limit() {
        let mut predictor = MovementPredictor::new();

        // Fill buffer beyond limit
        for _ in 0..150 {
            let seq = predictor.next_sequence();
            let input = make_input(seq, Vec3::ZERO);
            let state = PredictedState::default();
            predictor.record_input(input, 0.016, &state);
        }

        // Buffer should be capped
        assert!(predictor.pending_input_count() <= 128);
    }

    #[test]
    fn test_reset() {
        let mut predictor = MovementPredictor::with_state(
            Vec3::new(10.0, 5.0, 10.0),
            Vec3::new(1.0, 0.0, 1.0),
            true,
        );

        let seq = predictor.next_sequence();
        let input = make_input(seq, Vec3::ZERO);
        predictor.record_input(input, 0.016, &PredictedState::default());

        predictor.reset();

        assert_eq!(predictor.pending_input_count(), 0);
        assert_eq!(predictor.current_sequence(), 0);
        assert_eq!(predictor.state().position, Vec3::ZERO);
    }
}
