// Space cruiser (E19) — a small polygonal ship drawn in its own pass *after* the palette
// post-process, so its true colours (and glowing nav-lights) survive instead of being mapped
// onto the biome palette. Lit by a fixed key + its vertex colour; its own depth buffer makes it
// self-occlude. (It draws over the world — no scene-depth test — so it reads as an always-visible
// landmark when parked.)

struct U {
    mvp: mat4x4<f32>,   // view_proj * model (translate to the cruiser, scale, yaw)
    model: mat4x4<f32>, // model only, for lighting the normal in world space
};
@group(0) @binding(0) var<uniform> u: U;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) emissive: f32,
};

@vertex
fn vs_main(
    @location(0) p: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) normal: vec3<f32>,
    @location(3) emissive: f32,
) -> VsOut {
    var o: VsOut;
    o.pos = u.mvp * vec4<f32>(p, 1.0);
    o.color = color;
    o.normal = normalize((u.model * vec4<f32>(normal, 0.0)).xyz);
    o.emissive = emissive;
    return o;
}

// Authored colours are display-sRGB; the surface re-encodes linear→sRGB on store, so decode here
// (matches the palette pass) for true on-screen colour.
fn srgb_to_linear(c: vec3<f32>) -> vec3<f32> {
    let lo = c / 12.92;
    let hi = pow((c + 0.055) / 1.055, vec3<f32>(2.4));
    return select(hi, lo, c <= vec3<f32>(0.04045));
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let key = normalize(vec3<f32>(0.4, 0.8, 0.5));
    let diff = max(dot(normalize(in.normal), key), 0.0);
    // Emissive parts (nav-lights) ignore shading and stay full, bright colour; hull is lit.
    let shade = mix(0.45 + 0.55 * diff, 1.25, in.emissive);
    return vec4<f32>(srgb_to_linear(in.color * shade), 1.0);
}
