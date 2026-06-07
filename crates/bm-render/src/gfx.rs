//! Minimal wgpu render state for the cross-platform spike.
//!
//! This is intentionally a single flat module. None of it is meant to survive
//! into the real engine unchanged — see `docs/architecture.md` for the planned
//! `platform` / `render` / `world` / `mesh` crate split. The spike exists only
//! to prove the wgpu render path compiles and runs on desktop **and** the web.

use std::collections::HashMap;
use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;
use winit::window::Window;

use crate::foliage::SplatInstance;
use crate::mesh::{pack, ChunkMesh};
use crate::particles::{ParticleInstance, CUBE_INDICES, CUBE_POSITIONS};
use crate::post::Bloom;
use crate::scene::Frustum;
use crate::textures::{material_mip_chain, mip_levels, LAYERS, TILE};
use crate::visibility::{visible_set, FaceGraph};
use crate::world::ChunkCoord;

/// One mesh instance to draw: a chunk mesh (chunk-local) at a world `origin`.
#[derive(Clone)]
pub struct ChunkInstance {
    pub coord: ChunkCoord,
    pub origin: Vec3,
    pub mesh: ChunkMesh,
    /// Face-connectivity graph for visibility-graph culling (M5).
    pub graph: FaceGraph,
    /// World-positioned ground-foliage splats for this chunk (E6).
    pub foliage: Vec<SplatInstance>,
}

/// Live on/off switches for renderer features, so we can A/B their cost + look
/// (D6). Default: everything on. Indexed `0..N` for the keyboard / web UI.
#[derive(Copy, Clone)]
pub struct Toggles {
    pub frustum_cull: bool,
    pub cave_cull: bool,
    pub sky: bool,
    pub particles: bool,
    pub bloom: bool,
    pub fog: bool,
    pub ao: bool,
    pub block_light: bool,
    pub emissive: bool,
    pub relief: bool,
    pub sand: bool,
    pub foliage: bool,
    /// Distance-dissolve (M7): stipple distant terrain/foliage into a pixel haze. Opt-in
    /// (default off) since it's a strong stylistic choice.
    pub melt: bool,
    /// Directional sun (E3). On by default; turn it off to light the world only by the
    /// in-world emissive point lights (crystals) + dim ambient — a dark, point-lit mood.
    pub sun: bool,
    /// "Ink" blueprint grid (E10): darken thin lines along voxel edges so the cube lattice
    /// reads as drawn-on ink. Opt-in (default off) — a strong stylistic overlay.
    pub ink: bool,
}

/// Short labels for the toggles, in index order (HUD + web checkboxes).
pub const TOGGLE_LABELS: [&str; 15] = [
    "cull", "cave", "sky", "sparks", "bloom", "fog", "ao", "light", "glow", "relief", "sand",
    "foliage", "melt", "sun", "ink",
];

impl Default for Toggles {
    fn default() -> Self {
        Toggles {
            frustum_cull: true,
            cave_cull: true,
            sky: true,
            particles: true,
            bloom: true,
            fog: true,
            ao: true,
            block_light: true,
            emissive: true,
            relief: true,
            sand: false, // off by default — the falling-sand sim's re-meshing costs FPS
            foliage: true,
            melt: false, // opt-in (M7 distance dissolve)
            sun: false,  // default to the dark, point-lit mood (the resolved identity)
            ink: false,  // opt-in (E10 blueprint-grid overlay)
        }
    }
}

impl Toggles {
    pub fn get(&self, i: usize) -> bool {
        [
            self.frustum_cull,
            self.cave_cull,
            self.sky,
            self.particles,
            self.bloom,
            self.fog,
            self.ao,
            self.block_light,
            self.emissive,
            self.relief,
            self.sand,
            self.foliage,
            self.melt,
            self.sun,
            self.ink,
        ][i]
    }

    pub fn set(&mut self, i: usize, v: bool) {
        match i {
            0 => self.frustum_cull = v,
            1 => self.cave_cull = v,
            2 => self.sky = v,
            3 => self.particles = v,
            4 => self.bloom = v,
            5 => self.fog = v,
            6 => self.ao = v,
            7 => self.block_light = v,
            8 => self.emissive = v,
            9 => self.relief = v,
            10 => self.sand = v,
            11 => self.foliage = v,
            12 => self.melt = v,
            13 => self.sun = v,
            14 => self.ink = v,
            _ => {}
        }
    }

    pub fn toggle(&mut self, i: usize) {
        self.set(i, !self.get(i));
    }

    /// Pack the switches into a bitmask (bit `i` = `TOGGLE_LABELS[i]`), for the share
    /// codec and the web bridge.
    pub fn to_mask(self) -> u32 {
        let mut m = 0u32;
        for i in 0..TOGGLE_LABELS.len() {
            if self.get(i) {
                m |= 1 << i;
            }
        }
        m
    }

    /// Rebuild the switches from a bitmask (inverse of [`to_mask`](Self::to_mask)).
    pub fn from_mask(m: u32) -> Toggles {
        let mut t = Toggles::default();
        for i in 0..TOGGLE_LABELS.len() {
            t.set(i, m & (1 << i) != 0);
        }
        t
    }

    /// Compact readout of what's currently OFF, for the HUD (empty when all on).
    pub fn off_summary(&self) -> String {
        let off: Vec<&str> = TOGGLE_LABELS
            .iter()
            .enumerate()
            .filter(|(i, _)| !self.get(*i))
            .map(|(_, l)| *l)
            .collect();
        if off.is_empty() {
            String::new()
        } else {
            format!(" · off: {}", off.join(" "))
        }
    }
}

/// Per-frame draw counts, surfaced to the perf HUD (M5).
#[derive(Copy, Clone, Default)]
pub struct DrawStats {
    pub drawn_chunks: u32,
    pub total_chunks: u32,
    pub triangles: u32,
    pub particles: u32,
    pub splats: u32,
    /// Solid colossal-relic meshes drawn this frame (E18) — for the HUD, so the mesh↔points
    /// dissolve is visible (0 = all in-range relics are currently points).
    pub relics: u32,
}

/// Vertex buffer layout for the **packed** face vertex: two `u32`s per vertex (8
/// bytes, design §9–10) — word 0 = pos/dir/material/ao, word 1 = block light.
/// Unpacked in the shader.
const CHUNK_VERTEX_LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
    array_stride: (2 * std::mem::size_of::<u32>()) as wgpu::BufferAddress,
    step_mode: wgpu::VertexStepMode::Vertex,
    attributes: &wgpu::vertex_attr_array![0 => Uint32, 1 => Uint32],
};

/// Per-frame globals (bind group 0): the camera, the material palette, and the live
/// aesthetic dials (D2).
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Globals {
    view_proj: [[f32; 4]; 4],
    palette: [[f32; 4]; PALETTE.len()],
    /// x = wobble snap, y = colour steps, z = fog start, w = fog end.
    params: [f32; 4],
    /// Camera world position (xyz); w unused. For distance fog.
    camera_pos: [f32; 4],
    /// Fog / sky colour (rgb); w unused.
    fog_color: [f32; 4],
    /// Feature flags (0/1): x = AO, y = block light, z = emissive; w spare.
    flags: [f32; 4],
    /// Camera right vector (xyz) for billboarding splats; w = wind time (seconds).
    cam_right: [f32; 4],
    /// Camera up vector (xyz) for billboarding splats; w spare.
    cam_up: [f32; 4],
    /// A *lagged* camera position (xyz) that eases behind the real one, so point-splats
    /// recede from it with inertia (drift out of the way at their own pace) rather than
    /// tracking the camera rigidly. w unused. Only `splat.wgsl` reads it.
    lag_camera: [f32; 4],
}

/// Per-chunk uniform (bind group 1): the chunk's world origin (xyz; w unused).
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct ChunkUniform {
    origin: [f32; 4],
}

/// Debug material palette, indexed by `material` id (M4 replaces it with a texture
/// array). Index 0 / unknown render magenta so mistakes are loud.
const PALETTE: [[f32; 4]; 8] = [
    [0.95, 0.10, 0.95, 1.0], // 0: unused/unknown
    [0.55, 0.55, 0.58, 1.0], // 1: stone
    [0.55, 0.40, 0.25, 1.0], // 2: dirt
    [0.40, 0.70, 0.35, 1.0], // 3: grass
    [0.80, 0.78, 0.65, 1.0], // 4: sand
    [0.92, 0.94, 0.98, 1.0], // 5: snow
    [0.45, 0.95, 1.00, 1.0], // 6: crystal (emissive)
    [0.18, 0.38, 0.62, 1.0], // 7: water
];

// Fallback surface size used when the windowing layer reports a degenerate size
// (browsers can report 0x0 before the canvas is laid out). Should match the
// canvas in web/index.html / `INITIAL_SIZE` in lib.rs.
const FALLBACK_SIZE: (u32, u32) = (960, 720);

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const CLEAR_COLOR: wgpu::Color = wgpu::Color {
    r: 0.02,
    g: 0.03,
    b: 0.06,
    a: 1.0,
};

/// Distance fog (world units): terrain fades to the horizon colour between these, so
/// the streaming load edge (~5 chunks ≈ 160 u) dissolves instead of popping in.
const FOG_START: f32 = 72.0;
const FOG_END: f32 = 176.0;
/// Fog colour — the sky's **horizon** band (see `sky.wgsl`) so distant terrain melts
/// into the horizon rather than a flat background.
const FOG_COLOR: [f32; 4] = [0.30, 0.33, 0.42, 1.0];

pub struct State {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,

    pipeline: wgpu::RenderPipeline,
    /// Fullscreen sky-gradient pipeline (no bindings); drawn first behind everything.
    sky_pipeline: wgpu::RenderPipeline,
    /// Chunk draws keyed by chunk coordinate, so streaming can add/remove them at
    /// runtime. Each carries its packed buffers, world AABB, and per-chunk origin.
    draws: HashMap<ChunkCoord, ChunkDraw>,
    /// Kept so new chunk draws can be built after construction (streaming).
    chunk_bind_group_layout: wgpu::BindGroupLayout,

    globals_buffer: wgpu::Buffer,
    globals_bind_group: wgpu::BindGroup,
    /// Procedural material texture array + sampler (group 2). Constant for all chunks.
    material_bind_group: wgpu::BindGroup,
    depth_view: wgpu::TextureView,
    /// Offscreen colour target: the scene renders here, then bloom composites it to
    /// the surface.
    scene_view: wgpu::TextureView,
    bloom: Bloom,

    /// Palette post-process (E10): maps the finished image onto a small, configurable
    /// colour ramp with ordered dithering. When `palette_on` is false the pass is skipped
    /// and bloom composites straight to the surface (unchanged look). `palette_view` is the
    /// intermediate bloom lands in before the palette maps it to the surface; its bind group
    /// is rebuilt on resize.
    palette: crate::palette::PalettePass,
    /// Low-res *internal* buffer the scene + post chain render into; the palette pass then
    /// presents it to the surface with nearest sampling, upscaling by `pixel_scale` (E10).
    palette_view: wgpu::TextureView,
    palette_bind_group: wgpu::BindGroup,
    palette_on: bool,
    /// Internal-resolution divisor (1 = native; 2,3,4… = chunkier + cheaper). The signature
    /// halftone made deliberate, and the biggest single perf dial.
    pixel_scale: u32,

    /// Foliage splats (E6): an instanced billboard pipeline; per-chunk instance buffers
    /// live on each `ChunkDraw`. Shares the globals (group 0).
    splat_pipeline: wgpu::RenderPipeline,

    // Colossal structures (E18): all in-range giants' points in one growable instance buffer,
    // drawn with the splat pipeline. Updated from the app when the in-range set changes.
    structure_splats: wgpu::Buffer,
    structure_count: u32,
    structure_capacity: usize,
    /// Solid colossal structures (E18): extra greedy-meshed chunk draws (the giants you can
    /// land on), drawn with the terrain pipeline but kept out of the streaming `draws` map.
    structure_draws: Vec<ChunkDraw>,

    // Drifting wisp creatures (E15): a small swarm of points re-emitted and rewritten *every
    // frame* as they move, drawn with the splat pipeline. Separate from `structure_splats`
    // (which only changes when the in-range giant set does).
    creature_splats: wgpu::Buffer,
    creature_count: u32,
    creature_capacity: usize,

    /// Eased camera position driving the splat recession (E18 polish): lags the real camera
    /// so points drift out of the way with inertia. `lag_time` is the previous frame's clock
    /// for the frame-rate-independent ease; a negative value means "not yet initialised".
    lag_camera: Vec3,
    lag_time: f32,

    // Particles (E2): an instanced emissive-cube pipeline + a growable instance buffer.
    particle_pipeline: wgpu::RenderPipeline,
    cube_vertex_buffer: wgpu::Buffer,
    cube_index_buffer: wgpu::Buffer,
    particle_instances: wgpu::Buffer,
    particle_capacity: usize,

    /// Frame counter, used to throttle the stats log.
    frame_count: u64,
    /// Last frame's draw counts, for the perf HUD (M5).
    last_stats: DrawStats,
    /// Start time, for the foliage wind animation (E6).
    start: web_time::Instant,
    /// In-engine text overlay (HUD), drawn the same way on every platform.
    hud: crate::hud::HudOverlay,
    /// In-world text (E17): seed-placed glowing inscriptions, drawn as camera-facing billboards
    /// inside the scene pass. Rebuilt by the app when the in-range set changes.
    text: crate::text::WorldText,
    /// Explored-world map overlay (E10): a fullscreen biome map drawn over the frame when open.
    map: crate::map::MapView,
    map_active: bool,
    /// Space cruiser (E19): a polygonal ship drawn over the palette in true colour, with its own
    /// (surface-res) depth buffer. Shown parked while you're on foot.
    ship: crate::ship::ShipRenderer,
    ship_depth: wgpu::TextureView,
    ship_active: bool,
    ship_pos: Vec3,
    ship_yaw: f32,

    // The window must outlive the surface; keep an Arc so `Surface<'static>` is sound.
    window: Arc<Window>,
}

/// GPU buffers for one chunk mesh, plus its world AABB and per-chunk origin binding.
struct ChunkDraw {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    aabb_min: Vec3,
    aabb_max: Vec3,
    origin_bind_group: wgpu::BindGroup,
    graph: FaceGraph,
    /// Foliage splats for this chunk (E6): instance buffer + count (empty → `None`).
    foliage: Option<(wgpu::Buffer, u32)>,
}

impl State {
    pub async fn new(window: Arc<Window>, instances: &[ChunkInstance]) -> State {
        let mut size = window.inner_size();
        // Browsers can report a 0x0 (or 1x1) canvas before layout. Don't configure a
        // tiny surface — the page would stretch that single pixel across the whole
        // canvas. Fall back to a real size instead.
        if size.width <= 1 {
            size.width = FALLBACK_SIZE.0;
        }
        if size.height <= 1 {
            size.height = FALLBACK_SIZE.1;
        }

        // On the web, detect WebGPU vs WebGL2 so the GLES fallback actually kicks in
        // on browsers that expose `navigator.gpu` but can't create an adapter.
        #[cfg(target_arch = "wasm32")]
        let instance = wgpu::util::new_instance_with_webgpu_detection(
            wgpu::InstanceDescriptor::new_without_display_handle(),
        )
        .await;
        #[cfg(not(target_arch = "wasm32"))]
        let instance = wgpu::Instance::default();

        let surface = instance
            .create_surface(window.clone())
            .expect("failed to create surface");

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("no suitable GPU adapter found");

        log::info!("using adapter: {:?}", adapter.get_info());

        // Weak-hardware-first: ask only for the lowest-common-denominator limits so
        // the same code runs on integrated GPUs, mobile, and WebGL2.
        let required_limits = if cfg!(target_arch = "wasm32") {
            wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits())
        } else {
            wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits())
        };

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("brickmap-device"),
                required_features: wgpu::Features::empty(),
                required_limits,
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            })
            .await
            .expect("failed to create device");

        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

        // A uniform-buffer bind group layout used for both globals and per-chunk.
        let uniform_bgl = |label, visibility| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some(label),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            })
        };
        let globals_bgl = uniform_bgl(
            "globals-bgl",
            wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
        );
        let chunk_bgl = uniform_bgl("chunk-bgl", wgpu::ShaderStages::VERTEX);

        let globals_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("globals"),
            size: std::mem::size_of::<Globals>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let globals_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("globals-bg"),
            layout: &globals_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: globals_buffer.as_entire_binding(),
            }],
        });

        // --- Procedural material texture array (group 2, M4) ---
        let material_bgl = material_bind_group_layout(&device);
        let material_bind_group = build_material_bind_group(&device, &queue, &material_bgl);

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pipeline-layout"),
            bind_group_layouts: &[Some(&globals_bgl), Some(&chunk_bgl), Some(&material_bgl)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("chunk-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[CHUNK_VERTEX_LAYOUT],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                // Culling is off for the spike so winding mistakes can't hide the
                // cube. The real mesher will emit consistent CCW faces and cull.
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let sky_pipeline = build_sky_pipeline(&device, config.format);

        let mut draws: HashMap<ChunkCoord, ChunkDraw> = HashMap::new();
        for inst in instances {
            if let Some(draw) = build_chunk_draw(&device, &chunk_bgl, inst, false) {
                draws.insert(inst.coord, draw);
            }
        }

        // --- Particles (E2): instanced emissive cubes, sharing the globals group ---
        let particle_shader = device.create_shader_module(wgpu::include_wgsl!("particles.wgsl"));
        let particle_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("particle-pipeline-layout"),
                bind_group_layouts: &[Some(&globals_bgl)],
                immediate_size: 0,
            });
        let cube_layout = wgpu::VertexBufferLayout {
            array_stride: (3 * std::mem::size_of::<f32>()) as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![0 => Float32x3],
        };
        let instance_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<ParticleInstance>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &wgpu::vertex_attr_array![1 => Float32x3, 2 => Float32, 3 => Float32x3],
        };
        let particle_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("particle-pipeline"),
            layout: Some(&particle_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &particle_shader,
                entry_point: Some("vs_main"),
                buffers: &[cube_layout, instance_layout],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &particle_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let cube_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("cube-vertices"),
            contents: bytemuck::bytes_of(&CUBE_POSITIONS),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let cube_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("cube-indices"),
            contents: bytemuck::bytes_of(&CUBE_INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });
        let particle_capacity = 2048usize;
        let particle_instances = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("particle-instances"),
            size: (particle_capacity * std::mem::size_of::<ParticleInstance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let structure_capacity = 16384usize;
        let structure_splats = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("structure-splats"),
            size: (structure_capacity * std::mem::size_of::<SplatInstance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let creature_capacity = 1024usize;
        let creature_splats = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("creature-splats"),
            size: (creature_capacity * std::mem::size_of::<SplatInstance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // --- Foliage splats (E6): instanced billboards sharing the globals group ---
        let splat_pipeline = build_splat_pipeline(&device, &globals_bgl, config.format);

        // Internal render resolution starts at native (pixel_scale = 1).
        let pixel_scale = 1u32;
        let (iw, ih) = (config.width.max(1), config.height.max(1));
        let depth_view = create_depth_view(&device, iw, ih);
        let scene_view = create_scene_view(&device, config.format, iw, ih);
        let bloom = Bloom::new(&device, config.format, iw, ih);
        let hud = crate::hud::HudOverlay::new(&device, config.format);
        let text = crate::text::WorldText::new(&device, config.format, DEPTH_FORMAT, &globals_bgl);
        let map = crate::map::MapView::new(&device, config.format);
        let ship = crate::ship::ShipRenderer::new(&device, config.format, DEPTH_FORMAT);
        // Ship depth is full surface resolution (it draws after the palette upscale).
        let ship_depth = create_depth_view(&device, config.width.max(1), config.height.max(1));

        // Palette post-process (E10), off by default so the live preview is unchanged until
        // a palette is chosen. Seeded with the engine's neutral default ramp; the game pushes
        // its own curated/biome ramp via `set_palette_colors`.
        let palette = crate::palette::PalettePass::new(&device, config.format);
        let palette_on = false;
        palette.set_colors(
            &queue,
            crate::palette::DEFAULT_RAMP,
            crate::palette::DEFAULT_RAMP.len() as u32,
            1.0,
            palette_on,
        );
        let palette_view = create_scene_view(&device, config.format, iw, ih);
        let palette_bind_group = palette.make_bind_group(&device, &palette_view);

        State {
            surface,
            device,
            queue,
            config,
            size,
            pipeline,
            sky_pipeline,
            draws,
            chunk_bind_group_layout: chunk_bgl,
            globals_buffer,
            globals_bind_group,
            material_bind_group,
            depth_view,
            scene_view,
            bloom,
            palette,
            palette_view,
            palette_bind_group,
            palette_on,
            pixel_scale,
            splat_pipeline,
            structure_splats,
            structure_count: 0,
            structure_capacity,
            structure_draws: Vec::new(),
            creature_splats,
            creature_count: 0,
            creature_capacity,
            lag_camera: Vec3::ZERO,
            lag_time: -1.0,
            particle_pipeline,
            cube_vertex_buffer,
            cube_index_buffer,
            particle_instances,
            particle_capacity,
            frame_count: 0,
            last_stats: DrawStats::default(),
            start: web_time::Instant::now(),
            hud,
            text,
            map,
            map_active: false,
            ship,
            ship_depth,
            ship_active: false,
            ship_pos: Vec3::ZERO,
            ship_yaw: 0.0,
            window,
        }
    }

    /// Update the in-engine text overlay (HUD), drawn each frame over the finished image.
    /// Word-wraps to the current surface width (at the HUD's font scale) so long status + biome
    /// lines break at the screen edge instead of running off it.
    pub fn set_hud(&mut self, text: &str) {
        let scale = (self.config.height / 360).max(2);
        let max_cols = ((self.config.width.saturating_sub(16)) / (8 * scale)).max(12) as usize;
        let wrapped = crate::hud::wrap(text, max_cols);
        self.hud.set_text(&self.device, &self.queue, &wrapped);
    }

    /// Replace the in-world inscriptions (E17): clears the current labels and rebuilds them
    /// from `labels` = `(text, script, center, world_height, color)`. Called by the app only
    /// when the in-range set changes (not every frame), so the per-label texture upload is rare.
    pub fn set_text_labels(
        &mut self,
        labels: &[(String, crate::text::Script, Vec3, f32, [f32; 3])],
    ) {
        self.text.clear();
        for (s, script, center, height, color) in labels {
            self.text.add_script(
                &self.device,
                &self.queue,
                s,
                *script,
                *center,
                *height,
                *color,
            );
        }
    }

    /// Replace the colossal-structure point set (E18). Grows the instance buffer if needed.
    /// Called from the app only when the in-range set of giants changes (not every frame).
    pub fn set_structure_points(&mut self, points: &[SplatInstance]) {
        self.structure_count = points.len() as u32;
        if points.is_empty() {
            return;
        }
        if points.len() > self.structure_capacity {
            self.structure_capacity = points.len().next_power_of_two();
            self.structure_splats = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("structure-splats"),
                size: (self.structure_capacity * std::mem::size_of::<SplatInstance>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        self.queue
            .write_buffer(&self.structure_splats, 0, bytemuck::cast_slice(points));
    }

    /// Replace the drifting wisp-creature points (E15). Unlike the giants, this is rewritten
    /// every frame as the swarm moves, so it's a small, cheap upload. Grows on demand.
    pub fn set_creature_points(&mut self, points: &[SplatInstance]) {
        self.creature_count = points.len() as u32;
        if points.is_empty() {
            return;
        }
        if points.len() > self.creature_capacity {
            self.creature_capacity = points.len().next_power_of_two();
            self.creature_splats = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("creature-splats"),
                size: (self.creature_capacity * std::mem::size_of::<SplatInstance>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        self.queue
            .write_buffer(&self.creature_splats, 0, bytemuck::cast_slice(points));
    }

    /// Replace the solid colossal-structure meshes (E18) — greedy-meshed giant instances drawn
    /// with the terrain pipeline. Rebuilt by the app when the in-range set changes.
    pub fn set_structure_meshes(&mut self, instances: &[ChunkInstance]) {
        self.structure_draws = instances
            .iter()
            .filter_map(|inst| {
                build_chunk_draw(&self.device, &self.chunk_bind_group_layout, inst, true)
            })
            .collect();
    }

    /// Explored-world map (E10): whether the fullscreen map overlay is shown this frame.
    pub fn set_map_active(&mut self, active: bool) {
        self.map_active = active;
    }
    /// Space cruiser (E19): whether to draw the parked ship this frame, and where/at what yaw.
    pub fn set_ship(&mut self, active: bool, pos: Vec3, yaw: f32) {
        self.ship_active = active;
        self.ship_pos = pos;
        self.ship_yaw = yaw;
    }
    /// Upload a fresh chunk-image for the map (RGBA, one texel per chunk). Only when it grows.
    pub fn set_map_image(&mut self, w: u32, h: u32, rgba: &[u8]) {
        self.map.set_image(&self.device, &self.queue, w, h, rgba);
    }
    /// Update the map's pan/zoom/user uniform (cheap, every frame the map is open).
    pub fn set_map_uniform(&self, u: &crate::map::MapUniform) {
        self.map.set_uniform(&self.queue, u);
    }

    /// Set the palette from an explicit colour ramp (a curated entry the game resolved, a
    /// biome-blended ramp, or the engine default). `on == false` makes the pass a passthrough.
    /// This is the engine's whole palette seam — it knows no curated set (M9).
    pub fn set_palette_colors(&mut self, colors: &[[f32; 3]], count: u32, dither: f32, on: bool) {
        self.palette_on = on;
        self.palette
            .set_colors(&self.queue, colors, count, dither, on);
    }

    /// Last frame's draw counts, for the perf HUD.
    pub fn stats(&self) -> DrawStats {
        self.last_stats
    }

    /// Current surface aspect ratio (width / height).
    pub fn aspect(&self) -> f32 {
        self.config.width as f32 / self.config.height as f32
    }

    /// Add or replace a chunk's GPU buffers (streaming). Empty meshes are dropped.
    pub fn upload_chunk(&mut self, instance: &ChunkInstance) {
        match build_chunk_draw(&self.device, &self.chunk_bind_group_layout, instance, false) {
            Some(draw) => {
                self.draws.insert(instance.coord, draw);
            }
            None => {
                self.draws.remove(&instance.coord);
            }
        }
    }

    /// Drop a chunk's GPU buffers (streaming).
    pub fn remove_chunk(&mut self, coord: ChunkCoord) {
        self.draws.remove(&coord);
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 {
            return;
        }
        self.size = new_size;
        self.config.width = new_size.width;
        self.config.height = new_size.height;
        self.surface.configure(&self.device, &self.config);
        self.rebuild_targets();
    }

    /// Internal render size = surface size / pixel scale (≥ 1px).
    fn internal_size(&self) -> (u32, u32) {
        let s = self.pixel_scale.max(1);
        (
            (self.config.width / s).max(1),
            (self.config.height / s).max(1),
        )
    }

    /// (Re)create every internal-resolution target (scene/depth/bloom/palette) + the palette
    /// bind group. Called on surface resize and on pixel-scale change.
    fn rebuild_targets(&mut self) {
        let (iw, ih) = self.internal_size();
        self.depth_view = create_depth_view(&self.device, iw, ih);
        self.scene_view = create_scene_view(&self.device, self.config.format, iw, ih);
        self.bloom.resize(&self.device, iw, ih);
        self.palette_view = create_scene_view(&self.device, self.config.format, iw, ih);
        self.palette_bind_group = self
            .palette
            .make_bind_group(&self.device, &self.palette_view);
        // The ship draws at full surface resolution (after the palette upscale), so its depth
        // buffer tracks the surface size, not the internal one.
        self.ship_depth = create_depth_view(
            &self.device,
            self.config.width.max(1),
            self.config.height.max(1),
        );
    }

    /// Set the internal-resolution divisor (1 = native; higher = chunkier + cheaper). Rebuilds
    /// the internal targets when it changes (E10 pixel scale / halftone dial).
    pub fn set_pixel_scale(&mut self, scale: u32) {
        let scale = scale.clamp(1, 8);
        if scale != self.pixel_scale {
            self.pixel_scale = scale;
            self.rebuild_targets();
        }
    }

    /// Reconfigure at the current size — used to recover a lost/outdated surface.
    pub fn reconfigure(&mut self) {
        self.resize(self.size);
    }

    /// Draw the scene from the given view-projection, plus the live particles. Chunk
    /// vertices are packed and chunk-local; the shader adds each chunk's world origin.
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        view_proj: Mat4,
        camera_pos: Vec3,
        cam_right: Vec3,
        cam_up: Vec3,
        particles: &[ParticleInstance],
        aesthetic: [f32; 2],
        toggles: Toggles,
        // Directional-sun amount 0..1 (biome mode passes a blended level; manual mode passes
        // 0/1 from the `sun` toggle) so the point-lit↔sunlit mood crossfades.
        sun: f32,
        // "Ink" blueprint-grid amount 0..1 (0/1 from the toggle in manual mode; a smooth biome
        // ethereal-pocket fade in biome mode). Carried in fog_color.w.
        ink: f32,
    ) {
        let time = self.start.elapsed().as_secs_f32();
        // Particles off → draw none (the system keeps simulating; cheap).
        let particles: &[ParticleInstance] = if toggles.particles { particles } else { &[] };
        // Fog off → push the band out past anything visible.
        let (fog_start, fog_end) = if toggles.fog {
            (FOG_START, FOG_END)
        } else {
            (1.0e9, 1.0e9 + 1.0)
        };
        // Ease the lagged camera toward the real one (frame-rate-independent exponential
        // smoothing). The recession field is driven by this, so the faster you move the
        // further it trails — a slow careful approach is responsive, a fast fly-by leaves
        // the foliage to drift out lazily in your wake. ~0.35 s time constant.
        if self.lag_time < 0.0 {
            self.lag_camera = camera_pos;
        } else {
            let dt = (time - self.lag_time).clamp(0.0, 0.25);
            let k = 1.0 - (-dt / 0.35).exp();
            self.lag_camera += (camera_pos - self.lag_camera) * k;
        }
        self.lag_time = time;

        let flag = |b: bool| if b { 1.0 } else { 0.0 };
        let globals = Globals {
            view_proj: view_proj.to_cols_array_2d(),
            palette: PALETTE,
            params: [aesthetic[0], aesthetic[1], fog_start, fog_end],
            // w = directional-sun flag (0 = off → point-lit only).
            camera_pos: [
                camera_pos.x,
                camera_pos.y,
                camera_pos.z,
                sun.clamp(0.0, 1.0),
            ],
            // w carries the "ink" blueprint-grid amount (E10) for the chunk shader.
            fog_color: [
                FOG_COLOR[0],
                FOG_COLOR[1],
                FOG_COLOR[2],
                ink.clamp(0.0, 1.0),
            ],
            flags: [
                flag(toggles.ao),
                flag(toggles.block_light),
                flag(toggles.emissive),
                flag(toggles.relief),
            ],
            cam_right: [cam_right.x, cam_right.y, cam_right.z, time],
            cam_up: [cam_up.x, cam_up.y, cam_up.z, flag(toggles.melt)],
            lag_camera: [self.lag_camera.x, self.lag_camera.y, self.lag_camera.z, 0.0],
        };
        self.queue
            .write_buffer(&self.globals_buffer, 0, bytemuck::bytes_of(&globals));

        // Upload particle instances (grow the buffer if needed) before the pass.
        if !particles.is_empty() {
            if particles.len() > self.particle_capacity {
                self.particle_capacity = particles.len().next_power_of_two();
                self.particle_instances = self.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("particle-instances"),
                    size: (self.particle_capacity * std::mem::size_of::<ParticleInstance>()) as u64,
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
            }
            self.queue
                .write_buffer(&self.particle_instances, 0, bytemuck::cast_slice(particles));
        }

        use wgpu::CurrentSurfaceTexture as Cst;
        let frame = match self.surface.get_current_texture() {
            Cst::Success(f) | Cst::Suboptimal(f) => f,
            // Transient: skip this frame and try again next redraw.
            Cst::Timeout | Cst::Occluded => return,
            // The surface no longer matches the window/canvas — rebuild it.
            Cst::Outdated | Cst::Lost => {
                self.reconfigure();
                return;
            }
            Cst::Validation => {
                log::error!("surface acquisition failed validation");
                return;
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame-encoder"),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("scene-pass"),
                // Render the scene to the offscreen target; bloom composites to the
                // surface afterwards.
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.scene_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(CLEAR_COLOR),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        // Nothing reads depth after this single forward pass, so don't pay
                        // to write it back to memory — free bandwidth on tiled GPUs (M8).
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            // Sky first (fullscreen, no depth write) so terrain draws over it.
            if toggles.sky {
                pass.set_pipeline(&self.sky_pipeline);
                pass.draw(0..3, 0..1);
            }

            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.globals_bind_group, &[]);
            pass.set_bind_group(2, &self.material_bind_group, &[]);

            let frustum = Frustum::from_view_proj(view_proj);

            // Visibility-graph "cave" cull, layered on the frustum (M5). Flood from the
            // camera's chunk through connected faces. Safe fallback: if the camera is
            // outside the loaded volume (flying above a surface world — the usual
            // case), skip it and frustum-cull only, so we can't wrongly hide terrain.
            let s = crate::world::Section::SIZE as f32;
            let cam_chunk = (
                (camera_pos.x / s).floor() as i32,
                (camera_pos.y / s).floor() as i32,
                (camera_pos.z / s).floor() as i32,
            );
            let visible = if toggles.cave_cull && self.draws.contains_key(&cam_chunk) {
                Some(visible_set(
                    cam_chunk,
                    |c| self.draws.get(&c).map(|d| d.graph),
                    |c| {
                        !toggles.frustum_cull
                            || self
                                .draws
                                .get(&c)
                                .is_some_and(|d| frustum.intersects_aabb(d.aabb_min, d.aabb_max))
                    },
                ))
            } else {
                None
            };

            // Build the visible draw list once (frustum + cave cull), then sort it
            // **front-to-back** by distance to the camera (M8): nearer chunks rasterise
            // first, so early-Z rejects the overdraw behind them — a real win on the
            // bandwidth-bound target. The same sorted list feeds the foliage pass.
            let mut visible_draws: Vec<&ChunkDraw> = self
                .draws
                .iter()
                .filter(|(coord, draw)| {
                    (!toggles.frustum_cull || frustum.intersects_aabb(draw.aabb_min, draw.aabb_max))
                        && visible.as_ref().is_none_or(|vis| vis.contains(*coord))
                })
                .map(|(_, draw)| draw)
                .collect();
            visible_draws.sort_by(|a, b| {
                let da = ((a.aabb_min + a.aabb_max) * 0.5 - camera_pos).length_squared();
                let db = ((b.aabb_min + b.aabb_max) * 0.5 - camera_pos).length_squared();
                da.total_cmp(&db)
            });

            let mut drawn = 0u32;
            let mut triangles = 0u32;
            for draw in &visible_draws {
                pass.set_bind_group(1, &draw.origin_bind_group, &[]);
                pass.set_vertex_buffer(0, draw.vertex_buffer.slice(..));
                pass.set_index_buffer(draw.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..draw.num_indices, 0, 0..1);
                drawn += 1;
                triangles += draw.num_indices / 3;
            }

            // Solid colossal structures (E18): same chunk pipeline + material, their own origin
            // per draw. Not frustum-culled (few, large); cheap relative to terrain.
            for draw in &self.structure_draws {
                pass.set_bind_group(1, &draw.origin_bind_group, &[]);
                pass.set_vertex_buffer(0, draw.vertex_buffer.slice(..));
                pass.set_index_buffer(draw.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..draw.num_indices, 0, 0..1);
                drawn += 1;
                triangles += draw.num_indices / 3;
            }

            // Foliage splats (E6): instanced billboards over the same visible (sorted)
            // chunks. One quad (6 verts) per instance, no index/vertex buffer. Drawn after
            // terrain so depth rejects buried blades.
            let mut splats = 0u32;
            if toggles.foliage {
                pass.set_pipeline(&self.splat_pipeline);
                pass.set_bind_group(0, &self.globals_bind_group, &[]);
                for draw in &visible_draws {
                    let Some((buf, count)) = &draw.foliage else {
                        continue;
                    };
                    pass.set_vertex_buffer(0, buf.slice(..));
                    pass.draw(0..6, 0..*count);
                    splats += count;
                }
            }

            // Colossal structures (E18): the in-range giants, one instanced draw, same pipeline.
            if self.structure_count > 0 {
                pass.set_pipeline(&self.splat_pipeline);
                pass.set_bind_group(0, &self.globals_bind_group, &[]);
                pass.set_vertex_buffer(0, self.structure_splats.slice(..));
                pass.draw(0..6, 0..self.structure_count);
                splats += self.structure_count;
            }

            // Drifting wisp creatures (E15): the swarm, re-uploaded each frame, same pipeline.
            if self.creature_count > 0 {
                pass.set_pipeline(&self.splat_pipeline);
                pass.set_bind_group(0, &self.globals_bind_group, &[]);
                pass.set_vertex_buffer(0, self.creature_splats.slice(..));
                pass.draw(0..6, 0..self.creature_count);
                splats += self.creature_count;
            }

            // In-world text (E17): glowing inscriptions, camera-facing billboards in the scene
            // pass (depth-tested against the world, palettised + fogged, glow through bloom).
            self.text.draw(&mut pass, &self.globals_bind_group);

            // Record stats for the HUD; still log occasionally on native.
            self.last_stats = DrawStats {
                drawn_chunks: drawn,
                total_chunks: self.draws.len() as u32,
                triangles,
                particles: particles.len() as u32,
                splats,
                relics: self.structure_draws.len() as u32,
            };
            self.frame_count += 1;
            if self.frame_count.is_multiple_of(120) {
                log::info!(
                    "drew {drawn}/{} chunks, {triangles} triangles, {} particles",
                    self.draws.len(),
                    particles.len()
                );
            }

            // Emissive particles on top, sharing the globals (group 0).
            if !particles.is_empty() {
                pass.set_pipeline(&self.particle_pipeline);
                pass.set_bind_group(0, &self.globals_bind_group, &[]);
                pass.set_vertex_buffer(0, self.cube_vertex_buffer.slice(..));
                pass.set_vertex_buffer(1, self.particle_instances.slice(..));
                pass.set_index_buffer(self.cube_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                pass.draw_indexed(0..CUBE_INDICES.len() as u32, 0, 0..particles.len() as u32);
            }
        }

        // Post chain (all at the internal resolution): bloom composites scene + glow into the
        // internal `palette_view`, then the palette pass *presents* it to the surface — it
        // palettises (when on) and always upscales by the pixel scale with nearest sampling.
        // Bloom toggled off → the scene is copied straight through instead of composited.
        if toggles.bloom {
            self.bloom.render(
                &self.device,
                &mut encoder,
                &self.scene_view,
                &self.palette_view,
            );
        } else {
            self.bloom.blit(
                &self.device,
                &mut encoder,
                &self.scene_view,
                &self.palette_view,
            );
        }
        self.palette
            .render(&mut encoder, &self.palette_bind_group, &view);

        // Space cruiser (E19): drawn over the palettised frame so its true colours + glowing
        // nav-lights survive (its own depth buffer self-occludes). Skipped under the map.
        if self.ship_active && !self.map_active {
            self.ship
                .set_transform(&self.queue, view_proj, self.ship_pos, self.ship_yaw);
            self.ship.draw(&mut encoder, &view, &self.ship_depth);
        }

        // Explored-world map (E10): when open, drawn fullscreen over the finished scene (the HUD
        // still composites on top, so status + biome stay visible).
        if self.map_active {
            self.map.draw(&mut encoder, &view);
        }

        // In-engine text overlay (HUD), composited last — identical on every platform.
        self.hud.draw(
            &self.queue,
            &mut encoder,
            &view,
            self.config.width,
            self.config.height,
        );

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
    }
}

/// Build a chunk's GPU buffers + per-chunk origin bind group. `None` for empty meshes.
fn build_chunk_draw(
    device: &wgpu::Device,
    chunk_bgl: &wgpu::BindGroupLayout,
    inst: &ChunkInstance,
    dissolve: bool,
) -> Option<ChunkDraw> {
    if inst.mesh.is_empty() {
        return None;
    }
    let packed: Vec<[u32; 2]> = inst.mesh.vertices.iter().map(pack).collect();
    // origin.w = 1 marks a solid colossal relic so the shader stipples it out with distance
    // (E18 mesh→points dissolve); 0 for terrain (no dissolve).
    let origin = ChunkUniform {
        origin: [
            inst.origin.x,
            inst.origin.y,
            inst.origin.z,
            if dissolve { 1.0 } else { 0.0 },
        ],
    };
    let origin_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("chunk-origin"),
        contents: bytemuck::bytes_of(&origin),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let origin_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("chunk-origin-bg"),
        layout: chunk_bgl,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: origin_buffer.as_entire_binding(),
        }],
    });
    let foliage = (!inst.foliage.is_empty()).then(|| {
        (
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("chunk-foliage"),
                contents: bytemuck::cast_slice(&inst.foliage),
                usage: wgpu::BufferUsages::VERTEX,
            }),
            inst.foliage.len() as u32,
        )
    });
    Some(ChunkDraw {
        vertex_buffer: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("chunk-vertices"),
            contents: bytemuck::cast_slice(&packed),
            usage: wgpu::BufferUsages::VERTEX,
        }),
        index_buffer: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("chunk-indices"),
            contents: bytemuck::cast_slice(&inst.mesh.indices),
            usage: wgpu::BufferUsages::INDEX,
        }),
        num_indices: inst.mesh.indices.len() as u32,
        aabb_min: Vec3::from(inst.mesh.aabb.min) + inst.origin,
        aabb_max: Vec3::from(inst.mesh.aabb.max) + inst.origin,
        origin_bind_group,
        graph: inst.graph,
        foliage,
    })
}

/// Instanced foliage-splat pipeline (E6): one unit quad built in the VS from the vertex
/// index, instanced per `SplatInstance`, billboarded with the camera basis. Alpha-test
/// (round mask via `discard`), depth-write on, no blend → no sorting. Shares the globals
/// group. `format` is the colour target. Shared with the headless renderer.
pub fn build_splat_pipeline(
    device: &wgpu::Device,
    globals_bgl: &wgpu::BindGroupLayout,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::include_wgsl!("splat.wgsl"));
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("splat-pipeline-layout"),
        bind_group_layouts: &[Some(globals_bgl)],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("splat-pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[SPLAT_INSTANCE_LAYOUT],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}

/// Per-instance vertex layout for a `SplatInstance` (offset, size, color, sway). The
/// quad corners come from `@builtin(vertex_index)`, so there's no per-vertex buffer.
const SPLAT_INSTANCE_LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
    array_stride: std::mem::size_of::<SplatInstance>() as wgpu::BufferAddress,
    step_mode: wgpu::VertexStepMode::Instance,
    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32, 2 => Float32x3, 3 => Float32, 4 => Float32],
};

/// Fullscreen sky-gradient pipeline: no bind groups, no vertex buffers (the vertex
/// shader builds a fullscreen triangle), no depth write (drawn behind the terrain).
/// Shared with the headless renderer. `format` is the colour target format.
pub fn build_sky_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::include_wgsl!("sky.wgsl"));
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("sky-pipeline-layout"),
        bind_group_layouts: &[],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("sky-pipeline"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(false),
            depth_compare: Some(wgpu::CompareFunction::Always),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}

/// Bind-group layout for the material texture array (group 2): a `2d_array` texture
/// plus a sampler, both fragment-only. Shared with the headless renderer.
pub fn material_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("material-bgl"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2Array,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    })
}

/// Upload the procedural material atlas to a texture array and build its bind group.
/// Unorm (not sRGB): the tiles are a brightness multiplier the shader applies to the
/// already-sRGB palette colour. Nearest + repeat so each voxel shows one crisp tile.
/// Shared with the headless renderer.
pub fn build_material_bind_group(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
) -> wgpu::BindGroup {
    let size = wgpu::Extent3d {
        width: TILE,
        height: TILE,
        depth_or_array_layers: LAYERS,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("material-array"),
        size,
        mip_level_count: mip_levels(),
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    // Upload every mip level (CPU box-filtered) so distant tiles minify cleanly.
    for (level, data) in material_mip_chain().iter().enumerate() {
        let s = TILE >> level;
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: level as u32,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(s * 4),
                rows_per_image: Some(s),
            },
            wgpu::Extent3d {
                width: s,
                height: s,
                depth_or_array_layers: LAYERS,
            },
        );
    }
    let view = texture.create_view(&wgpu::TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::D2Array),
        ..Default::default()
    });
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("material-sampler"),
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        address_mode_w: wgpu::AddressMode::Repeat,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::MipmapFilterMode::Nearest,
        ..Default::default()
    });
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("material-bg"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    })
}

/// Offscreen colour target the scene renders into (same format as the surface), so the
/// post chain can read it before presenting to the surface. Sized at the *internal*
/// resolution (`width`/`height` already divided by the pixel scale, E10).
fn create_scene_view(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    width: u32,
    height: u32,
) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("scene-color"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

fn create_depth_view(device: &wgpu::Device, width: u32, height: u32) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("depth-texture"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}
