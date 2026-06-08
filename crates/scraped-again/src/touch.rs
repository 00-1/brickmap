//! D9 — phone **touch controls**: the on-screen overlay layout + the **pure touch→action
//! mapping**. The engine ([`brickmap::touch`]) surfaces generic normalised touch points; this
//! module (game-side) owns *what they mean* — two vertical **sliders** (steer + altitude) down the
//! screen edges, four **buttons** (`1` console · `2` map · `A` cruise · `B` board/exit/hail), and
//! **tap-the-view** = cast the survey-beam (the universal verb). It adds **no new control logic** —
//! the app routes these onto the same `CameraController` / mode / console paths the keys + pad
//! already drive; this is just a new *input source*.
//!
//! Everything here is **pure + unit-tested** (no screen). On-device *feel*-tuning — slider
//! sensitivity, exact button size/placement, per-pixel tap targeting — is the device-gated human
//! follow-up (per the brief); the layout rects + sensitivities below are the pinned v1 defaults.
//!
//! D10 adds the **visible** overlay: [`Layout::overlay_rects`] turns these same `Layout` rects into
//! the engine's generic filled-rect HUD primitive ([`brickmap::hud::UiRect`]) — so the drawn
//! controls and the hit-zones share one source of truth and can't drift.

use crate::hud::UiRect;

/// A normalised rectangle (all in `0..=1`, `y` top→bottom).
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }
    /// A vertical slider's value for a touch at `y`: top = `+1`, centre = `0`, bottom = `-1`.
    fn slider_value(&self, y: f32) -> f32 {
        let mid = self.y + self.h * 0.5;
        (((mid - y) / (self.h * 0.5)).clamp(-1.0, 1.0) * 100.0).round() / 100.0
    }
}

/// Which control a touch landed on.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Region {
    LeftSlider,
    RightSlider,
    Btn1,
    Btn2,
    BtnA,
    BtnB,
    /// The central display (anything not on the control strips).
    View,
}

/// The context the buttons + view-tap reinterpret under (the modal control table).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Ctx {
    Flight,
    Walk,
    /// A menu (console / map / codex) is open.
    Menu,
}

/// A discrete action from a button press or a view tap (the app dispatches these onto the existing
/// key/mode/console paths).
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Tap {
    Cruise,            // A (flight): toggle auto-forward; (walk): toggle auto-walk
    Board,             // B: land&exit / board / hail
    Console,           // 1
    Map,               // 2
    Back,              // B in a menu: close
    Beam,              // tap the view in play: cast the survey-beam at centre (v1)
    MenuTap(f32, f32), // tap the view with a menu open: hit-test a block/routine at (x,y)
}

/// The on-screen control layout (pinned v1 defaults — landscape, a control strip down each edge).
#[derive(Copy, Clone, Debug)]
pub struct Layout {
    pub left_slider: Rect,
    pub right_slider: Rect,
    pub btn1: Rect,
    pub btn2: Rect,
    pub btn_a: Rect,
    pub btn_b: Rect,
}

impl Default for Layout {
    fn default() -> Self {
        // Left strip x∈[0.02,0.12]; right strip x∈[0.88,0.98]. Sliders up top, buttons below.
        let lx = 0.02;
        let rx = 0.88;
        let sw = 0.10;
        Layout {
            left_slider: Rect {
                x: lx,
                y: 0.06,
                w: sw,
                h: 0.50,
            },
            right_slider: Rect {
                x: rx,
                y: 0.06,
                w: sw,
                h: 0.50,
            },
            btn1: Rect {
                x: lx,
                y: 0.60,
                w: sw,
                h: 0.16,
            },
            btn2: Rect {
                x: lx,
                y: 0.80,
                w: sw,
                h: 0.16,
            },
            btn_a: Rect {
                x: rx,
                y: 0.60,
                w: sw,
                h: 0.16,
            },
            btn_b: Rect {
                x: rx,
                y: 0.80,
                w: sw,
                h: 0.16,
            },
        }
    }
}

impl Layout {
    /// Classify a touch position into the control it hit.
    pub fn classify(&self, x: f32, y: f32) -> Region {
        if self.left_slider.contains(x, y) {
            Region::LeftSlider
        } else if self.right_slider.contains(x, y) {
            Region::RightSlider
        } else if self.btn1.contains(x, y) {
            Region::Btn1
        } else if self.btn2.contains(x, y) {
            Region::Btn2
        } else if self.btn_a.contains(x, y) {
            Region::BtnA
        } else if self.btn_b.contains(x, y) {
            Region::BtnB
        } else {
            Region::View
        }
    }

    /// The slider value `[-1,1]` for a touch in a slider region (else `None`).
    pub fn slider_value(&self, region: Region, y: f32) -> Option<f32> {
        match region {
            Region::LeftSlider => Some(self.left_slider.slider_value(y)),
            Region::RightSlider => Some(self.right_slider.slider_value(y)),
            _ => None,
        }
    }

    /// The discrete action a **button press** maps to under `ctx` (rising-edge: call on touch
    /// *down*). Sliders/View return `None` (they're not buttons).
    pub fn button_tap(&self, region: Region, ctx: Ctx) -> Option<Tap> {
        match region {
            Region::Btn1 => Some(Tap::Console), // 1 toggles the console in every context
            Region::Btn2 => Some(Tap::Map),     // 2 toggles the map
            Region::BtnA => match ctx {
                Ctx::Menu => None, // A is unused in a menu
                _ => Some(Tap::Cruise),
            },
            Region::BtnB => match ctx {
                Ctx::Menu => Some(Tap::Back), // B closes the menu
                _ => Some(Tap::Board),        // land&exit / board / hail
            },
            _ => None,
        }
    }

    /// The action a **view tap** maps to under `ctx` (call on touch *up* in the View region):
    /// fire the beam in play, or hit-test the menu at `(x,y)`.
    pub fn view_tap(&self, ctx: Ctx, x: f32, y: f32) -> Tap {
        match ctx {
            Ctx::Menu => Tap::MenuTap(x, y),
            _ => Tap::Beam,
        }
    }

    /// Build the **visible overlay rects** (D10) for the renderer's generic filled-rect HUD, derived
    /// **from the same `Layout` rects used for hit-testing** (so the visual can't drift from the
    /// hit-zones). Per slider: a dimmed track + a brighter **handle** at its value (`left`/`right` ∈
    /// `[-1,1]`, top = `+1`). Per button: a dim rect, **brightened** if it's the `pressed` one.
    /// Pure → headless-renderable + unit-testable. (Colours/opacity are the deferred eye-tuning.)
    pub fn overlay_rects(&self, left: f32, right: f32, pressed: Option<Region>) -> Vec<UiRect> {
        const TRACK: [f32; 4] = [0.55, 0.62, 0.72, 0.16]; // dim cool, translucent
        const HANDLE: [f32; 4] = [0.85, 0.90, 1.0, 0.5];
        const BTN: [f32; 4] = [0.55, 0.62, 0.72, 0.22];
        const BTN_ON: [f32; 4] = [0.92, 0.96, 1.0, 0.55];
        let rect = |r: Rect, c: [f32; 4]| UiRect {
            x0: r.x,
            y0: r.y,
            x1: r.x + r.w,
            y1: r.y + r.h,
            color: c,
        };
        let mut out = Vec::with_capacity(8);
        for (sr, v) in [(self.left_slider, left), (self.right_slider, right)] {
            out.push(rect(sr, TRACK));
            // Handle: top for +1, bottom for -1.
            let frac = (1.0 - (v.clamp(-1.0, 1.0) + 1.0) * 0.5).clamp(0.0, 1.0);
            let hh = sr.h * 0.08;
            let cy = (sr.y + frac * sr.h).min(sr.y + sr.h - hh);
            out.push(rect(
                Rect {
                    x: sr.x,
                    y: cy,
                    w: sr.w,
                    h: hh,
                },
                HANDLE,
            ));
        }
        for (br, region) in [
            (self.btn1, Region::Btn1),
            (self.btn2, Region::Btn2),
            (self.btn_a, Region::BtnA),
            (self.btn_b, Region::BtnB),
        ] {
            let c = if pressed == Some(region) { BTN_ON } else { BTN };
            out.push(rect(br, c));
        }
        out
    }

    /// A compact text depiction of the on-screen controls for the HUD/text path (the v1 overlay —
    /// the precise edge-strip placement + dimming is the deferred on-device visual). Shows each
    /// slider as a 5-cell bar with a `●` at its current value (`left`/`right` ∈ `[-1,1]`), and the
    /// four labelled buttons, captioned by `ctx`. Pure → headless-renderable + unit-testable.
    pub fn overlay(&self, left: f32, right: f32, ctx: Ctx) -> String {
        // A 5-cell vertical slider rendered top→bottom; the marker cell holds `●`.
        let bar = |v: f32| {
            let cells = 5i32;
            let pos =
                (((1.0 - (v.clamp(-1.0, 1.0) + 1.0) * 0.5) * (cells - 1) as f32).round()) as i32;
            (0..cells)
                .map(|i| if i == pos { '●' } else { '·' })
                .collect::<String>()
        };
        let (a, b) = match ctx {
            Ctx::Menu => ("—", "back"),
            Ctx::Walk => ("walk", "board"),
            Ctx::Flight => ("cruise", "board"),
        };
        let tap = match ctx {
            Ctx::Menu => "tap: select",
            _ => "tap: beam",
        };
        format!(
            "TOUCH  L[alt {}]  R[turn {}]   [1]console [2]map  [A]{a} [B]{b}  · {tap}",
            bar(left),
            bar(right),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_hits_each_control_and_view() {
        let l = Layout::default();
        // Centre of each control rect classifies to it.
        let centre = |r: Rect| (r.x + r.w * 0.5, r.y + r.h * 0.5);
        let (x, y) = centre(l.left_slider);
        assert_eq!(l.classify(x, y), Region::LeftSlider);
        let (x, y) = centre(l.right_slider);
        assert_eq!(l.classify(x, y), Region::RightSlider);
        let (x, y) = centre(l.btn1);
        assert_eq!(l.classify(x, y), Region::Btn1);
        let (x, y) = centre(l.btn_b);
        assert_eq!(l.classify(x, y), Region::BtnB);
        // The middle of the screen is the View.
        assert_eq!(l.classify(0.5, 0.5), Region::View);
    }

    #[test]
    fn slider_value_top_centre_bottom() {
        let l = Layout::default();
        let s = l.left_slider;
        assert_eq!(l.slider_value(Region::LeftSlider, s.y), Some(1.0)); // top = +1
        assert_eq!(
            l.slider_value(Region::LeftSlider, s.y + s.h * 0.5),
            Some(0.0)
        ); // centre = 0
        assert_eq!(l.slider_value(Region::LeftSlider, s.y + s.h), Some(-1.0)); // bottom = -1
        assert_eq!(l.slider_value(Region::View, 0.5), None); // not a slider
    }

    #[test]
    fn buttons_are_context_modal() {
        let l = Layout::default();
        // A = cruise in flight/walk, nothing in a menu.
        assert_eq!(l.button_tap(Region::BtnA, Ctx::Flight), Some(Tap::Cruise));
        assert_eq!(l.button_tap(Region::BtnA, Ctx::Walk), Some(Tap::Cruise));
        assert_eq!(l.button_tap(Region::BtnA, Ctx::Menu), None);
        // B = board in play, back in a menu.
        assert_eq!(l.button_tap(Region::BtnB, Ctx::Flight), Some(Tap::Board));
        assert_eq!(l.button_tap(Region::BtnB, Ctx::Menu), Some(Tap::Back));
        // 1/2 always console/map; sliders/view aren't buttons.
        assert_eq!(l.button_tap(Region::Btn1, Ctx::Flight), Some(Tap::Console));
        assert_eq!(l.button_tap(Region::Btn2, Ctx::Menu), Some(Tap::Map));
        assert_eq!(l.button_tap(Region::View, Ctx::Flight), None);
        assert_eq!(l.button_tap(Region::LeftSlider, Ctx::Flight), None);
    }

    #[test]
    fn view_tap_is_beam_in_play_and_hit_test_in_menu() {
        let l = Layout::default();
        assert_eq!(l.view_tap(Ctx::Flight, 0.5, 0.5), Tap::Beam);
        assert_eq!(l.view_tap(Ctx::Walk, 0.3, 0.7), Tap::Beam);
        assert_eq!(l.view_tap(Ctx::Menu, 0.4, 0.6), Tap::MenuTap(0.4, 0.6));
    }

    #[test]
    fn overlay_rects_derive_from_layout_hit_zones() {
        // D10: the drawn rects come from the SAME Layout rects used for hit-testing, so the
        // visual == the hit-zone. 2 sliders (track + handle) + 4 buttons = 10 rects.
        let l = Layout::default();
        let rects = l.overlay_rects(0.0, 0.0, None);
        assert_eq!(rects.len(), 8); // 2 sliders × {track, handle} + 4 buttons
                                    // The first rect is the left-slider track — exactly the left_slider region.
        let track = rects[0];
        let s = l.left_slider;
        assert!((track.x0 - s.x).abs() < 1e-6 && (track.y0 - s.y).abs() < 1e-6);
        assert!((track.x1 - (s.x + s.w)).abs() < 1e-6 && (track.y1 - (s.y + s.h)).abs() < 1e-6);
        // A button rect matches its layout region (btn1 is rect index 4: 2 sliders × 2).
        let b1 = rects[4];
        assert!((b1.x0 - l.btn1.x).abs() < 1e-6 && (b1.y0 - l.btn1.y).abs() < 1e-6);
        // The handle sits inside its track; +1 → near the top, -1 → near the bottom.
        let top = l.overlay_rects(1.0, 0.0, None)[1]; // left handle at +1
        let bot = l.overlay_rects(-1.0, 0.0, None)[1]; // left handle at -1
        assert!(top.y0 < bot.y0, "handle moves down as value decreases");
        assert!(
            top.y0 >= s.y - 1e-6 && bot.y1 <= s.y + s.h + 1e-6,
            "handle stays in the track"
        );
        // A pressed button is brighter (higher alpha) than an unpressed one.
        let pressed = l.overlay_rects(0.0, 0.0, Some(Region::BtnA));
        let idx_a = 6; // 2 sliders×2 + btn1 + btn2
        assert!(pressed[idx_a].color[3] > rects[idx_a].color[3]);
    }

    #[test]
    fn overlay_shows_sliders_buttons_and_context() {
        let l = Layout::default();
        let s = l.overlay(1.0, 0.0, Ctx::Flight);
        // Both sliders + all four buttons + the tap caption are present.
        assert!(s.contains('●') && s.contains('·'));
        assert!(s.contains("[1]console") && s.contains("[2]map"));
        assert!(s.contains("[A]cruise") && s.contains("[B]board"));
        assert!(s.contains("tap: beam"));
        // Context relabels A/B + the tap caption.
        let m = l.overlay(0.0, 0.0, Ctx::Menu);
        assert!(m.contains("[B]back") && m.contains("tap: select"));
    }
}
