//! bm-mesh — the engine's **CPU geometry stage**: a binary greedy mesher that turns a
//! section (+ its neighbours) into the packed-vertex **draw contract** (≤8-byte face
//! vertices), and the per-chunk **visibility graph** used for cave/occlusion culling.
//! Pure CPU — no GPU types (architecture §3–4).

pub use bm_core;
// Re-export `world` under its original path so the moved mesher/visibility code keeps
// resolving `crate::world::…`.
pub use bm_world::world;

pub mod mesh;
pub mod visibility;
