//! Co-op session management with host migration and recovery.
//!
//! Provides primitives for deterministic session identity, membership tracking,
//! host election, and session recovery for peer-to-peer co-op worlds.

use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, HashMap, HashSet};

use serde::{Deserialize, Serialize};

/// Unique peer identifier within a session.
pub type PeerId = u64;

/// Session generation/epoch counter for ordering migrations.
pub type SessionGeneration = u32;

/// Default heartbeat timeout in milliseconds.
pub const DEFAULT_HEARTBEAT_TIMEOUT_MS: u64 = 5000;

/// Default lease duration in milliseconds.
pub const DEFAULT_LEASE_DURATION_MS: u64 = 10000;

/// Peer status within the session.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PeerStatus {
    /// Peer is connected and responsive.
    Connected,
    /// Peer missed heartbeats but lease not expired.
    Unresponsive,
    /// Peer disconnected, may rejoin.
    Disconnected,
    /// Peer left permanently.
    Left,
}

impl PeerStatus {
    /// Check if peer is still considered a member.
    #[must_use]
    pub fn is_member(&self) -> bool {
        matches!(
            self,
            Self::Connected | Self::Unresponsive | Self::Disconnected
        )
    }

    /// Check if peer is currently reachable.
    #[must_use]
    pub fn is_reachable(&self) -> bool {
        matches!(self, Self::Connected)
    }
}

/// Host election priority (lower value = higher priority).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ElectionPriority(pub u32);

impl ElectionPriority {
    /// Highest priority (preferred host).
    pub const HIGHEST: Self = Self(0);
    /// Normal priority.
    pub const NORMAL: Self = Self(100);
    /// Lowest priority (last resort).
    pub const LOWEST: Self = Self(u32::MAX);
}

impl Default for ElectionPriority {
    fn default() -> Self {
        Self::NORMAL
    }
}

/// Per-peer membership state.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PeerMembership {
    /// Peer identifier.
    pub peer_id: PeerId,
    /// Display name.
    pub name: String,
    /// Current status.
    pub status: PeerStatus,
    /// Host election priority.
    pub election_priority: ElectionPriority,
    /// Last heartbeat tick.
    pub last_heartbeat_tick: u64,
    /// Tick when peer joined.
    pub joined_tick: u64,
    /// Generation when peer last connected.
    pub last_generation: SessionGeneration,
}

impl PeerMembership {
    /// Create new membership record.
    #[must_use]
    pub fn new(peer_id: PeerId, name: String, tick: u64, generation: SessionGeneration) -> Self {
        Self {
            peer_id,
            name,
            status: PeerStatus::Connected,
            election_priority: ElectionPriority::default(),
            last_heartbeat_tick: tick,
            joined_tick: tick,
            last_generation: generation,
        }
    }

    /// Create with specific election priority.
    #[must_use]
    pub fn with_priority(mut self, priority: ElectionPriority) -> Self {
        self.election_priority = priority;
        self
    }

    /// Update heartbeat timestamp.
    pub fn update_heartbeat(&mut self, tick: u64) {
        self.last_heartbeat_tick = tick;
        if self.status == PeerStatus::Unresponsive {
            self.status = PeerStatus::Connected;
        }
    }

    /// Mark as unresponsive.
    pub fn mark_unresponsive(&mut self) {
        if self.status == PeerStatus::Connected {
            self.status = PeerStatus::Unresponsive;
        }
    }

    /// Mark as disconnected.
    pub fn mark_disconnected(&mut self) {
        if self.status != PeerStatus::Left {
            self.status = PeerStatus::Disconnected;
        }
    }

    /// Mark as left (permanent).
    pub fn mark_left(&mut self) {
        self.status = PeerStatus::Left;
    }

    /// Check if heartbeat has timed out.
    #[must_use]
    pub fn is_heartbeat_timeout(&self, current_tick: u64, timeout_ticks: u64) -> bool {
        current_tick.saturating_sub(self.last_heartbeat_tick) > timeout_ticks
    }
}

/// Migration handoff token for host transition validation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigrationToken {
    /// Session identifier.
    pub session_id: u64,
    /// Generation this token authorizes.
    pub generation: SessionGeneration,
    /// Previous host peer ID.
    pub from_host: PeerId,
    /// New host peer ID.
    pub to_host: PeerId,
    /// Tick when migration was initiated.
    pub migration_tick: u64,
    /// Expiration tick.
    pub expires_tick: u64,
    /// Checksum for validation.
    pub checksum: u64,
}

impl MigrationToken {
    /// Create a new migration token.
    #[must_use]
    pub fn new(
        session_id: u64,
        generation: SessionGeneration,
        from_host: PeerId,
        to_host: PeerId,
        migration_tick: u64,
        validity_ticks: u64,
    ) -> Self {
        let expires_tick = migration_tick.saturating_add(validity_ticks);
        let checksum =
            Self::compute_checksum(session_id, generation, from_host, to_host, migration_tick);
        Self {
            session_id,
            generation,
            from_host,
            to_host,
            migration_tick,
            expires_tick,
            checksum,
        }
    }

    fn compute_checksum(
        session_id: u64,
        generation: SessionGeneration,
        from_host: PeerId,
        to_host: PeerId,
        migration_tick: u64,
    ) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        h = h.wrapping_mul(0x0100_0000_01b3) ^ session_id;
        h = h.wrapping_mul(0x0100_0000_01b3) ^ u64::from(generation);
        h = h.wrapping_mul(0x0100_0000_01b3) ^ from_host;
        h = h.wrapping_mul(0x0100_0000_01b3) ^ to_host;
        h = h.wrapping_mul(0x0100_0000_01b3) ^ migration_tick;
        h
    }

    /// Validate token integrity and check if not expired.
    #[must_use]
    pub fn validate(&self, current_tick: u64) -> TokenValidation {
        let expected = Self::compute_checksum(
            self.session_id,
            self.generation,
            self.from_host,
            self.to_host,
            self.migration_tick,
        );
        if self.checksum != expected {
            return TokenValidation::InvalidChecksum;
        }
        if current_tick > self.expires_tick {
            return TokenValidation::Expired;
        }
        TokenValidation::Valid
    }

    /// Check if token matches expected session and generation.
    #[must_use]
    pub fn matches_session(&self, session_id: u64, expected_generation: SessionGeneration) -> bool {
        self.session_id == session_id && self.generation == expected_generation
    }
}

/// Token validation result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenValidation {
    /// Token is valid.
    Valid,
    /// Token checksum mismatch.
    InvalidChecksum,
    /// Token has expired.
    Expired,
}

impl TokenValidation {
    /// Check if validation passed.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        matches!(self, Self::Valid)
    }
}

/// Session recovery snapshot.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionSnapshot {
    /// Session identifier.
    pub session_id: u64,
    /// Current generation.
    pub generation: SessionGeneration,
    /// Current host peer ID.
    pub host_id: PeerId,
    /// Tick when snapshot was taken.
    pub snapshot_tick: u64,
    /// All peer memberships.
    pub members: Vec<PeerMembership>,
    /// Migration history for validation.
    pub migration_history: Vec<MigrationRecord>,
}

impl SessionSnapshot {
    /// Create empty snapshot.
    #[must_use]
    pub fn empty(session_id: u64) -> Self {
        Self {
            session_id,
            generation: 0,
            host_id: 0,
            snapshot_tick: 0,
            members: Vec::new(),
            migration_history: Vec::new(),
        }
    }

    /// Get connected peer count.
    #[must_use]
    pub fn connected_count(&self) -> usize {
        self.members
            .iter()
            .filter(|m| m.status == PeerStatus::Connected)
            .count()
    }

    /// Get total member count (including disconnected).
    #[must_use]
    pub fn member_count(&self) -> usize {
        self.members.iter().filter(|m| m.status.is_member()).count()
    }
}

/// Record of a completed migration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MigrationRecord {
    /// Generation after migration.
    pub generation: SessionGeneration,
    /// Previous host.
    pub from_host: PeerId,
    /// New host.
    pub to_host: PeerId,
    /// Tick when migration completed.
    pub completed_tick: u64,
    /// Reason for migration.
    pub reason: MigrationReason,
}

/// Reason for host migration.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MigrationReason {
    /// Host disconnected or timed out.
    HostTimeout,
    /// Host voluntarily transferred.
    VoluntaryTransfer,
    /// Host explicitly left session.
    HostLeft,
    /// Session recovery from snapshot.
    Recovery,
}

/// Session sync message for network transmission.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SessionMessage {
    /// Heartbeat from peer.
    Heartbeat { peer_id: PeerId, tick: u64 },
    /// Peer joined notification.
    PeerJoined { membership: PeerMembership },
    /// Peer left notification.
    PeerLeft {
        peer_id: PeerId,
        reason: LeaveReason,
    },
    /// Host migration announcement.
    HostMigration {
        token: MigrationToken,
        new_host_membership: PeerMembership,
    },
    /// Migration acknowledgment.
    MigrationAck {
        peer_id: PeerId,
        generation: SessionGeneration,
    },
    /// Request session snapshot for recovery.
    RequestSnapshot {
        peer_id: PeerId,
        last_known_generation: SessionGeneration,
    },
    /// Session snapshot response.
    Snapshot(SessionSnapshot),
    /// Rejoin request from disconnected peer.
    RejoinRequest {
        peer_id: PeerId,
        name: String,
        last_generation: SessionGeneration,
    },
    /// Rejoin accepted.
    RejoinAccepted {
        peer_id: PeerId,
        current_generation: SessionGeneration,
    },
    /// Rejoin rejected.
    RejoinRejected {
        peer_id: PeerId,
        reason: RejoinRejection,
    },
}

/// Reason for peer leaving.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LeaveReason {
    /// Voluntary leave.
    Voluntary,
    /// Connection timeout.
    Timeout,
    /// Kicked by host.
    Kicked,
}

/// Reason for rejoin rejection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RejoinRejection {
    /// Session no longer exists.
    SessionNotFound,
    /// Peer not recognized.
    UnknownPeer,
    /// Generation too old, state diverged.
    GenerationMismatch,
    /// Session is full.
    SessionFull,
}

/// Session manager for host-side session state.
#[derive(Clone, Debug)]
pub struct SessionManager {
    /// Session identifier.
    session_id: u64,
    /// Current generation.
    generation: SessionGeneration,
    /// Current host peer ID.
    host_id: PeerId,
    /// All peer memberships.
    members: BTreeMap<PeerId, PeerMembership>,
    /// Migration history.
    migration_history: Vec<MigrationRecord>,
    /// Pending migration acknowledgments.
    pending_migration_acks: HashSet<PeerId>,
    /// Current tick.
    current_tick: u64,
    /// Heartbeat timeout in ticks.
    heartbeat_timeout_ticks: u64,
    /// Lease duration in ticks.
    lease_duration_ticks: u64,
    /// Maximum session members.
    max_members: usize,
    /// Maximum migration history to keep.
    max_history: usize,
}

impl SessionManager {
    /// Create a new session manager.
    #[must_use]
    pub fn new(session_id: u64, host_id: PeerId, host_name: String) -> Self {
        let mut members = BTreeMap::new();
        let host_membership =
            PeerMembership::new(host_id, host_name, 0, 0).with_priority(ElectionPriority::HIGHEST);
        members.insert(host_id, host_membership);

        Self {
            session_id,
            generation: 0,
            host_id,
            members,
            migration_history: Vec::new(),
            pending_migration_acks: HashSet::new(),
            current_tick: 0,
            heartbeat_timeout_ticks: 100,
            lease_duration_ticks: 200,
            max_members: 16,
            max_history: 32,
        }
    }

    /// Create from recovery snapshot.
    #[must_use]
    pub fn from_snapshot(snapshot: SessionSnapshot) -> Self {
        let members: BTreeMap<_, _> = snapshot
            .members
            .into_iter()
            .map(|m| (m.peer_id, m))
            .collect();

        Self {
            session_id: snapshot.session_id,
            generation: snapshot.generation,
            host_id: snapshot.host_id,
            members,
            migration_history: snapshot.migration_history,
            pending_migration_acks: HashSet::new(),
            current_tick: snapshot.snapshot_tick,
            heartbeat_timeout_ticks: 100,
            lease_duration_ticks: 200,
            max_members: 16,
            max_history: 32,
        }
    }

    /// Get session identifier.
    #[must_use]
    pub fn session_id(&self) -> u64 {
        self.session_id
    }

    /// Get current generation.
    #[must_use]
    pub fn generation(&self) -> SessionGeneration {
        self.generation
    }

    /// Get current host ID.
    #[must_use]
    pub fn host_id(&self) -> PeerId {
        self.host_id
    }

    /// Get current tick.
    #[must_use]
    pub fn current_tick(&self) -> u64 {
        self.current_tick
    }

    /// Check if peer is the current host.
    #[must_use]
    pub fn is_host(&self, peer_id: PeerId) -> bool {
        self.host_id == peer_id
    }

    /// Set heartbeat timeout in ticks.
    pub fn set_heartbeat_timeout(&mut self, ticks: u64) {
        self.heartbeat_timeout_ticks = ticks;
    }

    /// Set lease duration in ticks.
    pub fn set_lease_duration(&mut self, ticks: u64) {
        self.lease_duration_ticks = ticks;
    }

    /// Set maximum members.
    pub fn set_max_members(&mut self, max: usize) {
        self.max_members = max;
    }

    /// Advance tick and check for timeouts.
    ///
    /// Returns peers that timed out this tick. The host is never timed out.
    pub fn update(&mut self, tick: u64) -> Vec<PeerId> {
        self.current_tick = tick;
        let mut timed_out = Vec::new();

        for (peer_id, member) in &mut self.members {
            if *peer_id == self.host_id {
                continue;
            }
            if member.status == PeerStatus::Connected
                && member.is_heartbeat_timeout(tick, self.heartbeat_timeout_ticks)
            {
                member.mark_unresponsive();
            }
            if member.status == PeerStatus::Unresponsive
                && member.is_heartbeat_timeout(tick, self.lease_duration_ticks)
            {
                member.mark_disconnected();
                timed_out.push(*peer_id);
            }
        }

        timed_out
    }

    /// Record heartbeat from peer.
    pub fn record_heartbeat(&mut self, peer_id: PeerId, tick: u64) -> bool {
        if let Some(member) = self.members.get_mut(&peer_id) {
            member.update_heartbeat(tick);
            true
        } else {
            false
        }
    }

    /// Register a new peer.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError::SessionFull`] if the session has reached its member limit.
    /// Returns [`SessionError::PeerAlreadyExists`] if a peer with the given ID already exists.
    pub fn add_peer(
        &mut self,
        peer_id: PeerId,
        name: String,
    ) -> Result<&PeerMembership, SessionError> {
        if self.members.len() >= self.max_members {
            return Err(SessionError::SessionFull);
        }
        match self.members.entry(peer_id) {
            Entry::Occupied(_) => Err(SessionError::PeerAlreadyExists),
            Entry::Vacant(entry) => {
                let membership =
                    PeerMembership::new(peer_id, name, self.current_tick, self.generation);
                Ok(entry.insert(membership))
            }
        }
    }

    /// Remove a peer permanently.
    pub fn remove_peer(&mut self, peer_id: PeerId) -> Option<PeerMembership> {
        if let Some(mut member) = self.members.remove(&peer_id) {
            member.mark_left();
            Some(member)
        } else {
            None
        }
    }

    /// Get peer membership.
    #[must_use]
    pub fn get_peer(&self, peer_id: PeerId) -> Option<&PeerMembership> {
        self.members.get(&peer_id)
    }

    /// Get mutable peer membership.
    pub fn get_peer_mut(&mut self, peer_id: PeerId) -> Option<&mut PeerMembership> {
        self.members.get_mut(&peer_id)
    }

    /// Get all connected peers.
    pub fn connected_peers(&self) -> impl Iterator<Item = &PeerMembership> {
        self.members
            .values()
            .filter(|m| m.status == PeerStatus::Connected)
    }

    /// Get all members (including disconnected).
    pub fn all_members(&self) -> impl Iterator<Item = &PeerMembership> {
        self.members.values().filter(|m| m.status.is_member())
    }

    /// Get connected peer count.
    #[must_use]
    pub fn connected_count(&self) -> usize {
        self.connected_peers().count()
    }

    /// Get total member count.
    #[must_use]
    pub fn member_count(&self) -> usize {
        self.all_members().count()
    }

    /// Check if host has timed out.
    #[must_use]
    pub fn is_host_timeout(&self) -> bool {
        if let Some(host) = self.members.get(&self.host_id) {
            host.status != PeerStatus::Connected
        } else {
            true
        }
    }

    /// Elect new host deterministically.
    ///
    /// Selection criteria (in order):
    /// 1. Lowest election priority value
    /// 2. Earliest join tick
    /// 3. Lowest peer ID (tie-breaker)
    #[must_use]
    pub fn elect_new_host(&self) -> Option<PeerId> {
        self.members
            .values()
            .filter(|m| m.status == PeerStatus::Connected && m.peer_id != self.host_id)
            .min_by(|a, b| {
                a.election_priority
                    .cmp(&b.election_priority)
                    .then_with(|| a.joined_tick.cmp(&b.joined_tick))
                    .then_with(|| a.peer_id.cmp(&b.peer_id))
            })
            .map(|m| m.peer_id)
    }

    /// Initiate host migration.
    ///
    /// Returns migration token if successful.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError::PeerNotFound`] if the new host is not a session member.
    /// Returns [`SessionError::PeerNotConnected`] if the new host is not currently connected.
    pub fn initiate_migration(
        &mut self,
        new_host: PeerId,
        reason: MigrationReason,
    ) -> Result<MigrationToken, SessionError> {
        if let Some(member) = self.members.get(&new_host) {
            if member.status != PeerStatus::Connected {
                return Err(SessionError::PeerNotConnected);
            }
        } else {
            return Err(SessionError::PeerNotFound);
        }

        let old_host = self.host_id;
        self.generation = self.generation.saturating_add(1);

        let token = MigrationToken::new(
            self.session_id,
            self.generation,
            old_host,
            new_host,
            self.current_tick,
            self.lease_duration_ticks,
        );

        self.pending_migration_acks = self
            .members
            .values()
            .filter(|m| m.status == PeerStatus::Connected)
            .map(|m| m.peer_id)
            .collect();

        let record = MigrationRecord {
            generation: self.generation,
            from_host: old_host,
            to_host: new_host,
            completed_tick: self.current_tick,
            reason,
        };
        self.migration_history.push(record);

        while self.migration_history.len() > self.max_history {
            self.migration_history.remove(0);
        }

        self.host_id = new_host;

        if let Some(member) = self.members.get_mut(&new_host) {
            member.election_priority = ElectionPriority::HIGHEST;
            member.last_generation = self.generation;
        }

        Ok(token)
    }

    /// Record migration acknowledgment from peer.
    pub fn record_migration_ack(&mut self, peer_id: PeerId, generation: SessionGeneration) -> bool {
        if generation == self.generation {
            self.pending_migration_acks.remove(&peer_id);
            if let Some(member) = self.members.get_mut(&peer_id) {
                member.last_generation = generation;
            }
            true
        } else {
            false
        }
    }

    /// Check if all peers acknowledged migration.
    #[must_use]
    pub fn is_migration_complete(&self) -> bool {
        self.pending_migration_acks.is_empty()
    }

    /// Get peers pending migration ack.
    pub fn pending_migration_peers(&self) -> impl Iterator<Item = PeerId> + '_ {
        self.pending_migration_acks.iter().copied()
    }

    /// Handle rejoin request from disconnected peer.
    ///
    /// # Errors
    ///
    /// Returns [`RejoinRejection::UnknownPeer`] if the peer is not recognized or has left.
    /// Returns [`RejoinRejection::GenerationMismatch`] if the peer's generation is too stale.
    pub fn handle_rejoin(
        &mut self,
        peer_id: PeerId,
        name: String,
        last_generation: SessionGeneration,
    ) -> Result<(), RejoinRejection> {
        let member = self.members.get_mut(&peer_id);

        match member {
            None => Err(RejoinRejection::UnknownPeer),
            Some(m) if m.status == PeerStatus::Left => Err(RejoinRejection::UnknownPeer),
            Some(_) if self.generation.saturating_sub(last_generation) > 5 => {
                Err(RejoinRejection::GenerationMismatch)
            }
            Some(m) => {
                m.status = PeerStatus::Connected;
                m.name = name;
                m.last_heartbeat_tick = self.current_tick;
                m.last_generation = self.generation;
                Ok(())
            }
        }
    }

    /// Validate a migration token.
    #[must_use]
    pub fn validate_token(&self, token: &MigrationToken) -> TokenValidation {
        if !token.matches_session(self.session_id, self.generation) {
            return TokenValidation::InvalidChecksum;
        }
        token.validate(self.current_tick)
    }

    /// Create recovery snapshot.
    #[must_use]
    pub fn create_snapshot(&self) -> SessionSnapshot {
        SessionSnapshot {
            session_id: self.session_id,
            generation: self.generation,
            host_id: self.host_id,
            snapshot_tick: self.current_tick,
            members: self.members.values().cloned().collect(),
            migration_history: self.migration_history.clone(),
        }
    }
}

/// Client-side session state tracker.
#[derive(Clone, Debug)]
pub struct ClientSessionState {
    /// Session identifier.
    session_id: u64,
    /// Last known generation.
    generation: SessionGeneration,
    /// Current host ID.
    host_id: PeerId,
    /// Own peer ID.
    local_peer_id: PeerId,
    /// Known members.
    members: HashMap<PeerId, PeerMembership>,
    /// Last heartbeat sent tick.
    last_heartbeat_sent: u64,
    /// Heartbeat interval in ticks.
    heartbeat_interval: u64,
    /// Pending migration token to acknowledge.
    pending_migration: Option<MigrationToken>,
}

impl ClientSessionState {
    /// Create new client session state.
    #[must_use]
    pub fn new(session_id: u64, local_peer_id: PeerId, host_id: PeerId) -> Self {
        Self {
            session_id,
            generation: 0,
            host_id,
            local_peer_id,
            members: HashMap::new(),
            last_heartbeat_sent: 0,
            heartbeat_interval: 20,
            pending_migration: None,
        }
    }

    /// Create from snapshot.
    #[must_use]
    pub fn from_snapshot(snapshot: &SessionSnapshot, local_peer_id: PeerId) -> Self {
        let members: HashMap<_, _> = snapshot
            .members
            .iter()
            .map(|m| (m.peer_id, m.clone()))
            .collect();

        Self {
            session_id: snapshot.session_id,
            generation: snapshot.generation,
            host_id: snapshot.host_id,
            local_peer_id,
            members,
            last_heartbeat_sent: snapshot.snapshot_tick,
            heartbeat_interval: 20,
            pending_migration: None,
        }
    }

    /// Get session ID.
    #[must_use]
    pub fn session_id(&self) -> u64 {
        self.session_id
    }

    /// Get current generation.
    #[must_use]
    pub fn generation(&self) -> SessionGeneration {
        self.generation
    }

    /// Get current host ID.
    #[must_use]
    pub fn host_id(&self) -> PeerId {
        self.host_id
    }

    /// Get local peer ID.
    #[must_use]
    pub fn local_peer_id(&self) -> PeerId {
        self.local_peer_id
    }

    /// Check if local peer is host.
    #[must_use]
    pub fn is_local_host(&self) -> bool {
        self.local_peer_id == self.host_id
    }

    /// Set heartbeat interval.
    pub fn set_heartbeat_interval(&mut self, ticks: u64) {
        self.heartbeat_interval = ticks;
    }

    /// Check if heartbeat should be sent.
    ///
    /// Returns true if interval has elapsed since last heartbeat, or if no heartbeat
    /// has been sent yet (`last_heartbeat_sent` == 0 and `current_tick` < interval).
    #[must_use]
    pub fn should_send_heartbeat(&self, current_tick: u64) -> bool {
        if self.last_heartbeat_sent == 0 && current_tick < self.heartbeat_interval {
            return true;
        }
        current_tick.saturating_sub(self.last_heartbeat_sent) >= self.heartbeat_interval
    }

    /// Mark heartbeat as sent.
    pub fn mark_heartbeat_sent(&mut self, tick: u64) {
        self.last_heartbeat_sent = tick;
    }

    /// Handle peer joined.
    pub fn on_peer_joined(&mut self, membership: PeerMembership) {
        self.members.insert(membership.peer_id, membership);
    }

    /// Handle peer left.
    pub fn on_peer_left(&mut self, peer_id: PeerId) {
        self.members.remove(&peer_id);
    }

    /// Handle host migration.
    pub fn on_migration(&mut self, token: MigrationToken, new_host: PeerMembership) {
        if token.validate(self.last_heartbeat_sent).is_valid()
            && token.session_id == self.session_id
        {
            self.generation = token.generation;
            self.host_id = token.to_host;
            self.members.insert(new_host.peer_id, new_host);
            self.pending_migration = Some(token);
        }
    }

    /// Get pending migration ack if any.
    #[must_use]
    pub fn pending_migration_ack(&self) -> Option<SessionGeneration> {
        self.pending_migration.as_ref().map(|t| t.generation)
    }

    /// Clear pending migration after ack sent.
    pub fn clear_pending_migration(&mut self) {
        self.pending_migration = None;
    }

    /// Apply full snapshot.
    pub fn apply_snapshot(&mut self, snapshot: SessionSnapshot) {
        self.session_id = snapshot.session_id;
        self.generation = snapshot.generation;
        self.host_id = snapshot.host_id;
        self.members = snapshot
            .members
            .into_iter()
            .map(|m| (m.peer_id, m))
            .collect();
    }

    /// Get known member.
    #[must_use]
    pub fn get_member(&self, peer_id: PeerId) -> Option<&PeerMembership> {
        self.members.get(&peer_id)
    }

    /// Get all known members.
    pub fn members(&self) -> impl Iterator<Item = &PeerMembership> {
        self.members.values()
    }

    /// Get member count.
    #[must_use]
    pub fn member_count(&self) -> usize {
        self.members.len()
    }
}

/// Session operation errors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionError {
    /// Session is full.
    SessionFull,
    /// Peer already exists.
    PeerAlreadyExists,
    /// Peer not found.
    PeerNotFound,
    /// Peer not connected.
    PeerNotConnected,
    /// Invalid migration token.
    InvalidToken,
    /// Generation mismatch.
    GenerationMismatch,
}

impl std::fmt::Display for SessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SessionFull => write!(f, "session is full"),
            Self::PeerAlreadyExists => write!(f, "peer already exists"),
            Self::PeerNotFound => write!(f, "peer not found"),
            Self::PeerNotConnected => write!(f, "peer not connected"),
            Self::InvalidToken => write!(f, "invalid migration token"),
            Self::GenerationMismatch => write!(f, "generation mismatch"),
        }
    }
}

impl std::error::Error for SessionError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_session() -> SessionManager {
        SessionManager::new(12345, 1, "Host".to_string())
    }

    #[test]
    fn session_creation() {
        let session = make_session();
        assert_eq!(session.session_id(), 12345);
        assert_eq!(session.generation(), 0);
        assert_eq!(session.host_id(), 1);
        assert_eq!(session.connected_count(), 1);
        assert!(session.is_host(1));
    }

    #[test]
    fn add_and_remove_peers() {
        let mut session = make_session();

        session.add_peer(2, "Alice".to_string()).unwrap();
        session.add_peer(3, "Bob".to_string()).unwrap();

        assert_eq!(session.connected_count(), 3);
        assert!(session.get_peer(2).is_some());

        let removed = session.remove_peer(2).unwrap();
        assert_eq!(removed.name, "Alice");
        assert_eq!(removed.status, PeerStatus::Left);
        assert_eq!(session.connected_count(), 2);
    }

    #[test]
    fn session_full_error() {
        let mut session = make_session();
        session.set_max_members(2);

        session.add_peer(2, "Alice".to_string()).unwrap();
        let result = session.add_peer(3, "Bob".to_string());

        assert_eq!(result.unwrap_err(), SessionError::SessionFull);
    }

    #[test]
    fn heartbeat_timeout() {
        let mut session = make_session();
        session.set_heartbeat_timeout(10);
        session.set_lease_duration(20);
        session.add_peer(2, "Alice".to_string()).unwrap();

        let timed_out = session.update(5);
        assert!(timed_out.is_empty());
        assert_eq!(session.get_peer(2).unwrap().status, PeerStatus::Connected);

        session.record_heartbeat(1, 5);

        let timed_out = session.update(16);
        assert!(timed_out.is_empty());
        assert_eq!(
            session.get_peer(2).unwrap().status,
            PeerStatus::Unresponsive
        );

        let timed_out = session.update(26);
        assert_eq!(timed_out, vec![2]);
        assert_eq!(
            session.get_peer(2).unwrap().status,
            PeerStatus::Disconnected
        );
    }

    #[test]
    fn heartbeat_recovery() {
        let mut session = make_session();
        session.set_heartbeat_timeout(10);
        session.add_peer(2, "Alice".to_string()).unwrap();

        session.update(15);
        assert_eq!(
            session.get_peer(2).unwrap().status,
            PeerStatus::Unresponsive
        );

        session.record_heartbeat(2, 16);
        assert_eq!(session.get_peer(2).unwrap().status, PeerStatus::Connected);
    }

    #[test]
    fn deterministic_host_election() {
        let mut session = make_session();

        session.add_peer(5, "Eve".to_string()).unwrap();
        session.get_peer_mut(5).unwrap().election_priority = ElectionPriority(50);

        session.add_peer(3, "Carol".to_string()).unwrap();
        session.get_peer_mut(3).unwrap().election_priority = ElectionPriority(50);

        session.add_peer(2, "Bob".to_string()).unwrap();
        session.get_peer_mut(2).unwrap().election_priority = ElectionPriority(100);

        let elected = session.elect_new_host().unwrap();
        assert_eq!(elected, 3);
    }

    #[test]
    fn host_election_by_join_order() {
        let mut session = make_session();

        session.add_peer(2, "Alice".to_string()).unwrap();
        session.update(10);

        session.add_peer(3, "Bob".to_string()).unwrap();

        let elected = session.elect_new_host().unwrap();
        assert_eq!(elected, 2);
    }

    #[test]
    fn host_election_excludes_current_host() {
        let mut session = make_session();
        session.add_peer(2, "Alice".to_string()).unwrap();

        let elected = session.elect_new_host().unwrap();
        assert_eq!(elected, 2);
        assert_ne!(elected, session.host_id());
    }

    #[test]
    fn migration_token_creation_and_validation() {
        let mut session = make_session();
        session.add_peer(2, "Alice".to_string()).unwrap();

        let token = session
            .initiate_migration(2, MigrationReason::VoluntaryTransfer)
            .unwrap();

        assert_eq!(token.from_host, 1);
        assert_eq!(token.to_host, 2);
        assert_eq!(token.generation, 1);
        assert!(token.validate(session.current_tick()).is_valid());

        assert_eq!(session.host_id(), 2);
        assert_eq!(session.generation(), 1);
    }

    #[test]
    fn migration_token_expiration() {
        let token = MigrationToken::new(100, 1, 1, 2, 50, 10);

        assert!(token.validate(55).is_valid());
        assert_eq!(token.validate(61), TokenValidation::Expired);
    }

    #[test]
    fn migration_token_checksum() {
        let mut token = MigrationToken::new(100, 1, 1, 2, 50, 100);
        assert!(token.validate(60).is_valid());

        token.to_host = 3;
        assert_eq!(token.validate(60), TokenValidation::InvalidChecksum);
    }

    #[test]
    fn migration_ack_tracking() {
        let mut session = make_session();
        session.add_peer(2, "Alice".to_string()).unwrap();
        session.add_peer(3, "Bob".to_string()).unwrap();

        session
            .initiate_migration(2, MigrationReason::HostTimeout)
            .unwrap();

        assert!(!session.is_migration_complete());
        assert_eq!(session.pending_migration_peers().count(), 3);

        session.record_migration_ack(1, 1);
        session.record_migration_ack(2, 1);
        assert!(!session.is_migration_complete());

        session.record_migration_ack(3, 1);
        assert!(session.is_migration_complete());
    }

    #[test]
    fn stale_migration_ack_rejected() {
        let mut session = make_session();
        session.add_peer(2, "Alice".to_string()).unwrap();

        session
            .initiate_migration(2, MigrationReason::VoluntaryTransfer)
            .unwrap();

        let accepted = session.record_migration_ack(1, 0);
        assert!(!accepted);

        let accepted = session.record_migration_ack(1, 1);
        assert!(accepted);
    }

    #[test]
    fn rejoin_flow() {
        let mut session = make_session();
        session.add_peer(2, "Alice".to_string()).unwrap();

        session.get_peer_mut(2).unwrap().mark_disconnected();

        let result = session.handle_rejoin(2, "Alice2".to_string(), 0);
        assert!(result.is_ok());
        assert_eq!(session.get_peer(2).unwrap().status, PeerStatus::Connected);
        assert_eq!(session.get_peer(2).unwrap().name, "Alice2");
    }

    #[test]
    fn rejoin_unknown_peer_rejected() {
        let mut session = make_session();

        let result = session.handle_rejoin(99, "Unknown".to_string(), 0);
        assert_eq!(result.unwrap_err(), RejoinRejection::UnknownPeer);
    }

    #[test]
    fn rejoin_stale_generation_rejected() {
        let mut session = make_session();
        session.add_peer(2, "Alice".to_string()).unwrap();
        session.add_peer(3, "Bob".to_string()).unwrap();

        for _ in 0..6 {
            let new_host = session.elect_new_host().unwrap();
            session
                .initiate_migration(new_host, MigrationReason::VoluntaryTransfer)
                .unwrap();
        }

        session.get_peer_mut(2).unwrap().mark_disconnected();

        let result = session.handle_rejoin(2, "Alice".to_string(), 0);
        assert_eq!(result.unwrap_err(), RejoinRejection::GenerationMismatch);
    }

    #[test]
    fn snapshot_roundtrip() {
        let mut session = make_session();
        session.add_peer(2, "Alice".to_string()).unwrap();
        session.add_peer(3, "Bob".to_string()).unwrap();
        session
            .initiate_migration(2, MigrationReason::VoluntaryTransfer)
            .unwrap();
        session.update(100);

        let snapshot = session.create_snapshot();

        assert_eq!(snapshot.session_id, 12345);
        assert_eq!(snapshot.generation, 1);
        assert_eq!(snapshot.host_id, 2);
        assert_eq!(snapshot.connected_count(), 3);
        assert_eq!(snapshot.migration_history.len(), 1);

        let restored = SessionManager::from_snapshot(snapshot);
        assert_eq!(restored.session_id(), 12345);
        assert_eq!(restored.generation(), 1);
        assert_eq!(restored.host_id(), 2);
        assert_eq!(restored.connected_count(), 3);
    }

    #[test]
    fn client_session_state() {
        let mut client = ClientSessionState::new(100, 2, 1);

        assert_eq!(client.session_id(), 100);
        assert_eq!(client.local_peer_id(), 2);
        assert_eq!(client.host_id(), 1);
        assert!(!client.is_local_host());

        let peer = PeerMembership::new(3, "Bob".to_string(), 0, 0);
        client.on_peer_joined(peer);
        assert_eq!(client.member_count(), 1);

        client.on_peer_left(3);
        assert_eq!(client.member_count(), 0);
    }

    #[test]
    fn client_heartbeat_timing() {
        let mut client = ClientSessionState::new(100, 2, 1);
        client.set_heartbeat_interval(10);

        assert!(client.should_send_heartbeat(0));
        client.mark_heartbeat_sent(5);
        assert!(!client.should_send_heartbeat(10));
        assert!(client.should_send_heartbeat(15));
    }

    #[test]
    fn client_migration_handling() {
        let mut client = ClientSessionState::new(100, 2, 1);

        let token = MigrationToken::new(100, 1, 1, 3, 50, 1000);
        let new_host = PeerMembership::new(3, "NewHost".to_string(), 50, 1);

        client.on_migration(token, new_host);

        assert_eq!(client.generation(), 1);
        assert_eq!(client.host_id(), 3);
        assert!(client.pending_migration_ack().is_some());

        client.clear_pending_migration();
        assert!(client.pending_migration_ack().is_none());
    }

    #[test]
    fn serde_roundtrip_session_message() {
        let membership = PeerMembership::new(2, "Alice".to_string(), 100, 0);
        let msg = SessionMessage::PeerJoined { membership };

        let serialized = bincode::serialize(&msg).unwrap();
        let deserialized: SessionMessage = bincode::deserialize(&serialized).unwrap();

        if let SessionMessage::PeerJoined { membership } = deserialized {
            assert_eq!(membership.peer_id, 2);
            assert_eq!(membership.name, "Alice");
        } else {
            panic!("expected PeerJoined variant");
        }
    }

    #[test]
    fn serde_roundtrip_migration_token() {
        let token = MigrationToken::new(12345, 5, 1, 2, 500, 100);

        let serialized = bincode::serialize(&token).unwrap();
        let deserialized: MigrationToken = bincode::deserialize(&serialized).unwrap();

        assert_eq!(deserialized.session_id, 12345);
        assert_eq!(deserialized.generation, 5);
        assert_eq!(deserialized.from_host, 1);
        assert_eq!(deserialized.to_host, 2);
        assert!(deserialized.validate(500).is_valid());
    }

    #[test]
    fn serde_roundtrip_snapshot() {
        let mut session = make_session();
        session.add_peer(2, "Alice".to_string()).unwrap();
        session
            .initiate_migration(2, MigrationReason::HostTimeout)
            .unwrap();

        let snapshot = session.create_snapshot();
        let serialized = bincode::serialize(&snapshot).unwrap();
        let deserialized: SessionSnapshot = bincode::deserialize(&serialized).unwrap();

        assert_eq!(deserialized.session_id, snapshot.session_id);
        assert_eq!(deserialized.generation, snapshot.generation);
        assert_eq!(deserialized.members.len(), snapshot.members.len());
        assert_eq!(deserialized.migration_history.len(), 1);
    }

    #[test]
    fn peer_status_transitions() {
        let mut member = PeerMembership::new(1, "Test".to_string(), 0, 0);

        assert!(member.status.is_member());
        assert!(member.status.is_reachable());

        member.mark_unresponsive();
        assert!(member.status.is_member());
        assert!(!member.status.is_reachable());

        member.update_heartbeat(10);
        assert_eq!(member.status, PeerStatus::Connected);

        member.mark_disconnected();
        assert!(member.status.is_member());
        assert!(!member.status.is_reachable());

        member.mark_left();
        assert!(!member.status.is_member());
    }

    #[test]
    fn election_priority_ordering() {
        assert!(ElectionPriority::HIGHEST < ElectionPriority::NORMAL);
        assert!(ElectionPriority::NORMAL < ElectionPriority::LOWEST);
        assert!(ElectionPriority(10) < ElectionPriority(20));
    }

    #[test]
    fn migration_to_disconnected_peer_fails() {
        let mut session = make_session();
        session.add_peer(2, "Alice".to_string()).unwrap();
        session.get_peer_mut(2).unwrap().mark_disconnected();

        let result = session.initiate_migration(2, MigrationReason::VoluntaryTransfer);
        assert_eq!(result.unwrap_err(), SessionError::PeerNotConnected);
    }

    #[test]
    fn migration_to_unknown_peer_fails() {
        let mut session = make_session();

        let result = session.initiate_migration(99, MigrationReason::VoluntaryTransfer);
        assert_eq!(result.unwrap_err(), SessionError::PeerNotFound);
    }

    #[test]
    fn multiple_migrations_increment_generation() {
        let mut session = make_session();
        session.add_peer(2, "Alice".to_string()).unwrap();
        session.add_peer(3, "Bob".to_string()).unwrap();

        session
            .initiate_migration(2, MigrationReason::VoluntaryTransfer)
            .unwrap();
        assert_eq!(session.generation(), 1);
        assert_eq!(session.host_id(), 2);

        session
            .initiate_migration(3, MigrationReason::VoluntaryTransfer)
            .unwrap();
        assert_eq!(session.generation(), 2);
        assert_eq!(session.host_id(), 3);

        session
            .initiate_migration(1, MigrationReason::VoluntaryTransfer)
            .unwrap();
        assert_eq!(session.generation(), 3);
        assert_eq!(session.host_id(), 1);
    }

    #[test]
    fn migration_history_limited() {
        let mut session = make_session();
        session.max_history = 3;
        session.add_peer(2, "Alice".to_string()).unwrap();
        session.add_peer(3, "Bob".to_string()).unwrap();

        for _ in 0..5 {
            let new_host = session.elect_new_host().unwrap();
            session
                .initiate_migration(new_host, MigrationReason::VoluntaryTransfer)
                .unwrap();
        }

        assert_eq!(session.migration_history.len(), 3);
        assert_eq!(session.migration_history[0].generation, 3);
    }

    #[test]
    fn client_from_snapshot() {
        let mut session = make_session();
        session.add_peer(2, "Alice".to_string()).unwrap();
        session.add_peer(3, "Bob".to_string()).unwrap();
        session.update(100);

        let snapshot = session.create_snapshot();
        let client = ClientSessionState::from_snapshot(&snapshot, 3);

        assert_eq!(client.session_id(), snapshot.session_id);
        assert_eq!(client.generation(), snapshot.generation);
        assert_eq!(client.local_peer_id(), 3);
        assert_eq!(client.member_count(), 3);
    }
}
