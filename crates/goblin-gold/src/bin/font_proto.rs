//! `font_proto` — BRICKMAP-GG1 mini-gate #1 evidence generator.
//!
//! Renders a representative Goblin-Gold **guide + question screen** (heading, a guide
//! *paragraph* — the legibility test — a worked example, and a drill question) through the
//! REAL wgpu path (offscreen, software-Vulkan ok) using the new anti-aliased TTF atlas, and
//! writes a PNG per reading size. The on-device/web render is the same wgpu pipeline, so at
//! 1:1 these pixels equal the phone's — the legibility verdict carries.
//!
//! Run (software Vulkan):
//!   VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json \
//!     cargo run -p goblin-gold --bin font_proto -- <out_dir>

use ab_glyph::FontRef;
use goblin_gold::text::{Atlas, Quad};
use std::mem::size_of;
use std::sync::mpsc;
use wgpu::util::DeviceExt;

const BG: [f32; 3] = [20.0 / 255.0, 12.0 / 255.0, 34.0 / 255.0]; // GG deep violet panel
const GOLD: [f32; 3] = [1.0, 214.0 / 255.0, 110.0 / 255.0];
const BODY: [f32; 3] = [226.0 / 255.0, 222.0 / 255.0, 238.0 / 255.0];
const DIM: [f32; 3] = [150.0 / 255.0, 140.0 / 255.0, 172.0 / 255.0];

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    pos: [f32; 2],
    uv: [f32; 2],
    col: [f32; 3],
}

const SHADER: &str = r#"
struct VsIn { @location(0) pos: vec2<f32>, @location(1) uv: vec2<f32>, @location(2) col: vec3<f32> };
struct VsOut { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32>, @location(1) col: vec3<f32> };
@vertex fn vs(in: VsIn) -> VsOut {
    var o: VsOut; o.pos = vec4<f32>(in.pos, 0.0, 1.0); o.uv = in.uv; o.col = in.col; return o;
}
@group(0) @binding(0) var atlas: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;
@fragment fn fs(in: VsOut) -> @location(0) vec4<f32> {
    let a = textureSample(atlas, samp, in.uv).r;   // coverage (anti-aliased)
    return vec4<f32>(in.col, a);
}
"#;

/// A laid-out text block plus its colour, ready to convert to vertices.
struct Block {
    quads: Vec<Quad>,
    col: [f32; 3],
}

fn main() {
    let out_dir = std::env::args().nth(1).unwrap_or_else(|| ".".to_string());
    std::fs::create_dir_all(&out_dir).ok();
    let font = FontRef::try_from_slice(goblin_gold::FONT_INSTRUMENT_SANS).expect("font");

    // A real GG guide+question screen. The PARAGRAPH is the legibility test.
    let topic = "Halving";
    let guide = "To halve a number, split it into two equal parts that add back up to the whole. \
Halve each digit in turn; when a digit is odd, carry a 5 into the next column. So half of 38 is 19, \
and half of 174 is 87. Halving twice is the same as dividing by four.";
    let example = "Worked example:  half of 48  =  24";
    let question = "Half of 168  =  ?";
    let hint = "Tap the number, then press ✓";

    // Phone-class canvas (portrait reading panel) + a few body reading sizes to judge.
    let width: u32 = 1080;
    let height: u32 = 1600;
    let margin = 72.0;
    let col_w = width as f32 - margin * 2.0;

    let (device, queue) = init_gpu();

    for &body_px in &[28.0_f32, 34.0, 42.0] {
        let head_px = (body_px * 1.7).round();
        let big_px = (body_px * 1.9).round();
        let a_body = Atlas::bake(&font, body_px);
        let a_head = Atlas::bake(&font, head_px);
        let a_big = Atlas::bake(&font, big_px);

        let mut blocks: Vec<(&Atlas, Block)> = Vec::new();
        let mut y = margin;
        // topic heading (gold)
        let (q, h) = a_head.layout(topic, margin, y, col_w);
        blocks.push((
            &a_head,
            Block {
                quads: q,
                col: GOLD,
            },
        ));
        y += h + body_px * 0.6;
        // guide paragraph (body — THE test)
        let (q, h) = a_body.layout(guide, margin, y, col_w);
        blocks.push((
            &a_body,
            Block {
                quads: q,
                col: BODY,
            },
        ));
        y += h + body_px * 0.8;
        // worked example (body, gold)
        let (q, h) = a_body.layout(example, margin, y, col_w);
        blocks.push((
            &a_body,
            Block {
                quads: q,
                col: GOLD,
            },
        ));
        y += h + body_px * 1.4;
        // question (big gold)
        let (q, h) = a_big.layout(question, margin, y, col_w);
        blocks.push((
            &a_big,
            Block {
                quads: q,
                col: GOLD,
            },
        ));
        y += h + body_px * 0.6;
        // hint (dim)
        let (q, _h) = a_body.layout(hint, margin, y, col_w);
        blocks.push((&a_body, Block { quads: q, col: DIM }));

        // Each atlas is its own texture → one draw per block (cheap; the prototype proves
        // legibility, not batching).
        let path = format!("{out_dir}/gg-prose-{}px.png", body_px as u32);
        render(&device, &queue, width, height, &blocks, &path);
        println!("wrote {path}");
    }
}

fn init_gpu() -> (wgpu::Device, wgpu::Queue) {
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::default(),
        compatible_surface: None,
        force_fallback_adapter: false,
    }))
    .expect("no Vulkan adapter — is mesa-vulkan-drivers installed? set VK_ICD_FILENAMES to lvp_icd.json");
    eprintln!("adapter: {:?}", adapter.get_info());
    pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("gg-font-proto"),
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::downlevel_defaults(),
        experimental_features: wgpu::ExperimentalFeatures::disabled(),
        memory_hints: wgpu::MemoryHints::Performance,
        trace: wgpu::Trace::Off,
    }))
    .expect("device")
}

fn render(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    width: u32,
    height: u32,
    blocks: &[(&Atlas, Block)],
    path: &str,
) {
    const FMT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("target"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FMT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = target.create_view(&wgpu::TextureViewDescriptor::default());

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("text"),
        source: wgpu::ShaderSource::Wgsl(SHADER.into()),
    });
    let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("text-bgl"),
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
    let pipe_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("text-pl"),
        bind_group_layouts: &[Some(&bind_layout)],
        immediate_size: 0,
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("text-pipe"),
        layout: Some(&pipe_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs"),
            compilation_options: Default::default(),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: size_of::<Vertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x3],
            }],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: FMT,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    });

    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });

    // Build, per block, an atlas texture + bind group + a vertex buffer.
    let to_ndc = |px: f32, py: f32| {
        [
            px / width as f32 * 2.0 - 1.0,
            1.0 - py / height as f32 * 2.0,
        ]
    };
    struct Drawable {
        bind: wgpu::BindGroup,
        vbuf: wgpu::Buffer,
        n: u32,
    }
    let mut drawables: Vec<Drawable> = Vec::new();
    for (atlas, block) in blocks {
        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("atlas"),
            size: wgpu::Extent3d {
                width: atlas.width,
                height: atlas.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &atlas.coverage,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(atlas.width),
                rows_per_image: Some(atlas.height),
            },
            wgpu::Extent3d {
                width: atlas.width,
                height: atlas.height,
                depth_or_array_layers: 1,
            },
        );
        let tview = tex.create_view(&wgpu::TextureViewDescriptor::default());
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("atlas-bg"),
            layout: &bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&tview),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        let mut verts: Vec<Vertex> = Vec::with_capacity(block.quads.len() * 6);
        for q in &block.quads {
            let tl = to_ndc(q.x, q.y);
            let tr = to_ndc(q.x + q.w, q.y);
            let bl = to_ndc(q.x, q.y + q.h);
            let br = to_ndc(q.x + q.w, q.y + q.h);
            let c = block.col;
            verts.push(Vertex {
                pos: tl,
                uv: [q.u0, q.v0],
                col: c,
            });
            verts.push(Vertex {
                pos: tr,
                uv: [q.u1, q.v0],
                col: c,
            });
            verts.push(Vertex {
                pos: bl,
                uv: [q.u0, q.v1],
                col: c,
            });
            verts.push(Vertex {
                pos: tr,
                uv: [q.u1, q.v0],
                col: c,
            });
            verts.push(Vertex {
                pos: br,
                uv: [q.u1, q.v1],
                col: c,
            });
            verts.push(Vertex {
                pos: bl,
                uv: [q.u0, q.v1],
                col: c,
            });
        }
        let vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("verts"),
            contents: bytemuck::cast_slice(&verts),
            usage: wgpu::BufferUsages::VERTEX,
        });
        drawables.push(Drawable {
            bind,
            vbuf,
            n: verts.len() as u32,
        });
    }

    let mut encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("text-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: BG[0] as f64,
                        g: BG[1] as f64,
                        b: BG[2] as f64,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&pipeline);
        for d in &drawables {
            pass.set_bind_group(0, &d.bind, &[]);
            pass.set_vertex_buffer(0, d.vbuf.slice(..));
            pass.draw(0..d.n, 0..1);
        }
    }

    // Readback → PNG (256-byte row alignment).
    let unpadded_bpr = width * 4;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let padded_bpr = unpadded_bpr.div_ceil(align) * align;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: (padded_bpr * height) as u64,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &target,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_bpr),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(std::iter::once(encoder.finish()));

    let (tx, rx) = mpsc::channel();
    readback.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    let _ = device.poll(wgpu::PollType::wait_indefinitely());
    rx.recv().expect("map channel").expect("buffer map failed");
    let data = readback.slice(..).get_mapped_range();
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for row in 0..height {
        let start = (row * padded_bpr) as usize;
        rgba.extend_from_slice(&data[start..start + unpadded_bpr as usize]);
    }
    drop(data);
    readback.unmap();

    let file = std::fs::File::create(path).expect("create png");
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), width, height);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header()
        .expect("png header")
        .write_image_data(&rgba)
        .expect("png data");
}
