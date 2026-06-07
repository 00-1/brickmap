//! The **survey-beam** (G2, game-mechanics §6): the signature manual verb. A heavy, vivid
//! energy line you cast from the player toward where you aim; it **persists then fades**,
//! **collects every glyph along its path** on cast (feeding G1's strata + codex), and is a
//! **rideable rail** (1-DoF attach + slide). Drawn through the engine's generic post-palette
//! overlay (`bm-render`), so it reads as the one vivid thing cutting through the muted world.
//!
//! This module is the pure/testable core (geometry, fade, intersection, ride math); the app
//! glues it to input, the collect path, the walker, and the cruiser-board mode machine.

use brickmap::overlay::OverlayVertex;
use glam::Vec3;

/// How long a cast beam lasts before it stops working (the reach budget, §6).
pub const LIFESPAN: f32 = 6.0;
/// Max cast length (world units) in the aim direction.
pub const LENGTH: f32 = 70.0;
/// Bright core half-width (world units); the glow halo is a few × this.
const CORE_HW: f32 = 0.16;
const HALO_HW: f32 = 0.5;
/// A glyph within this distance of the beam line is swept up on cast.
pub const COLLECT_RADIUS: f32 = 1.6;
/// The warm collection-beam colour (distinct from G3's cool scan beam).
pub const WARM: [f32; 3] = [1.0, 0.72, 0.30];

/// A cast beam: a fixed world-space segment with a birth time.
#[derive(Copy, Clone, Debug)]
pub struct Beam {
    pub a: Vec3,
    pub b: Vec3,
    pub born: f32,
}

impl Beam {
    /// Cast from `origin` toward unit `dir`, a [`LENGTH`] segment, born at `now`.
    pub fn cast(origin: Vec3, dir: Vec3, now: f32) -> Beam {
        Beam {
            a: origin,
            b: origin + dir.normalize_or_zero() * LENGTH,
            born: now,
        }
    }

    /// Fade factor in `[0, 1]` from full at birth to 0 at [`LIFESPAN`] (eased).
    pub fn alpha(&self, now: f32) -> f32 {
        let age = (now - self.born).max(0.0);
        if age >= LIFESPAN {
            return 0.0;
        }
        let t = 1.0 - age / LIFESPAN;
        t * t // ease-out: lingers bright, then drops away
    }

    pub fn dead(&self, now: f32) -> bool {
        now - self.born >= LIFESPAN
    }

    /// Parametric point along the segment (`t` in `[0,1]`) — the ride position.
    pub fn at(&self, t: f32) -> Vec3 {
        self.a.lerp(self.b, t.clamp(0.0, 1.0))
    }

    /// Build a camera-facing ribbon for the survey-beam (warm, faded by lifespan).
    pub fn ribbon(&self, cam: Vec3, now: f32) -> Vec<OverlayVertex> {
        ribbon_seg(self.a, self.b, cam, self.alpha(now), WARM)
    }
}

/// A camera-facing ribbon (a feathered glow quad-strip) along the segment `a..b`, at overall
/// `alpha` and `color`. Four longitudinal edges at offsets ±HALO/±CORE with alpha
/// 0/full/full/0 give a soft glow with no per-fragment work. Shared by the survey-beam and the
/// cool scan flick (G3). Robust when the camera is on the beam axis.
pub fn ribbon_seg(a: Vec3, b: Vec3, cam: Vec3, alpha: f32, color: [f32; 3]) -> Vec<OverlayVertex> {
    if alpha <= 0.0 {
        return Vec::new();
    }
    let dir = (b - a).normalize_or_zero();
    let mid = (a + b) * 0.5;
    let mut w = dir.cross(cam - mid);
    if w.length_squared() < 1e-4 {
        w = dir.cross(Vec3::Y);
    }
    if w.length_squared() < 1e-4 {
        w = dir.cross(Vec3::X); // segment is vertical
    }
    let w = w.normalize_or_zero();
    let offsets = [-HALO_HW, -CORE_HW, CORE_HW, HALO_HW];
    let edge_a = [0.0, alpha, alpha, 0.0]; // feathered: transparent halo edges, bright core
    let v = |p: Vec3, al: f32| OverlayVertex {
        pos: p.to_array(),
        color,
        alpha: al,
    };
    let mut out = Vec::with_capacity(18);
    for i in 0..3 {
        let (oa, ob) = (offsets[i], offsets[i + 1]);
        let (aa, ab) = (edge_a[i], edge_a[i + 1]);
        let (p0, p1) = (a + w * oa, a + w * ob);
        let (p2, p3) = (b + w * ob, b + w * oa);
        out.extend_from_slice(&[v(p0, aa), v(p1, ab), v(p2, ab)]);
        out.extend_from_slice(&[v(p0, aa), v(p2, ab), v(p3, aa)]);
    }
    out
}

/// Perpendicular distance from point `p` to the segment `a..b` (clamped to the segment).
pub fn dist_point_segment(p: Vec3, a: Vec3, b: Vec3) -> f32 {
    let ab = b - a;
    let len2 = ab.length_squared();
    if len2 <= 1e-6 {
        return (p - a).length();
    }
    let t = ((p - a).dot(ab) / len2).clamp(0.0, 1.0);
    (p - (a + ab * t)).length()
}

/// Is `p` close enough to the beam line to be swept up on cast?
pub fn on_path(p: Vec3, a: Vec3, b: Vec3) -> bool {
    dist_point_segment(p, a, b) <= COLLECT_RADIUS
}

/// Reach-gate for boarding the cruiser: only if the parked ship is within beam reach of the
/// player (so wandering far on foot has consequence). `LENGTH` is the reach.
pub fn within_reach(player: Vec3, ship: Vec3) -> bool {
    player.distance(ship) <= LENGTH
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fade_runs_full_to_zero() {
        let b = Beam::cast(Vec3::ZERO, Vec3::X, 0.0);
        assert!((b.alpha(0.0) - 1.0).abs() < 1e-6);
        assert!(b.alpha(LIFESPAN * 0.5) > 0.0 && b.alpha(LIFESPAN * 0.5) < 1.0);
        assert_eq!(b.alpha(LIFESPAN), 0.0);
        assert_eq!(b.alpha(LIFESPAN + 1.0), 0.0);
        assert!(!b.dead(LIFESPAN - 0.1));
        assert!(b.dead(LIFESPAN));
    }

    #[test]
    fn collect_path_picks_glyphs_near_the_line_only() {
        let a = Vec3::new(0.0, 0.0, 0.0);
        let b = Vec3::new(10.0, 0.0, 0.0);
        assert!(on_path(Vec3::new(5.0, 0.5, 0.0), a, b)); // near the line
        assert!(on_path(Vec3::new(5.0, 0.0, 1.0), a, b)); // within radius
        assert!(!on_path(Vec3::new(5.0, 5.0, 0.0), a, b)); // far off
        assert!(!on_path(Vec3::new(20.0, 0.0, 0.0), a, b)); // past the end
    }

    #[test]
    fn cast_makes_a_fixed_length_segment() {
        let b = Beam::cast(Vec3::new(1.0, 2.0, 3.0), Vec3::new(0.0, 0.0, 2.0), 0.0);
        assert!(((b.b - b.a).length() - LENGTH).abs() < 1e-3);
        assert_eq!(b.at(0.0), b.a);
        assert!((b.at(1.0) - b.b).length() < 1e-3);
    }

    #[test]
    fn reach_gate_bounds_recall() {
        assert!(within_reach(Vec3::ZERO, Vec3::new(0.0, 0.0, LENGTH - 1.0)));
        assert!(!within_reach(Vec3::ZERO, Vec3::new(0.0, 0.0, LENGTH + 1.0)));
    }

    #[test]
    fn ribbon_is_empty_once_faded() {
        let b = Beam::cast(Vec3::ZERO, Vec3::X, 0.0);
        assert!(!b.ribbon(Vec3::new(0.0, 5.0, 0.0), 0.0).is_empty());
        assert!(b
            .ribbon(Vec3::new(0.0, 5.0, 0.0), LIFESPAN + 1.0)
            .is_empty());
    }
}
