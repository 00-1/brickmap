// Instanced foliage splats (E6) — camera-facing billboards over the meshed terrain.
// One unit quad, instanced; the VS billboards it with the camera right/up from the
// globals and scales by the per-instance size, nudging the top by a cheap wind sway.
// The FS discards outside a disc (alpha-*test*, no blend) and flat-shades, so splats
// write depth and need no sorting. Bright greens glow through the existing bloom.

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
@group(0) @binding(0)
var<uniform> globals: Globals;

struct VsOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) fog: f32,
};

// A unit quad in [-0.5, 0.5]^2 from the vertex index (two triangles, 6 verts).
fn corner(i: u32) -> vec2<f32> {
    var c = array<vec2<f32>, 6>(
        vec2<f32>(-0.5, -0.5), vec2<f32>(0.5, -0.5), vec2<f32>(0.5, 0.5),
        vec2<f32>(-0.5, -0.5), vec2<f32>(0.5, 0.5), vec2<f32>(-0.5, 0.5),
    );
    return c[i];
}

@vertex
fn vs_main(
    @builtin(vertex_index) vid: u32,
    @location(0) offset: vec3<f32>,
    @location(1) size: f32,
    @location(2) color: vec3<f32>,
    @location(3) sway: f32,
) -> VsOut {
    let q = corner(vid);
    let right = globals.cam_right.xyz;
    let up = globals.cam_up.xyz;
    let time = globals.cam_right.w;

    // Ethereal recession: point-rendered things back away from the camera as you close in,
    // so foliage and the misty point-colossi drift off by a few blocks instead of letting you
    // reach them — you can never quite touch the dots. Pushed from the *fixed* instance offset
    // (a function of position, not the displaced result, so there's no feedback oscillation),
    // and in the horizontal plane so they slide away across the ground rather than sinking.
    let to_splat = offset - globals.camera_pos.xyz;
    let flat = vec3<f32>(to_splat.x, 0.0, to_splat.z);
    let fd = length(flat);
    let recede_r = 11.0;  // begins receding within ~11 blocks (horizontal)
    let recede_max = 5.0; // up to ~5 blocks of drift when you're right on top
    let push = recede_max * smoothstep(recede_r, 0.0, fd);
    let center = offset + select(vec3<f32>(0.0), flat / fd, fd > 0.001) * push;

    // Wind: nudge the blade sideways, strongest at the top (q.y > 0), incoherent via the
    // per-splat phase. Cheap sin, no per-frame state.
    let topness = q.y + 0.5; // 0 at base, 1 at top
    let gust = sin(time * 1.7 + sway) * 0.18 * topness * size;

    let world = center
        + right * (q.x * size + gust)
        + up * (q.y * size);

    var out: VsOut;
    out.clip_position = globals.view_proj * vec4<f32>(world, 1.0);
    out.color = color;
    out.uv = q;
    // Distance fog, matching the terrain (params.z = start, .w = end; fog_color = horizon).
    let dist = length(world - globals.camera_pos.xyz);
    out.fog = clamp((dist - globals.params.z) / max(globals.params.w - globals.params.z, 0.001), 0.0, 1.0);
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    // Round mask: discard outside the disc so splats read as points, not squares.
    if (dot(in.uv, in.uv) > 0.25) {
        discard;
    }
    // Distance dissolve (M7, opt-in via cam_up.w): stipple distant foliage away with the
    // terrain, using a screen-locked Bayer threshold ramped by the fog distance.
    if (globals.cam_up.w > 0.5) {
        var bayer = array<f32, 16>(
            0.0, 8.0, 2.0, 10.0,
            12.0, 4.0, 14.0, 6.0,
            3.0, 11.0, 1.0, 9.0,
            15.0, 7.0, 13.0, 5.0,
        );
        let bx = u32(in.clip_position.x) % 4u;
        let by = u32(in.clip_position.y) % 4u;
        let thr = (bayer[by * 4u + bx] + 0.5) / 16.0;
        if (thr < in.fog * 0.85) {
            discard;
        }
    }
    // Slight centre-bright shading so blades aren't flat discs.
    let d = 1.0 - dot(in.uv, in.uv) * 1.2;
    let lit = in.color * (0.7 + 0.3 * d);
    let rgb = mix(lit, globals.fog_color.rgb, in.fog);
    return vec4<f32>(rgb, 1.0);
}
