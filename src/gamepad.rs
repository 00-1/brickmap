//! Gamepad / controller input (D7) — fly with a pad. The platform layer reads a pad
//! each frame and produces a normalised [`PadInput`]; the app maps that onto the
//! `CameraController` (analog move + look) and the auto-fly toggle. Native desktop uses
//! `gilrs`; web uses the browser Gamepad API. Android-native pad support is deferred
//! (the web build already covers a phone + USB-C pad).
//!
//! Convention (so platforms agree regardless of their raw sign conventions):
//! `move_*`/`look_*` are −1..1 with **+forward = into the scene, +right, +up**.

/// One frame of pad input, deadzoned + sign-normalised.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct PadInput {
    pub strafe: f32,   // + = right
    pub forward: f32,  // + = forward
    pub vertical: f32, // + = up
    pub look_x: f32,   // + = turn right
    pub look_y: f32,   // + = look down (matches mouse dy)
    /// Rising-edge "toggle auto-fly" (face button A / cross).
    pub toggle_fly: bool,
    /// Any stick/button engaged this frame (so the app can yield auto-fly to the pad).
    pub active: bool,
}

/// Radial-ish deadzone on a single axis, rescaled so motion past the zone starts at 0.
pub fn deadzone(v: f32) -> f32 {
    const DZ: f32 = 0.18;
    if v.abs() < DZ {
        0.0
    } else {
        ((v.abs() - DZ) / (1.0 - DZ)).min(1.0) * v.signum()
    }
}

impl PadInput {
    /// Build a normalised input from raw stick/button readings (already in the +forward/
    /// +right/+up convention). Applies the deadzone and sets `active`. `look_*` are raw
    /// (scaled by the caller into pixels). Pure — unit-tested.
    pub fn from_raw(
        strafe: f32,
        forward: f32,
        vertical: f32,
        look_x: f32,
        look_y: f32,
        toggle_fly: bool,
    ) -> PadInput {
        let strafe = deadzone(strafe);
        let forward = deadzone(forward);
        let lx = deadzone(look_x);
        let ly = deadzone(look_y);
        let active = strafe != 0.0
            || forward != 0.0
            || vertical != 0.0
            || lx != 0.0
            || ly != 0.0
            || toggle_fly;
        PadInput {
            strafe,
            forward,
            vertical,
            look_x: lx,
            look_y: ly,
            toggle_fly,
            active,
        }
    }
}

/// How much a full look-stick deflection turns per frame, in "mouse pixels" fed to the
/// controller's existing sensitivity. Tuned for a ~60fps feel.
pub const LOOK_SPEED: f32 = 9.0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deadzone_kills_small_drift_and_rescales() {
        assert_eq!(deadzone(0.1), 0.0);
        assert_eq!(deadzone(0.0), 0.0);
        // Just past the zone starts near 0, full deflection reaches 1.
        assert!(deadzone(0.2).abs() < 0.1);
        assert!((deadzone(1.0) - 1.0).abs() < 1e-6);
        assert!((deadzone(-1.0) + 1.0).abs() < 1e-6);
    }

    #[test]
    fn from_raw_sets_active_and_deadzones() {
        let idle = PadInput::from_raw(0.05, -0.05, 0.0, 0.1, 0.0, false);
        assert!(!idle.active, "tiny drift should read as idle");
        let moving = PadInput::from_raw(0.9, 0.0, 0.0, 0.0, 0.0, false);
        assert!(moving.active);
        assert!(moving.strafe > 0.5);
        assert!(PadInput::from_raw(0.0, 0.0, 0.0, 0.0, 0.0, true).active); // button only
    }
}

// ---------------------------------------------------------------------------------------
// Platform pad readers. Each exposes `Pad::new()` + `Pad::poll() -> PadInput`.
// ---------------------------------------------------------------------------------------

#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
pub use native::Pad;
#[cfg(target_arch = "wasm32")]
pub use web::Pad;
// (Android re-exports `android::Pad` next to its module definition below.)

/// Desktop pad via `gilrs`.
#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
mod native {
    use super::PadInput;
    use gilrs::{Axis, Button, Gilrs};

    pub struct Pad {
        gilrs: Option<Gilrs>,
        prev_toggle: bool,
    }

    impl Pad {
        #[allow(clippy::new_without_default)]
        pub fn new() -> Pad {
            // If no gamepad subsystem is available, degrade to "no pad" rather than panic.
            let gilrs = Gilrs::new().ok();
            Pad {
                gilrs,
                prev_toggle: false,
            }
        }

        pub fn poll(&mut self) -> PadInput {
            let Some(gilrs) = self.gilrs.as_mut() else {
                return PadInput::default();
            };
            // Pump the event queue so gamepad state is current.
            while gilrs.next_event().is_some() {}
            let Some((_id, gp)) = gilrs.gamepads().next() else {
                return PadInput::default();
            };
            // gilrs: LeftStickY up = +1 (already +forward). Look stick: +X turns right,
            // and +Y up → we want +look_y = down, so negate.
            let strafe = gp.value(Axis::LeftStickX);
            let forward = gp.value(Axis::LeftStickY);
            let look_x = gp.value(Axis::RightStickX);
            let look_y = -gp.value(Axis::RightStickY);
            // Bumpers: RB up, LB down.
            let up = gp.is_pressed(Button::RightTrigger);
            let down = gp.is_pressed(Button::LeftTrigger);
            let vertical = (up as i32 - down as i32) as f32;
            // Toggle auto-fly on the rising edge of South (A / cross).
            let toggle_now = gp.is_pressed(Button::South);
            let toggle_fly = toggle_now && !self.prev_toggle;
            self.prev_toggle = toggle_now;
            PadInput::from_raw(strafe, forward, vertical, look_x, look_y, toggle_fly)
        }
    }
}

/// Web pad via the browser Gamepad API.
#[cfg(target_arch = "wasm32")]
mod web {
    use super::PadInput;

    pub struct Pad {
        prev_toggle: bool,
    }

    impl Pad {
        #[allow(clippy::new_without_default)]
        pub fn new() -> Pad {
            Pad { prev_toggle: false }
        }

        pub fn poll(&mut self) -> PadInput {
            let Some(pad) = first_gamepad() else {
                self.prev_toggle = false;
                return PadInput::default();
            };
            let axes = pad.axes();
            let buttons = pad.buttons();
            let axis = |i: u32| axes.get(i).as_f64().unwrap_or(0.0) as f32;
            let pressed = |i: u32| {
                buttons
                    .get(i)
                    .dyn_into::<web_sys::GamepadButton>()
                    .map(|b| b.pressed())
                    .unwrap_or(false)
            };
            // Standard mapping: axes 0/1 = left stick, 2/3 = right stick. Up = -1, so
            // +forward = -axis(1). buttons 4/5 = LB/RB, 0 = A.
            let strafe = axis(0);
            let forward = -axis(1);
            let look_x = axis(2);
            let look_y = axis(3); // +down already (web Y is +down)
            let vertical = (pressed(5) as i32 - pressed(4) as i32) as f32; // RB up, LB down
            let toggle_now = pressed(0);
            let toggle_fly = toggle_now && !self.prev_toggle;
            self.prev_toggle = toggle_now;
            PadInput::from_raw(strafe, forward, vertical, look_x, look_y, toggle_fly)
        }
    }

    use wasm_bindgen::JsCast;

    /// The first connected, non-null gamepad, if any.
    fn first_gamepad() -> Option<web_sys::Gamepad> {
        let pads = web_sys::window()?.navigator().get_gamepads().ok()?;
        for i in 0..pads.length() {
            if let Ok(gp) = pads.get(i).dyn_into::<web_sys::Gamepad>() {
                if gp.connected() {
                    return Some(gp);
                }
            }
        }
        None
    }
}

/// Android pad, fed from winit's `KeyboardInput`. winit drains the Android input queue
/// itself, so on Android we **drain the input queue ourselves** (before winit, see
/// `lib::android_main`) and read the full analog **sticks + triggers** from MotionEvents
/// plus buttons from KeyEvents. Left stick moves, right stick looks, shoulders add turn,
/// triggers go up/down, A toggles auto-fly.
#[cfg(target_os = "android")]
mod android {
    use super::PadInput;
    use winit::platform::android::activity::input::{Axis, InputEvent, KeyAction, Keycode};

    #[derive(Default)]
    pub struct Pad {
        // Latest analog stick + trigger state (read straight from MotionEvents).
        lx: f32,
        ly: f32,
        rx: f32,
        ry: f32,
        up: bool,
        down: bool,
        // Digital shoulder turn (fallback / extra), and the auto-fly toggle edge.
        turn_left: bool,
        turn_right: bool,
        toggle_pending: bool,
    }

    impl Pad {
        #[allow(clippy::new_without_default)]
        pub fn new() -> Pad {
            Pad::default()
        }

        /// Feed one Android input event. We drain the input queue ourselves (before
        /// winit) precisely so we can read the analog **stick** axes that winit drops.
        /// Returns `true` if it was a gamepad input we consumed.
        pub fn handle_android_input(&mut self, ev: &InputEvent) -> bool {
            match ev {
                InputEvent::MotionEvent(m) => {
                    let Some(p) = m.pointers().next() else {
                        return false;
                    };
                    // Gamepad sticks/triggers are normalised (−1..1 / 0..1). Touchscreen
                    // MotionEvents report *pixel* coords → out of range → ignored, so a
                    // stray screen touch can't fling the camera.
                    let ax = |a: Axis| {
                        let v = p.axis_value(a);
                        if v.abs() <= 1.05 {
                            v
                        } else {
                            0.0
                        }
                    };
                    self.lx = ax(Axis::X);
                    self.ly = ax(Axis::Y);
                    self.rx = ax(Axis::Z);
                    self.ry = ax(Axis::Rz);
                    let lt = ax(Axis::Ltrigger).max(ax(Axis::Brake));
                    let rt = ax(Axis::Rtrigger).max(ax(Axis::Gas));
                    self.down = lt > 0.5;
                    self.up = rt > 0.5;
                    true
                }
                InputEvent::KeyEvent(k) => {
                    let pressed = matches!(k.action(), KeyAction::Down);
                    match k.key_code() {
                        Keycode::ButtonL1 => self.turn_left = pressed,
                        Keycode::ButtonR1 => self.turn_right = pressed,
                        Keycode::ButtonR2 => self.up = pressed,
                        Keycode::ButtonL2 => self.down = pressed,
                        Keycode::ButtonA => {
                            if pressed {
                                self.toggle_pending = true;
                            }
                        }
                        _ => return false,
                    }
                    true
                }
                _ => false,
            }
        }

        pub fn poll(&mut self) -> PadInput {
            // Android joystick Y is +down, so forward = −ly. Right stick = look; shoulders
            // add a digital turn on top of the right-stick X.
            let look_x = self.rx + (self.turn_right as i32 - self.turn_left as i32) as f32;
            let vertical = (self.up as i32 - self.down as i32) as f32;
            let toggle = std::mem::take(&mut self.toggle_pending);
            PadInput::from_raw(self.lx, -self.ly, vertical, look_x, self.ry, toggle)
        }
    }
}
#[cfg(target_os = "android")]
pub use android::Pad;
