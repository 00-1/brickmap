//! World structures (E18) — large, seed-driven features placed *independently of chunk
//! terrain*: the first kind is the **fallen colossi** (giant collapsed figures from `body`).
//! A structure is positioned deterministically from the world seed on a coarse cell grid, so
//! the same seed always strews the same giants across the (infinite, streamed) world. More
//! structure kinds can join later behind the same placement scheme.

use glam::Vec3;

use crate::relic::Placement;

/// World-unit spacing of the candidate grid (one possible colossus per cell).
const CELL: f32 = 200.0;
/// Fraction of cells that actually hold a colossus (kept sparse so they feel monumental).
const PRESENCE: f32 = 0.4;

/// Hash a cell `(cx, cz)` + seed → a well-mixed `u32`.
fn hash(cx: i32, cz: i32, seed: u32) -> u32 {
    let mut h = (cx as u32).wrapping_mul(0x8DA6_B343)
        ^ (cz as u32).wrapping_mul(0xD8163841)
        ^ seed.wrapping_mul(0x9E37_79B1);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 13;
    h
}

/// All colossus placements whose anchor is within `radius` (world units, in the XZ plane) of
/// `cam`. Deterministic in `seed`; `ground(x, z)` drops each onto the terrain surface. Used by
/// the live app to stream giants in/out around the camera.
pub fn colossi_near(
    seed: u32,
    cam: Vec3,
    radius: f32,
    ground: impl Fn(f32, f32) -> f32,
) -> Vec<Placement> {
    let reach = (radius / CELL).ceil() as i32 + 1;
    let (ccx, ccz) = ((cam.x / CELL).floor() as i32, (cam.z / CELL).floor() as i32);
    let mut out = Vec::new();
    for cz in (ccz - reach)..=(ccz + reach) {
        for cx in (ccx - reach)..=(ccx + reach) {
            let h = hash(cx, cz, seed);
            if (h & 0xFFFF) as f32 / 65536.0 >= PRESENCE {
                continue;
            }
            // Jitter the anchor within the cell so they don't sit on a visible lattice.
            let jx = ((h >> 16) & 0xFF) as f32 / 255.0;
            let jz = ((h >> 24) & 0xFF) as f32 / 255.0;
            let x = cx as f32 * CELL + jx * CELL;
            let z = cz as f32 * CELL + jz * CELL;
            let dx = x - cam.x;
            let dz = z - cam.z;
            if dx * dx + dz * dz > radius * radius {
                continue;
            }
            let p = hash(cx ^ 0x5A5A, cz ^ 0x3C3C, seed);
            out.push(Placement {
                pos: Vec3::new(x, ground(x, z), z),
                yaw: (p % 6283) as f32 / 1000.0,
                voxel: 1.15 + ((p >> 5) % 70) as f32 / 100.0, // ~95–155 world units across
                seed: p | 1,
                solid: (p >> 12).is_multiple_of(3), // ~1 in 3 is a solid, explorable relic
            });
        }
    }
    out
}

/// A stable key for a placement's cell, so the live app can detect when the in-range set
/// changes (and only then rebuild the giants' point buffer).
pub fn cell_key(p: &Placement) -> (i32, i32) {
    (
        (p.pos.x / CELL).floor() as i32,
        (p.pos.z / CELL).floor() as i32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_and_within_radius() {
        let g = |_x: f32, _z: f32| 0.0;
        let cam = Vec3::new(123.0, 0.0, -77.0);
        let a = colossi_near(1337, cam, 500.0, g);
        let b = colossi_near(1337, cam, 500.0, g);
        assert_eq!(a.len(), b.len());
        assert!(!a.is_empty(), "some colossi should be in a 500-unit radius");
        for (p, q) in a.iter().zip(&b) {
            assert_eq!(cell_key(p), cell_key(q));
            let d2 = (p.pos.x - cam.x).powi(2) + (p.pos.z - cam.z).powi(2);
            assert!(d2 <= 500.0 * 500.0 + 1.0, "placement outside radius");
        }
    }

    #[test]
    fn different_seeds_place_differently() {
        let g = |_x: f32, _z: f32| 0.0;
        let cam = Vec3::ZERO;
        let a = colossi_near(1, cam, 600.0, g);
        let b = colossi_near(2, cam, 600.0, g);
        let keys_a: Vec<_> = a.iter().map(cell_key).collect();
        let keys_b: Vec<_> = b.iter().map(cell_key).collect();
        assert_ne!(keys_a, keys_b);
    }
}
