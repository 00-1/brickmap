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

const BG: [f32; 3] = [20.0 / 255.0, 12.0 / 255.0, 34.0 / 255.0];
const PANEL: [f32; 4] = [34.0 / 255.0, 22.0 / 255.0, 54.0 / 255.0, 1.0];
const KEYBG: [f32; 4] = [46.0 / 255.0, 32.0 / 255.0, 72.0 / 255.0, 1.0];
const INK: [f32; 4] = [16.0 / 255.0, 10.0 / 255.0, 28.0 / 255.0, 1.0];
const GOLD: [f32; 4] = [1.0, 214.0 / 255.0, 110.0 / 255.0, 1.0];
const BODY: [f32; 4] = [232.0 / 255.0, 228.0 / 255.0, 244.0 / 255.0, 1.0];
const DIM: [f32; 4] = [150.0 / 255.0, 140.0 / 255.0, 172.0 / 255.0, 1.0];
const GREEN: [f32; 4] = [120.0 / 255.0, 222.0 / 255.0, 142.0 / 255.0, 1.0];
/// Low-contrast build-watermark ink (a hair above the deep-violet background).
const WATERMARK: [f32; 4] = [78.0 / 255.0, 66.0 / 255.0, 104.0 / 255.0, 1.0];

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
const GLYPHS: &str = "−×÷²·—–";

impl Fonts {
    pub fn bake(font: &FontRef<'_>, h: f32) -> Fonts {
        Fonts {
            head: Atlas::bake_chars(font, (h * 0.045).clamp(20.0, 120.0), GLYPHS),
            q: Atlas::bake_chars(font, (h * 0.044).clamp(20.0, 120.0), GLYPHS),
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
        let font = FontRef::try_from_slice(crate::FONT_INSTRUMENT_SANS).expect("font");
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
                    Some(o) => results_frame(o, fonts, w, h),
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
                let (r, t) = items_frame(&self.save, fonts, w, h);
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
                let (r, t) = drill_frame(d, &self.keypad, fonts, w, h, fx);
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
) -> (Vec<RectRun>, Vec<TextRun<'a>>) {
    let cx = w / 2.0;
    let margin = w * 0.06;
    let col_w = w - margin * 2.0;
    let mut rects: Vec<RectRun> = Vec::new();
    let mut texts: Vec<TextRun> = Vec::new();

    // Heading + progress.
    let (q, _hh) = fonts.head.layout(&drill.name, margin, h * 0.035, col_w);
    texts.push(TextRun {
        atlas: &fonts.head,
        quads: q,
        rgba: GOLD,
    });
    let prog = format!("{} / {}", drill.solved(), drill.len());
    let pw = fonts.body.text_width(&prog);
    let (q, _hh) = fonts
        .body
        .layout(&prog, w - margin - pw, h * 0.05, pw + 4.0);
    texts.push(TextRun {
        atlas: &fonts.body,
        quads: q,
        rgba: DIM,
    });

    // Question card.
    let cy = h * 0.16;
    let ch = h * 0.16;
    rects.push(RectRun {
        x: margin,
        y: cy,
        w: col_w,
        h: ch,
        rgba: [GOLD[0], GOLD[1], GOLD[2], 0.5],
    });
    rects.push(RectRun {
        x: margin + 3.0,
        y: cy + 3.0,
        w: col_w - 6.0,
        h: ch - 6.0,
        rgba: PANEL,
    });
    // The question is the transform's prompt verbatim (e.g. "100", "3 × 7", "area 10×7"); the topic
    // name in the heading gives it context. Generic across all 46 topics (no per-topic framing).
    texts.push(TextRun {
        atlas: &fonts.q,
        quads: centered(&fonts.q, drill.prompt(), cx, cy, ch),
        rgba: GOLD,
    });

    // Answer box — frame colour reflects the last verdict; the value is the typed string (which is
    // EMPTY on the first frame → an empty text run, exercised by the renderer's empty-run guard).
    let (frame_col, ink) = match drill.last_mark() {
        Some(Mark::Right) => (GREEN, GREEN),
        Some(Mark::Skipped) => (DIM, DIM),
        None => (DIM, BODY),
    };
    // On a skip the box shows the revealed answer; otherwise the typed string.
    let box_text = drill.revealed().unwrap_or_else(|| drill.typed());
    let by = h * 0.35;
    let bh = h * 0.085;
    let bw = col_w * 0.7;
    let bx = cx - bw / 2.0;
    rects.push(RectRun {
        x: bx,
        y: by,
        w: bw,
        h: bh,
        rgba: frame_col,
    });
    rects.push(RectRun {
        x: bx + 3.0,
        y: by + 3.0,
        w: bw - 6.0,
        h: bh - 6.0,
        rgba: [28.0 / 255.0, 18.0 / 255.0, 44.0 / 255.0, 1.0],
    });
    texts.push(TextRun {
        atlas: &fonts.q,
        quads: centered(&fonts.q, box_text, cx, by, bh),
        rgba: ink,
    });

    // Verdict banner. There's no wrong state — the answer auto-checks as you type; the action bar
    // skips (revealing the answer).
    let (msg, col) = match drill.last_mark() {
        Some(Mark::Right) => ("Correct!", GREEN),
        Some(Mark::Skipped) => ("Skipped", DIM),
        None => ("Tap the digits — it checks itself", DIM),
    };
    let mw = fonts.body.text_width(msg);
    let (q, _hh) = fonts.body.layout(msg, cx - mw / 2.0, h * 0.46, mw + 4.0);
    texts.push(TextRun {
        atlas: &fonts.body,
        quads: q,
        rgba: col,
    });

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
    rects.push(RectRun {
        x: kp_left,
        y: hint_y,
        w: kp_right - kp_left,
        h: hint_h,
        rgba: [GOLD[0], GOLD[1], GOLD[2], 0.45],
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
        rgba: GOLD,
    });

    let mut key_quads: Vec<Quad> = Vec::new();
    for cell in &keypad.cells {
        match cell.key {
            Key::Enter => {
                // Outlined Skip bar (gold border + key-coloured fill) — the subtle action bar.
                rects.push(RectRun {
                    x: cell.x,
                    y: cell.y,
                    w: cell.w,
                    h: cell.h,
                    rgba: [GOLD[0], GOLD[1], GOLD[2], 0.55],
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
                    quads: centered(&fonts.key, "Skip", cell.x + cell.w / 2.0, cell.y, cell.h),
                    rgba: GOLD,
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
    let (rects, texts) = drill_frame(&drill, &keypad, &fonts, w, h, None);
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

/// Draw the shared bottom button (a gold bar with a dark inked label).
fn push_button<'a>(
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

    let (q, _hh) = fonts.head.layout("Collection", margin, h * 0.05, col_w);
    texts.push(TextRun {
        atlas: &fonts.head,
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

    push_button(&mut rects, &mut texts, fonts, "Back", w, h);
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

    push_button(&mut rects, &mut texts, fonts, "Back", w, h);
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
    fonts: &'a Fonts,
    w: f32,
    h: f32,
) -> (Vec<RectRun>, Vec<TextRun<'a>>) {
    let mut rects: Vec<RectRun> = Vec::new();
    let mut texts: Vec<TextRun> = Vec::new();
    let margin = w * 0.06;
    let col_w = w - margin * 2.0;
    let cx = w / 2.0;

    let (q, _hh) = fonts.head.layout("Round Complete", margin, h * 0.05, col_w);
    texts.push(TextRun {
        atlas: &fonts.head,
        quads: q,
        rgba: GOLD,
    });

    // The rank, large and centred — the headline reward of the run.
    texts.push(TextRun {
        atlas: &fonts.q,
        quads: centered(&fonts.q, &o.rank_name, cx, h * 0.15, h * 0.1),
        rgba: GREEN,
    });

    let accuracy = if o.total > 0 {
        (o.answered * 100 + o.total / 2) / o.total
    } else {
        0
    };
    let rows = [
        (
            "Answered".to_string(),
            format!("{} / {}", o.answered, o.total),
        ),
        ("Accuracy".to_string(), format!("{accuracy}%")),
        ("Time".to_string(), format!("{:.1}s", o.total_time)),
        ("Gold earned".to_string(), format!("+{}", o.gold_earned)),
        ("New collectibles".to_string(), format!("{}", o.newly.len())),
    ];
    let top = h * 0.30;
    let row_h = h * 0.085;
    let gap = h * 0.02;
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
        // Gold is the headline figure — tint it gold; the rest read as body text.
        let col = if i == 3 { GOLD } else { BODY };
        let (vq, _) = fonts
            .body
            .layout(value, margin + col_w * 0.95 - vw, ty, vw + 4.0);
        texts.push(TextRun {
            atlas: &fonts.body,
            quads: vq,
            rgba: col,
        });
    }

    push_button(&mut rects, &mut texts, fonts, "Continue", w, h);
    (rects, texts)
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
    let (rects, texts) = results_frame(&outcome, &fonts, w, h);
    painter.paint_rgba(DRILL_W, DRILL_H, BG, &rects, &texts)
}

/// A generic scrolling-free **list** screen — heading + subtitle + a panel row per `(left, right,
/// colour)` (auto-sized to fit) + a Back button. Shared by the Heroes/Events/Items drill-downs.
fn list_screen<'a>(
    title: &str,
    subtitle: &str,
    rows: &[(String, String, [f32; 4])],
    fonts: &'a Fonts,
    w: f32,
    h: f32,
) -> (Vec<RectRun>, Vec<TextRun<'a>>) {
    let mut rects: Vec<RectRun> = Vec::new();
    let mut texts: Vec<TextRun> = Vec::new();
    let margin = w * 0.06;
    let col_w = w - margin * 2.0;

    let (q, _hh) = fonts.head.layout(title, margin, h * 0.04, col_w);
    texts.push(TextRun {
        atlas: &fonts.head,
        quads: q,
        rgba: GOLD,
    });
    if !subtitle.is_empty() {
        let (q, _hh) = fonts.body.layout(subtitle, margin, h * 0.105, col_w);
        texts.push(TextRun {
            atlas: &fonts.body,
            quads: q,
            rgba: DIM,
        });
    }

    let n = rows.len().max(1);
    let (area_top, area_bot, gap) = (h * 0.155, h * 0.895, h * 0.008);
    let row_h = ((area_bot - area_top) / n as f32 - gap).clamp(h * 0.022, h * 0.06);
    // A denser list drops to the small face so the labels still fit.
    let font = if row_h < h * 0.038 {
        &fonts.tiny
    } else {
        &fonts.body
    };
    for (i, (left, right, color)) in rows.iter().enumerate() {
        let ry = area_top + i as f32 * (row_h + gap);
        rects.push(RectRun {
            x: margin,
            y: ry,
            w: col_w,
            h: row_h,
            rgba: PANEL,
        });
        let ty = ry + row_h / 2.0 - 0.59 * font.px;
        let (lq, _) = font.layout(left, margin + col_w * 0.05, ty, col_w);
        texts.push(TextRun {
            atlas: font,
            quads: lq,
            rgba: *color,
        });
        let vw = font.text_width(right);
        let (vq, _) = font.layout(right, margin + col_w * 0.95 - vw, ty, vw + 4.0);
        texts.push(TextRun {
            atlas: font,
            quads: vq,
            rgba: BODY,
        });
    }

    push_button(&mut rects, &mut texts, fonts, "Back", w, h);
    (rects, texts)
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
    let roster = crate::arena::roster();

    // Centred header "Heroes  N / 12".
    let title = format!("Heroes  {} / {}", roster.len(), roster.len());
    let tw = fonts.head.text_width(&title);
    texts.push(TextRun {
        atlas: &fonts.head,
        quads: centered(&fonts.head, &title, w / 2.0, h * 0.03, h * 0.05),
        rgba: GOLD,
    });
    let _ = tw;

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
        // Portrait (F1), boxed on a darker tile to read like the web's 48×48.
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
        let (role, pal) = crate::art::hero_icon(&hero.id, kind);
        paint_role(&mut rects, &role, &pal, tx0, ty0, tile / 16.0);

        let stats = crate::combat::effective_stats(&hero.id, &keys.iter().copied().collect())
            .unwrap_or(hero.base);
        let text_x = tx0 + tile + w * 0.03;
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
        // ★ rating (right).
        let rating = format!("* {}", hero_rating(&stats));
        let rtw = fonts.body.text_width(&rating);
        let (q, _) =
            fonts
                .body
                .layout(&rating, rx + rw - rtw - w * 0.02, ry + rh * 0.12, rtw + 4.0);
        texts.push(TextRun {
            atlas: &fonts.body,
            quads: q,
            rgba: GOLD,
        });
        // Effective stat chips.
        let chips = format!(
            "{} PWR   {} GRD   {} SPD   {} FOC",
            stats.power, stats.guard, stats.speed, stats.focus
        );
        let (q, _) = fonts.tiny.layout(&chips, text_x, ry + rh * 0.44, rw - tile);
        texts.push(TextRun {
            atlas: &fonts.tiny,
            quads: q,
            rgba: BODY,
        });
        // Boost line.
        let n = boost_count(&hero.id, &keys);
        let boost = if n > 0 {
            format!("Boosted by {n} · tap for details")
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

    push_button(&mut rects, &mut texts, fonts, "Back", w, h);
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
    push_button(&mut rects, &mut texts, fonts, "Back", w, h);
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
pub fn items_frame<'a>(
    save: &Save,
    fonts: &'a Fonts,
    w: f32,
    h: f32,
) -> (Vec<RectRun>, Vec<TextRun<'a>>) {
    use std::collections::BTreeMap;
    let owned: std::collections::HashSet<String> = save.collected.keys().cloned().collect();
    // Count owned vs total per category in one pass over the catalogue.
    let mut total: BTreeMap<String, u32> = BTreeMap::new();
    let mut have: BTreeMap<String, u32> = BTreeMap::new();
    for c in crate::catalogue::catalog() {
        let cat = format!("{:?}", c.cat);
        *total.entry(cat.clone()).or_default() += 1;
        if owned.contains(&c.id) {
            *have.entry(cat).or_default() += 1;
        }
    }
    let rows: Vec<(String, String, [f32; 4])> = total
        .iter()
        .map(|(cat, &t)| {
            let h_ = *have.get(cat).unwrap_or(&0);
            (
                cat.clone(),
                format!("{h_} / {t}"),
                if h_ > 0 { GOLD } else { DIM },
            )
        })
        .collect();
    let owned_total: u32 = have.values().sum();
    let sub = format!("{} / {} collected", owned_total, crate::catalogue::total());
    list_screen("Items", &sub, &rows, fonts, w, h)
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
    let (rects, texts) = drill_frame(&drill, &keypad, &fonts, w, h, None);
    painter.paint_rgba(REF_W, REF_H, BG, &rects, &texts)
}
pub fn render_items(painter: &crate::headless::Painter, font: &FontRef<'_>) -> Vec<u8> {
    let (w, h) = (DRILL_W as f32, DRILL_H as f32);
    let fonts = Fonts::bake(font, h);
    let (rects, texts) = items_frame(&sample_save(), &fonts, w, h);
    painter.paint_rgba(DRILL_W, DRILL_H, BG, &rects, &texts)
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
    let (q, _) = fonts.head.layout("Arena", margin, h * 0.028, col_w);
    texts.push(TextRun {
        atlas: &fonts.head,
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
        let rating = format!("* {}", hero_rating(&stats));
        let rw_txt = fonts.body.text_width(&rating);
        let (q, _) = fonts.body.layout(
            &rating,
            rx + rw - rw_txt - w * 0.02,
            ry + rh * 0.16,
            rw_txt + 4.0,
        );
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

    // Fight bar (gold when a party is picked, dim otherwise) + Back button.
    let (fbx, fby, fbw, fbh) = arena_fight_button(w, h);
    rects.push(RectRun {
        x: fbx,
        y: fby,
        w: fbw,
        h: fbh,
        rgba: if party.is_empty() { KEYBG } else { GOLD },
    });
    let flabel = if party.is_empty() {
        "Pick your party".to_string()
    } else {
        format!("Fight Tier {tier}")
    };
    texts.push(TextRun {
        atlas: &fonts.key,
        quads: centered(&fonts.key, &flabel, fbx + fbw / 2.0, fby, fbh),
        rgba: if party.is_empty() { DIM } else { INK },
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
    push_button(&mut rects, &mut texts, fonts, "Back", w, h);
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
            &FontRef::try_from_slice(crate::FONT_INSTRUMENT_SANS).unwrap(),
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
