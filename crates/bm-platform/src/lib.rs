//! bm-platform — the engine's **platform edges**: the bits that differ per target,
//! kept behind `cfg` so they don't smear through the engine (architecture §6). That's
//! **gamepad/controller input** (gilrs on desktop, the browser Gamepad API on web, winit's
//! activity input on Android) normalised into one `PadInput`, and a **save** abstraction
//! (a file per key on native/Android, `localStorage` on the web) behind one `Store` trait.
//!
//! Windowing/surface/timing still live in the app while it owns the event loop; they
//! join this crate as the runtime seam settles.

pub use bm_core;

pub mod gamepad;
pub mod save;
pub mod touch;
