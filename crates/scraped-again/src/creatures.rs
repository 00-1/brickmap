//! Point-cloud creatures (E15) — decorative **drifting wisps**: small swarms of points that
//! wander and swirl through the world, giving the static fly-through some life. Pure logic
//! (no wgpu): a wisp has a slowly-wandering centre and a cluster of members orbiting it; the
//! renderer draws the emitted points through the existing splat billboard pipeline. Seed-driven
//! and frame-stepped; kept loosely near a focus point (the camera) so they stay in view.
//!
//! Deliberately abstract for now (motes/wisps, not animals) — on-identity ethereal points;
//! the form is easy to evolve (flocking boids, point-cloud critters) once the look lands.

use glam::Vec3;

use crate::foliage::SplatInstance;

/// xorshift32 PRNG.
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

/// One drifting wisp: a wandering centre + a slowly-tumbling cluster of member points. The
/// members sit at fixed **organic** offsets (scattered in a ball, not on a grid/orbit); the
/// whole cluster spins about Y and each point wobbles, so it reads as a living mote, not a ring.
struct Wisp {
    center: Vec3,
    heading: f32,       // current drift heading (radians, in the XZ plane)
    speed: f32,         // drift speed (world units/s)
    swirl: f32,         // angular speed of the cluster spin (rad/s)
    phase: f32,         // per-wisp phase offset so swirls/bobs don't sync
    offsets: Vec<Vec3>, // organic member offsets (random points in a ball)
    tint: [f32; 3],
}

impl Wisp {
    /// Advance the centre: wander (slowly turn) + drift forward + a gentle bob, steering back
    /// toward `focus` if it strays past `leash` so it stays near the camera. `rng` jitters the
    /// turn; `t` drives the bob.
    fn step(&mut self, dt: f32, t: f32, focus: Vec3, leash: f32, rng: &mut Rng) {
        // Wander: nudge the heading; bias it back toward the focus when far out.
        let to_focus = focus - self.center;
        let flat = Vec3::new(to_focus.x, 0.0, to_focus.z);
        let dist = flat.length();
        self.heading += rng.range(-1.2, 1.2) * dt;
        if dist > leash {
            // Steer toward the focus heading (lerp the heading angle).
            let want = flat.z.atan2(flat.x);
            let mut d = want - self.heading;
            while d > std::f32::consts::PI {
                d -= std::f32::consts::TAU;
            }
            while d < -std::f32::consts::PI {
                d += std::f32::consts::TAU;
            }
            self.heading += d * (dt * 0.8);
        }
        let dir = Vec3::new(self.heading.cos(), 0.0, self.heading.sin());
        self.center += dir * (self.speed * dt);
        // Gentle vertical bob so they don't sit on a plane.
        self.center.y += (t * 0.6 + self.phase).sin() * dt * 2.0;
    }

    /// Emit the wisp's member points as splats at time `t`: each fixed organic offset is spun
    /// about Y (the cluster tumbles) and given a small per-point wobble, so the blob drifts and
    /// shimmers rather than holding a rigid shape.
    fn emit(&self, t: f32, out: &mut Vec<SplatInstance>) {
        let (sa, ca) = (t * self.swirl + self.phase).sin_cos();
        let n = self.offsets.len().max(1) as f32;
        for (i, o) in self.offsets.iter().enumerate() {
            let fi = i as f32;
            // Spin the offset about Y, then a small incoherent wobble per point.
            let wob = 0.4;
            let rx = o.x * ca - o.z * sa + (t * 1.3 + fi).sin() * wob;
            let rz = o.x * sa + o.z * ca + (t * 1.1 + fi * 2.0).cos() * wob;
            let ry = o.y + (t * 0.9 + fi * 0.7).sin() * wob;
            let p = self.center + Vec3::new(rx, ry, rz);
            // Dim, with a brighter core (points nearer the centre) — motes that glint.
            let glow = 0.45 + 0.3 * (1.0 - fi / n);
            out.push(SplatInstance {
                offset: [p.x, p.y, p.z],
                size: 0.34,
                color: [
                    self.tint[0] * glow,
                    self.tint[1] * glow,
                    self.tint[2] * glow,
                ],
                sway: 0.0,
                alpha: 1.0,
            });
        }
    }
}

/// A drifting swarm of wisps kept loosely around a focus point. Build with [`Swarm::new`],
/// advance each frame with [`Swarm::update`], collect the points with [`Swarm::points`].
pub struct Swarm {
    wisps: Vec<Wisp>,
    rng: Rng,
    t: f32,
    leash: f32,
}

impl Swarm {
    /// `count` wisps, seeded, initially scattered within `leash` of `focus`.
    pub fn new(seed: u32, count: u32, focus: Vec3, leash: f32) -> Swarm {
        let mut rng = Rng::new(seed ^ 0x57A2_117D);
        let wisps = (0..count)
            .map(|_| {
                let ang = rng.range(0.0, std::f32::consts::TAU);
                let d = rng.range(0.2, 1.0) * leash;
                let tint = [
                    rng.range(0.5, 0.8),
                    rng.range(0.7, 0.95),
                    rng.range(0.8, 1.0),
                ];
                // Organic cluster: scatter members in a ball (uniform-ish via cbrt radius), so
                // the mote is an irregular blob, not a ring.
                let radius = rng.range(4.0, 9.0);
                let count = rng.range(16.0, 30.0) as u32;
                let offsets = (0..count)
                    .map(|_| {
                        let dir = Vec3::new(
                            rng.range(-1.0, 1.0),
                            rng.range(-1.0, 1.0),
                            rng.range(-1.0, 1.0),
                        )
                        .normalize_or_zero();
                        dir * (radius * rng.unit().cbrt())
                    })
                    .collect();
                Wisp {
                    center: focus + Vec3::new(ang.cos() * d, rng.range(8.0, 34.0), ang.sin() * d),
                    heading: rng.range(0.0, std::f32::consts::TAU),
                    speed: rng.range(4.0, 10.0),
                    swirl: rng.range(0.4, 1.3) * if rng.unit() < 0.5 { -1.0 } else { 1.0 },
                    phase: rng.range(0.0, std::f32::consts::TAU),
                    offsets,
                    tint,
                }
            })
            .collect();
        Swarm {
            wisps,
            rng,
            t: 0.0,
            leash,
        }
    }

    /// Advance all wisps by `dt`, keeping them loosely tethered to `focus`.
    pub fn update(&mut self, dt: f32, focus: Vec3) {
        self.t += dt;
        let (t, leash) = (self.t, self.leash);
        for w in &mut self.wisps {
            w.step(dt, t, focus, leash, &mut self.rng);
        }
    }

    /// The current member points of every wisp, as splats for the billboard pipeline.
    pub fn points(&self) -> Vec<SplatInstance> {
        self.points_n(self.wisps.len())
    }

    /// Emit only the first `max` wisps' points — lets the app vary how many motes drift by the
    /// biome (denser biomes show more), without rebuilding the swarm. `max` is clamped.
    pub fn points_n(&self, max: usize) -> Vec<SplatInstance> {
        let mut out = Vec::new();
        for w in self.wisps.iter().take(max.min(self.wisps.len())) {
            w.emit(self.t, &mut out);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_points_and_is_deterministic() {
        let focus = Vec3::new(100.0, 20.0, -50.0);
        let mut a = Swarm::new(7, 6, focus, 80.0);
        let mut b = Swarm::new(7, 6, focus, 80.0);
        for _ in 0..120 {
            a.update(1.0 / 60.0, focus);
            b.update(1.0 / 60.0, focus);
        }
        let (pa, pb) = (a.points(), b.points());
        assert!(!pa.is_empty(), "a swarm should emit points");
        assert_eq!(pa.len(), pb.len());
        assert_eq!(pa[0].offset, pb[0].offset, "same seed → same motion");
    }

    #[test]
    fn wisps_stay_near_the_focus() {
        // Over a long run with a fixed focus, the leash should keep them from wandering off.
        let focus = Vec3::ZERO;
        let leash = 60.0;
        let mut s = Swarm::new(3, 8, focus, leash);
        for _ in 0..3000 {
            s.update(1.0 / 60.0, focus);
        }
        for p in s.points() {
            let d = (Vec3::from(p.offset) - focus).length();
            assert!(d < leash * 2.5, "wisp strayed too far: {d}");
        }
    }
}
