//! The **world-generation seam** (M9 seam #1). The engine streams, meshes, edits, and
//! collides against sections, but it does **not** know their contents: the game supplies
//! the recipe by implementing [`WorldGen`]. This single trait is what dissolves the old
//! worldgen/biome fusion — the noise primitives live in [`crate::noise`], the *recipe*
//! that composes them into a specific world lives in the game.

use crate::world::{ChunkCoord, Section};

/// How the engine asks the game for world content. Implemented by the game's terrain
/// recipe; consumed by the engine's streaming + edit/collision paths.
pub trait WorldGen: Send + Sync {
    /// Generate the section at chunk `coord` (the game's terrain recipe). Must be a pure
    /// function of `coord` (+ the implementor's seed) so the world stays seed-deterministic.
    fn generate(&self, coord: ChunkCoord) -> Section;

    /// Is the voxel at world `(x, y, z)` solid? Used for on-foot collision and DDA picking
    /// without materialising a whole section. Must agree with [`WorldGen::generate`].
    fn solid(&self, x: i32, y: i32, z: i32) -> bool;
}
