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
};

@vertex
fn vs_main(@location(0) packed: u32) -> VsOut {
    // Layout: x:6 | y:6 | z:6 | dir:3 | material:9 | ao:2
    let x = f32(packed & 63u);
    let y = f32((packed >> 6u) & 63u);
    let z = f32((packed >> 12u) & 63u);
    let dir = (packed >> 18u) & 7u;
    let material = (packed >> 21u) & 511u;
    let ao = (packed >> 30u) & 3u;

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
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let n = normalize(in.normal);
    let light_dir = normalize(vec3<f32>(0.4, 0.8, 0.5));
    let diffuse = max(dot(n, light_dir), 0.0);
    let shade = 0.35 + 0.65 * diffuse;
    // Baked ambient occlusion: corners (ao 0) sink to ~0.5 brightness, open faces
    // (ao 3) stay full. Multiplies the directional shade.
    let ao_factor = 0.5 + 0.5 * (in.ao / 3.0);

    // Material texture: tile per voxel in world space, axes chosen from the face
    // normal. Grayscale detail multiplies the palette tint.
    let an = abs(n);
    var uv: vec2<f32>;
    if (an.x > 0.5) {
        uv = in.world_pos.zy;
    } else if (an.y > 0.5) {
        uv = in.world_pos.xz;
    } else {
        uv = in.world_pos.xy;
    }
    let detail = textureSample(mat_tex, mat_samp, uv, i32(in.material)).r;

    var c = in.color * detail * shade * ao_factor;

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
    c = mix(c, globals.fog_color.rgb, fog);

    return vec4<f32>(c, 1.0);
}
