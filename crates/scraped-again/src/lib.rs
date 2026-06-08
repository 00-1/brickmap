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

// Scraped Again — the game: a lonely surveyor crossing a dead world of fallen giants.
// It owns the app/event-loop + mode machine, the world content (terrain recipe, biomes,
// colossi, inscriptions), the doom drone, and the player; it consumes the brickmap engine.
//
// The brickmap engine surface, re-exported under the original short module paths so the
// app + content modules keep resolving `crate::world::…`, `crate::gfx::…`, etc. (the game
// consumes the engine purely through the `brickmap` facade — never the bm-* crates).
pub use brickmap::{
    edit, foliage, gamepad, gfx, hud, map, mesh, noise, overlay, palette, particles, post, scene,
    ship, sim, text, textures, visibility, world, WorldGen,
};

pub mod audio;
pub mod biome;
pub mod creatures;
// The cruiser's geometry (the engine renderer is generic — the ship is the game's).
pub mod cruiser;
// Gameplay (Scraped Again): data strata, the codex, collect events (G1).
pub mod progress;
// The survey-beam (G2): cast → collect-along-path → persist/fade, drawn post-palette.
pub mod beam;
// Cruiser auto-scan (G3): forward-cone sensing → the map opportunity surface.
pub mod scan;
// The operations console (G4): the game's actions as clickable blocks + given routines.
pub mod console;
// The automated expedition (G8c): the cross-agent deploy→harvest→return phase machine.
pub mod expedition;
// Global weather (E9): a seeded Clear→Building→Precip→Clearing cycle driving precipitation.
pub mod weather;
// Touch controls (D9): the phone overlay layout + the pure touch→action mapping.
pub mod touch;
// Decipherment lexicon (G6): a seeded grammar that renders a comprehended script as words.
pub mod lexicon;
// The curated colour ramps (one per biome) — art-direction the engine doesn't carry.
pub mod palettes;
pub mod player;
pub mod relic;
// The terrain recipe — this game's specific world, composed from the engine's noise.
pub mod worldgen;
// cpal audio output (E16): desktop + **Android** (AAudio backend); web uses Web Audio.
#[cfg(not(target_arch = "wasm32"))]
pub mod audio_native;
#[cfg(not(target_arch = "wasm32"))]
pub mod headless;
pub mod model;
pub mod share;
pub mod structures;
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

/// One cached colossal relic (E18): its renderable form. Ethereal relics fill `points` (and
/// leave `meshes` empty); solid relics fill `meshes` (the shader dissolves them to dots with
/// distance). Generated once on cell-entry, kept until the cell leaves range.
struct CachedRelic {
    points: Vec<foliage::SplatInstance>,
    meshes: Vec<ChunkInstance>,
}

/// Movement mode (E19): in the cruiser (flying — autopilot or manual) or on foot (walking).
#[derive(Copy, Clone, PartialEq, Eq)]
enum Mode {
    Pilot,
    Walk,
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
    /// E11: water-seed accumulator (water seeds slower than sand — it pools).
    water_timer: f32,
    /// M8a: dynamic-resolution extra internal-res divisor (added to the art `pixel_scale`). 0 on
    /// capable hardware; rises under frame-time pressure so weak GPUs hold their rate.
    dyn_extra: u32,
    /// E9: the global weather cycle (drives precipitation; advanced only in the live loop, so the
    /// headless/golden render stays dry).
    weather: weather::Weather,
    /// E9: per-frame counter scattering precip-drop spawn positions (cosmetic).
    precip_seq: u32,
    /// Cinematic auto-fly / **autopilot**: on by default so the build is watchable with no input
    /// (mobile / hands-off). Manual input switches it off; `F` / pad A toggles it. In `Walk` mode
    /// it's ignored (you're on foot).
    auto_fly: bool,
    auto_fly_angle: f32,
    /// Wander clock for the autopilot heading (so it meanders to new terrain, not in a circle).
    auto_fly_t: f32,
    /// G7: this frame's autopilot steering, from the routine interpreter (a continuous nav block).
    nav_intent: Option<console::Block>,
    /// G7: a continuous routine asked to scan this frame (the app throttles it to `scan::INTERVAL`).
    scan_wanted: bool,
    /// G8a: the **autonomous ship**'s own heading clock + heading + scan cadence, used while you're
    /// on foot and the cruiser flies its routine independently (separate from the camera's).
    ship_t: f32,
    ship_angle: f32,
    ship_scan_timer: f32,
    /// E13: cinematic **photo mode** — pause the world + free-cam + FOV zoom. `saved` holds the
    /// camera (with its FOV) to restore on exit.
    photo_active: bool,
    photo_saved: Option<Camera>,
    /// G8c: the **automated expedition** — a `run(foot)` ship step deploys the walker to collect.
    expedition: expedition::Expedition,
    /// The deployed walker's world position + the site it's collecting (while an expedition runs).
    walker_pos: Vec3,
    expedition_target: Option<Vec3>,
    /// Movement mode (E19): piloting the cruiser (fly — autopilot or manual) vs walking on foot.
    mode: Mode,
    /// The cruiser's world position: tracks the camera while piloting; where it's parked once you
    /// exit on foot. You walk back to it to re-enter.
    cruiser_pos: Vec3,
    /// Walking physics state (gravity), used in `Walk` mode.
    walker: player::Walker,
    /// Live aesthetic dials (D2): `[wobble snap, colour steps]`. On the web these are
    /// driven by `controls`; on native they stay at the defaults.
    wobble: f32,
    color_steps: f32,
    /// Palette post-process (E10): selection pushed to the renderer each frame. `on` gates
    /// the whole pass; `index` picks a `palettes::PALETTES` entry; `count` is how many of its
    /// colours to use; `dither` is the ordered-dither spread. Driven by native keys or, on
    /// the web, the page controls.
    palette_on: bool,
    palette_index: usize,
    palette_count: u32,
    palette_dither: f32,
    /// Internal-resolution divisor (E10): 1 = native, higher = chunkier "fat pixels" + cheaper.
    pixel_scale: u32,
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
    /// Undo log of inverse edits (E14): each editing action pushes the edit that reverts
    /// it. The forward edits are the future broadcast/share payload (N1 groundwork).
    undo: Vec<edit::Edit>,
    /// Colossal structures (E18): per-cell cache of the in-range giants' geometry, keyed by
    /// structure cell. Lets us generate a giant only when its cell enters range (budgeted,
    /// ≤1/frame) instead of regenerating everything on every cell crossing (the streaming-hitch
    /// fix), and pick its LOD (mesh near ↔ points far) per frame from the cached pair.
    structures: std::collections::HashMap<(i32, i32), CachedRelic>,
    /// E18: the baked CC0 human surface points (model-space), decoded once from the embedded
    /// asset; transformed per placement into a fallen-giant point cloud (`human` colossi).
    human_points: Vec<Vec3>,
    /// Drifting wisp creatures (E15): a small seed-driven swarm of point-cloud motes kept
    /// loosely tethered to the camera, advanced + re-uploaded every frame so they drift and
    /// shimmer through the fly-through. Cheap (≈ a few hundred splats).
    creatures: creatures::Swarm,
    /// In-world text (E17): the set of inscription cells currently in range. Lets the app
    /// rebuild the giants' label textures only when the in-range set changes (not every frame).
    text_cells: std::collections::HashSet<(i32, i32, u8)>,
    /// Previous-frame camera position, for the reactive-audio (E16) flight-speed estimate.
    audio_prev_pos: Option<Vec3>,
    /// Scraped Again gameplay (G1): banked data strata + the codex of collected inscriptions.
    progress: progress::Progress,
    /// Nearby still-collectible inscriptions (rebuilt as text streams in) — the targets the
    /// collect pick (`T`) and the survey-beam aim at.
    collectible: Vec<progress::Collectible>,
    /// Whether the codex list overlay is open (key `J`).
    codex_open: bool,
    /// The operations console (G4): blocks + given routines; open with `O`.
    console: console::Console,
    /// The active survey-beam (G2), if one is cast — persists then fades.
    beam: Option<beam::Beam>,
    /// When attached to the beam as a rail (Walk mode): the parametric ride position `t∈[0,1]`.
    ride_t: Option<f32>,
    /// Cruiser auto-scan (G3): seconds since the last scan pulse, and the active cool flicks.
    scan_timer: f32,
    flicks: Vec<scan::Flick>,
    /// Chunks holding a **scanned-but-uncollected** site — the map opportunity surface (G3).
    map_scanned: std::collections::HashSet<(i32, i32)>,
    /// Monotonic seconds, accumulated each frame — the beam's clock (birth + fade).
    time: f32,
    /// Biome-driven auto mode (the new default): when on, the biome at the camera drives the
    /// palette, spawn densities, lighting/wobble, ground, and drone mix — blended smoothly as you
    /// move, no manual toggles. Off → the manual settings ("as we currently have it"). Key `G`.
    biome_mode: bool,
    /// The current biome label, refreshed each frame in biome mode (shown on the HUD).
    biome_label: String,
    /// Explored-world map (E10): biome colour per visited chunk `(cx, cz)`, built as you fly.
    map: std::collections::HashMap<(i32, i32), [u8; 3]>,
    /// Chunks where an inscription has been encountered — shown as bright markers on the map.
    map_text: std::collections::HashSet<(i32, i32)>,
    /// Chunks inside a rare pristine/ethereal pocket — marked with their own icon on the map.
    map_pristine: std::collections::HashSet<(i32, i32)>,
    /// Whether the fullscreen map view is open; the pan centre (chunk coords); whether the
    /// explored set grew since the GPU image was last built; and the cached image origin/dims.
    map_view: bool,
    map_pan: (f32, f32),
    map_dirty: bool,
    map_origin: (i32, i32),
    map_dims: (u32, u32),
    map_anim: f32,
    /// Gamepad/controller input (D7). Polled each frame; feeds analog move + look.
    pad: gamepad::Pad,
    /// Touch controls (D9): active (held) touches by finger id → normalised pos, the on-screen
    /// layout, and whether any touch has been seen (so the overlay only shows on touch devices).
    touches: std::collections::HashMap<u64, (f32, f32)>,
    touch_layout: touch::Layout,
    touch_seen: bool,
    /// D10: the most-recently-pressed button + the `self.time` it was pressed, for a brief press
    /// highlight in the on-screen overlay.
    touch_pressed: Option<(touch::Region, f32)>,
    /// cpal doom-drone output (E16). `None` if no audio device. Desktop + Android.
    #[cfg(not(target_arch = "wasm32"))]
    audio: Option<audio_native::AudioEngine>,
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
        self.structures.clear(); // force the new seed's colossi to rebuild next frame
        self.text_cells.clear(); // and the new seed's inscriptions
        self.map.clear(); // the explored map is per-world
        self.map_text.clear();
        self.map_pristine.clear();
        self.map_scanned.clear(); // G3 opportunity surface is per-world
        self.map_dirty = true;
        self.loader.set_seed(seed);
        let ground = worldgen::height(
            self.camera.position.x.floor() as i32,
            self.camera.position.z.floor() as i32,
            seed,
        ) as f32;
        self.camera.position.y = ground + CRUISE_HEIGHT;
        log::info!("seed → {seed}");
    }

    /// Cycle the palette post-process (E10): off → palette 0 → … → last → off. When turning a
    /// palette on, reset the colour count to that palette's full length.
    #[cfg(not(target_arch = "wasm32"))]
    fn cycle_palette(&mut self) {
        let n = palettes::PALETTES.len();
        if !self.palette_on {
            self.palette_on = true;
            self.palette_index = 0;
        } else if self.palette_index + 1 < n {
            self.palette_index += 1;
        } else {
            self.palette_on = false;
            self.palette_index = 0;
        }
        self.palette_count = palettes::PALETTES[self.palette_index].colors.len() as u32;
        let name = palettes::PALETTES[self.palette_index].name;
        if self.palette_on {
            log::info!("palette → {name} ({} colours)", self.palette_count);
        } else {
            log::info!("palette → off");
        }
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
                log::info!("share: #{}", self.share_string());
                return true;
            }
            // Voxel editing (E14): V place, B break, U undo.
            KeyCode::KeyV => {
                self.edit_look(true);
                return true;
            }
            KeyCode::KeyB => {
                self.edit_look(false);
                return true;
            }
            KeyCode::KeyU => {
                self.undo_edit();
                return true;
            }
            // Palette post-process (E10): C cycles off → each palette → off; -/= change the
            // colour count; [ / ] lower/raise the dither spread.
            KeyCode::KeyC => {
                self.cycle_palette();
                return true;
            }
            KeyCode::Minus => {
                self.palette_count = self.palette_count.saturating_sub(1).max(1);
                return true;
            }
            KeyCode::Equal => {
                let max = palettes::PALETTES[self.palette_index].colors.len() as u32;
                self.palette_count = (self.palette_count + 1).min(max);
                return true;
            }
            KeyCode::BracketLeft => {
                self.palette_dither = (self.palette_dither - 0.25).max(0.0);
                return true;
            }
            KeyCode::BracketRight => {
                self.palette_dither = (self.palette_dither + 0.25).min(2.0);
                return true;
            }
            // Pixel scale (E10): K cycles the internal-resolution divisor 1→2→3→4→1.
            KeyCode::KeyK => {
                self.pixel_scale = self.pixel_scale % 4 + 1;
                log::info!("pixel scale → {}", self.pixel_scale);
                return true;
            }
            // Biome auto mode (E10): G toggles biome-driven settings vs the manual look.
            KeyCode::KeyG => {
                self.biome_mode = !self.biome_mode;
                log::info!("biome mode → {}", self.biome_mode);
                return true;
            }
            // Explored map (E10): N opens/closes it; arrows pan it while open (else fall
            // through to camera movement).
            KeyCode::KeyN => {
                self.toggle_map();
                return true;
            }
            KeyCode::ArrowUp if self.map_view => {
                self.map_pan.1 -= MAP_PAN_STEP;
                return true;
            }
            KeyCode::ArrowDown if self.map_view => {
                self.map_pan.1 += MAP_PAN_STEP;
                return true;
            }
            KeyCode::ArrowLeft if self.map_view => {
                self.map_pan.0 -= MAP_PAN_STEP;
                return true;
            }
            KeyCode::ArrowRight if self.map_view => {
                self.map_pan.0 += MAP_PAN_STEP;
                return true;
            }
            // Audio (E16): M mutes/unmutes the drone (desktop only).
            #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
            KeyCode::KeyM => {
                if let Some(a) = &self.audio {
                    let on = a.toggle();
                    log::info!("audio {}", if on { "on" } else { "muted" });
                }
                return true;
            }
            _ => {}
        }
        #[cfg(target_arch = "wasm32")]
        let _ = code;
        false
    }

    /// Edit the voxel the camera is looking at (E14): `place` adds a block against the
    /// surface we hit, otherwise we remove the hit block. Routes through `edit::apply`
    /// (the command seam), records the inverse for undo, and re-meshes the chunk.
    fn edit_look(&mut self, place: bool) {
        let origin = self.camera.position;
        let dir = self.camera.forward();
        let seed = self.seed;
        // Overlay-aware solidity: an overlaid cell wins, else base terrain (y < height).
        let overlay = &self.overlay;
        let solid = move |p: [i32; 3]| match edit::world_to_chunk_local(p) {
            Some((coord, lx, ly, lz)) => match overlay.get(&coord) {
                Some(sec) => sec.get(lx, ly, lz).is_solid(),
                None => (p[1] as u32) < worldgen::height(p[0], p[2], seed),
            },
            None => false,
        };
        let Some(hit) = edit::raycast(origin, dir, 60.0, solid) else {
            return;
        };
        let target = if place {
            [
                hit.voxel[0] + hit.normal[0],
                hit.voxel[1] + hit.normal[1],
                hit.voxel[2] + hit.normal[2],
            ]
        } else {
            hit.voxel
        };
        let block = if place {
            world::BlockId(1) // stone
        } else {
            world::BlockId::AIR
        };
        let cmd = edit::Edit::Set { pos: target, block };
        if let Some((coord, inverse)) =
            edit::apply(&mut self.overlay, &worldgen::TerrainGen { seed }, &cmd)
        {
            self.undo.push(inverse);
            self.remesh(coord);
        }
    }

    /// Undo the last edit (E14) by applying its recorded inverse.
    fn undo_edit(&mut self) {
        let Some(inverse) = self.undo.pop() else {
            return;
        };
        let seed = self.seed;
        if let Some((coord, _)) =
            edit::apply(&mut self.overlay, &worldgen::TerrainGen { seed }, &inverse)
        {
            self.remesh(coord);
        }
    }

    /// Re-mesh + re-upload one overlay chunk (shared by editing and the sand sim).
    fn remesh(&mut self, coord: ChunkCoord) {
        if !self.loaded.contains(&coord) {
            return;
        }
        if let Some(sec) = self.overlay.get(&coord) {
            let inst = mesh_chunk(coord, sec, self.seed);
            if let Some(state) = self.state.as_mut() {
                state.upload_chunk(&inst);
            }
        }
    }

    /// The current world + view as a `ShareState` (E12) — for "copy link" / `--share`.
    /// G1: collect the inscription the player is aiming at — the nearest still-collectible
    /// glyph within reach and close to the view ray (the E14-style aim). Routed through the
    /// serializable `progress::Event`/`apply` seam; banks its stratum + records it in the codex.
    fn collect_aimed(&mut self) {
        self.collect_aimed_where(|_| true);
    }

    /// As [`collect_aimed`], but only considering sites that pass `keep` (the G5 `match` filter
    /// for the auto-collect path; manual `T` passes everything).
    fn collect_aimed_where(&mut self, keep: impl Fn(&progress::Collectible) -> bool) {
        const REACH: f32 = 60.0; // how far a collect pick carries
        const AIM_RADIUS: f32 = 3.0; // forgiving perpendicular tolerance (glyphs are ~1u tall)
        let origin = self.camera.position;
        let dir = self.camera.forward();
        let mut best_t = f32::INFINITY;
        let mut best_i: Option<usize> = None;
        for (i, c) in self.collectible.iter().enumerate() {
            if !keep(c) {
                continue;
            }
            let v = Vec3::from(c.pos) - origin;
            let t = v.dot(dir);
            if t <= 0.0 || t > REACH {
                continue; // behind us / out of reach
            }
            if (v - dir * t).length() <= AIM_RADIUS && t < best_t {
                best_t = t;
                best_i = Some(i);
            }
        }
        let Some(idx) = best_i else {
            log::info!("collect: nothing in your sights");
            return;
        };
        self.collect_index(idx);
    }

    /// The hands-off **auto-collect** the routine interpreter drives (on-scan / when / continuous
    /// `collect`): harvest the nearest known site within a generous spherical reach of the ship —
    /// so the autopilot loop actually collects at **cruise altitude** (sites sit ~`CRUISE_HEIGHT`
    /// below the forward aim ray, so the precise aim pick used by *manual* collect misses them).
    /// Honours the optional `match` filter.
    fn collect_nearby_where(&mut self, keep: impl Fn(&progress::Collectible) -> bool) {
        const AUTO_REACH: f32 = 45.0; // ~2× cruise height: takes sites the ship passes over
        let origin = self.camera.position;
        let mut best = AUTO_REACH * AUTO_REACH;
        let mut best_i: Option<usize> = None;
        for (i, c) in self.collectible.iter().enumerate() {
            if !keep(c) {
                continue;
            }
            let d2 = (Vec3::from(c.pos) - origin).length_squared();
            if d2 <= best {
                best = d2;
                best_i = Some(i);
            }
        }
        if let Some(idx) = best_i {
            self.collect_index(idx);
        }
    }

    /// Collect the `idx`-th collectible: remove it, bank it through the serializable event seam,
    /// and drop its chunk from the opportunity surface. Shared by the aim pick + the auto-collect.
    fn collect_index(&mut self, idx: usize) {
        let c = self.collectible.remove(idx);
        let ev = progress::Event::Collect {
            find_id: c.find_id,
            script: c.script,
            text: c.text.clone(),
            pos: c.pos,
        };
        if self.progress.apply(&ev) {
            log::info!(
                "collected \"{}\" → +{} {}",
                c.text,
                progress::yield_amount(c.script, progress::glyph_count(&c.text)),
                progress::stratum_of(c.script).label(),
            );
            self.forget_scanned_chunk(c.pos); // it's no longer an opportunity (G3)
        }
    }

    /// G2: cast the survey-beam from the camera along the aim direction. In Walk mode it can
    /// **board the cruiser** (lock-on, reach-gated) or **attach as a rail**; either way it
    /// sweeps up every collectible glyph along its path on cast (one-shot, feeding G1).
    fn cast_beam(&mut self) {
        let b = beam::Beam::cast(self.camera.position, self.camera.forward(), self.time);
        // Board: on foot, a beam that locks onto the parked cruiser within reach reels you in
        // and boards (the E19 "enter" as a *ranged* alternative to walk-up-and-press-E).
        if self.mode == Mode::Walk
            && beam::within_reach(self.camera.position, self.cruiser_pos)
            && beam::dist_point_segment(self.cruiser_pos, b.a, b.b) <= 6.0
        {
            self.beam = Some(b);
            self.camera.position = self.cruiser_pos + Vec3::new(0.0, 1.5, 0.0);
            self.mode = Mode::Pilot;
            self.auto_fly = false;
            self.ride_t = None;
            log::info!("beam: locked on — reeled in and boarded the cruiser");
            return;
        }
        let mut swept = 0u32;
        let mut done: Vec<u64> = Vec::new();
        let mut done_pos: Vec<[f32; 3]> = Vec::new();
        for c in &self.collectible {
            if beam::on_path(Vec3::from(c.pos), b.a, b.b) {
                let ev = progress::Event::Collect {
                    find_id: c.find_id,
                    script: c.script,
                    text: c.text.clone(),
                    pos: c.pos,
                };
                if self.progress.apply(&ev) {
                    swept += 1;
                    done.push(c.find_id);
                    done_pos.push(c.pos);
                }
            }
        }
        self.collectible.retain(|c| !done.contains(&c.find_id));
        for p in done_pos {
            self.forget_scanned_chunk(p); // collected → no longer opportunities (G3)
        }
        self.beam = Some(b);
        // On foot, attach to the fresh beam as a rail (ride it to escape pits/cliffs — and
        // casting mid-fall re-attaches, saving you).
        if self.mode == Mode::Walk {
            self.ride_t = Some(0.0);
        }
        log::info!("beam: cast — swept {swept} glyph(s)");
    }

    /// G3: while piloting, the cruiser auto-scans the forward cone — marking nearby
    /// uncollected sites **known** (the map opportunity surface) and firing brief cool flicks.
    /// Does not collect. Rate-limited so it reads as a sweep.
    fn autoscan(&mut self, dt: f32) {
        let now = self.time;
        self.flicks.retain(|f| !f.dead(now)); // prune spent flicks every frame
                                              // Driven by the interpreter (G7): scan only while a continuous routine asks + piloting.
        if self.mode != Mode::Pilot || !self.scan_wanted {
            return;
        }
        self.scan_timer += dt;
        if self.scan_timer < scan::INTERVAL {
            return;
        }
        self.scan_timer = 0.0;
        self.scan_pulse();
    }

    /// One scan pulse (`scan(shards)` block) from the player's vantage. Reused by the piloted
    /// routine tick and a manual `scan` click.
    fn scan_pulse(&mut self) {
        self.scan_from(self.camera.position, self.camera.forward(), true);
    }

    /// One scan pulse from an arbitrary **vantage** (`origin` + `forward`): mark uncollected sites
    /// in the forward cone **known** (the map opportunity surface) + fire cool flicks. When
    /// `do_on_scan` is set (the player's own ship), the interpreter's `on-scan` routines run
    /// (typically a collect); the **autonomous away-ship** passes `false` — it scans (fills the
    /// map) but doesn't bank (a cheap off-screen agent, game-system §7). (G8a)
    fn scan_from(&mut self, cam: Vec3, fwd: Vec3, do_on_scan: bool) {
        let now = self.time;
        // Gather candidates ahead (immutable borrow), then mark them known (mutable) — keeping
        // only the newly-known ones for flicks/map.
        let candidates: Vec<(u64, [f32; 3])> = self
            .collectible
            .iter()
            .filter(|c| {
                !self.progress.is_scanned(c.find_id)
                    && scan::in_cone(Vec3::from(c.pos), cam, fwd, scan::RANGE)
            })
            .map(|c| (c.find_id, c.pos))
            .collect();
        let fresh: Vec<[f32; 3]> = candidates
            .into_iter()
            .filter(|(id, _)| self.progress.scan(*id))
            .map(|(_, p)| p)
            .collect();
        let found = !fresh.is_empty();
        let nch = world::Section::SIZE as f32;
        let nose = cam + fwd * 1.5;
        for (i, p) in fresh.into_iter().enumerate() {
            let pv = Vec3::from(p);
            let k = ((pv.x / nch).floor() as i32, (pv.z / nch).floor() as i32);
            if self.map_scanned.insert(k) {
                self.map_dirty = true;
            }
            if i < scan::FLICKS_PER_PULSE {
                self.flicks.push(scan::Flick {
                    from: nose,
                    to: pv,
                    born: now,
                });
            }
        }
        // on-scan → the interpreter's on-scan routines (G7): typically a (filtered) collect.
        // Skipped for the autonomous away-ship (it fills the map but doesn't bank). The collect is
        // the generous nearby auto-collect, so the hands-off loop bites at cruise altitude (G7).
        if found && do_on_scan {
            // The scanning agent is whoever's at the helm: piloting → ship, on foot → walker (G8b).
            let agent = if self.mode == Mode::Walk {
                console::Agent::Foot
            } else {
                console::Agent::Ship
            };
            for act in self.console.on_scan_acts(agent) {
                match act.block {
                    console::Block::Collect => self.dispatch_collect(act.filter),
                    console::Block::FireBeam => self.cast_beam(),
                    console::Block::Decode => self.decode_action(),
                    _ => {}
                }
            }
        }
    }

    /// Run a routine `collect` act (the hands-off auto-collect), honouring an optional `match`
    /// filter (G5/G7). Uses the generous nearby reach so it harvests at cruise altitude.
    fn dispatch_collect(&mut self, filter: Option<console::MatchField>) {
        match filter {
            Some(console::MatchField::Rare) => self.collect_nearby_where(|c| {
                matches!(
                    progress::stratum_of(c.script),
                    progress::Stratum::Relics | progress::Stratum::Signals
                )
            }),
            None => self.collect_nearby_where(|_| true),
        }
    }

    /// G6: refresh which blocks the console offers from what's been comprehended (the growing
    /// vocabulary). Cheap (five checks); called before the console is used or rendered.
    fn sync_console_unlock(&mut self) {
        self.console.unlocked = progress::Stratum::ALL
            .into_iter()
            .filter(|&s| self.progress.is_comprehended(s))
            .collect();
    }

    /// G7: keyboard/pad control of the open console (no typing) — cursor + discrete buttons.
    /// Home: ↑↓ select · Enter run/toggle/create · E edit · X delete. Editor: ↑↓ move · ←→ change
    /// step/trigger kind · -/+ nudge a value · Enter insert · X remove · `[`/`]` reorder · O back.
    fn console_key(&mut self, code: KeyCode) {
        self.sync_console_unlock();
        match self.console.view {
            console::View::Home => match code {
                KeyCode::KeyO | KeyCode::Escape => self.console.open = false,
                KeyCode::ArrowUp | KeyCode::KeyW => self.console.move_cursor(-1),
                KeyCode::ArrowDown | KeyCode::KeyS => self.console.move_cursor(1),
                KeyCode::Enter | KeyCode::Space => self.console_confirm(),
                KeyCode::KeyE => {
                    if let console::Sel::Routine(i) = self.console.selected() {
                        self.console.open_editor(i);
                    }
                }
                KeyCode::KeyX | KeyCode::Delete | KeyCode::Backspace => {
                    if let console::Sel::Routine(i) = self.console.selected() {
                        self.console.delete_routine(i);
                    }
                }
                _ => {}
            },
            console::View::Edit(i) => match code {
                KeyCode::KeyO | KeyCode::Escape => self.console.close_editor(),
                KeyCode::ArrowUp | KeyCode::KeyW => self.console.move_cursor(-1),
                KeyCode::ArrowDown | KeyCode::KeyS => self.console.move_cursor(1),
                KeyCode::ArrowLeft | KeyCode::KeyA => self.console.cycle(-1),
                KeyCode::ArrowRight | KeyCode::KeyD => self.console.cycle(1),
                KeyCode::Minus => self.console.adjust(-1),
                KeyCode::Equal => self.console.adjust(1),
                KeyCode::Enter | KeyCode::Space => self.console.insert_step(i),
                KeyCode::KeyX | KeyCode::Delete | KeyCode::Backspace => self.console.remove_step(i),
                KeyCode::BracketLeft => self.console.move_step(i, -1),
                KeyCode::BracketRight => self.console.move_step(i, 1),
                KeyCode::Tab => self.console.cycle_agent(), // G8b: flip ship ↔ foot
                _ => {}
            },
        }
    }

    /// G7: confirm on the home cursor — run a block, toggle a routine, or create a new one.
    fn console_confirm(&mut self) {
        match self.console.selected() {
            console::Sel::Block(b) => self.dispatch_block(b),
            console::Sel::NewRoutine => {
                // New routines default to the ship agent; Tab in the editor flips to foot (G8b).
                self.console.create_routine(console::Agent::Ship);
            }
            console::Sel::Routine(i) => {
                let on = self.console.toggle_routine(i);
                // Re-enabling a routine that steers the autopilot re-engages the wander (generic —
                // keyed on the routine's body, not its name).
                if on && self.console.routines[i].is_nav() {
                    self.auto_fly = true;
                }
            }
        }
    }

    /// G4: run a console block once — parity with its keybind shortcut; routines call this too.
    fn dispatch_block(&mut self, b: console::Block) {
        if !self.console.is_unlocked(b) {
            log::info!(
                "block {}: not yet recovered — decode its stratum",
                b.label()
            );
            return;
        }
        match b {
            console::Block::Scan(_) => self.scan_pulse(),
            console::Block::Collect => self.collect_aimed(),
            console::Block::FireBeam => self.cast_beam(),
            console::Block::Drift => self.auto_fly = true, // (re)engage the wander
            console::Block::Decode => self.decode_action(),
            console::Block::Hail => self.hail_ship(), // G8a: recall the autonomous ship
            console::Block::RunFoot => self.start_expedition(), // G8c: deploy the walker
            other => log::info!("block {}: not available yet", other.label()),
        }
    }

    /// D9: which touch context the buttons + view-tap reinterpret under (menu > walk > flight).
    fn touch_ctx(&self) -> touch::Ctx {
        if self.console.open || self.map_view || self.codex_open {
            touch::Ctx::Menu
        } else if self.mode == Mode::Walk {
            touch::Ctx::Walk
        } else {
            touch::Ctx::Flight
        }
    }

    /// D9: route one normalised touch sample through the pure mapping. Buttons fire on touch-down
    /// (rising edge); sliders + view touches are tracked, and a view touch that lands fires its
    /// action (beam / menu hit-test) on touch-up.
    fn handle_touch(&mut self, tp: brickmap::touch::TouchPoint) {
        use brickmap::touch::TouchPhase as P;
        self.touch_seen = true;
        let region = self.touch_layout.classify(tp.x, tp.y);
        let ctx = self.touch_ctx();
        match tp.phase {
            P::Start => {
                if let Some(act) = self.touch_layout.button_tap(region, ctx) {
                    self.touch_pressed = Some((region, self.time)); // D10: brief press highlight
                    self.dispatch_tap(act); // a button: act immediately
                } else {
                    self.touches.insert(tp.id, (tp.x, tp.y)); // slider / view: track it
                }
            }
            P::Move => {
                if let Some(p) = self.touches.get_mut(&tp.id) {
                    *p = (tp.x, tp.y);
                }
            }
            P::End | P::Cancel => {
                if let Some((sx, sy)) = self.touches.remove(&tp.id) {
                    // A tap that began + ended in the view → fire the view action.
                    let began = self.touch_layout.classify(sx, sy);
                    if tp.phase == P::End
                        && began == touch::Region::View
                        && region == touch::Region::View
                    {
                        let act = self.touch_layout.view_tap(ctx, tp.x, tp.y);
                        self.dispatch_tap(act);
                    }
                }
            }
        }
    }

    /// D9: dispatch a discrete touch action onto the **existing** control paths (no new logic).
    fn dispatch_tap(&mut self, act: touch::Tap) {
        match act {
            touch::Tap::Cruise => self.auto_fly = !self.auto_fly, // A: like the pad's A / `F`
            touch::Tap::Board => self.touch_board(),              // B: board / land&exit / hail
            touch::Tap::Console => {
                self.sync_console_unlock();
                self.console.open = !self.console.open;
            }
            touch::Tap::Map => self.toggle_map(),
            touch::Tap::Back => {
                self.console.open = false;
                if self.map_view {
                    self.toggle_map();
                }
                self.codex_open = false;
            }
            touch::Tap::Beam => self.cast_beam(),
            touch::Tap::MenuTap(_x, y) => {
                // Tap a console row to run/toggle it (Home view). Mapping y→row is approximate —
                // precise tap targeting is the deferred on-device feel-tuning.
                if self.console.open && matches!(self.console.view, console::View::Home) {
                    self.sync_console_unlock();
                    let rows = self.console.home_rows().max(1);
                    self.console.cursor = ((y * rows as f32) as usize).min(rows - 1);
                    self.console_confirm();
                }
            }
        }
    }

    /// D9 `B`: board the parked ship if you're next to it, recall it (`hail`) if it's away (on
    /// foot); land & exit while piloting. Reuses the existing enter/exit + hail paths.
    fn touch_board(&mut self) {
        if self.mode == Mode::Walk {
            let near = (self.camera.position - self.cruiser_pos)
                .with_y(0.0)
                .length()
                <= CRUISER_ENTER_DIST;
            if near {
                self.toggle_cruiser();
            } else {
                self.hail_ship();
            }
        } else {
            self.toggle_cruiser();
        }
    }

    /// D9: the current (left, right) slider values from the held touches (for the HUD overlay).
    fn touch_slider_vals(&self) -> (f32, f32) {
        let (mut left, mut right) = (0.0f32, 0.0f32);
        for &(x, y) in self.touches.values() {
            match self.touch_layout.classify(x, y) {
                touch::Region::LeftSlider => {
                    left = self
                        .touch_layout
                        .slider_value(touch::Region::LeftSlider, y)
                        .unwrap_or(0.0)
                }
                touch::Region::RightSlider => {
                    right = self
                        .touch_layout
                        .slider_value(touch::Region::RightSlider, y)
                        .unwrap_or(0.0)
                }
                _ => {}
            }
        }
        (left, right)
    }

    /// D9: per-frame — drive steering + altitude/forward from the held slider touches, onto the
    /// same `CameraController` the keys/pad use. Touching a slider yields the autopilot (like
    /// WASD/stick). Called each frame from the update loop.
    fn apply_touch(&mut self) {
        let (mut steer, mut vert) = (0.0f32, 0.0f32);
        for &(x, y) in self.touches.values() {
            match self.touch_layout.classify(x, y) {
                touch::Region::RightSlider => {
                    steer = self
                        .touch_layout
                        .slider_value(touch::Region::RightSlider, y)
                        .unwrap_or(0.0);
                }
                touch::Region::LeftSlider => {
                    vert = self
                        .touch_layout
                        .slider_value(touch::Region::LeftSlider, y)
                        .unwrap_or(0.0);
                }
                _ => {}
            }
        }
        if steer == 0.0 && vert == 0.0 {
            return;
        }
        self.auto_fly = false; // a slider yields the autopilot
        self.controller.add_look(steer * TOUCH_TURN, 0.0); // right slider = yaw
        match self.mode {
            Mode::Walk => self.controller.add_move(0.0, 0.0, vert), // left slider = forward/back
            _ => self.controller.add_move(0.0, vert, 0.0),          // left slider = climb/descend
        }
    }

    /// G6: `decode` — comprehend the richest stratum you can currently afford (spends its data),
    /// making that script legible + (later) growing the vocabulary. The map text re-renders next
    /// time inscriptions stream in.
    fn decode_action(&mut self) {
        match self.progress.decodable() {
            Some(s) if self.progress.comprehend(s) => {
                self.text_cells.clear(); // force inscriptions to rebuild as translated
                log::info!("decoded {} — its script is now legible", s.label());
            }
            _ => log::info!(
                "decode: need {} of a stratum's data to comprehend it",
                progress::DECODE_COST
            ),
        }
    }

    /// The nearest known-uncollected site to `pos`, if any (by distance).
    fn nearest_site_to(&self, pos: Vec3) -> Option<Vec3> {
        self.collectible
            .iter()
            .min_by(|a, b| {
                (Vec3::from(a.pos) - pos)
                    .length_squared()
                    .total_cmp(&(Vec3::from(b.pos) - pos).length_squared())
            })
            .map(|c| Vec3::from(c.pos))
    }

    /// G5 `seek`: the nearest known-uncollected site to steer the ship toward, if any.
    fn seek_target(&self) -> Option<Vec3> {
        self.nearest_site_to(self.camera.position)
    }

    /// A site at `pos` was collected — drop its chunk from the opportunity surface (G3). A v1
    /// approximation (one opportunity per chunk); refine if chunks routinely hold several.
    fn forget_scanned_chunk(&mut self, pos: [f32; 3]) {
        let nch = world::Section::SIZE as f32;
        let k = ((pos[0] / nch).floor() as i32, (pos[2] / nch).floor() as i32);
        if self.map_scanned.remove(&k) {
            self.map_dirty = true;
        }
    }

    /// The G1 strata readout + the G3 known/found counts, for the HUD.
    fn strata_hud(&self) -> String {
        let s = &self.progress.strata;
        format!(
            "REC {} · SCH {} · RIT {} · REL {} · SIG {}   known {} · found {}   [T collect · J codex · O console]",
            s.records,
            s.schematics,
            s.rites,
            s.relics,
            s.signals,
            self.progress.known_count(),
            self.progress.collected_count(),
        )
    }

    /// The codex list overlay (text only, most-recent first) — the G1 archive screen.
    fn codex_text(&self) -> String {
        let c = &self.progress.codex;
        let mut out = format!("CODEX — {} finds   [J close]\n", c.len());
        if c.is_empty() {
            out.push_str("(nothing yet — aim at a glowing inscription and press T)");
        } else {
            for e in c.iter().rev().take(20) {
                out.push_str(&format!(
                    "{}  {}\n",
                    progress::stratum_of(e.script).label(),
                    e.text
                ));
            }
        }
        out
    }

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

    /// The full persisted/shared string: the view + the G1 progress segment (`pg=`). Both
    /// decoders are lenient and ignore each other's keys, so they ride one string.
    fn share_string(&self) -> String {
        format!(
            "{}&{}&{}",
            self.current_share().encode(),
            self.progress.encode(),
            self.console.encode(), // G5: the editable routine state (nav + filter)
        )
    }

    /// Stream colossal structures (E18) around the camera: seed-placed tube-tech relics, some
    /// **ethereal** (points), some **solid** (greedy-meshed). Caches each giant's geometry per
    /// cell and generates at most [`STRUCTURE_GEN_BUDGET`] new ones per frame (the streaming-hitch
    /// fix). The mesh→points **dissolve** is done in the shader by distance (solid relics stipple
    /// out as they recede), so geometry is uploaded once per cell — no per-frame LOD churn, no
    /// hitch at the transition. Rebuilds the combined buffers only when the cached set changes.
    fn update_structures(&mut self) {
        if self.state.is_none() {
            return;
        }
        let seed = self.seed;
        let placements =
            structures::colossi_near(seed, self.camera.position, STRUCTURE_RADIUS, |x, z| {
                worldgen::height(x.floor() as i32, z.floor() as i32, seed) as f32
            });
        let wanted: std::collections::HashSet<(i32, i32)> =
            placements.iter().map(structures::cell_key).collect();

        // Drop giants that have left range.
        let before = self.structures.len();
        self.structures.retain(|k, _| wanted.contains(k));
        let mut changed = self.structures.len() != before;

        // Generate newly-entered giants, budgeted so the cost spreads over frames. Solid relics
        // cache a mesh (the shader dissolves it with distance); ethereal ones a point cloud.
        let mut budget = STRUCTURE_GEN_BUDGET;
        for p in &placements {
            let key = structures::cell_key(p);
            if self.structures.contains_key(&key) {
                continue;
            }
            if budget == 0 {
                break;
            }
            budget -= 1;
            let entry = if p.human && p.solid {
                // E18: a fallen *human* giant you can land on — the baked figure voxelised + meshed
                // (a denser/smaller scale fills the shell). Pale metal, like the solid relics.
                CachedRelic {
                    points: Vec::new(),
                    meshes: human_solid_instances(
                        &self.human_points,
                        p.pos,
                        p.voxel * 22.0,
                        p.yaw,
                        world::BlockId(5),
                    ),
                }
            } else if p.human {
                // Ethereal fallen human — the baked figure as drift-through points (`fallen_splats`).
                CachedRelic {
                    points: model::fallen_splats(
                        &self.human_points,
                        p.pos,
                        p.voxel * 38.0, // ~44–59 world units (comparable to the tube-tech relics)
                        p.yaw,
                        HUMAN_COLOR,
                        p.seed,
                    ),
                    meshes: Vec::new(),
                }
            } else if p.solid {
                CachedRelic {
                    points: Vec::new(),
                    meshes: relic_chunk_instances(*p, world::BlockId(5)),
                }
            } else {
                CachedRelic {
                    points: relic::relic_points(p.pos, p.voxel, p.yaw, p.seed, COLOSSUS_COLOR),
                    meshes: Vec::new(),
                }
            };
            self.structures.insert(key, entry);
            changed = true;
        }
        if !changed {
            return;
        }

        // Rebuild the combined sets from the cache: ethereal → points, solid → meshes.
        let mut pts: Vec<foliage::SplatInstance> = Vec::new();
        let mut meshes: Vec<ChunkInstance> = Vec::new();
        for r in self.structures.values() {
            pts.extend_from_slice(&r.points);
            meshes.extend(r.meshes.iter().cloned());
        }
        if let Some(state) = self.state.as_mut() {
            state.set_structure_points(&pts);
            state.set_structure_meshes(&meshes);
        }
    }

    /// Stream in-world inscriptions (E17) around the camera, the same cache-on-change way as the
    /// colossi: regenerate the in-range set each frame (cheap — string composition), and only
    /// rebuild the label textures on the GPU when the set of in-range cells actually changes.
    fn update_inscriptions(&mut self) {
        if self.state.is_none() {
            return;
        }
        let seed = self.seed;
        let ground =
            |x: f32, z: f32| worldgen::height(x.floor() as i32, z.floor() as i32, seed) as f32;
        // Scattered grid markers (tag 0) + a monument label at each nearby colossus (tag 1). The
        // tag keeps the two grids' cell keys distinct so change-detection can't conflate them.
        let marks = structures::inscriptions_near(seed, self.camera.position, TEXT_RADIUS, ground);
        let colossi = structures::colossi_near(seed, self.camera.position, TEXT_RADIUS, ground);
        let mut wanted: std::collections::HashSet<(i32, i32, u8)> =
            std::collections::HashSet::new();
        wanted.extend(marks.iter().map(|m| (m.cell.0, m.cell.1, 0u8)));
        wanted.extend(colossi.iter().map(|p| {
            let c = structures::cell_key(p);
            (c.0, c.1, 1u8)
        }));
        if wanted == self.text_cells {
            return;
        }
        self.text_cells = wanted;
        // Remember which chunks hold inscriptions, so they show as markers on the explored map.
        let nch = world::Section::SIZE as f32;
        for pos in marks
            .iter()
            .map(|m| m.pos)
            .chain(colossi.iter().map(|p| p.pos))
        {
            let k = ((pos.x / nch).floor() as i32, (pos.z / nch).floor() as i32);
            if self.map_text.insert(k) {
                self.map_dirty = true;
            }
        }
        let inscriptions: Vec<structures::Inscription> = marks
            .into_iter()
            .chain(colossi.iter().map(structures::colossus_label))
            .collect();
        // G1: the still-collectible inscriptions in range (already-collected ones filtered
        // out), used as the collect pick's targets.
        self.collectible = inscriptions
            .iter()
            .filter_map(|m| {
                let id = progress::find_id(m.cell, m.script, &m.text);
                (!self.progress.has(id)).then(|| progress::Collectible {
                    find_id: id,
                    script: m.script,
                    text: m.text.clone(),
                    pos: m.pos.to_array(),
                })
            })
            .collect();
        // G6 decipherment: a comprehended script renders **translated** (a seeded lexicon
        // phrase in the Latin font) instead of glowing glyphs. The find id still hashes the
        // original glyphs (collecting stays stable), so only the *display* changes.
        let labels: Vec<(String, text::Script, Vec3, f32, [f32; 3])> = inscriptions
            .into_iter()
            .map(|m| {
                if self.progress.is_legible(m.script) {
                    let words = progress::glyph_count(&m.text);
                    (
                        lexicon::phrase(seed, m.cell, words),
                        text::Script::Latin,
                        m.pos,
                        m.height,
                        m.color,
                    )
                } else {
                    (m.text, m.script, m.pos, m.height, m.color)
                }
            })
            .collect();
        if let Some(state) = self.state.as_mut() {
            state.set_text_labels(&labels);
        }
    }

    /// Open/close the explored-map view (E10). Entering centres the pan on the camera + forces a
    /// rebuild so the latest explored set shows.
    fn toggle_map(&mut self) {
        self.map_view = !self.map_view;
        if self.map_view {
            let n = world::Section::SIZE as f32;
            self.map_pan = (self.camera.position.x / n, self.camera.position.z / n);
            self.map_dirty = true;
        }
        if let Some(state) = self.state.as_mut() {
            state.set_map_active(self.map_view);
        }
    }

    /// Enter/exit the cruiser (E19). Pilot → Walk only when landed (within `CRUISER_EXIT_ALT` of
    /// the ground, so you have to set down first); Walk → Pilot only when on foot near the parked
    /// ship. On entry you take manual control; the ship is parked just above the ground on exit.
    fn toggle_cruiser(&mut self) {
        let seed = self.seed;
        let ground =
            |x: f32, z: f32| worldgen::height(x.floor() as i32, z.floor() as i32, seed) as f32;
        match self.mode {
            Mode::Pilot => {
                let pos = self.camera.position;
                let gy = ground(pos.x, pos.z);
                if pos.y - gy <= CRUISER_EXIT_ALT {
                    self.cruiser_pos = Vec3::new(pos.x, gy + 2.0, pos.z); // park it here
                    self.camera.position.y = gy + player::EYE; // step out onto the ground
                    self.walker = player::Walker::default();
                    self.mode = Mode::Walk;
                    log::info!("exited cruiser → walking");
                } else {
                    log::info!("too high to exit — land the cruiser first");
                }
            }
            Mode::Walk => {
                let p = self.camera.position;
                let d = ((p.x - self.cruiser_pos.x).powi(2) + (p.z - self.cruiser_pos.z).powi(2))
                    .sqrt();
                if d <= CRUISER_ENTER_DIST {
                    self.camera.position = self.cruiser_pos + Vec3::new(0.0, 1.5, 0.0);
                    self.auto_fly = false; // take manual control on entry
                    self.mode = Mode::Pilot;
                    log::info!("entered cruiser → piloting (manual)");
                }
            }
        }
    }

    /// E13: toggle cinematic **photo mode** — entering saves the live camera + pauses the world;
    /// leaving restores the camera (with its FOV). Free-cam + FOV are handled in the update loop.
    fn toggle_photo(&mut self) {
        if self.photo_active {
            if let Some(cam) = self.photo_saved.take() {
                self.camera = cam; // restore the exact pre-photo shot
            }
            self.photo_active = false;
            log::info!("photo mode: off");
        } else {
            self.photo_saved = Some(self.camera);
            self.photo_active = true;
            self.set_capture(true); // grab the pointer for free-look
            log::info!("photo mode: on (paused · free-cam · -/= zoom)");
        }
    }

    /// The ship's effective world position: the camera while piloting, else the parked/autonomous
    /// cruiser. Used for arrival detection (G8c).
    fn ship_pos(&self) -> Vec3 {
        if self.mode == Mode::Pilot {
            self.camera.position
        } else {
            self.cruiser_pos
        }
    }

    /// G8c: has an agent at `pos` **arrived** at a site? — the nearest known-uncollected site is
    /// within arrival range. Drives the `on-arrive` trigger (seek → arrive → act).
    fn arrived_at(&self, pos: Vec3) -> bool {
        const ARRIVE_RADIUS: f32 = 12.0;
        self.seek_target()
            .is_some_and(|t| (t - pos).length() < ARRIVE_RADIUS)
    }

    /// Has the **ship** arrived at a site? (Arrival measured from the ship's position.)
    fn ship_arrived(&self) -> bool {
        self.arrived_at(self.ship_pos())
    }

    /// G8a: fly the **autonomous ship** while you're on foot. The cruiser runs its own ship
    /// routine independently — advancing `cruiser_pos` under the same nav intent the piloted
    /// autopilot uses (drift wander / seek nearest known site / circle), tracking cruise height —
    /// and **auto-scans** the world it passes (filling the map opportunity surface) when the
    /// `survey` routine is enabled. It does **not** bank (a cheap off-screen agent, game-system §7).
    /// Two agents working at once: the ship surveys ahead while you collect on foot.
    fn fly_autonomous_ship(&mut self, dt: f32) {
        // A continuous nav routine governs whether the ship flies itself; without one it stays put.
        if self.nav_intent.is_none() {
            return;
        }
        self.ship_t += dt;
        // Same steering math as the piloted autopilot, on the ship's own clock/heading.
        let (pos, angle) = autopilot_step(
            self.cruiser_pos,
            self.ship_angle,
            self.ship_t,
            self.nav_intent,
            self.seek_target(),
            self.seed,
            dt,
        );
        self.ship_angle = angle;
        self.cruiser_pos = pos;
        // Away-scan: the ship surveys the cone ahead of itself, filling the map (no collect).
        if self.scan_wanted {
            self.ship_scan_timer += dt;
            if self.ship_scan_timer >= scan::INTERVAL {
                self.ship_scan_timer = 0.0;
                let dir = Vec3::new(angle.cos(), 0.0, angle.sin());
                self.scan_from(self.cruiser_pos, dir, false);
            }
        }
    }

    /// G8a: **hail** — recall the autonomous/parked ship to the walker (the counterpart to the
    /// survey-beam's *board*, for when the ship has wandered off). It re-homes near you and resets
    /// its heading toward you, so you can re-board. Wireable into a foot routine (G8b/c).
    fn hail_ship(&mut self) {
        if self.mode != Mode::Walk {
            return;
        }
        let p = self.camera.position;
        let ground = worldgen::height(p.x.floor() as i32, p.z.floor() as i32, self.seed) as f32;
        // Set down a short step from the walker, parked just above the ground (board range).
        let drop = Vec3::new(p.x + 3.0, ground + 2.0, p.z);
        self.cruiser_pos = drop;
        self.ship_angle = (p.z - drop.z).atan2(p.x - drop.x);
        log::info!("hail: the cruiser returns to you");
    }

    /// Collect the nearest known site within reach of an arbitrary `origin` (the deployed walker
    /// during an expedition). Banks through the same G1 event seam. (G8c)
    fn collect_nearest_to(&mut self, origin: Vec3) {
        const REACH: f32 = 18.0; // close, on-foot reach
        let mut best = REACH * REACH;
        let mut best_i: Option<usize> = None;
        for (i, c) in self.collectible.iter().enumerate() {
            let d2 = (Vec3::from(c.pos) - origin).length_squared();
            if d2 <= best {
                best = d2;
                best_i = Some(i);
            }
        }
        if let Some(idx) = best_i {
            self.collect_index(idx);
        }
    }

    /// G8c: kick off an automated expedition (a ship `run(foot)` step / manual click) — deploy the
    /// walker from the ship to the nearest known site. Only while piloting (on foot you *are* the
    /// walker); needs a known site and no expedition already running.
    fn start_expedition(&mut self) {
        if self.mode != Mode::Pilot || self.expedition.active() {
            return;
        }
        let Some(target) = self.seek_target() else {
            log::info!("run(foot): no known site to collect");
            return;
        };
        self.expedition_target = Some(target);
        self.walker_pos = self.ship_pos();
        self.expedition.start();
        log::info!("run(foot): deploying the walker");
    }

    /// G8c: the **persistent away-walker** — while you pilot, the walker runs its own foot routine
    /// off-screen (the mirror of the autonomous away-ship in [`App::fly_autonomous_ship`]). A foot
    /// `walk` nav steers it toward the nearest known site (from *its* position) and its foot acts
    /// bank what it reaches — so a foot routine you authored keeps working while you fly. The
    /// ship-commanded expedition (`run(foot)`) takes precedence when active (handled by the caller).
    fn advance_away_walker(&mut self, dt: f32, data: u32) {
        let arrived = self.arrived_at(self.walker_pos);
        let foot = self.console.tick(console::Agent::Foot, data, arrived);
        if foot.nav == Some(console::Block::Walk) {
            if let Some(t) = self.nearest_site_to(self.walker_pos) {
                self.walker_pos = walk_toward(self.walker_pos, t, WALK_SPEED, dt);
                let g = worldgen::height(
                    self.walker_pos.x.floor() as i32,
                    self.walker_pos.z.floor() as i32,
                    self.seed,
                ) as f32;
                self.walker_pos.y = g + player::EYE;
            }
        }
        let walker_pos = self.walker_pos;
        for act in foot.acts {
            match act.block {
                console::Block::Collect => self.collect_nearest_to(walker_pos),
                console::Block::Decode => self.decode_action(),
                _ => {} // hail/fire-beam are no-ops for the off-screen walker
            }
        }
    }

    /// G8c: advance the **automated expedition** while piloting — the `run(foot)` cross-agent step
    /// deploys the walker, which walks out to the target site, collects, and returns to the ship
    /// (the ship holds at the site meanwhile; see the autopilot arm). The phase machine
    /// ([`expedition`]) is pure + tested; this drives the walker's position + the harvest collect.
    /// *(Feel-tuning — walk speed, the arrival/board radii, the dwell — is noted for end-of-run.)*
    fn advance_expedition(&mut self, dt: f32) {
        if !self.expedition.active() {
            return;
        }
        const ARRIVE: f32 = 4.0; // walker "at the site"
        const BOARD: f32 = 3.0; // walker "back at the ship"
        let ship = self.ship_pos();
        let target = self.expedition_target.unwrap_or(ship);
        // Move the walker per phase, snapping it to the ground it's crossing.
        let goal = match self.expedition.phase {
            expedition::Phase::Deploy => Some(target),
            expedition::Phase::Return => Some(ship),
            _ => None,
        };
        if let Some(goal) = goal {
            self.walker_pos = walk_toward(self.walker_pos, goal, WALK_SPEED, dt);
            let g = worldgen::height(
                self.walker_pos.x.floor() as i32,
                self.walker_pos.z.floor() as i32,
                self.seed,
            ) as f32;
            self.walker_pos.y = g + player::EYE;
        }
        let at_site = (self.walker_pos - target).with_y(0.0).length() < ARRIVE;
        let home = (self.walker_pos - ship).with_y(0.0).length() < BOARD;
        let prev = self.expedition.phase;
        let now = self.expedition.advance(at_site, home, dt);
        // On entering Harvest, the walker collects the ground-level find the ship couldn't reach.
        if prev != expedition::Phase::Harvest && now == expedition::Phase::Harvest {
            self.collect_nearest_to(self.walker_pos);
        }
    }

    /// Record a streamed-in chunk on the explored map: store its biome's representative colour at
    /// `(cx, cz)`. First time only (the world is one chunk layer). Marks the map image dirty.
    fn record_chunk(&mut self, coord: ChunkCoord) {
        let key = (coord.0, coord.2);
        if self.map.contains_key(&key) {
            return;
        }
        let n = world::Section::SIZE as i32;
        let (wx, wz) = ((coord.0 * n + n / 2) as f32, (coord.2 * n + n / 2) as f32);
        let c = biome::at(wx, wz, self.seed).colors[2]; // mid-ramp hue reads the biome
        self.map.insert(
            key,
            [
                (c[0] * 255.0) as u8,
                (c[1] * 255.0) as u8,
                (c[2] * 255.0) as u8,
            ],
        );
        // Mark the chunk if it sits in a rare pristine/ethereal pocket (its own map icon).
        if biome::variant(wx, wz, self.seed) > 0.5 {
            self.map_pristine.insert(key);
        }
        self.map_dirty = true;
    }

    /// Build the explored-map RGBA image (one texel per visited chunk, alpha 0 = unseen) over the
    /// bounding box of visited chunks. Returns `(w, h, min_cx, min_cz, rgba)` or `None` if empty.
    fn build_map_image(&self) -> Option<(u32, u32, i32, i32, Vec<u8>)> {
        if self.map.is_empty() {
            return None;
        }
        let (mut minx, mut maxx, mut minz, mut maxz) = (i32::MAX, i32::MIN, i32::MAX, i32::MIN);
        for &(cx, cz) in self
            .map
            .keys()
            .chain(self.map_text.iter())
            .chain(self.map_pristine.iter())
            .chain(self.map_scanned.iter())
        {
            minx = minx.min(cx);
            maxx = maxx.max(cx);
            minz = minz.min(cz);
            maxz = maxz.max(cz);
        }
        let (w, h) = ((maxx - minx + 1) as u32, (maxz - minz + 1) as u32);
        let mut rgba = vec![0u8; w as usize * h as usize * 4];
        for (&(cx, cz), &c) in &self.map {
            let i = (((cz - minz) as usize) * w as usize + (cx - minx) as usize) * 4;
            rgba[i..i + 4].copy_from_slice(&[c[0], c[1], c[2], 255]);
        }
        // Markers keep the biome colour in RGB but tag the **alpha** with a code, so the shader
        // can draw a *distinct shaped icon* per type rather than just a coloured pixel.
        // Codes: 255 plain, 200 scanned-uncollected (G3 opportunity), 160 text, 96 pristine.
        for &(cx, cz) in &self.map_text {
            let i = (((cz - minz) as usize) * w as usize + (cx - minx) as usize) * 4 + 3;
            rgba[i] = 160;
        }
        // G3 opportunity surface: a scanned-but-uncollected site (overrides the plain text dot).
        for &(cx, cz) in &self.map_scanned {
            let i = (((cz - minz) as usize) * w as usize + (cx - minx) as usize) * 4 + 3;
            rgba[i] = 200;
        }
        for &(cx, cz) in &self.map_pristine {
            // Pristine overrides text if a chunk has both.
            let i = (((cz - minz) as usize) * w as usize + (cx - minx) as usize) * 4 + 3;
            rgba[i] = 96;
        }
        Some((w, h, minx, minz, rgba))
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
            self.record_chunk(inst.coord); // build the explored map as we stream
        }
    }

    /// Cellular-voxel simulation step (E5 sand + E11 water). Seeds sand + water ahead of the
    /// camera (into loaded chunks only — so it never races the streaming loader), steps active
    /// overlay sections on a fixed tick, and re-meshes the few that changed. Re-mesh is
    /// synchronous: the sim is localized, so it's a small, occasional cost. Gated by the sim
    /// toggle (off by default — re-meshing costs FPS), so the golden render is untouched.
    fn sim(&mut self, dt: f32) {
        if !self.toggles.sand || self.state.is_none() {
            return;
        }
        self.sand_timer += dt;
        while self.sand_timer >= SAND_INTERVAL {
            self.sand_timer -= SAND_INTERVAL;
            self.seed_sand();
        }
        // Water seeds on its own, slower cadence (E11) — it pools, so less of it reads as more.
        self.water_timer += dt;
        while self.water_timer >= WATER_INTERVAL {
            self.water_timer -= WATER_INTERVAL;
            self.seed_water();
        }

        self.sim_timer += dt.min(0.1);
        let mut dirty: HashSet<ChunkCoord> = HashSet::new();
        while self.sim_timer >= SIM_TICK {
            self.sim_timer -= SIM_TICK;
            let active: Vec<ChunkCoord> = self.sim_active.iter().copied().collect();
            for coord in active {
                // Step both materials; `|` (not `||`) so water still runs when sand also moved.
                let moved = self
                    .overlay
                    .get_mut(&coord)
                    .map(|sec| sim::step_sand(sec) | sim::step_water(sec));
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

    /// E9: advance the global weather cycle and, during precipitation, spawn rain/snow particles
    /// around the camera scaled by intensity (snow in frost biomes — slow, white, drifting; rain
    /// elsewhere — fast, cool, thin). Advanced only here (the live loop), so the headless/golden
    /// render stays dry. *(Fog/wetness coupling + god-rays + the audio term are noted follow-ups.)*
    fn tick_weather(&mut self, dt: f32) {
        self.weather.advance(dt);
        let intensity = self.weather.intensity();
        if intensity <= 0.0 {
            return;
        }
        let cold = biome::at(self.camera.position.x, self.camera.position.z, self.seed)
            .name
            .contains("frost");
        let cam = self.camera.position;
        // A handful of fresh drops per frame, scaled by intensity (capped so a downpour is dense
        // but bounded). Spawn in a box above + around the camera so it falls through the view.
        let n = (intensity * WEATHER_MAX_DROPS as f32).round() as u32;
        for i in 0..n {
            // Cheap per-drop scatter (deterministic-ish in frame count + index; cosmetic).
            let h = |salt: u32| {
                let mut x = (self.precip_seq.wrapping_mul(0x9e37_79b9))
                    .wrapping_add(i.wrapping_mul(0x85eb_ca6b))
                    .wrapping_add(salt.wrapping_mul(0xc2b2_ae35));
                x ^= x >> 16;
                (x & 0xffff) as f32 / 65535.0
            };
            let off = Vec3::new((h(1) - 0.5) * 60.0, 18.0 + h(2) * 14.0, (h(3) - 0.5) * 60.0);
            if cold {
                self.particles.spawn(
                    cam + off,
                    Vec3::new((h(4) - 0.5) * 2.0, -3.5, (h(5) - 0.5) * 2.0), // slow drift
                    Vec3::new(0.85, 0.88, 0.95),                             // white
                    3.5,
                    0.16,
                );
            } else {
                self.particles.spawn(
                    cam + off,
                    Vec3::new(0.0, -34.0, 0.0), // fast straight fall
                    Vec3::new(0.45, 0.55, 0.7), // cool
                    0.7,
                    0.07,
                );
            }
        }
        self.precip_seq = self.precip_seq.wrapping_add(1);
    }

    /// Seed a clump of sand high in a loaded chunk ahead of the camera; it falls onto the terrain.
    fn seed_sand(&mut self) {
        self.seed_material(sim::SAND, 0.0);
    }

    /// Seed a clump of **water** (E11) ahead of the camera; it falls, runs downhill, and pools in
    /// hollows (offset laterally from the sand curtain so the two are distinct).
    fn seed_water(&mut self) {
        self.seed_material(sim::WATER, std::f32::consts::PI);
    }

    /// Drop a 2×2 clump of `mat` high in a loaded chunk ahead of the camera (a *curtain* across the
    /// forward path, pseudo-random from the camera position), far enough ahead that it's watchable
    /// from the cruise. `phase` offsets the lateral sweep so different materials seed apart.
    fn seed_material(&mut self, mat: crate::world::BlockId, phase: f32) {
        let mut fwd = self.camera.forward();
        fwd.y = 0.0;
        let fwd = fwd.normalize_or_zero();
        let right = Vec3::new(-fwd.z, 0.0, fwd.x);
        let t = self.camera.position.x * 0.7 + self.camera.position.z * 0.9;
        let lateral = (t + phase).sin() * 16.0;
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
                sec.set(x as u32, y, z as u32, mat);
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
    mesh_chunk_core(coord, center, &neighbors, seed)
}

/// Mesh `center` against pre-built `neighbors` + scatter foliage + bake connectivity. The
/// shared core of [`mesh_chunk`] and the cached web builder (so neither regenerates sections
/// the other already has).
fn mesh_chunk_core(
    coord: ChunkCoord,
    center: &Section,
    neighbors: &Neighbors,
    seed: u32,
) -> ChunkInstance {
    let (cx, _cy, cz) = coord;
    let mesh = greedy_mesh_section_with(center, neighbors);
    let s = Section::SIZE as f32;
    // Biome lushness varies slowly (E8), so sample it once at the chunk centre to scale
    // foliage density: wet biomes thick, dry ones thin (deserts/snow have no grass at all).
    let n = Section::SIZE as i32;
    let lush = worldgen::lushness(cx * n + n / 2, cz * n + n / 2, seed);
    // The biome at this chunk also scales spawns (E10): some biomes are thick with growth,
    // others barren. Sampled at the chunk centre; the field is continuous so density eases
    // across biome borders. Always applied (it's a worldgen property, independent of the
    // biome-mode *look* toggle).
    let (bf, bforest, _, _) =
        biome::density((cx * n + n / 2) as f32, (cz * n + n / 2) as f32, seed);
    let density = (FOLIAGE_DENSITY as f32 * lush * bf).round() as u32;
    let woods = (lush * bforest).clamp(0.0, 1.0);
    // Ground grass + undergrowth bushes + point-cloud trees (E6/E7) share one per-chunk
    // splat buffer.
    let mut foliage = foliage::scatter(center, cx, cz, seed, density);
    foliage.extend(foliage::scatter_bushes(center, cx, cz, seed, woods));
    foliage.extend(foliage::scatter_trees(center, cx, cz, seed, woods));
    ChunkInstance {
        coord,
        origin: Vec3::new(cx as f32 * s, 0.0, cz as f32 * s),
        mesh,
        graph: connectivity(center),
        foliage,
    }
}

/// A cache of generated sections keyed by `(cx, cz)` (the world is one chunk layer), so the
/// web meshing path generates each section once instead of regenerating its 4 neighbours every
/// time — ~60% of the per-chunk cost. Used by the web `ChunkLoader::drain`.
#[cfg(any(target_arch = "wasm32", test))]
type SectionCache = std::collections::HashMap<(i32, i32), Section>;

/// Build a chunk's [`ChunkInstance`], pulling its own + neighbour sections from `cache`
/// (generating + caching on a miss). The big web streaming win — see [`SectionCache`].
#[cfg(any(target_arch = "wasm32", test))]
fn build_chunk_instance_cached(
    coord: ChunkCoord,
    seed: u32,
    cache: &mut SectionCache,
) -> ChunkInstance {
    let (cx, _cy, cz) = coord;
    for (dx, dz) in [(0, 0), (-1, 0), (1, 0), (0, -1), (0, 1)] {
        cache
            .entry((cx + dx, cz + dz))
            .or_insert_with(|| worldgen::generate_section(cx + dx, cz + dz, seed));
    }
    let center = &cache[&(cx, cz)];
    let neighbors = Neighbors {
        faces: [
            cache.get(&(cx - 1, cz)),
            cache.get(&(cx + 1, cz)),
            None,
            None,
            cache.get(&(cx, cz - 1)),
            cache.get(&(cx, cz + 1)),
        ],
    };
    mesh_chunk_core(coord, center, &neighbors, seed)
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
    /// Generated-section cache so the inline web mesher doesn't regenerate neighbours 5×.
    #[cfg(target_arch = "wasm32")]
    cache: SectionCache,
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
            #[cfg(target_arch = "wasm32")]
            cache: SectionCache::new(),
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
        {
            self.queue.clear();
            self.cache.clear();
        }
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
        {
            // Mesh inline, but cap the work this frame by a *time* budget (not just a count):
            // a chunk crossing queues a whole ring, and meshing them all at once is the
            // ~1–2 s periodic hitch. The section cache makes each chunk cheaper too.
            let start = web_time::Instant::now();
            while out.len() < budget {
                let Some(coord) = self.queue.pop_front() else {
                    break;
                };
                self.pending.remove(&coord);
                out.push(build_chunk_instance_cached(
                    coord,
                    self.seed,
                    &mut self.cache,
                ));
                if start.elapsed().as_secs_f32() * 1000.0 >= STREAM_MESH_BUDGET_MS {
                    break;
                }
            }
            // Bound the cache (rare; ~150–200 live around the camera).
            if self.cache.len() > 600 {
                self.cache.clear();
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
        // Letters for the toggles past the number row: L = sun (E3 point-lit mood),
        // I = ink blueprint-grid (E10).
        KeyCode::KeyL => 13,
        KeyCode::KeyI => 14,
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
            // Native: just block until the GPU is ready, then hand the engine the cruiser mesh.
            let mut state = pollster::block_on(State::new(window, &[]));
            state.set_ship_mesh(&cruiser::hull(), &cruiser::lights());
            self.state = Some(state);
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
        let AppEvent::Initialized(mut state) = event;
        state.set_ship_mesh(&cruiser::hull(), &cruiser::lights());
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
            // D9: phone touch. Normalise to 0..1 against the surface, then route through the pure
            // touch mapping. Slider touches are tracked (applied per-frame); button/view taps fire
            // here (rising-edge on down for buttons, on up for a view tap).
            WindowEvent::Touch(t) => {
                if let Some(state) = self.state.as_ref() {
                    let sz = state.window().inner_size();
                    let phase = match t.phase {
                        winit::event::TouchPhase::Started => brickmap::touch::TouchPhase::Start,
                        winit::event::TouchPhase::Moved => brickmap::touch::TouchPhase::Move,
                        winit::event::TouchPhase::Ended => brickmap::touch::TouchPhase::End,
                        winit::event::TouchPhase::Cancelled => brickmap::touch::TouchPhase::Cancel,
                    };
                    let tp = brickmap::touch::TouchPoint::new(
                        t.id,
                        phase,
                        t.location.x as f32,
                        t.location.y as f32,
                        sz.width as f32,
                        sz.height as f32,
                    );
                    self.handle_touch(tp);
                    if let Some(s) = self.state.as_ref() {
                        s.window().request_redraw();
                    }
                }
            }
            WindowEvent::KeyboardInput { event: key, .. } => {
                if let PhysicalKey::Code(code) = key.physical_key {
                    let pressed = key.state.is_pressed();
                    if self.console.open {
                        // G4: while the operations console is open it owns the keyboard
                        // (cursor + confirm, no typing); everything else is suppressed.
                        if pressed {
                            self.console_key(code);
                        }
                    } else if code == KeyCode::KeyO && pressed {
                        self.console.open = true; // open the operations console (G4)
                    } else if code == KeyCode::Escape && pressed {
                        // Release the pointer. (The browser also does this on Esc.)
                        self.set_capture(false);
                    } else if code == KeyCode::KeyF && pressed {
                        self.auto_fly = !self.auto_fly; // toggle autopilot (cinematic)
                    } else if code == KeyCode::KeyE && pressed {
                        self.toggle_cruiser(); // enter/exit the cruiser
                    } else if code == KeyCode::KeyT && pressed {
                        self.collect_aimed(); // G1: collect the aimed inscription
                    } else if code == KeyCode::KeyH && pressed {
                        self.hail_ship(); // G8a: recall the autonomous ship (on foot)
                    } else if code == KeyCode::KeyK && pressed {
                        self.toggle_photo(); // E13: cinematic photo mode (pause + free-cam + zoom)
                    } else if self.photo_active
                        && pressed
                        && matches!(code, KeyCode::Minus | KeyCode::Equal)
                    {
                        // E13: zoom (FOV) while in photo mode.
                        let d = if code == KeyCode::Minus { 4.0 } else { -4.0 };
                        self.camera.fov_y = adjust_fov(self.camera.fov_y, d);
                    } else if code == KeyCode::KeyJ && pressed {
                        self.codex_open = !self.codex_open; // G1: codex list overlay
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
                // (Android gamepad input is read directly in `android_main`, not via
                // winit key events — see `gamepad::android` + the pump loop.)
            }
            // Left click captures the pointer for mouselook; once captured, a click **casts
            // the survey-beam** (G2) where you aim — the signature manual verb.
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state: ElementState::Pressed,
                ..
            } => {
                self.auto_fly = false;
                if self.cursor_locked {
                    self.cast_beam();
                } else {
                    self.set_capture(true);
                }
            }
            // Pointer lock is lost when focus leaves; reflect that in our state.
            WindowEvent::Focused(false) => self.set_capture(false),
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let real_dt = self
                    .last_frame
                    .map(|t| (now - t).as_secs_f32().min(0.1))
                    .unwrap_or(0.0);
                self.last_frame = Some(now);
                // E13: photo mode **pauses** the living world — every time-driven system advances
                // on this `dt`, so zeroing it freezes sim/autopilot/expedition/animation while the
                // free-cam (driven below on `real_dt`) + streaming + rendering keep running.
                let dt = if self.photo_active { 0.0 } else { real_dt };

                // Gamepad (D7): poll the pad and feed analog move + look. The A button
                // toggles auto-fly; any stick/look input yields auto-fly to manual (like
                // WASD/mouse do).
                let pad = self.pad.poll();
                if pad.toggle_fly {
                    self.auto_fly = !self.auto_fly;
                }
                // Y / triangle toggles the distance-dissolve "melt" so it's easy to see the
                // effect with a pad in hand (the same switch as the `melt` feature toggle).
                if pad.toggle_melt {
                    self.toggles.melt = !self.toggles.melt;
                }
                // X / square opens/closes the explored map; entering centres it on the camera.
                if pad.toggle_map {
                    self.toggle_map();
                }
                // B / circle enters/exits the cruiser (E19).
                if pad.toggle_enter {
                    self.toggle_cruiser();
                }
                let stick = pad.strafe != 0.0
                    || pad.forward != 0.0
                    || pad.vertical != 0.0
                    || pad.look_x != 0.0
                    || pad.look_y != 0.0;
                if self.map_view {
                    // In the map: the left stick pans (chunk space); the world keeps flying
                    // underneath, so the you-are-here dot drifts live. North (−z) is up.
                    let sp = MAP_ZOOM * dt * 0.7;
                    self.map_pan.0 += pad.strafe * sp;
                    self.map_pan.1 -= pad.forward * sp;
                } else if stick {
                    self.auto_fly = false;
                    self.controller
                        .add_move(pad.strafe, pad.vertical, pad.forward);
                    self.controller.add_look(
                        pad.look_x * gamepad::LOOK_SPEED,
                        pad.look_y * gamepad::LOOK_SPEED,
                    );
                }
                // D9: phone touch — held sliders steer + climb/forward (onto the same controller),
                // applied after the pad so either input source works.
                self.apply_touch();

                // E13: photo mode pauses the world — run a free-cam only (`real_dt`), skipping the
                // interpreter + movement entirely so nothing sim-side advances while you frame a shot.
                if self.photo_active {
                    self.controller.update(&mut self.camera, real_dt);
                } else {
                    // G7/G8b: run one interpreter tick per agent. The **ship** agent's intents replace
                    // the old named-accessor hacks: `nav` steers the autopilot, `scan` gates the
                    // auto-scan, one-shot acts (continuous non-nav + `when`-edge fires) dispatch now.
                    let data = self.progress.strata.total().min(u32::MAX as u64) as u32;
                    let arrived = self.ship_arrived();
                    let tick = self.console.tick(console::Agent::Ship, data, arrived);
                    self.nav_intent = tick.nav;
                    self.scan_wanted = tick.scan;
                    for act in tick.acts {
                        match act.block {
                            console::Block::FireBeam => self.cast_beam(),
                            console::Block::Decode => self.decode_action(),
                            console::Block::Collect => self.dispatch_collect(act.filter),
                            console::Block::Hail => self.hail_ship(),
                            console::Block::RunFoot => self.start_expedition(), // G8c cross-agent
                            // A `when`-fired nav block engages the autopilot.
                            b if b.is_nav() => self.auto_fly = true,
                            _ => {}
                        }
                    }
                    // G8b/G8c: on foot, the walker is a **second agent** — its routines run
                    // simultaneously. The foot `walk` nav steers the walker toward known sites
                    // (applied in the Walk movement arm below); shared acts fire (a continuous
                    // `collect` harvests as you explore; `when … → decode`/`hail`; `on-arrive` when
                    // the walker reaches the site it's heading to).
                    let mut foot_nav = None;
                    if self.mode == Mode::Walk {
                        let walker_arrived = self.arrived_at(self.camera.position);
                        let foot = self
                            .console
                            .tick(console::Agent::Foot, data, walker_arrived);
                        foot_nav = foot.nav;
                        for act in foot.acts {
                            match act.block {
                                console::Block::Collect => self.dispatch_collect(act.filter),
                                console::Block::Decode => self.decode_action(),
                                console::Block::Hail => self.hail_ship(),
                                console::Block::FireBeam => self.cast_beam(),
                                _ => {}
                            }
                        }
                    }
                    // A continuous nav routine governs the autopilot wander; without one, drift is off
                    // (you keep manual control) — re-enabling a nav routine re-engages it (in console_confirm).
                    if self.nav_intent.is_none() {
                        self.auto_fly = false;
                    }
                    // G8c: while piloting, the walker is the autonomous **away-agent** (mirror of the
                    // away-ship): a ship `run(foot)` expedition drives it if one is out, else its own
                    // foot routine does (the persistent away-walker).
                    if self.mode == Mode::Pilot {
                        if self.expedition.active() {
                            self.advance_expedition(dt);
                        } else {
                            self.advance_away_walker(dt, data);
                        }
                    }
                    match self.mode {
                        // The ship hovers in place while a `run(foot)` expedition is out.
                        Mode::Pilot if self.auto_fly && !self.expedition.active() => {
                            // Autopilot — cinematic travel that **wanders to new places** (not a
                            // circle): a low-frequency, mean-zero turn rate meanders the heading in
                            // long S-curves while always cruising onward over fresh terrain. The
                            // steering math is shared with the autonomous away-ship (G8a).
                            self.auto_fly_t += dt;
                            let (pos, angle) = autopilot_step(
                                self.camera.position,
                                self.auto_fly_angle,
                                self.auto_fly_t,
                                self.nav_intent,
                                self.seek_target(),
                                self.seed,
                                dt,
                            );
                            self.auto_fly_angle = angle;
                            self.camera = Camera::new(pos, angle, AUTO_FLY_PITCH);
                        }
                        Mode::Pilot => {
                            // Manual flight (free 6-DOF).
                            self.controller.update(&mut self.camera, dt);
                        }
                        Mode::Walk => {
                            // The controller drives the look + the *wanted* free-fly delta.
                            let prev = self.camera.position;
                            self.controller.update(&mut self.camera, dt);
                            let mut wanted = self.camera.position;
                            // G8c: a foot `walk` routine **auto-walks** the walker toward the nearest
                            // known site — but only when you're not steering yourself (manual input
                            // always wins). Combined with `on-arrive → collect`, this is the on-foot
                            // auto-collection loop you compose.
                            let moved = (wanted - prev).with_y(0.0).length() > 1e-4;
                            if !moved && foot_nav == Some(console::Block::Walk) {
                                if let Some(target) = self.seek_target() {
                                    wanted = walk_toward(prev, target, WALK_SPEED, dt);
                                    self.camera.position = wanted;
                                }
                            }
                            // Riding the survey-beam (G2): if attached to a live beam, the wanted
                            // movement projected onto the beam axis slides you along the rail (1-DoF,
                            // any angle). Reaching the far end detaches; expiry drops you (gravity).
                            let riding = self
                                .ride_t
                                .zip(self.beam)
                                .filter(|(_, b)| !b.dead(self.time));
                            if let Some((t, b)) = riding {
                                let seg = b.b - b.a;
                                let len = seg.length().max(1e-3);
                                let along = (wanted - prev).dot(seg / len) / len;
                                let nt = (t + along).clamp(0.0, 1.0);
                                self.camera.position = b.a.lerp(b.b, nt);
                                self.ride_t = (nt < 1.0).then_some(nt); // arrive → detach
                            } else {
                                // Not riding (no beam / expired): a voxel-collided walk (gravity +
                                // animated auto-step) — so you can descend into cave-mouths, and you
                                // *drop* when the rail fades out from under you.
                                self.ride_t = None;
                                let seed = self.seed;
                                self.camera.position =
                                    self.walker.constrain(prev, wanted, dt, |x, y, z| {
                                        worldgen::solid_at(x, y, z, seed)
                                    });
                            }
                        }
                    }
                } // end !photo_active (E13)
                  // While piloting, the cruiser is wherever you are (you're in it). On foot, the
                  // cruiser is an **autonomous agent**: it flies its own ship routine (G8a).
                if self.mode == Mode::Pilot {
                    self.cruiser_pos = self.camera.position;
                } else {
                    self.fly_autonomous_ship(dt);
                }
                // Stream chunks in/out around the (possibly moved) camera.
                self.stream();
                // Stream colossal structures (E18) in/out around the camera.
                self.update_structures();
                // Stream in-world inscriptions (E17) in/out around the camera.
                self.update_inscriptions();

                // Reactive audio (E16): the drone breathes with the flight. Estimate speed from
                // the camera's motion this frame and blend in altitude, so it opens up when you
                // fly fast / high and settles when you hang still or sink into a valley.
                let pos = self.camera.position;
                let speed = self
                    .audio_prev_pos
                    .map(|p| (pos - p).length() / dt.max(1e-3))
                    .unwrap_or(0.0);
                self.audio_prev_pos = Some(pos);
                let speed_n = (speed / AUTO_FLY_SPEED).clamp(0.0, 1.0);
                let alt_n = ((pos.y - 24.0) / 90.0).clamp(0.0, 1.0);
                let intensity = (0.5 * speed_n + 0.5 * alt_n).clamp(0.0, 1.0);
                // E16×E9: precipitation darkens + thickens the drone (storms sound heavier).
                let weather_amt = self.weather.intensity();
                #[cfg(not(target_arch = "wasm32"))]
                if let Some(a) = &self.audio {
                    a.set_intensity(intensity);
                    a.set_weather(weather_amt);
                }
                #[cfg(target_arch = "wasm32")]
                {
                    controls::set_audio_intensity(intensity);
                    controls::set_audio_weather(weather_amt); // E16×E9 web parity
                }
                let _ = (intensity, weather_amt); // used per-target above
                                                  // Drifting wisp creatures (E15): advance the swarm (re-tethered to the live
                                                  // camera) and re-upload its points each frame so they drift through the scene.
                self.creatures.update(dt, self.camera.position);
                // How many motes drift is scaled by the biome at the camera (E10): misty/abyssal
                // biomes swarm with them, barren ones are sparse. ~7 wisps at the baseline.
                let wisp_mult =
                    biome::at(self.camera.position.x, self.camera.position.z, self.seed).wisps;
                let wisp_n = (7.0 * wisp_mult).round() as usize;
                // The polygonal cruiser shows parked while you're on foot; while piloting you're
                // inside it (hidden). Drawn over the palette in true colour (gfx).
                let ship_shown = self.mode == Mode::Walk;
                let ship_pos = self.cruiser_pos;
                // D9: precompute the touch overlay line before the mutable `state` borrow below.
                let touch_overlay = self.touch_seen.then(|| {
                    let (lv, rv) = self.touch_slider_vals();
                    self.touch_layout.overlay(lv, rv, self.touch_ctx())
                });
                // D10: precompute the visible overlay rects (empty unless a touch device is in use).
                let touch_rects: Vec<brickmap::hud::UiRect> = if self.touch_seen {
                    let (lv, rv) = self.touch_slider_vals();
                    // A pressed button stays highlighted for ~0.18 s after the tap.
                    let pressed = self
                        .touch_pressed
                        .filter(|(_, t)| self.time - t < 0.18)
                        .map(|(r, _)| r);
                    self.touch_layout.overlay_rects(lv, rv, pressed)
                } else {
                    Vec::new()
                };
                if let Some(state) = self.state.as_mut() {
                    state.set_creature_points(&self.creatures.points_n(wisp_n));
                    state.set_ship(ship_shown, ship_pos, 0.0);
                }
                // Falling-sand simulation (E5): seed, step, re-mesh dirty overlay chunks.
                self.sim(dt);
                // Global weather (E9): advance the cycle + spawn precipitation when it's raining.
                self.tick_weather(dt);
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
                    let (pon, pidx, pcount, pdither) = controls::palette();
                    self.palette_on = pon;
                    self.palette_index = pidx;
                    self.palette_count = pcount;
                    self.palette_dither = pdither;
                    self.pixel_scale = controls::pixel_scale();
                    self.biome_mode = controls::biome_mode();
                    if let Some(seed) = controls::take_pending_seed() {
                        self.set_seed(seed);
                    }
                }
                // The current share string for "copy link" (computed before the mutable
                // `state` borrow below; cheap, refreshed every frame).
                #[cfg(target_arch = "wasm32")]
                let share_str = self.share_string();

                // Explored-map prep (E10): rebuild the GPU image only when the explored set grew.
                // `build_map_image` borrows `&self`, so it must run before the mutable `state`
                // borrow below; the upload + uniform are pushed inside the state block.
                self.map_anim += dt;
                // G2 beam clock + expiry, G3 auto-scan, then build this frame's overlay verts
                // (warm beam ribbon + cool scan flicks) before the mutable `state` borrow below.
                self.time += dt;
                if self.beam.is_some_and(|b| b.dead(self.time)) {
                    self.beam = None;
                }
                self.autoscan(dt);
                let mut overlay_verts = self
                    .beam
                    .map(|b| b.ribbon(self.camera.position, self.time))
                    .unwrap_or_default();
                for f in &self.flicks {
                    overlay_verts.extend(f.ribbon(self.camera.position, self.time));
                }
                let map_upload = (self.map_view && self.map_dirty)
                    .then(|| self.build_map_image())
                    .flatten();
                if let Some((w, h, ox, oz, _)) = &map_upload {
                    self.map_origin = (*ox, *oz);
                    self.map_dims = (*w, *h);
                    self.map_dirty = false;
                }

                // Biome-driven auto mode (E10): blend the biome at the camera and drive the
                // look + drone mix (palette below; wobble/steps/sun + audio here). Blended fields
                // vary continuously with position, so all of it transitions smoothly.
                let bio = self
                    .biome_mode
                    .then(|| biome::at(self.camera.position.x, self.camera.position.z, self.seed));
                if let Some(b) = bio {
                    self.wobble = b.wobble;
                    self.color_steps = b.steps;
                    self.biome_label = b.label();
                    // Structure-approach wobble (E18×E10): as you near a colossus the quantization
                    // wobble pulls toward its own extreme — some giants warp space heavily (snap →
                    // min), others snap crisp (snap → max). Deterministic per giant from its seed;
                    // the nearest in-range one wins, ramped by horizontal proximity.
                    let cam = self.camera.position;
                    let seed = self.seed;
                    let colossi = structures::colossi_near(seed, cam, STRUCTURE_RADIUS, |x, z| {
                        worldgen::height(x.floor() as i32, z.floor() as i32, seed) as f32
                    });
                    let (mut best, mut target) = (0.0f32, self.wobble);
                    for p in &colossi {
                        // Most giants are wobble-neutral; only rare ones warp (heavy, ~1 in 6) or
                        // still (crisp, ~1 in 6) the space around them.
                        let t = match (p.seed >> 9) % 6 {
                            0 => WOBBLE_HEAVY,
                            1 => WOBBLE_CRISP,
                            _ => continue,
                        };
                        let d = ((p.pos.x - cam.x).powi(2) + (p.pos.z - cam.z).powi(2)).sqrt();
                        let prox = (1.0 - d / WOBBLE_APPROACH).clamp(0.0, 1.0);
                        if prox > best {
                            best = prox;
                            target = t;
                        }
                    }
                    self.wobble += (target - self.wobble) * best;
                    // Ethereal/ink pockets are pristine: pull wobble to zero last, so it wins over
                    // both the biome base and any nearby giant — these are untouched special areas.
                    self.wobble += (WOBBLE_PRISTINE - self.wobble) * b.ink;
                    // Warp audio (E18×E16): only fires once the wobble has been pulled well below
                    // baseline (i.e. right up against a "warping" giant) — heavier, throbbing drone.
                    let warp_amt = ((60.0 - self.wobble) / (60.0 - WOBBLE_HEAVY)).clamp(0.0, 1.0);
                    #[cfg(not(target_arch = "wasm32"))]
                    if let Some(a) = &self.audio {
                        a.set_volume(b.vol);
                        a.set_drive(b.heavy);
                        a.set_tone(b.murk);
                        a.set_warp(warp_amt);
                        a.set_ethereal(b.ink);
                    }
                    #[cfg(target_arch = "wasm32")]
                    {
                        controls::set_audio_volume(b.vol);
                        controls::set_audio_drive(b.heavy);
                        controls::set_audio_tone(b.murk);
                        controls::set_audio_warp(warp_amt);
                        controls::set_audio_ethereal(b.ink);
                    }
                }
                let sun_amt = match bio {
                    Some(b) => b.sun,
                    None => f32::from(u8::from(self.toggles.sun)),
                };
                // Ink amount: the rare biome ethereal-pocket fade in biome mode, else the toggle.
                let ink_amt = match bio {
                    Some(b) => b.ink,
                    None => f32::from(u8::from(self.toggles.ink)),
                };

                // Camera basis for billboarding foliage splats (E6).
                let fwd = self.camera.forward();
                let cam_right = fwd.cross(Vec3::Y).normalize_or_zero();
                let cam_up = cam_right.cross(fwd).normalize_or_zero();

                // G1/G4 HUD overlays, computed before the mutable `state` borrow below.
                let strata_line = self.strata_hud();
                let codex_view = self.codex_open.then(|| self.codex_text());
                if self.console.open {
                    self.sync_console_unlock(); // refresh the vocabulary for the render (G6)
                }
                let console_view = self.console.open.then(|| self.console.render());

                if let Some(state) = self.state.as_mut() {
                    let view_proj = self.camera.view_proj(state.aspect());
                    // Push the palette (biome-blended ramp in biome mode, else the manual
                    // selection) + pixel scale (E10) before drawing.
                    if let Some(b) = bio {
                        state.set_palette_colors(&b.colors, b.count, b.dither, true);
                    } else {
                        // The game owns the curated set; resolve the chosen index to its ramp
                        // and feed the engine's colour seam (it carries no palette table).
                        let pal = &palettes::PALETTES
                            [self.palette_index.min(palettes::PALETTES.len() - 1)];
                        state.set_palette_colors(
                            pal.colors,
                            self.palette_count.max(1),
                            self.palette_dither,
                            self.palette_on,
                        );
                    }
                    // M8a: dynamic resolution — when frames run over budget, drop the internal
                    // render resolution (a larger divisor on top of the art-directed `pixel_scale`)
                    // so weak hardware holds frame-rate; recovers toward the base when fast. Only
                    // ever makes the image *chunkier* than the art base (on-thesis), never sharper.
                    // On capable hardware `dyn_extra` stays 0 → byte-identical to before.
                    self.dyn_extra = dyn_resolution_step(
                        self.dyn_extra,
                        self.frame_ms_ema,
                        DYN_TARGET_MS,
                        DYN_MAX_EXTRA,
                    );
                    state.set_pixel_scale(self.pixel_scale + self.dyn_extra);
                    // Survey-beam (G2): feed this frame's ribbon to the engine's post-palette
                    // overlay (empty when no beam is up).
                    state.set_overlay(&overlay_verts);
                    // Explored-map overlay (E10): upload a fresh image if it grew, push the
                    // pan/zoom/you-are-here uniform, and flag it on/off for the render pass.
                    if let Some((w, h, _, _, rgba)) = &map_upload {
                        state.set_map_image(*w, *h, rgba);
                    }
                    if self.map_view {
                        let nch = world::Section::SIZE as f32;
                        let blink = 0.5 + 0.5 * (self.map_anim * 7.0).sin();
                        state.set_map_uniform(&map::MapUniform {
                            origin_dims: [
                                self.map_origin.0 as f32,
                                self.map_origin.1 as f32,
                                self.map_dims.0 as f32,
                                self.map_dims.1 as f32,
                            ],
                            view: [self.map_pan.0, self.map_pan.1, MAP_ZOOM, state.aspect()],
                            user: [
                                self.camera.position.x / nch,
                                self.camera.position.z / nch,
                                blink,
                                0.0,
                            ],
                            // Show the cruiser marker when it's parked (you're on foot).
                            cruiser: [
                                self.cruiser_pos.x / nch,
                                self.cruiser_pos.z / nch,
                                if self.mode == Mode::Walk { 1.0 } else { 0.0 },
                                0.0,
                            ],
                        });
                    }
                    state.set_map_active(self.map_view);
                    // `render` handles lost/outdated/transient surfaces internally.
                    state.render(
                        view_proj,
                        self.camera.position,
                        cam_right,
                        cam_up,
                        &particles,
                        [self.wobble, self.color_steps],
                        self.toggles,
                        sun_amt,
                        ink_amt,
                        weather_amt, // E9: precipitation greys the horizon in (murk)
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
                        let mut meshing = if meshing > 0 {
                            format!(" · meshing {meshing}")
                        } else {
                            String::new()
                        };
                        // M8a: surface dynamic-resolution when it's engaged (chunkier under load).
                        if self.dyn_extra > 0 {
                            meshing.push_str(&format!(" · dynres +{}", self.dyn_extra));
                        }
                        // E9: surface the weather when it's doing something.
                        if self.weather.intensity() > 0.0 {
                            meshing.push_str(&format!(" · {}", self.weather.phase().label()));
                        }
                        // In biome mode, show the (blended) biome name; else the manual palette.
                        let pal = if self.biome_mode {
                            format!(" · biome {}", self.biome_label)
                        } else if self.palette_on {
                            format!(
                                " · {} {}c",
                                palettes::PALETTES[self.palette_index].name,
                                self.palette_count
                            )
                        } else {
                            String::new()
                        };
                        // In biome mode the toggles are auto-driven, so don't clutter with them.
                        let off = if self.biome_mode {
                            String::new()
                        } else {
                            self.toggles.off_summary()
                        };
                        // Movement mode (E19): piloting (autopilot/manual) or walking.
                        let mut mode = match self.mode {
                            Mode::Pilot if self.auto_fly => " · cruiser:auto [E exit when low]",
                            Mode::Pilot => " · cruiser:manual [E exit when low]",
                            Mode::Walk if self.ride_t.is_some() => " · riding the beam",
                            Mode::Walk => " · walking [E enter · H hail · click: survey-beam]",
                        }
                        .to_string();
                        // G8c: surface the automated expedition's phase when one is running.
                        if let Some(exp) = self.expedition.label() {
                            mode.push_str(" · ");
                            mode.push_str(exp);
                        }
                        // E13: photo mode readout (paused · free-cam · FOV).
                        if self.photo_active {
                            mode = format!(
                                " · PHOTO {:.0}° [WASD/look · -/= zoom · K exit]",
                                self.camera.fov_y.to_degrees()
                            );
                        }
                        // When the map is open, the HUD becomes its key + coordinates (crosshair
                        // centre + your position), instead of the perf line.
                        let hud = if let Some(console) = &console_view {
                            console.clone()
                        } else if let Some(codex) = &codex_view {
                            codex.clone()
                        } else if self.map_view {
                            let nch = world::Section::SIZE as f32;
                            let (px, pz) =
                                (self.camera.position.x as i32, self.camera.position.z as i32);
                            let (vx, vz) =
                                ((self.map_pan.0 * nch) as i32, (self.map_pan.1 * nch) as i32);
                            format!(
                                "MAP  seed {}\nyou x{px} z{pz}   +crosshair x{vx} z{vz}\nkey: yellow=you  amber ring=scanned (go collect)  cyan dot=text  violet=pristine  orange=cruiser\n[stick / arrows] pan    [X / N] close",
                                self.seed,
                            )
                        } else {
                            format!(
                                "brickmap {BUILD} · {fps:.0} fps · {:.1} ms · seed {} · {}/{} chunks · {} tris · {} fx · {} splats · {} relics{pal}{meshing}{mode}{off}\n{}",
                                self.frame_ms_ema,
                                self.seed,
                                s.drawn_chunks,
                                s.total_chunks,
                                s.triangles,
                                s.particles,
                                s.splats,
                                s.relics,
                                strata_line,
                            )
                        };
                        // D9: once a touch device is in use, append the on-screen control overlay
                        // (sliders + buttons) to the HUD/text path. (Edge-strip placement + dimming
                        // is the deferred on-device visual; this is the headless-renderable v1.)
                        let hud = match &touch_overlay {
                            Some(o) => format!("{hud}\n{o}"),
                            None => hud,
                        };
                        // D10: the visible touch-control overlay (rects from `touch::Layout`).
                        state.set_ui_rects(&touch_rects);
                        // In-engine text overlay on every platform (no DOM HUD).
                        state.set_hud(&hud);
                        #[cfg(not(target_arch = "wasm32"))]
                        state.window().set_title(&format!("brickmap — {hud}"));
                        #[cfg(target_arch = "wasm32")]
                        controls::set_current_share(&share_str); // keep copy-link fresh
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

/// Shared entry point used by the native binary and the WASM start shim. Builds the
/// default event loop; Android instead builds one carrying the `AndroidApp` (see
/// `android_main`) and calls [`run_event_loop`] directly.
pub fn run() {
    init_logging();
    let event_loop = EventLoop::<AppEvent>::with_user_event()
        .build()
        .expect("failed to build event loop");
    run_event_loop(event_loop);
}

/// Android entry point. `android-activity` calls this with the `AndroidApp`. We thread it
/// into winit for the window/surface + lifecycle, but **drive the loop ourselves** with
/// `pump_app_events` so we can drain the input queue *before* winit each frame — winit
/// consumes Android input and drops the gamepad stick axes, so reading it first is the
/// only way to get analog sticks. (Built blind — verified on-device.)
#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(android_app: winit::platform::android::activity::AndroidApp) {
    use winit::platform::android::activity::InputStatus;
    use winit::platform::android::EventLoopBuilderExtAndroid;
    use winit::platform::pump_events::{EventLoopExtPumpEvents, PumpStatus};

    init_logging();
    let mut event_loop = EventLoop::<AppEvent>::with_user_event()
        .with_android_app(android_app.clone())
        .build()
        .expect("failed to build event loop");
    let mut app = build_app(&event_loop);

    loop {
        // 1) Drain Android input ourselves → feed the pad (sticks + buttons).
        if let Ok(mut events) = android_app.input_events_iter() {
            while events.next(|event| {
                if app.pad.handle_android_input(event) {
                    InputStatus::Handled
                } else {
                    InputStatus::Unhandled
                }
            }) {}
        }
        // 2) Pump winit (lifecycle + redraw). Vsync present throttles the frame rate.
        match event_loop.pump_app_events(Some(std::time::Duration::from_millis(4)), &mut app) {
            PumpStatus::Exit(_) => break,
            PumpStatus::Continue => {}
        }
    }
}

/// Desktop/web: build the app and run the event loop to completion.
fn run_event_loop(event_loop: EventLoop<AppEvent>) {
    let mut app = build_app(&event_loop);
    event_loop.run_app(&mut app).expect("event loop error");
}

/// G8a: one autopilot integration step — advance `(pos, angle)` by the nav intent (drift wander /
/// seek the given `seek_target` / circle), then track cruise height over terrain. Pure (terrain via
/// the seeded `worldgen::height`), so it's shared by the piloted autopilot **and** the autonomous
/// away-ship, and is unit-testable without a GPU. Returns the new `(pos, angle)`.
fn autopilot_step(
    pos: Vec3,
    angle: f32,
    t: f32,
    nav: Option<console::Block>,
    seek_target: Option<Vec3>,
    seed: u32,
    dt: f32,
) -> (Vec3, f32) {
    // Drift heading: a slow, smooth **fbm** of incommensurate sines (per-seed phase) so the turn
    // rate varies and *crosses zero* on a ~10–30 s scale — the ship **meanders and covers ground**
    // like an unhurried survey sweep, instead of holding a near-constant rate into a tight circle
    // (the old two-sine version turned too steadily → a loop). Cheap + deterministic; live-loop
    // only (doesn't touch the golden hash). Shared by the piloted drift + the autonomous away-ship.
    let ph = (seed & 0xffff) as f32 * 6.283_185e-5; // per-seed phase offset
    let wander = (t * 0.13 + ph).sin() * 0.30
        + (t * 0.29 + 1.7 + ph).sin() * 0.18
        + (t * 0.07 + 4.1 + ph).sin() * 0.10;
    let turn = match nav {
        Some(console::Block::Seek) => match seek_target {
            Some(tg) => {
                let want = (tg.z - pos.z).atan2(tg.x - pos.x);
                let mut d = want - angle;
                while d > std::f32::consts::PI {
                    d -= std::f32::consts::TAU;
                }
                while d < -std::f32::consts::PI {
                    d += std::f32::consts::TAU;
                }
                (d * 2.0).clamp(-1.5, 1.5)
            }
            None => wander, // nothing known yet → wander until the scan finds one
        },
        Some(console::Block::Circle) => 0.5, // steady turn = loiter/orbit
        _ => wander,
    };
    let angle = angle + turn * dt;
    let dir = Vec3::new(angle.cos(), 0.0, angle.sin());
    let mut pos = pos + dir * (AUTO_FLY_SPEED * dt);
    let ground = worldgen::height(pos.x.floor() as i32, pos.z.floor() as i32, seed) as f32;
    let target_y = ground + CRUISE_HEIGHT;
    pos.y += (target_y - pos.y) * (dt * 1.2).min(1.0);
    (pos, angle)
}

/// E13: adjust a vertical FOV (radians) by `delta_deg` degrees, clamped to a sane photo range
/// (20°–100°). Pure + unit-testable.
fn adjust_fov(fov_rad: f32, delta_deg: f32) -> f32 {
    (fov_rad.to_degrees() + delta_deg)
        .clamp(20.0, 100.0)
        .to_radians()
}

/// G8c: a foot auto-walk step — a horizontal wanted-position toward `target` at `speed`. The
/// caller feeds this through the walker's voxel-collision constrain (gravity / auto-step), so
/// this only sets the horizontal intent. Pure + unit-testable.
fn walk_toward(pos: Vec3, target: Vec3, speed: f32, dt: f32) -> Vec3 {
    let to = (target - pos).with_y(0.0);
    let d = to.length();
    if d < 1e-3 {
        return pos;
    }
    let step = (speed * dt).min(d);
    pos + to / d * step
}

/// Set up the world/view + `App`. Shared by the desktop/web entry (`run`) and the Android
/// entry (`android_main`, which then drives the loop itself).
fn build_app(event_loop: &EventLoop<AppEvent>) -> App {
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
        water_timer: 0.0,
        dyn_extra: 0,
        weather: weather::Weather::new(view.seed),
        precip_seq: 0,
        auto_fly: true,
        auto_fly_angle: 0.0,
        auto_fly_t: 0.0,
        nav_intent: Some(console::Block::Drift),
        scan_wanted: true,
        ship_t: 0.0,
        ship_angle: 0.0,
        ship_scan_timer: 0.0,
        photo_active: false,
        photo_saved: None,
        expedition: expedition::Expedition::default(),
        walker_pos: pos,
        expedition_target: None,
        mode: Mode::Pilot, // start in the cruiser, on autopilot (the watchable default)
        cruiser_pos: pos,
        walker: player::Walker::default(),
        wobble: view.wobble,
        color_steps: view.color_steps,
        // House look by default: the `bruise` palette (index 11), all 5 colours, heavy dither.
        palette_on: true,
        palette_index: 11,
        palette_count: 5,
        palette_dither: 1.5,
        pixel_scale: 2,
        last_frame: None,
        cursor_locked: false,
        frame_ms_ema: 0.0,
        hud_timer: 0.0,
        toggles: Toggles::from_mask(view.toggles),
        seed: view.seed,
        undo: Vec::new(),
        structures: std::collections::HashMap::new(),
        human_points: model::decode_points(HUMAN_POINTS_BLOB), // E18 baked human (embedded)
        // A small swarm of drifting wisps tethered to the camera's start (re-tethered each
        // frame to the live camera in the redraw loop). Seed-driven off the world seed.
        // A generous base count; how many actually drift is scaled per-biome each frame.
        creatures: creatures::Swarm::new(view.seed ^ 0xE15_E15, 12, pos, 80.0),
        text_cells: std::collections::HashSet::new(),
        audio_prev_pos: None,
        // Restore collected strata + codex from the same share source as the view (G1).
        progress: initial_progress(),
        collectible: Vec::new(),
        codex_open: false,
        console: console::Console::default(),
        beam: None,
        ride_t: None,
        scan_timer: 0.0,
        flicks: Vec::new(),
        map_scanned: std::collections::HashSet::new(),
        time: 0.0,
        biome_mode: true, // the new default mode
        biome_label: String::new(),
        map: std::collections::HashMap::new(),
        map_text: std::collections::HashSet::new(),
        map_pristine: std::collections::HashSet::new(),
        map_view: false,
        map_pan: (0.0, 0.0),
        map_dirty: false,
        map_origin: (0, 0),
        map_dims: (0, 0),
        map_anim: 0.0,
        pad: gamepad::Pad::new(),
        touches: std::collections::HashMap::new(),
        touch_layout: touch::Layout::default(),
        touch_seen: false,
        touch_pressed: None,
        // Start the drone on the world seed so the dirge matches the world (desktop + Android;
        // a no-op None if there's no audio device). Web starts audio from the page on first tap.
        #[cfg(not(target_arch = "wasm32"))]
        audio: audio_native::AudioEngine::start(view.seed),
    };
    // G5: restore any saved routine edits (nav + filter) from the same share source.
    app.console.restore(&initial_share_source());
    app
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

/// Restore collected progress (strata + codex) from the same share source as the view (G1):
/// the native `--share <blob>` arg, or the web URL hash. Absent/garbled → empty progress.
#[cfg(not(target_arch = "wasm32"))]
fn initial_progress() -> progress::Progress {
    progress::Progress::decode(&initial_share_source())
}

/// The raw share source (the `--share <blob>` arg), for restoring progress + console state (G5).
#[cfg(not(target_arch = "wasm32"))]
fn initial_share_source() -> String {
    let args: Vec<String> = std::env::args().collect();
    args.iter()
        .position(|a| a == "--share")
        .and_then(|i| args.get(i + 1))
        .cloned()
        .unwrap_or_default()
}

/// Web: restore progress from the URL hash fragment (its own `pg=` key).
#[cfg(target_arch = "wasm32")]
fn initial_progress() -> progress::Progress {
    progress::Progress::decode(&initial_share_source())
}

/// The raw share source (the URL hash), for restoring progress + console state (G5).
#[cfg(target_arch = "wasm32")]
fn initial_share_source() -> String {
    web_sys::window()
        .and_then(|w| w.location().hash().ok())
        .unwrap_or_default()
}

/// Seed for the world (the default + the headless demo).
const WORLD_SEED: u32 = 1337;

/// Short build id (git SHA), embedded at compile time by `build.rs`; shown in the HUD on
/// every platform so a screenshot/report says exactly which build it is.
const BUILD: &str = env!("BRICKMAP_BUILD");

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

/// How far (world units) to stream colossal structures (E18) around the camera. A little
/// beyond the chunk stream radius (≈160) so a giant resolves at the fog edge as you approach.
const STRUCTURE_RADIUS: f32 = 210.0;
/// Radius (world units) within which in-world inscriptions (E17) are streamed in around the camera.
const TEXT_RADIUS: f32 = 150.0;
/// Explored-map view (E10): chunks shown across the screen height (zoom), and the per-keypress
/// pan step (chunks) for arrow-key panning.
const MAP_ZOOM: f32 = 110.0;
const MAP_PAN_STEP: f32 = 8.0;
/// Structure-approach wobble (E18×E10): within this (wide) horizontal distance of a colossus the
/// vertex wobble lerps toward the giant's own extreme (heavy = low snap, stiller = higher snap).
const WOBBLE_APPROACH: f32 = 200.0;
const WOBBLE_HEAVY: f32 = 7.0; // strong PS1 warp right up against a "warping" giant
const WOBBLE_CRISP: f32 = 220.0; // a "stilling" giant calms the wobble — but not to pristine
/// **Pristine** zero-wobble, reserved for the rare ethereal/ink pockets — so those read as
/// special, untouched places (no quantization warp at all). Nothing else reaches it.
const WOBBLE_PRISTINE: f32 = 1500.0;
/// The ethereal colossi's tint (cool pale; the palette recolours it in the house look).
const COLOSSUS_COLOR: [f32; 3] = [0.62, 0.72, 0.9];
/// E18: the fallen-human giant's tint — a pale bone/stone, distinct from the cool tube-tech relics
/// (environmental art / monument, not gore).
const HUMAN_COLOR: [f32; 3] = [0.70, 0.67, 0.60];
/// The baked CC0 human surface points (E18), embedded so no OBJ ships. Decoded once at startup.
static HUMAN_POINTS_BLOB: &[u8] = include_bytes!("../assets/human_points.bin");
/// How many newly-entered colossi to generate per frame (the rest wait for later frames), so
/// crossing structure cells spreads the heavy point/mesh generation instead of hitching.
const STRUCTURE_GEN_BUDGET: u32 = 1;
/// Finished meshes to upload to the GPU per frame (bounds main-thread upload work, and
/// on web caps inline meshing alongside the time budget). M6.
const STREAM_UPLOADS: usize = 4;
/// Web only: max wall-time (ms) spent meshing chunks inline in one frame, so crossing a chunk
/// boundary (which queues a ring) spreads over frames instead of a single ~1–2 s hitch.
#[cfg(target_arch = "wasm32")]
const STREAM_MESH_BUDGET_MS: f32 = 5.0;

/// Ground-foliage density (E6): target splats per grass column (hash-thinned per
/// column for a natural look). Bounded by the stream radius + per-chunk grass area.
const FOLIAGE_DENSITY: u32 = 4;

/// Falling-sand tick (seconds) and how often a clump is dropped (E5).
const SIM_TICK: f32 = 0.05;
const SAND_INTERVAL: f32 = 0.14;
/// How often a water clump is dropped (E11) — slower than sand: water pools, so less is more.
const WATER_INTERVAL: f32 = 0.55;
/// Max precipitation drops spawned per frame at full weather intensity (E9). Tunable by eye.
const WEATHER_MAX_DROPS: u32 = 28;
/// M8a dynamic resolution: the frame-time budget (ms) the controller holds, and the most extra
/// internal-res divisor steps it may add under load. Conservative target (~30 fps) so it only
/// engages when genuinely slow; capable hardware never leaves the art base. Tunable.
const DYN_TARGET_MS: f32 = 33.0;
const DYN_MAX_EXTRA: u32 = 3;

/// M8a: one dynamic-resolution control step. Over budget (with margin) → coarsen one step; well
/// under → recover one step; else hold (hysteresis avoids oscillation). Pure + unit-testable.
fn dyn_resolution_step(extra: u32, frame_ms: f32, target_ms: f32, max_extra: u32) -> u32 {
    if frame_ms > target_ms * 1.15 && extra < max_extra {
        extra + 1
    } else if frame_ms < target_ms * 0.7 && extra > 0 {
        extra - 1
    } else {
        extra
    }
}
/// Max sand sections re-meshed per frame (synchronous; bounds the frame cost of a wide
/// sandfall — leftovers settle next frame).
const SAND_REMESH_BUDGET: usize = 3;

/// How high above the terrain the cinematic camera cruises.
const CRUISE_HEIGHT: f32 = 22.0;
/// Cruiser (E19): exit-to-walk is only allowed when piloting within this height of the ground
/// (so you have to land first); you re-enter when on foot within this horizontal distance.
const CRUISER_EXIT_ALT: f32 = 9.0;
const CRUISER_ENTER_DIST: f32 = 11.0;
/// D9: touch right-slider → yaw look-delta scale (per-frame; full deflection ≈ a brisk turn).
/// A pinned v1 default — on-device sensitivity is the deferred human feel-tuning.
const TOUCH_TURN: f32 = 7.0;
/// Auto-fly cruise speed (world units/second).
const AUTO_FLY_SPEED: f32 = 26.0;
/// Downward tilt of the auto-fly camera (radians).
const AUTO_FLY_PITCH: f32 = -0.22;
/// Foot auto-walk speed (G8c) — the `walk` nav steers the walker toward known sites at this rate.
const WALK_SPEED: f32 = 10.0;

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
/// Voxelise a relic (E18) and greedy-mesh it into drawable chunk instances — the **solid /
/// explorable** kind (vs the ethereal points). Buckets the solid voxels into 32³ sections
/// (multi-layer in y), meshes each with its body-internal neighbours so interior seams are
/// culled, and returns instances positioned in the world. `material` tints it.
pub(crate) fn relic_chunk_instances(
    placement: relic::Placement,
    material: world::BlockId,
) -> Vec<ChunkInstance> {
    let voxels = relic::relic_voxels(
        placement.pos,
        placement.voxel,
        placement.yaw,
        placement.seed,
    );
    voxels_to_instances(voxels, material)
}

/// The **solid / explorable fallen-human** giant (E18): topple + scale + yaw the baked human
/// points to world space ([`model::fallen_world`]), snap them onto a world voxel grid (a deduped
/// surface shell), and greedy-mesh it like a solid relic. Renders + dissolves-with-distance on the
/// same `structure_draws` path as the solid relics, so its quality matches theirs. *(Shell density
/// / scale read as the chunky low-fi look; finer fill is the eye-gated tuning.)*
fn human_solid_instances(
    points: &[Vec3],
    feet: Vec3,
    scale: f32,
    yaw: f32,
    material: world::BlockId,
) -> Vec<ChunkInstance> {
    let mut set: std::collections::HashSet<(i32, i32, i32)> = std::collections::HashSet::new();
    for w in model::fallen_world(points, feet, scale, yaw) {
        set.insert((w.x.floor() as i32, w.y.floor() as i32, w.z.floor() as i32));
    }
    let voxels: Vec<glam::IVec3> = set
        .into_iter()
        .map(|(x, y, z)| glam::IVec3::new(x, y, z))
        .collect();
    voxels_to_instances(voxels, material)
}

/// Greedy-mesh a set of **world-space solid voxels** into drawable chunk instances (E18). Buckets
/// into 32³ sections (multi-layer in y), meshes each with its body-internal neighbours so interior
/// seams are culled. Shared by the solid tube-tech relics + the solid fallen-human giant.
pub(crate) fn voxels_to_instances(
    voxels: Vec<glam::IVec3>,
    material: world::BlockId,
) -> Vec<ChunkInstance> {
    use std::collections::HashMap;
    let n = Section::SIZE as i32;
    let mut sections: HashMap<ChunkCoord, Section> = HashMap::new();
    for v in voxels {
        let cc = (v.x.div_euclid(n), v.y.div_euclid(n), v.z.div_euclid(n));
        let sec = sections.entry(cc).or_default();
        sec.set(
            v.x.rem_euclid(n) as u32,
            v.y.rem_euclid(n) as u32,
            v.z.rem_euclid(n) as u32,
            material,
        );
    }
    let s = Section::SIZE as f32;
    let mut out = Vec::new();
    for (&coord, sec) in &sections {
        let (cx, cy, cz) = coord;
        let nb = |dx, dy, dz| sections.get(&(cx + dx, cy + dy, cz + dz));
        let neighbors = Neighbors {
            faces: [
                nb(-1, 0, 0),
                nb(1, 0, 0),
                nb(0, -1, 0),
                nb(0, 1, 0),
                nb(0, 0, -1),
                nb(0, 0, 1),
            ],
        };
        let mesh = greedy_mesh_section_with(sec, &neighbors);
        if mesh.is_empty() {
            continue;
        }
        out.push(ChunkInstance {
            coord,
            origin: Vec3::new(cx as f32 * s, cy as f32 * s, cz as f32 * s),
            mesh,
            // Relics are drawn unconditionally (never cave-culled), so skip the connectivity
            // flood-fill — it was wasted work and part of the generation cost.
            graph: Default::default(),
            foliage: Vec::new(),
        });
    }
    out
}

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
        let n = Section::SIZE as i32;
        let lush = worldgen::lushness(cx * n + n / 2, cz * n + n / 2, WORLD_SEED);
        let density = (FOLIAGE_DENSITY as f32 * lush).round() as u32;
        let mut foliage = foliage::scatter(section, cx, cz, WORLD_SEED, density);
        foliage.extend(foliage::scatter_bushes(section, cx, cz, WORLD_SEED, lush));
        foliage.extend(foliage::scatter_trees(section, cx, cz, WORLD_SEED, lush));
        instances.push(ChunkInstance {
            coord: (cx, cy, cz),
            origin,
            mesh,
            graph: connectivity(section),
            foliage,
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
    // Android: log to logcat (env_logger output is invisible there). Mutually exclusive
    // with the desktop branch so we never double-init the global logger.
    #[cfg(target_os = "android")]
    {
        android_logger::init_once(
            android_logger::Config::default().with_max_level(log::LevelFilter::Info),
        );
    }
    #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
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
        /// Palette post-process (E10): enabled flag, palette index, colour count, dither.
        /// Default to the chosen house look: `bruise` (index 11), all 5 colours, heavy dither.
        static PALETTE_ON: Cell<bool> = const { Cell::new(true) };
        static PALETTE_INDEX: Cell<u32> = const { Cell::new(11) };
        static PALETTE_COUNT: Cell<u32> = const { Cell::new(5) };
        static PALETTE_DITHER: Cell<f32> = const { Cell::new(1.5) };
        /// Internal-resolution divisor (E10 pixel scale): 1 = native, higher = chunkier.
        /// Default 2 — the deliberate halftone is part of the house look (slider keeps 1–6).
        static PIXEL_SCALE: Cell<u32> = const { Cell::new(2) };
        /// Feature-toggle bitmask, one bit per switch. `0xBFF` = bits 0–11 on except sand (10),
        /// with melt (12) + sun (13) off — the dark, point-lit default; sand off (sim costs FPS).
        static TOGGLES: Cell<u32> = const { Cell::new(0xBFF) };
        /// Biome-driven auto mode (the new default). When on, the biome at the camera drives the
        /// look + mix and the manual controls are disabled on the page.
        static BIOME_MODE: Cell<bool> = const { Cell::new(true) };
        /// A seed change requested from the page, consumed by the app next frame.
        static PENDING_SEED: Cell<Option<u32>> = const { Cell::new(None) };
        /// The app's current share string, refreshed each HUD tick for "copy link".
        static CURRENT_SHARE: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
        /// The live doom-drone (E16). The page's Web Audio ScriptProcessor pulls blocks via
        /// `audio_block`; sliders nudge its params. `None` until `audio_init` (on first tap).
        static AUDIO: std::cell::RefCell<Option<crate::audio::Drone>> = const { std::cell::RefCell::new(None) };
    }

    /// (Re)create the drone for `seed` at the audio context's sample rate. Called from the
    /// page when the user enables audio (a user gesture, required to start a Web AudioContext)
    /// and whenever the world seed changes, so the dirge matches the world.
    #[wasm_bindgen]
    pub fn audio_init(seed: u32, sample_rate: f32) {
        AUDIO.with(|a| *a.borrow_mut() = Some(crate::audio::Drone::new(seed, sample_rate as u32)));
    }

    /// Render `frames` interleaved stereo samples (L, R, …) for the Web Audio callback.
    /// Silent (zeros) until `audio_init`.
    #[wasm_bindgen]
    pub fn audio_block(frames: usize) -> Vec<f32> {
        let mut out = vec![0.0f32; frames * 2];
        AUDIO.with(|a| {
            if let Some(d) = a.borrow_mut().as_mut() {
                d.render_block(&mut out);
            }
        });
        out
    }

    /// Live audio params from the page sliders.
    #[wasm_bindgen]
    pub fn set_audio_volume(v: f32) {
        AUDIO.with(|a| {
            if let Some(d) = a.borrow_mut().as_mut() {
                d.set_volume(v);
            }
        });
    }
    #[wasm_bindgen]
    pub fn set_audio_drive(m: f32) {
        AUDIO.with(|a| {
            if let Some(d) = a.borrow_mut().as_mut() {
                d.set_drive(m);
            }
        });
    }
    #[wasm_bindgen]
    pub fn set_audio_tone(t: f32) {
        AUDIO.with(|a| {
            if let Some(d) = a.borrow_mut().as_mut() {
                d.set_tone(t);
            }
        });
    }

    /// Reactive intensity (E16), driven each frame by the app (not a page slider): the camera's
    /// flight state nudges the drone's openness/swell.
    pub(crate) fn set_audio_intensity(x: f32) {
        AUDIO.with(|a| {
            if let Some(d) = a.borrow_mut().as_mut() {
                d.set_intensity(x);
            }
        });
    }

    /// Weather (E16×E9), driven each frame: precipitation intensity → darker, thicker drone.
    pub(crate) fn set_audio_weather(x: f32) {
        AUDIO.with(|a| {
            if let Some(d) = a.borrow_mut().as_mut() {
                d.set_weather(x);
            }
        });
    }

    /// Warp (E18), driven each frame: proximity to a max-wobble colossus → heavier, throbbing.
    pub(crate) fn set_audio_warp(x: f32) {
        AUDIO.with(|a| {
            if let Some(d) = a.borrow_mut().as_mut() {
                d.set_warp(x);
            }
        });
    }

    /// Ethereal (E10), driven each frame: pristine-pocket amount → airy, clean, shimmering drone.
    pub(crate) fn set_audio_ethereal(x: f32) {
        AUDIO.with(|a| {
            if let Some(d) = a.borrow_mut().as_mut() {
                d.set_ethereal(x);
            }
        });
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

    /// Configure the palette post-process (E10) from the page: which palette `index`, how
    /// many of its colours (`count`), the ordered-dither `spread`, and whether it's on.
    #[wasm_bindgen]
    pub fn set_palette(index: u32, count: u32, spread: f32, enabled: bool) {
        PALETTE_INDEX.with(|c| c.set(index));
        PALETTE_COUNT.with(|c| c.set(count.max(1)));
        PALETTE_DITHER.with(|c| c.set(spread));
        PALETTE_ON.with(|c| c.set(enabled));
    }

    pub(crate) fn palette() -> (bool, usize, u32, f32) {
        (
            PALETTE_ON.with(Cell::get),
            PALETTE_INDEX.with(Cell::get) as usize,
            PALETTE_COUNT.with(Cell::get),
            PALETTE_DITHER.with(Cell::get),
        )
    }

    /// Internal-resolution divisor (E10 pixel scale), set from the page.
    #[wasm_bindgen]
    pub fn set_pixel_scale(scale: u32) {
        PIXEL_SCALE.with(|c| c.set(scale.clamp(1, 8)));
    }
    pub(crate) fn pixel_scale() -> u32 {
        PIXEL_SCALE.with(Cell::get)
    }

    /// Biome auto mode on/off, set from the page (the master "mode" switch).
    #[wasm_bindgen]
    pub fn set_biome_mode(on: bool) {
        BIOME_MODE.with(|c| c.set(on));
    }
    pub(crate) fn biome_mode() -> bool {
        BIOME_MODE.with(Cell::get)
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
    fn autopilot_drift_meanders_not_circles() {
        // Quick-fix: drift must turn *both ways* over time (a wandering survey sweep), not hold a
        // near-constant rate into a loop. Run a couple of minutes of drift; the heading delta
        // should go both positive and negative.
        let (mut pos, mut angle) = (Vec3::new(0.0, 50.0, 0.0), 0.0f32);
        let (mut saw_left, mut saw_right) = (false, false);
        for i in 0..2000 {
            let t = i as f32 * 0.1;
            let (np, na) = autopilot_step(pos, angle, t, Some(console::Block::Drift), None, 7, 0.1);
            let d = na - angle;
            if d > 1e-4 {
                saw_right = true;
            }
            if d < -1e-4 {
                saw_left = true;
            }
            pos = np;
            angle = na;
        }
        assert!(
            saw_left && saw_right,
            "drift should turn both ways (meander), not circle one way"
        );
    }

    #[test]
    fn autopilot_step_advances_and_tracks_height() {
        // G8a: a drift step moves the ship forward (~AUTO_FLY_SPEED·dt horizontally) and pulls it
        // toward cruise height. Shared by the piloted autopilot + the autonomous away-ship.
        let start = Vec3::new(0.0, 200.0, 0.0);
        let (pos, _angle) =
            autopilot_step(start, 0.0, 0.0, Some(console::Block::Drift), None, 7, 0.1);
        let horiz = ((pos.x - start.x).powi(2) + (pos.z - start.z).powi(2)).sqrt();
        assert!(horiz > 1.0, "ship should travel horizontally: {horiz}");
        assert!(
            pos.y < start.y,
            "should descend toward cruise height from way up high"
        );
    }

    #[test]
    fn autopilot_seek_turns_toward_the_target() {
        // With a known site off to one side, `seek` should steer the heading toward it (the new
        // angle points more at the target than the old one did).
        let pos = Vec3::new(0.0, 50.0, 0.0);
        let target = Vec3::new(100.0, 50.0, 100.0); // bearing ≈ +45°
        let want = (target.z - pos.z).atan2(target.x - pos.x);
        let (_p, angle) = autopilot_step(
            pos,
            0.0,
            0.0,
            Some(console::Block::Seek),
            Some(target),
            7,
            0.1,
        );
        assert!(
            (angle - want).abs() < (0.0_f32 - want).abs(),
            "seek should reduce the heading error toward the target"
        );
    }

    #[test]
    fn autopilot_circle_keeps_turning() {
        // `circle` is a steady non-zero turn rate (loiter), unlike drift's mean-zero wander.
        let (_p, angle) = autopilot_step(
            Vec3::ZERO,
            0.0,
            0.0,
            Some(console::Block::Circle),
            None,
            7,
            0.2,
        );
        assert!(angle.abs() > 0.05, "circle should turn steadily: {angle}");
    }

    #[test]
    fn dyn_resolution_coarsens_under_load_and_recovers() {
        // Over budget → step up (chunkier); clamped at max.
        assert_eq!(dyn_resolution_step(0, 50.0, 33.0, 3), 1);
        assert_eq!(dyn_resolution_step(3, 50.0, 33.0, 3), 3); // clamped
                                                              // Well under budget → step back down toward the base; clamped at 0.
        assert_eq!(dyn_resolution_step(2, 10.0, 33.0, 3), 1);
        assert_eq!(dyn_resolution_step(0, 10.0, 33.0, 3), 0); // clamped
                                                              // In the hysteresis band → hold (no oscillation).
        assert_eq!(dyn_resolution_step(1, 33.0, 33.0, 3), 1);
    }

    #[test]
    fn adjust_fov_steps_and_clamps() {
        // E13: stepping changes the FOV by the given degrees; it clamps to the 20°–100° range.
        let base = 60f32.to_radians();
        let wider = adjust_fov(base, 10.0);
        assert!((wider.to_degrees() - 70.0).abs() < 1e-3);
        // Clamp at both ends.
        assert!((adjust_fov(base, -999.0).to_degrees() - 20.0).abs() < 1e-3);
        assert!((adjust_fov(base, 999.0).to_degrees() - 100.0).abs() < 1e-3);
    }

    #[test]
    fn walk_toward_steps_horizontally_and_stops_at_target() {
        // G8c foot auto-walk: a horizontal step toward the target, capped at the remaining
        // distance (no overshoot), leaving y to the collision/gravity pass.
        let pos = Vec3::new(0.0, 5.0, 0.0);
        let target = Vec3::new(100.0, 80.0, 0.0); // far + way above
        let next = walk_toward(pos, target, WALK_SPEED, 0.1);
        assert!(
            (next.x - WALK_SPEED * 0.1).abs() < 1e-3,
            "steps ~speed·dt along x"
        );
        assert_eq!(next.y, pos.y, "y is left to gravity/collision");
        // Close to the target → clamps to it (horizontally), no overshoot.
        let near = Vec3::new(99.5, 5.0, 0.0);
        let stop = walk_toward(near, target, WALK_SPEED, 1.0);
        assert!((stop.x - target.x).abs() < 1e-3);
    }

    #[test]
    #[ignore = "timing probe, run explicitly with --ignored --nocapture"]
    fn time_build_chunk_instance() {
        // Warm up, then time the full per-chunk web streaming cost (generate the chunk + its
        // 4 neighbours, greedy-mesh, scatter foliage, connectivity).
        for c in 0..8 {
            std::hint::black_box(build_chunk_instance((c, 0, 0), WORLD_SEED));
        }
        let n = 64;
        let t = std::time::Instant::now();
        for c in 0..n {
            std::hint::black_box(build_chunk_instance((c, 0, c * 3), WORLD_SEED));
        }
        let per = t.elapsed().as_secs_f64() * 1000.0 / n as f64;
        eprintln!("build_chunk_instance: {per:.2} ms/chunk  → {STREAM_UPLOADS}/frame = {:.1} ms/frame burst", per * STREAM_UPLOADS as f64);

        // Breakdown: a single section generate vs the mesh+foliage.
        let t = std::time::Instant::now();
        for c in 0..n {
            std::hint::black_box(worldgen::generate_section(c, c * 3, WORLD_SEED));
        }
        let gen = t.elapsed().as_secs_f64() * 1000.0 / n as f64;
        eprintln!("  generate_section: {gen:.2} ms each  (build regenerates 5: self + 4 neighbours = {:.1} ms)", gen * 5.0);

        // The fix: a shared section cache (the web path), so each section is generated once.
        let mut cache = SectionCache::new();
        for c in 0..8 {
            std::hint::black_box(build_chunk_instance_cached(
                (c, 0, 0),
                WORLD_SEED,
                &mut cache,
            ));
        }
        cache.clear();
        let t = std::time::Instant::now();
        for c in 0..n {
            std::hint::black_box(build_chunk_instance_cached(
                (c, 0, 0),
                WORLD_SEED,
                &mut cache,
            ));
        }
        let cached = t.elapsed().as_secs_f64() * 1000.0 / n as f64;
        eprintln!("  build_chunk_instance_cached (shared cache, web path): {cached:.2} ms/chunk  ({:.0}% of uncached)", cached / per * 100.0);

        // Colossal relic generation cost (the dip when a relic's cell enters range).
        let mk = |solid: bool, seed: u32| relic::Placement {
            pos: Vec3::new(0.0, 8.0, 0.0),
            yaw: 0.6,
            voxel: 1.4,
            seed,
            solid,
            human: false,
        };
        let relics = 20;
        let t = std::time::Instant::now();
        for c in 0..relics {
            std::hint::black_box(relic::relic_points(
                Vec3::ZERO,
                1.4,
                0.6,
                c as u32 | 1,
                COLOSSUS_COLOR,
            ));
        }
        eprintln!(
            "  relic_points (ethereal gen): {:.2} ms each",
            t.elapsed().as_secs_f64() * 1000.0 / relics as f64
        );
        let t = std::time::Instant::now();
        for c in 0..relics {
            std::hint::black_box(relic::relic_voxels(Vec3::ZERO, 1.4, 0.6, c as u32 | 1));
        }
        eprintln!(
            "  relic_voxels (solid voxelise): {:.2} ms each",
            t.elapsed().as_secs_f64() * 1000.0 / relics as f64
        );
        let t = std::time::Instant::now();
        for c in 0..relics {
            std::hint::black_box(relic_chunk_instances(
                mk(true, c as u32 | 1),
                world::BlockId(5),
            ));
        }
        eprintln!(
            "  relic_chunk_instances (solid gen, voxelise+mesh): {:.2} ms each",
            t.elapsed().as_secs_f64() * 1000.0 / relics as f64
        );
    }

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
