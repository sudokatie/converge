// Ghost block placement preview shader
// Renders semi-transparent cube at placement position

struct GhostUniform {
    view_proj: mat4x4<f32>,
    model: mat4x4<f32>,
    color: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> ghost: GhostUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) tint: vec4<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let world_pos = (ghost.model * vec4<f32>(in.position, 1.0)).xyz;
    out.clip_position = ghost.view_proj * vec4<f32>(world_pos, 1.0);
    out.world_pos = world_pos;
    out.tint = ghost.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Simple tinted transparent output
    return in.tint;
}
