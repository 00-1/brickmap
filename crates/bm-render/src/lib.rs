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
pub mod text2d;
pub mod textures;
pub mod ui2d;

/// WGSL sources for the two pipelines the headless render-to-PNG tool rebuilds inline
/// (it lives in the app crate and can't `include_wgsl!` shaders that now live here).
pub const SHADER_WGSL: &str = include_str!("shader.wgsl");
pub const PARTICLES_WGSL: &str = include_str!("particles.wgsl");

/// Quantize a Bayer-dithered fade factor to the 4×4 matrix's **17 representable levels**
/// (0/16 … 16/16) so a slow fade steps cleanly through the dither levels instead of
/// rippling between them (M11). The dissolve in `shader.wgsl` mirrors this expression
/// exactly (`floor(melt_raw * 16.0 + 0.5) / 16.0`) — keep the two in lockstep.
pub fn quantize_fade(f: f32) -> f32 {
    (f * 16.0 + 0.5).floor() / 16.0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// M11: the fade quantizer hits exactly the 17 Bayer levels — every level is a fixed
    /// point, all outputs land on k/16, the map is monotone, and the shader still carries
    /// the mirrored expression.
    #[test]
    fn fade_quantization_levels_are_exact() {
        for k in 0..=16u32 {
            let level = k as f32 / 16.0;
            assert_eq!(
                quantize_fade(level),
                level,
                "level {k}/16 must be a fixed point"
            );
        }
        let mut prev = 0.0f32;
        for i in 0..=1000 {
            let q = quantize_fade(i as f32 / 1000.0);
            assert_eq!(
                (q * 16.0).fract(),
                0.0,
                "output {q} is not a 17-level value"
            );
            assert!(q >= prev, "quantizer must be monotone");
            prev = q;
        }
        assert_eq!(prev, 1.0);
        // The shader's dissolve must keep mirroring this function.
        assert!(SHADER_WGSL.contains("floor(melt_raw * 16.0 + 0.5) / 16.0"));
    }
}
