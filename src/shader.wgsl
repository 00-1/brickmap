// Chunk shader: unpacks the 4-byte face vertex (design §9–10), places it in the
// world by its chunk origin, and shades it with one fixed directional light.

struct Globals {
    view_proj: mat4x4<f32>,
    palette: array<vec4<f32>, 8>,
};
@group(0) @binding(0)
var<uniform> globals: Globals;

struct Chunk {
    origin: vec4<f32>, // xyz used
};
@group(1) @binding(0)
var<uniform> chunk: Chunk;

struct VsOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) normal: vec3<f32>,
};

@vertex
fn vs_main(@location(0) packed: u32) -> VsOut {
    // Layout: x:6 | y:6 | z:6 | dir:3 | material:9 | ao:2
    let x = f32(packed & 63u);
    let y = f32((packed >> 6u) & 63u);
    let z = f32((packed >> 12u) & 63u);
    let dir = (packed >> 18u) & 7u;
    let material = (packed >> 21u) & 511u;

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
    out.clip_position = globals.view_proj * vec4<f32>(world_pos, 1.0);
    out.normal = normal;
    out.color = globals.palette[min(material, 7u)].rgb;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let n = normalize(in.normal);
    let light_dir = normalize(vec3<f32>(0.4, 0.8, 0.5));
    let diffuse = max(dot(n, light_dir), 0.0);
    let shade = 0.35 + 0.65 * diffuse;
    return vec4<f32>(in.color * shade, 1.0);
}
