// Instanced emissive cube particles (E2). Shares the globals (view-projection) with
// the chunk shader; each instance carries its world offset, size, and emissive
// colour. No lighting — particles glow.

struct Globals {
    view_proj: mat4x4<f32>,
    palette: array<vec4<f32>, 8>,
};
@group(0) @binding(0)
var<uniform> globals: Globals;

struct VsOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(
    @location(0) cube_pos: vec3<f32>,
    @location(1) offset: vec3<f32>,
    @location(2) size: f32,
    @location(3) color: vec3<f32>,
) -> VsOut {
    let world = cube_pos * size + offset;
    var out: VsOut;
    out.clip_position = globals.view_proj * vec4<f32>(world, 1.0);
    out.color = color;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
