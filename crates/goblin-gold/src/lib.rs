//! Goblin Gold on brickmap — spike crate (BRICKMAP-GG1-SPEC).
//!
//! Mini-gates, font-first (the #1 blocker):
//! - **#1 (done):** a legible font path — [`text`] (bake a TrueType face to an AA coverage
//!   atlas + word-wrap) proven crisp through the real wgpu path.
//! - **#2:** a numeric [`keypad`] (data-free, engine-candidate) + a [`drill`] loop that
//!   consumes the **T229 content data seam** (`data/gg1/*.json`) — questions from the parity
//!   vectors, mode name from `modes.json`; right/wrong marking.
//!
//! [`render`] is the shared headless 2-D painter (text + rects) used by the prototype bins.
//! Nothing here touches the engine's voxel half; the UI/text pieces are written game-side but
//! data-free + one-way so they sink into `bm-render` as engine services later.

pub mod drill;
pub mod keypad;
pub mod text;

#[cfg(not(target_arch = "wasm32"))]
pub mod render;

/// The font under test (Instrument Sans, OFL — see `assets/InstrumentSans-OFL.txt`).
pub const FONT_INSTRUMENT_SANS: &[u8] = include_bytes!("../assets/InstrumentSans-Regular.ttf");
