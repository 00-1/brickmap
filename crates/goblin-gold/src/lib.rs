//! Goblin Gold on brickmap — spike crate (BRICKMAP-GG1-SPEC).
//!
//! Mini-gate #1 (the #1 blocker): a **legible font path**. This crate currently holds only
//! the [`text`] core (bake a TrueType face to an anti-aliased coverage atlas + word-wrap
//! prose) and the `font_proto` bin that renders guide-length prose headless via wgpu →
//! PNG, to settle whether the brickmap stack can show crisp reading-size text. Later
//! mini-gates (keypad+drill consuming the T229 data seam · golden-PNG-verified FX · clean
//! APK) extend this crate; nothing here touches the engine's voxel half.

pub mod text;

/// The font under test (Instrument Sans, OFL — see `assets/InstrumentSans-OFL.txt`).
pub const FONT_INSTRUMENT_SANS: &[u8] = include_bytes!("../assets/InstrumentSans-Regular.ttf");
