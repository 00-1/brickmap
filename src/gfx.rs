//! Minimal wgpu render state for the cross-platform spike.
//!
//! This is intentionally a single flat module. None of it is meant to survive
//! into the real engine unchanged — see `docs/architecture.md` for the planned
//! `platform` / `render` / `world` / `mesh` crate split. The spike exists only
//! to prove the wgpu render path compiles and runs on desktop **and** the web.

use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;
use winit::window::Window;

/// A single interleaved cube vertex: chunk-local-ish position + a flat colour.
///
/// NOTE: in the real engine this becomes a packed ~4–8 byte face vertex (see the
/// "compressed vertices" perf pillar). Here we keep it fat and obvious.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

impl Vertex {
    const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3],
    };
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct Uniforms {
    mvp: [[f32; 4]; 4],
}

// Unit cube centred on the origin. One colour per corner so the faces read as
// smooth gradients and rotation is obvious.
#[rustfmt::skip]
const VERTICES: &[Vertex] = &[
    Vertex { position: [-0.5, -0.5, -0.5], color: [0.0, 0.0, 0.0] },
    Vertex { position: [ 0.5, -0.5, -0.5], color: [1.0, 0.0, 0.0] },
    Vertex { position: [ 0.5,  0.5, -0.5], color: [1.0, 1.0, 0.0] },
    Vertex { position: [-0.5,  0.5, -0.5], color: [0.0, 1.0, 0.0] },
    Vertex { position: [-0.5, -0.5,  0.5], color: [0.0, 0.0, 1.0] },
    Vertex { position: [ 0.5, -0.5,  0.5], color: [1.0, 0.0, 1.0] },
    Vertex { position: [ 0.5,  0.5,  0.5], color: [1.0, 1.0, 1.0] },
    Vertex { position: [-0.5,  0.5,  0.5], color: [0.0, 1.0, 1.0] },
];

#[rustfmt::skip]
const INDICES: &[u16] = &[
    0, 1, 2,  0, 2, 3, // back
    4, 6, 5,  4, 7, 6, // front
    4, 5, 1,  4, 1, 0, // bottom
    3, 2, 6,  3, 6, 7, // top
    4, 0, 3,  4, 3, 7, // left
    1, 5, 6,  1, 6, 2, // right
];

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const CLEAR_COLOR: wgpu::Color = wgpu::Color { r: 0.02, g: 0.03, b: 0.06, a: 1.0 };

pub struct State {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,

    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,

    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    depth_view: wgpu::TextureView,

    /// Rotation accumulator (radians). Advanced a fixed amount per frame so the
    /// spike needs no clock and behaves identically on every platform.
    angle: f32,

    // The window must outlive the surface; keep an Arc so `Surface<'static>` is sound.
    window: Arc<Window>,
}

impl State {
    pub async fn new(window: Arc<Window>) -> State {
        let mut size = window.inner_size();
        // Browsers can report a 0x0 canvas on first frame; clamp to something sane.
        size.width = size.width.max(1);
        size.height = size.height.max(1);

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

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("uniforms"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("uniform-bgl"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("uniform-bg"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pipeline-layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("cube-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::LAYOUT],
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

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("cube-vertices"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("cube-indices"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let depth_view = create_depth_view(&device, &config);

        State {
            surface,
            device,
            queue,
            config,
            size,
            pipeline,
            vertex_buffer,
            index_buffer,
            num_indices: INDICES.len() as u32,
            uniform_buffer,
            uniform_bind_group,
            depth_view,
            angle: 0.0,
            window,
        }
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
    }

    /// Reconfigure at the current size — used to recover a lost/outdated surface.
    pub fn reconfigure(&mut self) {
        self.resize(self.size);
    }

    fn update_uniforms(&mut self) {
        self.angle += 0.01;
        let aspect = self.config.width as f32 / self.config.height as f32;
        let proj = Mat4::perspective_rh(60f32.to_radians(), aspect, 0.1, 100.0);
        let view = Mat4::look_at_rh(Vec3::new(0.0, 1.4, 3.0), Vec3::ZERO, Vec3::Y);
        let model = Mat4::from_rotation_y(self.angle) * Mat4::from_rotation_x(self.angle * 0.5);
        let mvp = proj * view * model;
        let uniforms = Uniforms { mvp: mvp.to_cols_array_2d() };
        self.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));
    }

    pub fn render(&mut self) {
        self.update_uniforms();

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
                label: Some("cube-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
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

            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.uniform_bind_group, &[]);
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            pass.draw_indexed(0..self.num_indices, 0, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
    }
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
