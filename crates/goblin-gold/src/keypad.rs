//! Goblin Gold keypad — the numeric keypad is now an **engine widget**
//! ([`brickmap::ui2d::keypad`], banked into `bm-render` in full-port phase 1); this module
//! re-exports it so the game's call sites (the drill, the app) are unchanged.

pub use brickmap::ui2d::keypad::*;
