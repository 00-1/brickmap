//! Capture the short git SHA at build time so the app can stamp a traceable build-watermark on
//! every screen (owner request — screenshots should say which build they're from). Falls back to
//! "unknown" off a git checkout; never fails the build.

use std::process::Command;

fn main() {
    let sha = Command::new("git")
        .args(["rev-parse", "--short=9", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=GG_BUILD_SHA={sha}");
    // Re-run if HEAD moves (best-effort; relative to the crate dir).
    println!("cargo:rerun-if-changed=../../.git/HEAD");
}
