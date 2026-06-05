//! Camera and free-fly controls — the start of the `scene` layer from
//! `docs/architecture.md`. Pure math + input *intent*; it knows nothing about
//! winit or wgpu (the app translates platform events into [`Action`]s and feeds
//! the resulting view-projection to the renderer).

use glam::{Mat4, Vec3};

/// A free-fly perspective camera. Orientation is yaw (around +Y) and pitch.
#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub position: Vec3,
    /// Radians around +Y. 0 looks toward +X.
    pub yaw: f32,
    /// Radians up/down, clamped away from straight up/down.
    pub pitch: f32,
    /// Vertical field of view, radians.
    pub fov_y: f32,
}

impl Camera {
    pub fn new(position: Vec3, yaw: f32, pitch: f32) -> Self {
        Camera {
            position,
            yaw,
            pitch,
            fov_y: 60f32.to_radians(),
        }
    }

    /// Point the camera at `target` from its current position.
    pub fn looking_at(position: Vec3, target: Vec3) -> Self {
        let dir = (target - position).normalize_or_zero();
        let yaw = dir.z.atan2(dir.x);
        let pitch = dir.y.clamp(-1.0, 1.0).asin();
        Camera::new(position, yaw, pitch)
    }

    /// Unit forward direction from yaw/pitch.
    pub fn forward(&self) -> Vec3 {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        Vec3::new(cp * cy, sp, cp * sy).normalize()
    }

    /// Unit right direction (horizontal).
    pub fn right(&self) -> Vec3 {
        self.forward().cross(Vec3::Y).normalize_or_zero()
    }

    pub fn view_proj(&self, aspect: f32) -> Mat4 {
        let proj = Mat4::perspective_rh(self.fov_y, aspect.max(0.001), 0.05, 2000.0);
        let view = Mat4::look_to_rh(self.position, self.forward(), Vec3::Y);
        proj * view
    }
}

/// A movement intent, decoupled from any particular key binding or platform.
#[derive(Clone, Copy, Debug)]
pub enum Action {
    Forward,
    Back,
    Left,
    Right,
    Up,
    Down,
}

/// Accumulates input and integrates it into a [`Camera`] each frame.
pub struct CameraController {
    forward: bool,
    back: bool,
    left: bool,
    right: bool,
    up: bool,
    down: bool,
    /// Look delta (in pixels) accumulated since the last `update`.
    look_dx: f32,
    look_dy: f32,
    /// Movement speed, world units per second.
    pub speed: f32,
    /// Look sensitivity, radians per pixel.
    pub sensitivity: f32,
}

impl CameraController {
    pub fn new(speed: f32) -> Self {
        CameraController {
            forward: false,
            back: false,
            left: false,
            right: false,
            up: false,
            down: false,
            look_dx: 0.0,
            look_dy: 0.0,
            speed,
            sensitivity: 0.0035,
        }
    }

    pub fn set_action(&mut self, action: Action, pressed: bool) {
        match action {
            Action::Forward => self.forward = pressed,
            Action::Back => self.back = pressed,
            Action::Left => self.left = pressed,
            Action::Right => self.right = pressed,
            Action::Up => self.up = pressed,
            Action::Down => self.down = pressed,
        }
    }

    /// Feed raw mouse motion (the app gates this on the pointer being captured).
    pub fn add_look(&mut self, dx: f32, dy: f32) {
        self.look_dx += dx;
        self.look_dy += dy;
    }

    /// Integrate accumulated input into `camera` over `dt` seconds.
    pub fn update(&mut self, camera: &mut Camera, dt: f32) {
        // Look: drag right turns right, drag up looks up.
        camera.yaw += self.look_dx * self.sensitivity;
        camera.pitch = (camera.pitch - self.look_dy * self.sensitivity).clamp(-1.547, 1.547);
        self.look_dx = 0.0;
        self.look_dy = 0.0;

        // Move along the camera basis; vertical is world-up.
        let mut dir = Vec3::ZERO;
        let forward = camera.forward();
        let right = camera.right();
        if self.forward {
            dir += forward;
        }
        if self.back {
            dir -= forward;
        }
        if self.right {
            dir += right;
        }
        if self.left {
            dir -= right;
        }
        if self.up {
            dir += Vec3::Y;
        }
        if self.down {
            dir -= Vec3::Y;
        }
        if dir != Vec3::ZERO {
            camera.position += dir.normalize() * self.speed * dt;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forward_is_unit_and_aims_where_yaw_points() {
        let c = Camera::new(Vec3::ZERO, 0.0, 0.0);
        assert!((c.forward().length() - 1.0).abs() < 1e-5);
        // yaw 0, pitch 0 -> +X
        assert!((c.forward() - Vec3::X).length() < 1e-5);
    }

    #[test]
    fn looking_at_faces_the_target() {
        let c = Camera::looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::new(10.0, 0.0, 0.0));
        assert!((c.forward() - Vec3::X).length() < 1e-4);
    }

    #[test]
    fn movement_is_framerate_independent() {
        let mut cam = Camera::new(Vec3::ZERO, 0.0, 0.0);
        let mut ctl = CameraController::new(10.0);
        ctl.set_action(Action::Forward, true);
        ctl.update(&mut cam, 1.0);
        // 10 units/sec for 1 sec along +X.
        assert!((cam.position - Vec3::new(10.0, 0.0, 0.0)).length() < 1e-4);
    }

    #[test]
    fn pitch_cannot_flip_over_the_top() {
        let mut cam = Camera::new(Vec3::ZERO, 0.0, 0.0);
        let mut ctl = CameraController::new(1.0);
        ctl.add_look(0.0, -100000.0); // yank the look up hard
        ctl.update(&mut cam, 0.016);
        assert!(cam.pitch <= 1.5471);
    }
}
