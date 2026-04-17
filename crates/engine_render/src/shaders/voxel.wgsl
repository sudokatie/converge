// Voxel chunk rendering shader
// Uses view-projection from camera, model matrix from chunk position

struct CameraUniform {
    view_proj: mat4x4<f32>,
    position: vec3<f32>,
    _padding: f32,
}

struct ChunkUniform {
    model: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var<uniform> chunk: ChunkUniform;

@group(2) @binding(0)
var t_diffuse: texture_2d_array<f32>;
@group(2) @binding(1)
var s_diffuse: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: u32,
    @location(2) uv: vec2<f32>,
    @location(3) ao: u32,
}

struct FogUniform {
    color: vec4<f32>,
    density: f32,
    start_distance: f32,
    end_distance: f32,
    enabled: u32,
}

@group(3) @binding(0)
var<uniform> fog: FogUniform;

struct PointLightData {
    position: vec3<f32>,
    _pad0: f32,
    color: vec3<f32>,
    intensity: f32,
    attenuation: f32,
    radius: f32,
    _pad1: vec2<f32>,
};

struct LightUniform {
    dir_direction: vec3<f32>,
    _pad0: f32,
    dir_color: vec3<f32>,
    _pad1: f32,
    dir_light_view_proj: mat4x4<f32>,
    ambient_color: vec3<f32>,
    ambient_intensity: f32,
    num_point_lights: u32,
    _pad2: vec3<u32>,
    point_lights: array<PointLightData, 64>,
};

@group(4) @binding(0)
var<uniform> light: LightUniform;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) ao: f32,
    @location(2) normal: vec3<f32>,
    @location(3) world_pos: vec3<f32>,
}

// Calculate fog factor based on distance from camera
fn fog_factor(world_pos: vec3<f32>) -> f32 {
    if fog.enabled == 0u {
        return 0.0; // No fog
    }
    let dist = distance(camera.position, world_pos);
    if dist <= fog.start_distance {
        return 0.0;
    }
    if dist >= fog.end_distance {
        return fog.density;
    }
    let t = (dist - fog.start_distance) / (fog.end_distance - fog.start_distance);
    return t * fog.density;
}

// Apply fog to a color
fn apply_fog(color: vec3<f32>, world_pos: vec3<f32>) -> vec3<f32> {
    let f = fog_factor(world_pos);
    return mix(color, fog.color.rgb, f);
}

// Normal lookup table (matches normals:: constants)
fn get_normal(index: u32) -> vec3<f32> {
    switch index {
        case 0u: { return vec3<f32>(-1.0, 0.0, 0.0); }  // -X
        case 1u: { return vec3<f32>(1.0, 0.0, 0.0); }   // +X
        case 2u: { return vec3<f32>(0.0, -1.0, 0.0); }  // -Y
        case 3u: { return vec3<f32>(0.0, 1.0, 0.0); }   // +Y
        case 4u: { return vec3<f32>(0.0, 0.0, -1.0); }  // -Z
        case 5u: { return vec3<f32>(0.0, 0.0, 1.0); }   // +Z
        default: { return vec3<f32>(0.0, 1.0, 0.0); }
    }
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    
    let world_pos = (chunk.model * vec4<f32>(in.position, 1.0)).xyz;
    out.clip_position = camera.view_proj * vec4<f32>(world_pos, 1.0);
    out.world_pos = world_pos;
    out.uv = in.uv;
    
    // Convert AO from 0-3 to 0.5-1.0 brightness
    // AO 3 = no occlusion = 1.0, AO 0 = full occlusion = 0.5
    let ao_value = f32(in.ao & 3u);
    out.ao = 0.5 + (ao_value / 6.0);
    
    out.normal = get_normal(in.normal);
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Directional lighting from light uniform
    let light_dir = normalize(light.dir_direction);
    let dir_color = light.dir_color;
    let ndotl = max(dot(in.normal, light_dir), 0.0);
    let ambient = light.ambient_color * light.ambient_intensity;
    let diffuse = dir_color * 0.7 * ndotl;
    
    // Point light contribution
    var point_light_color = vec3<f32>(0.0);
    for (var i = 0u; i < light.num_point_lights; i++) {
        let pl = light.point_lights[i];
        let to_light = pl.position - in.world_pos;
        let dist = length(to_light);
        if dist < pl.radius {
            let atten = pl.intensity / (1.0 + pl.attenuation * dist * dist);
            let ndotl_pl = max(dot(in.normal, normalize(to_light)), 0.0);
            point_light_color += pl.color * ndotl_pl * atten;
        }
    }
    
    let lighting = ambient + diffuse + point_light_color;
    
    // Apply AO
    let final_light = lighting * in.ao;
    
    // Debug: use solid color based on normal (no texture yet)
    var base_color: vec3<f32>;
    if in.normal.y > 0.5 {
        // Top face: grass green
        base_color = vec3<f32>(0.3, 0.6, 0.2);
    } else if in.normal.y < -0.5 {
        // Bottom face: dirt brown
        base_color = vec3<f32>(0.5, 0.35, 0.2);
    } else {
        // Side faces: grass/dirt blend
        base_color = vec3<f32>(0.45, 0.4, 0.25);
    }
    
    return vec4<f32>(apply_fog(base_color * final_light, in.world_pos), 1.0);
}

// Textured variant (for when atlas is ready)
@fragment
fn fs_textured(in: VertexOutput) -> @location(0) vec4<f32> {
    // Sample texture array (layer 0 for now)
    let tex_color = textureSample(t_diffuse, s_diffuse, in.uv, 0);
    
    // Directional lighting from light uniform
    let light_dir = normalize(light.dir_direction);
    let dir_color = light.dir_color;
    let ndotl = max(dot(in.normal, light_dir), 0.0);
    let ambient = light.ambient_color * light.ambient_intensity;
    let diffuse = dir_color * 0.7 * ndotl;
    
    // Point light contribution
    var point_light_color = vec3<f32>(0.0);
    for (var i = 0u; i < light.num_point_lights; i++) {
        let pl = light.point_lights[i];
        let to_light = pl.position - in.world_pos;
        let dist = length(to_light);
        if dist < pl.radius {
            let atten = pl.intensity / (1.0 + pl.attenuation * dist * dist);
            let ndotl_pl = max(dot(in.normal, normalize(to_light)), 0.0);
            point_light_color += pl.color * ndotl_pl * atten;
        }
    }
    
    let lighting = (ambient + diffuse + point_light_color) * in.ao;
    
    return vec4<f32>(apply_fog(tex_color.rgb * lighting, in.world_pos), tex_color.a);
}
