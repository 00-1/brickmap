//! Colossal bodies (E18) — giant fallen human figures, placed in the world as **structures**
//! (seed-driven, independent of chunk terrain; the first of a category we'll grow). Built here
//! procedurally as a *posed skeleton* (male/female proportions, natural collapsed poses) and
//! emitted as **surface points** ([`crate::foliage::SplatInstance`]) so they render through the
//! existing splat billboard pipeline — ethereal forms you drift through (no collision).
//!
//! The procedural figure is a **placeholder for real CC0 anatomy**: once models are dropped in,
//! the planned `voxelize` tool replaces [`figure_points`]'s geometry while the posing,
//! placement, and rendering around it stay. Pure logic (no wgpu); deterministic from the seed.

use glam::Vec3;

use crate::foliage::SplatInstance;

/// Sex variant — drives proportions (shoulder/hip width, limb thickness, torso length).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Sex {
    Male,
    Female,
}

/// Body proportions in model units (a unit ≈ `voxel` world units; figure ≈ 80–90 units tall).
struct Proportions {
    spine: f32,      // pelvis → chest
    neck: f32,       // chest → head base
    head_r: f32,     // head sphere radius
    shoulder_w: f32, // half-distance between shoulders (z)
    hip_w: f32,      // half-distance between hips (z)
    upper_arm: f32,  // shoulder → elbow
    fore_arm: f32,   // elbow → hand
    thigh: f32,      // hip → knee
    shin: f32,       // knee → foot
    torso_r: f32,    // trunk capsule radius
    limb_r: f32,     // arm/leg capsule radius
}

impl Proportions {
    fn of(sex: Sex) -> Proportions {
        match sex {
            Sex::Male => Proportions {
                spine: 24.0,
                neck: 7.0,
                head_r: 7.0,
                shoulder_w: 11.0,
                hip_w: 7.0,
                upper_arm: 16.0,
                fore_arm: 14.0,
                thigh: 21.0,
                shin: 19.0,
                torso_r: 9.5,
                limb_r: 4.6,
            },
            Sex::Female => Proportions {
                spine: 23.0,
                neck: 6.5,
                head_r: 6.5,
                shoulder_w: 9.0,
                hip_w: 8.5,
                upper_arm: 15.0,
                fore_arm: 13.0,
                thigh: 20.0,
                shin: 18.0,
                torso_r: 8.3,
                limb_r: 3.9,
            },
        }
    }
}

/// A capsule (segment `a`–`b`, radius `r`) — the figure is a union of these plus a head sphere.
struct Capsule {
    a: Vec3,
    b: Vec3,
    r: f32,
}

/// Small deterministic PRNG (xorshift32) for per-figure pose jitter.
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
    /// Uniform in `[lo, hi)`.
    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.unit() * (hi - lo)
    }
}

/// A point in the ground plane at distance `len` from `base`, heading `ang` (radians, around
/// +Y from the body's +X axis), lifted slightly off the ground by `lift`.
fn reach(base: Vec3, ang: f32, len: f32, lift: f32) -> Vec3 {
    base + Vec3::new(ang.cos() * len, lift, ang.sin() * len)
}

/// Build a **collapsed** skeleton (model space, lying in the ground plane along ~+X) for `sex`,
/// posed by `seed`: a sprawl of limbs at jittered angles, as if fallen. Returns the capsules +
/// the head sphere `(centre, radius)`.
fn collapsed(sex: Sex, seed: u32) -> (Vec<Capsule>, (Vec3, f32)) {
    let p = Proportions::of(sex);
    let mut rng = Rng::new(seed);
    let lr = p.limb_r;

    // Trunk lies along +X just above the ground (resting on its side/back).
    let pelvis = Vec3::new(0.0, p.torso_r, 0.0);
    // A slight twist/arch in the spine so it's not a ramrod line.
    let chest = pelvis + Vec3::new(p.spine, rng.range(-2.0, 2.0), rng.range(-2.5, 2.5));
    let head_base = chest + Vec3::new(p.neck, rng.range(-1.0, 2.0), rng.range(-2.0, 2.0));
    let head_c = head_base + Vec3::new(p.head_r * 0.8, 0.0, rng.range(-1.5, 1.5));

    let mut caps = Vec::new();
    let cap = |a: Vec3, b: Vec3, r: f32, v: &mut Vec<Capsule>| v.push(Capsule { a, b, r });

    // Trunk (fat capsule) + neck.
    cap(pelvis, chest, p.torso_r, &mut caps);
    cap(chest, head_base, lr * 1.2, &mut caps);

    // Legs from the hips (offset in ±z), sprawled at jittered angles, knees bent.
    for s in [-1.0f32, 1.0] {
        let hip = pelvis + Vec3::new(-2.0, 0.0, s * p.hip_w);
        // Thighs point "down-body" (−X) and splay outward in z.
        let thigh_ang = std::f32::consts::PI + s * rng.range(0.15, 0.7);
        let knee = reach(hip, thigh_ang, p.thigh, rng.range(-1.0, 2.0));
        let shin_ang = thigh_ang + s * rng.range(-0.5, 0.6);
        let foot = reach(knee, shin_ang, p.shin, rng.range(-1.0, 1.0));
        cap(hip, knee, lr, &mut caps);
        cap(knee, foot, lr * 0.9, &mut caps);
    }
    // Arms from the shoulders, flung out at jittered angles, elbows bent.
    for s in [-1.0f32, 1.0] {
        let shoulder = chest + Vec3::new(p.shoulder_w * 0.2, 0.0, s * p.shoulder_w);
        let up_ang = s * rng.range(0.3, 1.6);
        let elbow = reach(shoulder, up_ang, p.upper_arm, rng.range(-1.0, 2.0));
        let fore_ang = up_ang + s * rng.range(-0.8, 0.9);
        let hand = reach(elbow, fore_ang, p.fore_arm, rng.range(-1.0, 1.5));
        cap(shoulder, elbow, lr * 0.95, &mut caps);
        cap(elbow, hand, lr * 0.85, &mut caps);
    }
    (caps, (head_c, p.head_r))
}

fn seg_dist(p: Vec3, a: Vec3, b: Vec3) -> f32 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_squared().max(1e-6)).clamp(0.0, 1.0);
    (p - (a + ab * t)).length()
}

/// Emit the **surface points** of a fallen figure as world-space splats. `feet` is where the
/// figure rests (its lowest point sits ~there); `yaw` rotates it about +Y (facing on the
/// ground); `voxel` is world-units per model unit; tinted `color`. Deterministic in `seed`.
pub fn figure_points(
    feet: Vec3,
    voxel: f32,
    yaw: f32,
    sex: Sex,
    seed: u32,
    color: [f32; 3],
) -> Vec<SplatInstance> {
    let (caps, (head_c, head_r)) = collapsed(sex, seed);

    // Model-space bounding box (with radii) → integer grid.
    let mut lo = head_c - Vec3::splat(head_r);
    let mut hi = head_c + Vec3::splat(head_r);
    for c in &caps {
        lo = lo.min(c.a.min(c.b) - Vec3::splat(c.r));
        hi = hi.max(c.a.max(c.b) + Vec3::splat(c.r));
    }
    let lo = lo.floor();
    let dim = (hi - lo).ceil();
    let (nx, ny, nz) = (dim.x as i32 + 2, dim.y as i32 + 2, dim.z as i32 + 2);
    let solid_at = |gx: i32, gy: i32, gz: i32| -> bool {
        let p = lo + Vec3::new(gx as f32, gy as f32, gz as f32);
        if (p - head_c).length() <= head_r {
            return true;
        }
        caps.iter().any(|c| seg_dist(p, c.a, c.b) <= c.r)
    };
    // Occupancy grid, then surface = solid cell exposed to air on a face.
    let idx = |x: i32, y: i32, z: i32| ((y * nz + z) * nx + x) as usize;
    let mut occ = vec![false; (nx * ny * nz) as usize];
    for y in 0..ny {
        for z in 0..nz {
            for x in 0..nx {
                occ[idx(x, y, z)] = solid_at(x, y, z);
            }
        }
    }

    let (sy, cy) = yaw.sin_cos();
    let mut out = Vec::new();
    let mut rng = Rng::new(seed ^ 0x00B0_D1E5);
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
                // Model-space position (feet/lowest at model y≈0 after subtracting lo.y).
                let m = lo + Vec3::new(x as f32, y as f32, z as f32);
                let mx = m.x - 0.0;
                let my = m.y - lo.y; // lift so the lowest point sits at the ground
                let mz = m.z;
                // Yaw about +Y, then scale to world and drop onto `feet`.
                let wx = mx * cy - mz * sy;
                let wz = mx * sy + mz * cy;
                let world = feet + Vec3::new(wx * voxel, my * voxel, wz * voxel);
                let v = 0.85 + 0.15 * rng.unit();
                out.push(SplatInstance {
                    offset: [world.x, world.y, world.z],
                    size: voxel * 0.55,
                    color: [color[0] * v, color[1] * v, color[2] * v],
                    sway: rng.unit() * std::f32::consts::TAU,
                });
            }
        }
    }
    out
}

/// One placed body structure: where it lies, which way it faces, its sex, scale, and pose seed.
#[derive(Copy, Clone, Debug)]
pub struct Placement {
    pub pos: Vec3,
    pub yaw: f32,
    pub sex: Sex,
    pub voxel: f32,
    pub seed: u32,
}

/// Seed-driven placement of fallen colossi across a square world region centred on the origin
/// (half-extent `half`, world units). Deterministic: the same `seed` always lays the same
/// bodies in the same spots/poses — a "structure" layer independent of chunk terrain. `count`
/// candidates; the `ground` fn lifts each onto the terrain surface.
pub fn scatter_fallen(
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
            let sex = if rng.unit() < 0.5 {
                Sex::Male
            } else {
                Sex::Female
            };
            Placement {
                pos: Vec3::new(x, ground(x, z), z),
                yaw: rng.range(0.0, std::f32::consts::TAU),
                sex,
                voxel: rng.range(1.1, 1.7), // ~90–150 world units long
                seed: rng.0,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn figure_has_a_surface_shell() {
        let pts = figure_points(Vec3::ZERO, 1.0, 0.0, Sex::Male, 7, [0.7; 3]);
        assert!(pts.len() > 500, "too few surface points: {}", pts.len());
        assert!(pts.iter().all(|p| p.size > 0.0));
    }

    #[test]
    fn lies_flat_wider_than_tall() {
        // A collapsed figure spans far more horizontally than vertically.
        let pts = figure_points(Vec3::ZERO, 1.0, 0.0, Sex::Female, 3, [1.0; 3]);
        let span = |f: fn(&SplatInstance) -> f32| {
            let (mut lo, mut hi) = (f32::MAX, f32::MIN);
            for p in &pts {
                let v = f(p);
                lo = lo.min(v);
                hi = hi.max(v);
            }
            hi - lo
        };
        let height = span(|p| p.offset[1]);
        let length = span(|p| p.offset[0]).max(span(|p| p.offset[2]));
        assert!(
            length > height * 2.0,
            "should lie flat: len {length} vs h {height}"
        );
    }

    #[test]
    fn sexes_and_seeds_differ() {
        let m = figure_points(Vec3::ZERO, 1.0, 0.0, Sex::Male, 1, [1.0; 3]).len();
        let f = figure_points(Vec3::ZERO, 1.0, 0.0, Sex::Female, 1, [1.0; 3]).len();
        assert_ne!(m, f, "male/female proportions should differ");
        let a = figure_points(Vec3::ZERO, 1.0, 0.0, Sex::Male, 1, [1.0; 3]);
        let b = figure_points(Vec3::ZERO, 1.0, 0.0, Sex::Male, 2, [1.0; 3]);
        assert_ne!(
            a.len(),
            b.len(),
            "different pose seeds should give different sprawls"
        );
    }

    #[test]
    fn scatter_is_deterministic_and_on_ground() {
        let g = |x: f32, z: f32| (x + z) * 0.1;
        let a = scatter_fallen(99, 100.0, 5, g);
        let b = scatter_fallen(99, 100.0, 5, g);
        assert_eq!(a.len(), 5);
        for (p, q) in a.iter().zip(&b) {
            assert_eq!(p.pos, q.pos);
            assert_eq!(p.pos.y, g(p.pos.x, p.pos.z));
        }
    }
}
