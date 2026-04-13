//! Sound effect definitions and helpers.
//!
//! Defines common sound effects and provides helpers for playing
//! contextual sounds (footsteps by surface, block sounds, etc.).

use glam::Vec3;

use crate::{AudioManager, SoundId, VolumeCategory};

/// Surface type for footstep sounds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SurfaceType {
    /// Grass, dirt, soft ground.
    Grass,
    /// Stone, cobblestone, brick.
    Stone,
    /// Wood planks, logs.
    Wood,
    /// Sand, gravel.
    Sand,
    /// Water (splashing).
    Water,
    /// Snow.
    Snow,
    /// Metal.
    Metal,
}

impl SurfaceType {
    /// Get the footstep sound ID for this surface.
    #[must_use]
    pub fn footstep_sound(&self) -> SoundId {
        match self {
            SurfaceType::Grass => SoundId::FOOTSTEP_GRASS,
            SurfaceType::Stone => SoundId::FOOTSTEP_STONE,
            SurfaceType::Wood => SoundId::FOOTSTEP_WOOD,
            SurfaceType::Sand => SoundId::FOOTSTEP_SAND,
            SurfaceType::Water => SoundId(14), // FOOTSTEP_WATER
            SurfaceType::Snow => SoundId(15),  // FOOTSTEP_SNOW
            SurfaceType::Metal => SoundId(16), // FOOTSTEP_METAL
        }
    }

    /// Get surface type from a block ID.
    #[must_use]
    pub fn from_block_id(block_id: u16) -> Self {
        match block_id {
            0 => SurfaceType::Grass,      // Air (shouldn't happen)
            1 => SurfaceType::Stone,      // Stone
            2 | 3 => SurfaceType::Grass,  // Dirt, Grass
            4 => SurfaceType::Sand,       // Sand
            5 => SurfaceType::Water,      // Water
            6 | 8 => SurfaceType::Wood,   // Oak/Birch Log
            7 | 9 => SurfaceType::Grass,  // Leaves
            11 => SurfaceType::Wood,      // Planks
            12 => SurfaceType::Stone,     // Cobblestone
            13..=16 => SurfaceType::Stone, // Ores
            _ => SurfaceType::Stone,      // Default
        }
    }
}

/// Block sound events.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockSoundEvent {
    /// Block placed.
    Place,
    /// Block broken.
    Break,
    /// Block hit while mining.
    Hit,
}

impl BlockSoundEvent {
    /// Get the sound ID for this event.
    #[must_use]
    pub fn sound_id(&self) -> SoundId {
        match self {
            BlockSoundEvent::Place => SoundId::BLOCK_PLACE,
            BlockSoundEvent::Break => SoundId::BLOCK_BREAK,
            BlockSoundEvent::Hit => SoundId(3), // BLOCK_HIT
        }
    }
}

/// Combat sound events.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatSoundEvent {
    /// Player takes damage.
    PlayerHit,
    /// Creature takes damage.
    CreatureHit,
    /// Attack swing (miss or hit).
    Swing,
    /// Critical hit.
    Critical,
    /// Entity dies.
    Death,
    /// Arrow fired.
    ArrowShoot,
    /// Arrow hits.
    ArrowHit,
}

impl CombatSoundEvent {
    /// Get the sound ID for this event.
    #[must_use]
    pub fn sound_id(&self) -> SoundId {
        match self {
            CombatSoundEvent::PlayerHit => SoundId::HIT_PLAYER,
            CombatSoundEvent::CreatureHit => SoundId::HIT_CREATURE,
            CombatSoundEvent::Swing => SoundId(23),
            CombatSoundEvent::Critical => SoundId(24),
            CombatSoundEvent::Death => SoundId::DEATH,
            CombatSoundEvent::ArrowShoot => SoundId(25),
            CombatSoundEvent::ArrowHit => SoundId(26),
        }
    }
}

/// UI sound events.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiSoundEvent {
    /// Button click.
    Click,
    /// Menu/inventory open.
    Open,
    /// Menu/inventory close.
    Close,
    /// Item pickup.
    Pickup,
    /// Crafting success.
    Craft,
    /// Error/invalid action.
    Error,
    /// Level up or achievement.
    Success,
}

impl UiSoundEvent {
    /// Get the sound ID for this event.
    #[must_use]
    pub fn sound_id(&self) -> SoundId {
        match self {
            UiSoundEvent::Click => SoundId::UI_CLICK,
            UiSoundEvent::Open => SoundId::UI_OPEN,
            UiSoundEvent::Close => SoundId::UI_CLOSE,
            UiSoundEvent::Pickup => SoundId::PICKUP,
            UiSoundEvent::Craft => SoundId(43),
            UiSoundEvent::Error => SoundId(44),
            UiSoundEvent::Success => SoundId(45),
        }
    }
}

/// Ambient sound events.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AmbientSoundEvent {
    /// Cave ambiance.
    Cave,
    /// Wind.
    Wind,
    /// Rain.
    Rain,
    /// Thunder.
    Thunder,
    /// Water flowing.
    WaterFlow,
    /// Lava bubbling.
    Lava,
}

impl AmbientSoundEvent {
    /// Get the sound ID for this event.
    #[must_use]
    pub fn sound_id(&self) -> SoundId {
        match self {
            AmbientSoundEvent::Cave => SoundId(50),
            AmbientSoundEvent::Wind => SoundId(51),
            AmbientSoundEvent::Rain => SoundId(52),
            AmbientSoundEvent::Thunder => SoundId(53),
            AmbientSoundEvent::WaterFlow => SoundId(54),
            AmbientSoundEvent::Lava => SoundId(55),
        }
    }
}

/// Sound effect player with contextual helpers.
pub struct SoundEffects<'a> {
    audio: &'a mut AudioManager,
}

impl<'a> SoundEffects<'a> {
    /// Create a new sound effects helper.
    pub fn new(audio: &'a mut AudioManager) -> Self {
        Self { audio }
    }

    /// Play a footstep sound for the given surface.
    pub fn footstep(&mut self, surface: SurfaceType, position: Vec3) {
        self.audio.play_at(surface.footstep_sound(), position);
    }

    /// Play a block sound event.
    pub fn block(&mut self, event: BlockSoundEvent, position: Vec3) {
        self.audio.play_at(event.sound_id(), position);
    }

    /// Play a combat sound event.
    pub fn combat(&mut self, event: CombatSoundEvent, position: Vec3) {
        self.audio.play_at(event.sound_id(), position);
    }

    /// Play a UI sound event.
    pub fn ui(&mut self, event: UiSoundEvent) {
        self.audio.play_with_category(event.sound_id(), VolumeCategory::Ui);
    }

    /// Play an ambient sound event.
    pub fn ambient(&mut self, event: AmbientSoundEvent, position: Vec3) {
        self.audio.play_at(event.sound_id(), position);
    }

    /// Play eating sound.
    pub fn eat(&mut self) {
        self.audio.play(SoundId::EAT);
    }

    /// Play item pickup sound.
    pub fn pickup(&mut self) {
        self.audio.play_with_category(SoundId::PICKUP, VolumeCategory::Ui);
    }
}

/// Extension trait for AudioManager to easily play effects.
pub trait AudioEffectsExt {
    /// Get a sound effects helper.
    fn effects(&mut self) -> SoundEffects<'_>;
}

impl AudioEffectsExt for AudioManager {
    fn effects(&mut self) -> SoundEffects<'_> {
        SoundEffects::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_surface_footstep_sounds() {
        assert_eq!(SurfaceType::Grass.footstep_sound(), SoundId::FOOTSTEP_GRASS);
        assert_eq!(SurfaceType::Stone.footstep_sound(), SoundId::FOOTSTEP_STONE);
        assert_eq!(SurfaceType::Wood.footstep_sound(), SoundId::FOOTSTEP_WOOD);
    }

    #[test]
    fn test_surface_from_block_id() {
        assert_eq!(SurfaceType::from_block_id(1), SurfaceType::Stone);
        assert_eq!(SurfaceType::from_block_id(2), SurfaceType::Grass);
        assert_eq!(SurfaceType::from_block_id(4), SurfaceType::Sand);
        assert_eq!(SurfaceType::from_block_id(6), SurfaceType::Wood);
    }

    #[test]
    fn test_block_sound_events() {
        assert_eq!(BlockSoundEvent::Place.sound_id(), SoundId::BLOCK_PLACE);
        assert_eq!(BlockSoundEvent::Break.sound_id(), SoundId::BLOCK_BREAK);
    }

    #[test]
    fn test_combat_sound_events() {
        assert_eq!(CombatSoundEvent::PlayerHit.sound_id(), SoundId::HIT_PLAYER);
        assert_eq!(CombatSoundEvent::Death.sound_id(), SoundId::DEATH);
    }

    #[test]
    fn test_ui_sound_events() {
        assert_eq!(UiSoundEvent::Click.sound_id(), SoundId::UI_CLICK);
        assert_eq!(UiSoundEvent::Open.sound_id(), SoundId::UI_OPEN);
        assert_eq!(UiSoundEvent::Pickup.sound_id(), SoundId::PICKUP);
    }

    #[test]
    fn test_ambient_sound_events() {
        // Just verify they have unique IDs
        let cave = AmbientSoundEvent::Cave.sound_id();
        let wind = AmbientSoundEvent::Wind.sound_id();
        assert_ne!(cave, wind);
    }

    #[test]
    fn test_sound_effects_helper() {
        let mut audio = AudioManager::dummy();
        let mut effects = audio.effects();

        // These should not crash on dummy audio
        effects.footstep(SurfaceType::Grass, Vec3::ZERO);
        effects.block(BlockSoundEvent::Place, Vec3::ZERO);
        effects.ui(UiSoundEvent::Click);
        effects.eat();
        effects.pickup();
    }
}
