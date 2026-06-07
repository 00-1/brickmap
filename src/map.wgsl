// Explored-world map (E10): a fullscreen 2D overlay. The CPU keeps a texture with one texel per
// visited chunk, coloured by that chunk's biome (so it reads as a biome map with smooth
// transitions). This shader samples it with a pan/zoom in *chunk* space, fills unexplored area
// with a dim background, and draws a blinking "you are here" dot at the camera's chunk.

struct MapU {
    origin_dims: vec4<f32>, // xy = texture origin chunk (min cx, cz); zw = texture size (chunks)
    view: vec4<f32>,        // xy = pan centre (chunk coords); z = chunks per screen height; w = aspect
    user: vec4<f32>,        // xy = user chunk (fractional); z = blink 0..1; w unused
};
@group(0) @binding(0) var<uniform> u: MapU;
@group(0) @binding(1) var tex: texture_2d<f32>;
@group(0) @binding(2) var samp: sampler;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vid: u32) -> VsOut {
    // One oversized triangle covering the screen; uv is 0..1 across the viewport.
    var p = array<vec2<f32>, 3>(vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0));
    var out: VsOut;
    out.pos = vec4<f32>(p[vid], 0.0, 1.0);
    out.uv = p[vid] * 0.5 + 0.5;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let origin = u.origin_dims.xy;
    let dims = max(u.origin_dims.zw, vec2<f32>(1.0, 1.0));
    let cps_y = u.view.z;
    let cps_x = cps_y * u.view.w;
    // Screen → chunk coords (uv.y up; +z reads downward so north is up).
    let chunk = vec2<f32>(
        u.view.x + (in.uv.x - 0.5) * cps_x,
        u.view.y - (in.uv.y - 0.5) * cps_y,
    );

    var col = vec3<f32>(0.03, 0.04, 0.06); // unexplored / outside
    let tc = (chunk - origin) / dims;
    if (tc.x >= 0.0 && tc.x < 1.0 && tc.y >= 0.0 && tc.y < 1.0) {
        let s = textureSample(tex, samp, tc);
        if (s.a > 0.5) {
            col = s.rgb;
        } else {
            col = vec3<f32>(0.06, 0.07, 0.10); // inside the explored bbox but this chunk unseen
        }
    }

    // Faint chunk grid so the scale reads.
    let g = abs(fract(chunk) - vec2<f32>(0.5));
    if (cps_y < 90.0 && (g.x > 0.47 || g.y > 0.47)) {
        col = col * 0.82;
    }

    // Blinking "you are here" dot (size in chunk units, scaled a little with zoom).
    let d = distance(chunk, u.user.xy);
    let r = max(0.6, cps_y * 0.012);
    if (d < r) {
        col = mix(col, vec3<f32>(1.0, 0.88, 0.30), u.user.z);
    }
    return vec4<f32>(col, 1.0);
}
