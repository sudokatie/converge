//! Anti-cheat validation module
//! 
//! Implements spec 7.6 - input validation, speed limits, rate limits.

pub mod movement_validator;
pub mod rate_limiter;
pub mod resource_verifier;

pub use movement_validator::MovementValidator;
pub use rate_limiter::RateLimiter;
pub use resource_verifier::ResourceVerifier;

/// Result of a validation check
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationResult {
    /// Action is valid
    Valid,
    /// Action is suspicious but allowed (logged)
    Suspicious { reason: String },
    /// Action is invalid and rejected
    Invalid { reason: String },
    /// Player should be kicked
    Kick { reason: String },
}

impl ValidationResult {
    pub fn is_valid(&self) -> bool {
        matches!(self, ValidationResult::Valid | ValidationResult::Suspicious { .. })
    }

    pub fn is_kick(&self) -> bool {
        matches!(self, ValidationResult::Kick { .. })
    }
}

/// Configuration for anti-cheat validation
#[derive(Debug, Clone)]
pub struct ValidationConfig {
    /// Maximum allowed movement speed (blocks per second)
    pub max_speed: f32,
    /// Maximum allowed vertical speed (blocks per second)
    pub max_vertical_speed: f32,
    /// Position delta tolerance for latency compensation
    pub position_tolerance: f32,
    /// Maximum actions per second for rate limiting
    pub max_actions_per_second: u32,
    /// Number of violations before kick
    pub violation_threshold: u32,
    /// Whether to enable strict mode
    pub strict_mode: bool,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            max_speed: 20.0,          // ~sprint + speed II
            max_vertical_speed: 50.0,  // For falling/flying
            position_tolerance: 2.0,
            max_actions_per_second: 30,
            violation_threshold: 5,
            strict_mode: false,
        }
    }
}
