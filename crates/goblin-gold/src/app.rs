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

impl Fonts {
    pub fn bake(font: &FontRef<'_>, h: f32) -> Fonts {
        Fonts {
            head: Atlas::bake(font, (h * 0.045).clamp(20.0, 120.0)),
            q: Atlas::bake(font, (h * 0.044).clamp(20.0, 120.0)),
            body: Atlas::bake(font, (h * 0.030).clamp(16.0, 80.0)),
            key: Atlas::bake_chars(font, (h * 0.034).clamp(16.0, 90.0), "✓⌫"),
            // The build-watermark face — small, just digits/hex/punctuation.
            tiny: Atlas::bake(font, (h * 0.018).clamp(11.0, 40.0)),
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
    /// The hero roster detail (effective stats).
    Heroes,
    /// The daily-events detail.
    Events,
    /// The items-by-category detail.
    Items,
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
    keypad: Keypad,
    cursor: (f32, f32),
    fx_start: Option<Instant>,
    fx_seed: u32,
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
            keypad: Keypad::layout(0.0, 0.0, 1.0, 1.0, 0.0),
            cursor: (0.0, 0.0),
            fx_start: None,
            fx_seed: 0x6c1d_9e37,
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
        let kp_h = h * 0.46;
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
    }

    /// Fold the finished round into progression (initiation + mastery) and return to the topic
    /// list — where any newly-unlocked topics now appear.
    fn finish_round(&mut self) {
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
                    // Rows: 0 Items · 1 Collector · 2 Topics · 3 Heroes · 4 Events · 5 Gold.
                    self.screen = match row {
                        0 => Screen::Items,
                        1 => Screen::Ladder,
                        2 => Screen::Select, // topics live on the topic-select screen
                        3 => Screen::Heroes,
                        4 => Screen::Events,
                        _ => Screen::Collection, // Gold has no detail (yet)
                    };
                }
            }
            // The drill-down screens return to the Collection via their Back button.
            Screen::Ladder | Screen::Heroes | Screen::Events | Screen::Items => {
                if hit_bottom_button(w, h, x, y) {
                    self.screen = Screen::Collection;
                }
            }
            Screen::Results => {
                // Tap anywhere to continue back to the topic list.
                self.screen = Screen::Select;
            }
            Screen::Drill => {
                let mut done = false;
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
                                // …and fire the celebration.
                                self.fx_start = Some(Instant::now());
                                self.fx_seed ^= d.solved().wrapping_mul(2_654_435_761);
                            } else {
                                self.round_steps.push(RoundStep::Skip);
                            }
                            self.q_start = Some(Instant::now());
                        }
                    }
                    done = d.is_complete();
                }
                if done {
                    self.finish_round();
                }
            }
        }
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
            Screen::Heroes => {
                let (r, t) = heroes_frame(&self.save, fonts, w, h);
                (r, t, None)
            }
            Screen::Events => {
                let (r, t) = events_frame(&self.save, fonts, w, h);
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

    // Keypad.
    let back_label = if fonts.key.glyphs.contains_key(&'⌫') {
        "⌫"
    } else {
        "<"
    };
    let mut key_quads: Vec<Quad> = Vec::new();
    for cell in &keypad.cells {
        let is_enter = cell.key == Key::Enter;
        rects.push(RectRun {
            x: cell.x,
            y: cell.y,
            w: cell.w,
            h: cell.h,
            rgba: if is_enter { GOLD } else { KEYBG },
        });
        if is_enter {
            // The action bar is SKIP (GG1 has no submit key — answers auto-accept).
            let qd = centered(&fonts.key, "Skip", cell.x + cell.w / 2.0, cell.y, cell.h);
            texts.push(TextRun {
                atlas: &fonts.key,
                quads: qd,
                rgba: INK,
            });
            continue;
        }
        let s = match cell.key {
            Key::Digit(d) => ((b'0' + d) as char).to_string(),
            Key::Dot => ".".to_string(),
            Key::Back => back_label.to_string(),
            Key::Enter => unreachable!(),
        };
        key_quads.extend(centered(
            &fonts.key,
            &s,
            cell.x + cell.w / 2.0,
            cell.y,
            cell.h,
        ));
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
    let kp_h = h * 0.46;
    let kp_y = h - kp_h - margin;
    let keypad = Keypad::layout(margin, kp_y, kp_w, kp_h, w * 0.018);
    let (rects, texts) = drill_frame(&drill, &keypad, &fonts, w, h, None);
    painter.paint_rgba(DRILL_W, DRILL_H, BG, &rects, &texts)
}

// ── topic-select screen (phase 3: drive all 46 topics, progression-gated) ─────────────────────

/// Row rects for `count` topic-select rows, sized to `w`×`h` (top-down list).
fn topic_rows(count: usize, w: f32, h: f32) -> Vec<(f32, f32, f32, f32)> {
    let margin = w * 0.06;
    let row_h = h * 0.075;
    let gap = h * 0.014;
    let top = h * 0.17;
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
        let (q, _hh) = fonts.q.layout(
            &m.name,
            rx + rw * 0.06,
            ry + rh / 2.0 - 0.59 * fonts.q.px,
            rw,
        );
        texts.push(TextRun {
            atlas: &fonts.q,
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
    let heroes = crate::arena::roster().len();

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
        ("Heroes".to_string(), format!("{heroes} in the roster")),
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

/// The **Heroes** roster drill-down: each hero with its type and effective stats (base + the boosts
/// of everything collected — the catalogue→Arena bridge).
pub fn heroes_frame<'a>(
    save: &Save,
    fonts: &'a Fonts,
    w: f32,
    h: f32,
) -> (Vec<RectRun>, Vec<TextRun<'a>>) {
    let keys: Vec<&str> = save.collected.keys().map(String::as_str).collect();
    let rows: Vec<(String, String, [f32; 4])> = crate::arena::roster()
        .into_iter()
        .map(|hero| {
            let s = crate::arena::hero_stats(&hero.id, keys.iter().copied()).unwrap_or(hero.base);
            (
                format!("{} ({:?})", hero.name, hero.kind),
                format!("P{} G{} S{} F{}", s.power, s.guard, s.speed, s.focus),
                GOLD,
            )
        })
        .collect();
    list_screen("Heroes", "the Arena roster", &rows, fonts, w, h)
}

/// The **Events** drill-down: the 14 daily events, those with a reward earned shown in green.
pub fn events_frame<'a>(
    save: &Save,
    fonts: &'a Fonts,
    w: f32,
    h: f32,
) -> (Vec<RectRun>, Vec<TextRun<'a>>) {
    let keys: Vec<&str> = save.collected.keys().map(String::as_str).collect();
    let touched: std::collections::HashSet<String> = crate::events::touched(keys.iter().copied())
        .into_iter()
        .map(|e| e.id)
        .collect();
    let evs = crate::events::events();
    let rows: Vec<(String, String, [f32; 4])> = evs
        .iter()
        .map(|e| {
            let got = touched.contains(&e.id);
            (
                e.name.clone(),
                if got {
                    "earned".into()
                } else {
                    "locked".into()
                },
                if got { GREEN } else { DIM },
            )
        })
        .collect();
    let sub = format!("{} / {} earned", touched.len(), evs.len());
    list_screen("Daily Events", &sub, &rows, fonts, w, h)
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
pub fn render_heroes(painter: &crate::headless::Painter, font: &FontRef<'_>) -> Vec<u8> {
    let (w, h) = (DRILL_W as f32, DRILL_H as f32);
    let fonts = Fonts::bake(font, h);
    let (rects, texts) = heroes_frame(&sample_save(), &fonts, w, h);
    painter.paint_rgba(DRILL_W, DRILL_H, BG, &rects, &texts)
}
pub fn render_events(painter: &crate::headless::Painter, font: &FontRef<'_>) -> Vec<u8> {
    let (w, h) = (DRILL_W as f32, DRILL_H as f32);
    let fonts = Fonts::bake(font, h);
    let (rects, texts) = events_frame(&sample_save(), &fonts, w, h);
    painter.paint_rgba(DRILL_W, DRILL_H, BG, &rects, &texts)
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
