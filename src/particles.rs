//! CPU particle system (E2): emissive cube debris with gravity, lifetime, and fade.
//! Pure logic — knows nothing about wgpu. The renderer consumes [`ParticleInstance`].

use bytemuck::{Pod, Zeroable};
use glam::Vec3;

/// A unit cube centred on the origin (half-extent 0.5); scaled per-instance by size.
#[rustfmt::skip]
pub const CUBE_POSITIONS: [[f32; 3]; 8] = [
    [-0.5, -0.5, -0.5], [0.5, -0.5, -0.5], [0.5, 0.5, -0.5], [-0.5, 0.5, -0.5],
    [-0.5, -0.5,  0.5], [0.5, -0.5,  0.5], [0.5, 0.5,  0.5], [-0.5, 0.5,  0.5],
];
#[rustfmt::skip]
pub const CUBE_INDICES: [u16; 36] = [
    0, 2, 1, 0, 3, 2, // -Z
    4, 5, 6, 4, 6, 7, // +Z
    0, 1, 5, 0, 5, 4, // -Y
    3, 6, 2, 3, 7, 6, // +Y
    0, 7, 3, 0, 4, 7, // -X
    1, 2, 6, 1, 6, 5, // +X
];

/// Per-instance data uploaded to the GPU (one per live particle).
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct ParticleInstance {
    pub offset: [f32; 3],
    pub size: f32,
    pub color: [f32; 3],
    pub _pad: f32,
}

struct Particle {
    pos: Vec3,
    vel: Vec3,
    color: Vec3,
    life: f32,
    max_life: f32,
    size: f32,
}

/// An emissive-cube particle system with timed bursts from a set of emitter points.
pub struct ParticleSystem {
    particles: Vec<Particle>,
    emitters: Vec<Vec3>,
    gravity: f32,
    rng: u32,
    spawn_timer: f32,
    spawn_interval: f32,
    cap: usize,
}

impl ParticleSystem {
    pub fn new(emitters: Vec<Vec3>) -> Self {
        ParticleSystem {
            particles: Vec::new(),
            emitters,
            gravity: 16.0,
            rng: 0x9e37_79b9,
            spawn_timer: 0.0,
            spawn_interval: 0.16,
            cap: 4000,
        }
    }

    /// Replace the emitter points (e.g. to follow a travelling camera).
    pub fn set_emitters(&mut self, emitters: Vec<Vec3>) {
        self.emitters = emitters;
    }

    pub fn len(&self) -> usize {
        self.particles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.particles.is_empty()
    }

    /// Xorshift32 → `[0, 1)`. Deterministic, so renders are reproducible.
    fn rand(&mut self) -> f32 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        (x >> 8) as f32 / (1u32 << 24) as f32
    }

    /// Add a particle directly (used by bursts and tests).
    pub fn spawn(&mut self, pos: Vec3, vel: Vec3, color: Vec3, life: f32, size: f32) {
        if self.particles.len() >= self.cap {
            return;
        }
        self.particles.push(Particle {
            pos,
            vel,
            color,
            life,
            max_life: life,
            size,
        });
    }

    fn burst(&mut self) {
        if self.emitters.is_empty() {
            return;
        }
        let pick = (self.rand() * self.emitters.len() as f32) as usize;
        let origin = self.emitters[pick.min(self.emitters.len() - 1)];
        let count = 14 + (self.rand() * 10.0) as usize;
        for _ in 0..count {
            let angle = self.rand() * std::f32::consts::TAU;
            let spread = self.rand() * 5.0;
            let up = 13.0 + self.rand() * 9.0;
            let vel = Vec3::new(angle.cos() * spread, up, angle.sin() * spread);
            // Warm ember colours (emissive).
            let color = Vec3::new(1.0, 0.45 + self.rand() * 0.45, 0.08 + self.rand() * 0.18);
            let life = 1.6 + self.rand() * 1.2;
            let size = 0.4 + self.rand() * 0.45;
            self.spawn(origin, vel, color, life, size);
        }
    }

    pub fn update(&mut self, dt: f32) {
        let g = self.gravity;
        for p in &mut self.particles {
            p.vel.y -= g * dt;
            p.pos += p.vel * dt;
            p.life -= dt;
        }
        self.particles.retain(|p| p.life > 0.0);

        self.spawn_timer += dt;
        while self.spawn_timer >= self.spawn_interval {
            self.spawn_timer -= self.spawn_interval;
            self.burst();
        }
    }

    /// Instance data for the renderer; size and brightness fade as life runs out.
    pub fn instances(&self) -> Vec<ParticleInstance> {
        self.particles
            .iter()
            .map(|p| {
                let t = (p.life / p.max_life).clamp(0.0, 1.0);
                let c = p.color * (0.25 + 0.75 * t);
                ParticleInstance {
                    offset: p.pos.to_array(),
                    size: p.size * (0.35 + 0.65 * t),
                    color: c.to_array(),
                    _pad: 0.0,
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gravity_and_lifetime_drive_a_particle() {
        let mut sys = ParticleSystem::new(vec![]); // no emitters → no auto-spawn
        sys.spawn(Vec3::ZERO, Vec3::new(0.0, 10.0, 0.0), Vec3::ONE, 1.0, 1.0);
        assert_eq!(sys.len(), 1);

        // One step: vel.y -= g*dt, then pos += vel*dt; so it rises but less than ballistic-free.
        sys.update(0.1);
        assert_eq!(sys.len(), 1);
        let inst = sys.instances();
        assert!(inst[0].offset[1] > 0.0, "particle should have moved up");

        // Run past its life; it should be gone.
        for _ in 0..20 {
            sys.update(0.1);
        }
        assert_eq!(sys.len(), 0);
    }

    #[test]
    fn bursts_spawn_only_with_emitters() {
        let mut none = ParticleSystem::new(vec![]);
        none.update(1.0);
        assert!(none.is_empty());

        let mut sys = ParticleSystem::new(vec![Vec3::ZERO]);
        sys.update(1.0); // > several spawn intervals
        assert!(!sys.is_empty(), "bursts should have spawned particles");
    }
}
