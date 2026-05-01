//! Resource budget definitions for sandboxed mods.
//!
//! Budgets define limits on resource consumption for mods, enabling
//! enforcement of fair resource sharing and preventing abuse.

use serde::{Deserialize, Serialize};

/// Memory budget configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemoryBudget {
    pub heap_bytes: u64,
    pub stack_bytes: u64,
    pub texture_bytes: u64,
    pub buffer_bytes: u64,
}

impl MemoryBudget {
    #[must_use]
    pub const fn new(heap_bytes: u64) -> Self {
        Self {
            heap_bytes,
            stack_bytes: 1024 * 1024,
            texture_bytes: 64 * 1024 * 1024,
            buffer_bytes: 16 * 1024 * 1024,
        }
    }

    #[must_use]
    pub const fn total_bytes(self) -> u64 {
        self.heap_bytes + self.stack_bytes + self.texture_bytes + self.buffer_bytes
    }

    #[must_use]
    pub const fn with_stack(mut self, bytes: u64) -> Self {
        self.stack_bytes = bytes;
        self
    }

    #[must_use]
    pub const fn with_textures(mut self, bytes: u64) -> Self {
        self.texture_bytes = bytes;
        self
    }

    #[must_use]
    pub const fn with_buffers(mut self, bytes: u64) -> Self {
        self.buffer_bytes = bytes;
        self
    }

    /// Check if this budget fits within another budget.
    #[must_use]
    pub const fn fits_within(self, limit: &Self) -> bool {
        self.heap_bytes <= limit.heap_bytes
            && self.stack_bytes <= limit.stack_bytes
            && self.texture_bytes <= limit.texture_bytes
            && self.buffer_bytes <= limit.buffer_bytes
    }
}

impl Default for MemoryBudget {
    fn default() -> Self {
        Self::new(32 * 1024 * 1024)
    }
}

/// CPU time budget configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CpuBudget {
    pub frame_micros: u64,
    pub tick_micros: u64,
    pub init_micros: u64,
}

impl CpuBudget {
    #[must_use]
    pub const fn new(frame_micros: u64) -> Self {
        Self {
            frame_micros,
            tick_micros: frame_micros * 2,
            init_micros: 1_000_000,
        }
    }

    #[must_use]
    pub const fn with_tick(mut self, micros: u64) -> Self {
        self.tick_micros = micros;
        self
    }

    #[must_use]
    pub const fn with_init(mut self, micros: u64) -> Self {
        self.init_micros = micros;
        self
    }

    /// Check if this budget fits within another budget.
    #[must_use]
    pub const fn fits_within(self, limit: &Self) -> bool {
        self.frame_micros <= limit.frame_micros
            && self.tick_micros <= limit.tick_micros
            && self.init_micros <= limit.init_micros
    }
}

impl Default for CpuBudget {
    fn default() -> Self {
        Self::new(2000)
    }
}

/// Storage budget configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StorageBudget {
    pub data_bytes: u64,
    pub cache_bytes: u64,
    pub max_files: u32,
}

impl StorageBudget {
    #[must_use]
    pub const fn new(data_bytes: u64) -> Self {
        Self {
            data_bytes,
            cache_bytes: data_bytes / 4,
            max_files: 1000,
        }
    }

    #[must_use]
    pub const fn with_cache(mut self, bytes: u64) -> Self {
        self.cache_bytes = bytes;
        self
    }

    #[must_use]
    pub const fn with_max_files(mut self, count: u32) -> Self {
        self.max_files = count;
        self
    }

    /// Check if this budget fits within another budget.
    #[must_use]
    pub const fn fits_within(self, limit: &Self) -> bool {
        self.data_bytes <= limit.data_bytes
            && self.cache_bytes <= limit.cache_bytes
            && self.max_files <= limit.max_files
    }
}

impl Default for StorageBudget {
    fn default() -> Self {
        Self::new(100 * 1024 * 1024)
    }
}

/// Network budget configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NetworkBudget {
    pub bandwidth_bytes_per_sec: u64,
    pub max_connections: u32,
    pub max_requests_per_min: u32,
}

impl NetworkBudget {
    #[must_use]
    pub const fn new(bandwidth_bytes_per_sec: u64) -> Self {
        Self {
            bandwidth_bytes_per_sec,
            max_connections: 10,
            max_requests_per_min: 60,
        }
    }

    #[must_use]
    pub const fn with_connections(mut self, count: u32) -> Self {
        self.max_connections = count;
        self
    }

    #[must_use]
    pub const fn with_request_rate(mut self, per_min: u32) -> Self {
        self.max_requests_per_min = per_min;
        self
    }

    /// Check if this budget fits within another budget.
    #[must_use]
    pub const fn fits_within(self, limit: &Self) -> bool {
        self.bandwidth_bytes_per_sec <= limit.bandwidth_bytes_per_sec
            && self.max_connections <= limit.max_connections
            && self.max_requests_per_min <= limit.max_requests_per_min
    }
}

impl Default for NetworkBudget {
    fn default() -> Self {
        Self::new(1024 * 1024)
    }
}

/// Complete resource budget for a mod.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceBudget {
    #[serde(default)]
    pub memory: MemoryBudget,
    #[serde(default)]
    pub cpu: CpuBudget,
    #[serde(default)]
    pub storage: StorageBudget,
    #[serde(default)]
    pub network: NetworkBudget,
}

impl ResourceBudget {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_memory(mut self, budget: MemoryBudget) -> Self {
        self.memory = budget;
        self
    }

    #[must_use]
    pub fn with_cpu(mut self, budget: CpuBudget) -> Self {
        self.cpu = budget;
        self
    }

    #[must_use]
    pub fn with_storage(mut self, budget: StorageBudget) -> Self {
        self.storage = budget;
        self
    }

    #[must_use]
    pub fn with_network(mut self, budget: NetworkBudget) -> Self {
        self.network = budget;
        self
    }

    /// Check if this budget fits within another budget.
    #[must_use]
    pub fn fits_within(&self, limit: &Self) -> bool {
        self.memory.fits_within(&limit.memory)
            && self.cpu.fits_within(&limit.cpu)
            && self.storage.fits_within(&limit.storage)
            && self.network.fits_within(&limit.network)
    }

    /// Validate a requested budget against a policy limit.
    #[must_use]
    pub fn validate_against(&self, limit: &Self) -> BudgetValidation {
        let mut validation = BudgetValidation::ok();

        if !self.memory.fits_within(&limit.memory) {
            if self.memory.heap_bytes > limit.memory.heap_bytes {
                validation.add_violation(format!(
                    "heap memory {} exceeds limit {}",
                    self.memory.heap_bytes, limit.memory.heap_bytes
                ));
            }
            if self.memory.texture_bytes > limit.memory.texture_bytes {
                validation.add_violation(format!(
                    "texture memory {} exceeds limit {}",
                    self.memory.texture_bytes, limit.memory.texture_bytes
                ));
            }
        }

        if !self.cpu.fits_within(&limit.cpu) && self.cpu.frame_micros > limit.cpu.frame_micros {
            validation.add_violation(format!(
                "frame CPU {}us exceeds limit {}us",
                self.cpu.frame_micros, limit.cpu.frame_micros
            ));
        }

        if !self.storage.fits_within(&limit.storage)
            && self.storage.data_bytes > limit.storage.data_bytes
        {
            validation.add_violation(format!(
                "storage {} exceeds limit {}",
                self.storage.data_bytes, limit.storage.data_bytes
            ));
        }

        if !self.network.fits_within(&limit.network)
            && self.network.bandwidth_bytes_per_sec > limit.network.bandwidth_bytes_per_sec
        {
            validation.add_violation(format!(
                "bandwidth {}/s exceeds limit {}/s",
                self.network.bandwidth_bytes_per_sec, limit.network.bandwidth_bytes_per_sec
            ));
        }

        validation
    }
}

/// Result of validating a budget against limits.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BudgetValidation {
    pub valid: bool,
    pub violations: Vec<String>,
}

impl BudgetValidation {
    #[must_use]
    pub fn ok() -> Self {
        Self {
            valid: true,
            violations: Vec::new(),
        }
    }

    pub fn add_violation(&mut self, msg: impl Into<String>) {
        self.valid = false;
        self.violations.push(msg.into());
    }

    #[must_use]
    pub const fn is_valid(&self) -> bool {
        self.valid
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_budget_fits() {
        let small = MemoryBudget::new(16 * 1024 * 1024);
        let large = MemoryBudget::new(64 * 1024 * 1024);

        assert!(small.fits_within(&large));
        assert!(!large.fits_within(&small));
        assert!(small.fits_within(&small));
    }

    #[test]
    fn cpu_budget_fits() {
        let fast = CpuBudget::new(1000);
        let slow = CpuBudget::new(5000);

        assert!(fast.fits_within(&slow));
        assert!(!slow.fits_within(&fast));
    }

    #[test]
    fn storage_budget_fits() {
        let small = StorageBudget::new(50 * 1024 * 1024);
        let large = StorageBudget::new(200 * 1024 * 1024);

        assert!(small.fits_within(&large));
        assert!(!large.fits_within(&small));
    }

    #[test]
    fn network_budget_fits() {
        let limited = NetworkBudget::new(512 * 1024);
        let unlimited = NetworkBudget::new(10 * 1024 * 1024);

        assert!(limited.fits_within(&unlimited));
        assert!(!unlimited.fits_within(&limited));
    }

    #[test]
    fn resource_budget_validation() {
        let requested = ResourceBudget::new()
            .with_memory(MemoryBudget::new(100 * 1024 * 1024))
            .with_cpu(CpuBudget::new(5000));

        let limit = ResourceBudget::new()
            .with_memory(MemoryBudget::new(50 * 1024 * 1024))
            .with_cpu(CpuBudget::new(2000));

        let validation = requested.validate_against(&limit);
        assert!(!validation.is_valid());
        assert!(!validation.violations.is_empty());
    }

    #[test]
    fn resource_budget_validation_ok() {
        let requested = ResourceBudget::new()
            .with_memory(MemoryBudget::new(32 * 1024 * 1024))
            .with_cpu(CpuBudget::new(1000));

        let limit = ResourceBudget::new()
            .with_memory(MemoryBudget::new(64 * 1024 * 1024))
            .with_cpu(CpuBudget::new(2000));

        let validation = requested.validate_against(&limit);
        assert!(validation.is_valid());
        assert!(validation.violations.is_empty());
    }

    #[test]
    fn budget_serde_roundtrip() {
        let budget = ResourceBudget::new()
            .with_memory(MemoryBudget::new(64 * 1024 * 1024).with_textures(128 * 1024 * 1024))
            .with_cpu(CpuBudget::new(3000));

        let json = serde_json::to_string(&budget).unwrap();
        let restored: ResourceBudget = serde_json::from_str(&json).unwrap();
        assert_eq!(budget, restored);
    }

    #[test]
    fn budget_bincode_roundtrip() {
        let budget = ResourceBudget::new()
            .with_storage(StorageBudget::new(200 * 1024 * 1024))
            .with_network(NetworkBudget::new(5 * 1024 * 1024));

        let bytes = bincode::serialize(&budget).unwrap();
        let restored: ResourceBudget = bincode::deserialize(&bytes).unwrap();
        assert_eq!(budget, restored);
    }
}
