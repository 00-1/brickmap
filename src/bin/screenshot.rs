//! Capture the demo scene to a PNG via the headless (software-Vulkan) renderer.
//! Usage: `cargo run --bin screenshot -- [out.png] [width] [height]`
//!        `... [out.png] [w] [h] <eyeX eyeY eyeZ targetX targetY targetZ>` to override
//! the camera (e.g. a low side-on view to inspect caves/water).
//!        `cargo run --bin screenshot -- palettes [width] [height]` renders one PNG per
//! curated palette (E10) — `palette-<name>.png` — plus a no-palette `palette-off.png`,
//! to compare restrained-palette looks side by side.

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args: Vec<String> = std::env::args().skip(1).collect();

    // Palette-comparison mode: render the hero shot once per curated palette.
    if args.first().map(String::as_str) == Some("palettes") {
        let width: u32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(960);
        let height: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(720);
        brickmap::headless::capture_view(width, height, "palette-off.png", None, None, None);
        for (i, pal) in brickmap::palette::PALETTES.iter().enumerate() {
            let spec = brickmap::headless::PaletteSpec {
                index: i,
                count: pal.colors.len() as u32,
                dither: 1.0,
            };
            let path = format!("palette-{}.png", pal.name);
            brickmap::headless::capture_view(width, height, &path, None, None, Some(spec));
        }
        // Low-count + dither demos: prove a 2–3 colour palette still reads as extra shades
        // via ordered dithering (the "fake more colours" goal). `<name>-<count>[-nodither]`.
        for (name, idx, count) in [("mono", 0usize, 2u32), ("verdant", 1, 3)] {
            for (dither, suffix) in [(1.0f32, ""), (0.0, "-nodither")] {
                let spec = brickmap::headless::PaletteSpec {
                    index: idx,
                    count,
                    dither,
                };
                let path = format!("palette-{name}-{count}{suffix}.png");
                brickmap::headless::capture_view(width, height, &path, None, None, Some(spec));
            }
        }
        return;
    }
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

    brickmap::headless::capture_view(width, height, &path, eye, target, None);
}
