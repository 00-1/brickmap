//! brickmap — cross-platform voxel rendering engine.
//!
//! M1: a hand-built voxel chunk, meshed on the CPU and drawn through wgpu, that
//! you can fly around — on desktop and in the browser (WASM) from one code path.
//! See `docs/design.md`, `docs/architecture.md`, and `docs/roadmap.md`.

use std::sync::Arc;

use glam::Vec3;
use web_time::Instant;
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, DeviceId, ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{CursorGrabMode, Window, WindowId};

mod gfx;
pub mod mesh;
pub mod scene;
pub mod world;
use gfx::State;
use mesh::ChunkMesh;
use scene::{Action, Camera, CameraController};
use world::{BlockId, Section};

/// Window/canvas init size. On the web this is also the canvas backing size.
const INITIAL_SIZE: (u32, u32) = (960, 720);
#[cfg(target_arch = "wasm32")]
const CANVAS_ID: &str = "brickmap-canvas";

/// Delivered through the event loop once async GPU init finishes. Needed because
/// `request_device` is async and winit's `resumed` is not — on the web we cannot
/// block, so we hand the finished `State` back as a user event.
// Constructed only on the web (after async GPU init); matched on all targets.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
enum AppEvent {
    Initialized(State),
}

struct App {
    state: Option<State>,
    // Only used on the web (async init handoff); inert on native.
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    proxy: Option<EventLoopProxy<AppEvent>>,
    /// The scene to draw — built once on the CPU, uploaded when the GPU is ready.
    mesh: ChunkMesh,

    camera: Camera,
    controller: CameraController,
    /// Timestamp of the previous frame, for frame-rate-independent movement.
    last_frame: Option<Instant>,
    /// Whether the pointer is captured (mouselook active).
    cursor_locked: bool,
}

impl App {
    /// Capture/release the pointer for mouselook. Locked grab gives relative
    /// motion (and Pointer Lock on the web); fall back to Confined where Locked
    /// isn't supported.
    fn set_capture(&mut self, capture: bool) {
        let Some(state) = &self.state else { return };
        let window = state.window();
        if capture {
            let _ = window
                .set_cursor_grab(CursorGrabMode::Locked)
                .or_else(|_| window.set_cursor_grab(CursorGrabMode::Confined));
            window.set_cursor_visible(false);
        } else {
            let _ = window.set_cursor_grab(CursorGrabMode::None);
            window.set_cursor_visible(true);
        }
        self.cursor_locked = capture;
    }
}

/// Map a physical key to a movement intent (WASD + Space/Shift), or `None`.
fn key_action(code: KeyCode) -> Option<Action> {
    Some(match code {
        KeyCode::KeyW | KeyCode::ArrowUp => Action::Forward,
        KeyCode::KeyS | KeyCode::ArrowDown => Action::Back,
        KeyCode::KeyA | KeyCode::ArrowLeft => Action::Left,
        KeyCode::KeyD | KeyCode::ArrowRight => Action::Right,
        KeyCode::Space => Action::Up,
        KeyCode::ShiftLeft | KeyCode::ControlLeft => Action::Down,
        _ => return None,
    })
}

impl ApplicationHandler<AppEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return; // already initialised (e.g. resume after suspend on mobile)
        }

        let attrs = window_attributes();
        let window = Arc::new(
            event_loop
                .create_window(attrs)
                .expect("failed to create window"),
        );

        #[cfg(not(target_arch = "wasm32"))]
        {
            // Native: just block until the GPU is ready.
            self.state = Some(pollster::block_on(State::new(window, &self.mesh)));
        }

        #[cfg(target_arch = "wasm32")]
        {
            // Web: kick off async init and deliver the result via the proxy.
            let proxy = self.proxy.take().expect("proxy missing");
            let mesh = self.mesh.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let state = State::new(window, &mesh).await;
                let _ = proxy.send_event(AppEvent::Initialized(state));
            });
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: AppEvent) {
        let AppEvent::Initialized(state) = event;
        state.window().request_redraw();
        self.state = Some(state);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(state) = self.state.as_mut() {
                    state.resize(size);
                    state.window().request_redraw();
                }
            }
            WindowEvent::KeyboardInput { event: key, .. } => {
                if let PhysicalKey::Code(code) = key.physical_key {
                    if code == KeyCode::Escape && key.state.is_pressed() {
                        // Release the pointer. (The browser also does this on Esc.)
                        self.set_capture(false);
                    } else if let Some(action) = key_action(code) {
                        self.controller.set_action(action, key.state.is_pressed());
                    }
                }
            }
            // Click to capture the pointer for mouselook (and to re-capture after
            // Esc / tabbing away). Idempotent, so re-clicking is harmless.
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state: ElementState::Pressed,
                ..
            } => self.set_capture(true),
            // Pointer lock is lost when focus leaves; reflect that in our state.
            WindowEvent::Focused(false) => self.set_capture(false),
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = self
                    .last_frame
                    .map(|t| (now - t).as_secs_f32().min(0.1))
                    .unwrap_or(0.0);
                self.last_frame = Some(now);
                self.controller.update(&mut self.camera, dt);

                if let Some(state) = self.state.as_mut() {
                    let view_proj = self.camera.view_proj(state.aspect());
                    // `render` handles lost/outdated/transient surfaces internally.
                    state.render(view_proj);
                    // Drive a continuous loop so held keys animate.
                    state.window().request_redraw();
                }
            }
            _ => {}
        }
    }

    fn device_event(&mut self, _event_loop: &ActiveEventLoop, _id: DeviceId, event: DeviceEvent) {
        // Raw mouse motion drives mouselook while the pointer is captured.
        if let DeviceEvent::MouseMotion { delta } = event {
            if self.cursor_locked {
                self.controller.add_look(delta.0 as f32, delta.1 as f32);
            }
        }
    }
}

fn window_attributes() -> winit::window::WindowAttributes {
    let attrs = Window::default_attributes()
        .with_title("brickmap")
        .with_inner_size(winit::dpi::LogicalSize::new(
            INITIAL_SIZE.0 as f64,
            INITIAL_SIZE.1 as f64,
        ));

    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        use winit::platform::web::WindowAttributesExtWebSys;
        let canvas = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id(CANVAS_ID))
            .and_then(|e| e.dyn_into::<web_sys::HtmlCanvasElement>().ok())
            .expect("could not find a <canvas id=\"brickmap-canvas\"> element");
        canvas.set_width(INITIAL_SIZE.0);
        canvas.set_height(INITIAL_SIZE.1);
        return attrs.with_canvas(Some(canvas));
    }

    #[cfg(not(target_arch = "wasm32"))]
    attrs
}

/// Shared entry point used by both the native binary and the WASM start shim.
pub fn run() {
    init_logging();

    let event_loop = EventLoop::<AppEvent>::with_user_event()
        .build()
        .expect("failed to build event loop");

    let mesh = mesh::mesh_section(&demo_section());

    // Frame the camera on the meshed scene from its bounds, then let the user fly.
    let min = Vec3::from(mesh.aabb.min);
    let max = Vec3::from(mesh.aabb.max);
    let center = (min + max) * 0.5;
    let radius = ((max - min).length() * 0.5).max(1.0);
    let eye = center + Vec3::new(1.0, 0.7, 1.3).normalize() * radius * 2.6;

    let mut app = App {
        state: None,
        proxy: Some(event_loop.create_proxy()),
        mesh,
        camera: Camera::looking_at(eye, center),
        controller: CameraController::new(radius * 1.2),
        last_frame: None,
        cursor_locked: false,
    };

    event_loop.run_app(&mut app).expect("event loop error");
}

/// A hand-built test chunk: a stepped pyramid centred in the section. Its terraces
/// make the mesher's face-culling obvious — vertical step faces and exposed tops,
/// no hidden interior faces. (M1 scaffolding; M3 replaces this with real terrain.)
fn demo_section() -> Section {
    // Cycle a few debug block types up the height so the steps read clearly.
    const PALETTE: [BlockId; 4] = [BlockId(1), BlockId(2), BlockId(3), BlockId(4)];
    const LEVELS: u32 = 8;

    let mut section = Section::new();
    for level in 0..LEVELS {
        let lo = 8 + level;
        let hi = 24 - level;
        let block = PALETTE[level as usize % PALETTE.len()];
        for z in lo..hi {
            for x in lo..hi {
                section.set(x, level, z, block);
            }
        }
    }
    section
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn wasm_start() {
    run();
}

fn init_logging() {
    #[cfg(target_arch = "wasm32")]
    {
        console_error_panic_hook::set_once();
        let _ = console_log::init_with_level(log::Level::Info);
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    }
}
