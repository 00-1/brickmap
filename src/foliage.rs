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

/// Xorshift32 → `[0, 1)`, advancing `state`. A per-tree local RNG so a tree's trunk +
/// canopy points are reproducible but varied.
fn frand(state: &mut u32) -> f32 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    *state = x;
    (x >> 8) as f32 / (1u32 << 24) as f32
}

/// Append one point-cloud tree at world `base` (its foot on the surface) to `out`: a
/// vertical trunk of brown splats and a rough-spherical green canopy, brighter at random
/// tips so the bloom catches them. `seed_pt` makes the tree deterministic.
fn plant_tree(out: &mut Vec<SplatInstance>, base: [f32; 3], seed_pt: u32) {
    let mut st = seed_pt | 1;
    let trunk_h = 3.5 + frand(&mut st) * 4.0;
    let sway_base = frand(&mut st) * std::f32::consts::TAU;

    // Trunk: splats climbing the stem.
    let steps = (trunk_h / 0.6).max(1.0) as u32;
    for i in 0..steps {
        let y = base[1] + i as f32 * 0.6;
        let j = (frand(&mut st) - 0.5) * 0.25;
        out.push(SplatInstance {
            offset: [base[0] + j, y, base[2] + j],
            size: 0.42 + frand(&mut st) * 0.12,
            color: [0.26 + frand(&mut st) * 0.12, 0.17, 0.10],
            sway: sway_base,
        });
    }

    // Canopy: a blob of green points above the trunk.
    let cx = base[0];
    let cy = base[1] + trunk_h + 0.6;
    let cz = base[2];
    let radius = 1.6 + frand(&mut st) * 1.3;
    let n = 16 + (frand(&mut st) * 14.0) as u32;
    for _ in 0..n {
        // Rejection-free spherical-ish offset (a cube biased toward the centre).
        let dx = (frand(&mut st) - 0.5) * 2.0 * radius;
        let dy = (frand(&mut st) - 0.5) * 2.0 * radius * 0.85;
        let dz = (frand(&mut st) - 0.5) * 2.0 * radius;
        let bright = frand(&mut st);
        // Lush greens; a few tips pushed bright to glow through bloom.
        let g = 0.5 + bright * 0.45;
        let glow = if bright > 0.82 { 0.25 } else { 0.0 };
        out.push(SplatInstance {
            offset: [cx + dx, cy + dy, cz + dz],
            size: 0.5 + frand(&mut st) * 0.4,
            color: [0.12 + glow, g + glow, 0.12 + glow * 0.5],
            sway: sway_base + frand(&mut st) * 0.6,
        });
    }
}

/// Scatter point-cloud **trees** over the grass columns of `section` (E7). Candidate
/// positions are a jittered grid (so woods look planted, not gridded); `forest` (0..1,
/// from biome lushness) sets how many candidates actually sprout. Deterministic.
pub fn scatter_trees(
    section: &Section,
    cx: i32,
    cz: i32,
    seed: u32,
    forest: f32,
) -> Vec<SplatInstance> {
    if forest <= 0.0 {
        return Vec::new();
    }
    let s = Section::SIZE as i32;
    let cell: u32 = 6; // grid spacing between candidate trees
    let mut out = Vec::new();
    let mut g = 0u32;
    while g * cell < Section::SIZE {
        let mut h = 0u32;
        while h * cell < Section::SIZE {
            let gx = g * cell;
            let gz = h * cell;
            let (wgx, wgz) = (cx * s + gx as i32, cz * s + gz as i32);
            // Jitter the candidate within its cell, then snap to a column.
            let jx = (hash01(wgx, wgz, seed ^ 0x7ee5) * (cell - 1) as f32) as u32;
            let jz = (hash01(wgx, wgz, seed ^ 0x3aa1) * (cell - 1) as f32) as u32;
            let (lx, lz) = (
                (gx + jx).min(Section::SIZE - 1),
                (gz + jz).min(Section::SIZE - 1),
            );
            h += 1;
            let Some(ty) = top_solid(section, lx, lz) else {
                continue;
            };
            if section.get(lx, ty, lz) != GRASS || ty + 1 >= Section::SIZE {
                continue;
            }
            let (wx, wz) = (cx * s + lx as i32, cz * s + lz as i32);
            // Forest-density roll: thicker woods in lusher biomes.
            if hash01(wx, wz, seed ^ 0x1f0e_57a3) > forest * 0.7 {
                continue;
            }
            plant_tree(
                &mut out,
                [wx as f32 + 0.5, (ty as f32) + 1.0, wz as f32 + 0.5],
                (wx as u32).wrapping_mul(0x9e37_79b9)
                    ^ (wz as u32).wrapping_mul(0x85eb_ca6b)
                    ^ seed,
            );
        }
        g += 1;
    }
    out
}

/// Scatter low **undergrowth** clumps (bushes / ferns) over grass — a mid tier between
/// the ground grass and the trees (E7 layered vegetation). Jittered grid, denser than
/// trees but sparser than grass; each clump is a few darker-green splats in a low dome.
/// Deterministic; `forest` (biome lushness) gates how many sprout.
pub fn scatter_bushes(
    section: &Section,
    cx: i32,
    cz: i32,
    seed: u32,
    forest: f32,
) -> Vec<SplatInstance> {
    if forest <= 0.0 {
        return Vec::new();
    }
    let s = Section::SIZE as i32;
    let cell: u32 = 4;
    let mut out = Vec::new();
    let mut g = 0u32;
    while g * cell < Section::SIZE {
        let mut k = 0u32;
        while k * cell < Section::SIZE {
            let (gx, gz) = (g * cell, k * cell);
            let (wgx, wgz) = (cx * s + gx as i32, cz * s + gz as i32);
            let jx = (hash01(wgx, wgz, seed ^ 0x2bb1) * (cell - 1) as f32) as u32;
            let jz = (hash01(wgx, wgz, seed ^ 0x9cc2) * (cell - 1) as f32) as u32;
            let (lx, lz) = (
                (gx + jx).min(Section::SIZE - 1),
                (gz + jz).min(Section::SIZE - 1),
            );
            k += 1;
            let Some(ty) = top_solid(section, lx, lz) else {
                continue;
            };
            if section.get(lx, ty, lz) != GRASS || ty + 1 >= Section::SIZE {
                continue;
            }
            let (wx, wz) = (cx * s + lx as i32, cz * s + lz as i32);
            if hash01(wx, wz, seed ^ 0x55de_4e21) > forest * 0.55 {
                continue;
            }
            let mut st = ((wx as u32).wrapping_mul(0x2545_f491)
                ^ (wz as u32).wrapping_mul(0x9e37_79b9)
                ^ seed)
                | 1;
            let base = [wx as f32 + 0.5, (ty as f32) + 1.0, wz as f32 + 0.5];
            let sway = frand(&mut st) * std::f32::consts::TAU;
            let count = 4 + (frand(&mut st) * 4.0) as u32;
            for _ in 0..count {
                let dx = (frand(&mut st) - 0.5) * 1.6;
                let dz = (frand(&mut st) - 0.5) * 1.6;
                let dy = frand(&mut st) * 1.2;
                let shade = frand(&mut st);
                out.push(SplatInstance {
                    offset: [base[0] + dx, base[1] + 0.3 + dy, base[2] + dz],
                    size: 0.42 + frand(&mut st) * 0.3,
                    // Darker, bluer greens than the grass so the tiers read apart.
                    color: [
                        0.10 + shade * 0.12,
                        0.34 + shade * 0.30,
                        0.16 + shade * 0.10,
                    ],
                    sway,
                });
            }
        }
        g += 1;
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
    fn trees_are_deterministic_and_gated_by_forest() {
        let s = grass_section();
        let a = scatter_trees(&s, 0, 0, 42, 1.0);
        let b = scatter_trees(&s, 0, 0, 42, 1.0);
        assert_eq!(a, b);
        assert!(
            !a.is_empty(),
            "a full-forest grass chunk should plant trees"
        );
        assert!(
            scatter_trees(&s, 0, 0, 42, 0.0).is_empty(),
            "no forest → no trees"
        );
    }

    #[test]
    fn bushes_deterministic_grass_gated() {
        let s = grass_section();
        let a = scatter_bushes(&s, 0, 0, 9, 1.0);
        assert_eq!(a, scatter_bushes(&s, 0, 0, 9, 1.0));
        assert!(!a.is_empty());
        assert!(scatter_bushes(&s, 0, 0, 9, 0.0).is_empty());
        // Not on stone.
        let mut stone = Section::new();
        for z in 0..Section::SIZE {
            for x in 0..Section::SIZE {
                stone.set(x, 0, z, BlockId(1));
            }
        }
        assert!(scatter_bushes(&stone, 0, 0, 9, 1.0).is_empty());
    }

    #[test]
    fn trees_only_on_grass() {
        let mut s = Section::new();
        for z in 0..Section::SIZE {
            for x in 0..Section::SIZE {
                s.set(x, 0, z, BlockId(1)); // stone
            }
        }
        assert!(scatter_trees(&s, 0, 0, 1, 1.0).is_empty());
    }

    #[test]
    fn tree_canopy_rises_above_the_trunk() {
        // A planted tree's highest splat should be well above the ground it sits on.
        let s = grass_section();
        let trees = scatter_trees(&s, 0, 0, 42, 1.0);
        let top = trees.iter().map(|t| t.offset[1]).fold(0.0_f32, f32::max);
        assert!(top > 5.0, "canopy should rise above the surface, got {top}");
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
