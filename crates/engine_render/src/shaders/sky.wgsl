// Procedural sky shader with atmospheric scattering
// Rendered as a full-screen triangle before the voxel pass

struct SkyUniform {
    inv_view_proj: mat4x4<f32>,
    camera_position: vec3<f32>,
    time_of_day: f32,
}

@group(0) @binding(0)
var<uniform> sky: SkyUniform;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) ray_dir: vec3<f32>,
}

// Full-screen triangle from vertex index (no vertex buffer)
@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;

    // Generate full-screen triangle vertices
    // Vertex 0: (-1, -1), Vertex 1: (3, -1), Vertex 2: (-1, 3)
    let x = f32(i32(vertex_index) / 2) * 4.0 - 1.0;
    let y = f32(i32(vertex_index) % 2) * 4.0 - 1.0;
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);

    // Compute ray direction from clip space position
    let ndc = vec4<f32>(x, y, 1.0, 1.0);
    let world_pos = sky.inv_view_proj * ndc;
    out.ray_dir = normalize(world_pos.xyz / world_pos.w - sky.camera_position);

    return out;
}

// Constants for atmospheric scattering
const PI: f32 = 3.14159265359;
const SUN_INTENSITY: f32 = 20.0;
const RAYLEIGH_COEFFICIENT: vec3<f32> = vec3<f32>(5.5e-6, 13.0e-6, 22.4e-6);

// Simple hash for star positions
fn hash(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    p3 = p3 + dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

// Get sun direction from time of day
fn get_sun_direction(time: f32) -> vec3<f32> {
    // Sun moves in a circular arc
    // 0.0 = midnight (below horizon), 0.25 = sunrise (east), 0.5 = noon (top), 0.75 = sunset (west)
    let angle = (time - 0.25) * 2.0 * PI;
    let y = sin(angle);
    let xz = cos(angle);
    return normalize(vec3<f32>(xz, y, 0.3));
}

// Get moon direction (opposite side of sun)
fn get_moon_direction(time: f32) -> vec3<f32> {
    let sun_dir = get_sun_direction(time);
    return -sun_dir;
}

// Rayleigh scattering approximation
fn rayleigh_scattering(cos_theta: f32) -> f32 {
    return 0.75 * (1.0 + cos_theta * cos_theta);
}

// Atmospheric scattering color
fn atmosphere_color(ray_dir: vec3<f32>, sun_dir: vec3<f32>) -> vec3<f32> {
    let cos_theta = dot(ray_dir, sun_dir);
    let sun_height = sun_dir.y;

    // Base sky color (blue during day)
    let day_zenith = vec3<f32>(0.2, 0.4, 0.8);
    let day_horizon = vec3<f32>(0.7, 0.8, 0.95);

    // Sunset/sunrise colors
    let sunset_zenith = vec3<f32>(0.1, 0.15, 0.4);
    let sunset_horizon = vec3<f32>(1.0, 0.4, 0.1);

    // Night colors
    let night_zenith = vec3<f32>(0.01, 0.01, 0.03);
    let night_horizon = vec3<f32>(0.02, 0.02, 0.05);

    // Interpolate based on sun height
    let day_factor = smoothstep(-0.1, 0.3, sun_height);
    let sunset_factor = smoothstep(-0.2, 0.0, sun_height) * smoothstep(0.3, 0.0, sun_height);

    // Zenith and horizon colors
    var zenith = mix(night_zenith, day_zenith, day_factor);
    zenith = mix(zenith, sunset_zenith, sunset_factor);

    var horizon = mix(night_horizon, day_horizon, day_factor);
    horizon = mix(horizon, sunset_horizon, sunset_factor * 2.0);

    // Gradient from horizon to zenith based on ray direction
    let horizon_factor = 1.0 - abs(ray_dir.y);
    let horizon_blend = pow(horizon_factor, 2.0);
    var color = mix(zenith, horizon, horizon_blend);

    // Add Rayleigh scattering effect (bluer away from sun, redder towards)
    let scatter = rayleigh_scattering(cos_theta);
    let scatter_color = RAYLEIGH_COEFFICIENT * scatter * SUN_INTENSITY * max(sun_height, 0.0);
    color = color + scatter_color * 50000.0;

    return color;
}

// Sun disk
fn sun_disk(ray_dir: vec3<f32>, sun_dir: vec3<f32>) -> vec3<f32> {
    let cos_angle = dot(ray_dir, sun_dir);
    let sun_angular_radius = 0.03;
    let sun_glow_radius = 0.15;

    // Hard sun disk
    let sun_disk_factor = smoothstep(cos(sun_angular_radius), 1.0, cos_angle);
    let sun_color = vec3<f32>(1.0, 0.95, 0.8) * SUN_INTENSITY * sun_disk_factor;

    // Soft glow around sun
    let glow_factor = smoothstep(cos(sun_glow_radius), 1.0, cos_angle);
    let glow_color = vec3<f32>(1.0, 0.8, 0.5) * glow_factor * 0.5;

    return sun_color + glow_color;
}

// Moon disk
fn moon_disk(ray_dir: vec3<f32>, moon_dir: vec3<f32>, sun_height: f32) -> vec3<f32> {
    // Only visible at night
    if sun_height > 0.0 {
        return vec3<f32>(0.0);
    }

    let cos_angle = dot(ray_dir, moon_dir);
    let moon_angular_radius = 0.025;

    let moon_factor = smoothstep(cos(moon_angular_radius), 1.0, cos_angle);
    let moon_brightness = smoothstep(0.0, -0.3, sun_height);

    return vec3<f32>(0.8, 0.85, 0.9) * moon_factor * moon_brightness * 2.0;
}

// Stars
fn stars(ray_dir: vec3<f32>, sun_height: f32) -> vec3<f32> {
    // Only visible at night
    if sun_height > 0.1 {
        return vec3<f32>(0.0);
    }

    // Project ray to sphere and use as UV
    let theta = atan2(ray_dir.z, ray_dir.x);
    let phi = asin(ray_dir.y);

    // Scale for star density
    let uv = vec2<f32>(theta, phi) * 100.0;
    let grid = floor(uv);

    // Hash for each grid cell
    let h = hash(grid);

    // Only some cells have stars
    var star: f32 = 0.0;
    if h > 0.97 {
        // Star position within cell
        let star_pos = vec2<f32>(hash(grid + 0.1), hash(grid + 0.2));
        let cell_uv = fract(uv);
        let dist = length(cell_uv - star_pos);

        // Star brightness varies
        let brightness = hash(grid + 0.3);
        star = smoothstep(0.05, 0.0, dist) * brightness;
    }

    // Fade stars as sun rises
    let star_visibility = smoothstep(0.1, -0.1, sun_height);

    // Slight twinkling (using a simple variation)
    let twinkle = 0.8 + 0.2 * sin(h * 1000.0);

    return vec3<f32>(1.0, 1.0, 0.95) * star * star_visibility * twinkle;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let ray_dir = normalize(in.ray_dir);
    let sun_dir = get_sun_direction(sky.time_of_day);
    let moon_dir = get_moon_direction(sky.time_of_day);

    // Atmospheric scattering
    var color = atmosphere_color(ray_dir, sun_dir);

    // Add sun
    color = color + sun_disk(ray_dir, sun_dir);

    // Add moon
    color = color + moon_disk(ray_dir, moon_dir, sun_dir.y);

    // Add stars
    color = color + stars(ray_dir, sun_dir.y);

    // Simple tone mapping
    color = color / (color + vec3<f32>(1.0));

    // Gamma correction
    color = pow(color, vec3<f32>(1.0 / 2.2));

    return vec4<f32>(color, 1.0);
}
