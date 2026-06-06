//! Embed a short build id (git SHA) so the app can show it in the HUD/overlay on every
//! platform. Falls back to "dev" when git isn't available (e.g. a source tarball).

use std::process::Command;

fn main() {
    let sha = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "dev".to_string());
    println!("cargo:rustc-env=BRICKMAP_BUILD={sha}");
    // Re-run if HEAD moves so the id stays current.
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs");
}
