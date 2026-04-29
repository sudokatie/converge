//! Volumetric effect definitions.
//!
//! Each effect kind has distinct visual properties: particle density,
//! scattering behavior, color tint, and animation parameters.

use bytemuck::{Pod, Zeroable};
use glam::Vec3;

/// Kind of volumetric effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum VolumetricEffectKind {
    /// Underwater fog with caustic light patterns.
    Underwater = 0,
    /// Blizzard with dense snow and reduced visibility.
    Blizzard = 1,
    /// Floating spore particles with subtle glow.
    Spores = 2,
    /// Dust motes visible in light shafts.
    Dust = 3,
    /// Vacuum leak with rushing gas particles.
    VacuumLeak = 4,
}

impl VolumetricEffectKind {
    /// All effect kinds in order.
    pub const ALL: [Self; 5] = [
        Self::Underwater,
        Self::Blizzard,
        Self::Spores,
        Self::Dust,
        Self::VacuumLeak,
    ];
}

/// Configuration for a volumetric fog effect.
///
/// Describes visual properties without GPU-specific details.
#[derive(Debug, Clone, Copy)]
pub struct VolumetricEffect {
    /// Effect type.
    pub kind: VolumetricEffectKind,
    /// Base fog color (linear RGB).
    pub color: Vec3,
    /// Fog density (0.0 = clear, 1.0 = opaque).
    pub density: f32,
    /// Scattering coefficient for light interactions.
    pub scattering: f32,
    /// Absorption coefficient (how much light is absorbed).
    pub absorption: f32,
    /// Particle count per cubic meter (for particle effects).
    pub particle_density: f32,
    /// Animation speed multiplier.
    pub animation_speed: f32,
    /// Whether light shafts are enabled for this effect.
    pub light_shafts_enabled: bool,
}

impl VolumetricEffect {
    /// Create an effect with default parameters for the given kind.
    #[must_use]
    pub fn from_kind(kind: VolumetricEffectKind) -> Self {
        match kind {
            VolumetricEffectKind::Underwater => Self::underwater(),
            VolumetricEffectKind::Blizzard => Self::blizzard(),
            VolumetricEffectKind::Spores => Self::spores(),
            VolumetricEffectKind::Dust => Self::dust(),
            VolumetricEffectKind::VacuumLeak => Self::vacuum_leak(),
        }
    }

    /// Underwater effect with caustic lighting.
    #[must_use]
    pub fn underwater() -> Self {
        Self {
            kind: VolumetricEffectKind::Underwater,
            color: Vec3::new(0.1, 0.3, 0.5),
            density: 0.4,
            scattering: 0.6,
            absorption: 0.3,
            particle_density: 50.0,
            animation_speed: 0.5,
            light_shafts_enabled: true,
        }
    }

    /// Blizzard effect with heavy snow.
    #[must_use]
    pub fn blizzard() -> Self {
        Self {
            kind: VolumetricEffectKind::Blizzard,
            color: Vec3::new(0.9, 0.92, 0.95),
            density: 0.7,
            scattering: 0.8,
            absorption: 0.1,
            particle_density: 500.0,
            animation_speed: 2.0,
            light_shafts_enabled: false,
        }
    }

    /// Spore cloud effect with bioluminescence.
    #[must_use]
    pub fn spores() -> Self {
        Self {
            kind: VolumetricEffectKind::Spores,
            color: Vec3::new(0.2, 0.8, 0.3),
            density: 0.15,
            scattering: 0.4,
            absorption: 0.05,
            particle_density: 30.0,
            animation_speed: 0.3,
            light_shafts_enabled: true,
        }
    }

    /// Dust motes effect for shafts of light.
    #[must_use]
    pub fn dust() -> Self {
        Self {
            kind: VolumetricEffectKind::Dust,
            color: Vec3::new(0.8, 0.75, 0.6),
            density: 0.05,
            scattering: 0.3,
            absorption: 0.02,
            particle_density: 100.0,
            animation_speed: 0.1,
            light_shafts_enabled: true,
        }
    }

    /// Vacuum leak effect with rushing particles.
    #[must_use]
    pub fn vacuum_leak() -> Self {
        Self {
            kind: VolumetricEffectKind::VacuumLeak,
            color: Vec3::new(0.6, 0.7, 0.9),
            density: 0.25,
            scattering: 0.5,
            absorption: 0.15,
            particle_density: 200.0,
            animation_speed: 5.0,
            light_shafts_enabled: false,
        }
    }

    /// Interpolate between two effects.
    #[must_use]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        Self {
            kind: if t < 0.5 { self.kind } else { other.kind },
            color: self.color.lerp(other.color, t),
            density: self.density + (other.density - self.density) * t,
            scattering: self.scattering + (other.scattering - self.scattering) * t,
            absorption: self.absorption + (other.absorption - self.absorption) * t,
            particle_density: self.particle_density
                + (other.particle_density - self.particle_density) * t,
            animation_speed: self.animation_speed
                + (other.animation_speed - self.animation_speed) * t,
            light_shafts_enabled: if t < 0.5 {
                self.light_shafts_enabled
            } else {
                other.light_shafts_enabled
            },
        }
    }

    /// Scale density and particle count.
    #[must_use]
    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.density *= intensity;
        self.particle_density *= intensity;
        self
    }
}

impl Default for VolumetricEffect {
    fn default() -> Self {
        Self::dust()
    }
}

/// GPU-friendly volumetric effect uniform.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct VolumetricEffectUniform {
    /// Effect color (RGB) + density (A).
    pub color_density: [f32; 4],
    /// Scattering, absorption, particle density, animation speed.
    pub params: [f32; 4],
    /// Effect kind (as u32) + light shafts enabled + padding.
    pub flags: [u32; 4],
}

impl From<VolumetricEffect> for VolumetricEffectUniform {
    fn from(effect: VolumetricEffect) -> Self {
        Self {
            color_density: [
                effect.color.x,
                effect.color.y,
                effect.color.z,
                effect.density,
            ],
            params: [
                effect.scattering,
                effect.absorption,
                effect.particle_density,
                effect.animation_speed,
            ],
            flags: [
                effect.kind as u32,
                u32::from(effect.light_shafts_enabled),
                0,
                0,
            ],
        }
    }
}

impl Default for VolumetricEffectUniform {
    fn default() -> Self {
        VolumetricEffect::default().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_all_kinds_have_constructors() {
        for kind in VolumetricEffectKind::ALL {
            let effect = VolumetricEffect::from_kind(kind);
            assert_eq!(effect.kind, kind);
            assert!(effect.density >= 0.0);
            assert!(effect.density <= 1.0);
        }
    }

    #[test]
    fn test_underwater_has_light_shafts() {
        let effect = VolumetricEffect::underwater();
        assert!(effect.light_shafts_enabled);
        assert!(
            effect.color.z > effect.color.x,
            "underwater should be blue-tinted"
        );
    }

    #[test]
    fn test_blizzard_high_density() {
        let effect = VolumetricEffect::blizzard();
        assert!(effect.density > 0.5, "blizzard should have high density");
        assert!(
            effect.particle_density > 100.0,
            "blizzard should have many particles"
        );
    }

    #[test]
    fn test_vacuum_leak_fast_animation() {
        let effect = VolumetricEffect::vacuum_leak();
        assert!(
            effect.animation_speed > 3.0,
            "vacuum leak should animate fast"
        );
    }

    #[test]
    fn test_lerp_endpoints() {
        let a = VolumetricEffect::underwater();
        let b = VolumetricEffect::blizzard();

        let at_a = a.lerp(b, 0.0);
        assert_relative_eq!(at_a.density, a.density, epsilon = 0.001);

        let at_b = a.lerp(b, 1.0);
        assert_relative_eq!(at_b.density, b.density, epsilon = 0.001);
    }

    #[test]
    fn test_lerp_midpoint() {
        let a = VolumetricEffect::dust();
        let b = VolumetricEffect::spores();

        let mid = a.lerp(b, 0.5);
        let expected_density = f32::midpoint(a.density, b.density);
        assert_relative_eq!(mid.density, expected_density, epsilon = 0.001);
    }

    #[test]
    fn test_with_intensity() {
        let base = VolumetricEffect::dust();
        let intense = base.with_intensity(2.0);

        assert_relative_eq!(intense.density, base.density * 2.0, epsilon = 0.001);
        assert_relative_eq!(
            intense.particle_density,
            base.particle_density * 2.0,
            epsilon = 0.001
        );
    }

    #[test]
    fn test_uniform_conversion() {
        let effect = VolumetricEffect::underwater();
        let uniform: VolumetricEffectUniform = effect.into();

        assert_relative_eq!(uniform.color_density[0], effect.color.x, epsilon = 0.001);
        assert_relative_eq!(uniform.color_density[3], effect.density, epsilon = 0.001);
        assert_eq!(uniform.flags[0], VolumetricEffectKind::Underwater as u32);
        assert_eq!(uniform.flags[1], 1); // light shafts enabled
    }

    #[test]
    fn test_uniform_size_aligned() {
        assert_eq!(
            std::mem::size_of::<VolumetricEffectUniform>() % 16,
            0,
            "uniform should be 16-byte aligned for GPU"
        );
    }
}
