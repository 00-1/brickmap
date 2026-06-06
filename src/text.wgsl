// In-world text (E17): draw a rasterised glyph texture on a camera-facing billboard at a
// world position, so inscriptions float in the 3D scene. Reuses the shared globals (view
// projection + camera basis). Alpha-tested (no blend, writes depth) and emissive-tinted so
// it glows through bloom and is recoloured by the palette like the rest of the scene.

struct Globals {
    view_proj: mat4x4<f32>,
    palette: array<vec4<f32>, 8>,
    params: vec4<f32>,
    camera_pos: vec4<f32>,
    fog_color: vec4<f32>,
    flags: vec4<f32>,
    cam_right: vec4<f32>, // xyz = camera right; w = wind time
    cam_up: vec4<f32>,    // xyz = camera up
};
@group(0) @binding(0) var<uniform> globals: Globals;

struct Label {
    center: vec4<f32>, // xyz = world centre
    half: vec4<f32>,   // xy = billboard half-extents (world units), aspect-correct
    color: vec4<f32>,  // rgb = emissive tint
};
@group(1) @binding(0) var<uniform> label: Label;
@group(1) @binding(1) var tex: texture_2d<f32>;
@group(1) @binding(2) var samp: sampler;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) fog: f32,
};

fn corner(i: u32) -> vec2<f32> {
    var c = array<vec2<f32>, 6>(
        vec2<f32>(-0.5, -0.5), vec2<f32>(0.5, -0.5), vec2<f32>(0.5, 0.5),
        vec2<f32>(-0.5, -0.5), vec2<f32>(0.5, 0.5), vec2<f32>(-0.5, 0.5),
    );
    return c[i];
}

@vertex
fn vs_main(@builtin(vertex_index) vid: u32) -> VsOut {
    let q = corner(vid);
    let right = globals.cam_right.xyz;
    let up = globals.cam_up.xyz;
    let world = label.center.xyz
        + right * (q.x * 2.0 * label.half.x)
        + up * (q.y * 2.0 * label.half.y);

    var out: VsOut;
    out.pos = globals.view_proj * vec4<f32>(world, 1.0);
    // Flip v so the text reads upright (texture row 0 is the top).
    out.uv = vec2<f32>(q.x + 0.5, 0.5 - q.y);
    let dist = length(world - globals.camera_pos.xyz);
    out.fog = clamp((dist - globals.params.z) / max(globals.params.w - globals.params.z, 0.001), 0.0, 1.0);
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let a = textureSample(tex, samp, in.uv).a;
    if (a < 0.5) {
        discard;
    }
    // Emissive tint, faded into the fog colour at distance like the terrain.
    let rgb = mix(label.color.rgb, globals.fog_color.rgb, in.fog);
    return vec4<f32>(rgb, 1.0);
}
