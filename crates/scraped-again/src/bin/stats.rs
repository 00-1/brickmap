//! M10 — print the reference scenes' **content counters** machine-readably (one `key=value`
//! per line per scene), so a human or a script can read the frame-cost numbers headlessly:
//!
//!   cargo run -p scraped-again --bin stats [seed]
//!
//! These are the same deterministic counters the CI budget gates assert (`budgets.rs`); the
//! live-only counters (upload bytes/frame, draw calls incl. passes, dynamic-res) are on the
//! in-game HUD via the engine's `DrawStats`.

fn main() {
    let seed = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(1337u32);
    println!("seed={seed}");
    for (name, pos) in scraped_again::budgets::reference_scenes(seed) {
        println!("pos={:.0},{:.0},{:.0}", pos.x, pos.y, pos.z);
        print!(
            "{}",
            scraped_again::budgets::scene_stats(seed, pos).report(name)
        );
    }
}
