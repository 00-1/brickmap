//! bm-render — the engine's **wgpu renderer**: device/queue/surface management, the
//! pipelines and passes (chunk + structure draws, the instanced **splat** pass, the
//! **post** chain — palette/dither/pixel-scale/bloom), procedural **textures**, the
//! **particle** system, and the bitmap **hud** / world-**text** / explored-**map**
//! overlays. It consumes the mesh draw contract + splat/`LookParams` data; it knows
//! nothing about how the world is generated or what it contains (architecture §3–4).
//!
//! The *curated palette set* and the *strings/markers* drawn through these renderers are
//! game content fed in as data; M9 Phase 3 finishes paring those out.

pub use bm_core;
// Re-export the dependency modules under their original paths so the moved render code
// keeps resolving `crate::world::…`, `crate::mesh::…`, `crate::scene::…`.
pub use bm_mesh::{mesh, visibility};
pub use bm_scene::scene;
pub use bm_world::world;

pub mod foliage;
pub mod gfx;
pub mod hud;
pub mod map;
pub mod overlay;
pub mod palette;
pub mod particles;
pub mod post;
pub mod ship;
pub mod text;
pub mod textures;

/// WGSL sources for the two pipelines the headless render-to-PNG tool rebuilds inline
/// (it lives in the app crate and can't `include_wgsl!` shaders that now live here).
pub const SHADER_WGSL: &str = include_str!("shader.wgsl");
pub const PARTICLES_WGSL: &str = include_str!("particles.wgsl");
