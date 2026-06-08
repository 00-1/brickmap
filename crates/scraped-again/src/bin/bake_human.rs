//! Bake the CC0 human mesh → a compact embedded point cloud (E18). A **one-time dev tool**, run
//! on native (the OBJ is present at build-host time):
//!
//!   cargo run -p scraped-again --bin bake_human
//!
//! It samples `assets/base-human.obj`'s surface and writes `assets/human_points.bin` (committed),
//! which the live build embeds via `include_bytes!` — so all targets (incl. the wasm/web build)
//! get the human geometry **without** shipping + parsing the ~19k-vert OBJ text. Re-run if the
//! source mesh or the sample count changes.

use scraped_again::model;

fn main() {
    let obj = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/base-human.obj");
    let text = std::fs::read_to_string(obj).expect("read assets/base-human.obj");
    let mesh = model::load_obj(&text);
    // A point budget that reads as a recognisable figure but stays a small embed (~84 KB at 7000).
    let pts = model::surface_points(&mesh, 7000, 1);
    let blob = model::encode_points(&pts);
    let out = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/human_points.bin");
    std::fs::write(out, &blob).expect("write assets/human_points.bin");
    eprintln!(
        "baked {} surface points → {} bytes → {out}",
        pts.len(),
        blob.len()
    );
}
