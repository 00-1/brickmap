//! Colossal bodies (E18) — giant human figures in the world. This module builds them
//! procedurally (a capsule/sphere blockout, no external assets yet) and emits their **surface
//! voxels as points** ([`crate::foliage::SplatInstance`]), so they render through the existing
//! splat billboard pipeline: an **ethereal, drifting point-form** you can fly through (there's
//! no collision). A later stage voxelises real CC0 anatomy and adds the solid, explorable kind.
//!
//! Pure logic (no wgpu): given a placement, return the points. Deterministic from the seed.

use glam::Vec3;

use crate::foliage::SplatInstance;

/// Model-space dimensions of the humanoid occupancy grid (units; scaled to world by `voxel`).
const GW: i32 = 36; // width (x)
const GH: i32 = 88; // height (y), feet at 0
const GD: i32 = 18; // depth (z)

/// Distance from point `p` to segment `a`–`b` (model space).
fn seg_dist(p: Vec3, a: Vec3, b: Vec3) -> f32 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_squared().max(1e-6)).clamp(0.0, 1.0);
    (p - (a + ab * t)).length()
}

/// Is model-space cell `(x,y,z)` inside the humanoid blockout? Built from a few capsules
/// (limbs, torso, neck) + a head sphere — a deliberately rough, readable figure.
fn solid(x: i32, y: i32, z: i32) -> bool {
    let p = Vec3::new(x as f32, y as f32, z as f32);
    let cx = GW as f32 * 0.5;
    let cz = GD as f32 * 0.5;
    let c = |px: f32, py: f32, pz: f32| Vec3::new(px, py, pz);

    // Legs (feet at y≈2 → hips at y≈44), slightly apart.
    if seg_dist(p, c(cx - 5.0, 2.0, cz), c(cx - 5.5, 44.0, cz)) <= 5.0 {
        return true;
    }
    if seg_dist(p, c(cx + 5.0, 2.0, cz), c(cx + 5.5, 44.0, cz)) <= 5.0 {
        return true;
    }
    // Pelvis + torso (a fat capsule, tapering implied by the radius).
    if seg_dist(p, c(cx, 44.0, cz), c(cx, 66.0, cz)) <= 10.5 {
        return true;
    }
    // Arms, hanging from the shoulders (y≈64) down past the hips.
    if seg_dist(p, c(cx - 10.0, 64.0, cz), c(cx - 15.0, 40.0, cz)) <= 3.6 {
        return true;
    }
    if seg_dist(p, c(cx + 10.0, 64.0, cz), c(cx + 15.0, 40.0, cz)) <= 3.6 {
        return true;
    }
    // Neck.
    if seg_dist(p, c(cx, 66.0, cz), c(cx, 72.0, cz)) <= 3.0 {
        return true;
    }
    // Head.
    if (p - c(cx, 79.0, cz)).length() <= 7.5 {
        return true;
    }
    false
}

/// Build the ethereal point-form of a colossal humanoid standing with its feet at `feet`
/// (world space), `voxel` world-units per model unit (so total height ≈ `GH * voxel`). Emits
/// one point per **surface** model cell (a solid cell with at least one empty 6-neighbour),
/// tinted a cool pale `color` with slight per-point variation. `seed` jitters the variation.
pub fn humanoid_points(feet: Vec3, voxel: f32, color: [f32; 3], seed: u32) -> Vec<SplatInstance> {
    let cx = GW as f32 * 0.5;
    let cz = GD as f32 * 0.5;
    let mut out = Vec::new();
    let mut h = seed | 1;
    let mut rnd = || {
        // xorshift32 → [0,1)
        h ^= h << 13;
        h ^= h >> 17;
        h ^= h << 5;
        (h >> 8) as f32 / (1u32 << 24) as f32
    };
    for y in 0..GH {
        for z in 0..GD {
            for x in 0..GW {
                if !solid(x, y, z) {
                    continue;
                }
                // Surface only: keep cells exposed to air on a face (interior is hidden).
                let exposed = !solid(x - 1, y, z)
                    || !solid(x + 1, y, z)
                    || !solid(x, y - 1, z)
                    || !solid(x, y + 1, z)
                    || !solid(x, y, z - 1)
                    || !solid(x, y, z + 1);
                if !exposed {
                    continue;
                }
                let world = feet
                    + Vec3::new(
                        (x as f32 - cx) * voxel,
                        y as f32 * voxel,
                        (z as f32 - cz) * voxel,
                    );
                let v = 0.85 + 0.15 * rnd();
                out.push(SplatInstance {
                    offset: [world.x, world.y, world.z],
                    size: voxel * 0.55,
                    color: [color[0] * v, color[1] * v, color[2] * v],
                    sway: rnd() * std::f32::consts::TAU,
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn humanoid_has_a_surface_shell() {
        let pts = humanoid_points(Vec3::ZERO, 1.0, [0.6, 0.7, 0.85], 7);
        assert!(pts.len() > 500, "too few surface points: {}", pts.len());
        // Interior is culled: far fewer points than the solid volume (sanity on "surface").
        let solid_count = (0..GH)
            .flat_map(|y| (0..GD).flat_map(move |z| (0..GW).map(move |x| (x, y, z))))
            .filter(|&(x, y, z)| solid(x, y, z))
            .count();
        assert!(
            pts.len() < solid_count,
            "surface should be a shell of the volume"
        );
    }

    #[test]
    fn humanoid_is_left_right_symmetric() {
        let pts = humanoid_points(Vec3::ZERO, 1.0, [1.0; 3], 1);
        let (mut left, mut right) = (0i32, 0i32);
        for p in &pts {
            if p.offset[0] < -0.5 {
                left += 1;
            } else if p.offset[0] > 0.5 {
                right += 1;
            }
        }
        // Roughly mirror-symmetric about x = 0 (centre).
        let diff = (left - right).abs();
        assert!(diff < left.max(right) / 5, "lopsided: L={left} R={right}");
    }

    #[test]
    fn taller_voxel_scales_the_figure() {
        let small = humanoid_points(Vec3::ZERO, 1.0, [1.0; 3], 1);
        let big = humanoid_points(Vec3::ZERO, 2.0, [1.0; 3], 1);
        let top = |v: &[SplatInstance]| v.iter().map(|p| p.offset[1]).fold(0.0_f32, f32::max);
        assert!(
            top(&big) > top(&small) * 1.8,
            "voxel size should scale height"
        );
    }
}
