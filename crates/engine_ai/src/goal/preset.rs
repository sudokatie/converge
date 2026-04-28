//! Preset goal definitions for common survival behaviors.

use super::consideration::{Consideration, InputBinding};
use super::curve::UtilityCurve;
use super::definition::{GoalDef, GoalId, GoalTag};

/// Create a preset goal for satisfying hunger.
#[must_use]
pub fn preset_satisfy_hunger() -> GoalDef {
    GoalDef::new(GoalId::satisfy_hunger(), "Satisfy Hunger")
        .with_priority(1.5)
        .with_threshold(0.1)
        .with_tag(GoalTag::survival())
        .with_cooldown(60)
        .with_consideration(
            Consideration::new("hunger_deficit")
                .with_input(InputBinding::need_deficit("hunger"))
                .with_curve(UtilityCurve::quadratic())
                .with_weight(1.0),
        )
        .with_consideration(
            Consideration::new("safety")
                .with_input(InputBinding::ThreatLevel)
                .with_curve(UtilityCurve::inverse_linear())
                .with_weight(0.8)
                .with_veto(0.05),
        )
}

/// Create a preset goal for seeking water.
#[must_use]
pub fn preset_seek_water() -> GoalDef {
    GoalDef::new(GoalId::seek_water(), "Seek Water")
        .with_priority(1.6)
        .with_threshold(0.1)
        .with_tag(GoalTag::survival())
        .with_cooldown(60)
        .with_consideration(
            Consideration::new("thirst_deficit")
                .with_input(InputBinding::need_deficit("thirst"))
                .with_curve(UtilityCurve::quadratic())
                .with_weight(1.0),
        )
        .with_consideration(
            Consideration::new("safety")
                .with_input(InputBinding::ThreatLevel)
                .with_curve(UtilityCurve::inverse_linear())
                .with_weight(0.7)
                .with_veto(0.05),
        )
}

/// Create a preset goal for seeking oxygen.
#[must_use]
pub fn preset_seek_oxygen() -> GoalDef {
    GoalDef::new(GoalId::seek_oxygen(), "Seek Oxygen")
        .with_priority(2.5)
        .with_threshold(0.05)
        .with_tag(GoalTag::survival())
        .with_interruptible(false)
        .with_consideration(
            Consideration::new("oxygen_deficit")
                .with_input(InputBinding::need_deficit("oxygen"))
                .with_curve(UtilityCurve::new(super::curve::CurveKind::Polynomial {
                    slope: 1.0,
                    exponent: 3.0,
                    x_shift: 0.0,
                    y_shift: 0.0,
                }))
                .with_weight(1.0),
        )
}

/// Create a preset goal for warming up.
#[must_use]
pub fn preset_warm_up() -> GoalDef {
    GoalDef::new(GoalId::warm_up(), "Warm Up")
        .with_priority(1.3)
        .with_threshold(0.15)
        .with_tag(GoalTag::survival())
        .with_cooldown(120)
        .with_consideration(
            Consideration::new("warmth_deficit")
                .with_input(InputBinding::need_deficit("warmth"))
                .with_curve(UtilityCurve::sigmoid())
                .with_weight(1.0),
        )
        .with_consideration(
            Consideration::new("cold_severity")
                .with_input(InputBinding::fact("temperature_danger"))
                .with_curve(UtilityCurve::linear())
                .with_weight(0.5),
        )
        .with_consideration(
            Consideration::new("safety")
                .with_input(InputBinding::ThreatLevel)
                .with_curve(UtilityCurve::inverse_linear())
                .with_weight(0.6),
        )
}

/// Create a preset goal for cooling down.
#[must_use]
pub fn preset_cool_down() -> GoalDef {
    GoalDef::new(GoalId::cool_down(), "Cool Down")
        .with_priority(1.3)
        .with_threshold(0.15)
        .with_tag(GoalTag::survival())
        .with_cooldown(120)
        .with_consideration(
            Consideration::new("heat_level")
                .with_input(InputBinding::need_value("warmth"))
                .with_curve(UtilityCurve::new(super::curve::CurveKind::Polynomial {
                    slope: 1.0,
                    exponent: 2.0,
                    x_shift: 0.7,
                    y_shift: 0.0,
                }))
                .with_weight(1.0),
        )
        .with_consideration(
            Consideration::new("heat_severity")
                .with_input(InputBinding::fact("heat_danger"))
                .with_curve(UtilityCurve::linear())
                .with_weight(0.5),
        )
}

/// Create a preset goal for resting.
#[must_use]
pub fn preset_rest() -> GoalDef {
    GoalDef::new(GoalId::rest(), "Rest")
        .with_priority(1.2)
        .with_threshold(0.2)
        .with_tag(GoalTag::survival())
        .with_cooldown(300)
        .with_min_duration(60)
        .with_consideration(
            Consideration::new("rest_deficit")
                .with_input(InputBinding::need_deficit("rest"))
                .with_curve(UtilityCurve::quadratic())
                .with_weight(1.0),
        )
        .with_consideration(
            Consideration::new("safety")
                .with_input(InputBinding::ThreatLevel)
                .with_curve(UtilityCurve::inverse_quadratic())
                .with_weight(1.0)
                .with_veto(0.1),
        )
        .with_consideration(
            Consideration::new("shelter")
                .with_input(InputBinding::fact("has_shelter"))
                .with_curve(UtilityCurve::linear())
                .with_weight(0.3),
        )
}

/// Create a preset goal for fleeing danger.
#[must_use]
pub fn preset_flee_danger() -> GoalDef {
    GoalDef::new(GoalId::flee_danger(), "Flee Danger")
        .with_priority(3.0)
        .with_threshold(0.2)
        .with_tag(GoalTag::survival())
        .with_tag(GoalTag::combat())
        .with_interruptible(false)
        .with_min_duration(30)
        .with_cooldown(60)
        .with_consideration(
            Consideration::new("threat_level")
                .with_input(InputBinding::ThreatLevel)
                .with_curve(UtilityCurve::sigmoid())
                .with_weight(1.0),
        )
        .with_consideration(
            Consideration::new("danger_proximity")
                .with_input(InputBinding::DangerProximity)
                .with_curve(UtilityCurve::quadratic())
                .with_weight(1.2),
        )
        .with_consideration(
            Consideration::new("enemy_count")
                .with_input(InputBinding::enemy_count(5))
                .with_curve(UtilityCurve::linear())
                .with_weight(0.5),
        )
}

/// Create a preset goal for seeking allies.
#[must_use]
pub fn preset_seek_allies() -> GoalDef {
    GoalDef::new(GoalId::seek_allies(), "Seek Allies")
        .with_priority(0.8)
        .with_threshold(0.15)
        .with_tag(GoalTag::social())
        .with_cooldown(180)
        .with_consideration(
            Consideration::new("social_deficit")
                .with_input(InputBinding::need_deficit("social"))
                .with_curve(UtilityCurve::linear())
                .with_weight(1.0),
        )
        .with_consideration(
            Consideration::new("ally_scarcity")
                .with_input(InputBinding::ally_count(3))
                .with_curve(UtilityCurve::inverse_linear())
                .with_weight(0.8),
        )
        .with_consideration(
            Consideration::new("safety")
                .with_input(InputBinding::ThreatLevel)
                .with_curve(UtilityCurve::inverse_linear())
                .with_weight(0.5),
        )
}

/// Create a preset goal for defending territory.
#[must_use]
pub fn preset_defend_territory() -> GoalDef {
    GoalDef::new(GoalId::defend_territory(), "Defend Territory")
        .with_priority(1.8)
        .with_threshold(0.2)
        .with_tag(GoalTag::combat())
        .with_min_duration(60)
        .with_cooldown(120)
        .with_consideration(
            Consideration::new("territory_ownership")
                .with_input(InputBinding::TerritoryOwnership)
                .with_curve(UtilityCurve::linear())
                .with_weight(0.6),
        )
        .with_consideration(
            Consideration::new("enemy_presence")
                .with_input(InputBinding::enemy_count(3))
                .with_curve(UtilityCurve::sigmoid())
                .with_weight(1.0),
        )
        .with_consideration(
            Consideration::new("ally_support")
                .with_input(InputBinding::ally_count(5))
                .with_curve(UtilityCurve::linear())
                .with_weight(0.4),
        )
}

/// Create a preset goal for investigating a stimulus.
#[must_use]
pub fn preset_investigate_stimulus() -> GoalDef {
    GoalDef::new(GoalId::investigate_stimulus(), "Investigate Stimulus")
        .with_priority(0.9)
        .with_threshold(0.1)
        .with_tag(GoalTag::exploration())
        .with_cooldown(90)
        .with_consideration(
            Consideration::new("stimulus_intensity")
                .with_input(InputBinding::stimulus_intensity("sight"))
                .with_curve(UtilityCurve::linear())
                .with_weight(1.0),
        )
        .with_consideration(
            Consideration::new("sound_stimulus")
                .with_input(InputBinding::stimulus_intensity("sound"))
                .with_curve(UtilityCurve::linear())
                .with_weight(0.8),
        )
        .with_consideration(
            Consideration::new("safety")
                .with_input(InputBinding::ThreatLevel)
                .with_curve(UtilityCurve::inverse_quadratic())
                .with_weight(0.7)
                .with_veto(0.1),
        )
        .with_consideration(
            Consideration::new("curiosity")
                .with_input(InputBinding::time_since_goal("investigate_stimulus", 600))
                .with_curve(UtilityCurve::linear())
                .with_weight(0.3),
        )
}

/// Create a preset goal for following a leader.
#[must_use]
pub fn preset_follow_leader() -> GoalDef {
    GoalDef::new(GoalId::follow_leader(), "Follow Leader")
        .with_priority(1.1)
        .with_threshold(0.1)
        .with_tag(GoalTag::social())
        .with_consideration(
            Consideration::new("leader_distance")
                .with_input(InputBinding::leader_distance(50.0))
                .with_curve(UtilityCurve::quadratic())
                .with_weight(1.0),
        )
        .with_consideration(
            Consideration::new("has_leader")
                .with_input(InputBinding::fact("has_leader"))
                .with_curve(UtilityCurve::step())
                .with_weight(1.0)
                .with_veto(0.5),
        )
        .with_consideration(
            Consideration::new("safety")
                .with_input(InputBinding::ThreatLevel)
                .with_curve(UtilityCurve::inverse_linear())
                .with_weight(0.4),
        )
}

/// Create a preset goal for patrolling.
#[must_use]
pub fn preset_patrol() -> GoalDef {
    GoalDef::new(GoalId::patrol(), "Patrol")
        .with_priority(0.6)
        .with_threshold(0.1)
        .with_tag(GoalTag::work())
        .with_tag(GoalTag::exploration())
        .with_cooldown(180)
        .with_consideration(
            Consideration::new("territory_ownership")
                .with_input(InputBinding::TerritoryOwnership)
                .with_curve(UtilityCurve::linear())
                .with_weight(0.5),
        )
        .with_consideration(
            Consideration::new("time_since_patrol")
                .with_input(InputBinding::time_since_goal("patrol", 600))
                .with_curve(UtilityCurve::linear())
                .with_weight(0.8),
        )
        .with_consideration(
            Consideration::new("safety")
                .with_input(InputBinding::ThreatLevel)
                .with_curve(UtilityCurve::inverse_linear())
                .with_weight(0.6),
        )
        .with_consideration(
            Consideration::new("energy")
                .with_input(InputBinding::need_value("rest"))
                .with_curve(UtilityCurve::linear())
                .with_weight(0.4),
        )
}

/// Create a preset goal for idling.
#[must_use]
pub fn preset_idle() -> GoalDef {
    GoalDef::new(GoalId::idle(), "Idle")
        .with_priority(0.1)
        .with_threshold(0.0)
        .with_tag(GoalTag::idle())
        .with_consideration(
            Consideration::new("baseline")
                .with_input(InputBinding::Constant(0.5))
                .with_curve(UtilityCurve::constant(1.0))
                .with_weight(1.0),
        )
}

/// Get all preset survival goals.
#[must_use]
pub fn survival_goals() -> Vec<GoalDef> {
    vec![
        preset_satisfy_hunger(),
        preset_seek_water(),
        preset_seek_oxygen(),
        preset_warm_up(),
        preset_cool_down(),
        preset_rest(),
        preset_flee_danger(),
        preset_seek_allies(),
        preset_defend_territory(),
        preset_investigate_stimulus(),
        preset_follow_leader(),
        preset_patrol(),
        preset_idle(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::goal::{GoalContext, GoalSelector};

    #[test]
    fn test_preset_satisfy_hunger() {
        let goal = preset_satisfy_hunger();

        assert_eq!(goal.id, GoalId::satisfy_hunger());
        assert!(goal.has_tag(&GoalTag::survival()));
        assert!(goal.consideration_count() >= 2);
    }

    #[test]
    fn test_preset_seek_water() {
        let goal = preset_seek_water();

        assert_eq!(goal.id, GoalId::seek_water());
        assert!(goal.has_tag(&GoalTag::survival()));
    }

    #[test]
    fn test_preset_seek_oxygen() {
        let goal = preset_seek_oxygen();

        assert_eq!(goal.id, GoalId::seek_oxygen());
        assert!(!goal.interruptible);
        assert!((goal.base_priority - 2.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_preset_flee_danger() {
        let goal = preset_flee_danger();

        assert_eq!(goal.id, GoalId::flee_danger());
        assert!(goal.has_tag(&GoalTag::survival()));
        assert!(goal.has_tag(&GoalTag::combat()));
        assert!(!goal.interruptible);
    }

    #[test]
    fn test_preset_idle() {
        let goal = preset_idle();

        assert_eq!(goal.id, GoalId::idle());
        assert!(goal.has_tag(&GoalTag::idle()));
        assert!(goal.base_priority < 0.5);
    }

    #[test]
    fn test_survival_goals_complete() {
        let goals = survival_goals();

        assert!(goals.len() >= 10);

        let ids: Vec<_> = goals.iter().map(|g| g.id.as_str()).collect();
        assert!(ids.contains(&GoalId::SATISFY_HUNGER));
        assert!(ids.contains(&GoalId::FLEE_DANGER));
        assert!(ids.contains(&GoalId::REST));
        assert!(ids.contains(&GoalId::IDLE));
    }

    #[test]
    fn test_survival_goals_unique_ids() {
        let goals = survival_goals();
        let mut ids: Vec<_> = goals.iter().map(|g| g.id.as_str()).collect();
        ids.sort_unstable();
        let original_len = ids.len();
        ids.dedup();

        assert_eq!(ids.len(), original_len, "Duplicate goal IDs found");
    }

    #[test]
    fn test_preset_goals_evaluate() {
        let mut selector = GoalSelector::new();

        for goal in survival_goals() {
            selector.register(goal);
        }
        selector.set_fallback(GoalId::idle());

        let ctx = GoalContext::builder()
            .with_tick(100)
            .with_threat_level(0.0)
            .build();

        let result = selector.evaluate(&ctx);

        assert!(result.selected.score >= 0.0);
    }

    #[test]
    fn test_high_hunger_selects_satisfy_hunger() {
        let mut selector = GoalSelector::new();

        for goal in survival_goals() {
            selector.register(goal);
        }

        let ctx = GoalContext::builder()
            .with_tick(100)
            .with_need_value(&crate::needs::NeedId::hunger(), 0.1)
            .with_threat_level(0.0)
            .build();

        let result = selector.evaluate(&ctx);

        assert_eq!(result.selected.id, GoalId::satisfy_hunger());
    }

    #[test]
    fn test_high_threat_selects_flee() {
        let mut selector = GoalSelector::new();

        for goal in survival_goals() {
            selector.register(goal);
        }

        let ctx = GoalContext::builder()
            .with_tick(100)
            .with_threat_level(0.9)
            .with_danger_proximity(0.8)
            .build();

        let result = selector.evaluate(&ctx);

        assert_eq!(result.selected.id, GoalId::flee_danger());
    }

    #[test]
    fn test_low_oxygen_takes_priority() {
        let mut selector = GoalSelector::new();

        for goal in survival_goals() {
            selector.register(goal);
        }

        let ctx = GoalContext::builder()
            .with_tick(100)
            .with_need_value(&crate::needs::NeedId::oxygen(), 0.1)
            .with_need_value(&crate::needs::NeedId::hunger(), 0.1)
            .with_threat_level(0.0)
            .build();

        let result = selector.evaluate(&ctx);

        assert_eq!(result.selected.id, GoalId::seek_oxygen());
    }

    #[test]
    fn test_preset_warm_up() {
        let goal = preset_warm_up();

        assert_eq!(goal.id, GoalId::warm_up());
        assert!(goal.has_tag(&GoalTag::survival()));
        assert!(goal.consideration_count() >= 2);
    }

    #[test]
    fn test_preset_defend_territory() {
        let goal = preset_defend_territory();

        assert_eq!(goal.id, GoalId::defend_territory());
        assert!(goal.has_tag(&GoalTag::combat()));
        assert!(goal.min_duration > 0);
    }

    #[test]
    fn test_preset_follow_leader() {
        let goal = preset_follow_leader();

        assert_eq!(goal.id, GoalId::follow_leader());
        assert!(goal.has_tag(&GoalTag::social()));
    }

    #[test]
    fn test_preset_patrol() {
        let goal = preset_patrol();

        assert_eq!(goal.id, GoalId::patrol());
        assert!(goal.has_tag(&GoalTag::work()));
        assert!(goal.has_tag(&GoalTag::exploration()));
    }

    #[test]
    fn test_presets_serde_round_trip() {
        for goal in survival_goals() {
            let json = serde_json::to_string(&goal).unwrap();
            let restored: GoalDef = serde_json::from_str(&json).unwrap();

            assert_eq!(restored.id, goal.id);
            assert!((restored.base_priority - goal.base_priority).abs() < f32::EPSILON);
        }
    }
}
