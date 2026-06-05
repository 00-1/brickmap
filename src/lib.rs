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

pub mod foliage;
mod gfx;
#[cfg(not(target_arch = "wasm32"))]
pub mod headless;
pub mod mesh;
pub mod particles;
mod post;
pub mod scene;
pub mod share;
pub mod sim;
pub mod textures;
pub mod visibility;
pub mod world;
pub mod worldgen;
use gfx::{ChunkInstance, State, Toggles};
use mesh::{greedy_mesh_section_with, Neighbors};
use particles::ParticleSystem;
use scene::{Action, Camera, CameraController};
use visibility::connectivity;
use world::{ChunkCoord, Section};
// `World` is only used by the native headless renderer's fixed demo scene now;
// the live app streams via self-generating workers (M6).
#[cfg(not(target_arch = "wasm32"))]
use world::World;

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
    /// Off-thread chunk generation + meshing (M6); workers regenerate from the seed,
    /// so the app keeps no `World` for streaming.
    loader: ChunkLoader,
    /// Chunk coords currently uploaded to the GPU (the renderer's draw set).
    loaded: HashSet<ChunkCoord>,
    /// Falling-sand simulation (E5): an overlay of sim-modified sections (base terrain +
    /// sand) keyed by chunk, the set still in motion, and the tick/seed accumulators.
    /// Only ever covers a few loaded chunks near the camera; forgotten on eviction.
    overlay: std::collections::HashMap<ChunkCoord, Section>,
    sim_active: HashSet<ChunkCoord>,
    sim_timer: f32,
    sand_timer: f32,
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
    /// Smoothed frame time (ms) and a throttle accumulator for the perf HUD (M5).
    frame_ms_ema: f32,
    hud_timer: f32,
    /// Live feature on/off switches (D6).
    toggles: Toggles,
    /// The seed this world is generated from (E12). Drives streaming, sand, emitters,
    /// and the auto-fly ground query; changing it via `set_seed` regenerates the world.
    seed: u32,
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

    /// Switch to a new world seed (E12): tear down the streamed world + sand overlay,
    /// reset the loader, and drop the camera back onto the new ground. The world
    /// re-streams from the next frame. No-op if the seed is unchanged.
    fn set_seed(&mut self, seed: u32) {
        if seed == self.seed {
            return;
        }
        self.seed = seed;
        if let Some(state) = self.state.as_mut() {
            for &coord in self.loaded.iter() {
                state.remove_chunk(coord);
            }
        }
        self.loaded.clear();
        self.overlay.clear();
        self.sim_active.clear();
        self.loader.set_seed(seed);
        let ground = worldgen::height(
            self.camera.position.x.floor() as i32,
            self.camera.position.z.floor() as i32,
            seed,
        ) as f32;
        self.camera.position.y = ground + CRUISE_HEIGHT;
        log::info!("seed → {seed}");
    }

    /// Dev seed keys on native: `R` reseeds to a fresh random world, `P` prints the
    /// current share string. Returns whether the key was handled. No-op on web (the page
    /// has buttons for these).
    fn handle_seed_key(&mut self, code: KeyCode) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        match code {
            KeyCode::KeyR => {
                self.set_seed(random_seed());
                return true;
            }
            KeyCode::KeyP => {
                log::info!("share: #{}", self.current_share().encode());
                return true;
            }
            _ => {}
        }
        #[cfg(target_arch = "wasm32")]
        let _ = code;
        false
    }

    /// The current world + view as a `ShareState` (E12) — for "copy link" / `--share`.
    fn current_share(&self) -> share::ShareState {
        share::ShareState {
            seed: self.seed,
            pos: self.camera.position.into(),
            yaw: self.camera.yaw,
            pitch: self.camera.pitch,
            wobble: self.wobble,
            color_steps: self.color_steps,
            toggles: self.toggles.to_mask(),
        }
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
        let keep = STREAM_RADIUS + 1;
        let within = |coord: ChunkCoord| {
            (coord.0 - ccx).abs() <= keep && coord.1 == 0 && (coord.2 - ccz).abs() <= keep
        };

        // Evict GPU draws that have drifted beyond the radius (+1 hysteresis), and
        // forget any sand overlay there. Workers self-generate, so there's no CPU-side
        // base world to prune. Field-split borrows.
        let state = self.state.as_mut().unwrap();
        let overlay = &mut self.overlay;
        let sim_active = &mut self.sim_active;
        self.loaded.retain(|&coord| {
            if within(coord) {
                true
            } else {
                state.remove_chunk(coord);
                overlay.remove(&coord);
                sim_active.remove(&coord);
                false
            }
        });

        // Request nearest-missing chunks (cheap — dispatches a mesh job), ring by ring,
        // up to a per-frame request budget. Not loaded and not already in flight.
        let mut requests = STREAM_REQUESTS;
        'rings: for ring in 0..=STREAM_RADIUS {
            for dz in -ring..=ring {
                for dx in -ring..=ring {
                    if dx.abs().max(dz.abs()) != ring {
                        continue; // outer shell of this ring only
                    }
                    let coord = (ccx + dx, 0, ccz + dz);
                    if self.loaded.contains(&coord) || self.loader.is_pending(coord) {
                        continue;
                    }
                    self.loader.request(coord);
                    requests -= 1;
                    if requests == 0 {
                        break 'rings;
                    }
                }
            }
        }

        // Drain finished meshes and upload them (bounded GPU work per frame). On native
        // the meshing happened off-thread; on web `drain` does it inline, time-sliced.
        for inst in self.loader.drain(STREAM_UPLOADS) {
            if !within(inst.coord) {
                continue; // camera moved on while it meshed — drop it
            }
            self.state.as_mut().unwrap().upload_chunk(&inst);
            self.loaded.insert(inst.coord);
        }
    }

    /// Falling-sand simulation step (E5). Seeds sand ahead of the camera (into loaded
    /// chunks only — so it never races the streaming loader), steps active overlay
    /// sections on a fixed tick, and re-meshes the few that changed. Re-mesh is
    /// synchronous: the sim is localized, so it's a small, occasional cost.
    fn sim(&mut self, dt: f32) {
        if !self.toggles.sand || self.state.is_none() {
            return;
        }
        self.sand_timer += dt;
        while self.sand_timer >= SAND_INTERVAL {
            self.sand_timer -= SAND_INTERVAL;
            self.seed_sand();
        }

        self.sim_timer += dt.min(0.1);
        let mut dirty: HashSet<ChunkCoord> = HashSet::new();
        while self.sim_timer >= SIM_TICK {
            self.sim_timer -= SIM_TICK;
            let active: Vec<ChunkCoord> = self.sim_active.iter().copied().collect();
            for coord in active {
                let moved = self.overlay.get_mut(&coord).map(sim::step_sand);
                match moved {
                    Some(true) => {
                        dirty.insert(coord);
                    }
                    _ => {
                        self.sim_active.remove(&coord); // settled or gone
                    }
                }
            }
        }

        // Re-mesh changed overlay chunks (synchronous — localized), bounded per frame
        // so a wide sandfall can't spike the frame; any extra settle next frame.
        let mut budget = SAND_REMESH_BUDGET;
        for coord in dirty {
            if budget == 0 {
                break;
            }
            if !self.loaded.contains(&coord) {
                continue;
            }
            if let Some(sec) = self.overlay.get(&coord) {
                let inst = mesh_chunk(coord, sec, self.seed);
                if let Some(state) = self.state.as_mut() {
                    state.upload_chunk(&inst);
                    budget -= 1;
                }
            }
        }
    }

    /// Seed a clump of sand high in a loaded chunk ahead of the camera; it falls onto
    /// the terrain. Spread into a *curtain* across the forward path (pseudo-random from
    /// the camera position) and placed far enough ahead that grains finish falling while
    /// we approach — so the fall is watchable from the cruise instead of whooshing past.
    fn seed_sand(&mut self) {
        let mut fwd = self.camera.forward();
        fwd.y = 0.0;
        let fwd = fwd.normalize_or_zero();
        let right = Vec3::new(-fwd.z, 0.0, fwd.x);
        let t = self.camera.position.x * 0.7 + self.camera.position.z * 0.9;
        let lateral = t.sin() * 16.0;
        let ahead = 42.0 + (t * 1.7).cos() * 6.0;
        let p = self.camera.position + fwd * ahead + right * lateral;
        let sz = Section::SIZE as f32;
        let coord = ((p.x / sz).floor() as i32, 0, (p.z / sz).floor() as i32);
        if !self.loaded.contains(&coord) {
            return;
        }
        let lx = p.x.rem_euclid(sz) as i32;
        let lz = p.z.rem_euclid(sz) as i32;
        let y = Section::SIZE - 2;
        let n = Section::SIZE as i32;
        let sec = self
            .overlay
            .entry(coord)
            .or_insert_with(|| worldgen::generate_section(coord.0, coord.2, self.seed));
        let mut added = false;
        for (dx, dz) in [(0i32, 0i32), (1, 0), (0, 1), (1, 1)] {
            let (x, z) = (lx + dx, lz + dz);
            if (0..n).contains(&x) && (0..n).contains(&z) && sec.get(x as u32, y, z as u32).is_air()
            {
                sec.set(x as u32, y, z as u32, sim::SAND);
                added = true;
            }
        }
        if added {
            self.sim_active.insert(coord);
        }
    }
}

/// Greedy-mesh a chunk's `center` section against its 4 (regenerated) procedural
/// neighbours for seam culling, and bake its connectivity graph. Shared by the
/// streaming worker and the sand re-mesh.
fn mesh_chunk(coord: ChunkCoord, center: &Section, seed: u32) -> ChunkInstance {
    let (cx, _cy, cz) = coord;
    let west = worldgen::generate_section(cx - 1, cz, seed);
    let east = worldgen::generate_section(cx + 1, cz, seed);
    let south = worldgen::generate_section(cx, cz - 1, seed);
    let north = worldgen::generate_section(cx, cz + 1, seed);
    let neighbors = Neighbors {
        faces: [
            Some(&west),
            Some(&east),
            None, // -y: open sky below the single chunk layer
            None, // +y: open sky above
            Some(&south),
            Some(&north),
        ],
    };
    let mesh = greedy_mesh_section_with(center, &neighbors);
    let s = Section::SIZE as f32;
    ChunkInstance {
        coord,
        origin: Vec3::new(cx as f32 * s, 0.0, cz as f32 * s),
        mesh,
        graph: connectivity(center),
    }
}

/// The pure off-thread streaming worker: regenerate a chunk from the seed and mesh it.
/// Self-contained and `Send`, so it can run on a rayon thread (M6).
fn build_chunk_instance(coord: ChunkCoord, seed: u32) -> ChunkInstance {
    let center = worldgen::generate_section(coord.0, coord.2, seed);
    mesh_chunk(coord, &center, seed)
}

/// Off-thread chunk loader (M6). Native dispatches mesh jobs to rayon and collects
/// finished `ChunkInstance`s over a channel; web meshes inline, time-sliced. Same
/// interface either way: `request` to enqueue, `drain` to collect.
struct ChunkLoader {
    /// The seed the world is currently generated from (E12 runtime seed).
    seed: u32,
    /// Bumped on reseed; in-flight jobs carry the epoch they were dispatched under so
    /// stale (old-seed) results can be discarded as they trickle back.
    epoch: u64,
    /// Coords currently in flight (so we don't request them twice).
    pending: HashSet<ChunkCoord>,
    #[cfg(not(target_arch = "wasm32"))]
    tx: std::sync::mpsc::Sender<(u64, ChunkInstance)>,
    #[cfg(not(target_arch = "wasm32"))]
    rx: std::sync::mpsc::Receiver<(u64, ChunkInstance)>,
    #[cfg(target_arch = "wasm32")]
    queue: std::collections::VecDeque<ChunkCoord>,
}

impl ChunkLoader {
    fn new(seed: u32) -> ChunkLoader {
        #[cfg(not(target_arch = "wasm32"))]
        let (tx, rx) = std::sync::mpsc::channel();
        ChunkLoader {
            seed,
            epoch: 0,
            pending: HashSet::new(),
            #[cfg(not(target_arch = "wasm32"))]
            tx,
            #[cfg(not(target_arch = "wasm32"))]
            rx,
            #[cfg(target_arch = "wasm32")]
            queue: std::collections::VecDeque::new(),
        }
    }

    fn is_pending(&self, coord: ChunkCoord) -> bool {
        self.pending.contains(&coord)
    }

    fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Switch the world to a new seed: drop all in-flight work (results still arriving
    /// under the old epoch are discarded in `drain`) and start fresh.
    fn set_seed(&mut self, seed: u32) {
        self.seed = seed;
        self.epoch += 1;
        self.pending.clear();
        #[cfg(target_arch = "wasm32")]
        self.queue.clear();
    }

    /// Queue a chunk to be meshed. Native: dispatch a rayon job. Web: enqueue.
    fn request(&mut self, coord: ChunkCoord) {
        if !self.pending.insert(coord) {
            return;
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let tx = self.tx.clone();
            let (seed, epoch) = (self.seed, self.epoch);
            rayon::spawn(move || {
                let _ = tx.send((epoch, build_chunk_instance(coord, seed)));
            });
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.queue.push_back(coord);
        }
    }

    /// Collect up to `budget` finished meshes. Native: non-blocking `try_recv` (stale-
    /// epoch results are dropped). Web: mesh up to `budget` queued chunks inline (this is
    /// where the web cost lives).
    fn drain(&mut self, budget: usize) -> Vec<ChunkInstance> {
        let mut out = Vec::new();
        #[cfg(not(target_arch = "wasm32"))]
        while out.len() < budget {
            match self.rx.try_recv() {
                Ok((epoch, inst)) => {
                    if epoch != self.epoch {
                        continue; // result from a superseded seed — discard
                    }
                    self.pending.remove(&inst.coord);
                    out.push(inst);
                }
                Err(_) => break,
            }
        }
        #[cfg(target_arch = "wasm32")]
        for _ in 0..budget {
            match self.queue.pop_front() {
                Some(coord) => {
                    self.pending.remove(&coord);
                    out.push(build_chunk_instance(coord, self.seed));
                }
                None => break,
            }
        }
        out
    }
}

/// Debris emitters scattered on the terrain *ahead of* the camera along its
/// heading, so the flight sweeps over rising embers. Emitting at the camera itself
/// just leaves the debris behind at cruise speed (it spawns and the camera is gone).
fn lead_emitters(camera: &Camera, seed: u32) -> Vec<Vec3> {
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
            let g = worldgen::height(p.x.floor() as i32, p.z.floor() as i32, seed) as f32 + 0.5;
            Vec3::new(p.x, g, p.z)
        })
        .collect()
}

/// Map a number key `1..9` to a feature-toggle index (D6 debug switches).
fn toggle_index(code: KeyCode) -> Option<usize> {
    Some(match code {
        KeyCode::Digit1 => 0,
        KeyCode::Digit2 => 1,
        KeyCode::Digit3 => 2,
        KeyCode::Digit4 => 3,
        KeyCode::Digit5 => 4,
        KeyCode::Digit6 => 5,
        KeyCode::Digit7 => 6,
        KeyCode::Digit8 => 7,
        KeyCode::Digit9 => 8,
        KeyCode::Digit0 => 9,
        _ => return None,
    })
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
                    } else if pressed && self.handle_seed_key(code) {
                        // handled: R reseed / P print share (native)
                    } else if let Some(i) = toggle_index(code) {
                        // Number keys flip renderer features (D6); web uses checkboxes.
                        if pressed {
                            self.toggles.toggle(i);
                            log::info!(
                                "toggle {} = {}",
                                gfx::TOGGLE_LABELS[i],
                                self.toggles.get(i)
                            );
                        }
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
                        worldgen::height(pos.x.floor() as i32, pos.z.floor() as i32, self.seed)
                            as f32;
                    let target_y = ground + CRUISE_HEIGHT;
                    pos.y += (target_y - pos.y) * (dt * 1.2).min(1.0);
                    self.camera = Camera::new(pos, yaw, AUTO_FLY_PITCH);
                } else {
                    self.controller.update(&mut self.camera, dt);
                }
                // Stream chunks in/out around the (possibly moved) camera.
                self.stream();
                // Falling-sand simulation (E5): seed, step, re-mesh dirty overlay chunks.
                self.sim(dt);
                // Ambient debris bursts ahead of the camera so the flight sweeps
                // over rising embers (there's always motion in frame).
                self.particles
                    .set_emitters(lead_emitters(&self.camera, self.seed));
                self.particles.update(dt);
                let particles = self.particles.instances();

                // On the web, pull the latest dial values set by the page sliders, plus
                // any seed change requested from the page (E12).
                #[cfg(target_arch = "wasm32")]
                {
                    self.wobble = controls::wobble();
                    self.color_steps = controls::color_steps();
                    self.toggles = controls::toggles();
                    if let Some(seed) = controls::take_pending_seed() {
                        self.set_seed(seed);
                    }
                }
                // The current share string for "copy link" (computed before the mutable
                // `state` borrow below; cheap, refreshed every frame).
                #[cfg(target_arch = "wasm32")]
                let share_str = self.current_share().encode();

                if let Some(state) = self.state.as_mut() {
                    let view_proj = self.camera.view_proj(state.aspect());
                    // `render` handles lost/outdated/transient surfaces internally.
                    state.render(
                        view_proj,
                        self.camera.position,
                        &particles,
                        [self.wobble, self.color_steps],
                        self.toggles,
                    );

                    // Perf HUD (M5): smooth the frame time, refresh a few times/sec.
                    let ms = dt * 1000.0;
                    self.frame_ms_ema = if self.frame_ms_ema == 0.0 {
                        ms
                    } else {
                        self.frame_ms_ema * 0.9 + ms * 0.1
                    };
                    self.hud_timer += dt;
                    if self.hud_timer >= 0.2 {
                        self.hud_timer = 0.0;
                        let s = state.stats();
                        let fps = if self.frame_ms_ema > 0.0 {
                            1000.0 / self.frame_ms_ema
                        } else {
                            0.0
                        };
                        let meshing = self.loader.pending_count();
                        let meshing = if meshing > 0 {
                            format!(" · meshing {meshing}")
                        } else {
                            String::new()
                        };
                        let hud = format!(
                            "{fps:.0} fps · {:.1} ms · seed {} · {}/{} chunks · {} tris · {} fx{meshing}{}",
                            self.frame_ms_ema,
                            self.seed,
                            s.drawn_chunks,
                            s.total_chunks,
                            s.triangles,
                            s.particles,
                            self.toggles.off_summary(),
                        );
                        #[cfg(not(target_arch = "wasm32"))]
                        state.window().set_title(&format!("brickmap — {hud}"));
                        #[cfg(target_arch = "wasm32")]
                        {
                            set_hud(&hud);
                            // Keep the page's "copy link" payload fresh.
                            controls::set_current_share(&share_str);
                        }
                    }

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

    // The default world/view: start above the terrain at the origin, looking along +x;
    // streaming fills the world in around us on the first frames.
    let default_ground = worldgen::height(0, 0, WORLD_SEED) as f32;
    let default_view = share::ShareState {
        seed: WORLD_SEED,
        pos: [0.0, default_ground + CRUISE_HEIGHT, 0.0],
        yaw: 0.0,
        pitch: AUTO_FLY_PITCH,
        wobble: 85.0,
        color_steps: 4.0,
        toggles: Toggles::default().to_mask(),
    };
    // Override from a shared link / CLI seed if present (E12).
    let view = initial_view(default_view);

    // If a seed was chosen but no explicit camera position came with it, drop the
    // camera onto the new seed's ground so we don't start buried or floating.
    let mut pos = Vec3::from(view.pos);
    if view.seed != default_view.seed && view.pos == default_view.pos {
        pos.y = worldgen::height(pos.x.floor() as i32, pos.z.floor() as i32, view.seed) as f32
            + CRUISE_HEIGHT;
    }
    let camera = Camera::new(pos, view.yaw, view.pitch);

    // On the web, seed the JS-facing control cells so the page reflects the restored
    // state (and the per-frame read-back doesn't clobber it).
    #[cfg(target_arch = "wasm32")]
    controls::init_from(&view);

    let mut app = App {
        state: None,
        proxy: Some(event_loop.create_proxy()),
        camera,
        controller: CameraController::new(45.0),
        particles: ParticleSystem::new(Vec::new()),
        loader: ChunkLoader::new(view.seed),
        loaded: HashSet::new(),
        overlay: std::collections::HashMap::new(),
        sim_active: HashSet::new(),
        sim_timer: 0.0,
        sand_timer: 0.0,
        auto_fly: true,
        auto_fly_angle: 0.0,
        wobble: view.wobble,
        color_steps: view.color_steps,
        last_frame: None,
        cursor_locked: false,
        frame_ms_ema: 0.0,
        hud_timer: 0.0,
        toggles: Toggles::from_mask(view.toggles),
        seed: view.seed,
    };

    event_loop.run_app(&mut app).expect("event loop error");
}

/// Resolve the initial world/view, overriding `default` from a shared link or CLI seed.
/// Native: `--share <blob>`, `--seed <int|text>`, `--daily`. Web: the URL hash fragment.
#[cfg(not(target_arch = "wasm32"))]
fn initial_view(default: share::ShareState) -> share::ShareState {
    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;
    let mut view = default;
    while i < args.len() {
        match args[i].as_str() {
            "--share" if i + 1 < args.len() => {
                if let Some(v) = share::ShareState::decode(&args[i + 1], view) {
                    view = v;
                }
                i += 2;
            }
            "--seed" if i + 1 < args.len() => {
                if let Some(s) = share::seed_from_text(&args[i + 1]) {
                    view.seed = s;
                }
                i += 2;
            }
            "--daily" => {
                let secs = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);
                view.seed = share::seed_of_the_day(&share::date_utc_from_unix_secs(secs));
                i += 1;
            }
            _ => i += 1,
        }
    }
    view
}

/// Web: restore the view from the URL hash fragment (`#s=…&x=…&…`) if present.
#[cfg(target_arch = "wasm32")]
fn initial_view(default: share::ShareState) -> share::ShareState {
    web_sys::window()
        .and_then(|w| w.location().hash().ok())
        .and_then(|h| share::ShareState::decode(&h, default))
        .unwrap_or(default)
}

/// Seed for the world (the default + the headless demo).
const WORLD_SEED: u32 = 1337;

/// A fresh pseudo-random seed from the wall clock (native `R` key). splitmix64-style
/// mix so nearby nanos still produce well-spread seeds.
#[cfg(not(target_arch = "wasm32"))]
fn random_seed() -> u32 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let mut z = nanos.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    (z ^ (z >> 31)) as u32
}
/// Radius of the *demo* world (headless render only), in chunks — a `(2r+1)²` grid.
#[cfg(not(target_arch = "wasm32"))]
const WORLD_RADIUS: i32 = 2;

/// Streaming radius around the camera, in chunks (Chebyshev). The world is one
/// vertical layer of chunks (`cy == 0`) for M3.
const STREAM_RADIUS: i32 = 5;
/// Mesh jobs to dispatch per frame (cheap on native — just hands work to rayon). M6.
const STREAM_REQUESTS: usize = 8;
/// Finished meshes to upload to the GPU per frame (bounds main-thread upload work, and
/// on web bounds inline meshing). M6.
const STREAM_UPLOADS: usize = 4;

/// Falling-sand tick (seconds) and how often a clump is dropped (E5).
const SIM_TICK: f32 = 0.05;
const SAND_INTERVAL: f32 = 0.14;
/// Max sand sections re-meshed per frame (synchronous; bounds the frame cost of a wide
/// sandfall — leftovers settle next frame).
const SAND_REMESH_BUDGET: usize = 3;

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
            graph: connectivity(section),
        });
    }
    instances
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn wasm_start() {
    run();
}

/// Push the perf-HUD text into the page's overlay element (web only).
#[cfg(target_arch = "wasm32")]
fn set_hud(text: &str) {
    if let Some(el) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id("hud"))
    {
        el.set_text_content(Some(text));
    }
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
        /// Feature-toggle bitmask, one bit per switch; all 11 on by default.
        static TOGGLES: Cell<u32> = const { Cell::new(0x7FF) };
        /// A seed change requested from the page, consumed by the app next frame.
        static PENDING_SEED: Cell<Option<u32>> = const { Cell::new(None) };
        /// The app's current share string, refreshed each HUD tick for "copy link".
        static CURRENT_SHARE: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
    }

    /// Seed the control cells from a restored view on startup (so the page reflects it).
    pub(crate) fn init_from(view: &crate::share::ShareState) {
        WOBBLE.with(|c| c.set(view.wobble));
        COLOR_STEPS.with(|c| c.set(view.color_steps));
        TOGGLES.with(|c| c.set(view.toggles));
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

    /// Switch to a seed parsed from user text (numeric or folded). Returns the resolved
    /// `u32` seed so the page can display it; empty text is ignored (returns the current).
    #[wasm_bindgen]
    pub fn set_seed_text(text: &str) -> u32 {
        match crate::share::seed_from_text(text) {
            Some(seed) => {
                PENDING_SEED.with(|c| c.set(Some(seed)));
                seed
            }
            None => 0,
        }
    }

    /// Switch to an explicit numeric seed (e.g. the 🎲 random button). Returns it back.
    #[wasm_bindgen]
    pub fn set_seed(seed: u32) -> u32 {
        PENDING_SEED.with(|c| c.set(Some(seed)));
        seed
    }

    /// The deterministic seed for a `YYYY-MM-DD` date (seed-of-the-day). Pure; the page
    /// passes its local UTC date and then calls `set_seed` with the result.
    #[wasm_bindgen]
    pub fn seed_of_the_day(date: &str) -> u32 {
        crate::share::seed_of_the_day(date)
    }

    /// The current share string (`s=…&x=…&…`) for building a copy-link URL.
    #[wasm_bindgen]
    pub fn current_share() -> String {
        CURRENT_SHARE.with(|c| c.borrow().clone())
    }

    pub(crate) fn set_current_share(s: &str) {
        CURRENT_SHARE.with(|c| *c.borrow_mut() = s.to_string());
    }

    pub(crate) fn take_pending_seed() -> Option<u32> {
        PENDING_SEED.with(|c| c.take())
    }

    pub(crate) fn wobble() -> f32 {
        WOBBLE.with(Cell::get)
    }

    pub(crate) fn color_steps() -> f32 {
        COLOR_STEPS.with(Cell::get)
    }

    /// Flip a renderer feature on/off by index (D6). Called from the page checkboxes.
    #[wasm_bindgen]
    pub fn set_toggle(index: u32, on: bool) {
        TOGGLES.with(|c| {
            let mut m = c.get();
            if on {
                m |= 1 << index;
            } else {
                m &= !(1 << index);
            }
            c.set(m);
        });
    }

    pub(crate) fn toggles() -> crate::gfx::Toggles {
        let m = TOGGLES.with(Cell::get);
        let mut t = crate::gfx::Toggles::default();
        for i in 0..crate::gfx::TOGGLE_LABELS.len() {
            t.set(i, m & (1 << i) != 0);
        }
        t
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn loader_meshes_a_chunk_off_thread() {
        // The native loader dispatches to rayon; the mesh comes back via the channel.
        let mut loader = ChunkLoader::new(WORLD_SEED);
        loader.request((0, 0, 0));
        assert!(loader.is_pending((0, 0, 0)));

        let mut done = Vec::new();
        for _ in 0..2000 {
            done.extend(loader.drain(8));
            if !done.is_empty() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert_eq!(done.len(), 1, "the off-thread mesh should have come back");
        assert_eq!(done[0].coord, (0, 0, 0));
        assert!(!done[0].mesh.is_empty(), "origin chunk should have terrain");
        assert!(!loader.is_pending((0, 0, 0)), "pending cleared after drain");
    }

    #[test]
    fn build_chunk_instance_is_deterministic() {
        let a = build_chunk_instance((2, 0, -1), 1234);
        let b = build_chunk_instance((2, 0, -1), 1234);
        assert_eq!(a.coord, (2, 0, -1));
        assert_eq!(a.mesh.vertices.len(), b.mesh.vertices.len());
        assert_eq!(a.mesh.indices, b.mesh.indices);
        assert!(!a.mesh.vertices.is_empty());
    }
}
