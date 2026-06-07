//! On-foot player movement (E19): walking with gravity and an **animated** auto-step, collided
//! against the actual **voxels** (terrain + caves) via a solidity query, so you can walk down into
//! cave-mouths, along cave floors, and up small ledges — not just glide over the surface. Pure
//! logic: given `solid(x, y, z)` it constrains a free-fly camera step down to a walk. (Colossal
//! structures aren't in the solidity query yet, so they're not collidable on foot.)

use glam::Vec3;

/// Eye height above the feet (world units).
pub const EYE: f32 = 1.7;
/// Downhill drop the feet follow the ground for; steeper and they fall.
const STEP_DOWN: f32 = 1.5;
/// Gravity (world units/s²) for falls off ledges/cliffs.
const GRAVITY: f32 = 26.0;
/// Vertical follow speed (blocks/s) — animates the auto-step + downhill settle (not a teleport).
const STEP_RATE: f32 = 7.0;
/// Fraction of the free-fly horizontal delta kept on foot (a walk is slower than the cruiser).
const WALK_SCALE: f32 = 0.45;

/// Walking state — the vertical velocity carried between frames (for falling).
#[derive(Default)]
pub struct Walker {
    vy: f32,
}

/// Can the player stand in column `(bx, bz)` with feet in cell `fy` — i.e. is it clear, or a
/// step-up-able single block (solid feet cell, clear head + the cell above)?
fn passable(solid: &impl Fn(i32, i32, i32) -> bool, bx: i32, fy: i32, bz: i32) -> bool {
    // Head must be clear, and either the feet cell is clear (walk straight in) or there's room
    // two cells up so we can step onto the feet block.
    !solid(bx, fy + 1, bz) && (!solid(bx, fy, bz) || !solid(bx, fy + 2, bz))
}

impl Walker {
    /// Constrain a free-fly camera step to a voxel-collided walk. `prev` → `wanted` is the move
    /// the controller wanted (look already applied to the camera by the caller); we keep
    /// `WALK_SCALE` of the horizontal, block it per-axis against walls (allowing a one-block
    /// step-up), then resolve the vertical against gravity + the ground, animating the rise so
    /// stepping onto a block isn't instant. Returns the walked position.
    pub fn constrain(
        &mut self,
        prev: Vec3,
        wanted: Vec3,
        dt: f32,
        solid: impl Fn(i32, i32, i32) -> bool,
    ) -> Vec3 {
        let feet0 = prev.y - EYE;
        let fy = feet0.floor() as i32;
        let (mut px, mut pz) = (prev.x, prev.z);

        // Horizontal, per-axis (so you slide along walls), scaled to a walk.
        let tx = px + (wanted.x - px) * WALK_SCALE;
        if tx.floor() as i32 == px.floor() as i32
            || passable(&solid, tx.floor() as i32, fy, pz.floor() as i32)
        {
            px = tx;
        }
        let tz = pz + (wanted.z - pz) * WALK_SCALE;
        if tz.floor() as i32 == pz.floor() as i32
            || passable(&solid, px.floor() as i32, fy, tz.floor() as i32)
        {
            pz = tz;
        }

        // Support: the top of the highest solid voxel in a small window at/below the feet.
        let (bx, bz) = (px.floor() as i32, pz.floor() as i32);
        let mut support = feet0 - 64.0;
        for y in (fy - 3)..=(fy + 1) {
            if solid(bx, y, bz) {
                support = (y + 1) as f32; // ascending scan → ends on the topmost solid
            }
        }

        // Vertical: animate toward the support when on/near ground (step-up, downhill, ledges
        // within STEP_DOWN); otherwise fall under gravity.
        let on_ground = feet0 < support || feet0 - support <= STEP_DOWN;
        let feet = if on_ground {
            let rate = STEP_RATE * dt;
            self.vy = 0.0;
            if (support - feet0).abs() <= rate {
                support
            } else {
                feet0 + (support - feet0).signum() * rate
            }
        } else {
            self.vy -= GRAVITY * dt;
            let f = feet0 + self.vy * dt;
            if f <= support {
                self.vy = 0.0;
                support
            } else {
                f
            }
        };
        Vec3::new(px, feet + EYE, pz)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Flat solid ground with its top face at `top` (blocks fill y < top).
    fn flat(top: i32) -> impl Fn(i32, i32, i32) -> bool {
        move |_x, y, _z| y < top
    }

    #[test]
    fn walks_along_flat_ground_at_walk_speed() {
        let g = flat(10); // feet rest at y=10
        let mut w = Walker::default();
        let start = Vec3::new(0.5, 10.0 + EYE, 0.5);
        let p = w.constrain(start, Vec3::new(5.0, 40.0, 0.5), 0.1, g);
        assert!((p.y - (10.0 + EYE)).abs() < 1e-3, "feet stay on the ground");
        assert!(
            p.x > 0.5 && p.x < 5.0,
            "moved toward target, slower than free-fly"
        );
    }

    #[test]
    fn auto_step_is_animated_not_instant() {
        // Ground top 10, but a +1 block (top 11) for x >= 1.
        let g = |x: i32, y: i32, _z: i32| y < 10 || (x >= 1 && y < 11);
        let mut w = Walker::default();
        let p = w.constrain(
            Vec3::new(0.5, 10.0 + EYE, 0.5),
            Vec3::new(5.0, 10.0 + EYE, 0.5),
            0.1,
            g,
        );
        assert!(p.x > 0.5, "stepped forward onto the ledge column");
        assert!(p.y > 10.0 + EYE, "feet began rising");
        assert!(
            p.y < 11.0 + EYE,
            "but the rise is animated, not an instant teleport"
        );
    }

    #[test]
    fn a_tall_wall_blocks_the_walk() {
        let g = |x: i32, y: i32, _z: i32| y < 10 || (x >= 1 && y < 14); // +4 wall
        let mut w = Walker::default();
        let p = w.constrain(
            Vec3::new(0.5, 10.0 + EYE, 0.5),
            Vec3::new(5.0, 10.0 + EYE, 0.5),
            0.1,
            g,
        );
        assert!(
            (p.x - 0.5).abs() < 1e-3,
            "a tall wall blocks horizontal movement"
        );
    }

    #[test]
    fn falls_off_a_cliff() {
        let g = flat(0); // solid only far below → airborne
        let mut w = Walker::default();
        let p = w.constrain(
            Vec3::new(0.5, 30.0 + EYE, 0.5),
            Vec3::new(0.5, 30.0 + EYE, 0.5),
            0.1,
            g,
        );
        assert!(p.y < 30.0 + EYE, "fell under gravity");
    }
}
