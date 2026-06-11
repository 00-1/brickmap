//! G10 — **typed shards**: the collectible upgrade currency. Seed-scattered world items on
//! their own grid (denser than inscriptions — they're the bulk currency), each typed by one of
//! the five strata **domains** and graded by **rarity** (common/uncommon/rare ≈ 85/13/2,
//! yield 1/3/9). Rendered as small emissive splat clusters, domain-tinted, with rarity scaling
//! glow + size (a rare one glints at distance). Pure + deterministic; the app streams them
//! near the camera like inscriptions/colossi and the collect routes pick them up.

use glam::Vec3;

use crate::foliage::SplatInstance;
use crate::progress::Stratum;

/// Shard rarity — the excitement axis (volume stays calm; rarity does the work).
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Rarity {
    Common,
    Uncommon,
    Rare,
}

impl Rarity {
    pub const ALL: [Rarity; 3] = [Rarity::Common, Rarity::Uncommon, Rarity::Rare];
    /// Bank yield on collect (the spend currency).
    pub fn yield_amount(self) -> u64 {
        match self {
            Rarity::Common => 1,
            Rarity::Uncommon => 3,
            Rarity::Rare => 9,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Rarity::Common => "common",
            Rarity::Uncommon => "uncommon",
            Rarity::Rare => "rare",
        }
    }
    /// Index for the progress count table.
    pub fn idx(self) -> usize {
        match self {
            Rarity::Common => 0,
            Rarity::Uncommon => 1,
            Rarity::Rare => 2,
        }
    }
}

/// One world shard: where it sits, its domain (stratum) + rarity. The cell is the dedup key.
#[derive(Copy, Clone, Debug)]
pub struct Shard {
    pub cell: (i32, i32),
    pub pos: Vec3,
    pub domain: Stratum,
    pub rarity: Rarity,
}

/// Grid spacing (world units) — denser than the inscriptions' 82 (bulk currency).
const CELL: f32 = 46.0;
/// Fraction of cells holding a shard (conservative: volume stays calm — charter §4).
const PRESENCE: f32 = 0.45;

fn hash(cx: i32, cz: i32, seed: u32) -> u32 {
    let mut h = (cx as u32).wrapping_mul(0x9E37_79B1)
        ^ (cz as u32).wrapping_mul(0x85EB_CA6B)
        ^ seed.wrapping_mul(0xC2B2_AE35);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 13;
    h
}

/// All shards within `radius` of `cam` (XZ), deterministic in `seed`; `ground` drops each just
/// above the terrain. Mirrors `structures::inscriptions_near`'s scheme.
pub fn shards_near(
    seed: u32,
    cam: Vec3,
    radius: f32,
    ground: impl Fn(f32, f32) -> f32,
) -> Vec<Shard> {
    let reach = (radius / CELL).ceil() as i32 + 1;
    let (ccx, ccz) = ((cam.x / CELL).floor() as i32, (cam.z / CELL).floor() as i32);
    let mut out = Vec::new();
    for cz in (ccz - reach)..=(ccz + reach) {
        for cx in (ccx - reach)..=(ccx + reach) {
            let h = hash(cx, cz, seed.wrapping_add(0x5AAD_0000));
            if (h & 0xFFFF) as f32 / 65536.0 >= PRESENCE {
                continue;
            }
            let jx = ((h >> 16) & 0xFF) as f32 / 255.0;
            let jz = ((h >> 24) & 0xFF) as f32 / 255.0;
            let x = cx as f32 * CELL + jx * CELL;
            let z = cz as f32 * CELL + jz * CELL;
            if (x - cam.x).powi(2) + (z - cam.z).powi(2) > radius * radius {
                continue;
            }
            // Domain: uniform over the five strata. Rarity: ~85/13/2.
            let domain = Stratum::ALL[((h >> 8) % 5) as usize];
            let r = (h >> 11) % 100;
            let rarity = if r < 85 {
                Rarity::Common
            } else if r < 98 {
                Rarity::Uncommon
            } else {
                Rarity::Rare
            };
            out.push(Shard {
                cell: (cx, cz),
                pos: Vec3::new(x, ground(x, z) + 0.8, z),
                domain,
                rarity,
            });
        }
    }
    out
}

/// The domain's emissive tint — the same colour family as its script's inscriptions, so a
/// shard reads as "a fleck of that stratum".
pub fn domain_tint(d: Stratum) -> [f32; 3] {
    match d {
        Stratum::Records => [0.95, 0.62, 0.25],    // amber (Latin)
        Stratum::Schematics => [0.40, 0.85, 0.95], // cyan (Greek)
        Stratum::Rites => [0.55, 0.95, 0.45],      // green (Hiragana)
        Stratum::Relics => [0.75, 0.50, 0.95],     // violet (Runic)
        Stratum::Signals => [0.92, 0.95, 1.0],     // pale signal-white (Galactic)
    }
}

/// Render one shard as a small emissive cluster (3 + rarity·2 points), domain-tinted, rarity
/// scaling size + glow so a rare one glints from far off. Deterministic per cell.
pub fn splats(shard: &Shard) -> Vec<SplatInstance> {
    let (n, size, glow) = match shard.rarity {
        Rarity::Common => (3, 0.22, 0.65),
        Rarity::Uncommon => (5, 0.30, 0.95),
        Rarity::Rare => (7, 0.42, 1.45),
    };
    let tint = domain_tint(shard.domain);
    let mut h = hash(shard.cell.0, shard.cell.1, 0x51AB);
    let mut rng = move || {
        h ^= h << 13;
        h ^= h >> 17;
        h ^= h << 5;
        (h & 0xFFFF) as f32 / 65536.0
    };
    (0..n)
        .map(|_| {
            let dx = (rng() - 0.5) * 1.4;
            let dy = rng() * 0.9;
            let dz = (rng() - 0.5) * 1.4;
            SplatInstance {
                offset: [shard.pos.x + dx, shard.pos.y + dy, shard.pos.z + dz],
                size,
                color: [tint[0] * glow, tint[1] * glow, tint[2] * glow],
                sway: rng() * std::f32::consts::TAU,
                alpha: 1.0,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scatter_is_deterministic_and_density_bounded() {
        let g = |_x: f32, _z: f32| 10.0;
        let a = shards_near(1337, Vec3::ZERO, 400.0, g);
        let b = shards_near(1337, Vec3::ZERO, 400.0, g);
        assert_eq!(a.len(), b.len());
        assert!(!a.is_empty());
        for (x, y) in a.iter().zip(&b) {
            assert_eq!(x.cell, y.cell);
            assert_eq!(x.domain, y.domain);
            assert_eq!(x.rarity, y.rarity);
        }
        // Density bound: ≤ PRESENCE × the cell count in the disc (with a grid-edge margin).
        let cells = (2.0 * 400.0 / CELL + 3.0).powi(2);
        assert!(
            (a.len() as f32) < cells * PRESENCE,
            "shard density out of bounds: {} of ~{cells} cells",
            a.len()
        );
    }

    #[test]
    fn rarity_distribution_within_tolerance() {
        // Over a wide area the seeded rarity split should sit near 85/13/2.
        let g = |_x: f32, _z: f32| 0.0;
        let all = shards_near(42, Vec3::ZERO, 2500.0, g);
        let n = all.len() as f32;
        assert!(n > 1000.0, "need a large sample, got {n}");
        let frac = |r: Rarity| all.iter().filter(|s| s.rarity == r).count() as f32 / n;
        assert!((frac(Rarity::Common) - 0.85).abs() < 0.05);
        assert!((frac(Rarity::Uncommon) - 0.13).abs() < 0.04);
        assert!((frac(Rarity::Rare) - 0.02).abs() < 0.02);
        // All five domains appear.
        for d in Stratum::ALL {
            assert!(all.iter().any(|s| s.domain == d), "missing domain {d:?}");
        }
    }

    #[test]
    fn splats_scale_with_rarity() {
        let mk = |rarity| Shard {
            cell: (3, -2),
            pos: Vec3::new(1.0, 5.0, 2.0),
            domain: Stratum::Relics,
            rarity,
        };
        let c = splats(&mk(Rarity::Common));
        let r = splats(&mk(Rarity::Rare));
        assert!(r.len() > c.len(), "rare clusters are bigger");
        assert!(r[0].size > c[0].size, "rare points are larger");
        // Deterministic per cell.
        assert_eq!(splats(&mk(Rarity::Rare))[0].offset, r[0].offset);
    }
}
