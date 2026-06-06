//! Configurable colour palette (E10) — a post-process that maps the finished image to a
//! small, restrained palette with ordered dithering. Moves the look off the default
//! "Minecraft" material colours toward a curated, low-count aesthetic you can dial.
//!
//! `PalettePass` is reusable by the windowed renderer and the headless one, so the look
//! is identical and verifiable offscreen. Palettes are curated + deliberately restrained.

use bytemuck::{Pod, Zeroable};

/// A named, ordered (dark → light) colour ramp. Kept small + restrained on purpose.
pub struct Palette {
    pub name: &'static str,
    pub colors: &'static [[f32; 3]],
}

/// Curated palettes, none of them the stock voxel/Minecraft hues. Index 0 is a neutral
/// mono ramp; the rest lean into a single restrained mood. `count` (below) can use fewer
/// than a palette's full length for an even harder look.
pub const PALETTES: &[Palette] = &[
    Palette {
        name: "mono",
        colors: &[
            [0.05, 0.06, 0.08],
            [0.30, 0.33, 0.38],
            [0.58, 0.62, 0.68],
            [0.86, 0.89, 0.94],
        ],
    },
    Palette {
        name: "verdant",
        colors: &[
            [0.06, 0.09, 0.08],
            [0.13, 0.24, 0.20],
            [0.27, 0.42, 0.31],
            [0.52, 0.66, 0.44],
            [0.83, 0.86, 0.66],
        ],
    },
    Palette {
        name: "ash",
        colors: &[
            [0.07, 0.08, 0.10],
            [0.22, 0.26, 0.31],
            [0.40, 0.46, 0.52],
            [0.62, 0.68, 0.72],
            [0.88, 0.90, 0.92],
        ],
    },
    Palette {
        name: "ember",
        colors: &[
            [0.05, 0.04, 0.06],
            [0.24, 0.12, 0.14],
            [0.55, 0.22, 0.16],
            [0.82, 0.45, 0.22],
            [0.95, 0.80, 0.55],
        ],
    },
    Palette {
        name: "dusk",
        colors: &[
            [0.06, 0.06, 0.11],
            [0.20, 0.18, 0.33],
            [0.40, 0.34, 0.52],
            [0.63, 0.52, 0.66],
            [0.88, 0.80, 0.82],
        ],
    },
    Palette {
        name: "mist",
        colors: &[
            [0.09, 0.12, 0.14],
            [0.24, 0.34, 0.36],
            [0.45, 0.58, 0.57],
            [0.70, 0.80, 0.76],
            [0.92, 0.95, 0.92],
        ],
    },
    // --- Two-hue palettes: a dark, grimy base ramp with a contrasting accent at the bright
    // end. Because the look maps luminance onto the ramp, that accent lands on highlights —
    // i.e. the point lights — so they "pop" in a clashing hue against the base. Best with
    // the sun off.
    Palette {
        // Red base with pops of green (the requested look): a blood/rust ramp, acid-green
        // glints where the light hits.
        name: "rust",
        colors: &[
            [0.06, 0.04, 0.05],
            [0.24, 0.07, 0.07],
            [0.48, 0.13, 0.10],
            [0.74, 0.28, 0.16],
            [0.46, 0.92, 0.38],
        ],
    },
    Palette {
        // Deep purple → hot magenta, with a cyan pop. Synthwave-in-a-cave.
        name: "neon",
        colors: &[
            [0.04, 0.03, 0.07],
            [0.15, 0.07, 0.22],
            [0.36, 0.10, 0.40],
            [0.82, 0.18, 0.60],
            [0.40, 0.95, 0.96],
        ],
    },
    Palette {
        // Cold slate base with warm sodium-amber pops — streetlights through fog.
        name: "sodium",
        colors: &[
            [0.04, 0.05, 0.07],
            [0.10, 0.15, 0.19],
            [0.20, 0.28, 0.30],
            [0.62, 0.40, 0.14],
            [1.00, 0.78, 0.34],
        ],
    },
    Palette {
        // Dark soil/olive base with an acid-lime pop — toxic bog.
        name: "bog",
        colors: &[
            [0.05, 0.05, 0.04],
            [0.15, 0.13, 0.09],
            [0.28, 0.24, 0.12],
            [0.40, 0.46, 0.16],
            [0.78, 0.98, 0.34],
        ],
    },
    // --- Batch 2 (10 more): a wider spread of one- and two-hue ramps, all dark-leaning to
    // suit the grimy mood. Two-hue ones (oxide, bruise, cobalt, slime) put a clashing accent
    // at the bright end so the point lights pop in a complementary colour.
    Palette {
        // Verdigris teal base with a rust-orange pop — weathered copper/patina.
        name: "oxide",
        colors: &[
            [0.05, 0.07, 0.07],
            [0.10, 0.20, 0.19],
            [0.21, 0.38, 0.34],
            [0.55, 0.34, 0.16],
            [0.93, 0.62, 0.26],
        ],
    },
    Palette {
        // Deep indigo base, sickly acid yellow-green pop — a bruise.
        name: "bruise",
        colors: &[
            [0.05, 0.04, 0.08],
            [0.16, 0.10, 0.24],
            [0.31, 0.16, 0.34],
            [0.48, 0.30, 0.30],
            [0.82, 0.88, 0.32],
        ],
    },
    Palette {
        // Cold deep-sea blue ramp, black → cyan-white.
        name: "abyss",
        colors: &[
            [0.02, 0.03, 0.06],
            [0.06, 0.12, 0.24],
            [0.12, 0.28, 0.45],
            [0.31, 0.53, 0.67],
            [0.80, 0.93, 0.97],
        ],
    },
    Palette {
        // Toxic green ramp, near-black → acid lime-white.
        name: "venom",
        colors: &[
            [0.03, 0.05, 0.03],
            [0.08, 0.18, 0.08],
            [0.18, 0.36, 0.14],
            [0.42, 0.66, 0.22],
            [0.82, 0.98, 0.60],
        ],
    },
    Palette {
        // Lava ramp: black → blood → orange → yellow-white. Brighter/hotter than ember.
        name: "magma",
        colors: &[
            [0.04, 0.02, 0.02],
            [0.24, 0.05, 0.03],
            [0.55, 0.14, 0.05],
            [0.86, 0.42, 0.10],
            [1.00, 0.88, 0.48],
        ],
    },
    Palette {
        // Oily black → brown → amber — tar, sump, crude.
        name: "tar",
        colors: &[
            [0.03, 0.03, 0.03],
            [0.14, 0.10, 0.07],
            [0.28, 0.20, 0.10],
            [0.50, 0.36, 0.16],
            [0.86, 0.68, 0.32],
        ],
    },
    Palette {
        // Navy base with a hot-orange pop — classic complementary blue/orange.
        name: "cobalt",
        colors: &[
            [0.04, 0.05, 0.10],
            [0.09, 0.13, 0.28],
            [0.16, 0.25, 0.46],
            [0.62, 0.40, 0.18],
            [1.00, 0.66, 0.26],
        ],
    },
    Palette {
        // Dark teal base with a magenta pop — toxic slime.
        name: "slime",
        colors: &[
            [0.04, 0.06, 0.07],
            [0.10, 0.20, 0.20],
            [0.18, 0.36, 0.34],
            [0.52, 0.18, 0.42],
            [0.94, 0.36, 0.72],
        ],
    },
    Palette {
        // Muted sepia → cream — old parchment, candlelight. Soft and warm.
        name: "parchment",
        colors: &[
            [0.06, 0.05, 0.04],
            [0.20, 0.16, 0.11],
            [0.40, 0.33, 0.22],
            [0.64, 0.56, 0.40],
            [0.92, 0.86, 0.70],
        ],
    },
    Palette {
        // Cold blue → ice white — frost, distinct from mist (bluer, colder).
        name: "frost",
        colors: &[
            [0.04, 0.05, 0.09],
            [0.14, 0.20, 0.31],
            [0.30, 0.43, 0.55],
            [0.58, 0.73, 0.83],
            [0.90, 0.97, 1.00],
        ],
    },
];

/// Max palette colours the shader uniform holds.
pub const MAX_COLORS: usize = 16;

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
    pub fn set(&self, queue: &wgpu::Queue, index: usize, count: u32, dither: f32, enabled: bool) {
        let pal = &PALETTES[index.min(PALETTES.len() - 1)];
        let n = (count as usize).clamp(1, pal.colors.len()).min(MAX_COLORS);
        let mut colors = [[0.0f32; 4]; MAX_COLORS];
        for (i, slot) in colors.iter_mut().enumerate().take(n) {
            let c = pal.colors[i % pal.colors.len()];
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
