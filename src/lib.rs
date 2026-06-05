//! brickmap — cross-platform voxel rendering engine.
//!
//! M1: a hand-built voxel chunk, meshed on the CPU and drawn through wgpu, that
//! you can fly around — on desktop and in the browser (WASM) from one code path.
//! See `docs/design.md`, `docs/architecture.md`, and `docs/roadmap.md`.

use std::collections::HashSet;
use std::sync::Arc;

use glam::Vec3;
use web_time::Instant;
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, DeviceId, ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{CursorGrabMode, Window, WindowId};

mod gfx;
#[cfg(not(target_arch = "wasm32"))]
pub mod headless;
pub mod mesh;
pub mod particles;
pub mod scene;
pub mod world;
pub mod worldgen;
use gfx::{ChunkInstance, State};
use mesh::{greedy_mesh_section_with, Neighbors};
use particles::ParticleSystem;
use scene::{Action, Camera, CameraController};
use world::{ChunkCoord, Section, World};

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
    camera: Camera,
    controller: CameraController,
    particles: ParticleSystem,
    /// The procedural world, streamed in around the camera as it travels (M3). Only
    /// chunks near the camera are resident; distant ones are evicted to bound memory.
    world: World,
    /// Chunk coords currently uploaded to the GPU (the renderer's draw set).
    loaded: HashSet<ChunkCoord>,
    /// Cinematic auto-fly: on by default so the build is watchable with no input
    /// (mobile / hands-off). Manual input switches it off; `F` toggles it.
    auto_fly: bool,
    auto_fly_angle: f32,
    /// Live aesthetic dials (D2): `[wobble snap, colour steps]`. On the web these are
    /// driven by `controls`; on native they stay at the defaults.
    wobble: f32,
    color_steps: f32,
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

    /// Stream chunks in and out around the camera (M3 part 3). Generates + meshes
    /// newly-entered chunks (nearest first, within a per-frame budget) and evicts
    /// those that have fallen outside the radius. Synchronous for now (design D3);
    /// M6 moves it off-thread.
    fn stream(&mut self) {
        if self.state.is_none() {
            return;
        }
        let s = Section::SIZE as f32;
        let ccx = (self.camera.position.x / s).floor() as i32;
        let ccz = (self.camera.position.z / s).floor() as i32;

        // Evict chunks (GPU + CPU) that have drifted beyond the radius (+1 hysteresis
        // so a chunk straddling the boundary doesn't thrash). Field-split borrows.
        let keep = STREAM_RADIUS + 1;
        let state = self.state.as_mut().unwrap();
        let world = &mut self.world;
        self.loaded.retain(|&coord| {
            let (cx, _cy, cz) = coord;
            if (cx - ccx).abs() > keep || (cz - ccz).abs() > keep {
                state.remove_chunk(coord);
                world.remove(coord);
                false
            } else {
                true
            }
        });

        // Load nearest-missing chunks, ring by ring, until the frame budget runs out.
        let mut budget = STREAM_BUDGET;
        'rings: for ring in 0..=STREAM_RADIUS {
            for dz in -ring..=ring {
                for dx in -ring..=ring {
                    // Only this ring's outer shell (Chebyshev distance == ring).
                    if dx.abs().max(dz.abs()) != ring {
                        continue;
                    }
                    let coord = (ccx + dx, 0, ccz + dz);
                    if self.loaded.contains(&coord) {
                        continue;
                    }
                    let (cx, _cy, cz) = coord;
                    // Generate this chunk and its 4 horizontal neighbours first, so
                    // seam culling sees solid neighbours (no faces at chunk borders).
                    ensure_chunk(&mut self.world, cx, cz);
                    ensure_chunk(&mut self.world, cx - 1, cz);
                    ensure_chunk(&mut self.world, cx + 1, cz);
                    ensure_chunk(&mut self.world, cx, cz - 1);
                    ensure_chunk(&mut self.world, cx, cz + 1);

                    let section = self.world.get(coord).expect("just generated");
                    let neighbors = Neighbors {
                        faces: [
                            self.world.get((cx - 1, 0, cz)),
                            self.world.get((cx + 1, 0, cz)),
                            None, // -y: open sky below the single chunk layer
                            None, // +y: open sky above
                            self.world.get((cx, 0, cz - 1)),
                            self.world.get((cx, 0, cz + 1)),
                        ],
                    };
                    let mesh = greedy_mesh_section_with(section, &neighbors);
                    let origin = Vec3::new(cx as f32 * s, 0.0, cz as f32 * s);
                    let inst = ChunkInstance {
                        coord,
                        origin,
                        mesh,
                    };
                    self.state.as_mut().unwrap().upload_chunk(&inst);
                    self.loaded.insert(coord);

                    budget -= 1;
                    if budget == 0 {
                        break 'rings;
                    }
                }
            }
        }

        // Log only on frames that actually changed the draw set (quiet once settled).
        if budget < STREAM_BUDGET {
            if let Some(state) = self.state.as_ref() {
                log::debug!("streaming: {} chunks resident", state.chunk_count());
            }
        }
    }
}

/// Generate a terrain section at `(cx, 0, cz)` into `world` if it isn't there yet.
fn ensure_chunk(world: &mut World, cx: i32, cz: i32) {
    if !world.contains((cx, 0, cz)) {
        world.insert((cx, 0, cz), worldgen::generate_section(cx, cz, WORLD_SEED));
    }
}

/// Debris emitters scattered on the terrain *ahead of* the camera along its
/// heading, so the flight sweeps over rising embers. Emitting at the camera itself
/// just leaves the debris behind at cruise speed (it spawns and the camera is gone).
fn lead_emitters(camera: &Camera) -> Vec<Vec3> {
    // Horizontal heading + right vector (the camera looks slightly down).
    let mut fwd = camera.forward();
    fwd.y = 0.0;
    let fwd = fwd.normalize_or_zero();
    let right = Vec3::new(-fwd.z, 0.0, fwd.x);
    // A loose band 24–48 units ahead, scattered side to side, on the surface.
    [(-1.0_f32, 0.0_f32), (0.6, 8.0), (-0.4, 16.0), (1.0, 24.0)]
        .iter()
        .map(|&(side, extra)| {
            let p = camera.position + fwd * (24.0 + extra) + right * (side * 11.0);
            let g =
                worldgen::height(p.x.floor() as i32, p.z.floor() as i32, WORLD_SEED) as f32 + 0.5;
            Vec3::new(p.x, g, p.z)
        })
        .collect()
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

        // The draw set starts empty; chunks stream in around the camera each frame.
        #[cfg(not(target_arch = "wasm32"))]
        {
            // Native: just block until the GPU is ready.
            self.state = Some(pollster::block_on(State::new(window, &[])));
        }

        #[cfg(target_arch = "wasm32")]
        {
            // Web: kick off async init and deliver the result via the proxy.
            let proxy = self.proxy.take().expect("proxy missing");
            wasm_bindgen_futures::spawn_local(async move {
                let state = State::new(window, &[]).await;
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
                    let pressed = key.state.is_pressed();
                    if code == KeyCode::Escape && pressed {
                        // Release the pointer. (The browser also does this on Esc.)
                        self.set_capture(false);
                    } else if code == KeyCode::KeyF && pressed {
                        self.auto_fly = !self.auto_fly; // toggle cinematic orbit
                    } else if let Some(action) = key_action(code) {
                        self.controller.set_action(action, pressed);
                        if pressed {
                            self.auto_fly = false; // manual movement takes the wheel
                        }
                    }
                }
            }
            // Click to capture the pointer for mouselook (and to re-capture after
            // Esc / tabbing away). Idempotent, so re-clicking is harmless.
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state: ElementState::Pressed,
                ..
            } => {
                self.auto_fly = false;
                self.set_capture(true);
            }
            // Pointer lock is lost when focus leaves; reflect that in our state.
            WindowEvent::Focused(false) => self.set_capture(false),
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = self
                    .last_frame
                    .map(|t| (now - t).as_secs_f32().min(0.1))
                    .unwrap_or(0.0);
                self.last_frame = Some(now);
                if self.auto_fly {
                    // Cinematic travel: cruise forward, banking gently, hugging the
                    // terrain — an endless flight that streams the world in around us.
                    self.auto_fly_angle += dt * AUTO_FLY_TURN;
                    let yaw = self.auto_fly_angle;
                    let dir = Vec3::new(yaw.cos(), 0.0, yaw.sin());
                    let mut pos = self.camera.position + dir * (AUTO_FLY_SPEED * dt);
                    let ground =
                        worldgen::height(pos.x.floor() as i32, pos.z.floor() as i32, WORLD_SEED)
                            as f32;
                    let target_y = ground + CRUISE_HEIGHT;
                    pos.y += (target_y - pos.y) * (dt * 1.2).min(1.0);
                    self.camera = Camera::new(pos, yaw, AUTO_FLY_PITCH);
                } else {
                    self.controller.update(&mut self.camera, dt);
                }
                // Stream chunks in/out around the (possibly moved) camera.
                self.stream();
                // Ambient debris bursts ahead of the camera so the flight sweeps
                // over rising embers (there's always motion in frame).
                self.particles.set_emitters(lead_emitters(&self.camera));
                self.particles.update(dt);
                let particles = self.particles.instances();

                // On the web, pull the latest dial values set by the page sliders.
                #[cfg(target_arch = "wasm32")]
                {
                    self.wobble = controls::wobble();
                    self.color_steps = controls::color_steps();
                }

                if let Some(state) = self.state.as_mut() {
                    let view_proj = self.camera.view_proj(state.aspect());
                    // `render` handles lost/outdated/transient surfaces internally.
                    state.render(
                        view_proj,
                        self.camera.position,
                        &particles,
                        [self.wobble, self.color_steps],
                    );
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

    // Start above the terrain at the origin, looking along +x; streaming fills the
    // world in around us on the first frames.
    let start_ground = worldgen::height(0, 0, WORLD_SEED) as f32;
    let start = Vec3::new(0.0, start_ground + CRUISE_HEIGHT, 0.0);
    let camera = Camera::new(start, 0.0, AUTO_FLY_PITCH);

    let mut app = App {
        state: None,
        proxy: Some(event_loop.create_proxy()),
        camera,
        controller: CameraController::new(45.0),
        particles: ParticleSystem::new(Vec::new()),
        world: World::new(),
        loaded: HashSet::new(),
        auto_fly: true,
        auto_fly_angle: 0.0,
        wobble: 85.0,
        color_steps: 4.0,
        last_frame: None,
        cursor_locked: false,
    };

    event_loop.run_app(&mut app).expect("event loop error");
}

/// Seed for the world (shared by terrain + emitters).
const WORLD_SEED: u32 = 1337;
/// Radius of the *demo* world (headless render only), in chunks — a `(2r+1)²` grid.
#[cfg(not(target_arch = "wasm32"))]
const WORLD_RADIUS: i32 = 2;

/// Streaming radius around the camera, in chunks (Chebyshev). The world is one
/// vertical layer of chunks (`cy == 0`) for M3.
const STREAM_RADIUS: i32 = 5;
/// Max chunks to generate + mesh per frame, to cap how long a hitch can get.
const STREAM_BUDGET: usize = 4;

/// How high above the terrain the cinematic camera cruises.
const CRUISE_HEIGHT: f32 = 22.0;
/// Auto-fly cruise speed (world units/second) and turn rate (radians/second).
const AUTO_FLY_SPEED: f32 = 26.0;
const AUTO_FLY_TURN: f32 = 0.05;
/// Downward tilt of the auto-fly camera (radians).
const AUTO_FLY_PITCH: f32 = -0.22;

/// The demo world: a procedurally-generated noise-terrain world (M3). Used by the
/// native headless renderer (a fixed scene); the live app streams instead.
#[cfg(not(target_arch = "wasm32"))]
fn demo_world() -> World {
    worldgen::generate_world(WORLD_RADIUS, WORLD_SEED)
}

/// A handful of emitter points sitting on the terrain surface, for ambient debris.
#[cfg(not(target_arch = "wasm32"))]
fn demo_emitters() -> Vec<Vec3> {
    [
        (-40, -40),
        (40, 30),
        (0, 0),
        (-20, 60),
        (70, 70),
        (30, -50),
        (60, 10),
        (-55, 35),
    ]
    .iter()
    .map(|&(x, z)| {
        Vec3::new(
            x as f32,
            worldgen::height(x, z, WORLD_SEED) as f32 + 0.5,
            z as f32,
        )
    })
    .collect()
}

/// Frame the camera on the whole scene from the combined world bounds. Returns the
/// camera and the scene radius (used for move speed). Used by the native headless
/// renderer to show a fixed framing of the demo world.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn frame_camera(instances: &[ChunkInstance]) -> (Camera, Vec3, f32) {
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    for inst in instances {
        min = min.min(Vec3::from(inst.mesh.aabb.min) + inst.origin);
        max = max.max(Vec3::from(inst.mesh.aabb.max) + inst.origin);
    }
    if instances.is_empty() {
        min = Vec3::ZERO;
        max = Vec3::splat(Section::SIZE as f32);
    }
    let center = (min + max) * 0.5;
    let radius = ((max - min).length() * 0.5).max(1.0);
    let eye = center + Vec3::new(1.0, 0.6, 1.3).normalize() * radius * 1.8;
    (Camera::looking_at(eye, center), center, radius)
}

/// Greedy-mesh each chunk with neighbour-aware seam culling. Meshes stay
/// chunk-local; the world `origin` travels alongside (the shader applies it).
/// App-level glue: it's allowed to touch `world` + `mesh` together. Used by the
/// native headless renderer (the live app meshes incrementally via streaming).
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn build_world_meshes(world: &World) -> Vec<ChunkInstance> {
    let s = Section::SIZE as f32;
    let mut instances = Vec::new();
    for ((cx, cy, cz), section) in world.chunks() {
        let neighbors = Neighbors {
            faces: [
                world.get((cx - 1, cy, cz)),
                world.get((cx + 1, cy, cz)),
                world.get((cx, cy - 1, cz)),
                world.get((cx, cy + 1, cz)),
                world.get((cx, cy, cz - 1)),
                world.get((cx, cy, cz + 1)),
            ],
        };
        let mesh = greedy_mesh_section_with(section, &neighbors);
        if mesh.is_empty() {
            continue;
        }
        let origin = Vec3::new(cx as f32 * s, cy as f32 * s, cz as f32 * s);
        instances.push(ChunkInstance {
            coord: (cx, cy, cz),
            origin,
            mesh,
        });
    }
    instances
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

/// Live aesthetic dials, settable from the web page's sliders (D2). Single-threaded
/// wasm, so a thread-local cell is enough; the render loop reads these each frame.
#[cfg(target_arch = "wasm32")]
pub mod controls {
    use std::cell::Cell;
    use wasm_bindgen::prelude::*;

    thread_local! {
        static WOBBLE: Cell<f32> = const { Cell::new(85.0) };
        static COLOR_STEPS: Cell<f32> = const { Cell::new(4.0) };
    }

    /// Vertex-wobble snap (lower = chunkier). Called from JS.
    #[wasm_bindgen]
    pub fn set_wobble(value: f32) {
        WOBBLE.with(|c| c.set(value));
    }

    /// Dither colour steps (lower = more posterised). Called from JS.
    #[wasm_bindgen]
    pub fn set_color_steps(value: f32) {
        COLOR_STEPS.with(|c| c.set(value));
    }

    pub(crate) fn wobble() -> f32 {
        WOBBLE.with(Cell::get)
    }

    pub(crate) fn color_steps() -> f32 {
        COLOR_STEPS.with(Cell::get)
    }
}
