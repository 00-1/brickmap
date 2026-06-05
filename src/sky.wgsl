// Fullscreen screen-space sky gradient (E3). No bindings: a single fullscreen
// triangle, vertical gradient by clip-space Y (horizon low → zenith high). Drawn
// first, behind everything, with no depth write so the terrain draws over it. A
// screen-space gradient is correct under translation + yaw (the auto-fly path);
// only manual pitch would reveal it (a view-ray sky is a later upgrade).

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) t: f32,
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
    out.pos = vec4<f32>(p, 1.0, 1.0); // z = 1 (far plane)
    out.t = clamp(p.y * 0.5 + 0.5, 0.0, 1.0);
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    // Horizon (low) is the fog colour so terrain dissolves into it; zenith (high)
    // is a deeper night-leaning blue.
    let horizon = vec3<f32>(0.30, 0.33, 0.42);
    let zenith = vec3<f32>(0.04, 0.06, 0.13);
    let c = mix(horizon, zenith, in.t);
    return vec4<f32>(c, 1.0);
}
