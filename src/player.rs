//! On-foot player movement (E19): walking with gravity, ground-following, and an auto-step up
//! small ledges. Pure logic — given the ground height under any `(x, z)` it constrains a free-fly
//! camera step down to a walk on the surface, so no voxel world is needed (the surface height
//! field is enough). Caves/overhangs aren't collidable on foot yet (you walk the surface).

use glam::Vec3;

/// Eye height above the feet (world units).
pub const EYE: f32 = 1.7;
/// Tallest ledge the player auto-steps up (≈ one block — the "auto jump").
const AUTO_STEP: f32 = 1.2;
/// Downhill drop the feet stick to the ground for; steeper than this and they fall.
const STEP_DOWN: f32 = 1.6;
/// Gravity (world units/s²) for falls off ledges/cliffs.
const GRAVITY: f32 = 26.0;
/// Fraction of the free-fly horizontal delta kept on foot (a walk is slower than the cruiser).
const WALK_SCALE: f32 = 0.45;

/// Walking state — just the vertical velocity carried between frames (for falling).
#[derive(Default)]
pub struct Walker {
    vy: f32,
}

impl Walker {
    /// Constrain a free-fly camera step to a walk: take the horizontal move the camera *wanted*
    /// (`prev` → `wanted`, e.g. from the controller), scale it to walk speed, block it if it would
    /// climb a wall taller than the auto-step, then resolve the vertical against gravity + the
    /// ground under the new spot. Returns the walked position; the caller keeps the camera's
    /// look (yaw/pitch) — only the position is constrained here.
    pub fn constrain(
        &mut self,
        prev: Vec3,
        wanted: Vec3,
        dt: f32,
        ground: impl Fn(f32, f32) -> f32,
    ) -> Vec3 {
        let feet0 = prev.y - EYE;
        let g0 = ground(prev.x, prev.z);
        let grounded = feet0 <= g0 + 0.2;

        // Horizontal intent, scaled to a walk; blocked if it climbs a too-tall step.
        let mut nx = prev.x + (wanted.x - prev.x) * WALK_SCALE;
        let mut nz = prev.z + (wanted.z - prev.z) * WALK_SCALE;
        if grounded && ground(nx, nz) - feet0 > AUTO_STEP {
            nx = prev.x;
            nz = prev.z; // wall ahead — don't climb it
        }

        // Vertical: stick to the ground (auto-stepping small ledges) when grounded; else fall.
        let g = ground(nx, nz);
        let feet = if feet0 <= g + 0.1 || (grounded && feet0 - g <= STEP_DOWN) {
            self.vy = 0.0;
            g
        } else {
            self.vy -= GRAVITY * dt;
            let f = feet0 + self.vy * dt;
            if f < g {
                self.vy = 0.0;
                g
            } else {
                f
            }
        };
        Vec3::new(nx, feet + EYE, nz)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn walks_along_flat_ground_at_walk_speed() {
        let flat = |_x: f32, _z: f32| 10.0;
        let mut w = Walker::default();
        let start = Vec3::new(0.0, 10.0 + EYE, 0.0);
        // The controller "wanted" to fly 5 units; on foot we keep WALK_SCALE of it and stay grounded.
        let p = w.constrain(start, Vec3::new(5.0, 40.0, 0.0), 0.1, flat);
        assert!(
            (p.y - (10.0 + EYE)).abs() < 1e-3,
            "feet glued to flat ground"
        );
        assert!(
            p.x > 0.0 && p.x < 5.0,
            "moved toward the target, slower than free-fly"
        );
    }

    #[test]
    fn auto_steps_small_ledge_but_not_a_wall() {
        let ledge = |x: f32, _z: f32| if x > 1.0 { 11.0 } else { 10.0 }; // +1 block step
        let mut w = Walker::default();
        let up = w.constrain(
            Vec3::new(0.5, 10.0 + EYE, 0.0),
            Vec3::new(4.0, 10.0 + EYE, 0.0),
            0.1,
            ledge,
        );
        assert!(up.x > 0.5, "stepped up onto the ledge");
        assert!((up.y - (11.0 + EYE)).abs() < 1e-3, "feet rose one block");

        let wall = |x: f32, _z: f32| if x > 1.0 { 14.0 } else { 10.0 }; // +4 block wall
        let mut w2 = Walker::default();
        let blocked = w2.constrain(
            Vec3::new(0.5, 10.0 + EYE, 0.0),
            Vec3::new(4.0, 10.0 + EYE, 0.0),
            0.1,
            wall,
        );
        assert!(
            (blocked.x - 0.5).abs() < 1e-3,
            "a tall wall blocks the walk"
        );
    }

    #[test]
    fn falls_when_airborne() {
        let flat = |_x: f32, _z: f32| 0.0;
        let mut w = Walker::default();
        // Start well above the ground, not moving: gravity pulls the feet down.
        let p = w.constrain(
            Vec3::new(0.0, 20.0 + EYE, 0.0),
            Vec3::new(0.0, 20.0 + EYE, 0.0),
            0.1,
            flat,
        );
        assert!(p.y < 20.0 + EYE, "fell under gravity");
        assert!(p.y > 0.0 + EYE, "hasn't reached the ground in one step");
    }
}
