//! Integration tests for multiplayer synchronization

use engine_network::sync::{SyncManager, SyncMessage, EntityState};
use engine_network::protocol::{Packet, PacketType};
use engine_network::validation::{MovementValidator, ValidationConfig, ValidationResult};

#[test]
fn test_entity_state_sync() {
    let mut sync_manager = SyncManager::new();
    
    // Create entity state
    let state = EntityState {
        entity_id: 1,
        position: (10.0, 64.0, 10.0),
        rotation: (0.0, 90.0),
        velocity: (1.0, 0.0, 0.0),
        timestamp: 1000,
    };
    
    // Queue state update
    sync_manager.queue_state_update(state.clone());
    
    // Generate sync packet
    let packet = sync_manager.generate_sync_packet();
    assert!(packet.is_some());
    
    let packet = packet.unwrap();
    assert_eq!(packet.packet_type(), PacketType::EntitySync);
    
    // Verify packet contains our entity
    let states = packet.decode_entity_states();
    assert_eq!(states.len(), 1);
    assert_eq!(states[0].entity_id, 1);
}

#[test]
fn test_sync_packet_ordering() {
    let mut sync_manager = SyncManager::new();
    
    // Add multiple updates in order
    for i in 0..10 {
        sync_manager.queue_state_update(EntityState {
            entity_id: i,
            position: (i as f64, 64.0, 0.0),
            rotation: (0.0, 0.0),
            velocity: (0.0, 0.0, 0.0),
            timestamp: i as u64 * 100,
        });
    }
    
    // Packet should contain all updates
    let packet = sync_manager.generate_sync_packet().unwrap();
    let states = packet.decode_entity_states();
    assert_eq!(states.len(), 10);
    
    // Verify ordering by timestamp
    for i in 1..states.len() {
        assert!(states[i].timestamp >= states[i - 1].timestamp);
    }
}

#[test]
fn test_movement_validation_accepts_valid_movement() {
    let config = ValidationConfig::default();
    let mut validator = MovementValidator::new(config);
    
    validator.register_player(1, (0.0, 64.0, 0.0).into());
    
    // Normal walking speed (~4 blocks/second)
    std::thread::sleep(std::time::Duration::from_millis(100));
    let result = validator.validate_movement(1, (0.4, 64.0, 0.0).into(), true);
    
    assert!(result.is_valid());
}

#[test]
fn test_movement_validation_rejects_teleport() {
    let config = ValidationConfig::default();
    let mut validator = MovementValidator::new(config);
    
    validator.register_player(1, (0.0, 64.0, 0.0).into());
    
    // Small delay to ensure time passes
    std::thread::sleep(std::time::Duration::from_millis(10));
    
    // Instant teleport across the map
    let result = validator.validate_movement(1, (1000.0, 64.0, 1000.0).into(), true);
    
    assert!(!result.is_valid());
}

#[test]
fn test_sync_handles_player_disconnect() {
    let mut sync_manager = SyncManager::new();
    
    // Add player
    sync_manager.add_player(1);
    
    // Add some state for the player
    sync_manager.queue_state_update(EntityState {
        entity_id: 1,
        position: (0.0, 64.0, 0.0),
        rotation: (0.0, 0.0),
        velocity: (0.0, 0.0, 0.0),
        timestamp: 1000,
    });
    
    // Remove player
    sync_manager.remove_player(1);
    
    // Generate packet - should not include removed player
    let packet = sync_manager.generate_sync_packet();
    if let Some(packet) = packet {
        let states = packet.decode_entity_states();
        assert!(states.is_empty() || states.iter().all(|s| s.entity_id != 1));
    }
}

#[test]
fn test_delta_compression() {
    let mut sync_manager = SyncManager::new();
    sync_manager.enable_delta_compression(true);
    
    // Initial full state
    let state1 = EntityState {
        entity_id: 1,
        position: (10.0, 64.0, 10.0),
        rotation: (0.0, 0.0),
        velocity: (0.0, 0.0, 0.0),
        timestamp: 1000,
    };
    sync_manager.queue_state_update(state1);
    let packet1 = sync_manager.generate_sync_packet().unwrap();
    
    // Small movement - should compress well
    let state2 = EntityState {
        entity_id: 1,
        position: (10.1, 64.0, 10.0),
        rotation: (0.0, 0.0),
        velocity: (0.1, 0.0, 0.0),
        timestamp: 1100,
    };
    sync_manager.queue_state_update(state2);
    let packet2 = sync_manager.generate_sync_packet().unwrap();
    
    // Delta packet should be smaller than full packet
    assert!(packet2.size() <= packet1.size());
}

#[test]
fn test_packet_sequence_numbers() {
    let mut sync_manager = SyncManager::new();
    
    let mut last_seq = 0;
    for _ in 0..5 {
        sync_manager.queue_state_update(EntityState {
            entity_id: 1,
            position: (0.0, 64.0, 0.0),
            rotation: (0.0, 0.0),
            velocity: (0.0, 0.0, 0.0),
            timestamp: 0,
        });
        
        let packet = sync_manager.generate_sync_packet().unwrap();
        assert!(packet.sequence_number() > last_seq);
        last_seq = packet.sequence_number();
    }
}
