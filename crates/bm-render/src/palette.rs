//! Configurable colour palette (E10) — a post-process that maps the finished image to a
//! small palette with ordered dithering. The *mechanism* is the engine's; the **curated
//! ramps are content** and live in the game (M9). `PalettePass::set_colors` takes whatever
//! ramp it's handed, so the look is the caller's choice; [`DEFAULT_RAMP`] is a neutral
//! built-in so the engine demo isn't blank.
//!
//! `PalettePass` is reusable by the windowed renderer and the headless one, so the look
//! is identical and verifiable offscreen.

use bytemuck::{Pod, Zeroable};

/// Max palette colours the shader uniform holds.
pub const MAX_COLORS: usize = 16;

/// A neutral dark→light mono ramp the engine ships so its demo (and an un-themed frame)
/// has *a* palette. The game supplies its own curated ramps via [`PalettePass::set_colors`].
pub const DEFAULT_RAMP: &[[f32; 3]] = &[
    [0.05, 0.06, 0.08],
    [0.30, 0.33, 0.38],
    [0.58, 0.62, 0.68],
    [0.86, 0.89, 0.94],
];

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Params {
    colors: [[f32; 4]; MAX_COLORS],
    count: u32,
    enabled: u32,
    dither: f32,
    _pad: f32,
}

/// Fullscreen palette-mapping pass.
pub struct PalettePass {
    pipeline: wgpu::RenderPipeline,
    bgl: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    ubuf: wgpu::Buffer,
}

impl PalettePass {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> PalettePass {
        let shader = device.create_shader_module(wgpu::include_wgsl!("palette.wgsl"));
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("palette-bgl"),
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
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("palette-pipeline-layout"),
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("palette-pipeline"),
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
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        // Nearest sampling: this pass doubles as the present/upscale of the (optionally
        // low-res) internal buffer, so we want crisp fat pixels, not a blurred upscale.
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("palette-sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let ubuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("palette-params"),
            size: std::mem::size_of::<Params>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        PalettePass {
            pipeline,
            bgl,
            sampler,
            ubuf,
        }
    }

    /// Set the palette (`index` into [`PALETTES`]), how many of its colours to use
    /// (`count`, clamped to the palette length), the dither spread, and whether the pass
    /// recolours at all (`enabled == false` → passthrough).
    /// Set the palette from an explicit `src` colour ramp (any ramp — a curated entry, a
    /// biome-blended one, or [`DEFAULT_RAMP`]). The engine holds no curated set; the caller
    /// owns the look. `count` is clamped to the colours supplied and `MAX_COLORS`; `enabled
    /// == false` makes the pass a passthrough (used for upscale-only).
    pub fn set_colors(
        &self,
        queue: &wgpu::Queue,
        src: &[[f32; 3]],
        count: u32,
        dither: f32,
        enabled: bool,
    ) {
        let avail = src.len().max(1);
        let n = (count as usize).clamp(1, avail).min(MAX_COLORS);
        let mut colors = [[0.0f32; 4]; MAX_COLORS];
        for (i, slot) in colors.iter_mut().enumerate().take(n) {
            let c = src[i % avail];
            *slot = [c[0], c[1], c[2], 1.0];
        }
        let params = Params {
            colors,
            count: n as u32,
            enabled: enabled as u32,
            dither,
            _pad: 0.0,
        };
        queue.write_buffer(&self.ubuf, 0, bytemuck::bytes_of(&params));
    }

    /// Build the bind group for a given input texture (rebuild when the target resizes).
    pub fn make_bind_group(
        &self,
        device: &wgpu::Device,
        src: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("palette-bg"),
            layout: &self.bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(src),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.ubuf.as_entire_binding(),
                },
            ],
        })
    }

    /// Map `bind_group`'s input into `out` (fullscreen).
    pub fn render(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        bind_group: &wgpu::BindGroup,
        out: &wgpu::TextureView,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("palette-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: out,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}
