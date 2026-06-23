//! A tiny **headless 2-D painter** (engine-candidate) + a **golden-diff layer**: clear a
//! background, fill flat rects, draw anti-aliased text (the [`crate::text`] atlas), optionally
//! recolour the finished frame through the engine's **palette dither** recipe
//! ([`brickmap::palette::PalettePass`] — Bayer 4×4 over a curated ramp), all through the real
//! wgpu path → a PNG. One unified scene pipeline: every quad samples a coverage texture (a glyph
//! for text, a 1×1 white texel for rects) and multiplies it into the vertex alpha.
//!
//! Native-only (the on-device/web render is the same wgpu; this proves the pixels headless on a
//! software adapter). The golden helpers ([`rgba_from_png`], [`diff`], [`matches`]) are pure CPU
//! so a golden-image test's *comparator* runs in CI without a GPU, while the GPU render that
//! produces the candidate frame runs under lavapipe.

use crate::text::{Atlas, Quad};
use brickmap::palette::PalettePass;
use std::mem::size_of;
use std::sync::mpsc;
use wgpu::util::DeviceExt;

/// A run of text: its atlas, the laid-out glyph quads, and an RGBA colour (0..1).
pub struct TextRun<'a> {
    pub atlas: &'a Atlas,
    pub quads: Vec<Quad>,
    pub rgba: [f32; 4],
}

/// A filled rectangle (pixel coords, top-left origin) with an RGBA colour (0..1).
#[derive(Clone, Copy)]
pub struct RectRun {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub rgba: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    pos: [f32; 2],
    uv: [f32; 2],
    rgba: [f32; 4],
}

const SHADER: &str = r#"
struct VsIn { @location(0) pos: vec2<f32>, @location(1) uv: vec2<f32>, @location(2) rgba: vec4<f32> };
struct VsOut { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32>, @location(1) rgba: vec4<f32> };
@vertex fn vs(in: VsIn) -> VsOut {
    var o: VsOut; o.pos = vec4<f32>(in.pos, 0.0, 1.0); o.uv = in.uv; o.rgba = in.rgba; return o;
}
@group(0) @binding(0) var cov: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;
@fragment fn fs(in: VsOut) -> @location(0) vec4<f32> {
    let a = textureSample(cov, samp, in.uv).r;   // glyph coverage, or 1.0 for the white rect texel
    return vec4<f32>(in.rgba.rgb, in.rgba.a * a);
}
"#;

/// Scene format (the quad pipeline draws into this). Sampleable, so the palette pass can read it.
const FMT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
/// Palettised output format. sRGB so the engine's `srgb_to_linear` ramp lands as authored — the
/// palette pass is built for an sRGB target (see `palette.wgsl`).
const SRGB: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

/// One drawable run (a vertex buffer + the coverage texture it samples).
struct Drawable {
    bind: wgpu::BindGroup,
    vbuf: wgpu::Buffer,
    n: u32,
}

/// A headless 2-D painter holding the device + pipeline + a 1×1 white texel (for rects).
pub struct Painter {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::RenderPipeline,
    bind_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    white: wgpu::Texture,
}

impl Default for Painter {
    fn default() -> Self {
        Self::new()
    }
}

impl Painter {
    pub fn new() -> Painter {
        let instance = wgpu::Instance::default();
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .expect("no Vulkan adapter — install mesa-vulkan-drivers + set VK_ICD_FILENAMES to lvp_icd.json");
        eprintln!("adapter: {:?}", adapter.get_info());
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("gg-painter"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_defaults(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        }))
        .expect("device");

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ui"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ui-bgl"),
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
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ui-pl"),
            bind_group_layouts: &[Some(&bind_layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ui-pipe"),
            layout: Some(&pl),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: size_of::<Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4],
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
        // 1×1 fully-opaque coverage texel for solid rects.
        let white = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("white"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
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
                texture: &white,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &[255u8],
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(1),
                rows_per_image: Some(1),
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );

        Painter {
            device,
            queue,
            pipeline,
            bind_layout,
            sampler,
            white,
        }
    }

    fn upload_coverage(&self, w: u32, h: u32, bytes: &[u8]) -> wgpu::Texture {
        let tex = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("atlas"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(w),
                rows_per_image: Some(h),
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        tex
    }

    fn bind(&self, tex: &wgpu::Texture) -> wgpu::BindGroup {
        let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ui-bg"),
            layout: &self.bind_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        })
    }

    /// Build one drawable per run (rects in their own white-texel run, then a run per text atlas).
    /// Returns the drawables + the atlas textures that must outlive the render pass.
    fn build_drawables(
        &self,
        width: u32,
        height: u32,
        rects: &[RectRun],
        texts: &[TextRun<'_>],
    ) -> (Vec<Drawable>, Vec<wgpu::Texture>) {
        let to_ndc = |px: f32, py: f32| {
            [
                px / width as f32 * 2.0 - 1.0,
                1.0 - py / height as f32 * 2.0,
            ]
        };
        let push_quad = |v: &mut Vec<Vertex>,
                         x: f32,
                         y: f32,
                         w: f32,
                         h: f32,
                         u0: f32,
                         vv0: f32,
                         u1: f32,
                         v1: f32,
                         rgba: [f32; 4]| {
            let tl = to_ndc(x, y);
            let tr = to_ndc(x + w, y);
            let bl = to_ndc(x, y + h);
            let br = to_ndc(x + w, y + h);
            v.push(Vertex {
                pos: tl,
                uv: [u0, vv0],
                rgba,
            });
            v.push(Vertex {
                pos: tr,
                uv: [u1, vv0],
                rgba,
            });
            v.push(Vertex {
                pos: bl,
                uv: [u0, v1],
                rgba,
            });
            v.push(Vertex {
                pos: tr,
                uv: [u1, vv0],
                rgba,
            });
            v.push(Vertex {
                pos: br,
                uv: [u1, v1],
                rgba,
            });
            v.push(Vertex {
                pos: bl,
                uv: [u0, v1],
                rgba,
            });
        };

        let mut drawables: Vec<Drawable> = Vec::new();
        if !rects.is_empty() {
            let mut v = Vec::with_capacity(rects.len() * 6);
            for r in rects {
                push_quad(&mut v, r.x, r.y, r.w, r.h, 0.5, 0.5, 0.5, 0.5, r.rgba);
            }
            let vbuf = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("rects"),
                    contents: bytemuck::cast_slice(&v),
                    usage: wgpu::BufferUsages::VERTEX,
                });
            drawables.push(Drawable {
                bind: self.bind(&self.white),
                vbuf,
                n: v.len() as u32,
            });
        }

        let mut atlas_texs: Vec<wgpu::Texture> = Vec::new();
        for run in texts {
            let tex = self.upload_coverage(run.atlas.width, run.atlas.height, &run.atlas.coverage);
            let mut v = Vec::with_capacity(run.quads.len() * 6);
            for q in &run.quads {
                push_quad(&mut v, q.x, q.y, q.w, q.h, q.u0, q.v0, q.u1, q.v1, run.rgba);
            }
            let vbuf = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("text"),
                    contents: bytemuck::cast_slice(&v),
                    usage: wgpu::BufferUsages::VERTEX,
                });
            drawables.push(Drawable {
                bind: self.bind(&tex),
                vbuf,
                n: v.len() as u32,
            });
            atlas_texs.push(tex);
        }
        (drawables, atlas_texs)
    }

    /// Record the scene (clear `bg`, then every drawable in order) into `view`.
    fn record_scene(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        bg: [f32; 3],
        drawables: &[Drawable],
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("ui-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: bg[0] as f64,
                        g: bg[1] as f64,
                        b: bg[2] as f64,
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
        pass.set_pipeline(&self.pipeline);
        for d in drawables {
            pass.set_bind_group(0, &d.bind, &[]);
            pass.set_vertex_buffer(0, d.vbuf.slice(..));
            pass.draw(0..d.n, 0..1);
        }
    }

    /// Copy `target` back to CPU as tightly-packed RGBA8 (`width*height*4`).
    fn read_target(
        &self,
        target: &wgpu::Texture,
        width: u32,
        height: u32,
        mut encoder: wgpu::CommandEncoder,
    ) -> Vec<u8> {
        let unpadded_bpr = width * 4;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_bpr = unpadded_bpr.div_ceil(align) * align;
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: (padded_bpr * height) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: target,
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
        self.queue.submit(std::iter::once(encoder.finish()));

        let (tx, rx) = mpsc::channel();
        readback.slice(..).map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        let _ = self.device.poll(wgpu::PollType::wait_indefinitely());
        rx.recv().expect("map channel").expect("buffer map failed");
        let data = readback.slice(..).get_mapped_range();
        let mut rgba = Vec::with_capacity((width * height * 4) as usize);
        for row in 0..height {
            let start = (row * padded_bpr) as usize;
            rgba.extend_from_slice(&data[start..start + unpadded_bpr as usize]);
        }
        drop(data);
        readback.unmap();
        rgba
    }

    /// Paint `rects` then `texts` over a cleared `bg`, at `width`×`height`, to a PNG at `path`.
    pub fn paint(
        &self,
        width: u32,
        height: u32,
        bg: [f32; 3],
        rects: &[RectRun],
        texts: &[TextRun<'_>],
        path: &str,
    ) {
        let (drawables, _keep) = self.build_drawables(width, height, rects, texts);
        let target = self.device.create_texture(&wgpu::TextureDescriptor {
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
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        self.record_scene(&mut encoder, &view, bg, &drawables);
        let rgba = self.read_target(&target, width, height, encoder);
        write_png(path, width, height, &rgba);
    }

    /// Render the scene, then recolour it through the engine's **palette dither** recipe
    /// ([`PalettePass`]: luminance → the curated `ramp`, Bayer 4×4 by `dither`), to an sRGB
    /// target. Returns the readback RGBA (so a golden test can diff it); writes a PNG too if
    /// `path` is `Some`. This is the engine-native FX look — NOT a port of the web `fxgl.js`.
    #[allow(clippy::too_many_arguments)]
    pub fn paint_palettized(
        &self,
        width: u32,
        height: u32,
        bg: [f32; 3],
        rects: &[RectRun],
        texts: &[TextRun<'_>],
        ramp: &[[f32; 3]],
        count: u32,
        dither: f32,
        path: Option<&str>,
    ) -> Vec<u8> {
        let (drawables, _keep) = self.build_drawables(width, height, rects, texts);
        let scene = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("fx-scene"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FMT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let scene_view = scene.create_view(&wgpu::TextureViewDescriptor::default());
        let target = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("fx-target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: SRGB,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let target_view = target.create_view(&wgpu::TextureViewDescriptor::default());

        let pal = PalettePass::new(&self.device, SRGB);
        pal.set_colors(&self.queue, ramp, count, dither, true);
        let pal_bg = pal.make_bind_group(&self.device, &scene_view);

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        self.record_scene(&mut encoder, &scene_view, bg, &drawables);
        pal.render(&mut encoder, &pal_bg, &target_view);
        let rgba = self.read_target(&target, width, height, encoder);
        if let Some(p) = path {
            write_png(p, width, height, &rgba);
        }
        rgba
    }
}

/// Write tightly-packed RGBA8 to a PNG.
pub fn write_png(path: &str, width: u32, height: u32, rgba: &[u8]) {
    let file = std::fs::File::create(path).expect("create png");
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), width, height);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.set_compression(png::Compression::Best); // dithered frames compress poorly — squeeze hard
    enc.write_header()
        .expect("png header")
        .write_image_data(rgba)
        .expect("png data");
}

// ── golden-diff layer ──────────────────────────────────────────────────────────────────────
// Pure CPU, so a golden-image test's *comparator* runs in CI without a GPU.

/// Decode a PNG to `(width, height, RGBA8)`.
pub fn rgba_from_png(path: &str) -> (u32, u32, Vec<u8>) {
    let file = std::fs::File::open(path).unwrap_or_else(|e| panic!("open {path}: {e}"));
    let decoder = png::Decoder::new(std::io::BufReader::new(file));
    let mut reader = decoder.read_info().expect("png info");
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).expect("png frame");
    buf.truncate(info.buffer_size());
    // Normalise to RGBA8 (the painter always writes RGBA8, but be defensive).
    let rgba = match info.color_type {
        png::ColorType::Rgba => buf,
        png::ColorType::Rgb => buf
            .chunks(3)
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        other => panic!("unsupported golden colour type {other:?}"),
    };
    (info.width, info.height, rgba)
}

/// Result of comparing two equal-length RGBA8 buffers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiffStats {
    /// Pixels with any channel differing by more than the tolerance.
    pub changed: usize,
    /// The largest single-channel absolute difference seen.
    pub max_delta: u8,
    /// Total pixel count.
    pub total: usize,
}

/// Per-pixel diff of two RGBA8 buffers: a pixel counts as `changed` if any channel differs by
/// more than `tol`. Panics if the buffers differ in length (a size mismatch is a hard fail).
pub fn diff(a: &[u8], b: &[u8], tol: u8) -> DiffStats {
    assert_eq!(
        a.len(),
        b.len(),
        "golden size mismatch ({} vs {} bytes)",
        a.len(),
        b.len()
    );
    let mut changed = 0usize;
    let mut max_delta = 0u8;
    for (pa, pb) in a.chunks_exact(4).zip(b.chunks_exact(4)) {
        let mut pixel_changed = false;
        for k in 0..4 {
            let d = pa[k].abs_diff(pb[k]);
            max_delta = max_delta.max(d);
            if d > tol {
                pixel_changed = true;
            }
        }
        if pixel_changed {
            changed += 1;
        }
    }
    DiffStats {
        changed,
        max_delta,
        total: a.len() / 4,
    }
}

/// True if `a` matches `b` within tolerance: no more than `max_fraction` of pixels differ by
/// more than `tol` per channel (a small allowance for software-rasteriser jitter).
pub fn matches(a: &[u8], b: &[u8], tol: u8, max_fraction: f32) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let d = diff(a, b, tol);
    (d.changed as f32) <= max_fraction * d.total as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    // The comparator has teeth: identical buffers match; an injected block of changed pixels is
    // counted and breaks the match. (Pure — no GPU; this is the CI-safe half of the golden test.)
    #[test]
    fn diff_detects_an_injected_change() {
        let base = vec![10u8; 64 * 64 * 4];
        assert!(
            matches(&base, &base, 0, 0.0),
            "a buffer must match itself exactly"
        );

        let mut perturbed = base.clone();
        // Tint an 8×8 block bright — a regression the diff must catch.
        for y in 0..8 {
            for x in 0..8 {
                let i = (y * 64 + x) * 4;
                perturbed[i] = 250;
            }
        }
        let d = diff(&base, &perturbed, 8);
        assert_eq!(d.changed, 64, "the 8×8 block (64 px) must be flagged");
        assert_eq!(d.max_delta, 240);
        assert!(
            !matches(&base, &perturbed, 8, 0.001),
            "a 64-px change must fail a tight match"
        );
    }

    #[test]
    fn size_mismatch_never_matches() {
        assert!(!matches(&[0u8; 16], &[0u8; 32], 255, 1.0));
    }
}
