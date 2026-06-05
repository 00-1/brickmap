//! Ground foliage scatter (E6) — the first layer of the point-cloud / foliage pivot.
//! Pure logic (knows nothing about wgpu): given a generated section, emit a handful of
//! **world-positioned** grass splats just above each grass-topped column, with hashed
//! jitter / size / green-variation / wind-sway phase. Deterministic from the seed, so
//! renders reproduce. The renderer (`gfx`) draws these as instanced camera-facing
//! billboards; this module only decides *where* the points go.

use bytemuck::{Pod, Zeroable};

use crate::world::{BlockId, Section};

/// Grass surface block (mirrors `worldgen::GRASS`); foliage only sprouts on grass.
const GRASS: BlockId = BlockId(3);

/// One foliage splat for the renderer: a world position, a size, a flat colour, and a
/// wind-sway phase. `#[repr(C)]` + `Pod` so it uploads straight to an instance buffer.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Pod, Zeroable)]
pub struct SplatInstance {
    /// World-space centre (the chunk origin is already folded in).
    pub offset: [f32; 3],
    /// Billboard half-extent (world units).
    pub size: f32,
    /// Flat RGB (slightly emissive greens glow through bloom).
    pub color: [f32; 3],
    /// Per-splat phase so the wind sway is incoherent across the field.
    pub sway: f32,
}

/// A small integer hash → `[0, 1)`, same shape as `worldgen::hash` but salted per call
/// so successive draws from one column are independent.
fn hash01(x: i32, z: i32, salt: u32) -> f32 {
    let mut h = (x as u32)
        .wrapping_mul(0x1657_4c2f)
        .wrapping_add((z as u32).wrapping_mul(0x68b3_8d2b))
        .wrapping_add(salt.wrapping_mul(0x9e37_79b9));
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297a_2d39);
    h ^= h >> 15;
    (h & 0x00ff_ffff) as f32 / 0x0100_0000 as f32
}

/// The highest solid voxel in a column, or `None` if the column is empty.
fn top_solid(section: &Section, x: u32, z: u32) -> Option<u32> {
    (0..Section::SIZE)
        .rev()
        .find(|&y| section.get(x, y, z).is_solid())
}

/// Scatter foliage over the grass columns of `section` (at chunk `cx, cz`). Emits up to
/// `density` splats per grass column (a hashed subset thin it for a natural look), each
/// world-positioned just above the surface. Deterministic in `(cx, cz, seed)`.
pub fn scatter(section: &Section, cx: i32, cz: i32, seed: u32, density: u32) -> Vec<SplatInstance> {
    if density == 0 {
        return Vec::new();
    }
    let s = Section::SIZE as i32;
    let mut out = Vec::new();
    for z in 0..Section::SIZE {
        for x in 0..Section::SIZE {
            let Some(ty) = top_solid(section, x, z) else {
                continue;
            };
            // Foliage only on grass, and never right at the section ceiling (no room).
            if section.get(x, ty, z) != GRASS || ty + 1 >= Section::SIZE {
                continue;
            }
            let (wx, wz) = (cx * s + x as i32, cz * s + z as i32);
            // A per-column thinning roll so not every grass cell is equally dense. (Biome
            // lushness is applied by the caller scaling `density`, since it varies slowly.)
            let lushness = hash01(wx, wz, seed ^ 0x0f01_1a6e);
            let n = ((density as f32) * (0.35 + 0.65 * lushness)).round() as u32;
            for k in 0..n {
                let salt = seed
                    .wrapping_add(0x00a1_0000)
                    .wrapping_add(k.wrapping_mul(0x9e37));
                let hx = hash01(wx, wz, salt);
                let hz = hash01(wx, wz, salt ^ 0x55aa_55aa);
                let hs = hash01(wx, wz, salt ^ 0x1234_abcd);
                let hc = hash01(wx, wz, salt ^ 0x7777_3333);
                let size = 0.22 + hs * 0.42;
                // Lush, varied greens; brighter blades catch the bloom.
                let g = 0.45 + hc * 0.40;
                let r = 0.12 + hc * 0.22;
                let b = 0.08 + (1.0 - hc) * 0.14;
                out.push(SplatInstance {
                    offset: [
                        wx as f32 + 0.5 + (hx - 0.5) * 0.9,
                        // Sit the blade's base on the surface top face (local y = ty+1).
                        (ty as f32) + 1.0 + size * 0.5,
                        wz as f32 + 0.5 + (hz - 0.5) * 0.9,
                    ],
                    size,
                    color: [r, g, b],
                    sway: hash01(wx, wz, salt ^ 0x2468_ace0) * std::f32::consts::TAU,
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A flat one-section world: grass on top of dirt across the whole floor at y=4.
    fn grass_section() -> Section {
        let mut s = Section::new();
        for z in 0..Section::SIZE {
            for x in 0..Section::SIZE {
                s.set(x, 0, z, BlockId(2)); // dirt
                s.set(x, 1, z, BlockId(2));
                s.set(x, 2, z, GRASS);
            }
        }
        s
    }

    #[test]
    fn scatter_is_deterministic() {
        let s = grass_section();
        let a = scatter(&s, 1, -2, 99, 3);
        let b = scatter(&s, 1, -2, 99, 3);
        assert_eq!(a, b);
        assert!(!a.is_empty());
    }

    #[test]
    fn density_zero_emits_nothing() {
        assert!(scatter(&grass_section(), 0, 0, 1, 0).is_empty());
    }

    #[test]
    fn more_density_emits_more() {
        let s = grass_section();
        let sparse = scatter(&s, 0, 0, 7, 1).len();
        let dense = scatter(&s, 0, 0, 7, 6).len();
        assert!(
            dense > sparse,
            "denser scatter should emit more ({dense} > {sparse})"
        );
    }

    #[test]
    fn only_grass_sprouts_foliage() {
        // Stone-topped world: no foliage at all.
        let mut s = Section::new();
        for z in 0..Section::SIZE {
            for x in 0..Section::SIZE {
                s.set(x, 0, z, BlockId(1)); // stone
            }
        }
        assert!(scatter(&s, 0, 0, 3, 4).is_empty());
    }

    #[test]
    fn splats_sit_just_above_the_surface() {
        // Grass top is at local y=2, so its top face is at y=3; blades rise from there.
        let splats = scatter(&grass_section(), 0, 0, 5, 4);
        for sp in &splats {
            assert!(
                sp.offset[1] >= 3.0,
                "blade base below surface: {}",
                sp.offset[1]
            );
            assert!(
                sp.offset[1] < 5.0,
                "blade floating too high: {}",
                sp.offset[1]
            );
            assert!(sp.size > 0.0);
        }
    }
}
