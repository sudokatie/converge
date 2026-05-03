//! Colony logistics system for storage, routing, and resource transfer.

use super::ids::{ResourceId, RouteId, StorageNodeId, TransferId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Resource amount with type.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceAmount {
    pub resource: ResourceId,
    pub quantity: u32,
}

impl ResourceAmount {
    #[must_use]
    pub fn new(resource: impl Into<ResourceId>, quantity: u32) -> Self {
        Self {
            resource: resource.into(),
            quantity,
        }
    }
}

/// Storage node configuration.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StorageNode {
    pub id: StorageNodeId,
    pub name: String,
    pub capacity: u32,
    pub allowed_resources: BTreeSet<ResourceId>,
    pub contents: BTreeMap<ResourceId, u32>,
    pub reserved: BTreeMap<ResourceId, u32>,
    pub priority: i32,
    pub is_input: bool,
    pub is_output: bool,
    pub is_enabled: bool,
}

impl StorageNode {
    #[must_use]
    pub fn new(id: StorageNodeId, name: impl Into<String>, capacity: u32) -> Self {
        Self {
            id,
            name: name.into(),
            capacity,
            allowed_resources: BTreeSet::new(),
            contents: BTreeMap::new(),
            reserved: BTreeMap::new(),
            priority: 0,
            is_input: true,
            is_output: true,
            is_enabled: true,
        }
    }

    #[must_use]
    pub fn with_allowed(mut self, resource: impl Into<ResourceId>) -> Self {
        self.allowed_resources.insert(resource.into());
        self
    }

    #[must_use]
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    #[must_use]
    pub fn input_only(mut self) -> Self {
        self.is_input = true;
        self.is_output = false;
        self
    }

    #[must_use]
    pub fn output_only(mut self) -> Self {
        self.is_input = false;
        self.is_output = true;
        self
    }

    #[must_use]
    pub fn total_stored(&self) -> u32 {
        self.contents.values().sum()
    }

    #[must_use]
    pub fn total_reserved(&self) -> u32 {
        self.reserved.values().sum()
    }

    #[must_use]
    pub fn available_capacity(&self) -> u32 {
        self.capacity.saturating_sub(self.total_stored())
    }

    #[must_use]
    pub fn available_quantity(&self, resource: &ResourceId) -> u32 {
        let stored = self.contents.get(resource).copied().unwrap_or(0);
        let reserved = self.reserved.get(resource).copied().unwrap_or(0);
        stored.saturating_sub(reserved)
    }

    #[must_use]
    pub fn can_accept(&self, resource: &ResourceId, quantity: u32) -> bool {
        if !self.is_input || !self.is_enabled {
            return false;
        }
        if !self.allowed_resources.is_empty() && !self.allowed_resources.contains(resource) {
            return false;
        }
        self.available_capacity() >= quantity
    }

    #[must_use]
    pub fn can_provide(&self, resource: &ResourceId, quantity: u32) -> bool {
        if !self.is_output || !self.is_enabled {
            return false;
        }
        self.available_quantity(resource) >= quantity
    }

    pub fn store(&mut self, resource: &ResourceId, quantity: u32) -> u32 {
        let available = self.available_capacity();
        let to_store = quantity.min(available);
        if to_store > 0 {
            *self.contents.entry(resource.clone()).or_insert(0) += to_store;
        }
        to_store
    }

    pub fn withdraw(&mut self, resource: &ResourceId, quantity: u32) -> u32 {
        let available = self.available_quantity(resource);
        let to_withdraw = quantity.min(available);
        if to_withdraw > 0
            && let Some(stored) = self.contents.get_mut(resource)
        {
            *stored = stored.saturating_sub(to_withdraw);
            if *stored == 0 {
                self.contents.remove(resource);
            }
        }
        to_withdraw
    }

    pub fn reserve(&mut self, resource: &ResourceId, quantity: u32) -> bool {
        if !self.can_provide(resource, quantity) {
            return false;
        }
        *self.reserved.entry(resource.clone()).or_insert(0) += quantity;
        true
    }

    pub fn release_reservation(&mut self, resource: &ResourceId, quantity: u32) {
        if let Some(reserved) = self.reserved.get_mut(resource) {
            *reserved = reserved.saturating_sub(quantity);
            if *reserved == 0 {
                self.reserved.remove(resource);
            }
        }
    }

    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "capacity bounded")]
    pub fn fill_ratio(&self) -> f32 {
        if self.capacity == 0 {
            return 0.0;
        }
        self.total_stored() as f32 / self.capacity as f32
    }

    #[must_use]
    pub fn is_full(&self) -> bool {
        self.total_stored() >= self.capacity
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.total_stored() == 0
    }
}

/// Registry for storage nodes.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct StorageRegistry {
    nodes: BTreeMap<StorageNodeId, StorageNode>,
    next_id: u64,
}

impl StorageRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create(&mut self, name: impl Into<String>, capacity: u32) -> StorageNodeId {
        let id = StorageNodeId::new(self.next_id);
        self.next_id += 1;
        let node = StorageNode::new(id, name, capacity);
        self.nodes.insert(id, node);
        id
    }

    pub fn register(&mut self, node: StorageNode) {
        let id = node.id;
        self.nodes.insert(id, node);
        if id.raw() >= self.next_id {
            self.next_id = id.raw() + 1;
        }
    }

    pub fn remove(&mut self, id: StorageNodeId) -> Option<StorageNode> {
        self.nodes.remove(&id)
    }

    #[must_use]
    pub fn get(&self, id: StorageNodeId) -> Option<&StorageNode> {
        self.nodes.get(&id)
    }

    pub fn get_mut(&mut self, id: StorageNodeId) -> Option<&mut StorageNode> {
        self.nodes.get_mut(&id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &StorageNode> {
        self.nodes.values()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut StorageNode> {
        self.nodes.values_mut()
    }

    #[must_use]
    pub fn count(&self) -> usize {
        self.nodes.len()
    }

    pub fn with_resource(&self, resource: &ResourceId) -> impl Iterator<Item = &StorageNode> {
        self.nodes
            .values()
            .filter(|n| n.contents.contains_key(resource))
    }

    pub fn can_accept(
        &self,
        resource: &ResourceId,
        quantity: u32,
    ) -> impl Iterator<Item = &StorageNode> {
        self.nodes
            .values()
            .filter(move |n| n.can_accept(resource, quantity))
    }

    pub fn can_provide(
        &self,
        resource: &ResourceId,
        quantity: u32,
    ) -> impl Iterator<Item = &StorageNode> {
        self.nodes
            .values()
            .filter(move |n| n.can_provide(resource, quantity))
    }

    #[must_use]
    pub fn total_quantity(&self, resource: &ResourceId) -> u32 {
        self.nodes
            .values()
            .map(|n| n.contents.get(resource).copied().unwrap_or(0))
            .sum()
    }

    #[must_use]
    pub fn total_available(&self, resource: &ResourceId) -> u32 {
        self.nodes
            .values()
            .map(|n| n.available_quantity(resource))
            .sum()
    }

    #[must_use]
    pub fn total_capacity(&self) -> u32 {
        self.nodes.values().map(|n| n.capacity).sum()
    }

    #[must_use]
    pub fn total_stored(&self) -> u32 {
        self.nodes.values().map(StorageNode::total_stored).sum()
    }
}

/// Logistics route between storage nodes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Route {
    pub id: RouteId,
    pub source: StorageNodeId,
    pub destination: StorageNodeId,
    pub cost: u32,
    pub capacity_per_tick: u32,
    pub current_load: u32,
    pub is_enabled: bool,
    pub distance: f32,
}

impl Route {
    #[must_use]
    pub fn new(id: RouteId, source: StorageNodeId, destination: StorageNodeId) -> Self {
        Self {
            id,
            source,
            destination,
            cost: 10,
            capacity_per_tick: 100,
            current_load: 0,
            is_enabled: true,
            distance: 1.0,
        }
    }

    #[must_use]
    pub fn with_cost(mut self, cost: u32) -> Self {
        self.cost = cost;
        self
    }

    #[must_use]
    pub fn with_capacity(mut self, capacity: u32) -> Self {
        self.capacity_per_tick = capacity;
        self
    }

    #[must_use]
    pub fn with_distance(mut self, distance: f32) -> Self {
        self.distance = distance;
        self
    }

    #[must_use]
    pub fn available_capacity(&self) -> u32 {
        self.capacity_per_tick.saturating_sub(self.current_load)
    }

    #[must_use]
    pub fn can_transfer(&self, quantity: u32) -> bool {
        self.is_enabled && self.available_capacity() >= quantity
    }

    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "capacity bounded")]
    pub fn utilization(&self) -> f32 {
        if self.capacity_per_tick == 0 {
            return 0.0;
        }
        self.current_load as f32 / self.capacity_per_tick as f32
    }

    pub fn add_load(&mut self, quantity: u32) {
        self.current_load = self.current_load.saturating_add(quantity);
    }

    pub fn reset_load(&mut self) {
        self.current_load = 0;
    }
}

/// Registry for routes.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RouteRegistry {
    routes: BTreeMap<RouteId, Route>,
    by_source: BTreeMap<StorageNodeId, BTreeSet<RouteId>>,
    by_destination: BTreeMap<StorageNodeId, BTreeSet<RouteId>>,
    next_id: u64,
}

impl RouteRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create(&mut self, source: StorageNodeId, destination: StorageNodeId) -> RouteId {
        let id = RouteId::new(self.next_id);
        self.next_id += 1;
        let route = Route::new(id, source, destination);
        self.by_source.entry(source).or_default().insert(id);
        self.by_destination
            .entry(destination)
            .or_default()
            .insert(id);
        self.routes.insert(id, route);
        id
    }

    pub fn register(&mut self, route: Route) {
        let id = route.id;
        self.by_source.entry(route.source).or_default().insert(id);
        self.by_destination
            .entry(route.destination)
            .or_default()
            .insert(id);
        self.routes.insert(id, route);
        if id.raw() >= self.next_id {
            self.next_id = id.raw() + 1;
        }
    }

    pub fn remove(&mut self, id: RouteId) -> Option<Route> {
        if let Some(route) = self.routes.remove(&id) {
            if let Some(set) = self.by_source.get_mut(&route.source) {
                set.remove(&id);
            }
            if let Some(set) = self.by_destination.get_mut(&route.destination) {
                set.remove(&id);
            }
            Some(route)
        } else {
            None
        }
    }

    #[must_use]
    pub fn get(&self, id: RouteId) -> Option<&Route> {
        self.routes.get(&id)
    }

    pub fn get_mut(&mut self, id: RouteId) -> Option<&mut Route> {
        self.routes.get_mut(&id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Route> {
        self.routes.values()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Route> {
        self.routes.values_mut()
    }

    #[must_use]
    pub fn count(&self) -> usize {
        self.routes.len()
    }

    pub fn from_source(&self, source: StorageNodeId) -> impl Iterator<Item = &Route> {
        self.by_source
            .get(&source)
            .into_iter()
            .flat_map(|ids| ids.iter().filter_map(|id| self.routes.get(id)))
    }

    pub fn to_destination(&self, dest: StorageNodeId) -> impl Iterator<Item = &Route> {
        self.by_destination
            .get(&dest)
            .into_iter()
            .flat_map(|ids| ids.iter().filter_map(|id| self.routes.get(id)))
    }

    pub fn reset_loads(&mut self) {
        for route in self.routes.values_mut() {
            route.reset_load();
        }
    }
}

/// Status of a transfer operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TransferStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

/// Planned resource transfer.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Transfer {
    pub id: TransferId,
    pub resource: ResourceId,
    pub quantity: u32,
    pub source: StorageNodeId,
    pub destination: StorageNodeId,
    pub route: Option<RouteId>,
    pub status: TransferStatus,
    pub transferred: u32,
    pub created_tick: u64,
    pub started_tick: Option<u64>,
    pub completed_tick: Option<u64>,
    pub priority: i32,
}

impl Transfer {
    #[must_use]
    pub fn new(
        id: TransferId,
        resource: impl Into<ResourceId>,
        quantity: u32,
        source: StorageNodeId,
        destination: StorageNodeId,
        created_tick: u64,
    ) -> Self {
        Self {
            id,
            resource: resource.into(),
            quantity,
            source,
            destination,
            route: None,
            status: TransferStatus::Pending,
            transferred: 0,
            created_tick,
            started_tick: None,
            completed_tick: None,
            priority: 0,
        }
    }

    #[must_use]
    pub fn with_route(mut self, route: RouteId) -> Self {
        self.route = Some(route);
        self
    }

    #[must_use]
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    #[must_use]
    pub fn remaining(&self) -> u32 {
        self.quantity.saturating_sub(self.transferred)
    }

    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "quantity bounded")]
    pub fn progress(&self) -> f32 {
        if self.quantity == 0 {
            return 1.0;
        }
        self.transferred as f32 / self.quantity as f32
    }

    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.transferred >= self.quantity
    }

    pub fn start(&mut self, tick: u64) {
        self.status = TransferStatus::InProgress;
        self.started_tick = Some(tick);
    }

    pub fn apply(&mut self, amount: u32) {
        self.transferred = self.transferred.saturating_add(amount);
        if self.is_complete() {
            self.status = TransferStatus::Completed;
        }
    }

    pub fn complete(&mut self, tick: u64) {
        self.status = TransferStatus::Completed;
        self.completed_tick = Some(tick);
    }

    pub fn fail(&mut self, tick: u64) {
        self.status = TransferStatus::Failed;
        self.completed_tick = Some(tick);
    }

    pub fn cancel(&mut self, tick: u64) {
        self.status = TransferStatus::Cancelled;
        self.completed_tick = Some(tick);
    }
}

/// Registry for transfers.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TransferRegistry {
    transfers: BTreeMap<TransferId, Transfer>,
    next_id: u64,
}

impl TransferRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create(
        &mut self,
        resource: impl Into<ResourceId>,
        quantity: u32,
        source: StorageNodeId,
        destination: StorageNodeId,
        created_tick: u64,
    ) -> TransferId {
        let id = TransferId::new(self.next_id);
        self.next_id += 1;
        let transfer = Transfer::new(id, resource, quantity, source, destination, created_tick);
        self.transfers.insert(id, transfer);
        id
    }

    pub fn register(&mut self, transfer: Transfer) {
        let id = transfer.id;
        self.transfers.insert(id, transfer);
        if id.raw() >= self.next_id {
            self.next_id = id.raw() + 1;
        }
    }

    pub fn remove(&mut self, id: TransferId) -> Option<Transfer> {
        self.transfers.remove(&id)
    }

    #[must_use]
    pub fn get(&self, id: TransferId) -> Option<&Transfer> {
        self.transfers.get(&id)
    }

    pub fn get_mut(&mut self, id: TransferId) -> Option<&mut Transfer> {
        self.transfers.get_mut(&id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Transfer> {
        self.transfers.values()
    }

    #[must_use]
    pub fn count(&self) -> usize {
        self.transfers.len()
    }

    pub fn pending(&self) -> impl Iterator<Item = &Transfer> {
        self.transfers
            .values()
            .filter(|t| t.status == TransferStatus::Pending)
    }

    pub fn active(&self) -> impl Iterator<Item = &Transfer> {
        self.transfers
            .values()
            .filter(|t| t.status == TransferStatus::InProgress)
    }

    pub fn for_resource(&self, resource: &ResourceId) -> impl Iterator<Item = &Transfer> {
        self.transfers
            .values()
            .filter(move |t| &t.resource == resource)
    }
}

/// Supply/demand entry for a resource.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ResourceBalance {
    pub supply: u32,
    pub demand: u32,
    pub reserved: u32,
    pub pending_inbound: u32,
    pub pending_outbound: u32,
}

impl ResourceBalance {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn available(&self) -> i32 {
        #[expect(
            clippy::cast_possible_wrap,
            reason = "supply/demand bounded by game limits"
        )]
        {
            (self.supply as i32) - (self.demand as i32) - (self.reserved as i32)
        }
    }

    #[must_use]
    pub fn shortage(&self) -> u32 {
        let avail = self.available();
        if avail < 0 { avail.unsigned_abs() } else { 0 }
    }

    #[must_use]
    pub fn surplus(&self) -> u32 {
        let avail = self.available();
        if avail > 0 {
            #[expect(clippy::cast_sign_loss, reason = "we checked it's positive")]
            {
                avail as u32
            }
        } else {
            0
        }
    }

    #[must_use]
    pub fn is_balanced(&self) -> bool {
        self.shortage() == 0
    }

    #[must_use]
    pub fn projected_supply(&self) -> u32 {
        self.supply.saturating_add(self.pending_inbound)
    }

    #[must_use]
    pub fn projected_demand(&self) -> u32 {
        self.demand.saturating_add(self.pending_outbound)
    }
}

/// Logistics event types.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LogisticsEvent {
    pub tick: u64,
    pub kind: LogisticsEventKind,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum LogisticsEventKind {
    TransferCreated {
        transfer: TransferId,
        resource: ResourceId,
        quantity: u32,
    },
    TransferStarted {
        transfer: TransferId,
    },
    TransferCompleted {
        transfer: TransferId,
        quantity: u32,
    },
    TransferFailed {
        transfer: TransferId,
        reason: String,
    },
    Shortage {
        resource: ResourceId,
        quantity: u32,
    },
    Overflow {
        node: StorageNodeId,
        resource: ResourceId,
        quantity: u32,
    },
    RouteBlocked {
        route: RouteId,
    },
    RouteRestored {
        route: RouteId,
    },
    StorageNodeFull {
        node: StorageNodeId,
    },
    StorageNodeEmpty {
        node: StorageNodeId,
    },
}

impl LogisticsEvent {
    #[must_use]
    pub fn new(tick: u64, kind: LogisticsEventKind) -> Self {
        Self { tick, kind }
    }

    #[must_use]
    pub fn shortage(tick: u64, resource: ResourceId, quantity: u32) -> Self {
        Self::new(tick, LogisticsEventKind::Shortage { resource, quantity })
    }

    #[must_use]
    pub fn overflow(tick: u64, node: StorageNodeId, resource: ResourceId, quantity: u32) -> Self {
        Self::new(
            tick,
            LogisticsEventKind::Overflow {
                node,
                resource,
                quantity,
            },
        )
    }

    #[must_use]
    pub fn transfer_created(
        tick: u64,
        transfer: TransferId,
        resource: ResourceId,
        quantity: u32,
    ) -> Self {
        Self::new(
            tick,
            LogisticsEventKind::TransferCreated {
                transfer,
                resource,
                quantity,
            },
        )
    }

    #[must_use]
    pub fn transfer_completed(tick: u64, transfer: TransferId, quantity: u32) -> Self {
        Self::new(
            tick,
            LogisticsEventKind::TransferCompleted { transfer, quantity },
        )
    }
}

/// Summary of logistics state.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LogisticsSummary {
    pub tick: u64,
    pub total_storage_nodes: u32,
    pub total_routes: u32,
    pub total_transfers: u32,
    pub active_transfers: u32,
    pub total_capacity: u32,
    pub total_stored: u32,
    pub shortages: Vec<(ResourceId, u32)>,
    pub overflows: Vec<(StorageNodeId, u32)>,
}

impl LogisticsSummary {
    #[must_use]
    pub fn new(tick: u64) -> Self {
        Self {
            tick,
            ..Default::default()
        }
    }

    #[must_use]
    #[expect(clippy::cast_precision_loss, reason = "capacity bounded")]
    pub fn utilization(&self) -> f32 {
        if self.total_capacity == 0 {
            return 0.0;
        }
        self.total_stored as f32 / self.total_capacity as f32
    }

    #[must_use]
    pub fn has_shortages(&self) -> bool {
        !self.shortages.is_empty()
    }

    #[must_use]
    pub fn has_overflows(&self) -> bool {
        !self.overflows.is_empty()
    }

    #[must_use]
    pub fn checksum(&self) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&self.tick.to_le_bytes());
        hasher.update(&self.total_storage_nodes.to_le_bytes());
        hasher.update(&self.total_routes.to_le_bytes());
        hasher.update(&self.total_transfers.to_le_bytes());
        hasher.update(&self.total_stored.to_le_bytes());
        hasher.finalize()
    }
}

/// Projection of future logistics state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LogisticsProjection {
    pub base_tick: u64,
    pub projected_tick: u64,
    pub estimated_transfers_completed: u32,
    pub estimated_shortages: Vec<(ResourceId, u32)>,
    pub estimated_utilization: f32,
    pub confidence: f32,
}

impl LogisticsProjection {
    #[must_use]
    pub fn new(base_tick: u64, projected_tick: u64) -> Self {
        Self {
            base_tick,
            projected_tick,
            estimated_transfers_completed: 0,
            estimated_shortages: Vec::new(),
            estimated_utilization: 0.0,
            confidence: 1.0,
        }
    }
}

/// Fingerprint for logistics state verification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LogisticsFingerprint(pub u32);

impl LogisticsFingerprint {
    #[must_use]
    pub fn raw(self) -> u32 {
        self.0
    }
}

impl std::fmt::Display for LogisticsFingerprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "logistics:{:08x}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_node_basic() {
        let node = StorageNode::new(StorageNodeId::new(1), "Warehouse", 1000);

        assert_eq!(node.capacity, 1000);
        assert_eq!(node.total_stored(), 0);
        assert!(node.is_empty());
        assert!(!node.is_full());
    }

    #[test]
    fn test_storage_node_store_withdraw() {
        let mut node = StorageNode::new(StorageNodeId::new(1), "Warehouse", 100);
        let resource = ResourceId::new("iron");

        let stored = node.store(&resource, 50);
        assert_eq!(stored, 50);
        assert_eq!(node.total_stored(), 50);
        assert_eq!(node.available_quantity(&resource), 50);

        let withdrawn = node.withdraw(&resource, 30);
        assert_eq!(withdrawn, 30);
        assert_eq!(node.available_quantity(&resource), 20);
    }

    #[test]
    fn test_storage_node_capacity_limit() {
        let mut node = StorageNode::new(StorageNodeId::new(1), "Small", 50);
        let resource = ResourceId::new("wood");

        let stored = node.store(&resource, 100);
        assert_eq!(stored, 50);
        assert!(node.is_full());
        assert!(!node.can_accept(&resource, 1));
    }

    #[test]
    fn test_storage_node_reservation() {
        let mut node = StorageNode::new(StorageNodeId::new(1), "Test", 100);
        let resource = ResourceId::new("stone");

        node.store(&resource, 50);
        assert!(node.reserve(&resource, 30));
        assert_eq!(node.available_quantity(&resource), 20);

        assert!(!node.reserve(&resource, 25));

        node.release_reservation(&resource, 30);
        assert_eq!(node.available_quantity(&resource), 50);
    }

    #[test]
    fn test_storage_node_allowed_resources() {
        let node = StorageNode::new(StorageNodeId::new(1), "Food Storage", 100)
            .with_allowed("food")
            .with_allowed("water");

        assert!(node.can_accept(&ResourceId::new("food"), 10));
        assert!(!node.can_accept(&ResourceId::new("iron"), 10));
    }

    #[test]
    fn test_storage_registry() {
        let mut registry = StorageRegistry::new();

        let id1 = registry.create("Warehouse 1", 1000);
        let _id2 = registry.create("Warehouse 2", 500);

        assert_eq!(registry.count(), 2);

        let node = registry.get_mut(id1).unwrap();
        node.store(&ResourceId::new("iron"), 100);

        assert_eq!(registry.total_quantity(&ResourceId::new("iron")), 100);
        assert_eq!(registry.total_capacity(), 1500);
    }

    #[test]
    fn test_route_basic() {
        let route = Route::new(
            RouteId::new(1),
            StorageNodeId::new(1),
            StorageNodeId::new(2),
        )
        .with_cost(20)
        .with_capacity(200);

        assert_eq!(route.cost, 20);
        assert_eq!(route.capacity_per_tick, 200);
        assert!(route.can_transfer(100));
    }

    #[test]
    fn test_route_load() {
        let mut route = Route::new(
            RouteId::new(1),
            StorageNodeId::new(1),
            StorageNodeId::new(2),
        )
        .with_capacity(100);

        route.add_load(60);
        assert_eq!(route.available_capacity(), 40);
        assert!((route.utilization() - 0.6).abs() < 0.001);

        route.reset_load();
        assert_eq!(route.current_load, 0);
    }

    #[test]
    fn test_route_registry() {
        let mut registry = RouteRegistry::new();

        let source = StorageNodeId::new(1);
        let dest = StorageNodeId::new(2);

        let _id = registry.create(source, dest);
        assert_eq!(registry.count(), 1);

        let routes_from: Vec<_> = registry.from_source(source).collect();
        assert_eq!(routes_from.len(), 1);

        let routes_to: Vec<_> = registry.to_destination(dest).collect();
        assert_eq!(routes_to.len(), 1);
    }

    #[test]
    fn test_transfer_basic() {
        let transfer = Transfer::new(
            TransferId::new(1),
            "iron",
            100,
            StorageNodeId::new(1),
            StorageNodeId::new(2),
            0,
        );

        assert_eq!(transfer.status, TransferStatus::Pending);
        assert_eq!(transfer.remaining(), 100);
        assert!((transfer.progress() - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_transfer_lifecycle() {
        let mut transfer = Transfer::new(
            TransferId::new(1),
            "wood",
            100,
            StorageNodeId::new(1),
            StorageNodeId::new(2),
            0,
        );

        transfer.start(10);
        assert_eq!(transfer.status, TransferStatus::InProgress);
        assert_eq!(transfer.started_tick, Some(10));

        transfer.apply(50);
        assert_eq!(transfer.transferred, 50);
        assert!((transfer.progress() - 0.5).abs() < 0.001);

        transfer.apply(50);
        assert!(transfer.is_complete());
        assert_eq!(transfer.status, TransferStatus::Completed);
    }

    #[test]
    fn test_transfer_registry() {
        let mut registry = TransferRegistry::new();

        let id = registry.create("iron", 100, StorageNodeId::new(1), StorageNodeId::new(2), 0);

        assert_eq!(registry.count(), 1);
        assert!(registry.pending().count() > 0);

        let transfer = registry.get_mut(id).unwrap();
        transfer.start(10);

        assert!(registry.active().count() > 0);
    }

    #[test]
    fn test_resource_balance() {
        let mut balance = ResourceBalance::new();
        balance.supply = 100;
        balance.demand = 60;
        balance.reserved = 20;

        assert_eq!(balance.available(), 20);
        assert_eq!(balance.surplus(), 20);
        assert_eq!(balance.shortage(), 0);
        assert!(balance.is_balanced());
    }

    #[test]
    fn test_resource_balance_shortage() {
        let mut balance = ResourceBalance::new();
        balance.supply = 50;
        balance.demand = 80;

        assert_eq!(balance.available(), -30);
        assert_eq!(balance.shortage(), 30);
        assert_eq!(balance.surplus(), 0);
        assert!(!balance.is_balanced());
    }

    #[test]
    fn test_logistics_event() {
        let event = LogisticsEvent::shortage(100, ResourceId::new("food"), 50);
        assert_eq!(event.tick, 100);
    }

    #[test]
    fn test_logistics_summary() {
        let mut summary = LogisticsSummary::new(100);
        summary.total_capacity = 1000;
        summary.total_stored = 600;

        assert!((summary.utilization() - 0.6).abs() < 0.001);

        let checksum1 = summary.checksum();
        let checksum2 = summary.checksum();
        assert_eq!(checksum1, checksum2);
    }

    #[test]
    fn test_logistics_projection() {
        let projection = LogisticsProjection::new(0, 100);

        assert_eq!(projection.base_tick, 0);
        assert_eq!(projection.projected_tick, 100);
        assert!((projection.confidence - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_logistics_fingerprint() {
        let fp = LogisticsFingerprint(0x1234_5678);
        assert_eq!(fp.raw(), 0x1234_5678);
        assert_eq!(format!("{fp}"), "logistics:12345678");
    }

    #[test]
    fn test_serde_storage_node() {
        let mut node = StorageNode::new(StorageNodeId::new(1), "Test", 100);
        node.store(&ResourceId::new("iron"), 50);

        let json = serde_json::to_string(&node).unwrap();
        let restored: StorageNode = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.id, node.id);
        assert_eq!(restored.total_stored(), 50);
    }

    #[test]
    fn test_serde_route() {
        let route = Route::new(
            RouteId::new(1),
            StorageNodeId::new(1),
            StorageNodeId::new(2),
        )
        .with_cost(30);

        let json = serde_json::to_string(&route).unwrap();
        let restored: Route = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.id, route.id);
        assert_eq!(restored.cost, 30);
    }

    #[test]
    fn test_serde_transfer() {
        let transfer = Transfer::new(
            TransferId::new(1),
            "wood",
            100,
            StorageNodeId::new(1),
            StorageNodeId::new(2),
            50,
        )
        .with_priority(10);

        let json = serde_json::to_string(&transfer).unwrap();
        let restored: Transfer = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.id, transfer.id);
        assert_eq!(restored.priority, 10);
    }

    #[test]
    fn test_bincode_storage_node() {
        let mut node = StorageNode::new(StorageNodeId::new(42), "Bincode Test", 500);
        node.store(&ResourceId::new("stone"), 100);

        let bytes = bincode::serialize(&node).unwrap();
        let restored: StorageNode = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.id.raw(), 42);
        assert_eq!(restored.capacity, 500);
        assert_eq!(restored.total_stored(), 100);
    }

    #[test]
    fn test_bincode_route() {
        let route = Route::new(
            RouteId::new(99),
            StorageNodeId::new(1),
            StorageNodeId::new(2),
        )
        .with_capacity(250);

        let bytes = bincode::serialize(&route).unwrap();
        let restored: Route = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.id.raw(), 99);
        assert_eq!(restored.capacity_per_tick, 250);
    }

    #[test]
    fn test_bincode_transfer() {
        let transfer = Transfer::new(
            TransferId::new(77),
            "coal",
            200,
            StorageNodeId::new(1),
            StorageNodeId::new(2),
            100,
        );

        let bytes = bincode::serialize(&transfer).unwrap();
        let restored: Transfer = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.id.raw(), 77);
        assert_eq!(restored.quantity, 200);
    }

    #[test]
    fn test_bincode_summary() {
        let mut summary = LogisticsSummary::new(500);
        summary.total_storage_nodes = 10;
        summary.total_routes = 20;

        let bytes = bincode::serialize(&summary).unwrap();
        let restored: LogisticsSummary = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.tick, 500);
        assert_eq!(restored.total_storage_nodes, 10);
    }

    #[test]
    fn test_bincode_projection() {
        let projection = LogisticsProjection::new(100, 500);

        let bytes = bincode::serialize(&projection).unwrap();
        let restored: LogisticsProjection = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.base_tick, 100);
        assert_eq!(restored.projected_tick, 500);
    }

    #[test]
    fn test_bincode_fingerprint() {
        let fp = LogisticsFingerprint(0xABCD_EF01);

        let bytes = bincode::serialize(&fp).unwrap();
        let restored: LogisticsFingerprint = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored.raw(), 0xABCD_EF01);
    }
}
