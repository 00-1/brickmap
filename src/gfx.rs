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
}

/// Short labels for the toggles, in index order (HUD + web checkboxes).
pub const TOGGLE_LABELS: [&str; 10] = [
    "cull", "cave", "sky", "sparks", "bloom", "fog", "ao", "light", "glow", "relief",
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
            _ => {}
        }
    }

    pub fn toggle(&mut self, i: usize) {
        self.set(i, !self.get(i));
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
    [0.95, 0.10, 0.95, 1.0], // 7
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
            if let Some(draw) = build_chunk_draw(&device, &chunk_bgl, inst) {
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

        let depth_view = create_depth_view(&device, &config);
        let scene_view = create_scene_view(&device, &config);
        let bloom = Bloom::new(&device, config.format, config.width, config.height);

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
            particle_pipeline,
            cube_vertex_buffer,
            cube_index_buffer,
            particle_instances,
            particle_capacity,
            frame_count: 0,
            last_stats: DrawStats::default(),
            window,
        }
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
        match build_chunk_draw(&self.device, &self.chunk_bind_group_layout, instance) {
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
        self.depth_view = create_depth_view(&self.device, &self.config);
        self.scene_view = create_scene_view(&self.device, &self.config);
        self.bloom
            .resize(&self.device, self.config.width, self.config.height);
    }

    /// Reconfigure at the current size — used to recover a lost/outdated surface.
    pub fn reconfigure(&mut self) {
        self.resize(self.size);
    }

    /// Draw the scene from the given view-projection, plus the live particles. Chunk
    /// vertices are packed and chunk-local; the shader adds each chunk's world origin.
    pub fn render(
        &mut self,
        view_proj: Mat4,
        camera_pos: Vec3,
        particles: &[ParticleInstance],
        aesthetic: [f32; 2],
        toggles: Toggles,
    ) {
        // Particles off → draw none (the system keeps simulating; cheap).
        let particles: &[ParticleInstance] = if toggles.particles { particles } else { &[] };
        // Fog off → push the band out past anything visible.
        let (fog_start, fog_end) = if toggles.fog {
            (FOG_START, FOG_END)
        } else {
            (1.0e9, 1.0e9 + 1.0)
        };
        let flag = |b: bool| if b { 1.0 } else { 0.0 };
        let globals = Globals {
            view_proj: view_proj.to_cols_array_2d(),
            palette: PALETTE,
            params: [aesthetic[0], aesthetic[1], fog_start, fog_end],
            camera_pos: [camera_pos.x, camera_pos.y, camera_pos.z, 0.0],
            fog_color: FOG_COLOR,
            flags: [
                flag(toggles.ao),
                flag(toggles.block_light),
                flag(toggles.emissive),
                flag(toggles.relief),
            ],
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
                        store: wgpu::StoreOp::Store,
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

            let mut drawn = 0u32;
            let mut triangles = 0u32;
            for (coord, draw) in self.draws.iter() {
                if toggles.frustum_cull && !frustum.intersects_aabb(draw.aabb_min, draw.aabb_max) {
                    continue;
                }
                if let Some(vis) = &visible {
                    if !vis.contains(coord) {
                        continue; // sealed off from the camera — cave-culled
                    }
                }
                pass.set_bind_group(1, &draw.origin_bind_group, &[]);
                pass.set_vertex_buffer(0, draw.vertex_buffer.slice(..));
                pass.set_index_buffer(draw.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..draw.num_indices, 0, 0..1);
                drawn += 1;
                triangles += draw.num_indices / 3;
            }

            // Record stats for the HUD; still log occasionally on native.
            self.last_stats = DrawStats {
                drawn_chunks: drawn,
                total_chunks: self.draws.len() as u32,
                triangles,
                particles: particles.len() as u32,
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

        // Bloom: bright-pass + blur the scene target, composite scene + glow → surface.
        // Toggled off → copy the scene straight through.
        if toggles.bloom {
            self.bloom
                .render(&self.device, &mut encoder, &self.scene_view, &view);
        } else {
            self.bloom
                .blit(&self.device, &mut encoder, &self.scene_view, &view);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
    }
}

/// Build a chunk's GPU buffers + per-chunk origin bind group. `None` for empty meshes.
fn build_chunk_draw(
    device: &wgpu::Device,
    chunk_bgl: &wgpu::BindGroupLayout,
    inst: &ChunkInstance,
) -> Option<ChunkDraw> {
    if inst.mesh.is_empty() {
        return None;
    }
    let packed: Vec<[u32; 2]> = inst.mesh.vertices.iter().map(pack).collect();
    let origin = ChunkUniform {
        origin: [inst.origin.x, inst.origin.y, inst.origin.z, 0.0],
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
    })
}

/// Fullscreen sky-gradient pipeline: no bind groups, no vertex buffers (the vertex
/// shader builds a fullscreen triangle), no depth write (drawn behind the terrain).
/// Shared with the headless renderer. `format` is the colour target format.
pub(crate) fn build_sky_pipeline(
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
pub(crate) fn material_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
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
pub(crate) fn build_material_bind_group(
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

/// Offscreen colour target the scene renders into (same format as the surface), so
/// the bloom post-chain can read it before compositing to the surface.
fn create_scene_view(
    device: &wgpu::Device,
    config: &wgpu::SurfaceConfiguration,
) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("scene-color"),
        size: wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: config.format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

fn create_depth_view(
    device: &wgpu::Device,
    config: &wgpu::SurfaceConfiguration,
) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("depth-texture"),
        size: wgpu::Extent3d {
            width: config.width,
            height: config.height,
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
