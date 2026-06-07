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
    let cell = fract(chunk) - vec2<f32>(0.5); // position within this chunk cell, −0.5..0.5
    let tc = (chunk - origin) / dims;
    if (tc.x >= 0.0 && tc.x < 1.0 && tc.y >= 0.0 && tc.y < 1.0) {
        let s = textureSample(tex, samp, tc);
        if (s.a < 0.06) {
            col = vec3<f32>(0.06, 0.07, 0.10); // inside the explored bbox but this chunk unseen
        } else {
            col = s.rgb; // biome colour
            // Marker icons (alpha codes): text ≈160/255, pristine ≈96/255. Drawn as distinct
            // *shapes* within the cell so they don't read as the same dot.
            let dd = length(cell);
            if (s.a > 0.55 && s.a < 0.72 && dd < 0.30) {
                col = vec3<f32>(0.45, 0.85, 1.0); // text: filled cyan dot
            }
            let dia = abs(cell.x) + abs(cell.y);
            if (s.a > 0.30 && s.a < 0.45 && dia > 0.22 && dia < 0.42) {
                col = vec3<f32>(0.85, 0.55, 1.0); // pristine: hollow violet diamond
            }
        }
    }

    // Faint chunk grid so the scale reads.
    let g = abs(cell);
    if (cps_y < 90.0 && (g.x > 0.47 || g.y > 0.47)) {
        col = col * 0.82;
    }

    // Blinking "you are here" dot (size in chunk units, scaled a little with zoom).
    let d = distance(chunk, u.user.xy);
    let r = max(0.6, cps_y * 0.012);
    if (d < r) {
        col = mix(col, vec3<f32>(1.0, 0.88, 0.30), u.user.z);
    }

    // Centre crosshair: marks the spot whose coords the HUD reads out (pan centre = where the
    // map is hovering). A thin white plus.
    let hx = abs(in.uv.x - 0.5);
    let hy = abs(in.uv.y - 0.5);
    if ((hx < 0.0016 && hy < 0.03) || (hy < 0.0024 && hx < 0.022)) {
        col = vec3<f32>(0.95, 0.97, 1.0);
    }
    return vec4<f32>(col, 1.0);
}
