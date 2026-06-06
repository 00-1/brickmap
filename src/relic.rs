//! Colossal **tube-tech relics** (E18) — giant ancient-machine structures: wild, non-human
//! tangles of tubes (pipes, girders, spars), the "limbs everywhere" look. Placed in the world
//! as `structures` (seed-driven, independent of chunk terrain). Generated procedurally as a
//! union of capsules and emitted as **surface points** ([`crate::foliage::SplatInstance`],
//! ethereal/drift-through) or **solid voxels** (greedy-meshed, explorable). The other E18 kind,
//! real human figures from CC0 models, is a separate track. Pure logic; deterministic in seed.

use glam::Vec3;

use crate::foliage::SplatInstance;

/// Small deterministic PRNG (xorshift32).
struct Rng(u32);
impl Rng {
    fn new(seed: u32) -> Rng {
        Rng(seed | 1)
    }
    fn unit(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 >> 8) as f32 / (1u32 << 24) as f32
    }
    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.unit() * (hi - lo)
    }
}

/// A tube (segment `a`–`b`, radius `r`). A relic is a union of these.
struct Capsule {
    a: Vec3,
    b: Vec3,
    r: f32,
}

fn seg_dist(p: Vec3, a: Vec3, b: Vec3) -> f32 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_squared().max(1e-6)).clamp(0.0, 1.0);
    (p - (a + ab * t)).length()
}

/// A near-axis unit direction with jitter — the angular, pipework/girder feel (vs organic).
fn axis_dir(rng: &mut Rng) -> Vec3 {
    let base = match (rng.range(0.0, 3.0)) as i32 {
        0 => Vec3::X,
        1 => Vec3::Y,
        _ => Vec3::Z,
    };
    let s = if rng.unit() < 0.5 { -1.0 } else { 1.0 };
    let jitter = Vec3::new(
        rng.range(-0.35, 0.35),
        rng.range(-0.35, 0.35),
        rng.range(-0.35, 0.35),
    );
    (base * s + jitter).normalize_or_zero()
}

/// Build a wild tube-tangle (model space, ~standing in a bounded sprawl) from `seed`: a few hub
/// nodes joined by thick girders, near-axis pipe runs branching off each (often with an elbow),
/// and a couple of long spars — varied radii. Reads as ancient mechanical tech, not a body.
fn tangle(seed: u32) -> Vec<Capsule> {
    let mut rng = Rng::new(seed);
    let mut caps: Vec<Capsule> = Vec::new();

    // Bounded sprawl (model units; scaled to world later). Wider than tall so it sprawls.
    let hx = rng.range(13.0, 20.0);
    let hz = rng.range(13.0, 20.0);
    let hy = rng.range(16.0, 30.0);

    // Hub nodes.
    let n_hubs = rng.range(3.0, 7.0) as usize;
    let hubs: Vec<Vec3> = (0..n_hubs.max(2))
        .map(|_| Vec3::new(rng.range(-hx, hx), rng.range(2.0, hy), rng.range(-hz, hz)))
        .collect();

    // Thick girders chaining the hubs together.
    for w in hubs.windows(2) {
        caps.push(Capsule {
            a: w[0],
            b: w[1],
            r: rng.range(2.5, 4.5),
        });
    }

    // Pipe runs branching off each hub, in near-axis directions, often with a perpendicular
    // elbow — the tangle.
    for &h in &hubs {
        let branches = rng.range(2.0, 5.0) as usize;
        for _ in 0..branches {
            let dir = axis_dir(&mut rng);
            let len = rng.range(8.0, 20.0);
            let mid = h + dir * len;
            let r = rng.range(0.8, 2.4);
            caps.push(Capsule { a: h, b: mid, r });
            if rng.unit() < 0.6 {
                let end = mid + axis_dir(&mut rng) * rng.range(6.0, 16.0);
                caps.push(Capsule {
                    a: mid,
                    b: end,
                    r: r * rng.range(0.6, 1.0),
                });
            }
        }
    }

    // A couple of long spars lancing across the whole structure.
    for _ in 0..(rng.range(1.0, 3.0) as usize) {
        let a = Vec3::new(rng.range(-hx, hx), rng.range(2.0, hy), rng.range(-hz, hz));
        let dir = axis_dir(&mut rng);
        caps.push(Capsule {
            a,
            b: a + dir * rng.range(18.0, 32.0),
            r: rng.range(1.0, 2.0),
        });
    }
    caps
}

/// Model-space AABB of a capsule set (with radii).
fn bounds(caps: &[Capsule]) -> (Vec3, Vec3) {
    let mut lo = Vec3::splat(f32::MAX);
    let mut hi = Vec3::splat(f32::MIN);
    for c in caps {
        lo = lo.min(c.a.min(c.b) - Vec3::splat(c.r));
        hi = hi.max(c.a.max(c.b) + Vec3::splat(c.r));
    }
    (lo.floor(), hi.ceil())
}

/// Rasterise capsules into an occupancy grid (cell size `step`, origin `lo`, dims `nx*ny*nz`).
/// **Per-capsule**: only the cells in each tube's own bounding box are visited — O(Σ capsule
/// bboxes), not O(grid × capsules). This is the difference between a ~100 ms voxelise and ~1 ms.
fn rasterize(caps: &[Capsule], lo: Vec3, step: f32, nx: i32, ny: i32, nz: i32) -> Vec<bool> {
    let idx = |x: i32, y: i32, z: i32| ((y * nz + z) * nx + x) as usize;
    let mut occ = vec![false; (nx * ny * nz) as usize];
    for c in caps {
        let clo = ((c.a.min(c.b) - Vec3::splat(c.r) - lo) / step).floor();
        let chi = ((c.a.max(c.b) + Vec3::splat(c.r) - lo) / step).ceil();
        let (x0, x1) = ((clo.x as i32).max(0), (chi.x as i32).min(nx - 1));
        let (y0, y1) = ((clo.y as i32).max(0), (chi.y as i32).min(ny - 1));
        let (z0, z1) = ((clo.z as i32).max(0), (chi.z as i32).min(nz - 1));
        for z in z0..=z1 {
            for y in y0..=y1 {
                for x in x0..=x1 {
                    let p = lo + Vec3::new(x as f32, y as f32, z as f32) * step;
                    if seg_dist(p, c.a, c.b) <= c.r {
                        occ[idx(x, y, z)] = true;
                    }
                }
            }
        }
    }
    occ
}

/// Emit the **surface points** of a relic as world-space splats. `feet` is where it rests
/// (lowest point sits there); `yaw` rotates it about +Y; `voxel` is world-units per model unit;
/// tinted `color`. Deterministic in `seed`.
pub fn relic_points(
    feet: Vec3,
    voxel: f32,
    yaw: f32,
    seed: u32,
    color: [f32; 3],
) -> Vec<SplatInstance> {
    // Sample the surface on a *coarse* grid (STEP model-units per cell): fewer, sparser points
    // — smaller/airier ethereal look, and far cheaper to generate (the cell-crossing cost that
    // was hitching the framerate scales with 1/STEP³).
    const STEP: f32 = 1.5;
    let caps = tangle(seed);
    let (lo, hi) = bounds(&caps);
    let dim = hi - lo;
    let (nx, ny, nz) = (
        (dim.x / STEP) as i32 + 2,
        (dim.y / STEP) as i32 + 2,
        (dim.z / STEP) as i32 + 2,
    );
    let idx = |x: i32, y: i32, z: i32| ((y * nz + z) * nx + x) as usize;
    let occ = rasterize(&caps, lo, STEP, nx, ny, nz);

    let (sy, cy) = yaw.sin_cos();
    let mut rng = Rng::new(seed ^ 0x00B0_D1E5);
    let mut out = Vec::new();
    for y in 0..ny {
        for z in 0..nz {
            for x in 0..nx {
                if !occ[idx(x, y, z)] {
                    continue;
                }
                let air = |x: i32, y: i32, z: i32| {
                    x < 0 || y < 0 || z < 0 || x >= nx || y >= ny || z >= nz || !occ[idx(x, y, z)]
                };
                let exposed = air(x - 1, y, z)
                    || air(x + 1, y, z)
                    || air(x, y - 1, z)
                    || air(x, y + 1, z)
                    || air(x, y, z - 1)
                    || air(x, y, z + 1);
                if !exposed {
                    continue;
                }
                // Jitter each point off its grid cell (up to ~¾ of a cell) so the cloud reads
                // as natural scatter, not a regular lattice.
                let j = STEP * 0.75;
                let m = lo
                    + Vec3::new(x as f32, y as f32, z as f32) * STEP
                    + Vec3::new(rng.range(-j, j), rng.range(-j, j), rng.range(-j, j));
                let (mx, my, mz) = (m.x, m.y - lo.y, m.z);
                let wx = mx * cy - mz * sy;
                let wz = mx * sy + mz * cy;
                let world = feet + Vec3::new(wx * voxel, my * voxel, wz * voxel);
                let v = 0.85 + 0.15 * rng.unit();
                out.push(SplatInstance {
                    offset: [world.x, world.y, world.z],
                    // Smaller than before (was 0.55) — airier points, less overdraw.
                    size: voxel * 0.24,
                    color: [color[0] * v, color[1] * v, color[2] * v],
                    sway: rng.unit() * std::f32::consts::TAU,
                });
            }
        }
    }
    out
}

/// Voxelise a relic into **solid world-voxel coords** for greedy meshing (the solid, explorable
/// kind). Same placement transform as [`relic_points`]. Rasterises each tube into **world** space
/// per-capsule (only its own bbox), so it's ~1 ms, not ~100 ms of testing every voxel × capsule.
pub fn relic_voxels(feet: Vec3, voxel: f32, yaw: f32, seed: u32) -> Vec<glam::IVec3> {
    let caps = tangle(seed);
    let (mlo, _mhi) = bounds(&caps);
    let (sy, cy) = yaw.sin_cos();
    // Transform a model point to world (matches `relic_points`, so mesh + dissolve coincide).
    let to_world = |m: Vec3| {
        let wx = m.x * cy - m.z * sy;
        let wz = m.x * sy + m.z * cy;
        feet + Vec3::new(wx * voxel, (m.y - mlo.y) * voxel, wz * voxel)
    };
    let mut set = std::collections::HashSet::new();
    for c in &caps {
        // The capsule, transformed to world (rotation + uniform scale → still a segment + radius).
        let a = to_world(c.a);
        let b = to_world(c.b);
        let r = c.r * voxel;
        let lo = (a.min(b) - Vec3::splat(r)).floor();
        let hi = (a.max(b) + Vec3::splat(r)).ceil();
        for iz in (lo.z as i32)..=(hi.z as i32) {
            for iy in (lo.y as i32)..=(hi.y as i32) {
                for ix in (lo.x as i32)..=(hi.x as i32) {
                    let p = Vec3::new(ix as f32 + 0.5, iy as f32 + 0.5, iz as f32 + 0.5);
                    if seg_dist(p, a, b) <= r {
                        set.insert(glam::IVec3::new(ix, iy, iz));
                    }
                }
            }
        }
    }
    set.into_iter().collect()
}

/// One placed relic structure: where it rests, its yaw, scale, generative seed, and whether it
/// renders **solid** (greedy-meshed, explorable) vs **ethereal** (points, drift-through).
#[derive(Copy, Clone, Debug)]
pub struct Placement {
    pub pos: Vec3,
    pub yaw: f32,
    pub voxel: f32,
    pub seed: u32,
    pub solid: bool,
}

/// Seed-driven scatter of relics across a square region centred on the origin (half-extent
/// `half`, world units). Deterministic; `ground(x, z)` drops each onto the terrain. (The live
/// app uses `structures::colossi_near`; this is for fixed-region demos/tests.)
pub fn scatter(
    seed: u32,
    half: f32,
    count: u32,
    ground: impl Fn(f32, f32) -> f32,
) -> Vec<Placement> {
    let mut rng = Rng::new(seed ^ 0x00C0_1055);
    (0..count)
        .map(|_| {
            let x = rng.range(-half, half);
            let z = rng.range(-half, half);
            Placement {
                pos: Vec3::new(x, ground(x, z), z),
                yaw: rng.range(0.0, std::f32::consts::TAU),
                voxel: rng.range(1.1, 1.7),
                seed: rng.0 | 1,
                solid: false, // demo scatter is ethereal; the live placer decides per relic
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relic_has_a_surface_shell() {
        let pts = relic_points(Vec3::ZERO, 1.0, 0.0, 7, [0.7; 3]);
        assert!(pts.len() > 120, "too few surface points: {}", pts.len());
        assert!(pts.iter().all(|p| p.size > 0.0));
    }

    #[test]
    fn seeds_give_different_tangles() {
        let a = relic_points(Vec3::ZERO, 1.0, 0.0, 1, [1.0; 3]).len();
        let b = relic_points(Vec3::ZERO, 1.0, 0.0, 2, [1.0; 3]).len();
        assert_ne!(a, b, "different seeds should tangle differently");
    }

    #[test]
    fn voxels_are_solid_and_nonempty() {
        let v = relic_voxels(Vec3::ZERO, 1.0, 0.3, 42);
        assert!(
            v.len() > 1000,
            "expected a solid volume of voxels: {}",
            v.len()
        );
    }

    #[test]
    fn scatter_is_deterministic_and_on_ground() {
        let g = |x: f32, z: f32| (x + z) * 0.1;
        let a = scatter(99, 100.0, 5, g);
        let b = scatter(99, 100.0, 5, g);
        assert_eq!(a.len(), 5);
        for (p, q) in a.iter().zip(&b) {
            assert_eq!(p.pos, q.pos);
            assert_eq!(p.pos.y, g(p.pos.x, p.pos.z));
        }
    }
}
