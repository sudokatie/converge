//! Network transport layer using renet.

pub mod client;
pub mod server;

pub use client::GameClient;
pub use renet::ClientId;
pub use server::GameServer;

/// Default server port.
pub const DEFAULT_PORT: u16 = 27015;

/// Channel IDs for different message types.
pub mod channels {
    use renet::ChannelConfig;
    
    /// Unreliable channel for frequent updates (inputs, snapshots).
    pub const UNRELIABLE: u8 = 0;
    
    /// Reliable channel for important messages (chat, block changes).
    pub const RELIABLE: u8 = 1;
    
    /// Chunk data channel (reliable, ordered).
    pub const CHUNK: u8 = 2;
    
    /// Get channel configurations for renet.
    pub fn channel_configs() -> Vec<ChannelConfig> {
        vec![
            // Unreliable for inputs/snapshots - high frequency, can drop
            ChannelConfig {
                channel_id: UNRELIABLE,
                max_memory_usage_bytes: 5 * 1024 * 1024,
                send_type: renet::SendType::Unreliable,
            },
            // Reliable ordered for chat, block changes
            ChannelConfig {
                channel_id: RELIABLE,
                max_memory_usage_bytes: 2 * 1024 * 1024,
                send_type: renet::SendType::ReliableOrdered {
                    resend_time: std::time::Duration::from_millis(200),
                },
            },
            // Chunk data - reliable, can be large
            ChannelConfig {
                channel_id: CHUNK,
                max_memory_usage_bytes: 20 * 1024 * 1024,
                send_type: renet::SendType::ReliableOrdered {
                    resend_time: std::time::Duration::from_millis(300),
                },
            },
        ]
    }
}
