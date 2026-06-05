// Spike shader: a single coloured cube, lit by one fixed directional light so its
// faces read as distinct planes as it tumbles. Its job is to exercise the
// cross-platform render path (buffers, bind groups, a depth target, a pipeline),
// not to look pretty.

struct Uniforms {
    mvp: mat4x4<f32>,
    model: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> u: Uniforms;

struct VsOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
};

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
) -> VsOut {
    var out: VsOut;
    out.clip_position = u.mvp * vec4<f32>(position, 1.0);
    out.color = color;
    // `model` is rotation-only, so it carries normals into world space directly.
    out.world_normal = (u.model * vec4<f32>(normal, 0.0)).xyz;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let n = normalize(in.world_normal);
    let light_dir = normalize(vec3<f32>(0.4, 0.8, 0.5));
    let diffuse = max(dot(n, light_dir), 0.0);
    let shade = 0.35 + 0.65 * diffuse;
    return vec4<f32>(in.color * shade, 1.0);
}
