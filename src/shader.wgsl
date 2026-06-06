// Chunk shader: unpacks the 4-byte face vertex (design §9–10), places it in the
// world by its chunk origin, shades it with one fixed directional light, and then
// leans into the tech as the aesthetic (E1, design §11): vertex-quantization
// wobble + ordered dithering.

struct Globals {
    view_proj: mat4x4<f32>,
    palette: array<vec4<f32>, 8>,
    params: vec4<f32>,     // x = wobble snap, y = colour steps (D2); z = fog start, w = fog end
    camera_pos: vec4<f32>, // xyz = camera world position
    fog_color: vec4<f32>,  // rgb = fog / sky colour
    flags: vec4<f32>,      // x = AO, y = block light, z = emissive (1 = on)
    cam_right: vec4<f32>,  // xyz = camera right; w = wind/animation time (seconds)
    cam_up: vec4<f32>,     // xyz = camera up
};
@group(0) @binding(0)
var<uniform> globals: Globals;

struct Chunk {
    origin: vec4<f32>, // xyz used
};
@group(1) @binding(0)
var<uniform> chunk: Chunk;

// Procedural material textures (M4): grayscale detail per material, tinted by the
// palette colour. Layer index == material id; tiled per voxel in world space.
@group(2) @binding(0)
var mat_tex: texture_2d_array<f32>;
@group(2) @binding(1)
var mat_samp: sampler;

struct VsOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) world_pos: vec3<f32>,
    @location(3) ao: f32,
    @location(4) @interpolate(flat) material: u32,
    @location(5) block_light: vec3<f32>,
};

@vertex
fn vs_main(@location(0) packed: u32, @location(1) packed1: u32) -> VsOut {
    // word0: x:6 | y:6 | z:6 | dir:3 | material:9 | ao:2
    let x = f32(packed & 63u);
    let y = f32((packed >> 6u) & 63u);
    let z = f32((packed >> 12u) & 63u);
    let dir = (packed >> 18u) & 7u;
    let material = (packed >> 21u) & 511u;
    let ao = (packed >> 30u) & 3u;
    // word1: lr:4 | lg:4 | lb:4  — baked block light, 0..15 per channel.
    let block_light = vec3<f32>(
        f32(packed1 & 15u),
        f32((packed1 >> 4u) & 15u),
        f32((packed1 >> 8u) & 15u),
    ) / 15.0;

    // Direction -> normal: 0:+X 1:-X 2:+Y 3:-Y 4:+Z 5:-Z.
    let axis = dir >> 1u;
    let sgn = 1.0 - 2.0 * f32(dir & 1u);
    var normal = vec3<f32>(0.0, 0.0, 0.0);
    if (axis == 0u) {
        normal.x = sgn;
    } else if (axis == 1u) {
        normal.y = sgn;
    } else {
        normal.z = sgn;
    }

    let world_pos = vec3<f32>(x, y, z) + chunk.origin.xyz;

    var out: VsOut;
    var clip = globals.view_proj * vec4<f32>(world_pos, 1.0);
    // Vertex-quantization wobble: snap NDC to a coarse grid (PS1-style). Exposes the
    // compressed-vertex quantization as motion jitter.
    let snap = max(globals.params.x, 1.0);
    let w = clip.w;
    let ndc = round((clip.xy / w) * snap) / snap;
    out.clip_position = vec4<f32>(ndc * w, clip.z, w);

    out.normal = normal;
    out.color = globals.palette[min(material, 7u)].rgb;
    out.world_pos = world_pos;
    out.ao = f32(ao);
    out.material = material;
    out.block_light = block_light;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let n = normalize(in.normal);

    // Material UV + tangent basis, both from the axis-aligned face normal. The texture
    // tiles once per voxel in world space.
    let an = abs(n);
    var uv: vec2<f32>;
    var tangent: vec3<f32>;
    var bitangent: vec3<f32>;
    if (an.x > 0.5) {
        uv = in.world_pos.zy;
        tangent = vec3<f32>(0.0, 0.0, 1.0);
        bitangent = vec3<f32>(0.0, 1.0, 0.0);
    } else if (an.y > 0.5) {
        uv = in.world_pos.xz;
        tangent = vec3<f32>(1.0, 0.0, 0.0);
        bitangent = vec3<f32>(0.0, 0.0, 1.0);
    } else {
        uv = in.world_pos.xy;
        tangent = vec3<f32>(1.0, 0.0, 0.0);
        bitangent = vec3<f32>(0.0, 1.0, 0.0);
    }
    let detail = textureSample(mat_tex, mat_samp, uv, i32(in.material)).r;

    // Sub-voxel bump relief (E4): treat the detail texture as a height field and
    // perturb the lit normal by its gradient — cheap depth, no parallax marching.
    // `flags.w` toggles it; the texture's mips fade the relief with distance.
    let eps = 1.0 / 16.0; // one texel of the per-voxel tile
    let hu = textureSample(mat_tex, mat_samp, uv + vec2<f32>(eps, 0.0), i32(in.material)).r;
    let hv = textureSample(mat_tex, mat_samp, uv + vec2<f32>(0.0, eps), i32(in.material)).r;
    let bump = (hu - detail) * tangent + (hv - detail) * bitangent;
    let nrm = normalize(n - 1.6 * bump * globals.flags.w);

    let sun_dir = normalize(vec3<f32>(0.4, 0.8, 0.5));
    let diffuse = max(dot(nrm, sun_dir), 0.0);
    // Hemispheric ambient (E3, cheap fake bounce): cool sky tint from above, warm
    // ground bounce from below, by face normal — plus a warm directional sun. Kept low
    // (a dim floor, not a flood) so faces turned away from the sun read as real shadow and
    // the scene keeps a wide light→dark range instead of washing out.
    let up = n.y * 0.5 + 0.5;
    let sky_ambient = vec3<f32>(0.17, 0.22, 0.31);
    let ground_ambient = vec3<f32>(0.06, 0.05, 0.05);
    // Strong, slightly warm key light so lit faces stay bright against the dim ambient.
    // `camera_pos.w` is the sun on/off flag — off (0) leaves the world lit only by the
    // in-world point lights + ambient, for a dark, point-lit mood.
    let sun = vec3<f32>(1.05, 0.97, 0.82) * (1.0 * diffuse) * globals.camera_pos.w;
    // Baked block light (flood-fill, E3): an emitter's colour spills onto nearby surfaces
    // and bleeds around corners. A softer-than-quadratic curve widens the pool so the
    // points actually light the world (especially with the sun off), boosted so near a
    // light reads bright. `flags.y` toggles it.
    let bl = in.block_light;
    let block = bl * (bl * 0.5 + 0.5) * 2.6 * globals.flags.y;
    let light = mix(ground_ambient, sky_ambient, up) + sun + block;
    // Baked ambient occlusion: deepen it into a contact shadow. Concave corners (ao 0)
    // sink to ~0.16 brightness and the curve is biased dark so even partial occlusion
    // (edges, ao 1–2) reads as shade. Open faces (ao 3) stay full. `flags.x` toggles off.
    let ao_n = in.ao / 3.0;
    let ao_factor = mix(1.0, 0.16 + 0.84 * ao_n * ao_n, globals.flags.x);

    var c = in.color * detail * light * ao_factor;
    // Emissive crystal (material 6): unshaded and boosted past 1.0 so the bright-pass
    // catches it and it glows through bloom. `flags.z` toggles emissive off (then the
    // crystal shades like any other block).
    if (in.material == 6u && globals.flags.z > 0.5) {
        c = in.color * detail * 1.3;
    }

    // Stylised water (E8/E9, material 7): single-pass, opaque. An animated crossed-sine
    // ripple plays over the surface, and a Fresnel-ish term brightens it toward the sky
    // colour at grazing view angles — so it reads as moving water, not a flat blue slab.
    if (in.material == 7u) {
        let t = globals.cam_right.w;
        let ripple = sin(in.world_pos.x * 0.7 + t * 1.3)
            * sin(in.world_pos.z * 0.6 + t * 1.1);
        let sheen = 0.5 + 0.5 * ripple;
        let view = normalize(globals.camera_pos.xyz - in.world_pos);
        let fres = pow(1.0 - max(dot(n, view), 0.0), 3.0);
        let sky = vec3<f32>(0.55, 0.74, 0.92);
        c = mix(in.color, sky, 0.18 * sheen + 0.45 * fres) + vec3<f32>(0.03, 0.05, 0.07) * sheen;
    }

    // Ordered (4x4 Bayer) dithering: posterise the shading into a few levels with a
    // deliberate dot pattern, instead of smoothing the banding away.
    var bayer = array<f32, 16>(
        0.0, 8.0, 2.0, 10.0,
        12.0, 4.0, 14.0, 6.0,
        3.0, 11.0, 1.0, 9.0,
        15.0, 7.0, 13.0, 5.0,
    );
    let px = u32(in.clip_position.x) % 4u;
    let py = u32(in.clip_position.y) % 4u;
    let threshold = (bayer[py * 4u + px] + 0.5) / 16.0;
    let steps = max(globals.params.y, 1.0);
    c = floor(c * steps + threshold) / steps;

    // Distance fog: fade terrain into the sky colour so the streaming load edge
    // dissolves instead of popping. Cheap — one distance + a smoothstep + a mix.
    let fog_dist = distance(in.world_pos, globals.camera_pos.xyz);
    let fog = smoothstep(globals.params.z, globals.params.w, fog_dist);

    // Distance dissolve (M7, opt-in via cam_up.w): past the fog start, stipple the
    // terrain away with the same Bayer threshold so it crumbles into a pixel haze toward
    // the horizon instead of a hard fog wall. Capped so some stipple survives into the fog.
    if (globals.cam_up.w > 0.5) {
        let melt = clamp((fog_dist - globals.params.z) / max(globals.params.w - globals.params.z, 0.001), 0.0, 1.0) * 0.85;
        if (threshold < melt) {
            discard;
        }
    }

    c = mix(c, globals.fog_color.rgb, fog);

    return vec4<f32>(c, 1.0);
}
