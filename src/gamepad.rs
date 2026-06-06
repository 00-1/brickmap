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
/// itself and surfaces gamepad **buttons** as key events (D-pad → arrows, face/shoulder
/// buttons → `Unidentified(NativeKeyCode::Android(code))`) — but it does *not* expose the
/// analog **stick** axes (joystick `MotionEvent`s are treated as touch). So a native pad
/// here is digital: D-pad moves (via the normal arrow-key path), shoulders turn, triggers
/// go up/down, A toggles auto-fly. Full analog sticks need bypassing winit's input loop
/// (a bigger change) — the web build keeps the full-stick experience meanwhile.
#[cfg(target_os = "android")]
mod android {
    use super::PadInput;

    // Android `KEYCODE_BUTTON_*` values (frameworks/native KeyCodes).
    const BUTTON_A: u32 = 96;
    const BUTTON_L1: u32 = 102;
    const BUTTON_R1: u32 = 103;
    const BUTTON_L2: u32 = 104;
    const BUTTON_R2: u32 = 105;

    #[derive(Default)]
    pub struct Pad {
        turn_left: bool,
        turn_right: bool,
        up: bool,
        down: bool,
        toggle_pending: bool,
    }

    impl Pad {
        #[allow(clippy::new_without_default)]
        pub fn new() -> Pad {
            Pad::default()
        }

        /// Feed an Android gamepad button (native keycode) from winit's `KeyboardInput`.
        /// Returns `true` if it was a button we handle. Shoulders turn, triggers raise/
        /// lower, A toggles auto-fly (on press).
        pub fn on_android_key(&mut self, keycode: u32, pressed: bool) -> bool {
            match keycode {
                BUTTON_L1 => self.turn_left = pressed,
                BUTTON_R1 => self.turn_right = pressed,
                BUTTON_R2 => self.up = pressed,
                BUTTON_L2 => self.down = pressed,
                BUTTON_A => {
                    if pressed {
                        self.toggle_pending = true;
                    }
                }
                _ => return false,
            }
            true
        }

        pub fn poll(&mut self) -> PadInput {
            let look_x = (self.turn_right as i32 - self.turn_left as i32) as f32;
            let vertical = (self.up as i32 - self.down as i32) as f32;
            let toggle = std::mem::take(&mut self.toggle_pending);
            // Movement (forward/strafe) comes from the D-pad via the normal arrow-key
            // path, so leave those zero here; we contribute turn + vertical + toggle.
            PadInput::from_raw(0.0, 0.0, vertical, look_x, 0.0, toggle)
        }
    }
}
#[cfg(target_os = "android")]
pub use android::Pad;
