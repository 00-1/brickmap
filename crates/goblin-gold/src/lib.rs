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

// The Arena: bestiary + hero roster over `balance.json`, with collected items boosting hero stats
// (the catalogue → fight bridge). Pure logic; data-driven.
pub mod arena;

// Daily events: the 14-event rotation (reward keys + the 14-day UTC cycle structure). Pure logic;
// data-driven from the `Events` collectibles.
pub mod events;

// The Arena combat resolution (T233b-combat): the 3v3 team battle from main.js/Enemies.teamBattle,
// proven vs `combat-vectors.json`. Pure logic; composes the catalogue/arena boost bridge.
pub mod combat;

// Event-play (T233c): the deterministic daily-event gauntlet + UTC-day schedule + reward tiers from
// events.js/main.js, proven vs `events-vectors.json`. Pure logic; reuses the synth PRNG + transforms.
pub mod event_play;

// Procedural portraits (F1+F2): the 16×16 hero/foe generators from collectibles.js drawIcon +
// monsters.js buildGrid, proven byte-identical vs `art-vectors.json`. Pure logic; the screens paint
// these role grids through the per-id/per-type palettes.
pub mod art;

// Procedural backdrops + banners (F3+F4): the scenery.js + eventart.js full-colour generators (Arena
// region scenes 28×11 + per-event emblem crests 24×16), proven byte-identical vs `scenes-vectors.json`.
pub mod scenes;

// Earning: the rule turning a finished round + running totals into awarded collectible keys,
// re-impl'd from collectibles.js and proven vs `earning-vectors.json`. Pure logic.
pub mod earning;

// The Goblin Gold economy: the round-payout formulas re-impl'd from main.js, proven vs
// `gold-vectors.json`. Pure logic.
pub mod gold;

// The deterministic gold-HOARD coin-pile (`fxgl.js seedHoard`) — the Home hub's signature gold pile.
// Pure: surface-coin placement (mound profile + xorshift scatter); the screen paints the coins.
pub mod hoard;

// The topic pixel-glyphs (`glyphs.js` + `modes.js TOPIC_GLYPHS`) — the operator marks (`×/2`, `a×b`,
// …) drawn in the Home tree's nodes. Pure: token DSL → ink-code grid; the screen paints the cells.
pub mod glyphs;

// Phase 4 (audio): the sound effects re-authored from sound.js as pure sample buffers (same DSP
// style as scraped-again's Drone). Perceptual parity is owner-by-ear; tests gate the mechanics.
pub mod sfx;

// Phase 4 (audio): the generative-music SCORE generator from synth.js — vector-proven against the
// `synth_score_*.json` goldens (the note schedule; token synthesis is the by-ear half).
pub mod synth;

// Phase 4 (audio): the music renderer — turns a synth score into audible mono audio (perceptual,
// owner-by-ear; tests gate the mechanics). First cut; patch-faithful timbre is the refinement pass.
pub mod music;

// Phase 4 (audio): playback wiring — a cpal output stream that mixes the re-authored SFX + the
// generative music bed so they're audible in the live app. Pure mixer core (tested) + cpal (device).
#[cfg(not(target_arch = "wasm32"))]
pub mod audio;

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

/// **JetBrains Mono** (OFL — see `assets/JetBrainsMono-OFL.txt`) — web GG1's monospace face for
/// every headline number / count / chip (`mark`, `eb-tag`, `eb-count`, …). The owner's V25 side-by-
/// side flagged "looks nothing alike" because the previous all-Instrument-Sans render was a smooth
/// rounded sans where web shows a crisp, uniform-width mono. Bundled and used as the default UI
/// face here on (Instrument Sans is kept as a fallback for any later display-face needs).
pub const FONT_JETBRAINS_MONO: &[u8] = include_bytes!("../assets/JetBrainsMono-Regular.ttf");

/// The short git SHA captured at build time (see `build.rs`) — the build-watermark stamped on every
/// on-device screen so screenshots are traceable.
pub const BUILD_SHA: &str = env!("GG_BUILD_SHA");

/// The build-watermark label (version + SHA), e.g. `v0.0.1 · 602f2bd`.
pub fn build_tag() -> String {
    format!("v{} · {}", env!("CARGO_PKG_VERSION"), BUILD_SHA)
}
