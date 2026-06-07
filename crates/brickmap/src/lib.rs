//! brickmap — the **engine facade**.
//!
//! brickmap is a cross-platform voxel **rendering engine**. This crate is a thin facade
//! that re-exports the `bm-*` engine library crates as one import surface, so a consumer
//! (the game, the engine demo) can `use brickmap::{world, mesh, gfx, scene, …}` without
//! depending on each crate individually. It owns **no** binary and **no** game content —
//! the engine/game boundary is enforced by the crate graph (architecture §3, milestone
//! M9): nothing here, or below it, may depend on the game crate.
//!
//! Layers (low → high): `bm-core` → `bm-world` / `bm-mesh` / `bm-scene` → `bm-render`,
//! with `bm-platform` at the edge. See `docs/architecture.md`.

// Re-export the engine crates themselves (e.g. `brickmap::bm_render::SHADER_WGSL`)…
pub use {bm_core, bm_mesh, bm_platform, bm_render, bm_scene, bm_world};

// …and their modules under the conventional short paths (`brickmap::world`, etc.).
pub use bm_mesh::{mesh, visibility};
pub use bm_platform::gamepad;
pub use bm_render::{foliage, gfx, hud, map, palette, particles, post, ship, text, textures};
pub use bm_scene::scene;
pub use bm_world::{edit, sim, world, worldgen};
