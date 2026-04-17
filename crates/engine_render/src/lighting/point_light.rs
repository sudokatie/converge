//! Point light implementation for torches, fires, and glowing blocks.

use glam::Vec3;

/// Maximum number of point lights that can be active simultaneously.
pub const MAX_POINT_LIGHTS: usize = 64;

/// A point light source (torch, fire, glowing block).
#[derive(Debug, Clone, Copy)]
pub struct PointLight {
    /// World position of the light.
    pub position: Vec3,
    /// Light color (linear RGB).
    pub color: Vec3,
    /// Light intensity/brightness.
    pub intensity: f32,
    /// Attenuation coefficient (k in 1/(1 + k*d^2)).
    pub attenuation: f32,
    /// Maximum range (for culling, not physically accurate).
    pub radius: f32,
}

impl Default for PointLight {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            color: Vec3::new(1.0, 0.9, 0.7), // Warm torch color
            intensity: 1.0,
            attenuation: 0.09,
            radius: 16.0,
        }
    }
}

impl PointLight {
    /// Create a new point light.
    #[must_use]
    pub fn new(position: Vec3, color: Vec3, intensity: f32) -> Self {
        Self {
            position,
            color,
            intensity,
            ..Default::default()
        }
    }

    /// Create a torch-style light.
    #[must_use]
    pub fn torch(position: Vec3) -> Self {
        Self {
            position,
            color: Vec3::new(1.0, 0.7, 0.4), // Orange-yellow
            intensity: 1.5,
            attenuation: 0.07,
            radius: 14.0,
        }
    }

    /// Create a fire-style light.
    #[must_use]
    pub fn fire(position: Vec3) -> Self {
        Self {
            position,
            color: Vec3::new(1.0, 0.5, 0.1), // Deep orange
            intensity: 2.0,
            attenuation: 0.05,
            radius: 20.0,
        }
    }

    /// Create a glowstone-style light.
    #[must_use]
    pub fn glowing_block(position: Vec3) -> Self {
        Self {
            position,
            color: Vec3::new(1.0, 0.95, 0.8), // Warm white
            intensity: 1.2,
            attenuation: 0.06,
            radius: 15.0,
        }
    }

    /// Calculate light contribution at a given point.
    ///
    /// Returns the light color multiplied by attenuation.
    #[must_use]
    pub fn calculate_contribution(&self, point: Vec3) -> Vec3 {
        let distance = (point - self.position).length();
        if distance > self.radius {
            return Vec3::ZERO;
        }

        // Attenuation: 1 / (1 + k * d^2)
        let attenuation_factor = 1.0 / (1.0 + self.attenuation * distance * distance);

        self.color * self.intensity * attenuation_factor
    }
}

/// Manages active point lights with efficient culling.
#[derive(Debug)]
pub struct PointLightManager {
    lights: Vec<PointLight>,
    /// Sorted indices for distance-based culling.
    active_indices: Vec<usize>,
}

impl Default for PointLightManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PointLightManager {
    /// Create a new point light manager.
    #[must_use]
    pub fn new() -> Self {
        Self {
            lights: Vec::with_capacity(MAX_POINT_LIGHTS * 2),
            active_indices: Vec::with_capacity(MAX_POINT_LIGHTS),
        }
    }

    /// Add a point light. Returns the light's index.
    pub fn add(&mut self, light: PointLight) -> usize {
        let index = self.lights.len();
        self.lights.push(light);
        index
    }

    /// Remove a point light by index.
    pub fn remove(&mut self, index: usize) {
        if index < self.lights.len() {
            self.lights.swap_remove(index);
        }
    }

    /// Get a mutable reference to a light.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut PointLight> {
        self.lights.get_mut(index)
    }

    /// Clear all lights.
    pub fn clear(&mut self) {
        self.lights.clear();
        self.active_indices.clear();
    }

    /// Update active light list, culling by distance from viewer.
    ///
    /// Returns the number of active lights (max `MAX_POINT_LIGHTS`).
    pub fn update_active(&mut self, viewer_position: Vec3) -> usize {
        self.active_indices.clear();

        // Calculate distances and collect indices
        let mut distances: Vec<(usize, f32)> = self
            .lights
            .iter()
            .enumerate()
            .map(|(i, light)| {
                let dist = (light.position - viewer_position).length();
                (i, dist)
            })
            .filter(|(i, dist)| *dist <= self.lights[*i].radius + 50.0) // Early cull far lights
            .collect();

        // Sort by distance
        distances.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        // Take closest MAX_POINT_LIGHTS
        self.active_indices = distances
            .into_iter()
            .take(MAX_POINT_LIGHTS)
            .map(|(i, _)| i)
            .collect();

        self.active_indices.len()
    }

    /// Get active lights for rendering.
    #[must_use]
    pub fn active_lights(&self) -> Vec<&PointLight> {
        self.active_indices
            .iter()
            .filter_map(|&i| self.lights.get(i))
            .collect()
    }

    /// Get raw light data for GPU upload.
    #[must_use]
    pub fn active_light_data(&self) -> Vec<PointLightGpuData> {
        self.active_indices
            .iter()
            .filter_map(|&i| self.lights.get(i))
            .map(|light| PointLightGpuData {
                position: light.position.to_array(),
                radius: light.radius,
                color: light.color.to_array(),
                intensity: light.intensity,
                attenuation: light.attenuation,
                _padding: [0.0; 3],
            })
            .collect()
    }

    /// Total number of lights (including inactive).
    #[must_use]
    pub fn total_count(&self) -> usize {
        self.lights.len()
    }

    /// Number of currently active lights.
    #[must_use]
    pub fn active_count(&self) -> usize {
        self.active_indices.len()
    }
}

/// GPU-friendly point light data.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PointLightGpuData {
    pub position: [f32; 3],
    pub radius: f32,
    pub color: [f32; 3],
    pub intensity: f32,
    pub attenuation: f32,
    pub _padding: [f32; 3],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_point_light_attenuation() {
        let light = PointLight::new(Vec3::ZERO, Vec3::ONE, 1.0);

        // At origin, should be full intensity
        let contrib_origin = light.calculate_contribution(Vec3::ZERO);
        assert_eq!(contrib_origin, Vec3::ONE);

        // Further away, should be less
        let contrib_near = light.calculate_contribution(Vec3::new(2.0, 0.0, 0.0));
        let contrib_far = light.calculate_contribution(Vec3::new(10.0, 0.0, 0.0));
        assert!(contrib_near.length() > contrib_far.length());
    }

    #[test]
    fn test_light_beyond_radius_zero() {
        let light = PointLight {
            radius: 10.0,
            ..Default::default()
        };

        let contrib = light.calculate_contribution(Vec3::new(15.0, 0.0, 0.0));
        assert_eq!(contrib, Vec3::ZERO);
    }

    #[test]
    fn test_manager_max_lights() {
        let mut manager = PointLightManager::new();

        // Add more than max lights
        for i in 0..100 {
            manager.add(PointLight::new(
                Vec3::new(i as f32, 0.0, 0.0),
                Vec3::ONE,
                1.0,
            ));
        }

        manager.update_active(Vec3::ZERO);

        assert!(manager.active_count() <= MAX_POINT_LIGHTS);
    }

    #[test]
    fn test_manager_closest_lights_selected() {
        let mut manager = PointLightManager::new();

        // Add far light
        manager.add(PointLight::new(Vec3::new(100.0, 0.0, 0.0), Vec3::ONE, 1.0));

        // Add close light
        manager.add(PointLight::new(Vec3::new(1.0, 0.0, 0.0), Vec3::ONE, 1.0));

        manager.update_active(Vec3::ZERO);

        let active = manager.active_lights();
        assert!(!active.is_empty());
        // First active light should be the closer one
        assert!(active[0].position.x < 50.0);
    }
}
