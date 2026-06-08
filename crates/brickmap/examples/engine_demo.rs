//! Engine-alone demo (M9 Phase 4) — the proof the engine/game boundary holds.
//!
//! This streams **flat, raw terrain** through a trivial [`WorldGen`] and renders it with
//! the brickmap engine, depending on **nothing but the `bm-*` crates** (via the facade).
//! There is no game content here: no biomes, colossi, inscriptions, doom drone, palette
//! set, or player — just the engine drawing a generated world. If this ever needs the
//! game crate to compile, the boundary has leaked.
//!
//! Run it: `cargo run -p brickmap --example engine_demo` (opens a window). It also serves
//! as a permanent engine smoke test — CI builds it, so the engine API can't silently grow
//! a game dependency.

use std::sync::Arc;

use brickmap::gfx::{ChunkInstance, State, Toggles};
use brickmap::mesh::{greedy_mesh_section_with, Neighbors};
use brickmap::scene::Camera;
use brickmap::visibility::connectivity;
use brickmap::world::{BlockId, ChunkCoord, Section};
use brickmap::WorldGen;
use glam::Vec3;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

/// The whole "game": a flat slab eight voxels deep. A real game implements this with its
/// terrain recipe; here it's the minimum that proves the engine renders *something* it was
/// handed without knowing what it is.
struct FlatGen;

const GROUND: BlockId = BlockId(1);
const FLOOR_DEPTH: u32 = 8;

impl WorldGen for FlatGen {
    fn generate(&self, _coord: ChunkCoord) -> Section {
        let mut s = Section::new();
        for z in 0..Section::SIZE {
            for x in 0..Section::SIZE {
                for y in 0..FLOOR_DEPTH {
                    s.set(x, y, z, GROUND);
                }
            }
        }
        s
    }
    fn solid(&self, _x: i32, y: i32, _z: i32) -> bool {
        (0..FLOOR_DEPTH as i32).contains(&y)
    }
}

/// Stream a small grid of chunks from `gen` and mesh each into the engine's draw contract
/// (empty foliage — no game splats). Pure engine API: generate → greedy-mesh → connectivity.
fn stream_terrain(gen: &dyn WorldGen) -> Vec<ChunkInstance> {
    let radius = 3;
    let s = Section::SIZE as f32;
    let mut out = Vec::new();
    for cz in -radius..=radius {
        for cx in -radius..=radius {
            let coord: ChunkCoord = (cx, 0, cz);
            let section = gen.generate(coord);
            out.push(ChunkInstance {
                coord,
                origin: Vec3::new(cx as f32 * s, 0.0, cz as f32 * s),
                mesh: greedy_mesh_section_with(&section, &Neighbors::NONE),
                graph: connectivity(&section),
                foliage: Vec::new(),
            });
        }
    }
    out
}

struct Demo {
    instances: Vec<ChunkInstance>,
    window: Option<Arc<Window>>,
    state: Option<State>,
    camera: Camera,
}

impl ApplicationHandler for Demo {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = Window::default_attributes().with_title("brickmap — engine demo (no game)");
        let window = Arc::new(event_loop.create_window(attrs).unwrap());
        let state = pollster::block_on(State::new(window.clone(), &self.instances));
        self.window = Some(window);
        self.state = Some(state);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(state) = self.state.as_mut() else {
            return;
        };
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => state.resize(size),
            WindowEvent::RedrawRequested => {
                let view_proj = self.camera.view_proj(state.aspect());
                state.render(
                    view_proj,
                    self.camera.position,
                    self.camera.right(),
                    Vec3::Y,
                    &[],
                    [0.0, 0.0],
                    Toggles::default(),
                    1.0,
                    0.0,
                    0.0, // murk (E9): clear air in the engine demo
                );
                if let Some(w) = self.window.as_ref() {
                    w.request_redraw();
                }
            }
            _ => {}
        }
    }
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let instances = stream_terrain(&FlatGen);
    log::info!(
        "engine demo: streamed {} chunks (zero game content)",
        instances.len()
    );
    let event_loop = EventLoop::new().unwrap();
    let mut demo = Demo {
        instances,
        window: None,
        state: None,
        camera: Camera::looking_at(Vec3::new(60.0, 48.0, 60.0), Vec3::new(0.0, 4.0, 0.0)),
    };
    event_loop.run_app(&mut demo).unwrap();
}
