// Bloom post-processing (E3): bright-pass → separable blur (¼-res) → additive
// composite. Cheap LDR bloom — makes the emissive particles (and any bright blocks)
// glow against the dusk sky. All passes are fullscreen; texel sizes come from
// textureDimensions so no extra uniforms are needed.

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VsOut {
    var xy = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    let p = xy[vi];
    var out: VsOut;
    out.pos = vec4<f32>(p, 0.0, 1.0);
    // Map clip [-1,1] → uv [0,1], flipping Y so it matches texture space.
    out.uv = vec2<f32>(p.x * 0.5 + 0.5, 1.0 - (p.y * 0.5 + 0.5));
    return out;
}

@group(0) @binding(0) var src: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;

// Bright-pass: keep only what's brighter than the knee, softly. Output is the
// over-threshold colour (so warm embers pass, the lit terrain mostly doesn't).
@fragment
fn fs_bright(in: VsOut) -> @location(0) vec4<f32> {
    let c = textureSample(src, samp, in.uv).rgb;
    let lum = dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
    let knee = 0.62;
    let w = clamp((lum - knee) / max(1.0 - knee, 0.001), 0.0, 1.0);
    return vec4<f32>(c * w, 1.0);
}

// 9-tap Gaussian weights (centre + 4 each side).
fn gauss(tex: texture_2d<f32>, uv: vec2<f32>, dir: vec2<f32>) -> vec3<f32> {
    let texel = 1.0 / vec2<f32>(textureDimensions(tex));
    let step = dir * texel;
    var sum = textureSample(tex, samp, uv).rgb * 0.2270270270;
    sum += textureSample(tex, samp, uv + step * 1.0).rgb * 0.1945945946;
    sum += textureSample(tex, samp, uv - step * 1.0).rgb * 0.1945945946;
    sum += textureSample(tex, samp, uv + step * 2.0).rgb * 0.1216216216;
    sum += textureSample(tex, samp, uv - step * 2.0).rgb * 0.1216216216;
    sum += textureSample(tex, samp, uv + step * 3.0).rgb * 0.0540540541;
    sum += textureSample(tex, samp, uv - step * 3.0).rgb * 0.0540540541;
    sum += textureSample(tex, samp, uv + step * 4.0).rgb * 0.0162162162;
    sum += textureSample(tex, samp, uv - step * 4.0).rgb * 0.0162162162;
    return sum;
}

@fragment
fn fs_blur_h(in: VsOut) -> @location(0) vec4<f32> {
    return vec4<f32>(gauss(src, in.uv, vec2<f32>(1.0, 0.0)), 1.0);
}

@fragment
fn fs_blur_v(in: VsOut) -> @location(0) vec4<f32> {
    return vec4<f32>(gauss(src, in.uv, vec2<f32>(0.0, 1.0)), 1.0);
}

// Composite: scene + bloom. A second sampler/texture for the bloom layer.
@group(0) @binding(2) var bloom: texture_2d<f32>;

@fragment
fn fs_composite(in: VsOut) -> @location(0) vec4<f32> {
    let scene = textureSample(src, samp, in.uv).rgb;
    let glow = textureSample(bloom, samp, in.uv).rgb;
    return vec4<f32>(scene + glow * 1.1, 1.0);
}
