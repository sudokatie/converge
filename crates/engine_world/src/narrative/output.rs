//! Narrative output types and queue.

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

use super::{EventId, NarrativeEventKind};

/// Priority level for output ordering.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OutputPriority(pub u8);

impl OutputPriority {
    /// Highest priority (disasters, critical alerts).
    pub const CRITICAL: Self = Self(255);
    /// High priority (objectives, important events).
    pub const HIGH: Self = Self(200);
    /// Normal priority (anomalies, standard events).
    pub const NORMAL: Self = Self(150);
    /// Low priority (ambient radio, flavor text).
    pub const LOW: Self = Self(100);
    /// Minimal priority (background chatter).
    pub const MINIMAL: Self = Self(50);

    /// Create from raw level.
    #[must_use]
    pub const fn from_level(level: u8) -> Self {
        Self(level)
    }

    /// Get raw level.
    #[must_use]
    pub const fn level(&self) -> u8 {
        self.0
    }
}

impl PartialOrd for OutputPriority {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OutputPriority {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0).reverse()
    }
}

impl Default for OutputPriority {
    fn default() -> Self {
        Self::NORMAL
    }
}

/// Type of narrative output content.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OutputKind {
    /// Text message to display.
    Text,
    /// Audio cue to play.
    Audio,
    /// Both text and audio.
    TextAndAudio,
    /// Objective update (progress, status change).
    ObjectiveUpdate,
    /// System notification.
    Notification,
}

/// A narrative output ready for consumption by UI/audio systems.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NarrativeOutput {
    /// Unique output ID.
    pub id: u64,

    /// Source event ID.
    pub event_id: EventId,

    /// Event kind for categorization.
    pub kind: NarrativeEventKind,

    /// Output content type.
    pub output_kind: OutputKind,

    /// Priority for ordering.
    pub priority: OutputPriority,

    /// Tick when this output was generated.
    pub tick: u64,

    /// Text content (if applicable).
    pub text: Option<String>,

    /// Audio cue identifier (if applicable).
    pub audio_cue: Option<String>,

    /// Duration this output should be displayed (ticks).
    pub display_duration: u64,

    /// Whether this output has been consumed.
    pub consumed: bool,

    /// Custom metadata.
    pub metadata: Vec<(String, String)>,
}

impl NarrativeOutput {
    /// Create a new narrative output.
    #[must_use]
    pub fn new(id: u64, event_id: EventId, kind: NarrativeEventKind, tick: u64) -> Self {
        Self {
            id,
            event_id,
            kind,
            output_kind: OutputKind::Text,
            priority: OutputPriority::from_level(kind.default_priority()),
            tick,
            text: None,
            audio_cue: None,
            display_duration: 300,
            consumed: false,
            metadata: Vec::new(),
        }
    }

    /// Set output kind.
    #[must_use]
    pub fn with_output_kind(mut self, kind: OutputKind) -> Self {
        self.output_kind = kind;
        self
    }

    /// Set priority.
    #[must_use]
    pub fn with_priority(mut self, priority: OutputPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Set text content.
    #[must_use]
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        if self.output_kind == OutputKind::Audio {
            self.output_kind = OutputKind::TextAndAudio;
        } else if self.output_kind != OutputKind::TextAndAudio {
            self.output_kind = OutputKind::Text;
        }
        self
    }

    /// Set audio cue.
    #[must_use]
    pub fn with_audio(mut self, cue: impl Into<String>) -> Self {
        self.audio_cue = Some(cue.into());
        if self.output_kind == OutputKind::Text {
            self.output_kind = OutputKind::TextAndAudio;
        } else if self.output_kind != OutputKind::TextAndAudio {
            self.output_kind = OutputKind::Audio;
        }
        self
    }

    /// Set display duration.
    #[must_use]
    pub fn with_display_duration(mut self, duration: u64) -> Self {
        self.display_duration = duration;
        self
    }

    /// Add metadata.
    #[must_use]
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.push((key.into(), value.into()));
        self
    }

    /// Mark as consumed.
    pub fn consume(&mut self) {
        self.consumed = true;
    }

    /// Check if still active (not consumed and within display duration).
    #[must_use]
    pub fn is_active(&self, current_tick: u64) -> bool {
        !self.consumed && current_tick < self.tick + self.display_duration
    }
}

impl Eq for NarrativeOutput {}

impl PartialOrd for NarrativeOutput {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for NarrativeOutput {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority
            .cmp(&other.priority)
            .then_with(|| self.tick.cmp(&other.tick))
            .then_with(|| self.id.cmp(&other.id))
    }
}

/// Queue of narrative outputs awaiting consumption.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct OutputQueue {
    /// Queued outputs.
    outputs: Vec<NarrativeOutput>,

    /// Next output ID.
    next_id: u64,

    /// Maximum queue size (0 = unlimited).
    max_size: usize,
}

impl OutputQueue {
    /// Create a new output queue.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a queue with maximum size.
    #[must_use]
    pub fn with_max_size(max_size: usize) -> Self {
        Self {
            outputs: Vec::new(),
            next_id: 0,
            max_size,
        }
    }

    /// Enqueue an output.
    pub fn enqueue(&mut self, mut output: NarrativeOutput) {
        output.id = self.next_id;
        self.next_id += 1;

        self.outputs.push(output);
        self.outputs.sort();

        if self.max_size > 0 && self.outputs.len() > self.max_size {
            self.outputs.truncate(self.max_size);
        }
    }

    /// Create and enqueue an output.
    pub fn emit(
        &mut self,
        event_id: EventId,
        kind: NarrativeEventKind,
        tick: u64,
    ) -> &mut NarrativeOutput {
        let output = NarrativeOutput::new(self.next_id, event_id, kind, tick);
        self.next_id += 1;

        self.outputs.push(output);
        let idx = self.outputs.len() - 1;

        if self.max_size > 0 && self.outputs.len() > self.max_size {
            self.outputs.sort();
            self.outputs.truncate(self.max_size);
        }

        let final_idx = idx.min(self.outputs.len() - 1);
        &mut self.outputs[final_idx]
    }

    /// Dequeue the highest priority output.
    pub fn dequeue(&mut self) -> Option<NarrativeOutput> {
        if self.outputs.is_empty() {
            return None;
        }
        self.outputs.sort();
        Some(self.outputs.remove(0))
    }

    /// Peek at the highest priority output.
    #[must_use]
    pub fn peek(&self) -> Option<&NarrativeOutput> {
        if self.outputs.is_empty() {
            return None;
        }
        self.outputs.iter().min()
    }

    /// Get all pending outputs.
    #[must_use]
    pub fn pending(&self) -> &[NarrativeOutput] {
        &self.outputs
    }

    /// Get pending outputs by kind.
    pub fn by_kind(&self, kind: NarrativeEventKind) -> impl Iterator<Item = &NarrativeOutput> {
        self.outputs.iter().filter(move |o| o.kind == kind)
    }

    /// Consume outputs by marking them and removing expired ones.
    pub fn cleanup(&mut self, current_tick: u64) {
        self.outputs.retain(|o| o.is_active(current_tick));
    }

    /// Consume a specific output by ID.
    pub fn consume(&mut self, id: u64) -> bool {
        if let Some(output) = self.outputs.iter_mut().find(|o| o.id == id) {
            output.consume();
            true
        } else {
            false
        }
    }

    /// Clear all outputs.
    pub fn clear(&mut self) {
        self.outputs.clear();
    }

    /// Number of pending outputs.
    #[must_use]
    pub fn len(&self) -> usize {
        self.outputs.len()
    }

    /// Check if queue is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.outputs.is_empty()
    }

    /// Compute checksum of queue state.
    #[must_use]
    pub fn checksum(&self) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&self.next_id.to_le_bytes());
        for output in &self.outputs {
            hasher.update(&output.id.to_le_bytes());
            hasher.update(&output.event_id.0.to_le_bytes());
            hasher.update(&[output.kind as u8]);
            hasher.update(&output.tick.to_le_bytes());
            hasher.update(&[u8::from(output.consumed)]);
        }
        hasher.finalize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn priority_ordering() {
        assert!(OutputPriority::CRITICAL < OutputPriority::HIGH);
        assert!(OutputPriority::HIGH < OutputPriority::NORMAL);
        assert!(OutputPriority::NORMAL < OutputPriority::LOW);
    }

    #[test]
    fn output_creation() {
        let output = NarrativeOutput::new(1, EventId(100), NarrativeEventKind::Radio, 500)
            .with_text("Incoming transmission")
            .with_audio("radio_static");

        assert_eq!(output.output_kind, OutputKind::TextAndAudio);
        assert!(output.text.is_some());
        assert!(output.audio_cue.is_some());
    }

    #[test]
    fn output_active() {
        let output = NarrativeOutput::new(1, EventId(1), NarrativeEventKind::Radio, 100)
            .with_display_duration(200);

        assert!(output.is_active(100));
        assert!(output.is_active(299));
        assert!(!output.is_active(300));
    }

    #[test]
    fn output_ordering() {
        let high = NarrativeOutput::new(1, EventId(1), NarrativeEventKind::Disaster, 100);
        let low = NarrativeOutput::new(2, EventId(2), NarrativeEventKind::Radio, 100);

        assert!(high < low);
    }

    #[test]
    fn queue_enqueue_dequeue() {
        let mut queue = OutputQueue::new();

        queue.enqueue(NarrativeOutput::new(
            0,
            EventId(1),
            NarrativeEventKind::Radio,
            100,
        ));
        queue.enqueue(NarrativeOutput::new(
            0,
            EventId(2),
            NarrativeEventKind::Disaster,
            100,
        ));

        let first = queue.dequeue().unwrap();
        assert_eq!(first.kind, NarrativeEventKind::Disaster);
    }

    #[test]
    fn queue_max_size() {
        let mut queue = OutputQueue::with_max_size(2);

        for i in 0..5 {
            queue.enqueue(NarrativeOutput::new(
                0,
                EventId(i),
                NarrativeEventKind::Radio,
                i,
            ));
        }

        assert_eq!(queue.len(), 2);
    }

    #[test]
    fn queue_cleanup() {
        let mut queue = OutputQueue::new();

        queue.enqueue(
            NarrativeOutput::new(0, EventId(1), NarrativeEventKind::Radio, 100)
                .with_display_duration(50),
        );
        queue.enqueue(
            NarrativeOutput::new(0, EventId(2), NarrativeEventKind::Radio, 200)
                .with_display_duration(100),
        );

        queue.cleanup(160);
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn queue_consume() {
        let mut queue = OutputQueue::new();
        queue.enqueue(NarrativeOutput::new(
            0,
            EventId(1),
            NarrativeEventKind::Radio,
            100,
        ));

        let id = queue.pending()[0].id;
        assert!(queue.consume(id));
        assert!(queue.pending()[0].consumed);
    }

    #[test]
    fn queue_checksum_deterministic() {
        let mut q1 = OutputQueue::new();
        let mut q2 = OutputQueue::new();

        q1.enqueue(NarrativeOutput::new(
            0,
            EventId(1),
            NarrativeEventKind::Radio,
            100,
        ));
        q2.enqueue(NarrativeOutput::new(
            0,
            EventId(1),
            NarrativeEventKind::Radio,
            100,
        ));

        assert_eq!(q1.checksum(), q2.checksum());
    }

    #[test]
    fn serde_round_trip() {
        let output = NarrativeOutput::new(1, EventId(100), NarrativeEventKind::Anomaly, 500)
            .with_text("Strange signal detected")
            .with_priority(OutputPriority::HIGH);

        let json = serde_json::to_string(&output).unwrap();
        let recovered: NarrativeOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered.text, output.text);
        assert_eq!(recovered.priority, output.priority);

        let mut queue = OutputQueue::new();
        queue.enqueue(output);

        let json = serde_json::to_string(&queue).unwrap();
        let recovered: OutputQueue = serde_json::from_str(&json).unwrap();
        assert_eq!(recovered.len(), 1);
    }
}
