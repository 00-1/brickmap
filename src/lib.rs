//! brickmap — cross-platform voxel rendering engine (kickoff spike).
//!
//! At this stage the crate is only the render spike described in
//! `docs/spikes.md`: clear the screen and draw one spinning cube via wgpu,
//! on desktop and in the browser (WASM). Voxel-specific code comes later, once
//! this cross-platform render path is proven. See `docs/design.md` and
//! `docs/architecture.md` for where this is heading.

use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
use winit::window::{Window, WindowId};

mod gfx;
pub mod mesh;
pub mod world;
use gfx::State;
use mesh::ChunkMesh;
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
        let Some(state) = self.state.as_mut() else {
            return;
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                state.resize(size);
                state.window().request_redraw();
            }
            WindowEvent::RedrawRequested => {
                // `render` handles lost/outdated/transient surfaces internally.
                state.render();
                // Continuously animate.
                state.window().request_redraw();
            }
            _ => {}
        }
    }
}

fn window_attributes() -> winit::window::WindowAttributes {
    let attrs = Window::default_attributes()
        .with_title("brickmap — spike")
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

    let mut app = App {
        state: None,
        proxy: Some(event_loop.create_proxy()),
        mesh: mesh::mesh_section(&demo_section()),
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
