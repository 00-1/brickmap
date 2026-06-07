//! bm-scene — the engine's **camera + culling policy**: camera state, view/projection,
//! frustum extraction, and the floating-origin look. Combines the frustum with the
//! visibility graph into draw lists (architecture §3, §5).

pub use bm_core;

pub mod scene;
