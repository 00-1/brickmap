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
//! Post-GO (full port): the UI/text/FX pieces are **banked into the engine** — text →
//! `bm-render::text2d`, the keypad + 2-D UI primitives → `bm-render::ui2d`, save →
//! `bm-platform::save` — and re-exported here so the game consumes engine services. Nothing here
//! touches the engine's voxel half.

// The keypad + legible-text paths are engine services (`bm-render::ui2d` / `::text2d`), re-exported
// here; the drill consumes the keypad. Native-only because goblin-gold only depends on the engine
// facade off-wasm (and is only ever built native/Android — the web app is `scraped-again`).
#[cfg(not(target_arch = "wasm32"))]
pub mod drill;
#[cfg(not(target_arch = "wasm32"))]
pub mod keypad;

#[cfg(not(target_arch = "wasm32"))]
pub mod text;

// Phase 2: the GG1 question transforms (proven vs the T229 parity vectors) + the unlock-chain /
// mastery progression, re-implemented in Rust. Pure logic (serde_json only) — no engine dep.
pub mod progression;
pub mod transforms;

// Phase 3: the metagame over the T230/T232 data — the collector ladder (and, incrementally, arena
// / events / save). Pure logic; data-driven from the synced `content/gg1/` export.
pub mod collector;

// The full collectible catalogue (2352 items) over the T230/T232 export — the unified reward set
// whose ids ARE the save's `collected` keys. Pure logic; data-driven from `collectibles.json`.
pub mod catalogue;

// The save model (the central `collected` keystone + gold/last-mode), persisted through the engine
// `bm-platform::save` Store seam. Native-gated like the other engine-facade consumers.
#[cfg(not(target_arch = "wasm32"))]
pub mod save;

#[cfg(not(target_arch = "wasm32"))]
pub mod app;
// Android immersive-sticky fullscreen (hide the system bars) — reached over JNI from the
// native-activity side, since cargo-apk packages no Java of ours to set a theme.
#[cfg(not(target_arch = "wasm32"))]
pub mod fx;
#[cfg(not(target_arch = "wasm32"))]
pub mod headless;
#[cfg(target_os = "android")]
pub mod immersive;

/// The font under test (Instrument Sans, OFL — see `assets/InstrumentSans-OFL.txt`).
pub const FONT_INSTRUMENT_SANS: &[u8] = include_bytes!("../assets/InstrumentSans-Regular.ttf");
