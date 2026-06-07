//! Cruiser **auto-scan** (G3, game-mechanics §6/§8.1): the autopilot/idle sensing layer. As
//! the cruiser drifts, it reads sites in a **forward cone** and marks them *known* on the map
//! (the opportunity surface) — it does **not** collect. Cheap: a cone test + a brief **cool**
//! overlay flick (distinct from G2's warm collection beam). Pure logic here; the app drives it.

use brickmap::overlay::OverlayVertex;
use glam::Vec3;

/// How far ahead the basic scan reaches (tech gating — *Scan Range* — is G4).
pub const RANGE: f32 = 150.0;
/// Cosine of the forward cone's half-angle (~0.6 ≈ 53°): a site must be roughly ahead.
pub const COS_HALF_ANGLE: f32 = 0.6;
/// Seconds between scan pulses (reads as the ship sweeping, not an instant flood).
pub const INTERVAL: f32 = 0.25;
/// How long a scan flick lingers.
pub const FLICK_LIFE: f32 = 0.45;
/// Max flicks spawned per pulse (keeps the screen calm).
pub const FLICKS_PER_PULSE: usize = 3;
/// The cool scan colour (vs the survey-beam's warm).
pub const COOL: [f32; 3] = [0.35, 0.85, 1.0];

/// Is `p` inside the forward scan cone from `cam` looking `forward` (unit), within `RANGE`?
pub fn in_cone(p: Vec3, cam: Vec3, forward: Vec3, range: f32) -> bool {
    let to = p - cam;
    let d = to.length();
    if d <= 1e-3 {
        return true; // on top of us
    }
    d <= range && to.normalize().dot(forward) >= COS_HALF_ANGLE
}

/// A brief cool flick from the cruiser toward a freshly-scanned site.
#[derive(Copy, Clone, Debug)]
pub struct Flick {
    pub from: Vec3,
    pub to: Vec3,
    pub born: f32,
}

impl Flick {
    pub fn dead(&self, now: f32) -> bool {
        now - self.born >= FLICK_LIFE
    }

    /// Cool ribbon for the overlay, fading over the flick's short life.
    pub fn ribbon(&self, cam: Vec3, now: f32) -> Vec<OverlayVertex> {
        let age = (now - self.born).max(0.0);
        let a = if age >= FLICK_LIFE {
            0.0
        } else {
            1.0 - age / FLICK_LIFE
        };
        crate::beam::ribbon_seg(self.from, self.to, cam, a, COOL)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cone_accepts_ahead_rejects_behind_and_far() {
        let cam = Vec3::ZERO;
        let fwd = Vec3::Z;
        assert!(in_cone(Vec3::new(0.0, 0.0, 20.0), cam, fwd, RANGE)); // straight ahead
        assert!(in_cone(Vec3::new(8.0, 0.0, 20.0), cam, fwd, RANGE)); // within the cone
        assert!(!in_cone(Vec3::new(0.0, 0.0, -20.0), cam, fwd, RANGE)); // behind
        assert!(!in_cone(Vec3::new(40.0, 0.0, 5.0), cam, fwd, RANGE)); // off to the side
        assert!(!in_cone(Vec3::new(0.0, 0.0, 999.0), cam, fwd, RANGE)); // too far
    }

    #[test]
    fn flick_fades_out() {
        let f = Flick {
            from: Vec3::ZERO,
            to: Vec3::new(0.0, 0.0, 10.0),
            born: 0.0,
        };
        assert!(!f.dead(0.1) && f.dead(FLICK_LIFE));
        assert!(!f.ribbon(Vec3::new(5.0, 5.0, 0.0), 0.0).is_empty());
        assert!(f.ribbon(Vec3::new(5.0, 5.0, 0.0), FLICK_LIFE).is_empty());
    }
}
