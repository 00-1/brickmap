//! Bloom post-processing (E3). Owns the bright-pass / blur / composite pipelines and
//! the ¼-resolution ping-pong targets, so both the windowed (`gfx`) and headless
//! renderers share one implementation. The scene is rendered to an offscreen colour
//! texture; [`Bloom::render`] reads it, extracts + blurs the bright parts, and
//! composites scene + glow into the output view.

const BLOOM_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

pub(crate) struct Bloom {
    bright: wgpu::RenderPipeline,
    blur_h: wgpu::RenderPipeline,
    blur_v: wgpu::RenderPipeline,
    composite: wgpu::RenderPipeline,
    sampler: wgpu::Sampler,
    bgl_single: wgpu::BindGroupLayout,
    bgl_composite: wgpu::BindGroupLayout,
    view_a: wgpu::TextureView,
    view_b: wgpu::TextureView,
}

impl Bloom {
    /// `output_format` is the final target (surface / headless colour); the scene
    /// input and the composite output share it (both are the scene's colour format).
    pub(crate) fn new(
        device: &wgpu::Device,
        output_format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> Bloom {
        let shader = device.create_shader_module(wgpu::include_wgsl!("post.wgsl"));

        let tex_entry = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let sampler_entry = wgpu::BindGroupLayoutEntry {
            binding: 1,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        };
        // bright/blur: one input texture + sampler.
        let bgl_single = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bloom-bgl-single"),
            entries: &[tex_entry(0), sampler_entry],
        });
        // composite: scene (0) + sampler (1) + bloom (2).
        let bgl_composite = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bloom-bgl-composite"),
            entries: &[tex_entry(0), sampler_entry, tex_entry(2)],
        });

        let pipe = |label, fs: &str, target, bgl: &wgpu::BindGroupLayout| {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(label),
                bind_group_layouts: &[Some(bgl)],
                immediate_size: 0,
            });
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(fs),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: target,
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
            })
        };

        let bright = pipe("bloom-bright", "fs_bright", BLOOM_FORMAT, &bgl_single);
        let blur_h = pipe("bloom-blur-h", "fs_blur_h", BLOOM_FORMAT, &bgl_single);
        let blur_v = pipe("bloom-blur-v", "fs_blur_v", BLOOM_FORMAT, &bgl_single);
        let composite = pipe(
            "bloom-composite",
            "fs_composite",
            output_format,
            &bgl_composite,
        );

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("bloom-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        let (view_a, view_b) = Self::make_targets(device, width, height);
        Bloom {
            bright,
            blur_h,
            blur_v,
            composite,
            sampler,
            bgl_single,
            bgl_composite,
            view_a,
            view_b,
        }
    }

    /// Recreate the ¼-res ping-pong targets for a new output size.
    pub(crate) fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        let (a, b) = Self::make_targets(device, width, height);
        self.view_a = a;
        self.view_b = b;
    }

    fn make_targets(
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> (wgpu::TextureView, wgpu::TextureView) {
        let make = |label| {
            let t = device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width: (width / 4).max(1),
                    height: (height / 4).max(1),
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: BLOOM_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            t.create_view(&wgpu::TextureViewDescriptor::default())
        };
        (make("bloom-a"), make("bloom-b"))
    }

    /// Record the bloom passes: bright (scene → a), blur H (a → b), blur V (b → a),
    /// composite (scene + a → output).
    pub(crate) fn render(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        scene_view: &wgpu::TextureView,
        output_view: &wgpu::TextureView,
    ) {
        let single = |input: &wgpu::TextureView| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("bloom-bg"),
                layout: &self.bgl_single,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(input),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            })
        };
        let bright_bg = single(scene_view);
        let blur_h_bg = single(&self.view_a);
        let blur_v_bg = single(&self.view_b);
        let composite_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bloom-composite-bg"),
            layout: &self.bgl_composite,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(scene_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&self.view_a),
                },
            ],
        });

        run_pass(encoder, &self.view_a, &self.bright, &bright_bg);
        run_pass(encoder, &self.view_b, &self.blur_h, &blur_h_bg);
        run_pass(encoder, &self.view_a, &self.blur_v, &blur_v_bg);
        run_pass(encoder, output_view, &self.composite, &composite_bg);
    }
}

/// One fullscreen post pass: clear the target, draw the fullscreen triangle.
fn run_pass(
    encoder: &mut wgpu::CommandEncoder,
    target: &wgpu::TextureView,
    pipeline: &wgpu::RenderPipeline,
    bind_group: &wgpu::BindGroup,
) {
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("bloom-pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: target,
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
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, bind_group, &[]);
    pass.draw(0..3, 0..1);
}
