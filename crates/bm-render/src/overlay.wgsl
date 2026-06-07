// Post-palette overlay (G2 engine capability). Two pipelines:
//  • the *line* pass draws the game's world-space overlay triangles into an internal-res
//    buffer, depth-tested against the retained scene depth (so terrain in front occludes
//    them) — vivid, never palettised;
//  • the *composite* pass adds that buffer over the finished frame with a cheap additive
//    glow (it misses the scene's pre-palette bloom, so the glow is baked here).

struct U { view_proj: mat4x4<f32> };
@group(0) @binding(0) var<uniform> u: U;

struct VOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) alpha: f32,
};

@vertex
fn vs_line(
    @location(0) p: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) alpha: f32,
) -> VOut {
    var o: VOut;
    o.pos = u.view_proj * vec4<f32>(p, 1.0);
    o.color = color;
    o.alpha = alpha;
    return o;
}

// Authored colours are display-sRGB; the internal target is linear, so decode (the colour
// stays raw/vivid — it never passes through the palette map).
fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    let lo = c / 12.92;
    let hi = pow((c + 0.055) / 1.055, vec3<f32>(2.4));
    return select(hi, lo, c <= vec3<f32>(0.04045));
}

@fragment
fn fs_line(i: VOut) -> @location(0) vec4<f32> {
    return vec4<f32>(srgb_to_linear(i.color), i.alpha);
}

// --- composite (fullscreen) ---------------------------------------------------------------
@group(0) @binding(0) var tex: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;

struct COut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_comp(@builtin(vertex_index) vi: u32) -> COut {
    var o: COut;
    let x = f32((vi << 1u) & 2u);
    let y = f32(vi & 2u);
    o.uv = vec2<f32>(x, y);
    o.pos = vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
    return o;
}

@fragment
fn fs_comp(i: COut) -> @location(0) vec4<f32> {
    let c = textureSample(tex, samp, i.uv);
    // Premultiply so the feathered ribbon edges add softly; the composite pipeline blends
    // this additively over the finished frame (light cutting through the murk).
    return vec4<f32>(c.rgb * c.a, c.a);
}
