//! Machine runtime state.

use serde::{Deserialize, Serialize};

use super::config::{MachineConfig, ProcessId};
use super::identity::MachineTier;

/// Fault conditions that can affect machine operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
pub enum FaultKind {
    /// No fault - machine operational.
    None = 0,
    /// Insufficient power to operate.
    NoPower = 1,
    /// Overheated - temperature exceeded limit.
    Overheat = 2,
    /// Input resources depleted.
    NoInput = 3,
    /// Output storage full.
    OutputFull = 4,
    /// Fluid port blocked or empty.
    FluidFault = 5,
    /// Maintenance overdue.
    MaintenanceRequired = 6,
    /// Structural damage.
    Damaged = 7,
    /// Generic jam/blockage.
    Jammed = 8,
}

impl FaultKind {
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::NoPower => "no_power",
            Self::Overheat => "overheat",
            Self::NoInput => "no_input",
            Self::OutputFull => "output_full",
            Self::FluidFault => "fluid_fault",
            Self::MaintenanceRequired => "maintenance_required",
            Self::Damaged => "damaged",
            Self::Jammed => "jammed",
        }
    }

    #[must_use]
    pub const fn is_fault(&self) -> bool {
        !matches!(self, Self::None)
    }

    #[must_use]
    pub const fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::NoPower | Self::NoInput | Self::OutputFull | Self::FluidFault
        )
    }

    #[must_use]
    pub const fn requires_maintenance(&self) -> bool {
        matches!(
            self,
            Self::MaintenanceRequired | Self::Damaged | Self::Jammed
        )
    }
}

/// A process in the machine's queue.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QueuedProcess {
    /// Process definition ID.
    pub process_id: ProcessId,
    /// Quantity to produce (for batching).
    pub quantity: u32,
    /// Priority (lower = higher priority).
    pub priority: u8,
}

impl QueuedProcess {
    #[must_use]
    pub const fn new(process_id: ProcessId, quantity: u32) -> Self {
        Self {
            process_id,
            quantity,
            priority: 128,
        }
    }

    #[must_use]
    pub const fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }
}

impl PartialOrd for QueuedProcess {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for QueuedProcess {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.priority
            .cmp(&other.priority)
            .then_with(|| self.process_id.cmp(&other.process_id))
    }
}

impl Eq for QueuedProcess {}

/// Queue of pending processes.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ProcessQueue {
    items: Vec<QueuedProcess>,
    capacity: u8,
}

impl ProcessQueue {
    #[must_use]
    pub const fn new(capacity: u8) -> Self {
        Self {
            items: Vec::new(),
            capacity,
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    #[must_use]
    pub fn is_full(&self) -> bool {
        self.items.len() >= usize::from(self.capacity)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    #[must_use]
    pub const fn capacity(&self) -> u8 {
        self.capacity
    }

    pub fn enqueue(&mut self, process: QueuedProcess) -> bool {
        if self.is_full() {
            return false;
        }
        self.items.push(process);
        self.items.sort();
        true
    }

    pub fn dequeue(&mut self) -> Option<QueuedProcess> {
        if self.items.is_empty() {
            None
        } else {
            Some(self.items.remove(0))
        }
    }

    pub fn peek(&self) -> Option<&QueuedProcess> {
        self.items.first()
    }

    pub fn cancel(&mut self, process_id: ProcessId) -> bool {
        if let Some(idx) = self.items.iter().position(|p| p.process_id == process_id) {
            self.items.remove(idx);
            true
        } else {
            false
        }
    }

    pub fn cancel_all(&mut self) {
        self.items.clear();
    }

    pub fn iter(&self) -> impl Iterator<Item = &QueuedProcess> {
        self.items.iter()
    }
}

/// State of an active process.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ProcessState {
    /// Active process ID (None = idle).
    pub process_id: Option<ProcessId>,
    /// Progress in ticks.
    pub progress: u32,
    /// Total duration for current process.
    pub duration: u32,
    /// Remaining quantity in current batch.
    pub remaining_quantity: u32,
    /// Whether process is paused.
    pub paused: bool,
}

impl ProcessState {
    #[must_use]
    pub fn idle() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn active(process_id: ProcessId, duration: u32, quantity: u32) -> Self {
        Self {
            process_id: Some(process_id),
            progress: 0,
            duration,
            remaining_quantity: quantity,
            paused: false,
        }
    }

    #[must_use]
    pub fn is_idle(&self) -> bool {
        self.process_id.is_none()
    }

    #[must_use]
    pub fn is_active(&self) -> bool {
        self.process_id.is_some() && !self.paused
    }

    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.process_id.is_some() && self.progress >= self.duration
    }

    #[must_use]
    #[expect(clippy::cast_precision_loss)]
    pub fn progress_ratio(&self) -> f32 {
        if self.duration == 0 {
            0.0
        } else {
            (self.progress as f32 / self.duration as f32).clamp(0.0, 1.0)
        }
    }

    pub fn advance(&mut self, ticks: u32) {
        if self.is_active() {
            self.progress = self.progress.saturating_add(ticks);
        }
    }

    pub fn complete_one(&mut self) -> bool {
        if self.remaining_quantity > 0 {
            self.remaining_quantity -= 1;
            self.progress = 0;
            true
        } else {
            false
        }
    }

    pub fn reset(&mut self) {
        *self = Self::idle();
    }

    pub fn pause(&mut self) {
        self.paused = true;
    }

    pub fn resume(&mut self) {
        self.paused = false;
    }
}

/// Maintenance state tracking.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MaintenanceState {
    /// Ticks since last maintenance.
    pub ticks_since_maintenance: u32,
    /// Maintenance interval from config.
    pub interval: u32,
    /// Current wear level (0.0-1.0).
    pub wear: f32,
    /// Whether maintenance is currently required.
    pub required: bool,
}

impl MaintenanceState {
    #[must_use]
    pub fn new(interval: u32) -> Self {
        Self {
            ticks_since_maintenance: 0,
            interval,
            wear: 0.0,
            required: false,
        }
    }

    #[must_use]
    pub fn wear_ratio(&self) -> f32 {
        self.wear.clamp(0.0, 1.0)
    }

    #[must_use]
    pub fn is_overdue(&self) -> bool {
        self.interval > 0 && self.ticks_since_maintenance >= self.interval
    }

    #[expect(clippy::cast_precision_loss)]
    pub fn tick(&mut self, active: bool) {
        if active && self.interval > 0 {
            self.ticks_since_maintenance = self.ticks_since_maintenance.saturating_add(1);
            self.wear = self.ticks_since_maintenance as f32 / self.interval as f32;
            if self.is_overdue() {
                self.required = true;
            }
        }
    }

    pub fn perform_maintenance(&mut self) {
        self.ticks_since_maintenance = 0;
        self.wear = 0.0;
        self.required = false;
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    pub fn apply_tier(&mut self, tier: MachineTier) {
        let adjusted = (self.interval as f32 * tier.maintenance_interval_multiplier()) as u32;
        self.interval = adjusted;
    }
}

/// Resource buffer state (power, heat, fluid).
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BufferState {
    pub current: f32,
    pub max: f32,
}

impl BufferState {
    #[must_use]
    pub const fn new(max: f32) -> Self {
        Self { current: 0.0, max }
    }

    #[must_use]
    pub const fn full(max: f32) -> Self {
        Self { current: max, max }
    }

    #[must_use]
    pub fn ratio(self) -> f32 {
        if self.max <= 0.0 {
            0.0
        } else {
            (self.current / self.max).clamp(0.0, 1.0)
        }
    }

    #[must_use]
    pub fn is_empty(self) -> bool {
        self.current <= 0.0
    }

    #[must_use]
    pub fn is_full(self) -> bool {
        self.current >= self.max
    }

    pub fn add(&mut self, amount: f32) -> f32 {
        let space = self.max - self.current;
        let added = amount.min(space).max(0.0);
        self.current += added;
        added
    }

    pub fn remove(&mut self, amount: f32) -> f32 {
        let removed = amount.min(self.current).max(0.0);
        self.current -= removed;
        removed
    }
}

/// Complete runtime state for a machine instance.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MachineState {
    /// Machine tier.
    pub tier: MachineTier,
    /// Current fault condition.
    pub fault: FaultKind,
    /// Process state.
    pub process: ProcessState,
    /// Process queue.
    pub queue: ProcessQueue,
    /// Maintenance state.
    pub maintenance: MaintenanceState,
    /// Power buffer.
    pub power: BufferState,
    /// Heat buffer.
    pub heat: BufferState,
    /// Fluid port states (current, max).
    pub fluids: Vec<BufferState>,
    /// Last tick processed.
    pub last_tick: u64,
    /// Whether machine is enabled.
    pub enabled: bool,
    /// Total ticks machine has been active.
    pub total_active_ticks: u64,
    /// Total processes completed.
    pub total_completions: u64,
}

impl MachineState {
    #[must_use]
    pub fn new(config: &MachineConfig, tier: MachineTier) -> Self {
        let fluids = config
            .fluid_ports
            .iter()
            .map(|p| BufferState::new(p.capacity * tier.capacity_multiplier()))
            .collect();

        let mut maintenance = MaintenanceState::new(config.maintenance_interval);
        maintenance.apply_tier(tier);

        Self {
            tier,
            fault: FaultKind::None,
            process: ProcessState::idle(),
            queue: ProcessQueue::new(config.queue_capacity),
            maintenance,
            power: BufferState::new(config.power.buffer_capacity * tier.capacity_multiplier()),
            heat: BufferState::new(config.heat.max_temp),
            fluids,
            last_tick: 0,
            enabled: true,
            total_active_ticks: 0,
            total_completions: 0,
        }
    }

    #[must_use]
    pub fn is_idle(&self) -> bool {
        self.process.is_idle() && self.queue.is_empty()
    }

    #[must_use]
    pub fn is_active(&self) -> bool {
        self.enabled && !self.fault.is_fault() && self.process.is_active()
    }

    #[must_use]
    pub fn is_faulted(&self) -> bool {
        self.fault.is_fault()
    }

    #[must_use]
    pub fn can_start_process(&self) -> bool {
        self.enabled && !self.fault.is_fault() && self.process.is_idle()
    }

    pub fn start_process(
        &mut self,
        config: &MachineConfig,
        process_id: ProcessId,
        quantity: u32,
    ) -> bool {
        if !self.can_start_process() {
            return false;
        }

        if let Some(proc) = config.find_process(process_id) {
            if proc.min_tier > self.tier {
                return false;
            }
            let duration = proc.effective_duration(self.tier);
            self.process = ProcessState::active(process_id, duration, quantity);
            true
        } else {
            false
        }
    }

    pub fn enqueue_process(&mut self, process_id: ProcessId, quantity: u32) -> bool {
        self.queue.enqueue(QueuedProcess::new(process_id, quantity))
    }

    pub fn cancel_process(&mut self) {
        self.process.reset();
    }

    pub fn cancel_queued(&mut self, process_id: ProcessId) -> bool {
        self.queue.cancel(process_id)
    }

    pub fn cancel_all(&mut self) {
        self.process.reset();
        self.queue.cancel_all();
    }

    pub fn set_fault(&mut self, fault: FaultKind) {
        self.fault = fault;
        if fault.is_fault() {
            self.process.pause();
        }
    }

    pub fn clear_fault(&mut self) {
        if self.fault.is_recoverable() {
            self.fault = FaultKind::None;
            self.process.resume();
        }
    }

    pub fn perform_maintenance(&mut self) {
        self.maintenance.perform_maintenance();
        if self.fault == FaultKind::MaintenanceRequired {
            self.fault = FaultKind::None;
            self.process.resume();
        }
    }

    pub fn fingerprint(&self) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&[self.tier as u8]);
        hasher.update(&[self.fault as u8]);
        if let Some(pid) = self.process.process_id {
            hasher.update(&[1u8]);
            hasher.update(&pid.0.to_le_bytes());
            hasher.update(&self.process.progress.to_le_bytes());
        } else {
            hasher.update(&[0u8]);
        }
        #[expect(clippy::cast_possible_truncation)]
        let queue_len = self.queue.len() as u32;
        hasher.update(&queue_len.to_le_bytes());
        hasher.update(&self.power.current.to_le_bytes());
        hasher.update(&self.heat.current.to_le_bytes());
        hasher.update(&self.total_completions.to_le_bytes());
        hasher.finalize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::machine::config::MachineConfig;

    fn test_config() -> MachineConfig {
        MachineConfig::processor("Test Processor", 10.0)
    }

    #[test]
    fn fault_kind_properties() {
        assert!(!FaultKind::None.is_fault());
        assert!(FaultKind::NoPower.is_fault());
        assert!(FaultKind::NoPower.is_recoverable());
        assert!(!FaultKind::Damaged.is_recoverable());
        assert!(FaultKind::Damaged.requires_maintenance());
    }

    #[test]
    fn process_queue_operations() {
        let mut queue = ProcessQueue::new(3);
        assert!(queue.is_empty());

        queue.enqueue(QueuedProcess::new(ProcessId::new(1), 5));
        queue.enqueue(QueuedProcess::new(ProcessId::new(2), 3).with_priority(64));
        assert_eq!(queue.len(), 2);

        let next = queue.peek().unwrap();
        assert_eq!(next.process_id, ProcessId::new(2));

        let dequeued = queue.dequeue().unwrap();
        assert_eq!(dequeued.process_id, ProcessId::new(2));
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn process_queue_cancel() {
        let mut queue = ProcessQueue::new(5);
        queue.enqueue(QueuedProcess::new(ProcessId::new(1), 1));
        queue.enqueue(QueuedProcess::new(ProcessId::new(2), 1));

        assert!(queue.cancel(ProcessId::new(1)));
        assert_eq!(queue.len(), 1);
        assert!(!queue.cancel(ProcessId::new(1)));
    }

    #[test]
    fn process_state_progress() {
        let mut state = ProcessState::active(ProcessId::new(1), 100, 2);
        assert!(state.is_active());
        assert!(!state.is_complete());
        assert!((state.progress_ratio() - 0.0).abs() < f32::EPSILON);

        state.advance(50);
        assert!((state.progress_ratio() - 0.5).abs() < f32::EPSILON);

        state.advance(50);
        assert!(state.is_complete());
    }

    #[test]
    fn process_state_batch() {
        let mut state = ProcessState::active(ProcessId::new(1), 10, 3);
        state.advance(10);
        assert!(state.is_complete());

        assert!(state.complete_one());
        assert_eq!(state.remaining_quantity, 2);
        assert_eq!(state.progress, 0);
    }

    #[test]
    fn maintenance_state_tracking() {
        let mut maint = MaintenanceState::new(100);
        assert!(!maint.is_overdue());

        for _ in 0..100 {
            maint.tick(true);
        }
        assert!(maint.is_overdue());
        assert!(maint.required);

        maint.perform_maintenance();
        assert!(!maint.is_overdue());
        assert!(!maint.required);
    }

    #[test]
    fn buffer_state_operations() {
        let mut buffer = BufferState::new(100.0);
        assert!(buffer.is_empty());

        let added = buffer.add(50.0);
        assert!((added - 50.0).abs() < f32::EPSILON);
        assert!((buffer.ratio() - 0.5).abs() < f32::EPSILON);

        let removed = buffer.remove(30.0);
        assert!((removed - 30.0).abs() < f32::EPSILON);
        assert!((buffer.current - 20.0).abs() < f32::EPSILON);
    }

    #[test]
    fn machine_state_lifecycle() {
        let config = test_config();
        let state = MachineState::new(&config, MachineTier::Basic);

        assert!(state.is_idle());
        assert!(!state.is_faulted());
        assert!(state.can_start_process());
    }

    #[test]
    fn machine_state_fault_handling() {
        let config = test_config();
        let mut state = MachineState::new(&config, MachineTier::Basic);

        state.set_fault(FaultKind::NoPower);
        assert!(state.is_faulted());
        assert!(!state.can_start_process());

        state.clear_fault();
        assert!(!state.is_faulted());
    }

    #[test]
    fn machine_state_fingerprint_deterministic() {
        let config = test_config();
        let s1 = MachineState::new(&config, MachineTier::Basic);
        let s2 = MachineState::new(&config, MachineTier::Basic);
        assert_eq!(s1.fingerprint(), s2.fingerprint());
    }

    #[test]
    fn machine_state_fingerprint_changes() {
        let config = test_config();
        let mut state = MachineState::new(&config, MachineTier::Basic);
        let fp1 = state.fingerprint();

        state.power.add(50.0);
        let fp2 = state.fingerprint();
        assert_ne!(fp1, fp2);
    }

    #[test]
    fn serde_machine_state() {
        let config = test_config();
        let state = MachineState::new(&config, MachineTier::Advanced);
        let json = serde_json::to_string(&state).unwrap();
        let recovered: MachineState = serde_json::from_str(&json).unwrap();
        assert_eq!(state, recovered);
    }

    #[test]
    fn serde_process_queue() {
        let mut queue = ProcessQueue::new(5);
        queue.enqueue(QueuedProcess::new(ProcessId::new(1), 3));
        queue.enqueue(QueuedProcess::new(ProcessId::new(2), 5));

        let json = serde_json::to_string(&queue).unwrap();
        let recovered: ProcessQueue = serde_json::from_str(&json).unwrap();
        assert_eq!(queue, recovered);
    }
}
