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
use crate::keypad::{Key, Keypad};
use crate::text::{Atlas, Quad};

const SHADER: &str = r#"
struct VsIn { @location(0) pos: vec2<f32>, @location(1) uv: vec2<f32>, @location(2) rgba: vec4<f32> };
struct VsOut { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32>, @location(1) rgba: vec4<f32> };
@vertex fn vs(in: VsIn) -> VsOut {
    var o: VsOut; o.pos = vec4<f32>(in.pos, 0.0, 1.0); o.uv = in.uv; o.rgba = in.rgba; return o;
}
@group(0) @binding(0) var cov: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;
@fragment fn fs(in: VsOut) -> @location(0) vec4<f32> {
    let a = textureSample(cov, samp, in.uv).r;
    return vec4<f32>(in.rgba.rgb, in.rgba.a * a);
}
"#;

const BG: [f32; 3] = [20.0 / 255.0, 12.0 / 255.0, 34.0 / 255.0];
const PANEL: [f32; 4] = [34.0 / 255.0, 22.0 / 255.0, 54.0 / 255.0, 1.0];
const KEYBG: [f32; 4] = [46.0 / 255.0, 32.0 / 255.0, 72.0 / 255.0, 1.0];
const INK: [f32; 4] = [16.0 / 255.0, 10.0 / 255.0, 28.0 / 255.0, 1.0];
const GOLD: [f32; 4] = [1.0, 214.0 / 255.0, 110.0 / 255.0, 1.0];
const BODY: [f32; 4] = [232.0 / 255.0, 228.0 / 255.0, 244.0 / 255.0, 1.0];
const DIM: [f32; 4] = [150.0 / 255.0, 140.0 / 255.0, 172.0 / 255.0, 1.0];
const GREEN: [f32; 4] = [120.0 / 255.0, 222.0 / 255.0, 142.0 / 255.0, 1.0];
const RED: [f32; 4] = [240.0 / 255.0, 116.0 / 255.0, 116.0 / 255.0, 1.0];

/// How long the FX bloom plays after a correct answer.
const FX_SECS: f32 = 0.9;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    pos: [f32; 2],
    uv: [f32; 2],
    rgba: [f32; 4],
}

/// A filled rect (pixel coords, top-left origin).
#[derive(Clone, Copy)]
struct Rect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    rgba: [f32; 4],
}

/// A run of glyph quads sharing one atlas + colour.
struct Text<'a> {
    atlas: &'a Atlas,
    quads: Vec<Quad>,
    rgba: [f32; 4],
}

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
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: w,
            height: h,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: caps.alpha_modes[0],
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
        rects: &[Rect],
        texts: &[Text<'_>],
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

/// Baked atlases sized to the current window (re-baked on resize).
struct Fonts {
    head: Atlas,
    q: Atlas,
    body: Atlas,
    key: Atlas,
}

impl Fonts {
    fn bake(font: &FontRef<'_>, h: f32) -> Fonts {
        Fonts {
            head: Atlas::bake(font, (h * 0.045).clamp(20.0, 120.0)),
            q: Atlas::bake(font, (h * 0.044).clamp(20.0, 120.0)),
            body: Atlas::bake(font, (h * 0.030).clamp(16.0, 80.0)),
            key: Atlas::bake_chars(font, (h * 0.034).clamp(16.0, 90.0), "✓⌫"),
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

/// The live app: drill state + keypad + fonts + the FX timer, driving [`Gfx`].
struct App {
    font: FontRef<'static>,
    gfx: Option<Gfx>,
    window: Option<Arc<Window>>,
    fonts: Option<Fonts>,
    drill: Drill,
    keypad: Keypad,
    cursor: (f32, f32),
    fx_start: Option<Instant>,
    fx_seed: u32,
}

impl App {
    fn new() -> App {
        let font = FontRef::try_from_slice(crate::FONT_INSTRUMENT_SANS).expect("font");
        App {
            font,
            gfx: None,
            window: None,
            fonts: None,
            drill: Drill::from_seam("halves"),
            keypad: Keypad::layout(0.0, 0.0, 1.0, 1.0, 0.0),
            cursor: (0.0, 0.0),
            fx_start: None,
            fx_seed: 0x6c1d_9e37,
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

    fn tap(&mut self, x: f32, y: f32) {
        if let Some(key) = self.keypad.hit(x, y) {
            self.drill.press(key);
            if key == Key::Enter && self.drill.last_mark() == Some(Mark::Right) {
                self.fx_start = Some(Instant::now());
                self.fx_seed ^= self.drill.solved().wrapping_mul(2_654_435_761);
            }
        }
        if let Some(win) = &self.window {
            win.request_redraw();
        }
    }

    fn draw(&mut self) {
        let Some(gfx) = self.gfx.as_mut() else { return };
        let Some(fonts) = self.fonts.as_ref() else {
            return;
        };
        let (w, h) = (gfx.config.width as f32, gfx.config.height as f32);
        let cx = w / 2.0;
        let margin = w * 0.06;
        let col_w = w - margin * 2.0;

        // FX window state.
        let fx_t = self.fx_start.map(|s| s.elapsed().as_secs_f32());
        let fx_active = matches!(fx_t, Some(t) if t < FX_SECS);

        let mut rects: Vec<Rect> = Vec::new();
        let mut texts: Vec<Text> = Vec::new();

        // Heading + progress.
        let (q, _hh) = fonts
            .head
            .layout(&self.drill.name, margin, h * 0.035, col_w);
        texts.push(Text {
            atlas: &fonts.head,
            quads: q,
            rgba: GOLD,
        });
        let prog = format!("{} / {}", self.drill.solved(), self.drill.len());
        let pw = fonts.body.text_width(&prog);
        let (q, _hh) = fonts
            .body
            .layout(&prog, w - margin - pw, h * 0.05, pw + 4.0);
        texts.push(Text {
            atlas: &fonts.body,
            quads: q,
            rgba: DIM,
        });

        // Question card.
        let cy = h * 0.16;
        let ch = h * 0.16;
        rects.push(Rect {
            x: margin,
            y: cy,
            w: col_w,
            h: ch,
            rgba: [GOLD[0], GOLD[1], GOLD[2], 0.5],
        });
        rects.push(Rect {
            x: margin + 3.0,
            y: cy + 3.0,
            w: col_w - 6.0,
            h: ch - 6.0,
            rgba: PANEL,
        });
        let label = format!("Half of {}", self.drill.prompt());
        texts.push(Text {
            atlas: &fonts.q,
            quads: centered(&fonts.q, &label, cx, cy, ch),
            rgba: GOLD,
        });

        // Answer box — frame colour reflects the last verdict.
        let (frame_col, ink) = match self.drill.last_mark() {
            Some(Mark::Right) => (GREEN, GREEN),
            Some(Mark::Wrong) => (RED, RED),
            None => (DIM, BODY),
        };
        let by = h * 0.35;
        let bh = h * 0.085;
        let bw = col_w * 0.7;
        let bx = cx - bw / 2.0;
        rects.push(Rect {
            x: bx,
            y: by,
            w: bw,
            h: bh,
            rgba: frame_col,
        });
        rects.push(Rect {
            x: bx + 3.0,
            y: by + 3.0,
            w: bw - 6.0,
            h: bh - 6.0,
            rgba: [28.0 / 255.0, 18.0 / 255.0, 44.0 / 255.0, 1.0],
        });
        let shown = if self.drill.typed().is_empty() {
            "·".to_string()
        } else {
            self.drill.typed().to_string()
        };
        texts.push(Text {
            atlas: &fonts.q,
            quads: centered(&fonts.q, &shown, cx, by, bh),
            rgba: ink,
        });

        // Verdict banner.
        let (msg, col) = match self.drill.last_mark() {
            Some(Mark::Right) => ("Correct!", GREEN),
            Some(Mark::Wrong) => ("Try again", RED),
            None => ("Tap the digits, then Enter", DIM),
        };
        let mw = fonts.body.text_width(msg);
        let (q, _hh) = fonts.body.layout(msg, cx - mw / 2.0, h * 0.46, mw + 4.0);
        texts.push(Text {
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
        for cell in &self.keypad.cells {
            let is_enter = cell.key == Key::Enter;
            rects.push(Rect {
                x: cell.x,
                y: cell.y,
                w: cell.w,
                h: cell.h,
                rgba: if is_enter { GOLD } else { KEYBG },
            });
            if is_enter {
                let qd = centered(&fonts.key, "Enter", cell.x + cell.w / 2.0, cell.y, cell.h);
                texts.push(Text {
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
        texts.push(Text {
            atlas: &fonts.key,
            quads: key_quads,
            rgba: BODY,
        });

        // FX: an animated gold spark burst over the answer (engine particle system).
        if let Some(t) = fx_t {
            if fx_active {
                let steps = (t * 120.0) as u32 + 4;
                for s in crate::fx::celebrate_steps(
                    cx,
                    by + bh * 0.4,
                    w * 0.05,
                    w * 0.012,
                    self.fx_seed,
                    steps,
                ) {
                    rects.push(Rect {
                        x: s.x - s.size / 2.0,
                        y: s.y - s.size / 2.0,
                        w: s.size,
                        h: s.size,
                        rgba: s.rgba,
                    });
                }
            }
        }

        let fx_ramp = fx_active.then_some((
            &crate::fx::GOLD_RAMP[..],
            crate::fx::GOLD_RAMP.len() as u32,
            crate::fx::FX_DITHER,
        ));
        // Borrow split: `texts` borrows `self.fonts`, `gfx` is a separate field.
        let gfx = self.gfx.as_mut().unwrap();
        gfx.render(&rects, &texts, fx_ramp);
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
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
}

/// Desktop entry point: build the event loop and run the drill app.
pub fn run() {
    init_logging();
    let event_loop = EventLoop::builder().build().expect("event loop");
    let mut app = App::new();
    event_loop.run_app(&mut app).expect("run");
}

/// Android entry point — `android-activity` (via winit) calls this with the `AndroidApp`.
#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(android_app: winit::platform::android::activity::AndroidApp) {
    use winit::platform::android::EventLoopBuilderExtAndroid;
    init_logging();
    let event_loop = EventLoop::builder()
        .with_android_app(android_app)
        .build()
        .expect("event loop");
    let mut app = App::new();
    event_loop.run_app(&mut app).expect("run");
}
