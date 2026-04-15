//! Rate limiting for anti-cheat
//!
//! Prevents action spam and DoS attacks by limiting action frequency.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use super::{ValidationConfig, ValidationResult};

/// Types of actions that can be rate limited
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActionType {
    /// Block placement
    BlockPlace,
    /// Block breaking
    BlockBreak,
    /// Item use/interaction
    ItemUse,
    /// Chat message
    Chat,
    /// Command execution
    Command,
    /// Entity interaction
    EntityInteract,
    /// Inventory transaction
    Inventory,
    /// Movement packet (usually higher limit)
    Movement,
}

impl ActionType {
    /// Get the default rate limit for this action type (actions per second)
    pub fn default_limit(&self) -> u32 {
        match self {
            ActionType::BlockPlace => 20,
            ActionType::BlockBreak => 20,
            ActionType::ItemUse => 20,
            ActionType::Chat => 3,
            ActionType::Command => 5,
            ActionType::EntityInteract => 15,
            ActionType::Inventory => 30,
            ActionType::Movement => 60,
        }
    }

    /// Get the burst allowance for this action type
    pub fn burst_allowance(&self) -> u32 {
        match self {
            ActionType::Chat => 5,
            ActionType::Command => 3,
            _ => 10,
        }
    }
}

/// Token bucket state for rate limiting
#[derive(Debug)]
struct TokenBucket {
    tokens: f64,
    max_tokens: f64,
    refill_rate: f64, // tokens per second
    last_update: Instant,
}

impl TokenBucket {
    fn new(max_tokens: u32, refill_rate: f64) -> Self {
        Self {
            tokens: max_tokens as f64,
            max_tokens: max_tokens as f64,
            refill_rate,
            last_update: Instant::now(),
        }
    }

    fn try_consume(&mut self, count: f64) -> bool {
        self.refill();
        if self.tokens >= count {
            self.tokens -= count;
            true
        } else {
            false
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.max_tokens);
        self.last_update = now;
    }
}

/// Per-player rate limiting state
#[derive(Debug)]
struct PlayerRateLimitState {
    buckets: HashMap<ActionType, TokenBucket>,
    violations: u32,
    last_violation: Option<Instant>,
}

impl PlayerRateLimitState {
    fn new() -> Self {
        Self {
            buckets: HashMap::new(),
            violations: 0,
            last_violation: None,
        }
    }

    fn get_bucket(&mut self, action_type: ActionType) -> &mut TokenBucket {
        self.buckets.entry(action_type).or_insert_with(|| {
            TokenBucket::new(
                action_type.burst_allowance(),
                action_type.default_limit() as f64,
            )
        })
    }
}

/// Rate limiter for player actions
pub struct RateLimiter {
    config: ValidationConfig,
    players: HashMap<u64, PlayerRateLimitState>,
    custom_limits: HashMap<ActionType, u32>,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(config: ValidationConfig) -> Self {
        Self {
            config,
            players: HashMap::new(),
            custom_limits: HashMap::new(),
        }
    }

    /// Set a custom rate limit for an action type
    pub fn set_limit(&mut self, action_type: ActionType, limit: u32) {
        self.custom_limits.insert(action_type, limit);
    }

    /// Register a new player
    pub fn register_player(&mut self, player_id: u64) {
        self.players.insert(player_id, PlayerRateLimitState::new());
    }

    /// Remove a player
    pub fn remove_player(&mut self, player_id: u64) {
        self.players.remove(&player_id);
    }

    /// Check if an action is allowed
    pub fn check_action(&mut self, player_id: u64, action_type: ActionType) -> ValidationResult {
        let state = self.players
            .entry(player_id)
            .or_insert_with(PlayerRateLimitState::new);

        let bucket = state.get_bucket(action_type);

        if bucket.try_consume(1.0) {
            // Decay violations over time
            if let Some(last_violation) = state.last_violation {
                if last_violation.elapsed() > Duration::from_secs(60) {
                    state.violations = state.violations.saturating_sub(1);
                    if state.violations == 0 {
                        state.last_violation = None;
                    }
                }
            }
            ValidationResult::Valid
        } else {
            state.violations += 1;
            state.last_violation = Some(Instant::now());

            if state.violations >= self.config.violation_threshold {
                ValidationResult::Kick {
                    reason: format!(
                        "Rate limit exceeded too many times ({} violations)",
                        state.violations
                    ),
                }
            } else if state.violations >= 3 {
                ValidationResult::Invalid {
                    reason: format!(
                        "Rate limit exceeded for {:?} ({} violations)",
                        action_type, state.violations
                    ),
                }
            } else {
                ValidationResult::Suspicious {
                    reason: format!("Rate limit exceeded for {:?}", action_type),
                }
            }
        }
    }

    /// Check multiple actions at once (for batch operations)
    pub fn check_actions(
        &mut self,
        player_id: u64,
        action_type: ActionType,
        count: u32,
    ) -> ValidationResult {
        let state = self.players
            .entry(player_id)
            .or_insert_with(PlayerRateLimitState::new);

        let bucket = state.get_bucket(action_type);

        if bucket.try_consume(count as f64) {
            ValidationResult::Valid
        } else {
            state.violations += 1;
            state.last_violation = Some(Instant::now());

            ValidationResult::Invalid {
                reason: format!(
                    "Batch rate limit exceeded for {:?} (requested {})",
                    action_type, count
                ),
            }
        }
    }

    /// Get current violations for a player
    pub fn get_violations(&self, player_id: u64) -> u32 {
        self.players
            .get(&player_id)
            .map(|s| s.violations)
            .unwrap_or(0)
    }

    /// Reset rate limit state for a player
    pub fn reset(&mut self, player_id: u64) {
        if let Some(state) = self.players.get_mut(&player_id) {
            state.buckets.clear();
            state.violations = 0;
            state.last_violation = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_allows_normal_usage() {
        let mut limiter = RateLimiter::new(ValidationConfig::default());
        limiter.register_player(1);

        // Should allow normal actions
        for _ in 0..5 {
            let result = limiter.check_action(1, ActionType::BlockPlace);
            assert!(result.is_valid());
        }
    }

    #[test]
    fn test_rate_limit_blocks_spam() {
        let mut limiter = RateLimiter::new(ValidationConfig::default());
        limiter.register_player(1);

        // Exhaust the bucket
        for _ in 0..50 {
            let _ = limiter.check_action(1, ActionType::Chat);
        }

        // Should be rate limited now
        let result = limiter.check_action(1, ActionType::Chat);
        assert!(!result.is_valid() || matches!(result, ValidationResult::Suspicious { .. }));
    }

    #[test]
    fn test_different_action_types_independent() {
        let mut limiter = RateLimiter::new(ValidationConfig::default());
        limiter.register_player(1);

        // Exhaust chat bucket
        for _ in 0..20 {
            let _ = limiter.check_action(1, ActionType::Chat);
        }

        // Block placement should still work
        let result = limiter.check_action(1, ActionType::BlockPlace);
        assert!(result.is_valid());
    }
}
