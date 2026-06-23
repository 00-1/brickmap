//! Goblin Gold on brickmap — spike crate (BRICKMAP-GG1-SPEC).
//!
//! Mini-gates, font-first (the #1 blocker):
//! - **#1 (done):** a legible font path — [`text`] (bake a TrueType face to an AA coverage
//!   atlas + word-wrap) proven crisp through the real wgpu path.
//! - **#2 (done):** a numeric [`keypad`] (data-free, engine-candidate) + a [`drill`] loop that
//!   consumes the **T229 content data seam** (`data/gg1/*.json`) — questions from the parity
//!   vectors, mode name from `modes.json`; right/wrong marking.
//! - **#3 (done):** a correct-answer [`fx`] flourish built on brickmap's OWN recipes (palette
//!   dither + the CPU particle system — NOT a port of the web `fxgl.js`), asserted by a headless
//!   **golden-PNG diff** (the [`headless`] golden layer; the GPU render runs under lavapipe).
//! - **#4:** the windowed [`app`] runtime — drill + keypad + FX FULLSCREEN through a real wgpu
//!   **surface**, packaged by `cargo-apk` into a native **Android APK** (distinct package id, no
//!   voxel world). The final spike gate; the owner device-judges, then go/no-go.
//!
//! [`headless`] is the shared headless 2-D painter (text + rects + the palette-dither post) and
//! the golden-diff comparator used by the prototype bins/tests; [`app`] is the on-device runtime.
//! Nothing here touches the engine's voxel half; the UI/text/FX pieces are written game-side but
//! data-free + one-way so they sink into `bm-render` as engine services later.

pub mod drill;
pub mod keypad;

// The text path is the engine's `bm-render::text2d` service (re-exported); native-only because
// goblin-gold only depends on the engine facade off-wasm.
#[cfg(not(target_arch = "wasm32"))]
pub mod text;

#[cfg(not(target_arch = "wasm32"))]
pub mod app;
#[cfg(not(target_arch = "wasm32"))]
pub mod fx;
#[cfg(not(target_arch = "wasm32"))]
pub mod headless;

/// The font under test (Instrument Sans, OFL — see `assets/InstrumentSans-OFL.txt`).
pub const FONT_INSTRUMENT_SANS: &[u8] = include_bytes!("../assets/InstrumentSans-Regular.ttf");
