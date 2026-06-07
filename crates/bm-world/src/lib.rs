//! bm-world — the engine's **voxel data model**: palette-compressed 32³ sections, block
//! ids, chunk coordinates, the procedural-noise toolkit (`worldgen`), the `edit`/`apply`
//! mutation seam, and the cellular-automata substrate (`sim`). Knows nothing about wgpu
//! or how the world is drawn (architecture §3–4).
//!
//! `worldgen` currently still carries the *terrain recipe* (block ids, sea level, cave
//! thresholds) alongside the reusable noise; M9 Phase 3 lifts that recipe out into the
//! game behind a `WorldGen` trait, leaving the pure noise here.

pub use bm_core;

pub mod edit;
pub mod sim;
pub mod world;
pub mod worldgen;
