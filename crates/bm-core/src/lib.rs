//! bm-core — the brickmap engine's **dependency-light foundation**.
//!
//! It pins the shared math (`glam`) and POD-cast (`bytemuck`) crates in one place and
//! re-exports them so the engine crates and new code share one import surface, and it
//! houses small cross-cutting utilities as they earn their keep. It depends on nothing
//! else in the workspace — every other engine crate sits above it (architecture §3).
//!
//! Note: the existing worldgen/biome/foliage modules keep their *locally tuned* hash and
//! noise variants (different salts/curves), because the generated world is golden — see
//! the E12 voxel-hash test. [`hash01`] here is the canonical small PRNG for *new* engine
//! code (e.g. the engine demo's `WorldGen`), not a replacement for those.

pub use bytemuck;
pub use glam;

/// Seeded integer hash → a uniform float in `[0, 1)`. A small, fast splitmix-style mix;
/// deterministic and platform-stable. For new engine code and the engine demo.
pub fn hash01(x: i32, y: i32, seed: u32) -> f32 {
    let mut h = seed.wrapping_mul(0x9E37_79B9)
        ^ (x as u32).wrapping_mul(0x85EB_CA6B)
        ^ (y as u32).wrapping_mul(0xC2B2_AE35);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2545_F491);
    h ^= h >> 13;
    // Top 24 bits → [0,1); avoids the low-bit patterning of a raw modulo.
    (h >> 8) as f32 / (1u32 << 24) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash01_in_range_and_deterministic() {
        for x in -8..8 {
            for y in -8..8 {
                let v = hash01(x, y, 42);
                assert!((0.0..1.0).contains(&v), "out of range: {v}");
                assert_eq!(v, hash01(x, y, 42), "not deterministic");
            }
        }
        // Different seeds/coords diverge.
        assert_ne!(hash01(1, 2, 1), hash01(1, 2, 2));
        assert_ne!(hash01(1, 2, 1), hash01(2, 1, 1));
    }
}
