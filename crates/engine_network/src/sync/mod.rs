//! State synchronization for multiplayer.

mod chunk_sync;
mod hazard_sync;
mod interpolation;
mod relevancy;
mod session;

pub use chunk_sync::{ChunkPriority, ChunkRequest, ClientChunkSync, ServerChunkSync};
pub use hazard_sync::{
    ClientHazardSync, HazardSyncMessage, ServerClientHazardState, ServerHazardSync,
};
pub use interpolation::{InterpolatedState, InterpolationBuffer};
pub use relevancy::{
    EntityRelevancyManager, FULL_UPDATE_DISTANCE, MAX_RELEVANCE_DISTANCE, POSITION_ONLY_DISTANCE,
    RelevancyResult, UpdateLevel,
};
pub use session::{
    ClientSessionState, DEFAULT_HEARTBEAT_TIMEOUT_MS, DEFAULT_LEASE_DURATION_MS, ElectionPriority,
    LeaveReason, MigrationReason, MigrationRecord, MigrationToken, PeerId, PeerMembership,
    PeerStatus, RejoinRejection, SessionError, SessionGeneration, SessionManager, SessionMessage,
    SessionSnapshot, TokenValidation,
};
