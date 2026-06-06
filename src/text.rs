//! In-world text (E17). Rasterises a string to a transparent-background glyph texture with
//! the tiny `font8x8` bitmap font, so the renderer can draw it on a **world-space billboard**
//! (glowing inscriptions in the world), the same low-fi pixel font as the HUD.
//!
//! Existing writing systems only (not procedural runes): `font8x8` carries Basic Latin plus
//! Greek, Hiragana, and the Standard Galactic Alphabet — so an "unusual but real" script is
//! just a matter of which characters you pass. Pure logic; the GPU side lives in `gfx`.

use bytemuck::{Pod, Zeroable};
use font8x8::UnicodeFonts;
use glam::Vec3;

/// Glyph cell size (px).
pub const GLYPH: usize = 8;

/// Look a character up across the font sets we ship (Basic Latin → Latin-1 → Greek →
/// Hiragana → Standard Galactic). `None` if no set has it.
fn glyph(c: char) -> Option<[u8; 8]> {
    font8x8::BASIC_FONTS
        .get(c)
        .or_else(|| font8x8::LATIN_FONTS.get(c))
        .or_else(|| font8x8::GREEK_FONTS.get(c))
        .or_else(|| font8x8::HIRAGANA_FONTS.get(c))
        .or_else(|| font8x8::SGA_FONTS.get(c))
}

/// Rasterise `text` to a tightly-packed RGBA buffer: **white glyphs on a fully transparent
/// background** (one 8×8 cell per char), so a billboard can alpha-test the glyphs and tint
/// them. Unknown glyphs render blank (a space). Returns `(width, height, pixels)`.
pub fn rasterize(text: &str) -> (u32, u32, Vec<u8>) {
    let n = text.chars().count().max(1);
    let w = (n * GLYPH) as u32;
    let h = GLYPH as u32;
    let mut px = vec![0u8; (w * h * 4) as usize]; // transparent (alpha 0)
    for (gi, c) in text.chars().enumerate() {
        let Some(bits_rows) = glyph(c) else {
            continue;
        };
        for (row, bits) in bits_rows.iter().enumerate() {
            for col in 0..GLYPH {
                // font8x8 packs each row LSB-first (bit 0 = leftmost column).
                if bits & (1 << col) != 0 {
                    let x = gi * GLYPH + col;
                    let idx = (row * w as usize + x) * 4;
                    px[idx..idx + 4].copy_from_slice(&[255, 255, 255, 255]);
                }
            }
        }
    }
    (w, h, px)
}

/// Per-label uniform (std140): world centre, billboard half-extents, emissive tint.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct LabelUniform {
    center: [f32; 4],
    half: [f32; 4],
    color: [f32; 4],
}

/// World-space text (E17): rasterises strings to glyph textures and draws them as
/// camera-facing billboards **inside the scene pass** (so they depth-test against the world,
/// glow through bloom, and are recoloured by the palette). Shares the globals bind group
/// (group 0) with the rest of the scene; each label owns its texture + uniform (group 1).
/// Reused by the windowed (`gfx`) and headless renderers.
pub struct WorldText {
    pipeline: wgpu::RenderPipeline,
    bgl: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    labels: Vec<wgpu::BindGroup>,
}

impl WorldText {
    pub fn new(
        device: &wgpu::Device,
        color_format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
        globals_bgl: &wgpu::BindGroupLayout,
    ) -> WorldText {
        let shader = device.create_shader_module(wgpu::include_wgsl!("text.wgsl"));
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("worldtext-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    // Read in both stages: the VS billboards from `center`/`half`, the FS
                    // tints with `color`.
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
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
            label: Some("worldtext-layout"),
            bind_group_layouts: &[Some(globals_bgl), Some(&bgl)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("worldtext-pipeline"),
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
                    format: color_format,
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
            depth_stencil: Some(wgpu::DepthStencilState {
                format: depth_format,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("worldtext-sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        WorldText {
            pipeline,
            bgl,
            sampler,
            labels: Vec::new(),
        }
    }

    /// Add an inscription: `text` rasterised, billboarded at `center`, `world_height` tall
    /// (its width follows the text's aspect), tinted `color`.
    pub fn add(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        text: &str,
        center: Vec3,
        world_height: f32,
        color: [f32; 3],
    ) {
        let (w, h, px) = rasterize(text);
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("worldtext-tex"),
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
        let hy = world_height * 0.5;
        let hx = hy * (w as f32 / h as f32);
        let uniform = LabelUniform {
            center: [center.x, center.y, center.z, 0.0],
            half: [hx, hy, 0.0, 0.0],
            color: [color[0], color[1], color[2], 1.0],
        };
        let ubuf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("worldtext-label"),
            size: std::mem::size_of::<LabelUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&ubuf, 0, bytemuck::bytes_of(&uniform));
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("worldtext-bg"),
            layout: &self.bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: ubuf.as_entire_binding(),
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
        self.labels.push(bind_group);
    }

    pub fn is_empty(&self) -> bool {
        self.labels.is_empty()
    }

    /// Draw all labels into an in-progress scene render pass (shares its depth buffer).
    pub fn draw<'a>(
        &'a self,
        pass: &mut wgpu::RenderPass<'a>,
        globals_bind_group: &'a wgpu::BindGroup,
    ) {
        if self.labels.is_empty() {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, globals_bind_group, &[]);
        for bg in &self.labels {
            pass.set_bind_group(1, bg, &[]);
            pass.draw(0..6, 0..1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rasterize_sizes_and_draws_pixels() {
        let (w, h, px) = rasterize("ABC 123");
        assert_eq!(h, 8);
        assert_eq!(w, 7 * 8);
        assert_eq!(px.len() as u32, w * h * 4);
        // Some glyph pixels are opaque white; the background stays transparent.
        assert!(px.chunks_exact(4).any(|p| p == [255, 255, 255, 255]));
        assert!(px.chunks_exact(4).any(|p| p[3] == 0));
    }

    #[test]
    fn non_latin_scripts_render() {
        // Greek + Standard Galactic glyphs come from their own font sets, not Basic Latin.
        for s in ["ΑΩΔ", "αβγ"] {
            let (_, _, px) = rasterize(s);
            assert!(
                px.chunks_exact(4).any(|p| p[3] == 255),
                "expected glyph pixels for {s:?}"
            );
        }
    }
}
