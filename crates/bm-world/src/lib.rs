//! bm-world — the engine's **voxel data model**: palette-compressed 32³ sections, block
//! ids, chunk coordinates, the procedural-noise toolkit (`worldgen`), the `edit`/`apply`
//! mutation seam, and the cellular-automata substrate (`sim`). Knows nothing about wgpu
//! or how the world is drawn (architecture §3–4).
//!
//! The reusable procedural **noise** primitives live in [`noise`]; the terrain *recipe*
//! that composes them into a specific world is the game's, supplied through the
//! [`WorldGen`] seam (M9).

pub use bm_core;
pub use gen::WorldGen;

pub mod edit;
pub mod gen;
pub mod noise;
pub mod sim;
pub mod world;
