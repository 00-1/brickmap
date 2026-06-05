//! Procedural terrain (M3): dependency-free fractal **value noise**. Deterministic
//! (seeded) so renders/golden-images are reproducible. Knows only `world` types.

use crate::world::{BlockId, Section, World};

const STONE: BlockId = BlockId(1);
const DIRT: BlockId = BlockId(2);
const GRASS: BlockId = BlockId(3);
const SAND: BlockId = BlockId(4);
const SNOW: BlockId = BlockId(5);
/// Rare emissive crystal that sits on the surface and glows (feeds bloom, E3).
const CRYSTAL: BlockId = BlockId(6);
/// Fraction of columns that sprout a surface crystal (rare — they should feel special).
const CRYSTAL_CHANCE: f32 = 0.0022;

/// Hash a lattice point to `[0, 1)`.
fn hash(x: i32, z: i32, seed: u32) -> f32 {
    let mut h = (x as u32)
        .wrapping_mul(0x1657_4c2f)
        .wrapping_add((z as u32).wrapping_mul(0x68b3_8d2b))
        .wrapping_add(seed.wrapping_mul(0x9e37_79b9));
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297a_2d39);
    h ^= h >> 15;
    (h & 0x00ff_ffff) as f32 / 0x0100_0000 as f32
}

fn smoothstep(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Bilinearly-interpolated value noise at `(x, z)`.
fn value_noise(x: f32, z: f32, seed: u32) -> f32 {
    let (xi, zi) = (x.floor() as i32, z.floor() as i32);
    let (xf, zf) = (x - xi as f32, z - zi as f32);
    let (u, v) = (smoothstep(xf), smoothstep(zf));
    let top = lerp(hash(xi, zi, seed), hash(xi + 1, zi, seed), u);
    let bot = lerp(hash(xi, zi + 1, seed), hash(xi + 1, zi + 1, seed), u);
    lerp(top, bot, v)
}

/// Fractal Brownian motion (a few octaves) → `[0, 1)`.
fn fbm(x: f32, z: f32, seed: u32) -> f32 {
    let mut sum = 0.0;
    let mut amp = 1.0;
    let mut freq = 1.0;
    let mut norm = 0.0;
    for octave in 0..4 {
        sum += amp * value_noise(x * freq, z * freq, seed.wrapping_add(octave * 1013));
        norm += amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    sum / norm
}

/// Surface height (in voxels) at a world column.
pub fn height(wx: i32, wz: i32, seed: u32) -> u32 {
    let h = fbm(wx as f32 * 0.028, wz as f32 * 0.028, seed);
    // Bias toward lower ground with occasional peaks.
    let shaped = h * h * (3.0 - 2.0 * h);
    (3.0 + shaped * 26.0)
        .round()
        .clamp(1.0, (Section::SIZE - 1) as f32) as u32
}

/// The surface block at a column, by height band (sand low, grass mid, snow high).
fn surface_block(height: u32) -> BlockId {
    match height {
        0..=6 => SAND,
        7..=22 => GRASS,
        _ => SNOW,
    }
}

/// Fill one section at chunk `(cx, cz)` with seeded terrain (single vertical layer
/// of chunks for now; `cy` is assumed 0).
pub fn generate_section(cx: i32, cz: i32, seed: u32) -> Section {
    let s = Section::SIZE as i32;
    let mut section = Section::new();
    for z in 0..Section::SIZE {
        for x in 0..Section::SIZE {
            let (wx, wz) = (cx * s + x as i32, cz * s + z as i32);
            let h = height(wx, wz, seed);
            let surface = surface_block(h);
            for y in 0..h.min(Section::SIZE) {
                let depth = h - y;
                let block = match depth {
                    1 => surface,
                    2..=3 => DIRT,
                    _ => STONE,
                };
                section.set(x, y, z, block);
            }
            // A rare glowing crystal perched on the surface (one voxel above the top
            // solid), as long as it fits in the section.
            if h < Section::SIZE && hash(wx, wz, seed.wrapping_add(0x5151)) < CRYSTAL_CHANCE {
                section.set(x, h, z, CRYSTAL);
            }
        }
    }
    section
}

/// Generate a square world of `radius` chunks around the origin.
pub fn generate_world(radius: i32, seed: u32) -> World {
    let mut world = World::new();
    for cz in -radius..=radius {
        for cx in -radius..=radius {
            world.insert((cx, 0, cz), generate_section(cx, cz, seed));
        }
    }
    world
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generation_is_deterministic() {
        let a = generate_section(2, -1, 1234);
        let b = generate_section(2, -1, 1234);
        for z in 0..Section::SIZE {
            for y in 0..Section::SIZE {
                for x in 0..Section::SIZE {
                    assert_eq!(a.get(x, y, z), b.get(x, y, z));
                }
            }
        }
    }

    /// A stable FNV-1a hash over the block ids of a fixed 5×5 chunk grid — a compact
    /// fingerprint of the whole generated world for a seed. Uses only integer block ids
    /// (the portable part of worldgen, per the determinism caveat in the E12 brief).
    fn voxel_hash(seed: u32) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325; // FNV offset basis
        for cz in -2..=2 {
            for cx in -2..=2 {
                let sec = generate_section(cx, cz, seed);
                for z in 0..Section::SIZE {
                    for y in 0..Section::SIZE {
                        for x in 0..Section::SIZE {
                            h ^= sec.get(x, y, z).0 as u64;
                            h = h.wrapping_mul(0x0000_0100_0000_01b3); // FNV prime
                        }
                    }
                }
            }
        }
        h
    }

    #[test]
    fn golden_voxel_hash_is_stable() {
        // Guards against accidental worldgen changes (a known seed must keep hashing to
        // the same world). If worldgen *intentionally* changes, bump this constant and
        // the seed/worldgen version (E12 brief §5 caveat). The integer block-id path is
        // portable; the f32 noise feeding `height().round()` *may* drift across targets,
        // which is why a cross-target (wasm-in-CI) check is noted as a follow-up.
        assert_eq!(voxel_hash(1337), 147_136_511_805_429_047);
        // Different seeds must give different worlds.
        assert_ne!(voxel_hash(1337), voxel_hash(1338));
        assert_ne!(voxel_hash(0), voxel_hash(1));
    }

    #[test]
    fn terrain_has_a_grass_or_surface_top_and_is_non_empty() {
        let s = generate_section(0, 0, 7);
        assert!(!s.is_empty());
        // The top non-air voxel of a column should be a surface block, not stone.
        let mut checked = 0;
        for z in 0..Section::SIZE {
            for x in 0..Section::SIZE {
                let h = height(x as i32, z as i32, 7);
                let top = h.min(Section::SIZE) - 1;
                let b = s.get(x, top, z);
                assert!(b.is_solid());
                assert!(b == GRASS || b == SAND || b == SNOW, "top was {b:?}");
                checked += 1;
            }
        }
        assert!(checked > 0);
    }
}
