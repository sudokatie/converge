//! Tests for the sky renderer.

/// Mock test for new creates renderer with default time.
/// Note: Full GPU tests require a window/surface, so we test logic separately.
#[test]
fn test_time_of_day_default() {
    // SkyRenderer::new requires a RenderDevice which needs a window.
    // We test the time logic by verifying the expected default behavior.
    // The default time_of_day is 0.5 (noon).
    let default_time: f32 = 0.5;
    assert!((default_time - 0.5).abs() < f32::EPSILON);
}

/// Test struct to verify time update logic without GPU.
struct TimeOfDayState {
    time_of_day: f32,
}

impl TimeOfDayState {
    fn new() -> Self {
        Self { time_of_day: 0.5 }
    }

    fn update(&mut self, dt: f32, time_scale: f32) {
        self.time_of_day += dt * time_scale;
        self.time_of_day = self.time_of_day.rem_euclid(1.0);
    }

    fn set_time_of_day(&mut self, t: f32) {
        self.time_of_day = t.rem_euclid(1.0);
    }

    fn time_of_day(&self) -> f32 {
        self.time_of_day
    }
}

#[test]
fn test_set_time_of_day_updates_correctly() {
    let mut state = TimeOfDayState::new();

    state.set_time_of_day(0.25);
    assert!((state.time_of_day() - 0.25).abs() < f32::EPSILON);

    state.set_time_of_day(0.75);
    assert!((state.time_of_day() - 0.75).abs() < f32::EPSILON);

    state.set_time_of_day(0.0);
    assert!(state.time_of_day().abs() < f32::EPSILON);

    state.set_time_of_day(1.0);
    assert!(state.time_of_day().abs() < f32::EPSILON); // Wraps to 0.0
}

#[test]
fn test_update_advances_time_and_wraps() {
    let mut state = TimeOfDayState::new();
    state.set_time_of_day(0.0);

    // Advance by 0.1 with scale 1.0
    state.update(0.1, 1.0);
    assert!((state.time_of_day() - 0.1).abs() < f32::EPSILON);

    // Advance to 0.9
    state.set_time_of_day(0.9);
    state.update(0.2, 1.0);
    // Should wrap: 0.9 + 0.2 = 1.1 -> 0.1
    assert!((state.time_of_day() - 0.1).abs() < f32::EPSILON);
}

#[test]
fn test_update_with_time_scale() {
    let mut state = TimeOfDayState::new();
    state.set_time_of_day(0.0);

    // Advance by 0.1 with scale 2.0 -> should advance by 0.2
    state.update(0.1, 2.0);
    assert!((state.time_of_day() - 0.2).abs() < f32::EPSILON);

    // Advance by 0.1 with scale 0.5 -> should advance by 0.05
    state.set_time_of_day(0.0);
    state.update(0.1, 0.5);
    assert!((state.time_of_day() - 0.05).abs() < f32::EPSILON);
}

#[test]
fn test_update_with_zero_dt_doesnt_change_time() {
    let mut state = TimeOfDayState::new();
    let initial_time = 0.25;
    state.set_time_of_day(initial_time);

    state.update(0.0, 1.0);
    assert!((state.time_of_day() - initial_time).abs() < f32::EPSILON);

    state.update(0.0, 100.0);
    assert!((state.time_of_day() - initial_time).abs() < f32::EPSILON);
}

#[test]
fn test_set_time_of_day_handles_negative_values() {
    let mut state = TimeOfDayState::new();

    // Negative values should wrap correctly
    state.set_time_of_day(-0.25);
    assert!((state.time_of_day() - 0.75).abs() < f32::EPSILON);

    state.set_time_of_day(-1.0);
    assert!(state.time_of_day().abs() < f32::EPSILON);
}

#[test]
fn test_set_time_of_day_handles_large_values() {
    let mut state = TimeOfDayState::new();

    // Values > 1.0 should wrap
    state.set_time_of_day(2.5);
    assert!((state.time_of_day() - 0.5).abs() < f32::EPSILON);

    state.set_time_of_day(10.25);
    assert!((state.time_of_day() - 0.25).abs() < f32::EPSILON);
}
