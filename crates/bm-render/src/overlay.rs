//! Generic **post-palette overlay** (the G2 engine capability flagged in M9): vivid,
//! depth-tested, **non-palettised** triangles the game feeds each frame. It draws them into
//! an internal-resolution buffer with the **retained scene depth** (so terrain in front
//! occludes them), then **composites** that buffer additively over the finished frame with a
//! cheap own-glow (it misses the scene's pre-palette bloom).
//!
//! Engine-generic on purpose: it draws whatever world-space triangles it's handed — it knows
//! nothing about "beams". The game (`scraped-again`) expands its survey-beam into ribbons and
//! feeds them here, keeping the engine→game boundary intact.

use bytemuck::{Pod, Zeroable};
use glam::Mat4;

/// One overlay vertex (world space). The game builds camera-facing ribbons from these; the
/// per-vertex `alpha` carries the feathered glow falloff. Public so the game authors geometry.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct OverlayVertex {
    pub pos: [f32; 3],
    pub color: [f32; 3],
    pub alpha: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct OverlayU {
    view_proj: [[f32; 4]; 4],
}

/// Draws the game's overlay triangles after the palette, depth-tested + composited.
pub struct OverlayRenderer {
    line_pipeline: wgpu::RenderPipeline,
    ubuf: wgpu::Buffer,
    line_bg: wgpu::BindGroup,
    composite_pipeline: wgpu::RenderPipeline,
    composite_bgl: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    color_format: wgpu::TextureFormat,
    /// Internal-res buffer the lines draw into (then composited up to the surface).
    view: wgpu::TextureView,
    composite_bg: wgpu::BindGroup,
    /// Pooled vertex buffer (M11: grown on demand, written per frame — never created
    /// mid-frame once warm) + the live vertex count (0 = nothing to draw).
    verts: Option<(wgpu::Buffer, u32)>,
    verts_capacity: usize,
}

impl OverlayRenderer {
    pub fn new(
        device: &wgpu::Device,
        color_format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
        iw: u32,
        ih: u32,
    ) -> OverlayRenderer {
        let shader = device.create_shader_module(wgpu::include_wgsl!("overlay.wgsl"));

        // Line pass: a per-frame view_proj uniform.
        let line_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("overlay-line-bgl"),
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
        let line_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("overlay-line-layout"),
            bind_group_layouts: &[Some(&line_bgl)],
            immediate_size: 0,
        });
        let line_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("overlay-line-pipeline"),
            layout: Some(&line_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_line"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<OverlayVertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_line"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: color_format,
                    // Alpha-blend the feathered ribbon over the (transparent) overlay buffer.
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            // Depth-tested against the retained scene depth (terrain occludes), but never writes.
            depth_stencil: Some(wgpu::DepthStencilState {
                format: depth_format,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        // Composite pass: sample the overlay buffer, add it over the surface.
        let composite_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("overlay-composite-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
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
        });
        let composite_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("overlay-composite-layout"),
            bind_group_layouts: &[Some(&composite_bgl)],
            immediate_size: 0,
        });
        let composite_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("overlay-composite-pipeline"),
            layout: Some(&composite_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_comp"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_comp"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: color_format,
                    // Additive: the glow adds light over the finished frame.
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent::REPLACE,
                    }),
                    // RGB only — never touch the frame's alpha (clobbering it to 0 makes
                    // the headless RGBA capture fully transparent).
                    write_mask: wgpu::ColorWrites::COLOR,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let ubuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("overlay-uniform"),
            size: std::mem::size_of::<OverlayU>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let line_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("overlay-line-bg"),
            layout: &line_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: ubuf.as_entire_binding(),
            }],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("overlay-sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let (view, composite_bg) =
            make_target(device, color_format, &composite_bgl, &sampler, iw, ih);
        OverlayRenderer {
            line_pipeline,
            ubuf,
            line_bg,
            composite_pipeline,
            composite_bgl,
            sampler,
            color_format,
            view,
            composite_bg,
            verts: None,
            verts_capacity: 0,
        }
    }

    /// Recreate the internal-res buffer (+ its composite bind group) on resize.
    pub fn resize(&mut self, device: &wgpu::Device, iw: u32, ih: u32) {
        let (view, bg) = make_target(
            device,
            self.color_format,
            &self.composite_bgl,
            &self.sampler,
            iw,
            ih,
        );
        self.view = view;
        self.composite_bg = bg;
    }

    /// Upload this frame's overlay triangles (empty clears the overlay).
    pub fn set_lines_pooled(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        verts: &[OverlayVertex],
    ) {
        // M11 upload hygiene: a pooled buffer grown on demand (pow2) + `write_buffer`,
        // instead of a fresh `create_buffer_init` every frame the beam/flicks are live
        // (the wgpu#1242 hitch class is unthrottled mid-frame buffer creation).
        if verts.is_empty() {
            if let Some((_, count)) = &mut self.verts {
                *count = 0;
            }
            return;
        }
        if verts.len() > self.verts_capacity {
            self.verts_capacity = verts.len().next_power_of_two().max(256);
            self.verts = Some((
                device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("overlay-verts"),
                    size: (self.verts_capacity * std::mem::size_of::<OverlayVertex>()) as u64,
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }),
                0,
            ));
        }
        let (buf, count) = self.verts.as_mut().expect("just ensured");
        queue.write_buffer(buf, 0, bytemuck::cast_slice(verts));
        *count = verts.len() as u32;
    }

    /// One-shot variant (headless tooling): builds a fresh exact-size buffer.
    pub fn set_lines(&mut self, device: &wgpu::Device, verts: &[OverlayVertex]) {
        use wgpu::util::DeviceExt;
        self.verts_capacity = verts.len();
        self.verts = (!verts.is_empty()).then(|| {
            let buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("overlay-verts"),
                contents: bytemuck::cast_slice(verts),
                usage: wgpu::BufferUsages::VERTEX,
            });
            (buf, verts.len() as u32)
        });
    }

    pub fn set_view_proj(&self, queue: &wgpu::Queue, view_proj: Mat4) {
        let u = OverlayU {
            view_proj: view_proj.to_cols_array_2d(),
        };
        queue.write_buffer(&self.ubuf, 0, bytemuck::bytes_of(&u));
    }

    /// Is there anything to draw this frame? (Gates depth retention + the passes.)
    pub fn active(&self) -> bool {
        self.verts.as_ref().is_some_and(|(_, n)| *n > 0)
    }

    /// Draw the overlay triangles into the internal buffer, depth-tested against `depth` (the
    /// retained scene depth). Clears the buffer first.
    pub fn draw(&self, encoder: &mut wgpu::CommandEncoder, depth: &wgpu::TextureView) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("overlay-line-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load, // test against the scene's terrain depth
                    store: wgpu::StoreOp::Discard,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        if let Some((vbuf, count)) = self.verts.as_ref().filter(|(_, n)| *n > 0) {
            pass.set_pipeline(&self.line_pipeline);
            pass.set_bind_group(0, &self.line_bg, &[]);
            pass.set_vertex_buffer(0, vbuf.slice(..));
            pass.draw(0..*count, 0..1);
        }
    }

    /// Composite the overlay buffer additively over `surface` (the finished, palettised frame).
    pub fn composite(&self, encoder: &mut wgpu::CommandEncoder, surface: &wgpu::TextureView) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("overlay-composite-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: surface,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.composite_pipeline);
        pass.set_bind_group(0, &self.composite_bg, &[]);
        pass.draw(0..3, 0..1);
    }
}

fn make_target(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    bgl: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    iw: u32,
    ih: u32,
) -> (wgpu::TextureView, wgpu::BindGroup) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("overlay-color"),
        size: wgpu::Extent3d {
            width: iw.max(1),
            height: ih.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("overlay-composite-bg"),
        layout: bgl,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    });
    (view, bg)
}
