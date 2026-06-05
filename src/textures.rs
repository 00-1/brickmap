//! Procedural material textures (M4). Dependency-free: a small per-material detail
//! tile generated from hashed value noise. The tiles are **grayscale detail** (a
//! brightness multiplier ~`[lo, hi]`); the shader tints them with the material's
//! palette colour, so there's no colour duplicated here. Pure data → reproducible
//! for golden images. Texture-array **layer index == material id**.

/// Tile edge length (texels). Small + nearest-sampled + tiled per voxel.
pub const TILE: u32 = 16;
/// One layer per palette slot (mirrors `PALETTE` in `gfx`/`headless`).
pub const LAYERS: u32 = 8;

/// Per-material look: lattice cell size (bigger = blotchier), brightness range, and
/// how many quantised levels (fewer = more posterised, leans into the §11 look).
/// Indexed by material id. 0/6/7 are unused materials → flat white (loud magenta in
/// the palette shows through untouched).
struct Style {
    cell: u32,
    lo: f32,
    hi: f32,
    levels: u32,
}

const STYLES: [Style; LAYERS as usize] = [
    Style {
        cell: 1,
        lo: 1.0,
        hi: 1.0,
        levels: 1,
    }, // 0 unused
    Style {
        cell: 4,
        lo: 0.62,
        hi: 1.0,
        levels: 4,
    }, // 1 stone: coarse speckle
    Style {
        cell: 3,
        lo: 0.68,
        hi: 1.0,
        levels: 4,
    }, // 2 dirt: medium grain
    Style {
        cell: 2,
        lo: 0.72,
        hi: 1.0,
        levels: 5,
    }, // 3 grass: fine, busy
    Style {
        cell: 2,
        lo: 0.80,
        hi: 1.0,
        levels: 5,
    }, // 4 sand: fine, soft
    Style {
        cell: 5,
        lo: 0.88,
        hi: 1.0,
        levels: 3,
    }, // 5 snow: subtle
    Style {
        cell: 1,
        lo: 1.0,
        hi: 1.0,
        levels: 1,
    }, // 6 unused
    Style {
        cell: 1,
        lo: 1.0,
        hi: 1.0,
        levels: 1,
    }, // 7 unused
];

/// Hash a lattice point to `[0, 1)` (same shape as `worldgen::hash`, separate seed).
fn hash(x: i32, y: i32, layer: u32) -> f32 {
    let mut h = (x as u32)
        .wrapping_mul(0x27d4_eb2d)
        .wrapping_add((y as u32).wrapping_mul(0x1656_67b1))
        .wrapping_add(layer.wrapping_mul(0x9e37_79b9))
        .wrapping_add(0x85eb_ca6b);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 13;
    h = h.wrapping_mul(0x297a_2d39);
    h ^= h >> 16;
    (h & 0x00ff_ffff) as f32 / 0x0100_0000 as f32
}

fn smoothstep(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

/// Bilinearly-interpolated value noise on a `cell`-sized lattice, wrapping at the
/// tile edge so the tile is seamless when repeated per voxel.
fn tile_noise(x: u32, y: u32, layer: u32, cell: u32) -> f32 {
    let cells = (TILE / cell).max(1) as i32; // lattice points across the tile
    let fx = x as f32 / cell as f32;
    let fy = y as f32 / cell as f32;
    let (xi, yi) = (fx.floor() as i32, fy.floor() as i32);
    let (xf, yf) = (fx - xi as f32, fy - yi as f32);
    let (u, v) = (smoothstep(xf), smoothstep(yf));
    // Wrap lattice indices so the right/bottom edge meets the left/top.
    let w = |a: i32| a.rem_euclid(cells);
    let h00 = hash(w(xi), w(yi), layer);
    let h10 = hash(w(xi + 1), w(yi), layer);
    let h01 = hash(w(xi), w(yi + 1), layer);
    let h11 = hash(w(xi + 1), w(yi + 1), layer);
    let top = h00 + (h10 - h00) * u;
    let bot = h01 + (h11 - h01) * u;
    top + (bot - top) * v
}

/// Build the full material texture array as tightly-packed RGBA8: `LAYERS` layers of
/// `TILE×TILE`, grayscale detail (a brightness multiplier) in rgb, alpha 255. Row
/// stride is `TILE*4`; layers are contiguous (the layout `write_texture` expects).
pub fn material_atlas() -> Vec<u8> {
    let mut data = Vec::with_capacity((LAYERS * TILE * TILE * 4) as usize);
    for layer in 0..LAYERS {
        let s = &STYLES[layer as usize];
        for y in 0..TILE {
            for x in 0..TILE {
                let n = tile_noise(x, y, layer, s.cell);
                // Quantise into `levels` bands, then map to [lo, hi].
                let q = if s.levels <= 1 {
                    1.0
                } else {
                    let band = (n * s.levels as f32).floor().min((s.levels - 1) as f32);
                    band / (s.levels - 1) as f32
                };
                let b = s.lo + (s.hi - s.lo) * q;
                let v = (b.clamp(0.0, 1.0) * 255.0).round() as u8;
                data.extend_from_slice(&[v, v, v, 255]);
            }
        }
    }
    data
}

/// Number of mip levels for the tile (`TILE`, `TILE/2`, … , `1`).
pub fn mip_levels() -> u32 {
    TILE.trailing_zeros() + 1
}

/// Full mip chain for the material array: level 0 is [`material_atlas`], each
/// subsequent level is a 2×2 box-downsample of the previous (per layer, so layers
/// never bleed into each other). Nearest-sampled at runtime → chunky LOD that suits
/// the look while killing distant shimmer. Each `Vec` is `LAYERS` contiguous layers.
pub fn material_mip_chain() -> Vec<Vec<u8>> {
    let mut chain = vec![material_atlas()];
    let mut size = TILE;
    while size > 1 {
        let prev = chain.last().unwrap();
        let half = size / 2;
        let mut next = Vec::with_capacity((LAYERS * half * half * 4) as usize);
        for layer in 0..LAYERS {
            let base = (layer * size * size * 4) as usize;
            for y in 0..half {
                for x in 0..half {
                    let mut acc = [0u32; 4];
                    for dy in 0..2 {
                        for dx in 0..2 {
                            let (sx, sy) = (x * 2 + dx, y * 2 + dy);
                            let idx = base + ((sy * size + sx) * 4) as usize;
                            for (c, a) in acc.iter_mut().enumerate() {
                                *a += prev[idx + c] as u32;
                            }
                        }
                    }
                    for a in acc {
                        next.push((a / 4) as u8);
                    }
                }
            }
        }
        chain.push(next);
        size = half;
    }
    chain
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atlas_has_the_expected_size_and_alpha() {
        let data = material_atlas();
        assert_eq!(data.len(), (LAYERS * TILE * TILE * 4) as usize);
        // Every texel is opaque grayscale (r == g == b).
        for px in data.chunks_exact(4) {
            assert_eq!(px[0], px[1]);
            assert_eq!(px[1], px[2]);
            assert_eq!(px[3], 255);
        }
    }

    #[test]
    fn is_deterministic() {
        assert_eq!(material_atlas(), material_atlas());
    }

    #[test]
    fn mip_chain_halves_to_one_texel() {
        let chain = material_mip_chain();
        assert_eq!(chain.len(), mip_levels() as usize);
        for (level, data) in chain.iter().enumerate() {
            let s = TILE >> level;
            assert_eq!(data.len(), (LAYERS * s * s * 4) as usize, "level {level}");
        }
        // Last level is one texel per layer.
        assert_eq!(chain.last().unwrap().len(), (LAYERS * 4) as usize);
    }

    #[test]
    fn textured_materials_vary_but_unused_are_flat() {
        let data = material_atlas();
        let layer = |i: u32| {
            let start = (i * TILE * TILE * 4) as usize;
            data[start..start + (TILE * TILE * 4) as usize].to_vec()
        };
        // A textured material (stone, layer 1) has more than one brightness value.
        let stone = layer(1);
        let first = stone[0];
        assert!(
            stone.chunks_exact(4).any(|p| p[0] != first),
            "stone should have texture variation",
        );
        // An unused layer (0) is flat white.
        assert!(layer(0).chunks_exact(4).all(|p| p[0] == 255));
    }
}
