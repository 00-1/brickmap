//! Capture the demo scene to a PNG via the headless (software-Vulkan) renderer.
//! Usage: `cargo run --bin screenshot -- [out.png] [width] [height]`

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let mut args = std::env::args().skip(1);
    let path = args.next().unwrap_or_else(|| "screenshot.png".to_string());
    let width: u32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(960);
    let height: u32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(720);

    brickmap::headless::capture(width, height, &path);
}
