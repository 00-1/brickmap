//! Touchscreen input (D9) — the platform layer surfaces winit `WindowEvent::Touch` as a generic,
//! **content-agnostic** normalised stream (id, phase, position in `0..1` across the surface),
//! exactly as [`gamepad`](crate::gamepad) surfaces a normalised `PadInput`. **No game concepts
//! here** — the game (`scraped-again`) owns the overlay layout + the touch→action mapping.
//!
//! The app owns the winit event loop (architecture §6), so it converts `WindowEvent::Touch` into a
//! [`TouchPoint`] with [`TouchPoint::new`]; this crate just defines the generic type + the pure
//! pixel→`0..1` normalisation, so it carries no winit dependency and stays unit-testable.

/// What a touch is doing this event.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum TouchPhase {
    /// Finger went down.
    Start,
    /// Finger moved while down.
    Move,
    /// Finger lifted.
    End,
    /// The system cancelled the touch (treat as `End` with no action).
    Cancel,
}

/// One touch sample: a stable finger `id`, its `phase`, and its position **normalised** to the
/// surface — `x` left→right, `y` top→bottom, each in `0..=1`.
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct TouchPoint {
    pub id: u64,
    pub phase: TouchPhase,
    pub x: f32,
    pub y: f32,
}

impl TouchPoint {
    /// Build from raw surface pixels + the surface size, clamping to `0..=1`. `(w, h)` must be the
    /// physical surface dimensions the touch coords are in.
    pub fn new(id: u64, phase: TouchPhase, px: f32, py: f32, w: f32, h: f32) -> TouchPoint {
        TouchPoint {
            id,
            phase,
            x: (px / w.max(1.0)).clamp(0.0, 1.0),
            y: (py / h.max(1.0)).clamp(0.0, 1.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalises_and_clamps() {
        let t = TouchPoint::new(3, TouchPhase::Start, 640.0, 360.0, 1280.0, 720.0);
        assert_eq!((t.x, t.y), (0.5, 0.5));
        assert_eq!(t.id, 3);
        // Out-of-bounds + degenerate size clamp into range, never NaN/inf.
        let lo = TouchPoint::new(0, TouchPhase::Move, -50.0, 9999.0, 0.0, 0.0);
        assert!((0.0..=1.0).contains(&lo.x) && (0.0..=1.0).contains(&lo.y));
    }
}
