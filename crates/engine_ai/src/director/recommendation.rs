//! Director recommendations and events.

use super::ids::RecommendationId;
use super::pacing::PacingLevel;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Category of director recommendation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum RecommendationCategory {
    /// Adjust pacing intensity.
    Pacing,
    /// Trigger a challenge or event.
    Challenge,
    /// Provide respite or recovery time.
    Respite,
    /// Adjust resource pressure.
    ResourcePressure,
    /// Spawn creatures or threats.
    Spawn,
    /// Trigger environmental event.
    Environmental,
    /// Social or morale intervention.
    Social,
}

/// Priority of a recommendation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum RecommendationPriority {
    Low,
    Normal,
    High,
    Critical,
}

impl RecommendationPriority {
    #[must_use]
    pub fn weight(self) -> f32 {
        match self {
            Self::Low => 0.25,
            Self::Normal => 0.5,
            Self::High => 0.75,
            Self::Critical => 1.0,
        }
    }
}

/// A recommendation from the director.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Recommendation {
    pub id: RecommendationId,
    pub category: RecommendationCategory,
    pub priority: RecommendationPriority,
    /// Description of what should happen.
    pub action: String,
    /// Target intensity or value (context-dependent).
    pub target_value: Option<f32>,
    /// Suggested duration in ticks.
    pub duration: Option<u64>,
    /// Tick when generated.
    pub generated_tick: u64,
    /// Tick when expires (if any).
    pub expires_tick: Option<u64>,
    /// Whether this has been acted upon.
    pub acted: bool,
    /// Confidence in this recommendation (0.0 to 1.0).
    pub confidence: f32,
}

impl Recommendation {
    #[must_use]
    pub fn new(
        id: RecommendationId,
        category: RecommendationCategory,
        priority: RecommendationPriority,
        action: impl Into<String>,
        tick: u64,
    ) -> Self {
        Self {
            id,
            category,
            priority,
            action: action.into(),
            target_value: None,
            duration: None,
            generated_tick: tick,
            expires_tick: None,
            acted: false,
            confidence: 1.0,
        }
    }

    #[must_use]
    pub fn with_target_value(mut self, value: f32) -> Self {
        self.target_value = Some(value);
        self
    }

    #[must_use]
    pub fn with_duration(mut self, ticks: u64) -> Self {
        self.duration = Some(ticks);
        self
    }

    #[must_use]
    pub fn with_expiry(mut self, tick: u64) -> Self {
        self.expires_tick = Some(tick);
        self
    }

    #[must_use]
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    pub fn mark_acted(&mut self) {
        self.acted = true;
    }

    #[must_use]
    pub fn is_expired(&self, current_tick: u64) -> bool {
        self.expires_tick.is_some_and(|exp| current_tick > exp)
    }

    #[must_use]
    pub fn age(&self, current_tick: u64) -> u64 {
        current_tick.saturating_sub(self.generated_tick)
    }

    #[must_use]
    pub fn effective_priority(&self) -> f32 {
        self.priority.weight() * self.confidence
    }
}

/// Kind of director event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DirectorEventKind {
    /// Pacing intensity changed.
    PacingChanged,
    /// Pacing level crossed threshold.
    PacingLevelChanged,
    /// Grace period started.
    GracePeriodStarted,
    /// Grace period ended.
    GracePeriodEnded,
    /// Recommendation generated.
    RecommendationGenerated,
    /// Recommendation expired.
    RecommendationExpired,
    /// Recommendation acted upon.
    RecommendationActed,
    /// Competence assessment updated.
    CompetenceUpdated,
    /// Stockpile pressure updated.
    StockpilePressureUpdated,
    /// Shelter quality updated.
    ShelterQualityUpdated,
    /// Director tick completed.
    TickCompleted,
}

/// An event from the director system.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DirectorEvent {
    pub kind: DirectorEventKind,
    pub tick: u64,
    /// Associated value (context-dependent).
    pub value: Option<f32>,
    /// Previous value (for change events).
    pub previous_value: Option<f32>,
    /// Associated pacing level.
    pub pacing_level: Option<PacingLevel>,
    /// Associated recommendation ID.
    pub recommendation_id: Option<RecommendationId>,
    /// Description.
    pub description: Option<String>,
}

impl DirectorEvent {
    #[must_use]
    pub fn new(kind: DirectorEventKind, tick: u64) -> Self {
        Self {
            kind,
            tick,
            value: None,
            previous_value: None,
            pacing_level: None,
            recommendation_id: None,
            description: None,
        }
    }

    #[must_use]
    pub fn with_value(mut self, value: f32) -> Self {
        self.value = Some(value);
        self
    }

    #[must_use]
    pub fn with_change(mut self, previous: f32, current: f32) -> Self {
        self.previous_value = Some(previous);
        self.value = Some(current);
        self
    }

    #[must_use]
    pub fn with_pacing_level(mut self, level: PacingLevel) -> Self {
        self.pacing_level = Some(level);
        self
    }

    #[must_use]
    pub fn with_recommendation(mut self, id: RecommendationId) -> Self {
        self.recommendation_id = Some(id);
        self
    }

    #[must_use]
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    #[must_use]
    pub fn pacing_changed(tick: u64, previous: f32, current: f32, level: PacingLevel) -> Self {
        Self::new(DirectorEventKind::PacingChanged, tick)
            .with_change(previous, current)
            .with_pacing_level(level)
    }

    #[must_use]
    pub fn pacing_level_changed(tick: u64, level: PacingLevel) -> Self {
        Self::new(DirectorEventKind::PacingLevelChanged, tick).with_pacing_level(level)
    }

    #[must_use]
    pub fn grace_period_started(tick: u64, duration: u64) -> Self {
        #[expect(clippy::cast_precision_loss, reason = "tick value bounded")]
        Self::new(DirectorEventKind::GracePeriodStarted, tick).with_value(duration as f32)
    }

    #[must_use]
    pub fn grace_period_ended(tick: u64) -> Self {
        Self::new(DirectorEventKind::GracePeriodEnded, tick)
    }

    #[must_use]
    pub fn recommendation_generated(tick: u64, id: RecommendationId) -> Self {
        Self::new(DirectorEventKind::RecommendationGenerated, tick).with_recommendation(id)
    }

    #[must_use]
    pub fn competence_updated(tick: u64, score: f32) -> Self {
        Self::new(DirectorEventKind::CompetenceUpdated, tick).with_value(score)
    }
}

/// Log of director events.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DirectorEventLog {
    events: VecDeque<DirectorEvent>,
    capacity: usize,
}

impl DirectorEventLog {
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            events: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push(&mut self, event: DirectorEvent) {
        if self.events.len() >= self.capacity {
            self.events.pop_front();
        }
        self.events.push_back(event);
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.events.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &DirectorEvent> {
        self.events.iter()
    }

    #[must_use]
    pub fn recent(&self, count: usize) -> Vec<&DirectorEvent> {
        self.events.iter().rev().take(count).collect()
    }

    pub fn events_since(&self, tick: u64) -> impl Iterator<Item = &DirectorEvent> {
        self.events.iter().filter(move |e| e.tick >= tick)
    }

    pub fn events_by_kind(&self, kind: DirectorEventKind) -> impl Iterator<Item = &DirectorEvent> {
        self.events.iter().filter(move |e| e.kind == kind)
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }
}

/// Queue of pending recommendations.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RecommendationQueue {
    recommendations: VecDeque<Recommendation>,
    next_id: u64,
    max_size: usize,
}

impl RecommendationQueue {
    #[must_use]
    pub fn new(max_size: usize) -> Self {
        Self {
            recommendations: VecDeque::with_capacity(max_size),
            next_id: 1,
            max_size,
        }
    }

    pub fn generate_id(&mut self) -> RecommendationId {
        let id = RecommendationId::new(self.next_id);
        self.next_id += 1;
        id
    }

    pub fn push(&mut self, recommendation: Recommendation) {
        if self.recommendations.len() >= self.max_size {
            self.recommendations.pop_front();
        }
        self.recommendations.push_back(recommendation);
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.recommendations.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.recommendations.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Recommendation> {
        self.recommendations.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Recommendation> {
        self.recommendations.iter_mut()
    }

    #[must_use]
    pub fn pending(&self) -> Vec<&Recommendation> {
        self.recommendations.iter().filter(|r| !r.acted).collect()
    }

    #[must_use]
    pub fn by_category(&self, category: RecommendationCategory) -> Vec<&Recommendation> {
        self.recommendations
            .iter()
            .filter(|r| r.category == category)
            .collect()
    }

    #[must_use]
    pub fn highest_priority(&self) -> Option<&Recommendation> {
        self.recommendations
            .iter()
            .filter(|r| !r.acted)
            .max_by(|a, b| {
                a.effective_priority()
                    .partial_cmp(&b.effective_priority())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    pub fn cleanup_expired(&mut self, current_tick: u64) {
        self.recommendations.retain(|r| !r.is_expired(current_tick));
    }

    pub fn cleanup_acted(&mut self) {
        self.recommendations.retain(|r| !r.acted);
    }

    pub fn mark_acted(&mut self, id: RecommendationId) {
        if let Some(rec) = self.recommendations.iter_mut().find(|r| r.id == id) {
            rec.mark_acted();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recommendation_priority_weight() {
        assert!(RecommendationPriority::Low.weight() < RecommendationPriority::Normal.weight());
        assert!(RecommendationPriority::Critical.weight() > RecommendationPriority::High.weight());
    }

    #[test]
    fn test_recommendation_new() {
        let rec = Recommendation::new(
            RecommendationId::new(1),
            RecommendationCategory::Pacing,
            RecommendationPriority::High,
            "Reduce intensity",
            100,
        );

        assert_eq!(rec.id.raw(), 1);
        assert_eq!(rec.category, RecommendationCategory::Pacing);
        assert_eq!(rec.action, "Reduce intensity");
        assert!(!rec.acted);
    }

    #[test]
    fn test_recommendation_expiry() {
        let rec = Recommendation::new(
            RecommendationId::new(1),
            RecommendationCategory::Challenge,
            RecommendationPriority::Normal,
            "Spawn threat",
            100,
        )
        .with_expiry(200);

        assert!(!rec.is_expired(150));
        assert!(rec.is_expired(250));
    }

    #[test]
    fn test_recommendation_effective_priority() {
        let rec1 = Recommendation::new(
            RecommendationId::new(1),
            RecommendationCategory::Pacing,
            RecommendationPriority::High,
            "test",
            100,
        );

        let rec2 = Recommendation::new(
            RecommendationId::new(2),
            RecommendationCategory::Pacing,
            RecommendationPriority::High,
            "test",
            100,
        )
        .with_confidence(0.5);

        assert!(rec1.effective_priority() > rec2.effective_priority());
    }

    #[test]
    fn test_director_event_new() {
        let event = DirectorEvent::new(DirectorEventKind::PacingChanged, 100).with_change(0.5, 0.6);

        assert_eq!(event.kind, DirectorEventKind::PacingChanged);
        assert_eq!(event.previous_value, Some(0.5));
        assert_eq!(event.value, Some(0.6));
    }

    #[test]
    fn test_director_event_factories() {
        let event = DirectorEvent::pacing_changed(100, 0.4, 0.6, PacingLevel::Normal);
        assert_eq!(event.kind, DirectorEventKind::PacingChanged);
        assert_eq!(event.pacing_level, Some(PacingLevel::Normal));

        let event = DirectorEvent::grace_period_started(200, 500);
        assert_eq!(event.kind, DirectorEventKind::GracePeriodStarted);
        assert_eq!(event.value, Some(500.0));
    }

    #[test]
    fn test_director_event_log() {
        let mut log = DirectorEventLog::new(10);

        log.push(DirectorEvent::new(DirectorEventKind::TickCompleted, 100));
        log.push(DirectorEvent::new(DirectorEventKind::PacingChanged, 101));

        assert_eq!(log.len(), 2);

        let recent = log.recent(1);
        assert_eq!(recent[0].kind, DirectorEventKind::PacingChanged);
    }

    #[test]
    fn test_director_event_log_capacity() {
        let mut log = DirectorEventLog::new(3);

        for i in 0..5 {
            log.push(DirectorEvent::new(DirectorEventKind::TickCompleted, i));
        }

        assert_eq!(log.len(), 3);
    }

    #[test]
    fn test_recommendation_queue() {
        let mut queue = RecommendationQueue::new(10);

        let id = queue.generate_id();
        let rec = Recommendation::new(
            id,
            RecommendationCategory::Spawn,
            RecommendationPriority::Normal,
            "Spawn creatures",
            100,
        );
        queue.push(rec);

        assert_eq!(queue.len(), 1);
        assert_eq!(queue.pending().len(), 1);
    }

    #[test]
    fn test_recommendation_queue_highest_priority() {
        let mut queue = RecommendationQueue::new(10);

        let id1 = queue.generate_id();
        queue.push(Recommendation::new(
            id1,
            RecommendationCategory::Pacing,
            RecommendationPriority::Low,
            "low",
            100,
        ));

        let id2 = queue.generate_id();
        queue.push(Recommendation::new(
            id2,
            RecommendationCategory::Pacing,
            RecommendationPriority::Critical,
            "critical",
            100,
        ));

        let highest = queue.highest_priority().unwrap();
        assert_eq!(highest.priority, RecommendationPriority::Critical);
    }

    #[test]
    fn test_recommendation_queue_mark_acted() {
        let mut queue = RecommendationQueue::new(10);

        let id = queue.generate_id();
        queue.push(Recommendation::new(
            id,
            RecommendationCategory::Challenge,
            RecommendationPriority::Normal,
            "test",
            100,
        ));

        queue.mark_acted(id);

        assert_eq!(queue.pending().len(), 0);
    }

    #[test]
    fn test_recommendation_queue_cleanup() {
        let mut queue = RecommendationQueue::new(10);

        let id1 = queue.generate_id();
        queue.push(
            Recommendation::new(
                id1,
                RecommendationCategory::Pacing,
                RecommendationPriority::Normal,
                "expires",
                100,
            )
            .with_expiry(150),
        );

        let id2 = queue.generate_id();
        queue.push(Recommendation::new(
            id2,
            RecommendationCategory::Pacing,
            RecommendationPriority::Normal,
            "valid",
            100,
        ));

        queue.cleanup_expired(200);

        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn test_serde_recommendation() {
        let rec = Recommendation::new(
            RecommendationId::new(42),
            RecommendationCategory::Spawn,
            RecommendationPriority::High,
            "Spawn hostile pack",
            500,
        )
        .with_target_value(0.7)
        .with_duration(100);

        let json = serde_json::to_string(&rec).unwrap();
        let restored: Recommendation = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.id.raw(), 42);
        assert_eq!(restored.action, "Spawn hostile pack");
        assert_eq!(restored.target_value, Some(0.7));
    }

    #[test]
    fn test_bincode_director_event() {
        let event = DirectorEvent::pacing_changed(100, 0.3, 0.5, PacingLevel::Normal);

        let bytes = bincode::serialize(&event).unwrap();
        let restored: DirectorEvent = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.tick, 100);
        assert_eq!(restored.kind, DirectorEventKind::PacingChanged);
    }

    #[test]
    fn test_bincode_director_event_log() {
        let mut log = DirectorEventLog::new(50);
        log.push(DirectorEvent::new(DirectorEventKind::TickCompleted, 100));
        log.push(DirectorEvent::new(
            DirectorEventKind::CompetenceUpdated,
            101,
        ));

        let bytes = bincode::serialize(&log).unwrap();
        let restored: DirectorEventLog = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.len(), 2);
    }

    #[test]
    fn test_bincode_recommendation_queue() {
        let mut queue = RecommendationQueue::new(20);

        let id = queue.generate_id();
        queue.push(Recommendation::new(
            id,
            RecommendationCategory::Environmental,
            RecommendationPriority::Normal,
            "Weather event",
            200,
        ));

        let bytes = bincode::serialize(&queue).unwrap();
        let restored: RecommendationQueue = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.len(), 1);
    }
}
