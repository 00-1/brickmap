//! Embed a short build id (git SHA) so the app can show it in the HUD/overlay on every
//! platform. Falls back to "dev" when git isn't available (e.g. a source tarball).
//!
//! An explicit `BRICKMAP_BUILD` in the environment wins over the git lookup — used for
//! reproducible builds and to pin the id during golden-image comparisons (the HUD draws
//! it, so an unpinned id makes every commit's render differ).

use std::process::Command;

fn main() {
    let sha = std::env::var("BRICKMAP_BUILD")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            Command::new("git")
                .args(["rev-parse", "--short", "HEAD"])
                .output()
                .ok()
                .filter(|o| o.status.success())
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "dev".to_string())
        });
    println!("cargo:rustc-env=BRICKMAP_BUILD={sha}");
    println!("cargo:rerun-if-env-changed=BRICKMAP_BUILD");
    // Re-run if HEAD moves so the id stays current (the repo root is two levels up now
    // that the crate lives under `crates/brickmap/`).
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    println!("cargo:rerun-if-changed=../../.git/refs");
}
