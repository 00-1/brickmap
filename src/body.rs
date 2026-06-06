//! Colossal figures (E18) — giant fallen forms placed in the world as **structures** (see
//! `structures`): seed-driven, independent of chunk terrain. Built procedurally as a *posed
//! skeleton* (two builds via [`Sex`]; natural collapsed poses) and emitted as **surface points**
//! ([`crate::foliage::SplatInstance`]) so they render through the existing splat billboard
//! pipeline — ethereal forms you drift through (no collision). These procedural colossi are a
//! structure type in their own right; real anatomically-correct models are a separate, later
//! thing (an offline `voxelize` of CC0 meshes would slot in behind the same posing/placement).
//! Pure logic (no wgpu); deterministic from the seed.

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

/// A step of length `len` in heading `ang` (radians, in the ground XZ plane), lifted `lift`.
fn step(ang: f32, len: f32, lift: f32) -> Vec3 {
    Vec3::new(ang.cos() * len, lift, ang.sin() * len)
}

/// In-plane perpendicular of heading `ang` (for shoulder/hip offsets).
fn perp(ang: f32, w: f32) -> Vec3 {
    let a = ang + std::f32::consts::FRAC_PI_2;
    Vec3::new(a.cos() * w, 0.0, a.sin() * w)
}

/// Rotate `p` about the body's long (X) axis by `(sin, cos)` — the "roll" onto back/front/side.
fn roll_x(p: Vec3, s: f32, c: f32) -> Vec3 {
    Vec3::new(p.x, p.y * c - p.z * s, p.y * s + p.z * c)
}

/// Build a **collapsed** skeleton (model space, lying roughly along +X; the world yaw is added
/// later) for `sex`, posed by `seed`. Wide, generative variation so every figure reads as its
/// own fallen pose: a curling spine, each limb reaching in any direction with a bent joint, a
/// body roll onto back/side/front, and slight per-figure build. Returns capsules + head sphere.
fn collapsed(sex: Sex, seed: u32) -> (Vec<Capsule>, (Vec3, f32)) {
    let p = Proportions::of(sex);
    let mut rng = Rng::new(seed);
    let lr = p.limb_r * rng.range(0.85, 1.15); // slight build variation

    // Trunk: a curling chain (pelvis → chest → head), so spines range from straight to foetal.
    let d0 = rng.range(-0.4, 0.4); // initial trunk heading off +X
    let curl = rng.range(-0.7, 1.1); // per-segment turn → arch ↔ curl
    let pelvis = Vec3::new(0.0, p.torso_r, 0.0);
    let chest = pelvis + step(d0, p.spine, rng.range(-1.5, 1.5));
    let d1 = d0 + curl;
    let head_base = chest + step(d1, p.neck, rng.range(-1.0, 1.5));
    let mut head_c = head_base + step(d1 + curl * 0.5, p.head_r * 0.9, rng.range(-1.0, 1.0));

    let mut caps = Vec::new();
    let cap = |a: Vec3, b: Vec3, r: f32, v: &mut Vec<Capsule>| v.push(Capsule { a, b, r });
    cap(pelvis, chest, p.torso_r, &mut caps);
    cap(chest, head_base, lr * 1.2, &mut caps);

    // Legs from the hips: thighs roughly "down-body" but with a wide splay/cross, knees bent
    // anywhere (straight → drawn-up foetal).
    for s in [-1.0f32, 1.0] {
        let hip = pelvis + perp(d0, s * p.hip_w);
        let thigh_dir = d0 + std::f32::consts::PI + s * rng.range(-1.3, 1.3);
        let knee = hip + step(thigh_dir, p.thigh, rng.range(-2.0, 3.5));
        let shin_dir = thigh_dir + rng.range(-1.7, 1.7);
        let foot = knee + step(shin_dir, p.shin, rng.range(-2.0, 3.5));
        cap(hip, knee, lr, &mut caps);
        cap(knee, foot, lr * 0.9, &mut caps);
    }
    // Arms from the shoulders: reaching in *any* direction (over head, flung out, tucked),
    // elbows bent anywhere.
    for s in [-1.0f32, 1.0] {
        let shoulder = chest + perp(d1, s * p.shoulder_w);
        let arm_dir = rng.range(0.0, std::f32::consts::TAU);
        let elbow = shoulder + step(arm_dir, p.upper_arm, rng.range(-2.0, 3.5));
        let fore_dir = arm_dir + rng.range(-1.9, 1.9);
        let hand = elbow + step(fore_dir, p.fore_arm, rng.range(-2.0, 3.5));
        cap(shoulder, elbow, lr * 0.95, &mut caps);
        cap(elbow, hand, lr * 0.85, &mut caps);
    }

    // Roll the whole figure onto its back / side / front (modest, so it stays lying down).
    let (rs, rc) = rng.range(-0.6, 0.6).sin_cos();
    for c in caps.iter_mut() {
        c.a = roll_x(c.a, rs, rc);
        c.b = roll_x(c.b, rs, rc);
    }
    head_c = roll_x(head_c, rs, rc);
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
        // A fallen figure spans clearly wider on the ground than it is tall (even curled/rolled).
        assert!(
            length > height * 1.3,
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
