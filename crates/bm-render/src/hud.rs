//! In-engine text overlay — the HUD now, more UI text later. Rasterises a line to an RGBA
//! strip with the tiny `font8x8` bitmap font; the renderer draws it as a screen-space
//! quad. Used on **every** platform (web included) so text rendering is identical — no
//! DOM HUD. Pixel font on purpose: it's on-brand with the low-fi look.

use font8x8::legacy::BASIC_LEGACY;

/// Glyph cell size (px) of the `font8x8` font.
pub const GLYPH: usize = 8;

/// Rasterise `text` to a tightly-packed RGBA buffer: white text on a translucent dark
/// strip (so it stays legible over bright sky/foliage). One 8×8 cell per character;
/// non-printable / non-ASCII chars render as `.`. Honours embedded newlines (`\n`) as line
/// breaks — sized to the widest line × the line count. Returns `(width, height, pixels)`.
pub fn rasterize(text: &str) -> (u32, u32, Vec<u8>) {
    let lines: Vec<&str> = text.split('\n').collect();
    let cols = lines
        .iter()
        .map(|l| l.chars().count())
        .max()
        .unwrap_or(0)
        .max(1);
    let rows = lines.len().max(1);
    let w = (cols * GLYPH) as u32;
    let h = (rows * GLYPH) as u32;
    let mut px = vec![0u8; (w * h * 4) as usize];
    // Translucent dark background strip for contrast.
    for p in px.chunks_exact_mut(4) {
        p[3] = 150;
    }
    for (li, line) in lines.iter().enumerate() {
        let y0 = li * GLYPH;
        for (gi, c) in line.chars().enumerate() {
            let code = c as u32;
            let glyph = if (0x20..0x7f).contains(&code) {
                BASIC_LEGACY[code as usize]
            } else {
                BASIC_LEGACY[b'.' as usize]
            };
            for (row, bits) in glyph.iter().enumerate() {
                for col in 0..GLYPH {
                    // font8x8 packs each row LSB-first (bit 0 = leftmost column).
                    if bits & (1 << col) != 0 {
                        let x = gi * GLYPH + col;
                        let idx = ((y0 + row) * w as usize + x) * 4;
                        px[idx] = 255;
                        px[idx + 1] = 255;
                        px[idx + 2] = 255;
                        px[idx + 3] = 255;
                    }
                }
            }
        }
    }
    (w, h, px)
}

/// Word-wrap `text` to at most `max_cols` characters per line, inserting `\n` (and keeping any
/// explicit `\n`). Words longer than `max_cols` are hard-broken. Lets the HUD wrap at the screen
/// edge instead of running off it.
pub fn wrap(text: &str, max_cols: usize) -> String {
    if max_cols == 0 {
        return text.to_string();
    }
    let mut out = String::new();
    for (pi, para) in text.split('\n').enumerate() {
        if pi > 0 {
            out.push('\n');
        }
        let mut col = 0usize;
        for word in para.split(' ') {
            let wl = word.chars().count();
            if col > 0 && col + 1 + wl > max_cols {
                out.push('\n');
                col = 0;
            }
            if col > 0 {
                out.push(' ');
                col += 1;
            }
            if wl > max_cols {
                for c in word.chars() {
                    if col >= max_cols {
                        out.push('\n');
                        col = 0;
                    }
                    out.push(c);
                    col += 1;
                }
            } else {
                out.push_str(word);
                col += wl;
            }
        }
    }
    out
}

/// GPU overlay: rasterises the HUD line to a texture and draws it as a top-left
/// screen-space quad over the finished frame. Reused by the windowed renderer (`gfx`)
/// and the headless renderer, so the overlay looks identical and is verifiable offscreen.
pub struct HudOverlay {
    pipeline: wgpu::RenderPipeline,
    bgl: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    rect_buf: wgpu::Buffer,
    tex: Option<TexEntry>,
    last: String,
}

struct TexEntry {
    bind_group: wgpu::BindGroup,
    w: u32,
    h: u32,
}

impl HudOverlay {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> HudOverlay {
        let shader = device.create_shader_module(wgpu::include_wgsl!("hud.wgsl"));
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("hud-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("hud-pipeline-layout"),
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("hud-pipeline"),
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
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("hud-sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let rect_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("hud-rect"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        HudOverlay {
            pipeline,
            bgl,
            sampler,
            rect_buf,
            tex: None,
            last: String::new(),
        }
    }

    /// Update the overlay text (re-rasterises + re-uploads only when it changed).
    pub fn set_text(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, text: &str) {
        if text == self.last && self.tex.is_some() {
            return;
        }
        self.last = text.to_string();
        let (w, h, px) = rasterize(text);
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("hud-text"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &px,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(w * 4),
                rows_per_image: Some(h),
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("hud-bg"),
            layout: &self.bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.rect_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        self.tex = Some(TexEntry { bind_group, w, h });
    }

    /// Draw the overlay onto `view` (loading, so it composites over the finished frame).
    pub fn draw(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        surface_w: u32,
        surface_h: u32,
    ) {
        let Some(t) = &self.tex else {
            return;
        };
        // Scale the 8px font up for legibility on high-DPI screens; anchor top-left.
        let scale = (surface_h / 360).max(2);
        let (tw, th) = ((t.w * scale) as f32, (t.h * scale) as f32);
        let (fw, fh) = (surface_w.max(1) as f32, surface_h.max(1) as f32);
        let m = 8.0;
        let rect = [
            -1.0 + 2.0 * m / fw,        // x0 (left)
            1.0 - 2.0 * m / fh,         // y0 (top)
            -1.0 + 2.0 * (m + tw) / fw, // x1 (right)
            1.0 - 2.0 * (m + th) / fh,  // y1 (bottom)
        ];
        queue.write_buffer(&self.rect_buf, 0, bytemuck::cast_slice(&rect));
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("hud-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
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
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &t.bind_group, &[]);
        pass.draw(0..6, 0..1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rasterize_sizes_and_draws_pixels() {
        let (w, h, px) = rasterize("brickmap 0123");
        assert_eq!(h, 8);
        assert_eq!(w, 13 * 8);
        assert_eq!(px.len() as u32, w * h * 4);
        // Some text pixels must be fully white (a glyph was drawn).
        assert!(px.chunks_exact(4).any(|p| p == [255, 255, 255, 255]));
    }

    #[test]
    fn wrap_breaks_at_width_and_keeps_newlines() {
        let w = wrap("alpha beta gamma delta", 11);
        for line in w.split('\n') {
            assert!(line.chars().count() <= 11, "line too long: {line:?}");
        }
        assert!(w.contains('\n'), "should have wrapped");
        assert_eq!(wrap("a\nb", 80), "a\nb");
        let (_, h, _) = rasterize("a\nb\nc");
        assert_eq!(h, 24);
    }

    /// Dump a sample HUD strip to a PNG so the font/layout can be eyeballed.
    #[test]
    fn dump_sample_png() {
        let (w, h, px) = rasterize("brickmap e0a1c11 - 60 fps - seed 1337");
        let scale = 4u32;
        let (sw, sh) = (w * scale, h * scale);
        let mut up = vec![0u8; (sw * sh * 4) as usize];
        for y in 0..sh {
            for x in 0..sw {
                let s = ((y / scale) * w + (x / scale)) as usize * 4;
                let d = (y * sw + x) as usize * 4;
                up[d..d + 4].copy_from_slice(&px[s..s + 4]);
            }
        }
        let f = std::fs::File::create("/tmp/hud_sample.png").expect("create");
        let mut enc = png::Encoder::new(std::io::BufWriter::new(f), sw, sh);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        enc.write_header().unwrap().write_image_data(&up).unwrap();
    }
}
