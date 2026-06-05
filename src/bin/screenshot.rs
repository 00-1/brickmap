//! Capture the demo scene to a PNG via the headless (software-Vulkan) renderer.
//! Usage: `cargo run --bin screenshot -- [out.png] [width] [height]`
//!        `... [out.png] [w] [h] <eyeX eyeY eyeZ targetX targetY targetZ>` to override
//! the camera (e.g. a low side-on view to inspect caves/water).

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args: Vec<String> = std::env::args().skip(1).collect();
    let path = args
        .first()
        .cloned()
        .unwrap_or_else(|| "screenshot.png".to_string());
    let width: u32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(960);
    let height: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(720);

    // Optional camera override: 6 trailing floats = eye xyz + target xyz.
    let (eye, target) = if args.len() >= 9 {
        let f = |i: usize| args[i].parse::<f32>().ok();
        match (f(3), f(4), f(5), f(6), f(7), f(8)) {
            (Some(ex), Some(ey), Some(ez), Some(tx), Some(ty), Some(tz)) => (
                Some(glam::Vec3::new(ex, ey, ez)),
                Some(glam::Vec3::new(tx, ty, tz)),
            ),
            _ => (None, None),
        }
    } else {
        (None, None)
    };

    brickmap::headless::capture_view(width, height, &path, eye, target);
}
