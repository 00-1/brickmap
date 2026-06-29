//! The **windowed runtime** (mini-gate #4): boots the drill + keypad + correct-answer FX
//! FULLSCREEN through a real wgpu **surface** — desktop window or, packaged by `cargo-apk`, a
//! native Android APK (no voxel world). Touch/click hit-tests the data-free [`crate::keypad`];
//! the [`crate::drill`] consumes the T229 seam; a correct answer fires the engine-native FX
//! ([`crate::fx`]: particle burst + the palette-dither post). Native-only.
//!
//! It shares the engine recipes with the headless path — the same quad shader, and the same
//! `bm-render` [`PalettePass`] for the FX bloom — so what the APK shows on a phone matches the
//! golden the spike already self-verified offscreen.

use std::sync::Arc;
use std::time::Instant;

use ab_glyph::FontRef;
use brickmap::palette::PalettePass;
use wgpu::util::DeviceExt;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, TouchPhase, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

use crate::drill::{Drill, Mark};
use crate::headless::{RectRun, TextRun};
use crate::keypad::{Key, Keypad};
use crate::progression;
use crate::save::{RoundOutcome, RoundStep, Save};
use crate::text::{Atlas, Quad};
use brickmap::save::FileStore;
// The on-screen UI surface draws the engine's `ui2d` primitives with the engine's quad shader.
use brickmap::ui2d::UI_SHADER as SHADER;

// V26: web GG1's UI palette is a NEUTRAL near-black scheme (`#0E1116`); the home backdrop is the
// only screen that paints the purple HOME_PALETTE wash over it. The previous deep-violet defaults
// (`#140C22` / `#221636` / `#2E2048`) cast every other screen purple. Now all-neutral; only
// `home_frame` overlays its own purple bands.
const BG: [f32; 3] = [
    0x0e as f32 / 255.0,
    0x11 as f32 / 255.0,
    0x16 as f32 / 255.0,
];
const PANEL: [f32; 4] = [
    0x1c as f32 / 255.0,
    0x20 as f32 / 255.0,
    0x26 as f32 / 255.0,
    1.0,
];
const KEYBG: [f32; 4] = [
    0x16 as f32 / 255.0,
    0x1a as f32 / 255.0,
    0x20 as f32 / 255.0,
    1.0,
];
const INK: [f32; 4] = [
    0x08 as f32 / 255.0,
    0x0a as f32 / 255.0,
    0x0e as f32 / 255.0,
    1.0,
];
const GOLD: [f32; 4] = [1.0, 214.0 / 255.0, 110.0 / 255.0, 1.0];
const BODY: [f32; 4] = [
    0xe8 as f32 / 255.0,
    0xeb as f32 / 255.0,
    0xf2 as f32 / 255.0,
    1.0,
];
const DIM: [f32; 4] = [
    0x8c as f32 / 255.0,
    0x93 as f32 / 255.0,
    0xa0 as f32 / 255.0,
    1.0,
];
const GREEN: [f32; 4] = [
    0x3f as f32 / 255.0,
    0xce as f32 / 255.0,
    0x8c as f32 / 255.0,
    1.0,
];
/// Low-contrast build-watermark ink (a hair above the neutral near-black background).
const WATERMARK: [f32; 4] = [
    0x32 as f32 / 255.0,
    0x36 as f32 / 255.0,
    0x40 as f32 / 255.0,
    1.0,
];

/// How long the FX bloom plays after a correct answer.
const FX_SECS: f32 = 0.9;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    pos: [f32; 2],
    uv: [f32; 2],
    rgba: [f32; 4],
}

// The live render consumes the SAME [`RectRun`]/[`TextRun`] the headless painter does, so the
// drill-frame builder ([`drill_frame`]) is shared by the on-device path AND the golden test.

/// GPU surface state: device/queue/surface + the quad pipeline and the palette present pass.
struct Gfx {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    bind_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    white: wgpu::Texture,
    scene: wgpu::Texture,
    scene_view: wgpu::TextureView,
    present: PalettePass,
    present_bg: wgpu::BindGroup,
}

impl Gfx {
    async fn new(window: Arc<Window>) -> Gfx {
        let size = window.inner_size();
        let (w, h) = (size.width.max(1), size.height.max(1));
        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(window).expect("surface");
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("no GPU adapter");
        log::info!("goblin-gold adapter: {:?}", adapter.get_info());
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("gg-app-device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults()
                    .using_resolution(adapter.limits()),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            })
            .await
            .expect("device");

        let caps = surface.get_capabilities(&adapter);
        // Prefer an sRGB surface format (so the palette ramp lands as authored); fall back
        // defensively rather than index `[0]` blind — an empty caps list must not panic.
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .or_else(|| caps.formats.first().copied())
            .unwrap_or(wgpu::TextureFormat::Rgba8UnormSrgb);
        let alpha_mode = caps
            .alpha_modes
            .first()
            .copied()
            .unwrap_or(wgpu::CompositeAlphaMode::Auto);
        log::info!("goblin-gold surface: {w}x{h} fmt={format:?} alpha={alpha_mode:?}");
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: w,
            height: h,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("gg-ui"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("gg-ui-bgl"),
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
            label: Some("gg-ui-pl"),
            bind_group_layouts: &[Some(&bind_layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("gg-ui-pipe"),
            layout: Some(&pl),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
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
                    format,
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
        let white = make_white(&device, &queue);
        let (scene, scene_view) = make_scene(&device, format, w, h);
        let present = PalettePass::new(&device, format);
        let present_bg = present.make_bind_group(&device, &scene_view);

        Gfx {
            surface,
            device,
            queue,
            config,
            pipeline,
            bind_layout,
            sampler,
            white,
            scene,
            scene_view,
            present,
            present_bg,
        }
    }

    fn resize(&mut self, w: u32, h: u32) {
        if w == 0 || h == 0 {
            return;
        }
        self.config.width = w;
        self.config.height = h;
        self.surface.configure(&self.device, &self.config);
        let (scene, scene_view) = make_scene(&self.device, self.config.format, w, h);
        self.scene = scene;
        self.scene_view = scene_view;
        self.present_bg = self.present.make_bind_group(&self.device, &self.scene_view);
    }

    fn upload_coverage(&self, atlas: &Atlas) -> wgpu::Texture {
        let tex = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("gg-atlas"),
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
        self.queue.write_texture(
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
        tex
    }

    fn bind(&self, tex: &wgpu::Texture) -> wgpu::BindGroup {
        let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
        self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("gg-ui-bg"),
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

    /// Render `rects` then `texts` to the scene, then present to the swapchain through the palette
    /// pass — passthrough normally, or the gold Bayer dither while `fx_ramp` is `Some` (the bloom).
    fn render(
        &mut self,
        rects: &[RectRun],
        texts: &[TextRun<'_>],
        fx_ramp: Option<(&[[f32; 3]], u32, f32)>,
    ) {
        let (w, h) = (self.config.width as f32, self.config.height as f32);
        let to_ndc = |px: f32, py: f32| [px / w * 2.0 - 1.0, 1.0 - py / h * 2.0];
        let push = |v: &mut Vec<Vertex>,
                    x: f32,
                    y: f32,
                    ww: f32,
                    hh: f32,
                    uv: [f32; 4],
                    rgba: [f32; 4]| {
            let tl = to_ndc(x, y);
            let tr = to_ndc(x + ww, y);
            let bl = to_ndc(x, y + hh);
            let br = to_ndc(x + ww, y + hh);
            v.push(Vertex {
                pos: tl,
                uv: [uv[0], uv[1]],
                rgba,
            });
            v.push(Vertex {
                pos: tr,
                uv: [uv[2], uv[1]],
                rgba,
            });
            v.push(Vertex {
                pos: bl,
                uv: [uv[0], uv[3]],
                rgba,
            });
            v.push(Vertex {
                pos: tr,
                uv: [uv[2], uv[1]],
                rgba,
            });
            v.push(Vertex {
                pos: br,
                uv: [uv[2], uv[3]],
                rgba,
            });
            v.push(Vertex {
                pos: bl,
                uv: [uv[0], uv[3]],
                rgba,
            });
        };

        struct Run {
            bind: wgpu::BindGroup,
            vbuf: wgpu::Buffer,
            n: u32,
        }
        let mut runs: Vec<Run> = Vec::new();
        if !rects.is_empty() {
            let mut v = Vec::with_capacity(rects.len() * 6);
            for r in rects {
                push(&mut v, r.x, r.y, r.w, r.h, [0.5, 0.5, 0.5, 0.5], r.rgba);
            }
            let vbuf = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("gg-rects"),
                    contents: bytemuck::cast_slice(&v),
                    usage: wgpu::BufferUsages::VERTEX,
                });
            runs.push(Run {
                bind: self.bind(&self.white),
                vbuf,
                n: v.len() as u32,
            });
        }
        let mut keep: Vec<wgpu::Texture> = Vec::new();
        for t in texts {
            // Skip empty runs — an empty string (or chars absent from the atlas) yields zero
            // quads, and `create_buffer_init` panics on a zero-length slice. This is the exact
            // crash that force-closed the APK on the drill's first frame (empty answer box).
            if t.quads.is_empty() {
                continue;
            }
            let tex = self.upload_coverage(t.atlas);
            let mut v = Vec::with_capacity(t.quads.len() * 6);
            for q in &t.quads {
                push(&mut v, q.x, q.y, q.w, q.h, [q.u0, q.v0, q.u1, q.v1], t.rgba);
            }
            let vbuf = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("gg-text"),
                    contents: bytemuck::cast_slice(&v),
                    usage: wgpu::BufferUsages::VERTEX,
                });
            runs.push(Run {
                bind: self.bind(&tex),
                vbuf,
                n: v.len() as u32,
            });
            keep.push(tex);
        }

        use wgpu::CurrentSurfaceTexture as Cst;
        let frame = match self.surface.get_current_texture() {
            Cst::Success(f) | Cst::Suboptimal(f) => f,
            Cst::Timeout | Cst::Occluded => return,
            Cst::Outdated | Cst::Lost => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
            Cst::Validation => {
                log::error!("surface acquisition failed validation");
                return;
            }
        };
        let frame_view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        match fx_ramp {
            Some((ramp, count, dither)) => {
                self.present
                    .set_colors(&self.queue, ramp, count, dither, true)
            }
            None => self
                .present
                .set_colors(&self.queue, &[[0.0; 3]], 1, 0.0, false),
        }

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("gg-scene"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.scene_view,
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
            pass.set_pipeline(&self.pipeline);
            for r in &runs {
                pass.set_bind_group(0, &r.bind, &[]);
                pass.set_vertex_buffer(0, r.vbuf.slice(..));
                pass.draw(0..r.n, 0..1);
            }
        }
        self.present
            .render(&mut encoder, &self.present_bg, &frame_view);
        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
    }
}

fn make_white(device: &wgpu::Device, queue: &wgpu::Queue) -> wgpu::Texture {
    let white = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("gg-white"),
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
    white
}

fn make_scene(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    w: u32,
    h: u32,
) -> (wgpu::Texture, wgpu::TextureView) {
    let scene = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("gg-scene-tex"),
        size: wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = scene.create_view(&wgpu::TextureViewDescriptor::default());
    (scene, view)
}

/// Baked atlases sized to the current window (re-baked on resize). Public so the golden test can
/// render the same drill frame the device does (via [`drill_frame`]).
pub struct Fonts {
    head: Atlas,
    q: Atlas,
    body: Atlas,
    key: Atlas,
    tiny: Atlas,
}

/// Non-ASCII glyphs the game's text uses beyond printable ASCII: the math operators that appear in
/// drill prompts (`−` U+2212 minus · `×` times · `÷` divide · `²` squared), the `·` middot that
/// separates inline stat/label fields, and the `—`/`–` dashes used in event blurbs. Baked into every
/// prose face so prompts like "91 − 37" and blurbs like "bondfire — close …" render their symbols
/// (the default atlas is ASCII-only).
const GLYPHS: &str = "−×÷²·—–✓↓▶›";

impl Fonts {
    pub fn bake(font: &FontRef<'_>, h: f32) -> Fonts {
        Fonts {
            // V27: web GG1's headline ramp DOMINATES the screen (drill "144" + results "22.7s" each
            // ~⅕ of the height). Bump `head` (eyebrows) and especially `q` (the big drill prompt +
            // gold seconds count) so they read as the headlines they are.
            head: Atlas::bake_chars(font, (h * 0.058).clamp(24.0, 140.0), GLYPHS),
            q: Atlas::bake_chars(font, (h * 0.12).clamp(40.0, 220.0), GLYPHS),
            body: Atlas::bake_chars(font, (h * 0.030).clamp(16.0, 80.0), GLYPHS),
            key: Atlas::bake_chars(font, (h * 0.034).clamp(16.0, 90.0), "✓⌫"),
            // The build-watermark + chip face — small; also carries the math/middot glyphs for the
            // stat lines and gauntlet-size strings rendered at this size.
            tiny: Atlas::bake_chars(font, (h * 0.018).clamp(11.0, 40.0), GLYPHS),
        }
    }
}

fn centered(atlas: &Atlas, text: &str, cx: f32, top: f32, h: f32) -> Vec<Quad> {
    let w = atlas.text_width(text);
    atlas
        .layout(
            text,
            cx - w / 2.0,
            top + h / 2.0 - 0.59 * atlas.px,
            f32::INFINITY,
        )
        .0
}

/// V46: lay `text` as up-to-`max_lines` centred lines wrapped to `max_w`, top-anchored at
/// `(cx, top)`, each line `h` tall. Used by tile captions (codex BEASTS, arena-map foe names)
/// so long names ("Goblin Warrens · Brawn") wrap instead of truncating ("Goblin Warr…").
/// V30: render one stat PILL (rounded dark bg + "<val> <ABBR>" inline) at `(x, y)`, `h` tall.
/// Width adapts to the label so a 3-digit GRD fits cleanly next to a 1-digit FOC. Returns the
/// right edge so callers can lay chips in a row.
fn push_stat_chip<'a>(
    rects: &mut Vec<RectRun>,
    texts: &mut Vec<TextRun<'a>>,
    fonts: &'a Fonts,
    val: i64,
    abbr: &str,
    x: f32,
    y: f32,
    h: f32,
) -> f32 {
    let txt = format!("{val} {abbr}");
    // V30 v4 — agent gate caught v3: pills need a LIGHTER fill than the card PANEL (web's
    // chips read as "raised capsules ABOVE the card", not "holes punched into it"). PILL_BG
    // sits one luminance step lighter than PANEL (`#1c2026`) — distinct silhouette at every
    // render scale without the harsh inset look INK gave. Padding doubled for less cramped
    // chips (16 px total horizontal).
    const PILL_BG: [f32; 4] = [
        0x2c as f32 / 255.0,
        0x33 as f32 / 255.0,
        0x3d as f32 / 255.0,
        1.0,
    ];
    let pad = 16.0;
    let cw = fonts.tiny.text_width(&txt) + pad;
    rects.push(RectRun {
        x,
        y,
        w: cw,
        h,
        rgba: PILL_BG,
    });
    texts.push(TextRun {
        atlas: &fonts.tiny,
        quads: centered(&fonts.tiny, &txt, x + cw / 2.0, y, h),
        rgba: BODY,
    });
    x + cw
}

fn centered_wrapped(
    atlas: &Atlas,
    text: &str,
    cx: f32,
    top: f32,
    h: f32,
    max_w: f32,
    max_lines: usize,
) -> Vec<Quad> {
    let lines = wrap_lines(atlas, text, max_w, max_lines);
    let mut out: Vec<Quad> = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let lw = atlas.text_width(line);
        let (q, _) = atlas.layout(
            line,
            cx - lw / 2.0,
            top + i as f32 * h + h / 2.0 - 0.59 * atlas.px,
            f32::INFINITY,
        );
        out.extend(q);
    }
    out
}

/// Greedy word-wrap into at most `max_lines` lines that each fit `max_w`. If the text overflows
/// the line budget, the last line is allowed to overflow rather than dropping characters — we'd
/// rather see a slightly wide last line than a truncated label (V46).
fn wrap_lines(atlas: &Atlas, text: &str, max_w: f32, max_lines: usize) -> Vec<String> {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return vec![String::new()];
    }
    let mut lines: Vec<String> = Vec::new();
    let mut cur = String::new();
    for w in words {
        let trial = if cur.is_empty() {
            w.to_string()
        } else {
            format!("{cur} {w}")
        };
        if cur.is_empty() || atlas.text_width(&trial) <= max_w {
            cur = trial;
        } else {
            lines.push(std::mem::take(&mut cur));
            if lines.len() >= max_lines {
                cur = w.to_string();
                break;
            }
            cur = w.to_string();
        }
    }
    if !cur.is_empty() && lines.len() < max_lines {
        lines.push(cur);
    }
    lines
}

/// Which screen the app is showing.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Screen {
    /// The topic list (progression-gated): tap an unlocked topic to play it.
    Select,
    /// A drill round for the chosen topic.
    Drill,
    /// The metagame summary: what's been collected, the collector ladder, heroes, events.
    Collection,
    /// The collector ladder detail (tiers reached vs locked).
    Ladder,
    /// The end-of-round results summary (rank, awards, time, gold).
    Results,
    /// The 3v3 Arena: party-pick (unlocked heroes) → fight the next tier → grant. (Subsumes the old
    /// roster viewer — `arena_frame` shows each hero's portrait + effective stats too.)
    Arena,
    /// The daily-events detail.
    Events,
    /// The items-by-category detail.
    Items,
}

/// The current combo streak: trailing consecutive solves in the round (a skip resets it). GG1's
/// correct-chime pitch rises with this streak.
fn trailing_solves(steps: &[RoundStep]) -> u32 {
    steps
        .iter()
        .rev()
        .take_while(|s| matches!(s, RoundStep::Solve { .. }))
        .count() as u32
}

/// The generative-music scene (a [`crate::synth::STYLE_IDS`] name) that backs a given screen — the
/// arena bed on the hero roster, the menu bed everywhere else.
fn scene_for(screen: Screen) -> &'static str {
    match screen {
        Screen::Arena => "arena",
        _ => "menu",
    }
}

/// The wall clock as UTC epoch milliseconds (`u64` for save timestamps).
fn now_ms_u64() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// The wall clock as UTC epoch milliseconds (`i64` for the event schedule, which floors over it).
fn now_ms_i64() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// The full-width bottom button shared by Select ("Collection") and Collection ("Back"): its rect.
fn bottom_button(w: f32, h: f32) -> (f32, f32, f32, f32) {
    let bw = w * 0.6;
    let bh = h * 0.055;
    ((w - bw) / 2.0, h * 0.915, bw, bh)
}

/// Whether (`px`,`py`) hits the shared bottom button.
fn hit_bottom_button(w: f32, h: f32, px: f32, py: f32) -> bool {
    let (bx, by, bw, bh) = bottom_button(w, h);
    px >= bx && px < bx + bw && py >= by && py < by + bh
}

/// The live app: the topic graph + player progression, the current screen/drill, fonts, the FX
/// timer — driving [`Gfx`]. Phase 3 drives all 46 topics, gated by [`progression`].
struct App {
    font: FontRef<'static>,
    gfx: Option<Gfx>,
    window: Option<Arc<Window>>,
    fonts: Option<Fonts>,
    modes: Vec<progression::Mode>,
    progress: progression::Progress,
    /// The persisted game state (the `collected` keystone + gold + games). Round awards land here
    /// and progression is rebuilt from it, so the save is the single source of truth.
    save: Save,
    /// The durable backend the save persists through (a file under the platform's data dir); `None`
    /// if no writable dir was available — the game still runs, it just won't persist.
    store: Option<FileStore>,
    screen: Screen,
    drill: Option<Drill>,
    current: Option<String>,
    round_start: Option<Instant>,
    /// When the current question appeared (for per-question solve/spark timing).
    q_start: Option<Instant>,
    /// This round's steps in order (solves with their time, and skips) — feeds the solve/spark
    /// awards and the LIVE gold accrual (combo resets on a skip, so order matters).
    round_steps: Vec<RoundStep>,
    /// The most recent round's outcome — shown on the Results screen.
    last_outcome: Option<RoundOutcome>,
    /// The Arena party selection (≤3 unlocked hero ids) being assembled on the Arena screen.
    arena_party: Vec<String>,
    /// The most recent Arena battle outcome — shown as a banner on the Arena screen.
    last_arena: Option<crate::save::ArenaOutcome>,
    /// The id of the daily event the current drill is a gauntlet for (`None` = an ordinary topic
    /// drill). Routes [`finish_round`](App::finish_round) to the event award instead of the topic one.
    current_event: Option<String>,
    /// The most recent finished event-play run — shown as a banner on the Daily-Event screen.
    last_event: Option<EventOutcome>,
    /// The active Inventory (Items) tab (0..5 over [`INV_TABS`]).
    inv_tab: usize,
    keypad: Keypad,
    cursor: (f32, f32),
    fx_start: Option<Instant>,
    fx_seed: u32,
    /// The live audio engine (SFX + looping music bed) — `None` if no output device was available
    /// (the game still runs silent). Started lazily on the first `resumed`.
    audio: Option<crate::audio::Player>,
    /// The Android app handle — used to marshal the immersive-fullscreen JNI onto the UI thread.
    #[cfg(target_os = "android")]
    android_app: Option<winit::platform::android::activity::AndroidApp>,
}

impl App {
    /// Build the app, persisting the save under `data_dir` if given (the platform's writable app
    /// directory). Loads any existing save and seeds progression from it.
    fn new(data_dir: Option<std::path::PathBuf>) -> App {
        let font = FontRef::try_from_slice(crate::FONT_JETBRAINS_MONO).expect("font");
        let store = data_dir.and_then(|d| FileStore::open(d).ok());
        let save = store.as_ref().map(|s| Save::load(s)).unwrap_or_default();
        let progress = save.progress();
        App {
            font,
            gfx: None,
            window: None,
            fonts: None,
            modes: progression::modes(),
            progress,
            save,
            store,
            screen: Screen::Select,
            drill: None,
            current: None,
            round_start: None,
            q_start: None,
            round_steps: Vec::new(),
            last_outcome: None,
            arena_party: Vec::new(),
            last_arena: None,
            current_event: None,
            last_event: None,
            inv_tab: 0,
            keypad: Keypad::layout(0.0, 0.0, 1.0, 1.0, 0.0),
            cursor: (0.0, 0.0),
            fx_start: None,
            fx_seed: 0x6c1d_9e37,
            audio: None,
            #[cfg(target_os = "android")]
            android_app: None,
        }
    }

    /// Re-lay the keypad + re-bake fonts for the current surface size.
    fn relayout(&mut self) {
        let Some(gfx) = self.gfx.as_ref() else { return };
        let (w, h) = (gfx.config.width as f32, gfx.config.height as f32);
        self.fonts = Some(Fonts::bake(&self.font, h));
        let margin = w * 0.06;
        let kp_w = w - margin * 2.0;
        let kp_h = h * 0.40;
        let kp_y = h - kp_h - margin;
        self.keypad = Keypad::layout(margin, kp_y, kp_w, kp_h, w * 0.018);
    }

    /// Start a drill round for `id`: generate its questions and switch to the drill screen.
    fn start_drill(&mut self, id: &str) {
        self.drill = Some(Drill::from_topic(id));
        self.current = Some(id.to_string());
        let now = Instant::now();
        self.round_start = Some(now);
        self.q_start = Some(now);
        self.round_steps = Vec::new();
        self.fx_start = None;
        self.screen = Screen::Drill;
        self.sfx(crate::sfx::Sfx::RoundStart);
    }

    /// Start the daily **event** gauntlet for `eid`: build the deterministic gauntlet drill, flag the
    /// round as an event (so [`finish_round`](App::finish_round) awards event tiers, not topic gold),
    /// and switch to the drill screen. The drill loop/UI is shared with topic play.
    fn start_event(&mut self, eid: &str) {
        self.drill = Some(Drill::from_gauntlet(eid));
        self.current_event = Some(eid.to_string());
        self.current = None;
        let now = Instant::now();
        self.round_start = Some(now);
        self.q_start = Some(now);
        self.round_steps = Vec::new();
        self.fx_start = None;
        self.screen = Screen::Drill;
        self.sfx(crate::sfx::Sfx::RoundStart);
    }

    /// Fold a finished event gauntlet into the save: grant the `eventTiersEarned` keys (participation
    /// / well ≥ 0.7 / ace = flawless) — **no gold**, the reward IS the buff — then show the outcome
    /// banner back on the Daily-Event screen.
    fn finish_event(&mut self, eid: &str) {
        let total = self.drill.as_ref().map(|d| d.len() as u32).unwrap_or(0);
        let score = self.drill.as_ref().map(|d| d.solved()).unwrap_or(0);
        let ts = now_ms_u64();
        self.save.award_event(eid, score, total, ts);
        self.progress = self.save.progress();
        if let Some(store) = &self.store {
            let _ = self.save.store(store);
        }
        let keys = crate::event_play::event_tiers_earned(eid, score, total);
        let name = crate::event_play::roster()
            .into_iter()
            .find(|e| e.id == eid)
            .map(|e| e.name)
            .unwrap_or_else(|| eid.to_string());
        self.last_event = Some(EventOutcome {
            name,
            score,
            total,
            well: keys.iter().any(|k| k.ends_with(":well")),
            ace: keys.iter().any(|k| k.ends_with(":ace")),
        });
        self.drill = None;
        self.round_start = None;
        self.q_start = None;
        self.round_steps = Vec::new();
        self.fx_start = None;
        self.screen = Screen::Events;
        self.sfx(crate::sfx::Sfx::RoundComplete);
    }

    /// Fire a one-shot SFX through the audio engine (a no-op when there's no output device).
    fn sfx(&self, e: crate::sfx::Sfx) {
        if let Some(a) = &self.audio {
            a.play(e);
        }
    }

    /// Set the looping music bed to match the current screen. Uses GG1's own scene names: the menu
    /// bed under the UI/drill, the arena bed under the hero roster. (Beyond those literal names the
    /// drill→`menu` default is a conservative creative call — GG1's in-play music is style-pickable;
    /// the export carries no screen→scene map.) Cheap + idempotent (the player no-ops on no change).
    fn update_audio_scene(&self) {
        if let Some(a) = &self.audio {
            a.set_scene(Some(scene_for(self.screen)));
        }
    }

    /// Fold the finished round into progression (initiation + mastery) and return to the topic
    /// list — where any newly-unlocked topics now appear.
    fn finish_round(&mut self) {
        // A daily-event gauntlet awards event tiers (no gold) and returns to the Daily-Event screen.
        if let Some(eid) = self.current_event.take() {
            self.finish_event(&eid);
            return;
        }
        let total = self.drill.as_ref().map(|d| d.len() as u32).unwrap_or(0);
        // Answered = solved (skipped questions don't count): initiation needs ≥ half answered, and
        // mastery needs zero skips, so the skip count feeds straight into progression.
        let answered = self.drill.as_ref().map(|d| d.solved()).unwrap_or(0);
        let secs = self
            .round_start
            .map(|s| s.elapsed().as_secs_f64())
            .unwrap_or(f64::INFINITY);
        if let Some(id) = self.current.clone() {
            if let Some(m) = self.modes.iter().find(|m| m.id == id).cloned() {
                let run = progression::RunResult {
                    total,
                    answered,
                    total_time_secs: secs,
                };
                // Run the earning rule into the save (awards → the `collected` keystone), rebuild
                // progression from the save (its single source of truth), remember the last topic,
                // and persist — best-effort, so a write failure can't break play.
                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);
                let outcome = self.save.award_round(&m, &run, &self.round_steps, ts);
                self.save.last_mode = Some(m.id.clone());
                self.progress = self.save.progress();
                self.last_outcome = Some(outcome);
                if let Some(store) = &self.store {
                    let _ = self.save.store(store);
                }
            }
        }
        self.drill = None;
        self.current = None;
        self.round_start = None;
        self.q_start = None;
        self.round_steps = Vec::new();
        self.fx_start = None;
        // Show the per-run results summary (rank/awards/time/gold); tapping it returns to Select.
        self.screen = if self.last_outcome.is_some() {
            Screen::Results
        } else {
            Screen::Select
        };
        self.sfx(crate::sfx::Sfx::RoundComplete);
    }

    fn tap(&mut self, x: f32, y: f32) {
        let (w, h) = match self.gfx.as_ref() {
            Some(g) => (g.config.width as f32, g.config.height as f32),
            None => return,
        };
        match self.screen {
            Screen::Select => {
                // The bottom button opens the Collection; otherwise a topic row starts a drill.
                if hit_bottom_button(w, h, x, y) {
                    self.screen = Screen::Collection;
                } else if let Some(id) = topic_at(&self.modes, &self.progress, w, h, x, y) {
                    self.start_drill(&id);
                }
            }
            Screen::Collection => {
                if hit_bottom_button(w, h, x, y) {
                    self.screen = Screen::Select;
                } else if let Some(row) = collection_row_at(w, h, x, y) {
                    // Every stat row drills into its detail (owner: "nothing clickable").
                    // Rows: 0 Items · 1 Collector · 2 Topics · 3 Arena · 4 Events · 5 Gold.
                    let next = match row {
                        0 => Screen::Items,
                        1 => Screen::Ladder,
                        2 => Screen::Select, // topics live on the topic-select screen
                        3 => Screen::Arena,
                        4 => Screen::Events,
                        _ => Screen::Collection, // Gold has no detail (yet)
                    };
                    // Fresh Arena visit → clear the party pick + any stale outcome banner.
                    if next == Screen::Arena {
                        self.arena_party.clear();
                        self.last_arena = None;
                    }
                    // Fresh Daily-Event visit → clear any stale run banner.
                    if next == Screen::Events {
                        self.last_event = None;
                    }
                    self.screen = next;
                }
            }
            // The drill-down screens return to the Collection via their Back button.
            Screen::Ladder | Screen::Items => {
                if hit_bottom_button(w, h, x, y) {
                    self.screen = Screen::Collection;
                }
            }
            Screen::Events => {
                if hit_bottom_button(w, h, x, y) {
                    self.screen = Screen::Collection;
                } else if event_play_cta_hit(w, h, x, y) {
                    // Play today's live event → its gauntlet drill.
                    let eid = crate::event_play::live_event(now_ms_i64()).id;
                    self.start_event(&eid);
                }
            }
            Screen::Arena => {
                if hit_bottom_button(w, h, x, y) {
                    self.screen = Screen::Collection;
                } else if arena_fight_hit(w, h, x, y) && !self.arena_party.is_empty() {
                    // Resolve the fight against the next tier; on a win the save advances the tier.
                    let ts = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis() as u64)
                        .unwrap_or(0);
                    let party: Vec<&str> = self.arena_party.iter().map(String::as_str).collect();
                    if let Some(out) = self.save.resolve_arena(&party, ts) {
                        let win = out.win;
                        self.last_arena = Some(out);
                        self.progress = self.save.progress();
                        if let Some(store) = &self.store {
                            let _ = self.save.store(store);
                        }
                        self.sfx(if win {
                            crate::sfx::Sfx::RoundComplete
                        } else {
                            crate::sfx::Sfx::Skip
                        });
                    }
                } else if let Some(id) = arena_hero_at(&self.save, w, h, x, y) {
                    // Toggle the hero in/out of the party (capped at 3).
                    if let Some(pos) = self.arena_party.iter().position(|p| *p == id) {
                        self.arena_party.remove(pos);
                    } else if self.arena_party.len() < 3 {
                        self.arena_party.push(id);
                    }
                }
            }
            Screen::Results => {
                // Tap anywhere to continue back to the topic list.
                self.screen = Screen::Select;
            }
            Screen::Drill => {
                let mut done = false;
                // The per-keypress SFX, fired after the `self.drill` mutable borrow is released.
                let mut step_sfx: Option<crate::sfx::Sfx> = None;
                if let Some(d) = self.drill.as_mut() {
                    if let Some(key) = self.keypad.hit(x, y) {
                        // GG1 auto-accepts on the keypress that completes the answer (no submit
                        // key). Capture the current prompt before the press advances the question.
                        let prompt = d.prompt().to_string();
                        let before_solved = d.solved();
                        let before_consumed = d.consumed();
                        d.press(key);
                        if d.consumed() > before_consumed {
                            // A question was resolved (solved or skipped); record the step in order
                            // (gold combo resets on a skip), then restart the per-question timer.
                            if d.solved() > before_solved {
                                let t = self
                                    .q_start
                                    .map(|s| s.elapsed().as_secs_f64())
                                    .unwrap_or(0.0);
                                self.round_steps.push(RoundStep::Solve { prompt, dt: t });
                                // …and fire the celebration (FX + the combo-pitched chime).
                                self.fx_start = Some(Instant::now());
                                self.fx_seed ^= d.solved().wrapping_mul(2_654_435_761);
                                let combo = trailing_solves(&self.round_steps).saturating_sub(1);
                                step_sfx = Some(crate::sfx::Sfx::Correct { combo });
                            } else {
                                self.round_steps.push(RoundStep::Skip);
                                step_sfx = Some(crate::sfx::Sfx::Skip);
                            }
                            self.q_start = Some(Instant::now());
                        }
                    }
                    done = d.is_complete();
                }
                if let Some(e) = step_sfx {
                    self.sfx(e);
                }
                if done {
                    self.finish_round();
                }
            }
        }
        // Keep the music bed in step with whatever screen the tap landed on.
        self.update_audio_scene();
        if let Some(win) = &self.window {
            win.request_redraw();
        }
    }

    fn draw(&mut self) {
        if self.gfx.is_none() || self.fonts.is_none() {
            return;
        }
        let (w, h) = {
            let gfx = self.gfx.as_ref().unwrap();
            (gfx.config.width as f32, gfx.config.height as f32)
        };
        let fonts = self.fonts.as_ref().unwrap();
        let (rects, mut texts, fx_ramp) = match self.screen {
            Screen::Collection => {
                let (r, t) = collection_frame(&self.save, fonts, w, h);
                (r, t, None)
            }
            Screen::Ladder => {
                let keys = self.save.collected.keys().map(String::as_str);
                let items = crate::catalogue::earned(keys).len() as u32;
                let (r, t) = ladder_frame(items, fonts, w, h);
                (r, t, None)
            }
            Screen::Results => {
                let (r, t) = match &self.last_outcome {
                    Some(o) => results_frame(o, None, fonts, w, h),
                    // Defensive: no outcome (shouldn't happen) → an empty frame.
                    None => (Vec::new(), Vec::new()),
                };
                (r, t, None)
            }
            Screen::Arena => {
                let (r, t) = arena_frame(
                    &self.save,
                    &self.arena_party,
                    self.last_arena.as_ref(),
                    fonts,
                    w,
                    h,
                );
                (r, t, None)
            }
            Screen::Events => {
                let (r, t) = event_play_frame(
                    &self.save,
                    now_ms_i64(),
                    self.last_event.as_ref(),
                    fonts,
                    w,
                    h,
                );
                (r, t, None)
            }
            Screen::Items => {
                let (r, t) = items_frame(&self.save, self.inv_tab, now_ms_i64(), fonts, w, h);
                (r, t, None)
            }
            Screen::Select => {
                let (r, t) = topic_select_frame(&self.modes, &self.progress, fonts, w, h);
                (r, t, None)
            }
            Screen::Drill => {
                let fx_t = self.fx_start.map(|s| s.elapsed().as_secs_f32());
                let fx_active = matches!(fx_t, Some(t) if t < FX_SECS);
                let fx = if fx_active {
                    Some((self.fx_seed, fx_t.unwrap()))
                } else {
                    None
                };
                let d = self.drill.as_ref().expect("drill on the drill screen");
                // The live per-question timer (drives Spark scoring) — seconds since this question.
                let timer = self.q_start.map(|s| s.elapsed().as_secs_f32());
                let (r, t) = drill_frame(d, &self.keypad, fonts, w, h, fx, timer);
                let ramp = fx_active.then_some((
                    &crate::fx::GOLD_RAMP[..],
                    crate::fx::GOLD_RAMP.len() as u32,
                    crate::fx::FX_DITHER,
                ));
                (r, t, ramp)
            }
        };
        // Build watermark: a small, low-contrast SHA in the top-left corner so on-device screenshots
        // are traceable. Drawn here (the live path → every screen) and kept out of the goldens, so
        // they don't churn with each commit's SHA, and out of the hit-test (tap() never sees it).
        let tag = crate::build_tag();
        let (wq, _) = fonts.tiny.layout(&tag, w * 0.035, h * 0.012, w);
        texts.push(TextRun {
            atlas: &fonts.tiny,
            quads: wq,
            rgba: WATERMARK,
        });
        self.gfx.as_mut().unwrap().render(&rects, &texts, fx_ramp);
    }
}

/// Build the drill-screen frame (rects + text runs) for the current state, sized to `w`×`h`.
/// Shared by the on-device renderer ([`Gfx::render`]) AND the headless golden test, so the exact
/// frame the phone shows is the one self-verified offscreen. `fx = Some((seed, elapsed))` adds the
/// animated gold spark burst. NOTE: text runs may be **empty** (e.g. a blank answer box on the
/// first frame) — the renderer skips empty runs (the bug that crashed the APK), so this is safe.
pub fn drill_frame<'a>(
    drill: &Drill,
    keypad: &Keypad,
    fonts: &'a Fonts,
    w: f32,
    h: f32,
    fx: Option<(u32, f32)>,
    timer: Option<f32>,
) -> (Vec<RectRun>, Vec<TextRun<'a>>) {
    let cx = w / 2.0;
    let margin = w * 0.06;
    let col_w = w - margin * 2.0;
    let mut rects: Vec<RectRun> = Vec::new();
    let mut texts: Vec<TextRun> = Vec::new();

    // D1: drop the persistent topic/event title from the header — web's drill/event-play just shows
    // the progress counter on the LEFT and the per-question timer on the RIGHT.
    let prog = format!("{} / {}", drill.solved(), drill.len());
    let (q, _hh) = fonts.body.layout(&prog, margin, h * 0.035, col_w);
    texts.push(TextRun {
        atlas: &fonts.body,
        quads: q,
        rgba: DIM,
    });
    if let Some(t) = timer {
        let ts = format!("{t:.1}s");
        let tw = fonts.body.text_width(&ts);
        let (q, _) = fonts.body.layout(&ts, w - margin - tw, h * 0.035, tw + 4.0);
        texts.push(TextRun {
            atlas: &fonts.body,
            quads: q,
            rgba: BODY,
        });
    }

    // Top PROGRESS BAR (solved / len) — a thin track under the heading.
    let (pbx, pby, pbw, pbh) = (margin, h * 0.105, col_w, h * 0.006);
    rects.push(RectRun {
        x: pbx,
        y: pby,
        w: pbw,
        h: pbh,
        rgba: PANEL,
    });
    let frac = if drill.is_empty() {
        0.0
    } else {
        drill.solved() as f32 / drill.len() as f32
    };
    if frac > 0.0 {
        rects.push(RectRun {
            x: pbx,
            y: pby,
            w: pbw * frac,
            h: pbh,
            rgba: GOLD,
        });
    }

    // V34: the per-prompt cue ("half of ↓" / "solve ↓") shown ABOVE the big number — web's
    // `modes.js eyebrow`. Gold-ish (the only gold the drill carries — V35 says everything else is
    // neutral).
    texts.push(TextRun {
        atlas: &fonts.body,
        quads: centered(&fonts.body, &drill.eyebrow, cx, h * 0.135, h * 0.04),
        rgba: GOLD,
    });

    // The question — BORDERLESS large text (web's look; no card), the transform's prompt verbatim
    // (e.g. "100", "3 × 7", "area 10×7"). D2: rendered in BODY (white-ish) — web uses the neutral
    // drill look (not the gold theme).
    let cy = h * 0.19;
    let ch = h * 0.16;
    texts.push(TextRun {
        atlas: &fonts.q,
        quads: centered(&fonts.q, drill.prompt(), cx, cy, ch),
        rgba: BODY,
    });

    // The answer — BORDERLESS large text (web shows no box, no underline). V41: the empty state
    // renders a centred "– –" placeholder (dim) in place of the lone underline rule. The value is
    // the typed string (or revealed answer after Skip).
    let ink = match drill.last_mark() {
        Some(Mark::Right) => GREEN,
        Some(Mark::Skipped) => DIM,
        None => BODY,
    };
    let typed = drill.typed();
    let by = h * 0.35;
    let bh = h * 0.085;
    let (box_text, box_ink) = if let Some(r) = drill.revealed() {
        (r, ink)
    } else if typed.is_empty() {
        ("– –", DIM)
    } else {
        (typed, ink)
    };
    texts.push(TextRun {
        atlas: &fonts.q,
        quads: centered(&fonts.q, box_text, cx, by, bh),
        rgba: box_ink,
    });

    // Verdict banner — only shows after a verdict lands (web omits the "tap the digits" helper).
    if let Some((msg, col)) = match drill.last_mark() {
        Some(Mark::Right) => Some(("Correct!", GREEN)),
        Some(Mark::Skipped) => Some(("Skipped", DIM)),
        None => None,
    } {
        let mw = fonts.body.text_width(msg);
        let (q, _hh) = fonts.body.layout(msg, cx - mw / 2.0, h * 0.46, mw + 4.0);
        texts.push(TextRun {
            atlas: &fonts.body,
            quads: q,
            rgba: col,
        });
    }

    // Keypad zone — a "How to approach this" hint button above the calculator-order numpad, a pixel
    // backspace, and an OUTLINED Skip bar (web's subtle action bar, not a solid slab).
    let kp_top = keypad.cells.iter().map(|c| c.y).fold(f32::MAX, f32::min);
    let kp_left = keypad.cells.iter().map(|c| c.x).fold(f32::MAX, f32::min);
    let kp_right = keypad
        .cells
        .iter()
        .map(|c| c.x + c.w)
        .fold(f32::MIN, f32::max);
    let cell_h = keypad.cells.first().map(|c| c.h).unwrap_or(h * 0.07);
    // The hint button (outlined) just above the numpad — opens the "how to approach this" primer.
    let hint_h = cell_h * 0.82;
    let hint_y = kp_top - hint_h - h * 0.012;
    // V35: neutral DIM-outlined hint button (web keeps it grey — gold is for tiny accents only).
    rects.push(RectRun {
        x: kp_left,
        y: hint_y,
        w: kp_right - kp_left,
        h: hint_h,
        rgba: [DIM[0], DIM[1], DIM[2], 0.5],
    });
    rects.push(RectRun {
        x: kp_left + 2.0,
        y: hint_y + 2.0,
        w: kp_right - kp_left - 4.0,
        h: hint_h - 4.0,
        rgba: KEYBG,
    });
    texts.push(TextRun {
        atlas: &fonts.body,
        quads: centered(
            &fonts.body,
            "How to approach this",
            (kp_left + kp_right) / 2.0,
            hint_y,
            hint_h,
        ),
        rgba: BODY,
    });

    let mut key_quads: Vec<Quad> = Vec::new();
    for cell in &keypad.cells {
        match cell.key {
            Key::Enter => {
                // V35: NEUTRAL outlined Skip bar (DIM border + key-coloured fill + BODY label) —
                // web keeps Skip grey, not gold.
                rects.push(RectRun {
                    x: cell.x,
                    y: cell.y,
                    w: cell.w,
                    h: cell.h,
                    rgba: [DIM[0], DIM[1], DIM[2], 0.5],
                });
                rects.push(RectRun {
                    x: cell.x + 2.0,
                    y: cell.y + 2.0,
                    w: cell.w - 4.0,
                    h: cell.h - 4.0,
                    rgba: KEYBG,
                });
                texts.push(TextRun {
                    atlas: &fonts.key,
                    quads: centered(&fonts.key, "SKIP", cell.x + cell.w / 2.0, cell.y, cell.h),
                    rgba: BODY,
                });
            }
            Key::Back => {
                rects.push(RectRun {
                    x: cell.x,
                    y: cell.y,
                    w: cell.w,
                    h: cell.h,
                    rgba: KEYBG,
                });
                paint_backspace(
                    &mut rects,
                    cell.x + cell.w / 2.0,
                    cell.y + cell.h / 2.0,
                    cell.h * 0.07,
                );
            }
            Key::Digit(_) | Key::Dot => {
                rects.push(RectRun {
                    x: cell.x,
                    y: cell.y,
                    w: cell.w,
                    h: cell.h,
                    rgba: KEYBG,
                });
                let s = match cell.key {
                    Key::Digit(d) => ((b'0' + d) as char).to_string(),
                    Key::Dot => ".".to_string(),
                    _ => unreachable!(),
                };
                key_quads.extend(centered(
                    &fonts.key,
                    &s,
                    cell.x + cell.w / 2.0,
                    cell.y,
                    cell.h,
                ));
            }
        }
    }
    texts.push(TextRun {
        atlas: &fonts.key,
        quads: key_quads,
        rgba: BODY,
    });

    // FX: an animated gold spark burst over the answer (engine particle system).
    if let Some((seed, t)) = fx {
        let steps = (t * 120.0) as u32 + 4;
        for s in crate::fx::celebrate_steps(cx, by + bh * 0.4, w * 0.05, w * 0.012, seed, steps) {
            rects.push(RectRun {
                x: s.x - s.size / 2.0,
                y: s.y - s.size / 2.0,
                w: s.size,
                h: s.size,
                rgba: s.rgba,
            });
        }
    }

    (rects, texts)
}

/// Fixed dims for the initial-drill golden (portrait; tall enough for the bottom keypad).
pub const DRILL_W: u32 = 540;
pub const DRILL_H: u32 = 1000;

/// Render the **INITIAL drill frame** (empty answer box, no FX) through the headless painter —
/// the exact state that force-closed the APK on device (empty answer → empty text run). Shared by
/// the golden blesser (`fx_proto`) and the golden test, so both produce identical pixels; running
/// it at all proves the empty-run guard prevents the panic. Returns the readback RGBA.
pub fn render_initial_drill(painter: &crate::headless::Painter, font: &FontRef<'_>) -> Vec<u8> {
    let (w, h) = (DRILL_W as f32, DRILL_H as f32);
    let fonts = Fonts::bake(font, h);
    let drill = Drill::from_seam("halves");
    let margin = w * 0.06;
    let kp_w = w - margin * 2.0;
    let kp_h = h * 0.40;
    let kp_y = h - kp_h - margin;
    let keypad = Keypad::layout(margin, kp_y, kp_w, kp_h, w * 0.018);
    let (rects, texts) = drill_frame(&drill, &keypad, &fonts, w, h, None, None);
    painter.paint_rgba(DRILL_W, DRILL_H, BG, &rects, &texts)
}

// ── topic-select screen (phase 3: drive all 46 topics, progression-gated) ─────────────────────

// The topic grid's layout band (between the header and the Collection button) + spacing. The grid
// adapts to `count` so EVERY unlocked topic stays on-screen and tappable — a fixed row height ran
// the list off the bottom (and out of reach) once a player unlocked more than ~8 topics.
const TOPIC_TOP_FRAC: f32 = 0.17;
const TOPIC_BOT_FRAC: f32 = 0.895; // just above the bottom button (at 0.915)
const TOPIC_GAP_FRAC: f32 = 0.012;
const TOPIC_MAX_ROW_FRAC: f32 = 0.075; // the comfortable height (early game keeps the old look)
const TOPIC_MIN_ROW_FRAC: f32 = 0.012; // a floor so a fully-unlocked grid never collapses to nothing
const TOPIC_COL_GAP_FRAC: f32 = 0.03;
/// Beyond this many unlocked topics, split into a second column rather than shrink a tall list.
const TOPIC_ONE_COL_MAX: usize = 12;

/// How many columns the topic grid uses for `count` topics (1 while it fits comfortably, else 2).
fn topic_cols(count: usize) -> usize {
    if count > TOPIC_ONE_COL_MAX {
        2
    } else {
        1
    }
}

/// Rects for `count` topic rows, sized to `w`×`h`. Lays them out in [`topic_cols`] columns, filling
/// each column top-to-bottom, with the row height shrunk to fit the band so the whole grid is always
/// on-screen. Shared by the renderer ([`topic_select_frame`]) and the hit-test ([`topic_at`]).
fn topic_rows(count: usize, w: f32, h: f32) -> Vec<(f32, f32, f32, f32)> {
    if count == 0 {
        return Vec::new();
    }
    let margin = w * 0.06;
    let top = h * TOPIC_TOP_FRAC;
    let band = h * (TOPIC_BOT_FRAC - TOPIC_TOP_FRAC);
    let gap = h * TOPIC_GAP_FRAC;
    let cols = topic_cols(count);
    let rows_per_col = count.div_ceil(cols);
    let row_h =
        (band / rows_per_col as f32 - gap).clamp(h * TOPIC_MIN_ROW_FRAC, h * TOPIC_MAX_ROW_FRAC);
    let col_gap = w * TOPIC_COL_GAP_FRAC;
    let avail_w = w - margin * 2.0;
    let col_w = (avail_w - (cols as f32 - 1.0) * col_gap) / cols as f32;
    (0..count)
        .map(|i| {
            let col = i / rows_per_col;
            let row = i % rows_per_col;
            (
                margin + col as f32 * (col_w + col_gap),
                top + row as f32 * (row_h + gap),
                col_w,
                row_h,
            )
        })
        .collect()
}

/// The face for a topic row of height `row_h` — step down to a smaller atlas as the grid densifies
/// (a fully-unlocked 2-column grid needs the small face so labels still read).
fn topic_font(fonts: &Fonts, row_h: f32, h: f32) -> &Atlas {
    if row_h >= h * 0.05 {
        &fonts.q
    } else if row_h >= h * 0.032 {
        &fonts.body
    } else {
        &fonts.tiny
    }
}

/// Build the topic-select screen: heading + unlocked-count + a row per **unlocked** topic
/// (mastered rows in green, others gold). Shared by the on-device renderer + the golden, so the
/// list the phone shows is the one self-verified offscreen.
pub fn topic_select_frame<'a>(
    modes: &[progression::Mode],
    progress: &progression::Progress,
    fonts: &'a Fonts,
    w: f32,
    h: f32,
) -> (Vec<RectRun>, Vec<TextRun<'a>>) {
    let mut rects: Vec<RectRun> = Vec::new();
    let mut texts: Vec<TextRun> = Vec::new();
    let margin = w * 0.06;
    let col_w = w - margin * 2.0;

    let (q, _hh) = fonts.head.layout("Goblin Gold", margin, h * 0.05, col_w);
    texts.push(TextRun {
        atlas: &fonts.head,
        quads: q,
        rgba: GOLD,
    });

    let unlocked: Vec<&progression::Mode> =
        modes.iter().filter(|m| progress.is_unlocked(m)).collect();
    let count = format!("{} / {}", unlocked.len(), modes.len());
    let pw = fonts.body.text_width(&count);
    let (q, _hh) = fonts
        .body
        .layout(&count, w - margin - pw, h * 0.065, pw + 4.0);
    texts.push(TextRun {
        atlas: &fonts.body,
        quads: q,
        rgba: DIM,
    });

    for (m, (rx, ry, rw, rh)) in unlocked.iter().zip(topic_rows(unlocked.len(), w, h)) {
        rects.push(RectRun {
            x: rx,
            y: ry,
            w: rw,
            h: rh,
            rgba: PANEL,
        });
        let col = if progress.is_mastered(&m.id) {
            GREEN
        } else {
            GOLD
        };
        // Pick the face for this row's height, and clip the label to its column width.
        let font = topic_font(fonts, rh, h);
        let (q, _hh) = font.layout(&m.name, rx + rw * 0.06, ry + rh / 2.0 - 0.59 * font.px, rw);
        texts.push(TextRun {
            atlas: font,
            quads: q,
            rgba: col,
        });
    }

    // The bottom button into the Collection (metagame) screen.
    push_button(&mut rects, &mut texts, fonts, "Collection", w, h);
    (rects, texts)
}

/// The Home hub's bottom-nav destinations (web's `Best · Items · Heroes · Arena · Setup`).
const HOME_NAV: [&str; 5] = ["Best", "Items", "Heroes", "Arena", "Setup"];
/// `main.js HOME_PALETTE` — the home backdrop's purple theme (`[bg, accent, light]`).
const HOME_PALETTE: [[f32; 3]; 3] = [
    [
        0x0e as f32 / 255.0,
        0x11 as f32 / 255.0,
        0x16 as f32 / 255.0,
    ],
    [
        0x9a as f32 / 255.0,
        0x5c as f32 / 255.0,
        0xf6 as f32 / 255.0,
    ],
    [
        0xcd as f32 / 255.0,
        0xa9 as f32 / 255.0,
        0xff as f32 / 255.0,
    ],
];

/// The **Home** hub (web's main screen): a purple backdrop with the gold-HOARD pile, a gold-bar header,
/// the daily-event banner, the topic grid, and the bottom nav. Built to the visual bar against
/// `home-web.png`. `now_ms` drives the event countdown deterministically.
pub fn home_frame<'a>(
    save: &Save,
    modes: &[progression::Mode],
    progress: &progression::Progress,
    now_ms: i64,
    fonts: &'a Fonts,
    w: f32,
    h: f32,
) -> (Vec<RectRun>, Vec<TextRun<'a>>) {
    let mut rects: Vec<RectRun> = Vec::new();
    let mut texts: Vec<TextRun> = Vec::new();
    let margin = w * 0.05;
    let col_w = w - margin * 2.0;

    // Purple backdrop wash (HOME_PALETTE bg→accent, vertical bands). The web home's purple is a
    // VIVID saturated band that crests around the tree zone (y≈0.35) and fades toward the dark header
    // and the gold pile — so the wash follows a parabolic envelope peaking mid-screen, not a flat
    // linear ramp (calibrated against `home-web.png`: ~[100,72,152] at y≈0.3).
    let bands = 22;
    for i in 0..bands {
        let yc = (i as f32 + 0.5) / bands as f32;
        let env = (1.0 - ((yc - 0.44) / 0.40).powi(2)).clamp(0.0, 1.0);
        let f = 0.86 * env;
        let c = [
            HOME_PALETTE[0][0] + (HOME_PALETTE[1][0] - HOME_PALETTE[0][0]) * f,
            HOME_PALETTE[0][1] + (HOME_PALETTE[1][1] - HOME_PALETTE[0][1]) * f,
            HOME_PALETTE[0][2] + (HOME_PALETTE[1][2] - HOME_PALETTE[0][2]) * f,
        ];
        rects.push(RectRun {
            x: 0.0,
            y: h * (i as f32 / bands as f32),
            w,
            h: h / bands as f32 + 1.0,
            rgba: [c[0], c[1], c[2], 1.0],
        });
    }

    // Gold-bar header — V38: coin glyph (pixel disc) instead of a plain square.
    let coin = h * 0.022;
    push_coin(&mut rects, margin, h * 0.022, coin, GOLD);
    let (q, _) = fonts.body.layout(
        &format!("{} Goblin Gold", crate::gold::fmt_gold(save.gold)),
        margin + coin * 1.4,
        h * 0.02,
        col_w,
    );
    texts.push(TextRun {
        atlas: &fonts.body,
        quads: q,
        rgba: GOLD,
    });

    // Daily-event banner (name + countdown + "Again").
    let ev = crate::event_play::live_event(now_ms);
    let (by, bh) = (h * 0.048, h * 0.088);
    rects.push(RectRun {
        x: margin,
        y: by,
        w: col_w,
        h: bh,
        rgba: INK,
    });
    // Eyebrow · title · countdown (web's 3-line banner structure).
    let (q, _) = fonts.tiny.layout(
        "TODAY'S EVENT",
        margin + col_w * 0.03,
        by + bh * 0.12,
        col_w,
    );
    texts.push(TextRun {
        atlas: &fonts.tiny,
        quads: q,
        rgba: DIM,
    });
    let (q, _) = fonts
        .body
        .layout(&ev.name, margin + col_w * 0.03, by + bh * 0.36, col_w * 0.7);
    texts.push(TextRun {
        atlas: &fonts.body,
        quads: q,
        rgba: GOLD,
    });
    let (q, _) = fonts.tiny.layout(
        &format!("new event in {}", event_countdown(now_ms)),
        margin + col_w * 0.03,
        by + bh * 0.74,
        col_w,
    );
    texts.push(TextRun {
        atlas: &fonts.tiny,
        quads: q,
        rgba: DIM,
    });
    let (aw, ax) = (col_w * 0.18, margin + col_w - col_w * 0.18);
    rects.push(RectRun {
        x: ax,
        y: by + bh * 0.2,
        w: aw,
        h: bh * 0.6,
        rgba: GOLD,
    });
    // "Again" once today's event is cleared (the `event:<id>` reward key is in the save), else "Play".
    let ev_label = if save.has(&format!("event:{}", ev.id)) {
        "Again"
    } else {
        "Play"
    };
    texts.push(TextRun {
        atlas: &fonts.tiny,
        quads: centered(
            &fonts.tiny,
            ev_label,
            ax + aw / 2.0,
            by + bh * 0.2,
            bh * 0.6,
        ),
        rgba: INK,
    });

    // The topic TREE-graph (web's signature home view; `main.js techGraph`/`topicParts`/`renderTree`).
    // Build the SPINE (the main chain, the topics whose unlock is NOT a mastery gate, ordered by
    // following the `Played(prev)` chain from the always-open root) and the BRANCH map (`Mastered(X)`
    // → the topic gated on mastering X). Each spine topic is a ROW; its 1–N PARTS run left→right
    // following the branch chain; a thin amber CHAIN connector drops to the next spine row, a purple
    // MASTERY connector sits between parts. Lower rows fall behind the hoard + bottom card (the web
    // tree scrolls under the fixed bottom UI), so the top ~8 rows read like the reference.
    let spine = home_spine(modes);
    let branch_of = home_branches(modes);
    let tree_top = h * 0.15;
    let row_pitch = h * 0.064;
    let node_w = w * 0.165;
    let node_h = h * 0.046;
    let hgap = w * 0.012;
    let cx = w / 2.0;
    // Tree rows that would fall into the card/pile region are clipped (the web tree scrolls under the
    // fixed bottom UI); the visible top ~8 rows read like the reference.
    let card_top = h * 0.66;
    let row_visible = |i: usize| tree_top + i as f32 * row_pitch + node_h <= card_top;

    // The gold-HOARD pile (seedHoard), drawn BEHIND the tree — confined to a bottom band (the coins
    // bank up the side walls and dip in the middle); the bottom card paints over its crest.
    // The home pile rides the CANONICAL log-scaled wealth fraction (`gold::hoard_level`, the value
    // `main.js homeFxState` feeds `seedHoard`) — NOT the fxgl `gold/(gold+K)` helper, which saturates
    // far too early (987M → ~1.0 full pile instead of the web's ~half pile).
    // V36 v6: agent gate (`e2e1569`) — pile texture landed, but v5 went TOO LOW (~30%) and
    // the SIDE WINGS were weak. Lift the band to 0.55 (web's ~45% coverage) and below the
    // coin loop shift wing coins higher per-column so the edges climb to mid-screen.
    let level = (crate::gold::hoard_level(save.gold) * 1.4).min(1.0);
    let band_top = 0.55;

    // The pile's BULK — a gold-mass gradient under the surface coins (the "imply the bulk, render the
    // surface" trick). Web's bottom is near-SOLID gold even at a moderate level; without this the
    // purple backdrop shows through the coin gaps. Ramps from transparent at the crest to a solid
    // deep-gold floor. The floor stays solid regardless of `level` (any non-trivial pile has one);
    // `level` only gates a poorer save toward less mass (the `0.5 + 0.5·level` envelope).
    // V36 v6: mass_top tracks band_top.
    let mass_top = 0.55;
    let lvlf = (0.5 + 0.5 * level as f32).clamp(0.0, 1.0);
    let mbands = 16;
    for i in 0..mbands {
        let yc = mass_top + (i as f32 + 0.5) / mbands as f32 * (1.0 - mass_top);
        let tt = (yc - mass_top) / (1.0 - mass_top);
        let a = (0.05 + tt * 0.45).clamp(0.0, 0.5) * lvlf;
        let g0 = [120.0 / 255.0, 84.0 / 255.0, 22.0 / 255.0]; // deep gold (GOLD_TONES darkest)
        let g1 = [156.0 / 255.0, 112.0 / 255.0, 34.0 / 255.0];
        let c = [
            g0[0] + (g1[0] - g0[0]) * tt,
            g0[1] + (g1[1] - g0[1]) * tt,
            g0[2] + (g1[2] - g0[2]) * tt,
        ];
        rects.push(RectRun {
            x: 0.0,
            y: h * (mass_top + i as f32 / mbands as f32 * (1.0 - mass_top)),
            w,
            h: h * (1.0 - mass_top) / mbands as f32 + 1.0,
            rgba: [c[0], c[1], c[2], a],
        });
    }
    for coin in crate::hoard::seed_hoard(level, 0x601d, &[], 480) {
        // V36 v4: 2.0× coins (v3's 2.4× over-blended into solid gold; v2's 1.8× was sparse).
        let s = coin.size as f32 * (w / 430.0) * 2.0;
        // V36 v6 side WINGS: coins near the walls (x close to 0 or 1) get a per-column lift so
        // the edges climb to mid-screen (web shows clear rising side-wings, not a flat band).
        let wall = ((coin.x as f32 - 0.5).abs() * 2.0).powf(1.5);
        let band_top_col = (band_top - wall * 0.18).max(0.30);
        let cyn = band_top_col + coin.y as f32 * (1.0 - band_top_col);
        rects.push(RectRun {
            x: coin.x as f32 * w - s / 2.0,
            y: cyn * h - s / 2.0,
            w: s,
            h: s * coin.aspect as f32,
            rgba: [
                coin.r as f32 / 255.0,
                coin.g as f32 / 255.0,
                coin.b as f32 / 255.0,
                1.0,
            ],
        });
    }

    // The topic TREE on top of the pile.
    for (i, m) in spine.iter().enumerate() {
        if !row_visible(i) {
            break; // rows are top-to-bottom; the rest are below the fold.
        }
        let parts = topic_parts(m, &branch_of);
        let n = parts.len() as f32;
        let total_w = n * node_w + (n - 1.0) * hgap;
        let mut nx = cx - total_w / 2.0;
        let ny = tree_top + i as f32 * row_pitch;
        for (j, p) in parts.iter().enumerate() {
            if j > 0 {
                // Horizontal MASTERY arrow (purple ►) before this part: a shaft + an arrowhead
                // pointing into the node, lit once the gate is crossed (web's directional edge).
                let cy = ny + node_h * 0.5;
                let pa = if progress.is_unlocked(p) { 0.95 } else { 0.35 };
                let pcol = [
                    HOME_PALETTE[2][0],
                    HOME_PALETTE[2][1],
                    HOME_PALETTE[2][2],
                    pa,
                ];
                rects.push(RectRun {
                    x: nx - hgap * 1.0,
                    y: cy - node_h * 0.045,
                    w: hgap * 0.6,
                    h: node_h * 0.09,
                    rgba: pcol,
                });
                arrow_right(
                    &mut rects,
                    nx - hgap * 0.42,
                    cy,
                    node_h * 0.18,
                    hgap * 0.42,
                    pcol,
                );
            }
            home_node(
                &mut rects, &mut texts, fonts, save, progress, p, nx, ny, node_w, node_h,
            );
            nx += node_w + hgap;
        }
        // Vertical CHAIN arrow (amber ▼) down to the next spine row: a shaft + an arrowhead landing
        // on the next node, so each edge reads as a discrete directional link (not one long spine).
        if i + 1 < spine.len() && row_visible(i + 1) {
            let lit = progress.is_unlocked(spine[i + 1]);
            let acol = [GOLD[0], GOLD[1], GOLD[2], if lit { 0.9 } else { 0.3 }];
            let head_h = (row_pitch - node_h) * 0.45;
            rects.push(RectRun {
                x: cx - w * 0.004,
                y: ny + node_h,
                w: w * 0.008,
                h: row_pitch - node_h - head_h,
                rgba: acol,
            });
            arrow_down(
                &mut rects,
                cx,
                ny + row_pitch - head_h,
                w * 0.018,
                head_h,
                acol,
            );
        }
    }

    // The current-topic card on the pile (web's "<glyph> <name>  N/N · best…" footer card).
    let card = spine.first().copied().unwrap_or(&modes[0]);
    let (cy, chh) = (h * 0.665, h * 0.085);
    rects.push(RectRun {
        x: margin,
        y: cy,
        w: col_w,
        h: chh,
        rgba: INK,
    });
    home_glyph(
        &mut rects,
        card.id.as_str(),
        margin + col_w * 0.02,
        cy + chh * 0.2,
        col_w * 0.14,
        chh * 0.6,
        false,
    );
    texts.push(TextRun {
        atlas: &fonts.body,
        quads: fonts
            .body
            .layout(
                &card.name,
                margin + col_w * 0.2,
                cy + chh * 0.18,
                col_w * 0.6,
            )
            .0,
        rgba: GOLD,
    });
    let (ch_have, ch_total) = mode_progress(save, &card.id);
    texts.push(TextRun {
        atlas: &fonts.tiny,
        quads: fonts
            .tiny
            .layout(
                &{
                    // D3: pull this topic's best time from the save when one exists; else "No best yet".
                    let detail = match save.best_time(&card.id) {
                        Some(t) => format!("{ch_have}/{ch_total} · best {t:.1}s"),
                        None => format!("{ch_have}/{ch_total} · No best yet"),
                    };
                    detail
                },
                margin + col_w * 0.2,
                cy + chh * 0.58,
                col_w * 0.7,
            )
            .0,
        rgba: DIM,
    });

    // Start / Practice / Guide — Start is the SOLID-gold primary (V17), the rest outlined.
    let cta_labels = ["Start", "Practice", "Guide"];
    let cgap = w * 0.02;
    let cw3 = (col_w - cgap * 2.0) / 3.0;
    let (cty, cth) = (h * 0.785, h * 0.06);
    for (i, label) in cta_labels.iter().enumerate() {
        let bx = margin + i as f32 * (cw3 + cgap);
        if i == 0 {
            rects.push(RectRun {
                x: bx,
                y: cty,
                w: cw3,
                h: cth,
                rgba: GOLD,
            });
            texts.push(TextRun {
                atlas: &fonts.key,
                quads: centered(&fonts.key, label, bx + cw3 / 2.0, cty, cth),
                rgba: INK,
            });
        } else {
            rects.push(RectRun {
                x: bx,
                y: cty,
                w: cw3,
                h: cth,
                rgba: [GOLD[0], GOLD[1], GOLD[2], 0.5],
            });
            rects.push(RectRun {
                x: bx + 2.0,
                y: cty + 2.0,
                w: cw3 - 4.0,
                h: cth - 4.0,
                rgba: KEYBG,
            });
            texts.push(TextRun {
                atlas: &fonts.key,
                quads: centered(&fonts.key, label, bx + cw3 / 2.0, cty, cth),
                rgba: GOLD,
            });
        }
    }

    // Bottom nav row.
    let ngap = w * 0.015;
    let nw = (col_w - ngap * 4.0) / 5.0;
    let (ny, nh) = (h * 0.93, h * 0.05);
    for (i, name) in HOME_NAV.iter().enumerate() {
        let nx = margin + i as f32 * (nw + ngap);
        rects.push(RectRun {
            x: nx,
            y: ny,
            w: nw,
            h: nh,
            rgba: KEYBG,
        });
        texts.push(TextRun {
            atlas: &fonts.tiny,
            quads: centered(&fonts.tiny, name, nx + nw / 2.0, ny, nh),
            rgba: DIM,
        });
    }
    (rects, texts)
}

/// The home tree's SPINE: the main chain (topics whose unlock is not a mastery gate), ordered by
/// following the `Played(prev)` chain from the always-open root, then any orphans appended. Mirrors
/// `main.js techGraph` `order`.
fn home_spine(modes: &[progression::Mode]) -> Vec<&progression::Mode> {
    use progression::Unlock;
    let spine_modes: Vec<&progression::Mode> = modes
        .iter()
        .filter(|m| !matches!(m.unlock, Unlock::Mastered(_)))
        .collect();
    let mut order: Vec<&progression::Mode> = Vec::new();
    let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
    let mut cur = spine_modes
        .iter()
        .find(|m| matches!(m.unlock, Unlock::Always))
        .or_else(|| spine_modes.first())
        .copied();
    while let Some(c) = cur {
        if seen.contains(c.id.as_str()) {
            break;
        }
        order.push(c);
        seen.insert(c.id.as_str());
        let last = order[order.len() - 1].id.as_str();
        cur = spine_modes
            .iter()
            .find(|m| matches!(&m.unlock, Unlock::Played(x) if x == last))
            .copied();
    }
    for m in &spine_modes {
        if seen.insert(m.id.as_str()) {
            order.push(m);
        }
    }
    order
}

/// The branch map: `id → the topic whose unlock is `Mastered(id)`` (the off-spine part-chain edges).
fn home_branches(
    modes: &[progression::Mode],
) -> std::collections::HashMap<&str, &progression::Mode> {
    let mut m = std::collections::HashMap::new();
    for mode in modes {
        if let progression::Unlock::Mastered(x) = &mode.unlock {
            m.insert(x.as_str(), mode);
        }
    }
    m
}

/// The full part-chain for a spine topic: `[m, branch_of[m], branch_of[that], …]` (`main.js topicParts`).
fn topic_parts<'a>(
    m: &'a progression::Mode,
    branch_of: &std::collections::HashMap<&str, &'a progression::Mode>,
) -> Vec<&'a progression::Mode> {
    let mut parts = vec![m];
    let mut cur = m;
    while let Some(next) = branch_of.get(cur.id.as_str()) {
        parts.push(next);
        cur = next;
    }
    parts
}

/// Per-topic collection progress (`have`/`total`) — `main.js modeProgress`: the catalogue items
/// tagged with this topic's `modeId`, and how many the save has collected.
fn mode_progress(save: &Save, mode_id: &str) -> (usize, usize) {
    let items: Vec<String> = crate::catalogue::catalog()
        .into_iter()
        .filter(|c| c.mode_id.as_deref() == Some(mode_id))
        .map(|c| c.id)
        .collect();
    let have = items.iter().filter(|id| save.has(id)).count();
    (have, items.len())
}

/// A small downward ▼ arrowhead (a centred stack of narrowing axis-aligned rects — RectRun has no
/// triangles), apex at `(cx, top + height)`.
fn arrow_down(rects: &mut Vec<RectRun>, cx: f32, top: f32, half: f32, height: f32, rgba: [f32; 4]) {
    let steps = 4;
    for k in 0..steps {
        let t = k as f32 / steps as f32;
        let hw = half * (1.0 - t);
        rects.push(RectRun {
            x: cx - hw,
            y: top + t * height,
            w: hw * 2.0,
            h: height / steps as f32 + 0.7,
            rgba,
        });
    }
}

/// A small pixel **★ filled star** drawn as art (the bundled font has no U+2605 glyph), top-left
/// at `(x, y)`, `size` tall. Used as the "rating" prefix on hero cards / hero-detail (V28).
/// V48: pixel crossed-swords ⚔ glyph (7×7) — top-left at `(x, y)`, `size` tall. Stands in for
/// U+2694 across the arena screens (the bundled font has no ⚔).
fn push_swords(rects: &mut Vec<RectRun>, x: f32, y: f32, size: f32, rgba: [f32; 4]) {
    const SW: [&str; 7] = [
        "#.....#", "##...##", ".##.##.", "..###..", ".##.##.", "##...##", "#.....#",
    ];
    let cell = size / 7.0;
    for (r, row) in SW.iter().enumerate() {
        for (c, &b) in row.as_bytes().iter().enumerate() {
            if b == b'#' {
                rects.push(RectRun {
                    x: x + c as f32 * cell,
                    y: y + r as f32 * cell,
                    w: cell + 0.6,
                    h: cell + 0.6,
                    rgba,
                });
            }
        }
    }
}

/// V39: pixel calendar glyph (5×6) — the top "spine" + grid of date pips. Top-left at `(x, y)`,
/// `size` tall. Replaces the plain green square in the results MOMENTUM pill.
fn push_calendar(rects: &mut Vec<RectRun>, x: f32, y: f32, size: f32, rgba: [f32; 4]) {
    const CAL: [&str; 6] = ["#...#", "#####", "#####", "#.#.#", "#####", "#.#.#"];
    let cell = size / 6.0;
    for (r, row) in CAL.iter().enumerate() {
        for (c, &b) in row.as_bytes().iter().enumerate() {
            if b == b'#' {
                rects.push(RectRun {
                    x: x + c as f32 * cell,
                    y: y + r as f32 * cell,
                    w: cell + 0.6,
                    h: cell + 0.6,
                    rgba,
                });
            }
        }
    }
}

/// V39: pixel coin glyph (7×7) — round-ish disc with a centre `$`-like notch. Used in place of
/// a plain square wherever a gold-coin icon belongs (header gold tally, results gold tag).
fn push_coin(rects: &mut Vec<RectRun>, x: f32, y: f32, size: f32, rgba: [f32; 4]) {
    const C: [&str; 7] = [
        ".#####.", "#######", "###.###", "##.#.##", "###.###", "#######", ".#####.",
    ];
    let cell = size / 7.0;
    for (r, row) in C.iter().enumerate() {
        for (c, &b) in row.as_bytes().iter().enumerate() {
            if b == b'#' {
                rects.push(RectRun {
                    x: x + c as f32 * cell,
                    y: y + r as f32 * cell,
                    w: cell + 0.6,
                    h: cell + 0.6,
                    rgba,
                });
            }
        }
    }
}

fn push_star(rects: &mut Vec<RectRun>, x: f32, y: f32, size: f32, rgba: [f32; 4]) {
    // V28 v4: 13×13 filled five-pointed star — derived from the regular pentagram geometry
    // (apex (6,0), upper arms (12,4)/(0,4), lower legs (10,12)/(2,12), inner vertices at
    // (4,4)/(8,4)/(11,8)/(7,11)/(5,11)/(1,8)). Each row's left/right outline is linearly
    // interpolated along the outline edges and filled inward, so the shape reads as a clean ★
    // (the v3 11×11 grid was too coarse and got read as a blob by the agent gate).
    const ST: [&str; 13] = [
        "......#......", // y=0  apex point
        "......#......", // y=1  apex stem
        ".....###.....", // y=2  apex widening
        ".....###.....", // y=3
        "#############", // y=4  arms reach full width (left/right arm tips)
        ".###########.", // y=5
        "..#########..", // y=6  body narrows toward bottom inner vertices
        "..#########..", // y=7
        ".###.###.###.", // y=8  legs split with central spine + V-gaps
        ".###.....###.", // y=9  clean legs only
        ".##.......##.", // y=10
        ".##.......##.", // y=11
        "##.........##", // y=12 leg tips at the bottom corners
    ];
    let cell = size / 13.0;
    for (r, row) in ST.iter().enumerate() {
        for (c, &b) in row.as_bytes().iter().enumerate() {
            if b == b'#' {
                rects.push(RectRun {
                    x: x + c as f32 * cell,
                    y: y + r as f32 * cell,
                    w: cell + 0.6,
                    h: cell + 0.6,
                    rgba,
                });
            }
        }
    }
}

/// A small pixel **checkmark** ✓ drawn as art (the bundled font has no U+2713 glyph), top-left at
/// `(x, y)`, `size` tall. Used for the "complete" badges (inventory rows + mastered tree nodes).
fn push_check(rects: &mut Vec<RectRun>, x: f32, y: f32, size: f32, rgba: [f32; 4]) {
    const CK: [&str; 5] = ["......", ".....#", "#...#.", ".#.#..", "..#..."];
    let cell = size / 5.0;
    for (r, row) in CK.iter().enumerate() {
        for (c, &b) in row.as_bytes().iter().enumerate() {
            if b == b'#' {
                rects.push(RectRun {
                    x: x + c as f32 * cell,
                    y: y + r as f32 * cell,
                    w: cell + 0.6,
                    h: cell + 0.6,
                    rgba,
                });
            }
        }
    }
}

/// A small rightward ► arrowhead, apex at `(left + width, cy)`.
fn arrow_right(
    rects: &mut Vec<RectRun>,
    left: f32,
    cy: f32,
    half: f32,
    width: f32,
    rgba: [f32; 4],
) {
    let steps = 4;
    for k in 0..steps {
        let t = k as f32 / steps as f32;
        let hh = half * (1.0 - t);
        rects.push(RectRun {
            x: left + t * width,
            y: cy - hh,
            w: width / steps as f32 + 0.7,
            h: hh * 2.0,
            rgba,
        });
    }
}

/// Paint one topic glyph (the `glyphs.rs` ink grid) as little squares inside the box, body
/// `#E6E9EF` / accent `#F5B544` (dimmed when locked).
fn home_glyph(
    rects: &mut Vec<RectRun>,
    id: &str,
    gx: f32,
    gy: f32,
    gw: f32,
    gh: f32,
    locked: bool,
) {
    let g = crate::glyphs::build_grid(crate::glyphs::topic_glyph(id));
    if g.w == 0 {
        return;
    }
    let cell = (gw / g.w as f32).min(gh / g.h as f32).floor().max(1.0);
    let ox = gx + ((gw - cell * g.w as f32) / 2.0).floor();
    let oy = gy + ((gh - cell * g.h as f32) / 2.0).floor();
    // Locked nodes recede hard (web only keeps the unlocked frontier bright) — a near-backdrop dim.
    let body = if locked {
        [0.32, 0.31, 0.40, 1.0]
    } else {
        [230.0 / 255.0, 233.0 / 255.0, 239.0 / 255.0, 1.0]
    };
    let accent = if locked {
        [0.30, 0.27, 0.36, 1.0]
    } else {
        [245.0 / 255.0, 181.0 / 255.0, 68.0 / 255.0, 1.0]
    };
    for y in 0..g.h {
        for x in 0..g.w {
            let v = g.cells[y * g.w + x];
            if v == 0 {
                continue;
            }
            rects.push(RectRun {
                x: ox + x as f32 * cell,
                y: oy + y as f32 * cell,
                w: cell,
                h: cell,
                rgba: if v == 2 { accent } else { body },
            });
        }
    }
}

/// One tree node: a dark card with the topic glyph, a state badge (green ✓ when mastered), and the
/// `have/total` progress. Locked topics render dimmer.
#[allow(clippy::too_many_arguments)]
fn home_node<'a>(
    rects: &mut Vec<RectRun>,
    texts: &mut Vec<TextRun<'a>>,
    fonts: &'a Fonts,
    save: &Save,
    progress: &progression::Progress,
    m: &progression::Mode,
    x: f32,
    y: f32,
    nw: f32,
    nh: f32,
) {
    let unlocked = progress.is_unlocked(m);
    let mastered = progress.is_mastered(&m.id);
    // Unlocked nodes sit as raised dark cards; LOCKED nodes recede to a near-backdrop dim (web keeps
    // only the unlocked frontier bright — the `nodeState` locked/unlocked/done styling).
    let fill = if unlocked {
        [30.0 / 255.0, 26.0 / 255.0, 46.0 / 255.0, 1.0]
    } else {
        [20.0 / 255.0, 17.0 / 255.0, 30.0 / 255.0, 0.72]
    };
    rects.push(RectRun {
        x,
        y,
        w: nw,
        h: nh,
        rgba: fill,
    });
    // The glyph in the upper portion of the node.
    home_glyph(
        rects,
        &m.id,
        x + nw * 0.06,
        y + nh * 0.08,
        nw * 0.88,
        nh * 0.5,
        !unlocked,
    );
    // Progress + state badge along the bottom.
    let (have, total) = mode_progress(save, &m.id);
    let prog_col = if mastered {
        GOLD
    } else if unlocked {
        DIM
    } else {
        [0.34, 0.32, 0.42, 1.0] // locked count recedes with the node
    };
    texts.push(TextRun {
        atlas: &fonts.tiny,
        quads: centered(
            &fonts.tiny,
            &format!("{have}/{total}"),
            x + nw / 2.0,
            y + nh * 0.62,
            nh * 0.3,
        ),
        rgba: prog_col,
    });
    if mastered {
        // V37 corner ✓ BADGE — dark circular bg + green ✓ pinned top-right (web's "stamp"
        // treatment; replaces the earlier inline ✓ that read as just a tick over the card BG).
        let bs = nh * 0.34;
        let bx = x + nw - bs - 2.0;
        let by = y + 2.0;
        // Outer dark "circle" (square w/ rounded look once the cell ≥ ~4 px).
        rects.push(RectRun {
            x: bx,
            y: by,
            w: bs,
            h: bs,
            rgba: [0.0, 0.0, 0.0, 0.85],
        });
        // 1-px green border so the badge reads as a distinct cap, not just a hole.
        rects.push(RectRun {
            x: bx,
            y: by,
            w: bs,
            h: 1.0,
            rgba: [GREEN[0], GREEN[1], GREEN[2], 0.7],
        });
        rects.push(RectRun {
            x: bx,
            y: by + bs - 1.0,
            w: bs,
            h: 1.0,
            rgba: [GREEN[0], GREEN[1], GREEN[2], 0.7],
        });
        rects.push(RectRun {
            x: bx,
            y: by,
            w: 1.0,
            h: bs,
            rgba: [GREEN[0], GREEN[1], GREEN[2], 0.7],
        });
        rects.push(RectRun {
            x: bx + bs - 1.0,
            y: by,
            w: 1.0,
            h: bs,
            rgba: [GREEN[0], GREEN[1], GREEN[2], 0.7],
        });
        // Centred ✓ inside the badge.
        push_check(rects, bx + bs * 0.18, by + bs * 0.2, bs * 0.65, GREEN);
    }
}

/// Draw the shared bottom button (a gold bar with a dark inked label).
fn push_button<'a>(
    rects: &mut Vec<RectRun>,
    texts: &mut Vec<TextRun<'a>>,
    fonts: &'a Fonts,
    label: &str,
    w: f32,
    h: f32,
) {
    // OUTLINED treatment (gold border + dark fill + gold label) — web's subtle action bar, not a
    // bright solid-yellow slab (parity ledger V8/V12: the shared bottom bar across every screen).
    let (bx, by, bw, bh) = bottom_button(w, h);
    rects.push(RectRun {
        x: bx,
        y: by,
        w: bw,
        h: bh,
        rgba: [GOLD[0], GOLD[1], GOLD[2], 0.55],
    });
    rects.push(RectRun {
        x: bx + 2.0,
        y: by + 2.0,
        w: bw - 4.0,
        h: bh - 4.0,
        rgba: KEYBG,
    });
    texts.push(TextRun {
        atlas: &fonts.key,
        quads: centered(&fonts.key, label, bx + bw / 2.0, by, bh),
        rgba: GOLD,
    });
}

/// A neutral (non-gold) variant of [`push_button`] — DIM outline + BODY label. Used by screens
/// whose web reference keeps the bottom Back button grey-on-grey (e.g. Best Times, summary —
/// parity ledger V40).
fn push_neutral_button<'a>(
    rects: &mut Vec<RectRun>,
    texts: &mut Vec<TextRun<'a>>,
    fonts: &'a Fonts,
    label: &str,
    w: f32,
    h: f32,
) {
    let (bx, by, bw, bh) = bottom_button(w, h);
    rects.push(RectRun {
        x: bx,
        y: by,
        w: bw,
        h: bh,
        rgba: [DIM[0], DIM[1], DIM[2], 0.5],
    });
    rects.push(RectRun {
        x: bx + 2.0,
        y: by + 2.0,
        w: bw - 4.0,
        h: bh - 4.0,
        rgba: KEYBG,
    });
    texts.push(TextRun {
        atlas: &fonts.key,
        quads: centered(&fonts.key, label, bx + bw / 2.0, by, bh),
        rgba: BODY,
    });
}

/// The bottom **PRIMARY CTA** — a SOLID-gold bar with ink text (the prominent forward action, e.g.
/// Results "Continue"), vs the outlined secondary [`push_button`] ("Back"). Per parity ledger V17:
/// web reserves solid gold for the primary action, the outline for secondary nav.
fn push_cta<'a>(
    rects: &mut Vec<RectRun>,
    texts: &mut Vec<TextRun<'a>>,
    fonts: &'a Fonts,
    label: &str,
    w: f32,
    h: f32,
) {
    let (bx, by, bw, bh) = bottom_button(w, h);
    rects.push(RectRun {
        x: bx,
        y: by,
        w: bw,
        h: bh,
        rgba: GOLD,
    });
    texts.push(TextRun {
        atlas: &fonts.key,
        quads: centered(&fonts.key, label, bx + bw / 2.0, by, bh),
        rgba: INK,
    });
}

/// Build the **Collection** (metagame summary) screen from the save: items collected vs the
/// catalogue, the collector ladder's reached tier, topics initiated/mastered, the hero roster, and
/// events touched. Shared by the on-device renderer + the golden.
pub fn collection_frame<'a>(
    save: &Save,
    fonts: &'a Fonts,
    w: f32,
    h: f32,
) -> (Vec<RectRun>, Vec<TextRun<'a>>) {
    let mut rects: Vec<RectRun> = Vec::new();
    let mut texts: Vec<TextRun> = Vec::new();
    let margin = w * 0.06;
    let col_w = w - margin * 2.0;

    let keys: Vec<&str> = save.collected.keys().map(String::as_str).collect();
    let items = crate::catalogue::earned(keys.iter().copied()).len() as u32;
    let total = crate::catalogue::total();
    let ladder = crate::collector::earned(items);
    // The highest collector tier reached, else a hint toward the first one (ASCII — the baked atlas
    // has no em-dash).
    let tier = ladder.last().map(|t| t.name.clone()).unwrap_or_else(|| {
        crate::collector::ladder()
            .first()
            .map(|t| format!("next at {}", t.n))
            .unwrap_or_else(|| "none yet".to_string())
    });
    let count_pre = |p: &str| keys.iter().filter(|k| k.starts_with(p)).count();
    let modes = progression::modes().len();
    let events_touched = crate::events::touched(keys.iter().copied()).len();
    let tiers_cleared = count_pre("tier:");
    let arena_total = crate::combat::tier_count();

    let (q, _hh) = fonts.body.layout("COLLECTION", margin, h * 0.05, col_w);
    texts.push(TextRun {
        atlas: &fonts.body,
        quads: q,
        rgba: GOLD,
    });

    // Labelled stat rows.
    let rows = [
        ("Items".to_string(), format!("{items} / {total}")),
        ("Collector".to_string(), tier),
        (
            "Topics".to_string(),
            format!(
                "{}/{} played · {} mastered",
                count_pre("init:"),
                modes,
                count_pre("mastery:")
            ),
        ),
        (
            "Arena".to_string(),
            format!("{tiers_cleared} / {arena_total} cleared"),
        ),
        ("Events".to_string(), format!("{events_touched} / 14")),
        ("Gold".to_string(), format!("{}", save.gold as u64)),
    ];
    let (top, row_h, gap) = (
        h * COLLECTION_TOP_FRAC,
        h * COLLECTION_ROW_FRAC,
        h * COLLECTION_GAP_FRAC,
    );
    for (i, (label, value)) in rows.iter().enumerate() {
        let ry = top + i as f32 * (row_h + gap);
        rects.push(RectRun {
            x: margin,
            y: ry,
            w: col_w,
            h: row_h,
            rgba: PANEL,
        });
        let ty = ry + row_h / 2.0 - 0.59 * fonts.body.px;
        let (lq, _) = fonts.body.layout(label, margin + col_w * 0.05, ty, col_w);
        texts.push(TextRun {
            atlas: &fonts.body,
            quads: lq,
            rgba: DIM,
        });
        let vw = fonts.body.text_width(value);
        let (vq, _) = fonts
            .body
            .layout(value, margin + col_w * 0.95 - vw, ty, vw + 4.0);
        texts.push(TextRun {
            atlas: &fonts.body,
            quads: vq,
            rgba: BODY,
        });
    }

    push_neutral_button(&mut rects, &mut texts, fonts, "Back", w, h);
    (rects, texts)
}

// The Collection screen's stat-row layout (shared by the frame builder + the row hit-test).
const COLLECTION_TOP_FRAC: f32 = 0.18;
const COLLECTION_ROW_FRAC: f32 = 0.085;
const COLLECTION_GAP_FRAC: f32 = 0.02;
/// Which Collection stat row (0-based) contains (`px`,`py`), if any.
fn collection_row_at(w: f32, h: f32, px: f32, py: f32) -> Option<usize> {
    let margin = w * 0.06;
    let col_w = w - margin * 2.0;
    let (top, row_h, gap) = (
        h * COLLECTION_TOP_FRAC,
        h * COLLECTION_ROW_FRAC,
        h * COLLECTION_GAP_FRAC,
    );
    if px < margin || px > margin + col_w {
        return None;
    }
    (0..6).find(|&i| {
        let ry = top + i as f32 * (row_h + gap);
        py >= ry && py < ry + row_h
    })
}

/// Build the **Collector Ladder** screen: the collect-N tiers, those reached (green) vs locked
/// (dim), against the player's owned-item count. Reached by tapping the Collection's Collector row.
pub fn ladder_frame<'a>(
    items: u32,
    fonts: &'a Fonts,
    w: f32,
    h: f32,
) -> (Vec<RectRun>, Vec<TextRun<'a>>) {
    let mut rects: Vec<RectRun> = Vec::new();
    let mut texts: Vec<TextRun> = Vec::new();
    let margin = w * 0.06;
    let col_w = w - margin * 2.0;

    let (q, _hh) = fonts
        .head
        .layout("Collector Ladder", margin, h * 0.04, col_w);
    texts.push(TextRun {
        atlas: &fonts.head,
        quads: q,
        rgba: GOLD,
    });
    let sub = format!("{} / {} items", items, crate::catalogue::total());
    let (q, _hh) = fonts.body.layout(&sub, margin, h * 0.105, col_w);
    texts.push(TextRun {
        atlas: &fonts.body,
        quads: q,
        rgba: DIM,
    });

    let tiers = crate::collector::ladder();
    let top = h * 0.155;
    let row_h = h * 0.055;
    let gap = h * 0.008;
    for (i, t) in tiers.iter().enumerate() {
        let ry = top + i as f32 * (row_h + gap);
        rects.push(RectRun {
            x: margin,
            y: ry,
            w: col_w,
            h: row_h,
            rgba: PANEL,
        });
        let earned = items >= t.n;
        let col = if earned { GREEN } else { DIM };
        let ty = ry + row_h / 2.0 - 0.59 * fonts.body.px;
        let (lq, _) = fonts.body.layout(&t.name, margin + col_w * 0.05, ty, col_w);
        texts.push(TextRun {
            atlas: &fonts.body,
            quads: lq,
            rgba: col,
        });
        let label = format!("{} items", t.n);
        let vw = fonts.body.text_width(&label);
        let (vq, _) = fonts
            .body
            .layout(&label, margin + col_w * 0.95 - vw, ty, vw + 4.0);
        texts.push(TextRun {
            atlas: &fonts.body,
            quads: vq,
            rgba: if earned { GREEN } else { BODY },
        });
    }

    push_neutral_button(&mut rects, &mut texts, fonts, "Back", w, h);
    (rects, texts)
}

/// Render the **Collector Ladder** for a representative owned-count (some tiers earned, some not),
/// headless. Shared by the golden blesser + golden test.
pub fn render_ladder(painter: &crate::headless::Painter, font: &FontRef<'_>) -> Vec<u8> {
    let (w, h) = (DRILL_W as f32, DRILL_H as f32);
    let fonts = Fonts::bake(font, h);
    let (rects, texts) = ladder_frame(500, &fonts, w, h);
    painter.paint_rgba(DRILL_W, DRILL_H, BG, &rects, &texts)
}

/// Build the end-of-round **Results** screen from a [`RoundOutcome`]: the rank reached (prominent),
/// then accuracy/time/gold and the count of collectibles earned this run, with a Continue button.
pub fn results_frame<'a>(
    o: &RoundOutcome,
    momentum: Option<u32>,
    fonts: &'a Fonts,
    w: f32,
    h: f32,
) -> (Vec<RectRun>, Vec<TextRun<'a>>) {
    let mut rects: Vec<RectRun> = Vec::new();
    let mut texts: Vec<TextRun> = Vec::new();
    let margin = w * 0.06;
    let col_w = w - margin * 2.0;
    let cx = w / 2.0;

    // Starfield backdrop (deterministic scatter of dim pixels) — web's results sky.
    let mut rng = crate::synth::Rng::new(0x57a4_1e2d);
    for _ in 0..140 {
        let s = 1.4 + rng.next() as f32 * 2.4;
        let lum = 0.16 + rng.next() as f32 * 0.22;
        rects.push(RectRun {
            x: rng.next() as f32 * w,
            y: rng.next() as f32 * h,
            w: s,
            h: s,
            rgba: [lum, lum, lum * 1.1, 1.0],
        });
    }

    // V16: MOMENTUM pill — overlays the FINAL TIME header (web shows "MOMENTUM · N day(s)" when the
    // streak is positive; nothing when it's zero). A green calendar mark + the count + the label.
    if let Some(d) = momentum.filter(|d| *d > 0) {
        let pw = col_w * 0.46;
        let ph = h * 0.06;
        let px = cx - pw / 2.0;
        let py = h * 0.04;
        rects.push(RectRun {
            x: px,
            y: py,
            w: pw,
            h: ph,
            rgba: KEYBG,
        });
        // V39: calendar glyph — a small 5×6 pixel-art "calendar" instead of a plain square.
        let pip = ph * 0.5;
        let cal_x = px + ph * 0.18;
        let cal_y = py + (ph - pip) / 2.0;
        push_calendar(&mut rects, cal_x, cal_y, pip, GREEN);
        let label_x = px + ph * 0.18 + pip + ph * 0.18;
        let (q, _) = fonts.tiny.layout("MOMENTUM", label_x, py + ph * 0.16, pw);
        texts.push(TextRun {
            atlas: &fonts.tiny,
            quads: q,
            rgba: DIM,
        });
        let day_str = format!("{d} day{}", if d == 1 { "" } else { "s" });
        let (q, _) = fonts.body.layout(&day_str, label_x, py + ph * 0.44, pw);
        texts.push(TextRun {
            atlas: &fonts.body,
            quads: q,
            rgba: BODY,
        });
    }

    // The headline FINAL TIME — the run's reward in a speed game (gold, huge).
    texts.push(TextRun {
        atlas: &fonts.tiny,
        quads: centered(&fonts.tiny, "FINAL TIME", cx, h * 0.06, h * 0.022),
        rgba: DIM,
    });
    texts.push(TextRun {
        atlas: &fonts.q,
        quads: centered(
            &fonts.q,
            &format!("{:.1}s", o.total_time),
            cx,
            h * 0.085,
            h * 0.11,
        ),
        rgba: GOLD,
    });

    // RANK EARNED — its pixel portrait (the N1 item generator over the `rank:<key>` id) + name.
    let ry = h * 0.26;
    texts.push(TextRun {
        atlas: &fonts.tiny,
        quads: centered(&fonts.tiny, "RANK EARNED", cx, ry, h * 0.022),
        rgba: DIM,
    });
    let tile = h * 0.085;
    if let Some((id, rarity)) = rank_item(o.rank_idx) {
        if let Some((role, pal)) = crate::art::item_icon_for(&id, &rarity) {
            rects.push(RectRun {
                x: cx - tile - w * 0.06,
                y: ry + h * 0.03,
                w: tile,
                h: tile,
                rgba: INK,
            });
            paint_role(
                &mut rects,
                &role,
                &pal,
                cx - tile - w * 0.06,
                ry + h * 0.03,
                tile / 16.0,
            );
        }
    }
    texts.push(TextRun {
        atlas: &fonts.head,
        quads: {
            let (q, _) = fonts
                .head
                .layout(&o.rank_name, cx - w * 0.02, ry + h * 0.04, col_w);
            q
        },
        rgba: BODY,
    });

    // Accuracy + Skipped, as two centred stat columns (web's layout).
    let accuracy = if o.total > 0 {
        (o.answered * 100 + o.total / 2) / o.total
    } else {
        0
    };
    let skipped = o.total.saturating_sub(o.answered);
    let sy = h * 0.42;
    for (val, label, x) in [
        (format!("{accuracy}%"), "ACCURACY", w * 0.32),
        (format!("{skipped}"), "SKIPPED", w * 0.68),
    ] {
        texts.push(TextRun {
            atlas: &fonts.head,
            quads: centered(&fonts.head, &val, x, sy, h * 0.05),
            rgba: BODY,
        });
        texts.push(TextRun {
            atlas: &fonts.tiny,
            quads: centered(&fonts.tiny, label, x, sy + h * 0.055, h * 0.022),
            rgba: DIM,
        });
    }

    // Goblin Gold (with a coin pip) — V39: pixel coin glyph, not a plain square.
    let gy = h * 0.54;
    let coin = h * 0.026;
    push_coin(&mut rects, cx - w * 0.20, gy, coin, GOLD);
    texts.push(TextRun {
        atlas: &fonts.body,
        quads: {
            let (q, _) = fonts.body.layout(
                &format!("{} Goblin Gold", o.gold_earned),
                cx - w * 0.14,
                gy,
                col_w,
            );
            q
        },
        rgba: GOLD,
    });

    // SLOWEST ANSWERS section header (the per-question list needs round-step times — a follow-up; the
    // RoundOutcome doesn't carry them yet).
    texts.push(TextRun {
        atlas: &fonts.tiny,
        quads: {
            let (q, _) = fonts
                .tiny
                .layout("SLOWEST ANSWERS", margin, h * 0.63, col_w);
            q
        },
        rgba: DIM,
    });

    push_cta(&mut rects, &mut texts, fonts, "Continue", w, h);
    (rects, texts)
}

/// The catalogue Rank item (id + rarity) at rank tier `rank_idx` — for the Results rank portrait
/// (`item_icon("rank:<key>")`). The Rank category is added in tier order, so the nth is tier n.
fn rank_item(rank_idx: usize) -> Option<(String, String)> {
    crate::catalogue::catalog()
        .into_iter()
        .filter(|c| c.cat == crate::catalogue::Category::Rank)
        .nth(rank_idx)
        .map(|c| (c.id, c.rarity))
}

/// Render a representative **Results** screen (a strong-but-imperfect run), headless. Shared by the
/// golden blesser + golden test.
pub fn render_results(painter: &crate::headless::Painter, font: &FontRef<'_>) -> Vec<u8> {
    let (w, h) = (DRILL_W as f32, DRILL_H as f32);
    let fonts = Fonts::bake(font, h);
    let outcome = RoundOutcome {
        rank_idx: 16,
        rank_name: "Runelord".to_string(),
        newly: vec![
            "init:halves".to_string(),
            "flawless:halves".to_string(),
            "speed:halves:0".to_string(),
        ],
        gold_earned: 184,
        answered: 10,
        total: 10,
        total_time: 14.2,
    };
    let (rects, texts) = results_frame(&outcome, Some(1), &fonts, w, h);
    painter.paint_rgba(DRILL_W, DRILL_H, BG, &rects, &texts)
}

/// The representative end-of-round outcome (a flawless Runelord run) for the Results golden/ref.
fn results_sample() -> RoundOutcome {
    RoundOutcome {
        rank_idx: 16,
        rank_name: "Runelord".to_string(),
        newly: vec!["init:halves".to_string()],
        gold_earned: 184,
        answered: 10,
        total: 10,
        total_time: 14.2,
    }
}

/// Render the **Results** screen at the web reference aspect (430×880) — committed to halves as
/// `visual-ref/results-brickmap.png` for the Babysitter's side-by-side review.
pub fn render_results_ref(painter: &crate::headless::Painter, font: &FontRef<'_>) -> Vec<u8> {
    let (w, h) = (REF_W as f32, REF_H as f32);
    let fonts = Fonts::bake(font, h);
    let (rects, texts) = results_frame(&results_sample(), Some(1), &fonts, w, h);
    painter.paint_rgba(REF_W, REF_H, BG, &rects, &texts)
}

/// The three-letter section label for a hero type (the grouped `BRAWN` / `ARCANE` / `CUNNING`
/// headers, matching web GG1's `renderHeroes`).
fn hero_section_label(kind: crate::arena::Kind) -> &'static str {
    match kind {
        crate::arena::Kind::Brawn => "BRAWN",
        crate::arena::Kind::Arcane => "ARCANE",
        crate::arena::Kind::Cunning => "CUNNING",
    }
}

/// How many **owned** items boost hero `id` (the "Boosted by N" line). Mirrors `hero_stats`'
/// boost filter — an item counts once if its `boost.hero` targets this hero.
fn boost_count(id: &str, keys: &[&str]) -> usize {
    crate::catalogue::earned(keys.iter().copied())
        .into_iter()
        .filter(|item| item.boost.as_ref().is_some_and(|b| b.hero == id))
        .count()
}

/// One laid-out Heroes card row: its type, its rect `(x,y,w,h)`, and whether it's the first card of a
/// new type group (so the renderer draws a section header above it).
type HeroCardRow = (crate::arena::Kind, (f32, f32, f32, f32), bool);

/// The Heroes card-list layout: each card's row rect (single column), at a fixed comfortable height
/// with section headers between type groups. Cards past the frame clip off-screen (the list scrolls
/// on device — web GG1 captures the top of the same scroll). Shared by [`heroes_frame`] + a future
/// hit-test for the detail tap.
fn hero_card_rows(roster: &[crate::arena::Hero], w: f32, h: f32) -> Vec<HeroCardRow> {
    // (kind, rect, is_first_of_group) — the renderer draws a section header above each group's first.
    let margin = w * 0.05;
    let card_h = h * 0.115;
    let head_h = h * 0.04;
    let gap = h * 0.014;
    let mut out = Vec::new();
    let mut y = h * 0.10;
    let mut last_kind: Option<crate::arena::Kind> = None;
    for hero in roster {
        let first = last_kind != Some(hero.kind);
        if first {
            y += head_h;
            last_kind = Some(hero.kind);
        }
        out.push((hero.kind, (margin, y, w - margin * 2.0, card_h), first));
        y += card_h + gap;
    }
    out
}

/// The **Heroes** roster: a grouped, portrait-rich card list — section headers per type, a pixel
/// portrait (F1), a type dot + name, the `★`rating, the four **effective** stat chips, and a
/// "Boosted by N · tap for details" line. Built to the visual-parity bar against `heroes-web.png`.
pub fn heroes_frame<'a>(
    save: &Save,
    fonts: &'a Fonts,
    w: f32,
    h: f32,
) -> (Vec<RectRun>, Vec<TextRun<'a>>) {
    let mut rects: Vec<RectRun> = Vec::new();
    let mut texts: Vec<TextRun> = Vec::new();
    let margin = w * 0.05;
    let col_w = w - margin * 2.0;
    let keys: Vec<&str> = save.collected.keys().map(String::as_str).collect();
    let keyset: std::collections::HashSet<&str> = keys.iter().copied().collect();
    let roster = crate::arena::roster();
    let unlocked_n = roster
        .iter()
        .filter(|h| crate::combat::is_hero_unlocked(&h.id, &keyset))
        .count();

    // Centred header "HEROES  unlocked / total" — V42 ALL-CAPS chrome. v2: scale back from
    // `head` to `body` (web uses small tracked caps, not a giant headline — gate flagged the
    // v1 size as over-corrected).
    let title = format!("HEROES  {} / {}", unlocked_n, roster.len());
    texts.push(TextRun {
        atlas: &fonts.body,
        quads: centered(&fonts.body, &title, w / 2.0, h * 0.028, h * 0.034),
        rgba: GOLD,
    });

    for (hero, (kind, (rx, ry, rw, rh), first)) in roster.iter().zip(hero_card_rows(&roster, w, h))
    {
        // Section header above each type group's first card.
        if first {
            let (q, _) = fonts.tiny.layout(
                hero_section_label(kind),
                rx + col_w * 0.005,
                ry - rh * 0.34,
                col_w,
            );
            texts.push(TextRun {
                atlas: &fonts.tiny,
                quads: q,
                rgba: DIM,
            });
        }
        // Card panel.
        rects.push(RectRun {
            x: rx,
            y: ry,
            w: rw,
            h: rh,
            rgba: PANEL,
        });
        // Portrait tile (a darker box, web's 48×48).
        let tile = rh * 0.84;
        let tx0 = rx + rh * 0.08;
        let ty0 = ry + (rh - tile) / 2.0;
        rects.push(RectRun {
            x: tx0,
            y: ty0,
            w: tile,
            h: tile,
            rgba: INK,
        });
        let text_x = tx0 + tile + w * 0.03;

        // LOCKED heroes show a "?" portrait + dim name + the unlock hint (web's `renderHeroes`), in
        // place of stats/rating/boost. (In a full-collection save every hero is unlocked.)
        if !crate::combat::is_hero_unlocked(&hero.id, &keyset) {
            texts.push(TextRun {
                atlas: &fonts.q,
                quads: centered(
                    &fonts.q,
                    "?",
                    tx0 + tile / 2.0,
                    ty0 + tile * 0.1,
                    tile * 0.8,
                ),
                rgba: DIM,
            });
            let (q, _) = fonts.body.layout(&hero.name, text_x, ry + rh * 0.16, rw);
            texts.push(TextRun {
                atlas: &fonts.body,
                quads: q,
                rgba: DIM,
            });
            let (q, _) = fonts
                .tiny
                .layout(&hero.unlock_hint, text_x, ry + rh * 0.52, rw - tile);
            texts.push(TextRun {
                atlas: &fonts.tiny,
                quads: q,
                rgba: DIM,
            });
            continue;
        }

        let (role, pal) = crate::art::hero_icon(&hero.id, kind);
        paint_role(&mut rects, &role, &pal, tx0, ty0, tile / 16.0);
        let stats = crate::combat::effective_stats(&hero.id, &keyset).unwrap_or(hero.base);
        // Type dot + name.
        let dot = rh * 0.13;
        rects.push(RectRun {
            x: text_x,
            y: ry + rh * 0.16,
            w: dot,
            h: dot,
            rgba: type_rgba(kind),
        });
        let (q, _) = fonts
            .body
            .layout(&hero.name, text_x + dot * 1.6, ry + rh * 0.12, rw);
        texts.push(TextRun {
            atlas: &fonts.body,
            quads: q,
            rgba: BODY,
        });
        // ★ rating (right) — pixel star (V28 v5) + the count. Per Babysitter pixel spec: star
        // at the rank-number's cap-height (≈ 0.7 × font px ≈ rh * 0.14), positioned LEFT of the
        // number on the same baseline, no overlap with the trailing FOC chip below.
        let count = format!("{}", hero_rating(&stats));
        let ctw = fonts.body.text_width(&count);
        let sw = rh * 0.14;
        let star_x = rx + rw - ctw - sw * 1.4 - w * 0.02;
        push_star(&mut rects, star_x, ry + rh * 0.22, sw, GOLD);
        let (q, _) = fonts
            .body
            .layout(&count, star_x + sw * 1.3, ry + rh * 0.12, ctw + 4.0);
        texts.push(TextRun {
            atlas: &fonts.body,
            quads: q,
            rgba: GOLD,
        });
        // V30: per-stat PILL chips (rounded dark bg + bright number + dim 3-letter abbrev) — web's
        // stat-chip row. Adapt each chip width to its label so a 3-digit GRD doesn't collide.
        let chip_y = ry + rh * 0.44;
        let chip_h = rh * 0.22;
        let chip_gap = w * 0.012;
        let labels = [
            ("PWR", stats.power),
            ("GRD", stats.guard),
            ("SPD", stats.speed),
            ("FOC", stats.focus),
        ];
        let mut cx = text_x;
        for (lbl, val) in labels {
            cx = push_stat_chip(&mut rects, &mut texts, fonts, val, lbl, cx, chip_y, chip_h);
            cx += chip_gap;
        }
        // V31: "Boosted by N · tap for details ›" (gold + chevron) when boosted, neutral hint
        // otherwise — matches web's `renderHeroes` affordance.
        let n = boost_count(&hero.id, &keys);
        let boost = if n > 0 {
            format!("Boosted by {n} · tap for details ›")
        } else {
            "No items yet — collect to boost".to_string()
        };
        let (q, _) = fonts.tiny.layout(&boost, text_x, ry + rh * 0.7, rw - tile);
        texts.push(TextRun {
            atlas: &fonts.tiny,
            quads: q,
            rgba: if n > 0 { GOLD } else { DIM },
        });
    }

    push_neutral_button(&mut rects, &mut texts, fonts, "Back", w, h);
    (rects, texts)
}

/// `main.js STAT_ABBR` — the 3-letter chip label for a `boost.stat` value.
fn stat_abbr(stat: &str) -> &'static str {
    match stat {
        "power" => "PWR",
        "guard" => "GRD",
        "speed" => "SPD",
        "focus" => "FOC",
        _ => "?",
    }
}

/// The **Hero Detail** screen — the per-hero drill-down opened from the Heroes list. A header card
/// with the F1 portrait + type-coloured dot + name + type label + ★rating + 4 effective stat chips +
/// "N/N boosts collected"; then one row per boost item targeting this hero (rarity square + flavour
/// name + "+N STAT" gold tag, web's `renderHeroDetail`). Owned-only — locked heroes don't open.
pub fn hero_detail_frame<'a>(
    save: &Save,
    hero_id: &str,
    fonts: &'a Fonts,
    w: f32,
    h: f32,
) -> (Vec<RectRun>, Vec<TextRun<'a>>) {
    let mut rects: Vec<RectRun> = Vec::new();
    let mut texts: Vec<TextRun> = Vec::new();
    let margin = w * 0.05;
    let col_w = w - margin * 2.0;
    let keys: Vec<&str> = save.collected.keys().map(String::as_str).collect();
    let keyset: std::collections::HashSet<&str> = keys.iter().copied().collect();
    let hero = crate::arena::roster().into_iter().find(|h| h.id == hero_id);
    let Some(hero) = hero else {
        return (rects, texts);
    };
    let stats = crate::combat::effective_stats(&hero.id, &keyset).unwrap_or(hero.base);
    let rating = hero_rating(&stats);

    // "HERO" eyebrow.
    texts.push(TextRun {
        atlas: &fonts.tiny,
        quads: centered(&fonts.tiny, "HERO", w / 2.0, h * 0.02, h * 0.03),
        rgba: DIM,
    });

    // Header CARD — portrait + name + type + rating + stat chips + boosts-collected.
    let (cy, chh) = (h * 0.065, h * 0.18);
    rects.push(RectRun {
        x: margin,
        y: cy,
        w: col_w,
        h: chh,
        rgba: PANEL,
    });
    let tile = chh * 0.7;
    let (tx, ty) = (margin + col_w * 0.04, cy + chh * 0.14);
    rects.push(RectRun {
        x: tx,
        y: ty,
        w: tile,
        h: tile,
        rgba: INK,
    });
    let (role, pal) = crate::art::hero_icon(&hero.id, hero.kind);
    paint_role(&mut rects, &role, &pal, tx, ty, tile / 16.0);

    let text_x = tx + tile + w * 0.03;
    // Type dot + name + type label + ★ rating, on one line.
    let dot = chh * 0.09;
    rects.push(RectRun {
        x: text_x,
        y: cy + chh * 0.18,
        w: dot,
        h: dot,
        rgba: type_rgba(hero.kind),
    });
    let name_x = text_x + dot * 1.5;
    let (q, _) = fonts
        .body
        .layout(&hero.name, name_x, cy + chh * 0.14, col_w);
    texts.push(TextRun {
        atlas: &fonts.body,
        quads: q,
        rgba: BODY,
    });
    let name_w = fonts.body.text_width(&hero.name);
    let type_label = format!("{:?}", hero.kind).to_uppercase();
    let (q, _) = fonts.tiny.layout(
        &type_label,
        name_x + name_w + w * 0.02,
        cy + chh * 0.2,
        col_w,
    );
    texts.push(TextRun {
        atlas: &fonts.tiny,
        quads: q,
        rgba: DIM,
    });
    // V28 v6: ★ pixel star sized to the rating number's cap-height (chh * 0.12) — v5 used
    // chh*0.18 which read as oversized on the hero-detail card per the agent gate.
    let count = format!("{rating}");
    let ctw = fonts.body.text_width(&count);
    let sw = chh * 0.12;
    let count_x = margin + col_w - ctw - w * 0.02;
    let star_x = count_x - sw * 1.3;
    push_star(&mut rects, star_x, cy + chh * 0.21, sw, GOLD);
    let (q, _) = fonts
        .body
        .layout(&count, count_x, cy + chh * 0.14, ctw + 4.0);
    texts.push(TextRun {
        atlas: &fonts.body,
        quads: q,
        rgba: GOLD,
    });

    // 4 stat chips (PWR/GRD/SPD/FOC) — fit ALL 4 across the available width right of the portrait
    // (the V23 bug was anchoring to `col_w*0.78` from text_x, clipping the 4th chip "FOC" off the
    // card's right edge for high-stat heroes). Adapt each chip's width to its label so the row
    // fits cleanly even when the GRD count is 3 digits (e.g. Brannon's "561 GRD").
    let chip_y = cy + chh * 0.46;
    let chip_h = chh * 0.18;
    let chip_gap = w * 0.012;
    let labels = [
        ("PWR", stats.power),
        ("GRD", stats.guard),
        ("SPD", stats.speed),
        ("FOC", stats.focus),
    ];
    let chip_strs: Vec<String> = labels.iter().map(|(l, v)| format!("{v} {l}")).collect();
    let chip_pad = chip_h * 0.7;
    let chip_widths: Vec<f32> = chip_strs
        .iter()
        .map(|s| fonts.tiny.text_width(s) + chip_pad)
        .collect();
    let chips_total: f32 = chip_widths.iter().sum::<f32>() + chip_gap * 3.0;
    // The chip row can extend slightly past `text_x` if needed; cap at the card's right edge.
    let chips_right = margin + col_w - w * 0.02;
    let chips_left = (chips_right - chips_total).max(text_x);
    let mut cx_run = chips_left;
    for (i, txt) in chip_strs.iter().enumerate() {
        let cw = chip_widths[i];
        // V30 v4: lighter pill BG (`#2c333d`) above the card panel so the chip reads as a
        // raised capsule rather than an inset hole.
        const PILL_BG: [f32; 4] = [
            0x2c as f32 / 255.0,
            0x33 as f32 / 255.0,
            0x3d as f32 / 255.0,
            1.0,
        ];
        rects.push(RectRun {
            x: cx_run,
            y: chip_y,
            w: cw,
            h: chip_h,
            rgba: PILL_BG,
        });
        texts.push(TextRun {
            atlas: &fonts.tiny,
            quads: centered(&fonts.tiny, txt, cx_run + cw / 2.0, chip_y, chip_h),
            rgba: BODY,
        });
        cx_run += cw + chip_gap;
    }

    // "N / N boosts collected" — counts BOTH catalogue boosts (`boost.hero == hero.id`) and the
    // combat loot boosts targeting this hero (each owned `loot:*` id contributes).
    let cat_all: Vec<crate::catalogue::Collectible> = crate::catalogue::catalog()
        .into_iter()
        .filter(|c| c.boost.as_ref().is_some_and(|b| b.hero == hero.id))
        .collect();
    let owned: Vec<&crate::catalogue::Collectible> =
        cat_all.iter().filter(|c| save.has(&c.id)).collect();
    let loot_boosts = crate::combat::loot_boosts_for(&hero.id);
    let loot_owned = loot_boosts.iter().filter(|(id, _, _)| save.has(id)).count();
    let total_have = owned.len() + loot_owned;
    let total_all = cat_all.len() + loot_boosts.len();
    let prog = format!("{total_have} / {total_all} boosts collected");
    let (q, _) = fonts.body.layout(&prog, text_x, cy + chh * 0.74, col_w);
    texts.push(TextRun {
        atlas: &fonts.body,
        quads: q,
        rgba: GOLD,
    });

    // Boost rows (rarity square + flavour name + "+N STAT" gold tag), as many as fit.
    let list_top = cy + chh + h * 0.018;
    let row_h = h * 0.048;
    let rgap = h * 0.008;
    let bottom = h * 0.92;
    for (i, item) in owned.iter().enumerate() {
        let ry = list_top + i as f32 * (row_h + rgap);
        if ry + row_h > bottom {
            break;
        }
        rects.push(RectRun {
            x: margin,
            y: ry,
            w: col_w,
            h: row_h,
            rgba: PANEL,
        });
        // Rarity square (left).
        let sq = row_h * 0.36;
        rects.push(RectRun {
            x: margin + col_w * 0.025,
            y: ry + row_h * 0.32,
            w: sq,
            h: sq,
            rgba: rarity_rgba(&item.rarity),
        });
        // Flavour name (web's `it.flavour || it.name`).
        let display = if item.flavour.is_empty() {
            &item.name
        } else {
            &item.flavour
        };
        let (q, _) = fonts.body.layout(
            display,
            margin + col_w * 0.085,
            ry + row_h * 0.18,
            col_w * 0.65,
        );
        texts.push(TextRun {
            atlas: &fonts.body,
            quads: q,
            rgba: BODY,
        });
        // "+N STAT" right.
        if let Some(b) = &item.boost {
            let tag = format!("+{} {}", b.amount, stat_abbr(&b.stat));
            let tw = fonts.body.text_width(&tag);
            let (q, _) = fonts.body.layout(
                &tag,
                margin + col_w - tw - w * 0.02,
                ry + row_h * 0.18,
                tw + 4.0,
            );
            texts.push(TextRun {
                atlas: &fonts.body,
                quads: q,
                rgba: GOLD,
            });
        }
    }

    push_neutral_button(&mut rects, &mut texts, fonts, "Back", w, h);
    (rects, texts)
}

/// The 12 music-style labels (`synth.js SCENES.label`, in the web's Audio-panel grid order — 2 cols
/// × 6 rows). Pure presentation data — like the `TOPIC_GLYPHS` map, ported straight from source.
const MUSIC_STYLE_LABELS: [&str; 12] = [
    "Neon Lobby",
    "Lo-Fi Study",
    "Ambient Drift",
    "Chiptune Rush",
    "Synthwave Cruise",
    "Tropical Pluck",
    "Festival",
    "Hypno Techno",
    "Liquid DnB",
    "Phrygian Onslaught",
    "8-Bit Boss March",
    "Dubstep Victory",
];

/// The **Audio** screen — web's audio panel: AUDIO eyebrow, a Sound toggle card, two volume sliders
/// (Music · Sound FX, each with a "N/11" right-aligned count + a track + a square handle), a
/// "Music style" panel (12 button grid + "Auto" pill), then a "Test sound" Play card + Back.
pub fn audio_frame<'a>(fonts: &'a Fonts, w: f32, h: f32) -> (Vec<RectRun>, Vec<TextRun<'a>>) {
    let mut rects: Vec<RectRun> = Vec::new();
    let mut texts: Vec<TextRun> = Vec::new();
    let margin = w * 0.05;
    let col_w = w - margin * 2.0;

    // Eyebrow.
    texts.push(TextRun {
        atlas: &fonts.tiny,
        quads: centered(&fonts.tiny, "AUDIO", w / 2.0, h * 0.018, h * 0.03),
        rgba: DIM,
    });

    // Outlined panel helper.
    let panel = |rects: &mut Vec<RectRun>, y: f32, ph: f32| {
        rects.push(RectRun {
            x: margin,
            y,
            w: col_w,
            h: ph,
            rgba: [DIM[0], DIM[1], DIM[2], 0.5],
        });
        rects.push(RectRun {
            x: margin + 1.5,
            y: y + 1.5,
            w: col_w - 3.0,
            h: ph - 3.0,
            rgba: KEYBG,
        });
    };

    // Sound toggle card.
    let p1_y = h * 0.06;
    let p1_h = h * 0.07;
    panel(&mut rects, p1_y, p1_h);
    let (q, _) = fonts
        .body
        .layout("Sound", margin + col_w * 0.05, p1_y + p1_h * 0.28, col_w);
    texts.push(TextRun {
        atlas: &fonts.body,
        quads: q,
        rgba: BODY,
    });
    let on_w = fonts.body.text_width("On");
    let (q, _) = fonts.body.layout(
        "On",
        margin + col_w - on_w - p1_h * 0.5,
        p1_y + p1_h * 0.28,
        on_w + 4.0,
    );
    texts.push(TextRun {
        atlas: &fonts.body,
        quads: q,
        rgba: GOLD,
    });

    // Volume slider card (shared helper).
    let slider =
        |rects: &mut Vec<RectRun>, texts: &mut Vec<TextRun<'a>>, label: &str, val: u32, y: f32| {
            let ph = h * 0.10;
            panel(rects, y, ph);
            let (q, _) = fonts
                .body
                .layout(label, margin + col_w * 0.05, y + ph * 0.18, col_w);
            texts.push(TextRun {
                atlas: &fonts.body,
                quads: q,
                rgba: BODY,
            });
            let cnt = format!("{val} / 11");
            let cw = fonts.body.text_width(&cnt);
            let (q, _) = fonts.body.layout(
                &cnt,
                margin + col_w - cw - ph * 0.5,
                y + ph * 0.18,
                cw + 4.0,
            );
            texts.push(TextRun {
                atlas: &fonts.body,
                quads: q,
                rgba: GOLD,
            });
            // Track (thin DIM bar) + handle (gold square at val/11).
            let (tx, ty, tw, th) = (
                margin + col_w * 0.08,
                y + ph * 0.66,
                col_w * 0.84,
                ph * 0.05,
            );
            rects.push(RectRun {
                x: tx,
                y: ty,
                w: tw,
                h: th,
                rgba: DIM,
            });
            let hs = ph * 0.22;
            let hx = tx + tw * (val as f32 / 11.0) - hs / 2.0;
            rects.push(RectRun {
                x: hx,
                y: ty + th / 2.0 - hs / 2.0,
                w: hs,
                h: hs,
                rgba: GOLD,
            });
        };

    let p2_y = p1_y + p1_h + h * 0.018;
    slider(&mut rects, &mut texts, "Music volume", 6, p2_y);
    let p3_y = p2_y + h * 0.10 + h * 0.018;
    slider(&mut rects, &mut texts, "Sound FX volume", 6, p3_y);

    // Music style panel.
    let p4_y = p3_y + h * 0.10 + h * 0.018;
    let p4_h = h * 0.36;
    panel(&mut rects, p4_y, p4_h);
    let (q, _) = fonts.body.layout(
        "Music style",
        margin + col_w * 0.05,
        p4_y + h * 0.018,
        col_w,
    );
    texts.push(TextRun {
        atlas: &fonts.body,
        quads: q,
        rgba: BODY,
    });
    let auto_w = fonts.body.text_width("Auto");
    let (q, _) = fonts.body.layout(
        "Auto",
        margin + col_w - auto_w - p4_h * 0.06,
        p4_y + h * 0.018,
        auto_w + 4.0,
    );
    texts.push(TextRun {
        atlas: &fonts.body,
        quads: q,
        rgba: GOLD,
    });
    // 12-button grid (2 cols × 6 rows).
    let grid_top = p4_y + h * 0.06;
    let grid_pad = col_w * 0.04;
    let cell_w = (col_w - grid_pad * 3.0) / 2.0;
    let cell_h = (p4_h - h * 0.07) / 6.0 - h * 0.006;
    for (i, label) in MUSIC_STYLE_LABELS.iter().enumerate() {
        let (r, c) = (i / 2, i % 2);
        let cx = margin + grid_pad + c as f32 * (cell_w + grid_pad);
        let cy = grid_top + r as f32 * (cell_h + h * 0.006);
        rects.push(RectRun {
            x: cx,
            y: cy,
            w: cell_w,
            h: cell_h,
            rgba: [DIM[0], DIM[1], DIM[2], 0.4],
        });
        rects.push(RectRun {
            x: cx + 1.5,
            y: cy + 1.5,
            w: cell_w - 3.0,
            h: cell_h - 3.0,
            rgba: KEYBG,
        });
        texts.push(TextRun {
            atlas: &fonts.tiny,
            quads: centered(&fonts.tiny, label, cx + cell_w / 2.0, cy, cell_h),
            rgba: BODY,
        });
    }

    // Test sound card.
    let p5_y = p4_y + p4_h + h * 0.018;
    let p5_h = h * 0.07;
    panel(&mut rects, p5_y, p5_h);
    let (q, _) = fonts.body.layout(
        "Test sound",
        margin + col_w * 0.05,
        p5_y + p5_h * 0.28,
        col_w,
    );
    texts.push(TextRun {
        atlas: &fonts.body,
        quads: q,
        rgba: BODY,
    });
    let play_w = fonts.body.text_width("Play");
    let (q, _) = fonts.body.layout(
        "Play",
        margin + col_w - play_w - p5_h * 0.5,
        p5_y + p5_h * 0.28,
        play_w + 4.0,
    );
    texts.push(TextRun {
        atlas: &fonts.body,
        quads: q,
        rgba: GOLD,
    });

    push_neutral_button(&mut rects, &mut texts, fonts, "Back", w, h);
    (rects, texts)
}

/// The **Settings** screen — web's setup page: 3 nav cards (Audio · Developer · Fullscreen), each
/// with a label + an action chip on the right, then a red-bordered "DANGER ZONE" with the wipe-data
/// blurb + a coral "Clear all data" button. Pure UI, no state.
/// The **Best Times** screen (`renderSummary` / `#/best-times`) — a per-topic leaderboard of the
/// player's lowest finished-round `total_time_secs`. Topics without a recorded best are dimmed at
/// the bottom (locked or never finished flawlessly). **D3**: backed by `save.best_times`. Built to
/// the visual-parity bar against `summary-web.png`.
pub fn summary_frame<'a>(
    save: &Save,
    fonts: &'a Fonts,
    w: f32,
    h: f32,
) -> (Vec<RectRun>, Vec<TextRun<'a>>) {
    let mut rects: Vec<RectRun> = Vec::new();
    let mut texts: Vec<TextRun> = Vec::new();
    let margin = w * 0.05;
    let col_w = w - margin * 2.0;

    // V40: "BEST TIMES" centred eyebrow (letter-spaced, DIM — web uses muted grey, not gold).
    texts.push(TextRun {
        atlas: &fonts.tiny,
        quads: centered(&fonts.tiny, "BEST TIMES", w / 2.0, h * 0.025, h * 0.034),
        rgba: DIM,
    });
    // Subtitle (centred) — web's "Your best in each topic — tap one to play it." rendered in tiny
    // font so the full line fits the 430px ref width without truncating.
    let sub = "Your best in each topic — tap one to play it.";
    texts.push(TextRun {
        atlas: &fonts.tiny,
        quads: centered(&fonts.tiny, sub, w / 2.0, h * 0.06, h * 0.026),
        rgba: BODY,
    });

    // Cards (one per topic), in `modes.json` order. Each card: topic name (bold) + per-row status
    // line ("□ Not played" / a "□ best Xs · score N" line) + TIME column + SCORE column + Play ▶
    // button. Tap routes to the topic (live UI only — golden/ref shows the visual state).
    let modes = crate::progression::modes();
    let band_top = h * 0.115;
    let card_h = h * 0.075;
    let cgap = h * 0.008;
    let band_bot = h * 0.92;
    for (i, m) in modes.iter().enumerate() {
        let ry = band_top + i as f32 * (card_h + cgap);
        if ry + card_h > band_bot {
            break;
        }
        let bt = save.best_time(&m.id);
        // Outlined card (web's bordered panels — DIM border + KEYBG fill).
        rects.push(RectRun {
            x: margin,
            y: ry,
            w: col_w,
            h: card_h,
            rgba: [DIM[0], DIM[1], DIM[2], 0.5],
        });
        rects.push(RectRun {
            x: margin + 1.5,
            y: ry + 1.5,
            w: col_w - 3.0,
            h: card_h - 3.0,
            rgba: KEYBG,
        });
        // Topic name (top-left, body weight).
        let (q, _) = fonts.body.layout(
            &m.name,
            margin + col_w * 0.04,
            ry + card_h * 0.16,
            col_w * 0.55,
        );
        texts.push(TextRun {
            atlas: &fonts.body,
            quads: q,
            rgba: BODY,
        });
        // Status line "□ Not played" / "□ best Xs" with a small unchecked-square glyph.
        let sq = card_h * 0.16;
        let sq_x = margin + col_w * 0.04;
        let sq_y = ry + card_h * 0.62;
        rects.push(RectRun {
            x: sq_x,
            y: sq_y,
            w: sq,
            h: sq,
            rgba: [DIM[0], DIM[1], DIM[2], 0.6],
        });
        rects.push(RectRun {
            x: sq_x + 1.0,
            y: sq_y + 1.0,
            w: sq - 2.0,
            h: sq - 2.0,
            rgba: KEYBG,
        });
        let status = match bt {
            Some(t) => format!("best {t:.1}s"),
            None => "Not played".to_string(),
        };
        let (q, _) = fonts
            .tiny
            .layout(&status, sq_x + sq * 1.4, ry + card_h * 0.62, col_w * 0.5);
        texts.push(TextRun {
            atlas: &fonts.tiny,
            quads: q,
            rgba: DIM,
        });
        // TIME column (mid-right) — "—" if unrecorded, "Xs" otherwise.
        let time_str = match bt {
            Some(t) => format!("{t:.1}s"),
            None => "—".to_string(),
        };
        let tw = fonts.body.text_width(&time_str);
        let time_cx = margin + col_w * 0.62;
        let (q, _) = fonts
            .body
            .layout(&time_str, time_cx - tw / 2.0, ry + card_h * 0.32, tw + 4.0);
        texts.push(TextRun {
            atlas: &fonts.body,
            quads: q,
            // V40: neutral BODY for an existing record (web doesn't gold-accent the time column).
            rgba: if bt.is_some() { BODY } else { DIM },
        });
        // SCORE column (further right) — "—" if unrecorded, score otherwise. (Score isn't tracked
        // alongside best-time in the save yet — V32 future: extend `best_times` to carry score too.)
        let score_str = "—".to_string();
        let sw = fonts.body.text_width(&score_str);
        let score_cx = margin + col_w * 0.78;
        let (q, _) = fonts.body.layout(
            &score_str,
            score_cx - sw / 2.0,
            ry + card_h * 0.32,
            sw + 4.0,
        );
        texts.push(TextRun {
            atlas: &fonts.body,
            quads: q,
            rgba: DIM,
        });
        // Play ▶ button (far right) — a tiny right-pointing triangle. V40: muted DIM (web's
        // affordance is a quiet grey, not gold).
        let bx = margin + col_w - card_h * 0.6;
        let by = ry + card_h * 0.36;
        let bh = card_h * 0.3;
        push_play_triangle(&mut rects, bx, by, bh, DIM);
    }

    push_neutral_button(&mut rects, &mut texts, fonts, "Back", w, h);
    (rects, texts)
}

/// A small right-pointing ▶ triangle (the play affordance), top-left at `(x, y)`, `size` tall.
/// Drawn as a stack of horizontal RectRun bars (RectRun has no triangles).
fn push_play_triangle(rects: &mut Vec<RectRun>, x: f32, y: f32, size: f32, rgba: [f32; 4]) {
    let steps = 7;
    let half_full = size / 2.0;
    for k in 0..steps {
        let t = k as f32 / (steps - 1) as f32;
        let dist = (t - 0.5).abs() * 2.0; // 0 centre → 1 edge
        let bar_w = (1.0 - dist) * size * 0.7;
        rects.push(RectRun {
            x,
            y: y + k as f32 * size / steps as f32,
            w: bar_w,
            h: size / steps as f32 + 0.6,
            rgba,
        });
        let _ = half_full;
    }
}

pub fn settings_frame<'a>(fonts: &'a Fonts, w: f32, h: f32) -> (Vec<RectRun>, Vec<TextRun<'a>>) {
    let mut rects: Vec<RectRun> = Vec::new();
    let mut texts: Vec<TextRun> = Vec::new();
    let margin = w * 0.05;
    let col_w = w - margin * 2.0;

    // "SETTINGS" eyebrow.
    texts.push(TextRun {
        atlas: &fonts.tiny,
        quads: centered(&fonts.tiny, "SETTINGS", w / 2.0, h * 0.018, h * 0.03),
        rgba: DIM,
    });

    // 3 nav cards.
    let cards: [(&str, &str); 3] = [
        ("Audio", "Open"),
        ("Developer", "Open"),
        ("Fullscreen", "Enter"),
    ];
    let card_h = h * 0.07;
    let card_gap = h * 0.018;
    let card_top = h * 0.06;
    for (i, (label, action)) in cards.iter().enumerate() {
        let cy = card_top + i as f32 * (card_h + card_gap);
        // Outlined border + dark fill (web's bordered panels).
        rects.push(RectRun {
            x: margin,
            y: cy,
            w: col_w,
            h: card_h,
            rgba: [DIM[0], DIM[1], DIM[2], 0.5],
        });
        rects.push(RectRun {
            x: margin + 1.5,
            y: cy + 1.5,
            w: col_w - 3.0,
            h: card_h - 3.0,
            rgba: KEYBG,
        });
        let (q, _) = fonts.body.layout(
            label,
            margin + col_w * 0.05,
            cy + card_h * 0.28,
            col_w * 0.6,
        );
        texts.push(TextRun {
            atlas: &fonts.body,
            quads: q,
            rgba: BODY,
        });
        // Action chip on the right (gold).
        let aw = fonts.body.text_width(action);
        let (q, _) = fonts.body.layout(
            action,
            margin + col_w - aw - card_h * 0.5,
            cy + card_h * 0.28,
            aw + 4.0,
        );
        texts.push(TextRun {
            atlas: &fonts.body,
            quads: q,
            rgba: GOLD,
        });
    }

    // DANGER ZONE — red-bordered panel + blurb + coral wipe button.
    const RED: [f32; 4] = [240.0 / 255.0, 80.0 / 255.0, 80.0 / 255.0, 1.0];
    const CORAL: [f32; 4] = [1.0, 120.0 / 255.0, 120.0 / 255.0, 1.0];
    let dz_top = card_top + 3.0 * (card_h + card_gap) + h * 0.01;
    let dz_h = h * 0.30;
    rects.push(RectRun {
        x: margin,
        y: dz_top,
        w: col_w,
        h: dz_h,
        rgba: [RED[0], RED[1], RED[2], 0.7],
    });
    rects.push(RectRun {
        x: margin + 1.5,
        y: dz_top + 1.5,
        w: col_w - 3.0,
        h: dz_h - 3.0,
        rgba: [RED[0] * 0.1, RED[1] * 0.05, RED[2] * 0.05, 1.0],
    });
    let (q, _) = fonts.tiny.layout(
        "DANGER ZONE",
        margin + col_w * 0.05,
        dz_top + h * 0.012,
        col_w,
    );
    texts.push(TextRun {
        atlas: &fonts.tiny,
        quads: q,
        rgba: RED,
    });
    let (q, _) = fonts.tiny.layout(
        "Clear all data wipes every bit of progress saved on this device — heroes, items, scores, events, settings — and returns the app to a fresh start. This cannot be undone.",
        margin + col_w * 0.05,
        dz_top + h * 0.04,
        col_w * 0.88,
    );
    texts.push(TextRun {
        atlas: &fonts.tiny,
        quads: q,
        rgba: BODY,
    });
    // "Clear all data" coral button — sits BELOW the blurb (the blurb wraps to ~5 lines at body
    // size in `col_w*0.9`, occupying the top 65% of the panel; the button takes the bottom strip).
    let btn_w = col_w * 0.6;
    let btn_h = h * 0.058;
    let btn_x = margin + col_w * 0.05;
    let btn_y = dz_top + dz_h - btn_h - h * 0.025;
    rects.push(RectRun {
        x: btn_x,
        y: btn_y,
        w: btn_w,
        h: btn_h,
        rgba: CORAL,
    });
    texts.push(TextRun {
        atlas: &fonts.body,
        quads: centered(
            &fonts.body,
            "Clear all data",
            btn_x + btn_w / 2.0,
            btn_y,
            btn_h,
        ),
        rgba: INK,
    });

    push_neutral_button(&mut rects, &mut texts, fonts, "Back", w, h);
    (rects, texts)
}

/// The banner shown on the Daily-Event screen after a finished gauntlet run — the event, the
/// score, and which reward tier it cleared.
pub struct EventOutcome {
    pub name: String,
    pub score: u32,
    pub total: u32,
    /// Cleared the "well played" tier (≥ 70% solved).
    pub well: bool,
    /// A flawless run (every question solved).
    pub ace: bool,
}

impl EventOutcome {
    /// The headline reward tier reached (flawless ▸ well-played ▸ completed).
    fn tier_label(&self) -> &'static str {
        if self.ace {
            "Flawless!"
        } else if self.well {
            "Well played!"
        } else {
            "Completed"
        }
    }
}

/// Format the time remaining until the next UTC day rotates the live event: `"9h 30m"` / `"45m"`.
fn event_countdown(now_ms: i64) -> String {
    let day = crate::event_play::day_ms();
    let rem = day - now_ms.rem_euclid(day);
    let (h, m) = (rem / 3_600_000, (rem % 3_600_000) / 60_000);
    if h > 0 {
        format!("{h}h {m}m")
    } else {
        format!("{m}m")
    }
}

/// The **Daily Event** (event-play) screen: today's live event — its F4 crest banner, theme/rarity,
/// blurb, the gauntlet size, the three reward tiers (earned vs locked), a countdown to the next
/// rotation, and a **Play** CTA that launches the gauntlet drill. `now_ms` (UTC epoch ms) is injected
/// so the render is deterministic — the live app passes the wall clock; the golden a fixed sample.
pub fn event_play_frame<'a>(
    save: &Save,
    now_ms: i64,
    last: Option<&EventOutcome>,
    fonts: &'a Fonts,
    w: f32,
    h: f32,
) -> (Vec<RectRun>, Vec<TextRun<'a>>) {
    let mut rects: Vec<RectRun> = Vec::new();
    let mut texts: Vec<TextRun> = Vec::new();
    let margin = w * 0.05;
    let col_w = w - margin * 2.0;
    let ev = crate::event_play::live_event(now_ms);
    let owned = |k: &str| save.collected.contains_key(k);

    // Title + countdown.
    let (q, _) = fonts.head.layout("Daily Event", margin, h * 0.028, col_w);
    texts.push(TextRun {
        atlas: &fonts.head,
        quads: q,
        rgba: GOLD,
    });
    let cd = format!("next in {}", event_countdown(now_ms));
    let cw = fonts.body.text_width(&cd);
    let (q, _) = fonts.body.layout(&cd, w - margin - cw, h * 0.043, cw + 4.0);
    texts.push(TextRun {
        atlas: &fonts.body,
        quads: q,
        rgba: DIM,
    });

    // Crest banner (F4 eventart: a 24×16 colour grid stretched across the column).
    let (by, bh) = (h * 0.085, h * 0.19);
    let crest = crate::scenes::eventart_grid(ev.art_seed);
    paint_colors(&mut rects, &crest, margin, by, col_w / 24.0, bh / 16.0);

    // Event name below the banner, then theme · rarity, then the blurb.
    let nm = centered(
        &fonts.head,
        &ev.name,
        w / 2.0,
        by + bh + h * 0.012,
        h * 0.044,
    );
    texts.push(TextRun {
        atlas: &fonts.head,
        quads: nm,
        rgba: BODY,
    });
    let tr = format!("{} · {}", ev.theme, ev.rarity);
    let tline = centered(&fonts.tiny, &tr, w / 2.0, by + bh + h * 0.055, h * 0.03);
    texts.push(TextRun {
        atlas: &fonts.tiny,
        quads: tline,
        rgba: GOLD,
    });
    let (q, _) = fonts
        .body
        .layout(&ev.blurb, margin, by + bh + h * 0.085, col_w);
    texts.push(TextRun {
        atlas: &fonts.body,
        quads: q,
        rgba: DIM,
    });

    // Gauntlet size (below the blurb, which can wrap to two lines).
    let total: usize = ev.question_mix.iter().map(|m| m.n).sum();
    let gl = format!("{total} questions across {} topics", ev.question_mix.len());
    let (q, _) = fonts.body.layout(&gl, margin, by + bh + h * 0.18, col_w);
    texts.push(TextRun {
        atlas: &fonts.body,
        quads: q,
        rgba: GOLD,
    });

    // The three reward tiers (participation / well-played / flawless), green once earned.
    let tiers = [
        ("Play", &ev.reward, format!("event:{}", ev.id)),
        ("70%+", &ev.reward_well, format!("event:{}:well", ev.id)),
        ("Flawless", &ev.reward_ace, format!("event:{}:ace", ev.id)),
    ];
    let (ty0, trow, tgap) = (h * 0.56, h * 0.058, h * 0.008);
    for (i, (tag, name, key)) in tiers.iter().enumerate() {
        let ry = ty0 + i as f32 * (trow + tgap);
        let got = owned(key);
        rects.push(RectRun {
            x: margin,
            y: ry,
            w: col_w,
            h: trow,
            rgba: if got { GREEN } else { PANEL },
        });
        let lty = ry + trow / 2.0 - 0.59 * fonts.body.px;
        let (q, _) = fonts.body.layout(
            &format!("{tag} — {name}"),
            margin + col_w * 0.04,
            lty,
            col_w,
        );
        texts.push(TextRun {
            atlas: &fonts.body,
            quads: q,
            rgba: if got { INK } else { BODY },
        });
        let status = if got { "earned" } else { "locked" };
        let sw = fonts.tiny.text_width(status);
        let (q, _) = fonts
            .tiny
            .layout(status, margin + col_w * 0.96 - sw, lty, sw + 4.0);
        texts.push(TextRun {
            atlas: &fonts.tiny,
            quads: q,
            rgba: if got { INK } else { DIM },
        });
    }

    // Outcome banner (after a finished run).
    if let Some(o) = last {
        let msg = format!("{} — {}/{} · {}", o.name, o.score, o.total, o.tier_label());
        let mw = fonts.body.text_width(&msg);
        let (q, _) = fonts
            .body
            .layout(&msg, w / 2.0 - mw / 2.0, h * 0.805, mw + 4.0);
        texts.push(TextRun {
            atlas: &fonts.body,
            quads: q,
            rgba: GREEN,
        });
    }

    // Play CTA (gold) + Back.
    let (px, py, pw, ph) = event_play_cta(w, h);
    rects.push(RectRun {
        x: px,
        y: py,
        w: pw,
        h: ph,
        rgba: GOLD,
    });
    texts.push(TextRun {
        atlas: &fonts.key,
        quads: centered(&fonts.key, "Play today's event", px + pw / 2.0, py, ph),
        rgba: INK,
    });
    push_neutral_button(&mut rects, &mut texts, fonts, "Back", w, h);
    (rects, texts)
}

/// The Daily-Event "Play" CTA rect (just above the bottom Back button) — shares geometry with the
/// Arena Fight bar so the two action screens line up.
fn event_play_cta(w: f32, h: f32) -> (f32, f32, f32, f32) {
    let bw = w * 0.6;
    let bh = h * 0.05;
    ((w - bw) / 2.0, h * 0.85, bw, bh)
}

/// Whether (`px`,`py`) hits the Daily-Event Play CTA.
fn event_play_cta_hit(w: f32, h: f32, px: f32, py: f32) -> bool {
    let (bx, by, bw, bh) = event_play_cta(w, h);
    px >= bx && px < bx + bw && py >= by && py < by + bh
}

/// The **Items** drill-down: the catalogue by category — how many of each the player owns.
/// The five Inventory tabs (web's `renderInventory`) and the catalogue categories each gathers.
const INV_TABS: [&str; 5] = ["TOPICS", "AWARDS", "EVENTS", "LOOT", "CODEX"];
fn inv_tab_cats(tab: usize) -> &'static [crate::catalogue::Category] {
    use crate::catalogue::Category::*;
    match tab {
        // Awards = every non-event collectible category (web's AWARDS overview); Topics splits the
        // per-mode ones; Events its own. Loot/Codex are special (loot ids / bestiary — a later pass).
        1 => &[
            Rank, Initiation, Flawless, Speed, Mastery, Solved, Spark, Milestone, Collector,
        ],
        0 => &[Initiation, Flawless, Mastery, Speed, Solved, Spark],
        2 => &[Events],
        _ => &[],
    }
}

/// Every loot id (combat.json `lootBoosts`, tiers 1..=`tier_count`) — the 350 `loot:*` ids the
/// Inventory header counts (loot lives on the Loot tab, but the grand total includes it).
fn all_loot_ids() -> Vec<String> {
    (1..=crate::combat::tier_count())
        .flat_map(crate::combat::loot_for)
        .collect()
}

/// The tile/border colour for an item rarity (`collectibles.js RARITY`).
fn rarity_rgba(rarity: &str) -> [f32; 4] {
    match rarity {
        "uncommon" => crate::art::hex_rgba("#3fce8c"),
        "rare" => crate::art::hex_rgba("#3f97d8"),
        "epic" => crate::art::hex_rgba("#9a5cf6"),
        "legendary" => crate::art::hex_rgba("#f0ad3c"),
        _ => crate::art::hex_rgba("#7e8a97"), // common
    }
}

/// One Inventory progress-bar row: a panel with `label` (green when complete) + `have/total` (+ ✓)
/// and a green fill bar. Shared by the Topics list (and a candidate for the category tabs).
#[allow(clippy::too_many_arguments)]
fn push_progress_row<'a>(
    rects: &mut Vec<RectRun>,
    texts: &mut Vec<TextRun<'a>>,
    fonts: &'a Fonts,
    label: &str,
    have: u32,
    total: u32,
    margin: f32,
    col_w: f32,
    ry: f32,
    row_h: f32,
) {
    // V45: row as a BORDERED CARD (DIM outline + KEYBG fill) instead of a flat PANEL strip,
    // matching the per-row card framing on every other web inventory row.
    rects.push(RectRun {
        x: margin,
        y: ry,
        w: col_w,
        h: row_h,
        rgba: [DIM[0], DIM[1], DIM[2], 0.45],
    });
    rects.push(RectRun {
        x: margin + 1.0,
        y: ry + 1.0,
        w: col_w - 2.0,
        h: row_h - 2.0,
        rgba: PANEL,
    });
    let done = total > 0 && have >= total;
    let lc = if done { GREEN } else { BODY };
    // Keep the label on ONE line: the body face when it fits, else the smaller tiny face (the long
    // "<Realm> · tiers a–b" loot labels would otherwise wrap over the progress bar).
    let avail = col_w * 0.66;
    let (la, ly) = if fonts.body.text_width(label) <= avail {
        (&fonts.body, row_h * 0.08)
    } else {
        (&fonts.tiny, row_h * 0.16)
    };
    let (q, _) = la.layout(label, margin + col_w * 0.05, ry + ly, col_w);
    texts.push(TextRun {
        atlas: la,
        quads: q,
        rgba: lc,
    });
    // A prominent green ✓ at the far right when complete (web's per-row checkmark, V22), with the
    // count laid out to its left.
    let mut count_right = margin + col_w * 0.96;
    if done {
        let ck = row_h * 0.34;
        push_check(
            rects,
            margin + col_w * 0.96 - ck * 1.2,
            ry + row_h * 0.3,
            ck,
            GREEN,
        );
        count_right = margin + col_w * 0.92 - ck * 1.2;
    }
    let cnt = format!("{have} / {total}");
    let cw = fonts.tiny.text_width(&cnt);
    let (q, _) = fonts
        .tiny
        .layout(&cnt, count_right - cw, ry + row_h * 0.16, cw + 8.0);
    texts.push(TextRun {
        atlas: &fonts.tiny,
        quads: q,
        rgba: lc,
    });
    // V45 v2: bar on its OWN LINE below the label, with vertical breathing room — the row is
    // visually a "label row + bar row" stack, not a strip with text striking through it.
    let (bx, by, bw, bh) = (
        margin + col_w * 0.05,
        ry + row_h * 0.72,
        col_w * 0.90,
        row_h * 0.16,
    );
    rects.push(RectRun {
        x: bx,
        y: by,
        w: bw,
        h: bh,
        rgba: INK,
    });
    let frac = if total > 0 {
        (have as f32 / total as f32).min(1.0)
    } else {
        0.0
    };
    if frac > 0.0 {
        rects.push(RectRun {
            x: bx,
            y: by,
            w: bw * frac,
            h: bh,
            rgba: GREEN,
        });
    }
}

/// A 5-column grid of collectible tiles (rarity border + N1 pixel icon + short name; owned bright,
/// unowned dim), starting at `top` and filling down to the Back button. Shared by the tabs that show
/// an item grid (Events; Codex foes use their own portrait path).
#[allow(clippy::too_many_arguments)]
fn push_item_grid<'a>(
    rects: &mut Vec<RectRun>,
    texts: &mut Vec<TextRun<'a>>,
    fonts: &'a Fonts,
    items: &[(String, String, String)],
    owned: &std::collections::HashSet<&str>,
    margin: f32,
    col_w: f32,
    top: f32,
    h: f32,
    w: f32,
) {
    let cols = 5usize;
    let sgap = w * 0.02;
    let tile = (col_w - sgap * (cols as f32 - 1.0)) / cols as f32;
    let vgap = h * 0.03;
    let bottom = h * 0.88;
    for (i, (id, name, rarity)) in items.iter().enumerate() {
        let (r, c) = (i / cols, i % cols);
        let x = margin + c as f32 * (tile + sgap);
        let y = top + r as f32 * (tile + vgap);
        if y + tile > bottom {
            break;
        }
        let have_it = owned.contains(id.as_str());
        rects.push(RectRun {
            x: x - 1.5,
            y: y - 1.5,
            w: tile + 3.0,
            h: tile + 3.0,
            rgba: if have_it { rarity_rgba(rarity) } else { DIM },
        });
        rects.push(RectRun {
            x,
            y,
            w: tile,
            h: tile,
            rgba: INK,
        });
        if let Some((role, pal)) = crate::art::item_icon_for(id, rarity) {
            paint_role(
                rects,
                &role,
                &pal,
                x + tile * 0.08,
                y + tile * 0.08,
                tile * 0.84 / 16.0,
            );
        }
        // V46: wrap the full name into ≤2 lines under the tile, not truncate at 9 chars.
        texts.push(TextRun {
            atlas: &fonts.tiny,
            quads: centered_wrapped(
                &fonts.tiny,
                name,
                x + tile / 2.0,
                y + tile + h * 0.002,
                h * 0.016,
                tile,
                2,
            ),
            rgba: if have_it { BODY } else { DIM },
        });
    }
}

/// The Codex Beasts grid: a 5-col tile of region×type foe portraits via `art::foe_grid`. Encountered
/// cells paint the foe at full palette; locked cells paint a dim silhouette (web's "?" + filter).
#[allow(clippy::too_many_arguments)]
fn push_codex_beast_grid<'a>(
    rects: &mut Vec<RectRun>,
    texts: &mut Vec<TextRun<'a>>,
    fonts: &'a Fonts,
    beasts: &[(u32, crate::arena::Kind, u32, bool)],
    margin: f32,
    col_w: f32,
    top: f32,
    h: f32,
    w: f32,
) {
    let cols = 5usize;
    let sgap = w * 0.02;
    let tile = (col_w - sgap * (cols as f32 - 1.0)) / cols as f32;
    let vgap = h * 0.03;
    let bottom = h * 0.88;
    for (i, (region, kind, rep, enc)) in beasts.iter().enumerate() {
        let (r, c) = (i / cols, i % cols);
        let x = margin + c as f32 * (tile + sgap);
        let y = top + r as f32 * (tile + vgap);
        if y + tile > bottom {
            break;
        }
        // Tile background + a thin border (gold when encountered, dim otherwise).
        rects.push(RectRun {
            x: x - 1.5,
            y: y - 1.5,
            w: tile + 3.0,
            h: tile + 3.0,
            rgba: if *enc { GOLD } else { DIM },
        });
        rects.push(RectRun {
            x,
            y,
            w: tile,
            h: tile,
            rgba: INK,
        });
        // Foe portrait — `foe_grid` at the rep tier. The (name, kind) only affect the name; the grid
        // is seeded by tier-n + name, so passing the rep's combat.json name keeps the silhouette.
        let name = crate::combat::tier_meta(*rep)
            .map(|(n, _, _)| n)
            .unwrap_or_default();
        let (role, pal) = crate::art::foe_grid(*rep, &name, *kind);
        // For locked cells, render with a "silhouette" palette (everything outline-coloured) so the
        // shape reads but the type colour is hidden — matches web's CSS dark-silhouette filter.
        let cell = tile * 0.84 / 16.0;
        if *enc {
            paint_role(rects, &role, &pal, x + tile * 0.08, y + tile * 0.08, cell);
        } else {
            let dim = crate::art::Palette::mono("#3a3a4a");
            paint_role(rects, &role, &dim, x + tile * 0.08, y + tile * 0.08, cell);
        }
        // Caption "<Realm> · <Type>" — V46: WRAP into ≤2 lines (was truncated at 11 chars,
        // losing the " · Brawn/Cunning/Arcane" suffix on long region names).
        let cap = format!(
            "{} · {kind:?}",
            crate::combat::region_name(*region as usize)
        );
        texts.push(TextRun {
            atlas: &fonts.tiny,
            quads: centered_wrapped(
                &fonts.tiny,
                &cap,
                x + tile / 2.0,
                y + tile + h * 0.002,
                h * 0.014,
                tile,
                2,
            ),
            rgba: if *enc { BODY } else { DIM },
        });
    }
}

/// The **Inventory** (Items) screen: a tab row (Topics/Awards/Events/Loot/Codex) over a grid of the
/// selected tab's items, each a **pixel icon** (the N1 `item_icon` generator) on a rarity-coloured
/// tile + its name. Owned items are bright; unowned are dimmed. Built to the visual bar against
/// `inventory-*-web.png`. `tab` selects the active tab.
pub fn items_frame<'a>(
    save: &Save,
    tab: usize,
    now_ms: i64,
    fonts: &'a Fonts,
    w: f32,
    h: f32,
) -> (Vec<RectRun>, Vec<TextRun<'a>>) {
    let mut rects: Vec<RectRun> = Vec::new();
    let mut texts: Vec<TextRun> = Vec::new();
    let margin = w * 0.05;
    let col_w = w - margin * 2.0;
    let owned: std::collections::HashSet<&str> =
        save.collected.keys().map(String::as_str).collect();

    // Header: "Inventory  owned / total" over the GRAND total (catalogue + the 350 loot ids).
    let loot_ids = all_loot_ids();
    let cat_owned = crate::catalogue::catalog()
        .iter()
        .filter(|c| owned.contains(c.id.as_str()))
        .count();
    let loot_owned = loot_ids
        .iter()
        .filter(|id| owned.contains(id.as_str()))
        .count();
    let grand_total = crate::catalogue::total() as usize + loot_ids.len();
    // V42: ALL-CAPS title is wider than its title-case predecessor — use `body` atlas (vs head)
    // so "INVENTORY  2702 / 2702" doesn't clip at 430 px.
    let title = format!("INVENTORY  {} / {}", cat_owned + loot_owned, grand_total);
    texts.push(TextRun {
        atlas: &fonts.body,
        quads: centered(&fonts.body, &title, w / 2.0, h * 0.028, h * 0.034),
        rgba: GOLD,
    });

    // Tab row.
    let tgap = w * 0.015;
    let tw = (col_w - tgap * 4.0) / 5.0;
    let (ty, th) = (h * 0.085, h * 0.04);
    for (i, name) in INV_TABS.iter().enumerate() {
        let tx = margin + i as f32 * (tw + tgap);
        let sel = i == tab;
        if sel {
            rects.push(RectRun {
                x: tx - 1.5,
                y: ty - 1.5,
                w: tw + 3.0,
                h: th + 3.0,
                rgba: GOLD,
            });
        }
        rects.push(RectRun {
            x: tx,
            y: ty,
            w: tw,
            h: th,
            rgba: PANEL,
        });
        texts.push(TextRun {
            atlas: &fonts.tiny,
            quads: centered(&fonts.tiny, name, tx + tw / 2.0, ty, th),
            rgba: if sel { GOLD } else { DIM },
        });
    }

    // TOPICS tab — one progress-bar row per topic (web's per-mode `modeProgress` list: name +
    // have/total + ✓ + green bar, no detail strip). Distinct row source from the category tabs.
    if tab == 0 {
        use std::collections::BTreeMap;
        let mut mhave: BTreeMap<String, u32> = BTreeMap::new();
        let mut mtot: BTreeMap<String, u32> = BTreeMap::new();
        for c in crate::catalogue::catalog() {
            if let Some(mid) = &c.mode_id {
                *mtot.entry(mid.clone()).or_default() += 1;
                if owned.contains(c.id.as_str()) {
                    *mhave.entry(mid.clone()).or_default() += 1;
                }
            }
        }
        let modes = crate::progression::modes();
        let prog = |m: &progression::Mode| -> (u32, u32) {
            (
                *mhave.get(&m.id).unwrap_or(&0),
                *mtot.get(&m.id).unwrap_or(&0),
            )
        };
        let complete = modes
            .iter()
            .filter(|m| {
                let (hv, t) = prog(m);
                t > 0 && hv >= t
            })
            .count();
        let sum_have: u32 = modes.iter().map(|m| prog(m).0).sum();
        let sum_tot: u32 = modes.iter().map(|m| prog(m).1).sum();
        let pct = if sum_tot > 0 {
            (sum_have as f32 / sum_tot as f32 * 100.0).round() as u32
        } else {
            0
        };
        // Section label + "complete/total AT pct%".
        let (q, _) = fonts.body.layout("TOPICS", margin, h * 0.142, col_w);
        texts.push(TextRun {
            atlas: &fonts.body,
            quads: q,
            rgba: GOLD,
        });
        let sc = format!("{complete}/{} AT {pct}%", modes.len());
        let scw = fonts.tiny.text_width(&sc);
        let (q, _) = fonts
            .tiny
            .layout(&sc, w - margin - scw, h * 0.15, scw + 4.0);
        texts.push(TextRun {
            atlas: &fonts.tiny,
            quads: q,
            rgba: DIM,
        });
        // Rows (as many as fit above the Back button; the rest scroll on device).
        let band_top = h * 0.175;
        let band_bot = h * 0.88;
        let rgap = h * 0.012;
        let row_h = h * 0.055;
        for (i, m) in modes.iter().enumerate() {
            let ry = band_top + i as f32 * (row_h + rgap);
            if ry + row_h > band_bot {
                break;
            }
            let (hv, t) = prog(m);
            push_progress_row(
                &mut rects, &mut texts, fonts, &m.name, hv, t, margin, col_w, ry, row_h,
            );
        }
        push_neutral_button(&mut rects, &mut texts, fonts, "Back", w, h);
        return (rects, texts);
    }

    // LOOT tab — one progress-bar row per REGION ("<Realm> · tiers a–b" + loot-collected/total + ✓ +
    // bar), web's per-region loot list. Loot ids are per-tier; a region spans `regionSize` tiers.
    if tab == 3 {
        let rsize = crate::combat::region_size();
        let nregions = (crate::combat::tier_count() / rsize) as usize;
        // Per-region (have, total) over the region's loot ids.
        let region_counts: Vec<(u32, u32)> = (0..nregions)
            .map(|r| {
                let (mut hv, mut t) = (0u32, 0u32);
                for tier in (r as u32 * rsize + 1)..=((r as u32 + 1) * rsize) {
                    for id in crate::combat::loot_for(tier) {
                        t += 1;
                        if owned.contains(id.as_str()) {
                            hv += 1;
                        }
                    }
                }
                (hv, t)
            })
            .collect();
        let loot_total: u32 = region_counts.iter().map(|(_, t)| t).sum();
        let loot_have: u32 = region_counts.iter().map(|(hv, _)| hv).sum();
        // Section label "LOOT" + right "have/total".
        let (q, _) = fonts.body.layout("LOOT", margin, h * 0.142, col_w);
        texts.push(TextRun {
            atlas: &fonts.body,
            quads: q,
            rgba: GOLD,
        });
        let sc = format!("{loot_have} / {loot_total}");
        let scw = fonts.tiny.text_width(&sc);
        let (q, _) = fonts
            .tiny
            .layout(&sc, w - margin - scw, h * 0.15, scw + 4.0);
        texts.push(TextRun {
            atlas: &fonts.tiny,
            quads: q,
            rgba: DIM,
        });
        // Rows, one per region — slightly shorter than before so a per-region carousel can
        // fit at the bottom (V49 adds the loot-item strip web shows under the rows).
        let band_top = h * 0.175;
        let rgap = h * 0.010;
        let row_h = h * 0.048;
        for (r, &(hv, t)) in region_counts.iter().enumerate() {
            let ry = band_top + r as f32 * (row_h + rgap);
            let lo = r as u32 * rsize + 1;
            let hi = (r as u32 + 1) * rsize;
            let label = format!("{} · tiers {lo}–{hi}", crate::combat::region_name(r));
            push_progress_row(
                &mut rects, &mut texts, fonts, &label, hv, t, margin, col_w, ry, row_h,
            );
        }

        // V49: bottom carousel — `<REGION> · TIERS lo-hi  N/N` sub-header + a horizontal strip
        // of the first region's loot tiles (web shows the active region's loot in this slot).
        let cy = band_top + nregions as f32 * (row_h + rgap) + h * 0.012;
        let (q, _) = fonts
            .tiny
            .layout("GOBLIN WARREN · TIERS 1-12", margin, cy, col_w);
        texts.push(TextRun {
            atlas: &fonts.tiny,
            quads: q,
            rgba: DIM,
        });
        // Right-aligned have/total for region 0.
        let r0_count = format!("{} / {}", region_counts[0].0, region_counts[0].1);
        let rc_w = fonts.tiny.text_width(&r0_count);
        let (q, _) = fonts
            .tiny
            .layout(&r0_count, w - margin - rc_w, cy, rc_w + 4.0);
        texts.push(TextRun {
            atlas: &fonts.tiny,
            quads: q,
            rgba: DIM,
        });
        // Strip of 5 tiles for region-0 loot ids (rarity border + icon + short name).
        let cy2 = cy + h * 0.024;
        let region0_ids: Vec<String> = (1..=rsize).flat_map(crate::combat::loot_for).collect();
        let cols = 5usize;
        let sgap = w * 0.018;
        let tile = (col_w - sgap * (cols as f32 - 1.0)) / cols as f32;
        for (i, id) in region0_ids.iter().take(cols).enumerate() {
            let x = margin + i as f32 * (tile + sgap);
            let have_it = owned.contains(id.as_str());
            // Loot ids ("loot:N:idx") aren't in the collectibles catalogue — render with a
            // synthesised "Tier N" label rather than dumping the raw slug.
            let rarity = "common".to_string();
            let name = id
                .strip_prefix("loot:")
                .and_then(|s| s.split(':').next())
                .map(|t| format!("Tier {t}"))
                .unwrap_or_else(|| id.clone());
            rects.push(RectRun {
                x: x - 1.5,
                y: cy2 - 1.5,
                w: tile + 3.0,
                h: tile + 3.0,
                rgba: if have_it { rarity_rgba(&rarity) } else { DIM },
            });
            rects.push(RectRun {
                x,
                y: cy2,
                w: tile,
                h: tile,
                rgba: INK,
            });
            if let Some((role, pal)) = crate::art::item_icon_for(id, &rarity) {
                paint_role(
                    &mut rects,
                    &role,
                    &pal,
                    x + tile * 0.08,
                    cy2 + tile * 0.08,
                    tile * 0.84 / 16.0,
                );
            }
            texts.push(TextRun {
                atlas: &fonts.tiny,
                quads: centered_wrapped(
                    &fonts.tiny,
                    &name,
                    x + tile / 2.0,
                    cy2 + tile + h * 0.002,
                    h * 0.014,
                    tile,
                    2,
                ),
                rgba: if have_it { BODY } else { DIM },
            });
        }

        push_neutral_button(&mut rects, &mut texts, fonts, "Back", w, h);
        return (rects, texts);
    }

    // EVENTS tab — a live-event banner + a "Daily Events" progress row + a grid of the 42 Events
    // collectibles (the daily-event reward items), web's event collection view.
    if tab == 2 {
        let ev = crate::event_play::live_event(now_ms);
        let earned = owned.contains(format!("event:{}", ev.id).as_str());
        // V15: pass the procedural FLAVOUR name (web's `it.flavour || it.name`) so the grid
        // shows "Emberbond Ring" / "Phoenix Bond" etc., not the bare category title.
        let ev_items: Vec<(String, String, String)> = crate::catalogue::catalog()
            .into_iter()
            .filter(|c| matches!(c.cat, crate::catalogue::Category::Events))
            .map(|c| {
                let display = if c.flavour.is_empty() {
                    c.name
                } else {
                    c.flavour
                };
                (c.id, display, c.rarity)
            })
            .collect();
        let ev_total = ev_items.len() as u32;
        let ev_have = ev_items
            .iter()
            .filter(|(id, _, _)| owned.contains(id.as_str()))
            .count() as u32;

        // Live-event banner.
        let (by, bh) = (h * 0.115, h * 0.14);
        rects.push(RectRun {
            x: margin,
            y: by,
            w: col_w,
            h: bh,
            rgba: INK,
        });
        let eyebrow = if earned {
            "LIVE TODAY · REWARD EARNED"
        } else {
            "LIVE TODAY"
        };
        let (q, _) = fonts
            .tiny
            .layout(eyebrow, margin + col_w * 0.04, by + bh * 0.1, col_w);
        texts.push(TextRun {
            atlas: &fonts.tiny,
            quads: q,
            rgba: GOLD,
        });
        let (q, _) =
            fonts
                .body
                .layout(&ev.name, margin + col_w * 0.04, by + bh * 0.28, col_w * 0.6);
        texts.push(TextRun {
            atlas: &fonts.body,
            quads: q,
            rgba: BODY,
        });
        let (q, _) = fonts.tiny.layout(
            &ev.blurb,
            margin + col_w * 0.04,
            by + bh * 0.56,
            col_w * 0.56,
        );
        texts.push(TextRun {
            atlas: &fonts.tiny,
            quads: q,
            rgba: DIM,
        });
        // "Play again" / "Play" CTA (solid gold).
        let plabel = if earned { "Play again" } else { "Play" };
        let (pw, px) = (col_w * 0.26, margin + col_w - col_w * 0.28);
        rects.push(RectRun {
            x: px,
            y: by + bh * 0.32,
            w: pw,
            h: bh * 0.36,
            rgba: GOLD,
        });
        texts.push(TextRun {
            atlas: &fonts.tiny,
            quads: centered(
                &fonts.tiny,
                plabel,
                px + pw / 2.0,
                by + bh * 0.32,
                bh * 0.36,
            ),
            rgba: INK,
        });

        // "EVENTS  have/total" section + a single "Daily Events" progress row.
        let (q, _) = fonts.body.layout("EVENTS", margin, h * 0.285, col_w);
        texts.push(TextRun {
            atlas: &fonts.body,
            quads: q,
            rgba: GOLD,
        });
        let sc = format!("{ev_have} / {ev_total}");
        let scw = fonts.tiny.text_width(&sc);
        let (q, _) = fonts
            .tiny
            .layout(&sc, w - margin - scw, h * 0.292, scw + 4.0);
        texts.push(TextRun {
            atlas: &fonts.tiny,
            quads: q,
            rgba: DIM,
        });
        push_progress_row(
            &mut rects,
            &mut texts,
            fonts,
            "Daily Events",
            ev_have,
            ev_total,
            margin,
            col_w,
            h * 0.315,
            h * 0.05,
        );

        // "DAILY EVENTS  have/total" + the reward-item icon grid.
        let (q, _) = fonts.tiny.layout("DAILY EVENTS", margin, h * 0.39, col_w);
        texts.push(TextRun {
            atlas: &fonts.tiny,
            quads: q,
            rgba: DIM,
        });
        push_item_grid(
            &mut rects,
            &mut texts,
            fonts,
            &ev_items,
            &owned,
            margin,
            col_w,
            h * 0.415,
            h,
            w,
        );

        push_neutral_button(&mut rects, &mut texts, fonts, "Back", w, h);
        return (rects, texts);
    }

    // CODEX tab — a bestiary: 4 category progress rows (Beasts/Bosses/Realms/Events) + the Beasts
    // grid (region×type foe portraits via `art::foe_grid`). Discovery = `tier:n` keys ≥ `rep`
    // (`main.js invCodexHtml`); a "current" save has the whole ladder cleared, so every cell counts.
    if tab == 4 {
        use crate::arena::Kind::*;
        let rsize = crate::combat::region_size();
        let nregions = (crate::combat::tier_count() / rsize) as usize;
        let reached = crate::combat::reached_tier(save.collected.keys().map(String::as_str));
        let event_roster = crate::event_play::roster();

        // Enumerate the 4 sections (web order). Beasts: region × {Brawn,Cunning,Arcane}; Bosses:
        // every `RS`th tier; Realms: 1 per region (encountered once region's first tier is reached);
        // Events: each event with any `event:<id>*` key owned.
        type Beast = (u32, crate::arena::Kind, u32, bool); // (region, kind, rep_tier, encountered)
        let mut beasts: Vec<Beast> = Vec::new();
        for r in 0..nregions as u32 {
            for k in [Brawn, Cunning, Arcane] {
                if let Some(rep) = crate::combat::beast_rep_tier(r, k) {
                    beasts.push((r, k, rep, reached >= rep));
                }
            }
        }
        let bosses: Vec<(u32, u32, bool)> = (0..nregions as u32)
            .map(|r| {
                let n = (r + 1) * rsize;
                (r, n, reached >= n)
            })
            .collect();
        let realms_enc: Vec<bool> = (0..nregions as u32).map(|r| reached > r * rsize).collect();
        let events_enc: Vec<bool> = event_roster
            .iter()
            .map(|ev| {
                let pref = format!("event:{}", ev.id);
                save.collected.keys().any(|k| k.starts_with(&pref))
            })
            .collect();

        let cats: [(&str, u32, u32); 4] = [
            (
                "Beasts",
                beasts.iter().filter(|b| b.3).count() as u32,
                beasts.len() as u32,
            ),
            (
                "Bosses",
                bosses.iter().filter(|b| b.2).count() as u32,
                bosses.len() as u32,
            ),
            (
                "Realms",
                realms_enc.iter().filter(|&&e| e).count() as u32,
                realms_enc.len() as u32,
            ),
            (
                "Events",
                events_enc.iter().filter(|&&e| e).count() as u32,
                events_enc.len() as u32,
            ),
        ];
        let total: u32 = cats.iter().map(|(_, _, t)| t).sum();
        let have: u32 = cats.iter().map(|(_, h, _)| h).sum();

        // Section header.
        let (q, _) = fonts.body.layout("CODEX", margin, h * 0.142, col_w);
        texts.push(TextRun {
            atlas: &fonts.body,
            quads: q,
            rgba: GOLD,
        });
        let sc = format!("{have} / {total} DISCOVERED");
        let scw = fonts.tiny.text_width(&sc);
        let (q, _) = fonts
            .tiny
            .layout(&sc, w - margin - scw, h * 0.15, scw + 4.0);
        texts.push(TextRun {
            atlas: &fonts.tiny,
            quads: q,
            rgba: DIM,
        });

        // 4 progress rows.
        let band_top = h * 0.175;
        let rgap = h * 0.012;
        let row_h = h * 0.055;
        for (i, (label, hv, t)) in cats.iter().enumerate() {
            let ry = band_top + i as f32 * (row_h + rgap);
            push_progress_row(
                &mut rects, &mut texts, fonts, label, *hv, *t, margin, col_w, ry, row_h,
            );
        }

        // Beasts portrait grid (region×type) — encountered: bright foe; locked: dim silhouette.
        let beasts_top = band_top + 4.0 * (row_h + rgap) + h * 0.018;
        let (q, _) = fonts
            .tiny
            .layout("BEASTS", margin, beasts_top - h * 0.025, col_w);
        texts.push(TextRun {
            atlas: &fonts.tiny,
            quads: q,
            rgba: DIM,
        });
        push_codex_beast_grid(
            &mut rects, &mut texts, fonts, &beasts, margin, col_w, beasts_top, h, w,
        );

        push_neutral_button(&mut rects, &mut texts, fonts, "Back", w, h);
        return (rects, texts);
    }

    // Per-category owned/total over the catalogue (one pass), for the progress bars.
    use std::collections::BTreeMap;
    let mut have: BTreeMap<crate::catalogue::Category, u32> = BTreeMap::new();
    let mut tot: BTreeMap<crate::catalogue::Category, u32> = BTreeMap::new();
    for c in crate::catalogue::catalog() {
        *tot.entry(c.cat).or_default() += 1;
        if owned.contains(c.id.as_str()) {
            *have.entry(c.cat).or_default() += 1;
        }
    }

    // Section label + count (this tab's categories) — e.g. "AWARDS  2310 / 2310".
    let cats = inv_tab_cats(tab);
    let sec_tot: u32 = cats.iter().map(|c| *tot.get(c).unwrap_or(&0)).sum();
    let sec_have: u32 = cats.iter().map(|c| *have.get(c).unwrap_or(&0)).sum();
    let (q, _) = fonts
        .body
        .layout(&INV_TABS[tab].to_uppercase(), margin, h * 0.142, col_w);
    texts.push(TextRun {
        atlas: &fonts.body,
        quads: q,
        rgba: GOLD,
    });
    let sc = format!("{sec_have} / {sec_tot}");
    let scw = fonts.tiny.text_width(&sc);
    let (q, _) = fonts
        .tiny
        .layout(&sc, w - margin - scw, h * 0.15, scw + 4.0);
    texts.push(TextRun {
        atlas: &fonts.tiny,
        quads: q,
        rgba: DIM,
    });

    // One PROGRESS-BAR row per category: name + owned/total + ✓ (when complete) + a green fill bar.
    let n = cats.len().max(1);
    let band_top = h * 0.175;
    let band_bot = h * 0.70;
    let rgap = h * 0.012;
    let row_h = ((band_bot - band_top) / n as f32 - rgap).clamp(h * 0.03, h * 0.06);
    for (i, cat) in cats.iter().enumerate() {
        let (t, hv) = (*tot.get(cat).unwrap_or(&0), *have.get(cat).unwrap_or(&0));
        let ry = band_top + i as f32 * (row_h + rgap);
        rects.push(RectRun {
            x: margin,
            y: ry,
            w: col_w,
            h: row_h,
            rgba: PANEL,
        });
        let done = t > 0 && hv >= t;
        let label_col = if done { GREEN } else { BODY };
        let (q, _) = fonts.body.layout(
            &format!("{cat:?}"),
            margin + col_w * 0.04,
            ry + row_h * 0.12,
            col_w,
        );
        texts.push(TextRun {
            atlas: &fonts.body,
            quads: q,
            rgba: label_col,
        });
        let mut count_right = margin + col_w * 0.96;
        if done {
            let ck = row_h * 0.34;
            push_check(
                &mut rects,
                margin + col_w * 0.96 - ck * 1.2,
                ry + row_h * 0.3,
                ck,
                GREEN,
            );
            count_right = margin + col_w * 0.92 - ck * 1.2;
        }
        let cnt = format!("{hv} / {t}");
        let cw = fonts.tiny.text_width(&cnt);
        let (q, _) = fonts
            .tiny
            .layout(&cnt, count_right - cw, ry + row_h * 0.16, cw + 8.0);
        texts.push(TextRun {
            atlas: &fonts.tiny,
            quads: q,
            rgba: label_col,
        });
        // Progress bar (track + green fill).
        let (bx, by, bw, bh) = (
            margin + col_w * 0.04,
            ry + row_h * 0.62,
            col_w * 0.92,
            row_h * 0.18,
        );
        rects.push(RectRun {
            x: bx,
            y: by,
            w: bw,
            h: bh,
            rgba: INK,
        });
        let frac = if t > 0 { hv as f32 / t as f32 } else { 0.0 };
        if frac > 0.0 {
            rects.push(RectRun {
                x: bx,
                y: by,
                w: bw * frac,
                h: bh,
                rgba: GREEN,
            });
        }
    }

    // Detail strip: the first category's item tiles (icons), like web's "RANK 23/23" row.
    if let Some(first) = cats.first() {
        // V15: the detail strip uses the procedural FLAVOUR name (web's `it.flavour || it.name`),
        // not `name` (which for the Rank category is the bare title "Adept/Apprentice/…"). The
        // catalogue order is the export order — already what web shows.
        let strip: Vec<(String, String, String)> = crate::catalogue::catalog()
            .into_iter()
            .filter(|c| &c.cat == first)
            .map(|c| {
                let display = if c.flavour.is_empty() {
                    c.name
                } else {
                    c.flavour
                };
                (c.id, display, c.rarity)
            })
            .collect();
        let (q, _) = fonts.tiny.layout(
            &format!("{first:?}").to_uppercase(),
            margin,
            h * 0.715,
            col_w,
        );
        texts.push(TextRun {
            atlas: &fonts.tiny,
            quads: q,
            rgba: DIM,
        });
        let cols = 5usize;
        let sgap = w * 0.02;
        let tile = (col_w - sgap * (cols as f32 - 1.0)) / cols as f32;
        let sy = h * 0.74;
        for (i, (id, name, rarity)) in strip.iter().take(cols).enumerate() {
            let x = margin + i as f32 * (tile + sgap);
            let have_it = owned.contains(id.as_str());
            rects.push(RectRun {
                x: x - 1.5,
                y: sy - 1.5,
                w: tile + 3.0,
                h: tile + 3.0,
                rgba: if have_it { rarity_rgba(rarity) } else { DIM },
            });
            rects.push(RectRun {
                x,
                y: sy,
                w: tile,
                h: tile,
                rgba: INK,
            });
            if let Some((role, pal)) = crate::art::item_icon_for(id, rarity) {
                paint_role(
                    &mut rects,
                    &role,
                    &pal,
                    x + tile * 0.08,
                    sy + tile * 0.08,
                    tile * 0.84 / 16.0,
                );
            }
            // V46: wrap full flavour name into ≤2 lines instead of truncating at 9 chars.
            texts.push(TextRun {
                atlas: &fonts.tiny,
                quads: centered_wrapped(
                    &fonts.tiny,
                    name,
                    x + tile / 2.0,
                    sy + tile + h * 0.003,
                    h * 0.018,
                    tile,
                    2,
                ),
                rgba: if have_it { BODY } else { DIM },
            });
        }
    }

    push_neutral_button(&mut rects, &mut texts, fonts, "Back", w, h);
    (rects, texts)
}

/// Headless renders for the drill-down goldens (the representative sample save).
/// A **fully-collected** save (every catalogue item owned + a broad progression spread) — the state
/// the web visual-refs are captured in (everything unlocked + boosted), so Heroes/Items show maxed
/// effective stats and a full roster, matching `heroes-web.png`.
fn full_collection_sample() -> Save {
    let mut s = Save::default();
    let mut ts = 1u64;
    let mut mark = |s: &mut Save, k: String| {
        s.mark(&k, ts);
        ts += 1;
    };
    for m in crate::progression::modes() {
        for p in ["init", "mastery", "flawless"] {
            mark(&mut s, format!("{p}:{}", m.id));
        }
        for i in 0..4 {
            mark(&mut s, format!("speed:{}:{i}", m.id));
        }
    }
    for i in 1..=30 {
        mark(&mut s, format!("collector:{i}"));
    }
    for i in 1..=crate::combat::tier_count() {
        mark(&mut s, format!("tier:{i}"));
    }
    for c in crate::catalogue::catalog() {
        mark(&mut s, c.id.clone());
    }
    // …and every loot id (so the Inventory header reads the full 2702/2702).
    for id in all_loot_ids() {
        mark(&mut s, id);
    }
    s.gold = 99_999.0;
    s
}

pub fn render_heroes(painter: &crate::headless::Painter, font: &FontRef<'_>) -> Vec<u8> {
    let (w, h) = (DRILL_W as f32, DRILL_H as f32);
    let fonts = Fonts::bake(font, h);
    let (rects, texts) = heroes_frame(&full_collection_sample(), &fonts, w, h);
    painter.paint_rgba(DRILL_W, DRILL_H, BG, &rects, &texts)
}

/// Render the **Heroes** screen at the web reference aspect (430×880) — committed to halves as
/// `visual-ref/heroes-brickmap.png` for the Babysitter's side-by-side review.
pub fn render_heroes_ref(painter: &crate::headless::Painter, font: &FontRef<'_>) -> Vec<u8> {
    let (w, h) = (REF_W as f32, REF_H as f32);
    let fonts = Fonts::bake(font, h);
    let (rects, texts) = heroes_frame(&full_collection_sample(), &fonts, w, h);
    painter.paint_rgba(REF_W, REF_H, BG, &rects, &texts)
}

/// Render the **Heroes** screen in a PARTIAL save (only a few heroes unlocked) at the web reference
/// aspect — committed to halves as `visual-ref/heroes-partial-brickmap.png`. Exercises the locked-hero
/// rows (`?` portrait + unlock hint) the full-collection ref can't show.
pub fn render_heroes_partial_ref(
    painter: &crate::headless::Painter,
    font: &FontRef<'_>,
) -> Vec<u8> {
    let (w, h) = (REF_W as f32, REF_H as f32);
    let fonts = Fonts::bake(font, h);
    // N6 alignment: the heroes-partial web ref was captured against the `sample` capture state
    // (per `manifest.json`: `heroes-partial` → state `sample`), which yields 7/12 heroes
    // unlocked. The local `sample_save()` had a slightly different mix and rendered 3/12.
    let save = save_from_capture("sample");
    let (rects, texts) = heroes_frame(&save, &fonts, w, h);
    painter.paint_rgba(REF_W, REF_H, BG, &rects, &texts)
}
/// A fixed sample wall-clock for the event-play golden/visual-ref: 2026-06-26 14:30 UTC, which puts
/// `bondfire-night` (roster index 3) live — the event the sample save has already cleared
/// (`event:bondfire-night`), so its "Play" tier renders earned — with 9h 30m left on the countdown.
const EVENT_SAMPLE_NOW_MS: i64 = 1_782_052_200_000;

pub fn render_events(painter: &crate::headless::Painter, font: &FontRef<'_>) -> Vec<u8> {
    let (w, h) = (DRILL_W as f32, DRILL_H as f32);
    let fonts = Fonts::bake(font, h);
    let (rects, texts) = event_play_frame(&sample_save(), EVENT_SAMPLE_NOW_MS, None, &fonts, w, h);
    painter.paint_rgba(DRILL_W, DRILL_H, BG, &rects, &texts)
}

/// Render the **gauntlet drill** at the web reference aspect (430×880) — committed to halves as
/// `visual-ref/event-play-brickmap.png` to sit beside `event-play-web.png` (which captures the
/// *drill in gauntlet mode*, not a menu). The drill UI is shared with topic play; the heading is the
/// event name (a gauntlet spans several topics), the progress its size. Rendered mid-run (one solved)
/// so the "N / M" counter reads like the reference's "1 / 12".
/// Render the **Settings** screen at the web reference aspect — `visual-ref/settings-brickmap.png`.
pub fn render_settings_ref(painter: &crate::headless::Painter, font: &FontRef<'_>) -> Vec<u8> {
    let (w, h) = (REF_W as f32, REF_H as f32);
    let fonts = Fonts::bake(font, h);
    let (rects, texts) = settings_frame(&fonts, w, h);
    painter.paint_rgba(REF_W, REF_H, BG, &rects, &texts)
}

/// Render the **Best Times** screen at the web reference aspect — `visual-ref/summary-brickmap.png`.
/// Uses a representative save with seeded `best_times` (else every row would read "—").
pub fn render_summary_ref(painter: &crate::headless::Painter, font: &FontRef<'_>) -> Vec<u8> {
    let (w, h) = (REF_W as f32, REF_H as f32);
    let fonts = Fonts::bake(font, h);
    // V33: web `summary-web.png` is the EMPTY "Not played" capture-state — match it. Every card
    // reads "—" / "Not played" until the Babysitter re-captures the web ref against a seeded profile.
    let save = Save::default();
    let (rects, texts) = summary_frame(&save, &fonts, w, h);
    painter.paint_rgba(REF_W, REF_H, BG, &rects, &texts)
}

/// Render the **Audio** screen at the web reference aspect — `visual-ref/audio-brickmap.png`.
pub fn render_audio_ref(painter: &crate::headless::Painter, font: &FontRef<'_>) -> Vec<u8> {
    let (w, h) = (REF_W as f32, REF_H as f32);
    let fonts = Fonts::bake(font, h);
    let (rects, texts) = audio_frame(&fonts, w, h);
    painter.paint_rgba(REF_W, REF_H, BG, &rects, &texts)
}

/// Render the **Arena Journey Map** at the web reference aspect — `visual-ref/arena-map-brickmap.png`.
/// Uses a sample save with `tier:1..29` cleared (web ref captures tier 30 / region 2 "Gloamwood
/// YOU ARE HERE").
pub fn render_arena_map_ref(painter: &crate::headless::Painter, font: &FontRef<'_>) -> Vec<u8> {
    let (w, h) = (REF_W as f32, REF_H as f32);
    let fonts = Fonts::bake(font, h);
    let mut save = full_collection_sample();
    // Reset arena progress: rebuild collected, dropping all `tier:N` keys, then mark `tier:1..29`.
    save.collected.retain(|k, _| !k.starts_with("tier:"));
    let mut ts = save.collected.values().map(|s| s.ts).max().unwrap_or(0) + 1;
    for n in 1..=29u32 {
        save.mark(format!("tier:{n}").as_str(), ts);
        ts += 1;
    }
    let party = vec!["bram".to_string()];
    let (rects, texts) = arena_map_frame(&save, &party, &fonts, w, h);
    painter.paint_rgba(REF_W, REF_H, BG, &rects, &texts)
}

/// Render the **Arena CLEARED** screen at the web reference aspect — `visual-ref/arena-cleared-brickmap.png`.
pub fn render_arena_cleared_ref(painter: &crate::headless::Painter, font: &FontRef<'_>) -> Vec<u8> {
    let (w, h) = (REF_W as f32, REF_H as f32);
    let fonts = Fonts::bake(font, h);
    let (rects, texts) = arena_cleared_frame(&fonts, w, h);
    painter.paint_rgba(REF_W, REF_H, BG, &rects, &texts)
}

/// Render a **drill** in the web reference aspect (430×880) — committed to halves as
/// `visual-ref/drill-brickmap.png` for the side-by-side review (mirrors `drill-web.png`, which
/// captures a Halves round at "1 / 27 · 1.2s · half of 144").
pub fn render_drill_ref(painter: &crate::headless::Painter, font: &FontRef<'_>) -> Vec<u8> {
    let (w, h) = (REF_W as f32, REF_H as f32);
    let fonts = Fonts::bake(font, h);
    let drill = Drill::from_topic("halves");
    let margin = w * 0.06;
    let kp_w = w - margin * 2.0;
    let kp_h = h * 0.40;
    let kp_y = h - kp_h - margin;
    let keypad = Keypad::layout(margin, kp_y, kp_w, kp_h, w * 0.018);
    let (rects, texts) = drill_frame(&drill, &keypad, &fonts, w, h, None, Some(1.2));
    painter.paint_rgba(REF_W, REF_H, BG, &rects, &texts)
}

pub fn render_event_play_ref(painter: &crate::headless::Painter, font: &FontRef<'_>) -> Vec<u8> {
    let (w, h) = (REF_W as f32, REF_H as f32);
    let fonts = Fonts::bake(font, h);
    let eid = crate::event_play::live_event(EVENT_SAMPLE_NOW_MS).id;
    let mut drill = Drill::from_gauntlet(&eid);
    // Solve the first question so the render shows a mid-gauntlet state (progress advances; the next
    // prompt is live) — typing its exact answer auto-accepts, no submit key.
    for c in format!("{}", drill.expected()).chars() {
        if let Some(k) = Keypad::key_for_char(c) {
            drill.press(k);
        }
    }
    let margin = w * 0.06;
    let kp_w = w - margin * 2.0;
    let kp_h = h * 0.40;
    let kp_y = h - kp_h - margin;
    let keypad = Keypad::layout(margin, kp_y, kp_w, kp_h, w * 0.018);
    // A representative per-question time so the gauntlet ref shows the speed timer (web's "1.3s").
    let (rects, texts) = drill_frame(&drill, &keypad, &fonts, w, h, None, Some(1.3));
    painter.paint_rgba(REF_W, REF_H, BG, &rects, &texts)
}
pub fn render_items(painter: &crate::headless::Painter, font: &FontRef<'_>) -> Vec<u8> {
    let (w, h) = (DRILL_W as f32, DRILL_H as f32);
    let fonts = Fonts::bake(font, h);
    // The Awards tab over a full collection (every icon owned) — matches the web ref's seeded state.
    let (rects, texts) = items_frame(
        &full_collection_sample(),
        1,
        EVENT_SAMPLE_NOW_MS,
        &fonts,
        w,
        h,
    );
    painter.paint_rgba(DRILL_W, DRILL_H, BG, &rects, &texts)
}

/// Render the **Inventory** screen at the web reference aspect (430×880) — committed to halves as
/// `visual-ref/inventory-awards-brickmap.png` for the Babysitter's side-by-side review.
pub fn render_items_ref(painter: &crate::headless::Painter, font: &FontRef<'_>) -> Vec<u8> {
    let (w, h) = (REF_W as f32, REF_H as f32);
    let fonts = Fonts::bake(font, h);
    let (rects, texts) = items_frame(
        &full_collection_sample(),
        1,
        EVENT_SAMPLE_NOW_MS,
        &fonts,
        w,
        h,
    );
    painter.paint_rgba(REF_W, REF_H, BG, &rects, &texts)
}

/// Render the Inventory **Topics** tab (per-topic progress bars) at the web reference aspect —
/// committed to halves as `visual-ref/inventory-topics-brickmap.png`.
pub fn render_inventory_topics_ref(
    painter: &crate::headless::Painter,
    font: &FontRef<'_>,
) -> Vec<u8> {
    let (w, h) = (REF_W as f32, REF_H as f32);
    let fonts = Fonts::bake(font, h);
    let (rects, texts) = items_frame(
        &full_collection_sample(),
        0,
        EVENT_SAMPLE_NOW_MS,
        &fonts,
        w,
        h,
    );
    painter.paint_rgba(REF_W, REF_H, BG, &rects, &texts)
}

/// Render the Inventory **Loot** tab (per-region loot progress bars) at the web reference aspect —
/// committed to halves as `visual-ref/inventory-loot-brickmap.png`.
pub fn render_inventory_loot_ref(
    painter: &crate::headless::Painter,
    font: &FontRef<'_>,
) -> Vec<u8> {
    let (w, h) = (REF_W as f32, REF_H as f32);
    let fonts = Fonts::bake(font, h);
    let (rects, texts) = items_frame(
        &full_collection_sample(),
        3,
        EVENT_SAMPLE_NOW_MS,
        &fonts,
        w,
        h,
    );
    painter.paint_rgba(REF_W, REF_H, BG, &rects, &texts)
}

/// Render the Inventory **Events** tab (live-event banner + reward-item grid) at the web reference
/// aspect — committed to halves as `visual-ref/inventory-events-brickmap.png`.
pub fn render_inventory_events_ref(
    painter: &crate::headless::Painter,
    font: &FontRef<'_>,
) -> Vec<u8> {
    let (w, h) = (REF_W as f32, REF_H as f32);
    let fonts = Fonts::bake(font, h);
    let (rects, texts) = items_frame(
        &full_collection_sample(),
        2,
        EVENT_SAMPLE_NOW_MS,
        &fonts,
        w,
        h,
    );
    painter.paint_rgba(REF_W, REF_H, BG, &rects, &texts)
}

/// Render a **Hero Detail** screen for `hero_id` at the web reference aspect — committed to halves as
/// `visual-ref/hero-detail-<type>-brickmap.png` for the Babysitter's side-by-side review.
pub fn render_hero_detail_ref(
    painter: &crate::headless::Painter,
    font: &FontRef<'_>,
    hero_id: &str,
) -> Vec<u8> {
    let (w, h) = (REF_W as f32, REF_H as f32);
    let fonts = Fonts::bake(font, h);
    let (rects, texts) = hero_detail_frame(&full_collection_sample(), hero_id, &fonts, w, h);
    painter.paint_rgba(REF_W, REF_H, BG, &rects, &texts)
}

/// Render the Inventory **Codex** tab (4 category rows + Beasts portrait grid) at the web reference
/// aspect — committed to halves as `visual-ref/inventory-codex-brickmap.png`.
pub fn render_inventory_codex_ref(
    painter: &crate::headless::Painter,
    font: &FontRef<'_>,
) -> Vec<u8> {
    let (w, h) = (REF_W as f32, REF_H as f32);
    let fonts = Fonts::bake(font, h);
    let (rects, texts) = items_frame(
        &full_collection_sample(),
        4,
        EVENT_SAMPLE_NOW_MS,
        &fonts,
        w,
        h,
    );
    painter.paint_rgba(REF_W, REF_H, BG, &rects, &texts)
}

/// The id of the unlocked topic whose row contains (`px`,`py`), if any (touch routing).
pub fn topic_at(
    modes: &[progression::Mode],
    progress: &progression::Progress,
    w: f32,
    h: f32,
    px: f32,
    py: f32,
) -> Option<String> {
    let unlocked: Vec<&progression::Mode> =
        modes.iter().filter(|m| progress.is_unlocked(m)).collect();
    topic_rows(unlocked.len(), w, h)
        .into_iter()
        .zip(&unlocked)
        .find(|((rx, ry, rw, rh), _)| px >= *rx && px < rx + rw && py >= *ry && py < ry + rh)
        .map(|(_, m)| m.id.clone())
}

/// Render the **initial topic-select** (fresh progress → only the root topic unlocked) headless.
/// Shared by the golden blesser + golden test.
pub fn render_topic_select(painter: &crate::headless::Painter, font: &FontRef<'_>) -> Vec<u8> {
    let (w, h) = (DRILL_W as f32, DRILL_H as f32);
    let fonts = Fonts::bake(font, h);
    let modes = progression::modes();
    let progress = progression::Progress::default();
    let (rects, texts) = topic_select_frame(&modes, &progress, &fonts, w, h);
    painter.paint_rgba(DRILL_W, DRILL_H, BG, &rects, &texts)
}

/// A progress with **every** topic unlocked (a fully-progressed save) — for the
/// [`render_topic_select_full`] golden that proves the grid fits all 46 on-screen.
fn all_unlocked() -> progression::Progress {
    let keys: Vec<String> = progression::modes()
        .iter()
        .map(|m| format!("init:{}", m.id))
        .collect();
    progression::Progress::from_collected(keys.iter().map(String::as_str))
}

/// Render the topic-select with **all 46 topics unlocked** (the worst case for layout) headless —
/// the multi-column grid keeps every row on-screen. Shared by the golden blesser + golden test.
pub fn render_topic_select_full(painter: &crate::headless::Painter, font: &FontRef<'_>) -> Vec<u8> {
    let (w, h) = (DRILL_W as f32, DRILL_H as f32);
    let fonts = Fonts::bake(font, h);
    let modes = progression::modes();
    let (rects, texts) = topic_select_frame(&modes, &all_unlocked(), &fonts, w, h);
    painter.paint_rgba(DRILL_W, DRILL_H, BG, &rects, &texts)
}

/// The synced `capture-states.json` — the exact localStorage `collected` states the web visual refs
/// were captured with (+ the shared `gold`). Lets the state-dependent refs render against the SAME
/// data as the web (N6), so the compare reflects VISUAL diffs only.
const CAPTURE_STATES_JSON: &str = include_str!("../data/gg1/capture-states.json");

/// Build a [`Save`] from a named capture state (`full`/`empty`/`sample`/`partial`): its `collected`
/// keys + the shared `gold` balance.
fn save_from_capture(state: &str) -> Save {
    let v: serde_json::Value =
        serde_json::from_str(CAPTURE_STATES_JSON).expect("capture-states.json");
    let mut s = Save::default();
    if let Some(obj) = v.get(state).and_then(|x| x.as_object()) {
        let mut ts = 1u64;
        for k in obj.keys() {
            s.mark(k, ts);
            ts += 1;
        }
    }
    s.gold = v
        .get("gold")
        .and_then(|g| g.as_str())
        .and_then(|g| g.parse().ok())
        .unwrap_or(0.0);
    s
}

/// Render the **Home** hub for a given save at the web reference aspect (430×880).
fn render_home_with(
    painter: &crate::headless::Painter,
    font: &FontRef<'_>,
    save: &Save,
) -> Vec<u8> {
    let (w, h) = (REF_W as f32, REF_H as f32);
    let fonts = Fonts::bake(font, h);
    let modes = progression::modes();
    let progress = save.progress();
    let (rects, texts) = home_frame(save, &modes, &progress, EVENT_SAMPLE_NOW_MS, &fonts, w, h);
    painter.paint_rgba(REF_W, REF_H, BG, &rects, &texts)
}

/// Render the **Home** hub (fully-progressed `full` capture state) — committed to halves as
/// `visual-ref/home-brickmap.png`. Gold + every topic unlocked/mastered + the cleared event.
pub fn render_home_ref(painter: &crate::headless::Painter, font: &FontRef<'_>) -> Vec<u8> {
    let mut sample = full_collection_sample();
    sample.gold = 987_654_321.0; // the exact `capture-states.json` home balance ("988M" + ~½ pile).
    render_home_with(painter, font, &sample)
}

/// Render the **Home** hub for a NEW player (`empty` capture state → only the root topic unlocked,
/// every other node locked, "Play" event) — `visual-ref/home-fresh-brickmap.png`.
pub fn render_home_fresh_ref(painter: &crate::headless::Painter, font: &FontRef<'_>) -> Vec<u8> {
    render_home_with(painter, font, &save_from_capture("empty"))
}

/// Render the **Home** hub MID-GAME (`sample` capture state → a few topics unlocked/mastered, the
/// rest locked) — `visual-ref/home-midprogress-brickmap.png`.
pub fn render_home_midprogress_ref(
    painter: &crate::headless::Painter,
    font: &FontRef<'_>,
) -> Vec<u8> {
    render_home_with(painter, font, &save_from_capture("sample"))
}

/// A representative save for the Collection golden — a player who's made some progress, so earned
/// and not-yet-earned both show.
fn sample_save() -> Save {
    let mut s = Save::default();
    for (i, k) in [
        "init:halves",
        "mastery:halves",
        "flawless:halves",
        "init:times",
        "rank:goblin",
        "rank:kobold",
        "speed:halves:0",
        "collector:25",
        "event:bondfire-night",
    ]
    .iter()
    .enumerate()
    {
        s.mark(*k, 1000 + i as u64);
    }
    s.gold = 1234.0;
    s
}

/// Render the **Collection** screen for a representative save, headless. Shared by the golden
/// blesser + golden test.
pub fn render_collection(painter: &crate::headless::Painter, font: &FontRef<'_>) -> Vec<u8> {
    let (w, h) = (DRILL_W as f32, DRILL_H as f32);
    let fonts = Fonts::bake(font, h);
    let save = sample_save();
    let (rects, texts) = collection_frame(&save, &fonts, w, h);
    painter.paint_rgba(DRILL_W, DRILL_H, BG, &rects, &texts)
}

// ── procedural-art painting (F1–F4): role grids + colour grids → engine rects ──────────────────

/// Paint a 16×16 **role grid** (hero/foe portrait) at `(x0,y0)` with `cell`-px cells, colouring each
/// non-empty cell through `pal` ([`crate::art::Palette::role_hex`]). Cells overlap a hair to avoid
/// seams. Shared by every screen that shows a portrait.
pub fn paint_role(
    rects: &mut Vec<RectRun>,
    role: &crate::art::RoleGrid,
    pal: &crate::art::Palette,
    x0: f32,
    y0: f32,
    cell: f32,
) {
    for (y, row) in role.iter().enumerate() {
        for (x, &c) in row.iter().enumerate() {
            if let Some(hex) = pal.role_hex(c) {
                rects.push(RectRun {
                    x: x0 + x as f32 * cell,
                    y: y0 + y as f32 * cell,
                    w: cell + 0.5,
                    h: cell + 0.5,
                    rgba: crate::art::hex_rgba(hex),
                });
            }
        }
    }
}

/// Paint a low-fi **backspace** glyph (a left arrow: a `<` chevron + a short shaft) centred at
/// `(cx,cy)` with `s`-px pixels. The text face lacks U+232B ⌫, and a pixel arrow fits the game's
/// aesthetic better than the ASCII `<` fallback (a Babysitter parity note on the keypad).
fn paint_backspace(rects: &mut Vec<RectRun>, cx: f32, cy: f32, s: f32) {
    for (ix, iy) in [
        (-2, 0),
        (-1, -1),
        (0, -2),
        (-1, 1),
        (0, 2),
        (-1, 0),
        (0, 0),
        (1, 0),
        (2, 0),
    ] {
        rects.push(RectRun {
            x: cx + ix as f32 * s - s / 2.0,
            y: cy + iy as f32 * s - s / 2.0,
            w: s + 0.6,
            h: s + 0.6,
            rgba: BODY,
        });
    }
}

/// Paint a full **colour grid** (a scenery backdrop or event crest) at `(x0,y0)` with `cw`×`ch` cells.
pub fn paint_colors(
    rects: &mut Vec<RectRun>,
    grid: &crate::scenes::ColorGrid,
    x0: f32,
    y0: f32,
    cw: f32,
    ch: f32,
) {
    for (r, row) in grid.iter().enumerate() {
        for (c, hex) in row.iter().enumerate() {
            rects.push(RectRun {
                x: x0 + c as f32 * cw,
                y: y0 + r as f32 * ch,
                w: cw + 0.6,
                h: ch + 0.6,
                rgba: crate::art::hex_rgba(hex),
            });
        }
    }
}

// ── Arena screen (party-pick → battle → grant) ────────────────────────────────────────────────

/// The display colour for a combatant type (matching the portrait base hues).
fn type_rgba(kind: crate::arena::Kind) -> [f32; 4] {
    match kind {
        crate::arena::Kind::Brawn => crate::art::hex_rgba("#d05a4a"),
        crate::arena::Kind::Arcane => crate::art::hex_rgba("#8a5cf6"),
        crate::arena::Kind::Cunning => crate::art::hex_rgba("#3fce8c"),
    }
}

/// A hero's `★` rating (`power·1 + focus·0.8 + speed·0.5 + guard·0.3`, rounded) — the party-picker
/// heuristic shown on each card (per `combat.json constants.rating`).
fn hero_rating(s: &crate::arena::Stats) -> i64 {
    (s.power as f64 + s.focus as f64 * 0.8 + s.speed as f64 * 0.5 + s.guard as f64 * 0.3 + 0.5)
        .floor() as i64
}

/// The Arena hero-card layout band + spacing (shared by [`arena_frame`] + the row hit-test).
const ARENA_ROWS_TOP: f32 = 0.40;
const ARENA_ROWS_BOT: f32 = 0.79;

/// Row rects for `count` Arena hero cards, sized to `w`×`h` (single column, height shrunk to fit).
fn arena_hero_rows(count: usize, w: f32, h: f32) -> Vec<(f32, f32, f32, f32)> {
    if count == 0 {
        return Vec::new();
    }
    let margin = w * 0.05;
    let top = h * ARENA_ROWS_TOP;
    let band = h * (ARENA_ROWS_BOT - ARENA_ROWS_TOP);
    let gap = h * 0.012;
    let row_h = (band / count as f32 - gap).clamp(h * 0.045, h * 0.085);
    (0..count)
        .map(|i| {
            (
                margin,
                top + i as f32 * (row_h + gap),
                w - margin * 2.0,
                row_h,
            )
        })
        .collect()
}

/// Build the **Arena** screen: a foe showcase (region backdrop + foe portrait + name/type/stats over
/// the next tier) and a party-pick list of the player's **unlocked** heroes (portrait + type dot +
/// `★`rating + effective stat chips), with a Fight bar and a Back button. A prior outcome shows as a
/// banner. Shared by the on-device renderer + the golden.
pub fn arena_frame<'a>(
    save: &Save,
    party: &[String],
    last: Option<&crate::save::ArenaOutcome>,
    fonts: &'a Fonts,
    w: f32,
    h: f32,
) -> (Vec<RectRun>, Vec<TextRun<'a>>) {
    let mut rects: Vec<RectRun> = Vec::new();
    let mut texts: Vec<TextRun> = Vec::new();
    let margin = w * 0.05;
    let col_w = w - margin * 2.0;
    let keys: std::collections::HashSet<&str> = save.collected.keys().map(String::as_str).collect();
    let tier = crate::combat::next_tier(save.collected.keys().map(String::as_str));
    let region = crate::combat::tier_region(tier);

    // Heading + tier counter.
    let (q, _) = fonts.body.layout("ARENA", margin, h * 0.028, col_w);
    texts.push(TextRun {
        atlas: &fonts.body,
        quads: q,
        rgba: GOLD,
    });
    let tlabel = format!("Tier {tier} / {}", crate::combat::tier_count());
    let tw = fonts.body.text_width(&tlabel);
    let (q, _) = fonts
        .body
        .layout(&tlabel, w - margin - tw, h * 0.043, tw + 4.0);
    texts.push(TextRun {
        atlas: &fonts.body,
        quads: q,
        rgba: DIM,
    });

    // Foe TEAM showcase: region backdrop band, the tier's name, then the three typed foe cards (the
    // actual 3v3 you fight — lead + supports), each with a mini portrait, its type, and PWR/HP.
    let (sy, sh) = (h * 0.082, h * 0.20);
    let backdrop = crate::scenes::scenery_grid(region as i64);
    paint_colors(&mut rects, &backdrop, margin, sy, col_w / 28.0, sh / 11.0);
    let foes = crate::combat::tier_foes(tier);
    let lead_kind = foes
        .first()
        .map(|f| f.0)
        .unwrap_or(crate::arena::Kind::Brawn);
    let foe_name = crate::arena::bestiary()
        .into_iter()
        .find(|e| e.n == tier)
        .map(|e| e.name)
        .unwrap_or_else(|| format!("Tier {tier}"));
    let nm = centered(&fonts.body, &foe_name, w / 2.0, sy + sh * 0.02, h * 0.034);
    texts.push(TextRun {
        atlas: &fonts.body,
        quads: nm,
        rgba: BODY,
    });
    let nfoe = foes.len().max(1);
    let fgap = w * 0.02;
    let fcw = (col_w - fgap * (nfoe as f32 - 1.0)) / nfoe as f32;
    let fcard_y = sy + sh * 0.14;
    let fcard_h = sh * 0.84;
    for (i, &(kind, pow, hp)) in foes.iter().enumerate() {
        let fx = margin + i as f32 * (fcw + fgap);
        // A dark card floats over the backdrop so the portrait + stats read.
        rects.push(RectRun {
            x: fx,
            y: fcard_y,
            w: fcw,
            h: fcard_h,
            rgba: [INK[0], INK[1], INK[2], 0.74],
        });
        // Mini portrait — the lead silhouette tinted by THIS foe's type (supports carry no own art
        // export; their type is the real signal). Index-varied seed so supports look distinct.
        let seed_name = if i == 0 {
            foe_name.clone()
        } else {
            format!("{foe_name} {i}")
        };
        let (role, pal) = crate::art::foe_grid(tier, &seed_name, kind);
        let pcell = (fcard_h * 0.5) / 16.0;
        paint_role(
            &mut rects,
            &role,
            &pal,
            fx + fcw / 2.0 - 8.0 * pcell,
            fcard_y + fcard_h * 0.05,
            pcell,
        );
        // Type + PWR/HP under the portrait.
        let tline = centered(
            &fonts.tiny,
            &format!("{kind:?}"),
            fx + fcw / 2.0,
            fcard_y + fcard_h * 0.62,
            h * 0.026,
        );
        texts.push(TextRun {
            atlas: &fonts.tiny,
            quads: tline,
            rgba: type_rgba(kind),
        });
        let sline = centered(
            &fonts.tiny,
            &format!("{} P · {} H", pow.round() as i64, hp.round() as i64),
            fx + fcw / 2.0,
            fcard_y + fcard_h * 0.82,
            h * 0.024,
        );
        texts.push(TextRun {
            atlas: &fonts.tiny,
            quads: sline,
            rgba: BODY,
        });
    }

    // "How battles work" primer (the collapsible info box) — the type-triangle strategy.
    let primer_y = sy + sh + h * 0.012;
    rects.push(RectRun {
        x: margin,
        y: primer_y,
        w: col_w,
        h: h * 0.055,
        rgba: PANEL,
    });
    let (q, _) = fonts.tiny.layout(
        "How battles work:  Brawn > Cunning > Arcane > Brawn  (matchup ×1.5)",
        margin + col_w * 0.03,
        primer_y + h * 0.009,
        col_w * 0.94,
    );
    texts.push(TextRun {
        atlas: &fonts.tiny,
        quads: q,
        rgba: DIM,
    });

    // Party-pick header.
    let unlocked = crate::combat::unlocked_roster(&keys);
    let pick = format!("Choose your party  {}/3", party.len());
    let (q, _) = fonts.body.layout(&pick, margin, h * 0.375, col_w);
    texts.push(TextRun {
        atlas: &fonts.body,
        quads: q,
        rgba: GOLD,
    });

    // Hero cards (unlocked only).
    let roster = crate::arena::roster();
    for ((rx, ry, rw, rh), id) in arena_hero_rows(unlocked.len(), w, h)
        .into_iter()
        .zip(&unlocked)
    {
        let selected = party.iter().any(|p| p == id);
        // Selected = a subtle amber BORDER over the panel (web's look), not a heavy fill.
        if selected {
            rects.push(RectRun {
                x: rx - 2.0,
                y: ry - 2.0,
                w: rw + 4.0,
                h: rh + 4.0,
                rgba: GOLD,
            });
        }
        rects.push(RectRun {
            x: rx,
            y: ry,
            w: rw,
            h: rh,
            rgba: PANEL,
        });
        let hero = roster.iter().find(|hh| &hh.id == id);
        let kind = hero.map(|hh| hh.kind).unwrap_or(crate::arena::Kind::Brawn);
        let name = hero.map(|hh| hh.name.clone()).unwrap_or_else(|| id.clone());
        // Portrait on the left.
        let (role, pal) = crate::art::hero_icon(id, kind);
        let pcell = (rh * 0.82) / 16.0;
        paint_role(
            &mut rects,
            &role,
            &pal,
            rx + rh * 0.1,
            ry + rh * 0.09,
            pcell,
        );
        // Type dot + name + rating (top line); effective stat chips (bottom line).
        let stats = crate::combat::effective_stats(id, &keys).unwrap_or_default();
        let tx = rx + rh + w * 0.02;
        let dot = rh * 0.14;
        rects.push(RectRun {
            x: tx,
            y: ry + rh * 0.2,
            w: dot,
            h: dot,
            rgba: type_rgba(kind),
        });
        let (q, _) = fonts.body.layout(&name, tx + dot * 1.6, ry + rh * 0.16, rw);
        texts.push(TextRun {
            atlas: &fonts.body,
            quads: q,
            rgba: type_rgba(kind),
        });
        // V28: ★ pixel star + the rating number.
        let count = format!("{}", hero_rating(&stats));
        let ctw = fonts.body.text_width(&count);
        let sw = rh * 0.36;
        let count_x = rx + rw - ctw - w * 0.02;
        let star_x = count_x - sw * 1.3;
        push_star(&mut rects, star_x, ry + rh * 0.22, sw, GOLD);
        let (q, _) = fonts
            .body
            .layout(&count, count_x, ry + rh * 0.16, ctw + 4.0);
        texts.push(TextRun {
            atlas: &fonts.body,
            quads: q,
            rgba: GOLD,
        });
        let chips = format!(
            "{} PWR  {} GRD  {} SPD  {} FOC",
            stats.power, stats.guard, stats.speed, stats.focus
        );
        let (q, _) = fonts.tiny.layout(&chips, tx, ry + rh * 0.55, rw);
        texts.push(TextRun {
            atlas: &fonts.tiny,
            quads: q,
            rgba: DIM,
        });
        // Matchup badge vs the lead foe — the type-triangle strategy signal, bottom-right of the card.
        let mu = crate::combat::matchup_mult(kind, lead_kind);
        let (badge, bcol) = if mu > 1.0 {
            ("ADV ×1.5".to_string(), GREEN)
        } else if mu < 1.0 {
            ("WEAK".to_string(), type_rgba(crate::arena::Kind::Brawn))
        } else {
            ("EVEN".to_string(), DIM)
        };
        let bw_txt = fonts.tiny.text_width(&badge);
        let (q, _) = fonts.tiny.layout(
            &badge,
            rx + rw - bw_txt - w * 0.02,
            ry + rh * 0.58,
            bw_txt + 4.0,
        );
        texts.push(TextRun {
            atlas: &fonts.tiny,
            quads: q,
            rgba: bcol,
        });
    }

    // Outcome banner (if any).
    if let Some(o) = last {
        let (msg, col) = if o.win {
            (
                format!("Victory! Cleared tier {} (+{}g)", o.tier, o.gold_earned),
                GREEN,
            )
        } else {
            (
                format!("Defeated at tier {} ({} rounds)", o.tier, o.rounds),
                DIM,
            )
        };
        let mw = fonts.body.text_width(&msg);
        let (q, _) = fonts
            .body
            .layout(&msg, w / 2.0 - mw / 2.0, h * 0.805, mw + 4.0);
        texts.push(TextRun {
            atlas: &fonts.body,
            quads: q,
            rgba: col,
        });
    }

    // Fight bar — the PRIMARY CTA, always SOLID gold (parity V17; the label conveys state, not the
    // fill). "Pick your party" prompts before a pick, "Fight Tier N" once chosen.
    let (fbx, fby, fbw, fbh) = arena_fight_button(w, h);
    rects.push(RectRun {
        x: fbx,
        y: fby,
        w: fbw,
        h: fbh,
        rgba: GOLD,
    });
    let flabel = if party.is_empty() {
        "Pick your party".to_string()
    } else {
        format!("Fight Tier {tier}")
    };
    texts.push(TextRun {
        atlas: &fonts.key,
        quads: centered(&fonts.key, &flabel, fbx + fbw / 2.0, fby, fbh),
        rgba: INK,
    });
    // "Journey map" button (region-progress view; route stubbed) — pre-fight only, so it doesn't
    // collide with the post-fight outcome banner.
    if last.is_none() {
        let (jx, jy, jw, jh) = arena_journey_button(w, h);
        rects.push(RectRun {
            x: jx,
            y: jy,
            w: jw,
            h: jh,
            rgba: PANEL,
        });
        texts.push(TextRun {
            atlas: &fonts.body,
            quads: centered(&fonts.body, "Journey map", jx + jw / 2.0, jy, jh),
            rgba: DIM,
        });
    }
    push_neutral_button(&mut rects, &mut texts, fonts, "Back", w, h);
    (rects, texts)
}

/// The **Arena Journey Map** — web's tier-30/120 view: ARENA TIER N/M header + "Hide journey map"
/// toggle + 10 region rows (status: CONQUERED ✓ / YOU ARE HERE / LOCKED, with the region label,
/// boss name, and status colour) + the current-foe portrait card + Back/"Pick your party" CTAs.
pub fn arena_map_frame<'a>(
    save: &Save,
    party: &[String],
    fonts: &'a Fonts,
    w: f32,
    h: f32,
) -> (Vec<RectRun>, Vec<TextRun<'a>>) {
    let mut rects: Vec<RectRun> = Vec::new();
    let mut texts: Vec<TextRun> = Vec::new();
    let margin = w * 0.04;
    let col_w = w - margin * 2.0;
    let tier = crate::combat::next_tier(save.collected.keys().map(String::as_str));
    let cur_region = crate::combat::tier_region(tier);
    let total = crate::combat::tier_count();
    let rsize = crate::combat::region_size();
    let nregions = (total / rsize) as usize;

    // Header.
    let head = format!("ARENA TIER {tier} / {total}");
    texts.push(TextRun {
        atlas: &fonts.tiny,
        quads: centered(&fonts.tiny, &head, w / 2.0, h * 0.018, h * 0.03),
        rgba: GOLD,
    });

    // Hide-journey-map toggle (outlined wide pill).
    let (tx, ty, tw, th) = (margin, h * 0.058, col_w, h * 0.046);
    rects.push(RectRun {
        x: tx,
        y: ty,
        w: tw,
        h: th,
        rgba: [DIM[0], DIM[1], DIM[2], 0.5],
    });
    rects.push(RectRun {
        x: tx + 1.5,
        y: ty + 1.5,
        w: tw - 3.0,
        h: th - 3.0,
        rgba: KEYBG,
    });
    texts.push(TextRun {
        atlas: &fonts.tiny,
        quads: centered(&fonts.tiny, "• Hide journey map", tx + tw / 2.0, ty, th),
        rgba: BODY,
    });

    // 10 region rows.
    let rows_top = h * 0.125;
    let row_h = h * 0.04;
    let rgap = h * 0.005;
    const GREENF: [f32; 4] = [80.0 / 255.0, 180.0 / 255.0, 120.0 / 255.0, 1.0];
    const AMBER: [f32; 4] = [240.0 / 255.0, 173.0 / 255.0, 60.0 / 255.0, 1.0];
    for r in 0..nregions {
        let ry = rows_top + r as f32 * (row_h + rgap);
        rects.push(RectRun {
            x: margin,
            y: ry,
            w: col_w,
            h: row_h,
            rgba: KEYBG,
        });
        // Left status pip (green=conquered, amber=here, dim=locked).
        let r32 = r as u32;
        let (status, status_col, label_col, status_text) = if r32 < cur_region {
            ("CONQUERED", GREENF, GREENF, "x")
        } else if r32 == cur_region {
            ("YOU ARE HERE", AMBER, GOLD, "x")
        } else {
            ("LOCKED", DIM, DIM, "x")
        };
        let pip_sq = row_h * 0.42;
        rects.push(RectRun {
            x: margin + row_h * 0.18,
            y: ry + row_h * 0.29,
            w: pip_sq,
            h: pip_sq,
            rgba: status_col,
        });
        // Region name (label_col).
        let (q, _) = fonts.tiny.layout(
            crate::combat::region_name(r),
            margin + row_h * 1.05,
            ry + row_h * 0.28,
            col_w * 0.4,
        );
        texts.push(TextRun {
            atlas: &fonts.tiny,
            quads: q,
            rgba: label_col,
        });
        // V48: "⚔ <Boss>" centre (pixel crossed-swords + name; was an ASCII `x`).
        let _ = status_text;
        let boss_n = (r32 + 1) * rsize;
        let boss_name = crate::combat::tier_meta(boss_n)
            .map(|(n, _, _)| n)
            .unwrap_or_else(|| "Boss".to_string());
        let sword_size = row_h * 0.42;
        push_swords(
            &mut rects,
            margin + col_w * 0.42,
            ry + row_h * 0.29,
            sword_size,
            DIM,
        );
        let (q, _) = fonts.tiny.layout(
            &boss_name,
            margin + col_w * 0.42 + sword_size + row_h * 0.18,
            ry + row_h * 0.28,
            col_w * 0.4,
        );
        texts.push(TextRun {
            atlas: &fonts.tiny,
            quads: q,
            rgba: DIM,
        });
        // Status text (right).
        let sw = fonts.tiny.text_width(status);
        let (q, _) = fonts.tiny.layout(
            status,
            margin + col_w - sw - row_h * 0.3,
            ry + row_h * 0.28,
            sw + 4.0,
        );
        texts.push(TextRun {
            atlas: &fonts.tiny,
            quads: q,
            rgba: status_col,
        });
    }

    // Current-foe portrait card.
    let card_y = rows_top + nregions as f32 * (row_h + rgap) + h * 0.012;
    let card_h = h * 0.16;
    rects.push(RectRun {
        x: margin,
        y: card_y,
        w: col_w,
        h: card_h,
        rgba: KEYBG,
    });
    let (foe_name, foe_kind, _) = crate::combat::tier_meta(tier)
        .unwrap_or_else(|| ("Foe".to_string(), crate::arena::Kind::Brawn, false));
    let portrait_box = card_h * 0.85;
    let pbx = margin + col_w * 0.06;
    let pby = card_y + (card_h - portrait_box) / 2.0;
    rects.push(RectRun {
        x: pbx,
        y: pby,
        w: portrait_box,
        h: portrait_box,
        rgba: INK,
    });
    let (role, pal) = crate::art::foe_grid(tier, &foe_name, foe_kind);
    paint_role(&mut rects, &role, &pal, pbx, pby, portrait_box / 16.0);
    // Footer text: "<REGION> · REGION N · TIER X/Y" + foe name + type label.
    let info_x = pbx + portrait_box + col_w * 0.04;
    // V48: include the `/10` region denominator (web shows "REGION 3/10", not bare "REGION 3").
    let region_str = format!(
        "{} · REGION {}/{nregions} · TIER {}/{rsize}",
        crate::combat::region_name(cur_region as usize).to_uppercase(),
        cur_region + 1,
        ((tier - 1) % rsize) + 1
    );
    let info_w = col_w * 0.68;
    let (q, _) = fonts
        .tiny
        .layout(&region_str, info_x, card_y + card_h * 0.1, info_w);
    texts.push(TextRun {
        atlas: &fonts.tiny,
        quads: q,
        rgba: DIM,
    });
    let (q, _) = fonts
        .body
        .layout(&foe_name, info_x, card_y + card_h * 0.34, info_w);
    texts.push(TextRun {
        atlas: &fonts.body,
        quads: q,
        rgba: GREENF,
    });
    let type_str = format!("{foe_kind:?}").to_uppercase();
    let (q, _) = fonts
        .tiny
        .layout(&type_str, info_x, card_y + card_h * 0.72, info_w);
    texts.push(TextRun {
        atlas: &fonts.tiny,
        quads: q,
        rgba: DIM,
    });
    // V24: the foe-showcase stat line — the LEAD foe's `pow`+`hp` (the combat redesign replaced the
    // removed `DEF` with PWR·HP), preceded by a tiny crossed-swords mark (the bundled font has no
    // U+2694 glyph, so we draw it as art).
    let foes = crate::combat::tier_foes(tier);
    let stat_str = if let Some((_kind, pow, hp)) = foes.first() {
        format!("{}  PWR · {}  HP", pow.round() as i64, hp.round() as i64)
    } else {
        String::new()
    };
    if !stat_str.is_empty() {
        let tw = fonts.tiny.text_width(&type_str);
        let stat_x = info_x + tw + col_w * 0.06;
        let stat_y = card_y + card_h * 0.72;
        let mark_size = card_h * 0.12;
        // V48: real ⚔ pixel mark (was a checkmark stand-in).
        push_swords(&mut rects, stat_x, stat_y + card_h * 0.06, mark_size, GOLD);
        let (q, _) = fonts.tiny.layout(
            &stat_str,
            stat_x + mark_size + card_h * 0.04,
            stat_y,
            col_w * 0.5,
        );
        texts.push(TextRun {
            atlas: &fonts.tiny,
            quads: q,
            rgba: GOLD,
        });
    }

    // V48: "⚔ ENEMY TEAM" eyebrow — pixel swords + label.
    let foes_y = card_y + card_h + h * 0.012;
    let sw_size = h * 0.018;
    push_swords(&mut rects, margin, foes_y + h * 0.003, sw_size, DIM);
    texts.push(TextRun {
        atlas: &fonts.tiny,
        quads: fonts
            .tiny
            .layout("ENEMY TEAM", margin + sw_size + h * 0.008, foes_y, col_w)
            .0,
        rgba: DIM,
    });
    let _ = party; // party isn't shown here; consumed for signature parity with arena_frame.

    // Bottom buttons: Back (outline) + "Pick your party" (gold).
    let (bw, bh) = (col_w * 0.4, h * 0.06);
    let by = h * 0.93;
    rects.push(RectRun {
        x: margin,
        y: by,
        w: bw,
        h: bh,
        rgba: [GOLD[0], GOLD[1], GOLD[2], 0.5],
    });
    rects.push(RectRun {
        x: margin + 2.0,
        y: by + 2.0,
        w: bw - 4.0,
        h: bh - 4.0,
        rgba: KEYBG,
    });
    texts.push(TextRun {
        atlas: &fonts.body,
        quads: centered(&fonts.body, "Back", margin + bw / 2.0, by, bh),
        rgba: BODY,
    });
    let ptx = margin + col_w - bw;
    rects.push(RectRun {
        x: ptx,
        y: by,
        w: bw,
        h: bh,
        rgba: GOLD,
    });
    // V48: shrink CTA label to fit the bw (web's "Pick your party" overflowed/truncated).
    texts.push(TextRun {
        atlas: &fonts.tiny,
        quads: centered(&fonts.tiny, "Pick your party", ptx + bw / 2.0, by, bh),
        rgba: INK,
    });

    (rects, texts)
}

/// The **Arena CLEARED** screen — once the player defeats tier 120: "ARENA CLEARED!" header +
/// "Journey map" link + a star-rimmed panel ("Arena cleared — you defeated <BOSS>!") + a fading
/// region-scenery silhouette backdrop + Back / Cleared buttons.
pub fn arena_cleared_frame<'a>(
    fonts: &'a Fonts,
    w: f32,
    h: f32,
) -> (Vec<RectRun>, Vec<TextRun<'a>>) {
    let mut rects: Vec<RectRun> = Vec::new();
    let mut texts: Vec<TextRun> = Vec::new();
    let margin = w * 0.04;
    let col_w = w - margin * 2.0;

    // Eyebrow "ARENA CLEARED!".
    texts.push(TextRun {
        atlas: &fonts.tiny,
        quads: centered(&fonts.tiny, "ARENA CLEARED!", w / 2.0, h * 0.018, h * 0.03),
        rgba: GOLD,
    });

    // "Journey map" outlined pill.
    let (jy, jh) = (h * 0.058, h * 0.046);
    rects.push(RectRun {
        x: margin,
        y: jy,
        w: col_w,
        h: jh,
        rgba: [DIM[0], DIM[1], DIM[2], 0.5],
    });
    rects.push(RectRun {
        x: margin + 1.5,
        y: jy + 1.5,
        w: col_w - 3.0,
        h: jh - 3.0,
        rgba: KEYBG,
    });
    texts.push(TextRun {
        atlas: &fonts.tiny,
        quads: centered(&fonts.tiny, "Journey map", w / 2.0, jy, jh),
        rgba: BODY,
    });

    // Star-rimmed "Arena cleared" panel.
    let (py, ph) = (h * 0.13, h * 0.12);
    rects.push(RectRun {
        x: margin,
        y: py,
        w: col_w,
        h: ph,
        rgba: [GOLD[0], GOLD[1], GOLD[2], 0.5],
    });
    rects.push(RectRun {
        x: margin + 1.5,
        y: py + 1.5,
        w: col_w - 3.0,
        h: ph - 3.0,
        rgba: KEYBG,
    });
    // ★ at the left of the title.
    let star_x = margin + col_w * 0.05;
    texts.push(TextRun {
        atlas: &fonts.tiny,
        quads: fonts.tiny.layout("*", star_x, py + ph * 0.18, ph).0,
        rgba: GOLD,
    });
    let final_boss = crate::combat::tier_meta(crate::combat::tier_count())
        .map(|(n, _, _)| n)
        .unwrap_or_else(|| "The Void Sovereign".to_string());
    let title = format!("Arena cleared - you defeated\n{final_boss}!");
    let (q, _) = fonts
        .body
        .layout(&title, star_x + ph * 0.4, py + ph * 0.1, col_w * 0.85);
    texts.push(TextRun {
        atlas: &fonts.body,
        quads: q,
        rgba: BODY,
    });
    let blurb = "EVERY TIER HAS FALLEN. CHAMPION OF THE REALM.";
    let (q, _) = fonts
        .tiny
        .layout(blurb, star_x, py + ph * 0.74, col_w * 0.9);
    texts.push(TextRun {
        atlas: &fonts.tiny,
        quads: q,
        rgba: DIM,
    });

    // Fading scenery backdrop (red-tinted strip) — a stylised flag-of-victory band.
    const FLAG: [f32; 4] = [120.0 / 255.0, 35.0 / 255.0, 35.0 / 255.0, 1.0];
    let band_top = h * 0.46;
    let band_h = h * 0.18;
    for i in 0..6 {
        let t = i as f32 / 5.0;
        let a = 0.85 - t * 0.7;
        rects.push(RectRun {
            x: 0.0,
            y: band_top + t * band_h,
            w,
            h: band_h / 6.0 + 1.0,
            rgba: [FLAG[0], FLAG[1], FLAG[2], a],
        });
    }

    // Bottom Back + Cleared buttons.
    let (bw, bh) = (col_w * 0.4, h * 0.06);
    let by = h * 0.93;
    rects.push(RectRun {
        x: margin,
        y: by,
        w: bw,
        h: bh,
        rgba: [GOLD[0], GOLD[1], GOLD[2], 0.5],
    });
    rects.push(RectRun {
        x: margin + 2.0,
        y: by + 2.0,
        w: bw - 4.0,
        h: bh - 4.0,
        rgba: KEYBG,
    });
    texts.push(TextRun {
        atlas: &fonts.body,
        quads: centered(&fonts.body, "Back", margin + bw / 2.0, by, bh),
        rgba: BODY,
    });
    let cx = margin + col_w - bw;
    rects.push(RectRun {
        x: cx,
        y: by,
        w: bw,
        h: bh,
        rgba: GOLD,
    });
    texts.push(TextRun {
        atlas: &fonts.body,
        quads: centered(&fonts.body, "Cleared", cx + bw / 2.0, by, bh),
        rgba: INK,
    });

    (rects, texts)
}

/// The Arena "Journey map" button rect — sits just above the Fight bar (region-progress view).
fn arena_journey_button(w: f32, h: f32) -> (f32, f32, f32, f32) {
    let bw = w * 0.6;
    let bh = h * 0.042;
    ((w - bw) / 2.0, h * 0.792, bw, bh)
}

/// The Arena "Fight" bar rect (just above the bottom Back button).
fn arena_fight_button(w: f32, h: f32) -> (f32, f32, f32, f32) {
    let bw = w * 0.6;
    let bh = h * 0.05;
    ((w - bw) / 2.0, h * 0.85, bw, bh)
}

/// Whether (`px`,`py`) hits the Arena Fight bar.
fn arena_fight_hit(w: f32, h: f32, px: f32, py: f32) -> bool {
    let (bx, by, bw, bh) = arena_fight_button(w, h);
    px >= bx && px < bx + bw && py >= by && py < by + bh
}

/// The unlocked-hero id whose Arena card contains (`px`,`py`), if any (party-pick routing). Shares
/// [`arena_hero_rows`] with the renderer, so taps land on exactly the painted cards.
fn arena_hero_at(save: &Save, w: f32, h: f32, px: f32, py: f32) -> Option<String> {
    let keys: std::collections::HashSet<&str> = save.collected.keys().map(String::as_str).collect();
    let unlocked = crate::combat::unlocked_roster(&keys);
    arena_hero_rows(unlocked.len(), w, h)
        .into_iter()
        .zip(&unlocked)
        .find(|((rx, ry, rw, rh), _)| px >= *rx && px < rx + rw && py >= *ry && py < ry + rh)
        .map(|(_, id)| id.clone())
}

/// The representative Arena state for the golden / visual-ref (some heroes unlocked, a prior win).
fn arena_sample() -> (Save, Vec<String>, crate::save::ArenaOutcome) {
    (
        sample_save(),
        // bram + tovar are both unlocked under the sample save (init: + mastery:).
        vec!["bram".to_string(), "tovar".to_string()],
        crate::save::ArenaOutcome {
            tier: 3,
            win: true,
            rounds: 4,
            heroes_alive: 2,
            gold_earned: 38,
            loot: vec!["loot:3:0".to_string()],
            region_cleared: false,
        },
    )
}

/// Render the **Arena** screen for a representative save, headless. Shared by the golden blesser +
/// golden test.
pub fn render_arena(painter: &crate::headless::Painter, font: &FontRef<'_>) -> Vec<u8> {
    let (w, h) = (DRILL_W as f32, DRILL_H as f32);
    let fonts = Fonts::bake(font, h);
    let (save, party, _) = arena_sample();
    // The clean PRE-FIGHT state (no post-battle banner) — the party-pick the web ref captures.
    let (rects, texts) = arena_frame(&save, &party, None, &fonts, w, h);
    painter.paint_rgba(DRILL_W, DRILL_H, BG, &rects, &texts)
}

/// The web visual-ref aspect (`content/gg1/visual-ref/*-web.png` are 430×880).
pub const REF_W: u32 = 430;
pub const REF_H: u32 = 880;

/// Render the Arena screen at the **web reference aspect** (430×880) — committed to halves as
/// `visual-ref/arena-prefight-brickmap.png` for the Babysitter's side-by-side perceptual review.
pub fn render_arena_ref(painter: &crate::headless::Painter, font: &FontRef<'_>) -> Vec<u8> {
    let (w, h) = (REF_W as f32, REF_H as f32);
    let fonts = Fonts::bake(font, h);
    let (save, party, _) = arena_sample();
    // The clean PRE-FIGHT state (no post-battle banner) — the party-pick the web ref captures.
    let (rects, texts) = arena_frame(&save, &party, None, &fonts, w, h);
    painter.paint_rgba(REF_W, REF_H, BG, &rects, &texts)
}

/// A contact sheet of every procedural generator (F1 heroes · F2 foes · F3 scenery · F4 crests),
/// painted through the real engine rect path — a golden that proves the art renders correctly on the
/// GPU, before the screens consume the same paint helpers.
pub fn render_art_sheet(painter: &crate::headless::Painter, font: &FontRef<'_>) -> Vec<u8> {
    let (w, h) = (DRILL_W as f32, DRILL_H as f32);
    let fonts = Fonts::bake(font, h);
    let margin = w * 0.04;
    let mut rects: Vec<RectRun> = Vec::new();
    let mut texts: Vec<TextRun> = Vec::new();
    // A labelled section header — pushed inline (a closure capturing `fonts` would over-constrain the
    // atlas borrow to `'static`).
    let label = |s: &str, y: f32| -> TextRun {
        TextRun {
            atlas: &fonts.body,
            quads: fonts.body.layout(s, margin, y, w).0,
            rgba: GOLD,
        }
    };

    // F1 heroes — 12 portraits, 6 per row.
    texts.push(label("Heroes (F1)", h * 0.02));
    let roster = crate::arena::roster();
    let hcell = 4.0;
    for (i, hero) in roster.iter().enumerate() {
        let (role, pal) = crate::art::hero_icon(&hero.id, hero.kind);
        let (col, row) = (i % 6, i / 6);
        let x = margin + col as f32 * (16.0 * hcell + 8.0);
        let y = h * 0.05 + row as f32 * (16.0 * hcell + 8.0);
        paint_role(&mut rects, &role, &pal, x, y, hcell);
    }

    // F2 foes — the first 15 tiers, 5 per row.
    texts.push(label("Foes (F2)", h * 0.20));
    let bestiary = crate::arena::bestiary();
    let fcell = 3.5;
    for (i, foe) in bestiary.iter().take(15).enumerate() {
        let (role, pal) = crate::art::foe_grid(foe.n, &foe.name, foe.kind);
        let (col, row) = (i % 5, i / 5);
        let x = margin + col as f32 * (16.0 * fcell + 8.0);
        let y = h * 0.23 + row as f32 * (16.0 * fcell + 8.0);
        paint_role(&mut rects, &role, &pal, x, y, fcell);
    }

    // F3 scenery — 10 region backdrops, 2 per row.
    texts.push(label("Scenery (F3)", h * 0.42));
    let scw = 3.4;
    for region in 0..10i64 {
        let grid = crate::scenes::scenery_grid(region);
        let (col, row) = (region % 2, region / 2);
        let x = margin + col as f32 * (28.0 * scw + 10.0);
        let y = h * 0.45 + row as f32 * (11.0 * scw + 6.0);
        paint_colors(&mut rects, &grid, x, y, scw, scw);
    }

    // F4 event crests — the first 8 events, 4 per row.
    texts.push(label("Event crests (F4)", h * 0.74));
    let ecw = 3.4;
    for (i, ev) in crate::event_play::roster().iter().take(8).enumerate() {
        let grid = crate::scenes::eventart_grid(ev.art_seed);
        let (col, row) = (i % 4, i / 4);
        let x = margin + col as f32 * (24.0 * ecw + 10.0);
        let y = h * 0.77 + row as f32 * (16.0 * ecw + 6.0);
        paint_colors(&mut rects, &grid, x, y, ecw, ecw);
    }

    painter.paint_rgba(DRILL_W, DRILL_H, BG, &rects, &texts)
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Re-assert immersive-sticky fullscreen on every resume — the system clears the flags when
        // the window loses focus, so a one-shot at startup wouldn't survive an app switch.
        #[cfg(target_os = "android")]
        if let Some(app) = &self.android_app {
            crate::immersive::enable(app);
        }
        if self.gfx.is_some() {
            return; // resumed after suspend (mobile) — surface is rebuilt on Resized
        }
        let attrs = Window::default_attributes().with_title("Goblin Gold");
        let window = Arc::new(event_loop.create_window(attrs).expect("window"));
        let gfx = pollster::block_on(Gfx::new(window.clone()));
        self.window = Some(window);
        self.gfx = Some(gfx);
        self.relayout();
        // Start the audio engine once (it survives suspend/resume); a device-less machine stays
        // silent. Then set the bed for the current screen.
        if self.audio.is_none() {
            self.audio = crate::audio::Player::start();
        }
        self.update_audio_scene();
        if let Some(win) = &self.window {
            win.request_redraw();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(gfx) = self.gfx.as_mut() {
                    gfx.resize(size.width, size.height);
                }
                self.relayout();
                if let Some(win) = &self.window {
                    win.request_redraw();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = (position.x as f32, position.y as f32);
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                let (x, y) = self.cursor;
                self.tap(x, y);
            }
            WindowEvent::Touch(t) if t.phase == TouchPhase::Started => {
                self.tap(t.location.x as f32, t.location.y as f32);
            }
            WindowEvent::RedrawRequested => {
                self.draw();
                // Keep animating while the FX bloom plays; otherwise idle until the next input.
                if matches!(self.fx_start.map(|s| s.elapsed().as_secs_f32()), Some(t) if t < FX_SECS)
                {
                    if let Some(win) = &self.window {
                        win.request_redraw();
                    }
                }
            }
            _ => {}
        }
    }
}

fn init_logging() {
    #[cfg(target_os = "android")]
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Info),
    );
    #[cfg(not(target_os = "android"))]
    let _ = env_logger::try_init();

    // Route panics through the log. The default Rust panic handler writes to **stderr**, which
    // Android does NOT capture — so a `panic → abort` force-close shows no cause in `adb logcat`.
    // This hook logs the payload + location at ERROR (→ logcat via android_logger) BEFORE the
    // default handler aborts, so the exact failing `expect`/`unwrap` is named on the next launch.
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let loc = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "<unknown location>".to_string());
        let msg = info
            .payload()
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| info.payload().downcast_ref::<String>().map(|s| s.as_str()))
            .unwrap_or("<non-string panic payload>");
        log::error!("goblin-gold PANIC at {loc}: {msg}");
        default(info);
    }));
}

/// Desktop entry point: build the event loop and run the drill app.
pub fn run() {
    init_logging();
    let event_loop = EventLoop::builder().build().expect("event loop");
    // Persist the save under a stable per-user dir so desktop runs remember progress too.
    let data_dir = std::env::temp_dir().join("goblin-gold");
    let mut app = App::new(Some(data_dir));
    event_loop.run_app(&mut app).expect("run");
}

/// Android entry point — `android-activity` (via winit) calls this with the `AndroidApp`.
#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(android_app: winit::platform::android::activity::AndroidApp) {
    use winit::platform::android::EventLoopBuilderExtAndroid;
    init_logging();
    // The app's private writable dir on device (where the save lives).
    let data_dir = android_app.internal_data_path();
    let event_loop = EventLoop::builder()
        .with_android_app(android_app.clone())
        .build()
        .expect("event loop");
    let mut app = App::new(data_dir);
    // Keep the handle so `resumed` can marshal the immersive JNI onto the Java UI thread.
    app.android_app = Some(android_app);
    event_loop.run_app(&mut app).expect("run");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The topic grid must keep EVERY unlocked topic fully on-screen (above the Collection button)
    /// and within the horizontal margins — the bug this layout fixes was a fixed row height that ran
    /// the list off the bottom once a player unlocked more than ~8 topics.
    #[test]
    fn topic_grid_fits_on_screen_when_fully_unlocked() {
        let (w, h) = (DRILL_W as f32, DRILL_H as f32);
        let modes = progression::modes();
        let progress = all_unlocked();
        let unlocked: Vec<_> = modes.iter().filter(|m| progress.is_unlocked(m)).collect();
        assert_eq!(unlocked.len(), 46, "all topics should be unlocked");
        let (bx, by, _bw, _bh) = bottom_button(w, h);
        let _ = bx;
        for (rx, ry, rw, rh) in topic_rows(unlocked.len(), w, h) {
            assert!(
                ry >= h * TOPIC_TOP_FRAC - 0.5,
                "row starts above the band: {ry}"
            );
            assert!(
                ry + rh <= by + 0.5,
                "row {ry}+{rh} overruns the button at {by}"
            );
            assert!(
                rx >= 0.0 && rx + rw <= w + 0.5,
                "row out of width: {rx}+{rw} vs {w}"
            );
            assert!(rh > 0.0, "degenerate row height");
        }
    }

    /// Every unlocked topic must be reachable by a tap at its row centre — the renderer and the
    /// hit-test share `topic_rows`, so a multi-column layout can't strand a topic out of reach.
    #[test]
    fn every_unlocked_topic_is_tappable() {
        let (w, h) = (DRILL_W as f32, DRILL_H as f32);
        let modes = progression::modes();
        let progress = all_unlocked();
        let unlocked: Vec<_> = modes.iter().filter(|m| progress.is_unlocked(m)).collect();
        let rows = topic_rows(unlocked.len(), w, h);
        for (m, (rx, ry, rw, rh)) in unlocked.iter().zip(rows) {
            let (cx, cy) = (rx + rw / 2.0, ry + rh / 2.0);
            let hit = topic_at(&modes, &progress, w, h, cx, cy);
            assert_eq!(
                hit.as_deref(),
                Some(m.id.as_str()),
                "tap at row centre missed {}",
                m.id
            );
        }
    }

    /// Every unlocked Arena hero is tappable at its card centre, and the Fight bar is hittable — the
    /// renderer + hit-test share `arena_hero_rows`, so party-pick taps land on the painted cards.
    #[test]
    fn arena_cards_are_tappable() {
        let (w, h) = (DRILL_W as f32, DRILL_H as f32);
        let save = sample_save();
        let keys: std::collections::HashSet<&str> =
            save.collected.keys().map(String::as_str).collect();
        let unlocked = crate::combat::unlocked_roster(&keys);
        assert!(!unlocked.is_empty(), "the sample save unlocks some heroes");
        for ((rx, ry, rw, rh), id) in arena_hero_rows(unlocked.len(), w, h)
            .into_iter()
            .zip(&unlocked)
        {
            let hit = arena_hero_at(&save, w, h, rx + rw / 2.0, ry + rh / 2.0);
            assert_eq!(hit.as_deref(), Some(id.as_str()), "tap missed card {id}");
        }
        let (fbx, fby, fbw, fbh) = arena_fight_button(w, h);
        assert!(
            arena_fight_hit(w, h, fbx + fbw / 2.0, fby + fbh / 2.0),
            "fight bar hittable"
        );
        assert!(
            !arena_fight_hit(w, h, 1.0, 1.0),
            "top-left is not the fight bar"
        );
    }

    /// The event-play screen's sample timestamp puts the cleared event (`bondfire-night`) live, so its
    /// crest/name/tiers all resolve and the "Play" tier renders earned (the green-tier path is hit).
    #[test]
    fn event_play_sample_is_the_cleared_event() {
        let ev = crate::event_play::live_event(EVENT_SAMPLE_NOW_MS);
        assert_eq!(ev.id, "bondfire-night");
        // The sample save owns the participation key for that event.
        assert!(sample_save()
            .collected
            .contains_key(&format!("event:{}", ev.id)));
        // The countdown is a sane, non-empty remaining-time string.
        let cd = event_countdown(EVENT_SAMPLE_NOW_MS);
        assert!(cd.contains('m'), "countdown shows minutes: {cd}");
        // The frame builds without panicking and emits content.
        let (w, h) = (REF_W as f32, REF_H as f32);
        let fonts = Fonts::bake(
            &FontRef::try_from_slice(crate::FONT_JETBRAINS_MONO).unwrap(),
            h,
        );
        let (rects, texts) =
            event_play_frame(&sample_save(), EVENT_SAMPLE_NOW_MS, None, &fonts, w, h);
        assert!(!rects.is_empty() && !texts.is_empty());
    }

    /// The Play CTA is hittable at its centre (and a stray corner tap is not) — the renderer and the
    /// tap-router share `event_play_cta`, so the gauntlet always launches where the button is painted.
    #[test]
    fn event_play_cta_is_tappable() {
        let (w, h) = (DRILL_W as f32, DRILL_H as f32);
        let (bx, by, bw, bh) = event_play_cta(w, h);
        assert!(event_play_cta_hit(w, h, bx + bw / 2.0, by + bh / 2.0));
        assert!(!event_play_cta_hit(w, h, 1.0, 1.0));
    }

    /// Finishing an event gauntlet folds the right tiers into the save (no gold) and builds the
    /// outcome banner — a flawless run earns participation + well + ace; the save is unchanged on gold.
    #[test]
    fn finishing_an_event_awards_tiers_no_gold() {
        let eid = "bondfire-night";
        let g = crate::event_play::build_gauntlet(eid);
        let total = g.len() as u32;
        let mut save = Save::default();
        let gold_before = save.gold;
        // A flawless run: every gauntlet answer solved.
        let newly = save.award_event(eid, total, total, 42);
        assert!(newly.contains(&format!("event:{eid}")));
        assert!(newly.contains(&format!("event:{eid}:well")));
        assert!(newly.contains(&format!("event:{eid}:ace")));
        assert_eq!(save.gold, gold_before, "events pay no gold");
        // The banner classifies the run as flawless.
        let keys = crate::event_play::event_tiers_earned(eid, total, total);
        let o = EventOutcome {
            name: "Bondfire Night".into(),
            score: total,
            total,
            well: keys.iter().any(|k| k.ends_with(":well")),
            ace: keys.iter().any(|k| k.ends_with(":ace")),
        };
        assert_eq!(o.tier_label(), "Flawless!");
    }

    /// The Heroes card list lays out one row per hero, grouped by type — exactly three section-firsts
    /// (one per Brawn/Arcane/Cunning group), and the full-collection sample unlocks every hero so the
    /// roster + boosted effective stats render (matching the web ref's seeded-everything state).
    #[test]
    fn heroes_card_list_groups_by_type() {
        let roster = crate::arena::roster();
        let rows = hero_card_rows(&roster, DRILL_W as f32, DRILL_H as f32);
        assert_eq!(rows.len(), roster.len(), "one card per hero");
        let firsts = rows.iter().filter(|(_, _, first)| *first).count();
        assert_eq!(firsts, 3, "one section header per type group");
        // Every hero is boosted under the full-collection sample (its effective stats lift over base).
        let save = full_collection_sample();
        let keys: Vec<&str> = save.collected.keys().map(String::as_str).collect();
        for hero in &roster {
            assert!(
                boost_count(&hero.id, &keys) > 0,
                "{} should have owned boosts in the full collection",
                hero.id
            );
        }
    }

    /// The Inventory builds for every tab without panicking and resolves real item-icon content (the
    /// N1 generator over the tab's catalogue items), proving the grid + `item_icon_for` wiring.
    #[test]
    fn inventory_builds_for_every_tab() {
        let (w, h) = (REF_W as f32, REF_H as f32);
        let fonts = Fonts::bake(
            &FontRef::try_from_slice(crate::FONT_JETBRAINS_MONO).unwrap(),
            h,
        );
        let save = full_collection_sample();
        for tab in 0..INV_TABS.len() {
            let (rects, texts) = items_frame(&save, tab, EVENT_SAMPLE_NOW_MS, &fonts, w, h);
            assert!(!rects.is_empty() && !texts.is_empty(), "tab {tab} renders");
        }
        let award = crate::catalogue::catalog()
            .into_iter()
            .find(|c| inv_tab_cats(1).contains(&c.cat))
            .unwrap();
        assert!(
            crate::art::item_icon_for(&award.id, &award.rarity).is_some(),
            "award item icon builds"
        );
    }

    /// A full-collection save unlocks every hero; the representative partial save unlocks some but not
    /// all — exercising the locked-hero rows (`?` portrait + unlock hint) without panicking.
    #[test]
    fn heroes_partial_locks_some_full_unlocks_all() {
        let roster = crate::arena::roster();
        let count_unlocked = |save: &Save| {
            let set: std::collections::HashSet<&str> =
                save.collected.keys().map(String::as_str).collect();
            roster
                .iter()
                .filter(|hh| crate::combat::is_hero_unlocked(&hh.id, &set))
                .count()
        };
        assert_eq!(
            count_unlocked(&full_collection_sample()),
            roster.len(),
            "full collection unlocks every hero"
        );
        let partial = sample_save();
        let pu = count_unlocked(&partial);
        assert!(
            pu > 0 && pu < roster.len(),
            "partial save unlocks some but not all heroes (got {pu})"
        );
        let (w, h) = (REF_W as f32, REF_H as f32);
        let fonts = Fonts::bake(
            &FontRef::try_from_slice(crate::FONT_JETBRAINS_MONO).unwrap(),
            h,
        );
        let (rects, texts) = heroes_frame(&partial, &fonts, w, h);
        assert!(!rects.is_empty() && !texts.is_empty());
    }

    /// The single-column early-game layout (and the initial 1-row golden) must be unchanged: the
    /// first row sits at the band top at the comfortable max height.
    #[test]
    fn single_topic_keeps_the_comfortable_layout() {
        let (w, h) = (DRILL_W as f32, DRILL_H as f32);
        let rows = topic_rows(1, w, h);
        assert_eq!(rows.len(), 1);
        let (rx, ry, rw, rh) = rows[0];
        assert!((rx - w * 0.06).abs() < 0.5);
        assert!((ry - h * TOPIC_TOP_FRAC).abs() < 0.5);
        assert!(
            (rw - (w - w * 0.06 * 2.0)).abs() < 0.5,
            "full-width single column"
        );
        assert!(
            (rh - h * TOPIC_MAX_ROW_FRAC).abs() < 0.5,
            "comfortable max height"
        );
    }
}
