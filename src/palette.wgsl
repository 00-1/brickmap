// Palette post-process (E10): map the finished image to a small, configurable colour
// palette, with ordered (Bayer 4×4) dithering so a low palette count still produces
// extra apparent shades by mixing neighbouring palette colours spatially.

struct Params {
    colors: array<vec4<f32>, 16>, // palette (rgb); first `count` are active
    count: u32,
    enabled: u32,
    dither: f32, // spread of the ordered-dither offset (0 = hard quantise)
    _pad: f32,
};

@group(0) @binding(0) var src: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;
@group(0) @binding(2) var<uniform> p: Params;

// Palette colours are authored as sRGB display values, but the render target is sRGB and
// re-encodes linear→sRGB on store. Decode to linear here so what lands on screen is exactly
// the authored colour (otherwise the whole ramp washes out several stops brighter).
fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    let lo = c / 12.92;
    let hi = pow((c + 0.055) / 1.055, vec3<f32>(2.4));
    return select(hi, lo, c <= vec3<f32>(0.04045));
}

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

// Fullscreen triangle.
@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> VsOut {
    var verts = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0),
    );
    let v = verts[i];
    var out: VsOut;
    out.pos = vec4<f32>(v, 0.0, 1.0);
    out.uv = v * 0.5 + 0.5; // 0..1, y up
    out.uv.y = 1.0 - out.uv.y;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let c = textureSample(src, samp, in.uv).rgb;
    if (p.enabled == 0u || p.count == 0u) {
        return vec4<f32>(c, 1.0);
    }
    // Tonal gradient-map: the palettes are ordered dark→light ramps, so we map the image's
    // *luminance* onto the ramp rather than nearest-RGB (which collapses a bright scene onto
    // the light end and washes everything out). This recolours the whole image into the
    // restrained palette while keeping its lighting/structure — a deliberate poster look.
    let luma = dot(c, vec3<f32>(0.299, 0.587, 0.114));

    // Ordered (Bayer 4×4) dithering between adjacent ramp steps: a value landing between two
    // palette tones is pushed to the nearer one per-pixel, so a low palette count still reads
    // as extra in-between shades instead of hard banding.
    var bayer = array<f32, 16>(
        0.0, 8.0, 2.0, 10.0,
        12.0, 4.0, 14.0, 6.0,
        3.0, 11.0, 1.0, 9.0,
        15.0, 7.0, 13.0, 5.0,
    );
    let bx = u32(in.pos.x) % 4u;
    let by = u32(in.pos.y) % 4u;
    let t = ((bayer[by * 4u + bx] + 0.5) / 16.0) - 0.5; // centred Bayer, ~±0.5

    // Position on the ramp in [0, count-1]; dither nudges by up to half a step.
    let last = f32(p.count - 1u);
    let pos = clamp(luma * last + t * p.dither, 0.0, last);
    let idx = u32(floor(pos + 0.5));
    return vec4<f32>(srgb_to_linear(p.colors[idx].rgb), 1.0);
}
