//! In-world text (E17). Rasterises a string to a transparent-background glyph texture with
//! the tiny `font8x8` bitmap font, so the renderer can draw it on a **world-space billboard**
//! (glowing inscriptions in the world), the same low-fi pixel font as the HUD.
//!
//! Five writing systems via explicit [`Script`] selection: Latin, Greek, and Hiragana (from
//! `font8x8`), the Standard Galactic Alphabet (also `font8x8`, in the Private Use Area), and a
//! small hand-authored Elder-Futhark-style **runic** set. Real scripts, abstract content. Pure
//! logic; the GPU side lives in `gfx`.

use bytemuck::{Pod, Zeroable};
use font8x8::UnicodeFonts;
use glam::Vec3;

/// Glyph cell size (px).
pub const GLYPH: usize = 8;

/// A writing system to render a string in. Several real scripts ship in `font8x8`; `Galactic`
/// (Standard Galactic Alphabet) is keyed by Latin codepoints, so it needs an *explicit* script
/// (the Latin fallback would otherwise shadow it). `Runic` is our own small Elder-Futhark-style
/// set (a fifth, deliberately ancient-looking system). `Auto` is the multi-set fallback chain.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Script {
    Auto,
    Latin,
    Greek,
    Hiragana,
    Galactic,
    Runic,
}

impl Script {
    /// The five concrete writing systems, for seeded variety (excludes `Auto`).
    pub const ALL: [Script; 5] = [
        Script::Latin,
        Script::Greek,
        Script::Hiragana,
        Script::Galactic,
        Script::Runic,
    ];
}

/// A small Elder-Futhark-style runic set (hand-authored 8×8 bitmaps): a fifth, ancient-feeling
/// writing system beyond what `font8x8` ships. Any letter maps onto one of these angular staves
/// (abstract, not a faithful transliteration); spaces stay blank. Bit `col` = column `col` from
/// the left, matching [`rasterize`].
fn runic(c: char) -> Option<[u8; 8]> {
    if c == ' ' {
        return None;
    }
    // Deterministic stave per char (the overlay path keys the same index via `runic_pua`).
    Some(runic_stave((c as usize).wrapping_mul(7)))
}

/// PUA base for the runic staves on the **overlay** (HUD) path (G12): `RUNIC_PUA_BASE + idx`
/// renders stave `idx`. Distinct from the Standard-Galactic PUA block (U+E541..) so the HUD can
/// auto-detect either system by codepoint. (The world-text billboard keys staves off any char
/// directly via [`runic`]; the flat overlay needs a stable codepoint, hence this range.)
pub const RUNIC_PUA_BASE: u32 = 0xE600;
/// Count of distinct runic staves.
pub const RUNIC_STAVES: usize = 12;

/// One runic stave by index (wraps mod the stave count).
fn runic_stave(idx: usize) -> [u8; 8] {
    // Each rune is 8 rows of 8 chars; '#' = lit pixel. Angular, vertical-stave forms.
    const P: [[&str; 8]; 12] = [
        [
            "  #     ", "  ##    ", "  # #   ", "  ##    ", "  # #   ", "  #     ", "  #     ",
            "  #     ",
        ],
        [
            "  ###   ", "  #  #  ", "  #  #  ", "  #  #  ", "  #  #  ", "  #  #  ", "  #  #  ",
            "  #  #  ",
        ],
        [
            "  #     ", "  ##    ", "  # #   ", "  # #   ", "  ##    ", "  #     ", "  #     ",
            "  #     ",
        ],
        [
            "  #     ", "  ##    ", "  # #   ", "  #     ", "  ##    ", "  # #   ", "  #     ",
            "  #     ",
        ],
        [
            "  ##    ", "  # #   ", "  ##    ", "  # #   ", "  #  #  ", "  #     ", "  #     ",
            "  #     ",
        ],
        [
            "     #  ", "    #   ", "   #    ", "  #     ", "   #    ", "    #   ", "     #  ",
            "        ",
        ],
        [
            "#     # ", " #   #  ", "  # #   ", "   #    ", "  # #   ", " #   #  ", "#     # ",
            "        ",
        ],
        [
            "  ##    ", "  # #   ", "  ##    ", "  #     ", "  #     ", "  #     ", "  #     ",
            "  #     ",
        ],
        [
            "  #  #  ", "  #  #  ", "  #  #  ", "  ####  ", "  #  #  ", "  #  #  ", "  #  #  ",
            "  #  #  ",
        ],
        [
            "  #     ", "  #     ", "  #     ", "  #     ", "  #     ", "  #     ", "  #     ",
            "  #     ",
        ],
        [
            "  ####  ", "     #  ", "    #   ", "   #    ", "  #     ", "  #     ", "  ####  ",
            "        ",
        ],
        [
            "   #    ", "  ###   ", " # # #  ", "   #    ", "   #    ", "   #    ", "   #    ",
            "   #    ",
        ],
    ];
    let rows = P[idx % P.len()];
    let mut g = [0u8; 8];
    for (r, line) in rows.iter().enumerate() {
        for (col, b) in line.as_bytes().iter().enumerate().take(GLYPH) {
            if *b != b' ' {
                g[r] |= 1 << col;
            }
        }
    }
    g
}

/// Content-agnostic **mark glyphs** — a tiny PUA range of generic editorial/damage marks that
/// render identically in *every* script (world billboards and the flat HUD overlay alike):
/// a **lacuna** (a pitted patch where a glyph is lost), a **gouge** (a heavy strike band over a
/// blank cell), and an **underdot** (a low dot-row sub-mark). Engine-generic on purpose: these
/// are typographic damage/annotation cells, carrying no game concept — callers decide what they
/// mean. Sited next to the runic PUA block (distinct range, so codepoint auto-detection holds).
pub const MARK_PUA_BASE: u32 = 0xE620;
/// A lost glyph position (damage): a sparse pitted cell.
pub const MARK_LACUNA: char = '\u{E620}';
/// A deliberate strike/gouge over a blank cell.
pub const MARK_GOUGE: char = '\u{E621}';
/// A low dot-row sub-mark (annotates the preceding/overlying glyph cluster).
pub const MARK_UNDERDOT: char = '\u{E622}';
/// An **enclosure-open** mark: the left end of a bracket-frame drawn around a glyph run
/// (generic typography — callers decide what an enclosed run means).
pub const MARK_CARTOUCHE_OPEN: char = '\u{E623}';
/// The matching **enclosure-close** mark (right end of the bracket-frame).
pub const MARK_CARTOUCHE_CLOSE: char = '\u{E624}';
/// A **doubled-baseline** sub-mark: two faint parallel base rules under a blank cell (generic
/// typography — a "two writing lines" cell; callers decide what a doubled baseline means).
pub const MARK_BASELINE: char = '\u{E625}';

/// Bitmaps for the mark glyphs plus the standard epigraphic punctuation the overlay path should
/// render rather than dot out: U+27E6/U+27E7 (white square brackets) and U+2014 (em dash).
/// Generic punctuation/damage cells only — no game vocabulary.
fn mark_glyph(c: char) -> Option<[u8; 8]> {
    let rows: [&str; 8] = match c {
        MARK_LACUNA => [
            "        ", "  #   # ", "     #  ", " #      ", "    #   ", "  #   # ", "     #  ",
            "        ",
        ],
        MARK_GOUGE => [
            "        ", "        ", " ## ####", "########", "########", "#### ## ", "        ",
            "        ",
        ],
        MARK_UNDERDOT => [
            "        ", "        ", "        ", "        ", "        ", " ##  ## ", " ##  ## ",
            "        ",
        ],
        // The enclosure pair: bracket-frame ends whose horizontal arms reach the cell edge, so
        // consecutive cells read as one continuous frame around the enclosed run.
        MARK_CARTOUCHE_OPEN => [
            "   #####", "  #     ", " #      ", " #      ", " #      ", " #      ", "  #     ",
            "   #####",
        ],
        MARK_CARTOUCHE_CLOSE => [
            "#####   ", "     #  ", "      # ", "      # ", "      # ", "      # ", "     #  ",
            "#####   ",
        ],
        // Two faint parallel base rules — a subtle "doubled writing line" cell.
        MARK_BASELINE => [
            "        ", "        ", "        ", "        ", " # # # #", "        ", "# # # # ",
            "        ",
        ],
        '\u{27E6}' => [
            " ####   ", " # #    ", " # #    ", " # #    ", " # #    ", " # #    ", " # #    ",
            " ####   ",
        ],
        '\u{27E7}' => [
            "   #### ", "    # # ", "    # # ", "    # # ", "    # # ", "    # # ", "    # # ",
            "   #### ",
        ],
        '\u{2014}' => [
            "        ", "        ", "        ", "########", "########", "        ", "        ",
            "        ",
        ],
        _ => return None,
    };
    let mut g = [0u8; 8];
    for (r, line) in rows.iter().enumerate() {
        for (col, b) in line.as_bytes().iter().enumerate().take(GLYPH) {
            if *b != b' ' {
                g[r] |= 1 << col;
            }
        }
    }
    Some(g)
}

/// Look a character up in a given `script`. `Auto` walks the whole fallback chain; the explicit
/// scripts hit one set (so e.g. Standard Galactic isn't shadowed by Basic Latin). `None` if the
/// chosen set has no glyph for `c`. The generic [mark glyphs](MARK_PUA_BASE) render in every
/// script (damage/annotation cells are script-independent).
fn glyph(script: Script, c: char) -> Option<[u8; 8]> {
    if let Some(m) = mark_glyph(c) {
        return Some(m);
    }
    match script {
        Script::Auto => font8x8::BASIC_FONTS
            .get(c)
            .or_else(|| font8x8::LATIN_FONTS.get(c))
            .or_else(|| font8x8::GREEK_FONTS.get(c))
            .or_else(|| font8x8::HIRAGANA_FONTS.get(c))
            .or_else(|| font8x8::SGA_FONTS.get(c)),
        Script::Latin => font8x8::BASIC_FONTS
            .get(c)
            .or_else(|| font8x8::LATIN_FONTS.get(c)),
        Script::Greek => font8x8::GREEK_FONTS.get(c),
        Script::Hiragana => font8x8::HIRAGANA_FONTS.get(c),
        // SGA lives in the Private Use Area (U+E541..=U+E55A = a..z), so map Latin letters onto it.
        Script::Galactic => {
            let lc = c.to_ascii_lowercase();
            if lc.is_ascii_lowercase() {
                char::from_u32(0xE541 + (lc as u32 - 'a' as u32))
                    .and_then(|gc| font8x8::SGA_FONTS.get(gc))
            } else {
                None
            }
        }
        Script::Runic => runic(c),
    }
}

/// G12 — the **overlay** glyph path: a *single self-identifying codepoint* → its 8×8 bitmap,
/// for the flat HUD text path (which carries no per-char script tag, unlike the world billboards).
/// Covers the non-ASCII writing systems by codepoint: Greek and Hiragana (their own Unicode
/// blocks), the Standard Galactic Alphabet (its PUA block U+E541..), and our runic staves (the
/// [`RUNIC_PUA_BASE`] range). Returns `None` for ASCII and anything unknown, so the HUD keeps
/// rendering ASCII through its own legacy font (byte-identical) and unknowns as a dot.
pub fn overlay_glyph(c: char) -> Option<[u8; 8]> {
    if let Some(m) = mark_glyph(c) {
        return Some(m); // the generic mark/punctuation cells render on the HUD path too
    }
    let code = c as u32;
    if (RUNIC_PUA_BASE..RUNIC_PUA_BASE + RUNIC_STAVES as u32).contains(&code) {
        return Some(runic_stave((code - RUNIC_PUA_BASE) as usize));
    }
    font8x8::GREEK_FONTS
        .get(c)
        .or_else(|| font8x8::HIRAGANA_FONTS.get(c))
        .or_else(|| font8x8::SGA_FONTS.get(c))
}

/// The Standard-Galactic PUA codepoint for an ASCII letter (U+E541 + offset), or `None` for a
/// non-letter — the codepoint [`overlay_glyph`] renders as that SGA rune.
pub fn galactic_pua(c: char) -> Option<char> {
    let lc = c.to_ascii_lowercase();
    lc.is_ascii_lowercase()
        .then(|| char::from_u32(0xE541 + (lc as u32 - 'a' as u32)).unwrap())
}

/// The runic-stave PUA codepoint for `c` — keyed by the *same* index [`runic`] uses, so a string
/// mapped through here renders (via [`overlay_glyph`]) as the identical staves the world billboard
/// draws for the original string.
pub fn runic_pua(c: char) -> char {
    let idx = (c as usize).wrapping_mul(7) % RUNIC_STAVES;
    char::from_u32(RUNIC_PUA_BASE + idx as u32).unwrap()
}

/// Map a string from its **world-text representation** (what [`rasterize_script`] draws for a given
/// `script`) into the **self-identifying overlay codepoints** the flat HUD renders identically.
/// Greek/Hiragana/Latin already render by codepoint (identity); Galactic and Runic carry Latin
/// stand-in letters that only render correctly *with* their script, so they're remapped to their
/// PUA codepoints. The generic [mark glyphs](MARK_PUA_BASE) are already self-identifying and pass
/// through untouched in every script (a damage/enclosure cell must never remap to a stave).
/// This is what closes the world↔console recognition loop on the HUD.
pub fn to_overlay(s: &str, script: Script) -> String {
    s.chars()
        .map(|c| {
            if mark_glyph(c).is_some() {
                return c;
            }
            match script {
                Script::Galactic => galactic_pua(c).unwrap_or(c),
                Script::Runic => runic_pua(c),
                _ => c,
            }
        })
        .collect()
}

/// Rasterise `text` in the `Auto` fallback script. See [`rasterize_script`].
pub fn rasterize(text: &str) -> (u32, u32, Vec<u8>) {
    rasterize_script(text, Script::Auto)
}

/// Rasterise `text` (in `script`) to a tightly-packed RGBA buffer: **white glyphs on a fully
/// transparent background** (one 8×8 cell per char), so a billboard can alpha-test the glyphs
/// and tint them. Unknown glyphs render blank (a space). Returns `(width, height, pixels)`.
pub fn rasterize_script(text: &str, script: Script) -> (u32, u32, Vec<u8>) {
    let n = text.chars().count().max(1);
    let w = (n * GLYPH) as u32;
    let h = GLYPH as u32;
    let mut px = vec![0u8; (w * h * 4) as usize]; // transparent (alpha 0)
    for (gi, c) in text.chars().enumerate() {
        let Some(bits_rows) = glyph(script, c) else {
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

    /// Drop all labels (so the live world can rebuild the in-range set). The backing GPU
    /// textures/buffers are released with their bind groups.
    pub fn clear(&mut self) {
        self.labels.clear();
    }

    /// Add an inscription in the `Auto` fallback script. See [`WorldText::add_script`].
    pub fn add(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        text: &str,
        center: Vec3,
        world_height: f32,
        color: [f32; 3],
    ) {
        self.add_script(
            device,
            queue,
            text,
            Script::Auto,
            center,
            world_height,
            color,
        );
    }

    /// Add an inscription: `text` rasterised in `script`, billboarded at `center`,
    /// `world_height` tall (its width follows the text's aspect), tinted `color`.
    #[allow(clippy::too_many_arguments)]
    pub fn add_script(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        text: &str,
        script: Script,
        center: Vec3,
        world_height: f32,
        color: [f32; 3],
    ) {
        let (w, h, px) = rasterize_script(text, script);
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

    #[test]
    fn explicit_scripts_render_glyphs() {
        // Galactic (shadowed by Latin under Auto) and our Runic set must light pixels when
        // their script is chosen explicitly.
        for script in [Script::Galactic, Script::Runic, Script::Latin] {
            let (_, _, px) = rasterize_script("ABCD", script);
            assert!(
                px.chunks_exact(4).any(|p| p == [255, 255, 255, 255]),
                "expected glyph pixels for {script:?}"
            );
        }
    }

    /// G12 recognition loop: the **overlay** codepoints (what the flat HUD renders, by codepoint)
    /// reproduce the *exact* bitmaps the world-text billboard draws for the same string + script.
    /// So a block's console glyph cluster is visually identical to its world inscription.
    #[test]
    fn overlay_codepoints_reproduce_world_bitmaps() {
        // Galactic + Runic carry Latin-letter stand-ins; their PUA overlay codepoints must render
        // the same rune the world draws for the letter.
        for c in 'a'..='z' {
            assert_eq!(
                overlay_glyph(galactic_pua(c).unwrap()),
                glyph(Script::Galactic, c),
                "galactic overlay mismatch for {c}"
            );
            assert_eq!(
                overlay_glyph(runic_pua(c)),
                glyph(Script::Runic, c),
                "runic overlay mismatch for {c}"
            );
        }
        // Greek + Hiragana render by their own codepoints in both paths (identity overlay).
        for c in ['α', 'β', 'ω'] {
            assert_eq!(
                overlay_glyph(c),
                glyph(Script::Greek, c),
                "greek mismatch for {c}"
            );
        }
        for c in ['あ', 'き', 'ん'] {
            assert_eq!(
                overlay_glyph(c),
                glyph(Script::Hiragana, c),
                "hiragana mismatch for {c}"
            );
        }
        // ASCII is handled by the HUD's own legacy font (proven equal to Basic elsewhere); the
        // overlay path declines it so the HUD keeps its byte-identical ASCII rendering.
        assert_eq!(overlay_glyph('A'), None);
    }

    /// The content-agnostic mark glyphs render in **every** script on the world path, identically
    /// on the overlay (HUD) path, and are distinct from each other (a lacuna never reads as a
    /// gouge). The epigraphic punctuation (⟦ ⟧ —) renders on the overlay path too.
    #[test]
    fn mark_glyphs_render_in_all_scripts_and_on_the_overlay() {
        let marks = [
            MARK_LACUNA,
            MARK_GOUGE,
            MARK_UNDERDOT,
            MARK_CARTOUCHE_OPEN,
            MARK_CARTOUCHE_CLOSE,
            MARK_BASELINE,
        ];
        for mark in marks {
            let ov = overlay_glyph(mark).expect("mark renders on the overlay path");
            for script in Script::ALL {
                assert_eq!(
                    glyph(script, mark),
                    Some(ov),
                    "mark {mark:?} must render identically in {script:?}"
                );
            }
            assert!(ov.iter().any(|r| *r != 0), "mark {mark:?} lights pixels");
        }
        // Pairwise distinct (a lacuna never reads as a gouge; the enclosure ends differ).
        for i in 0..marks.len() {
            for j in (i + 1)..marks.len() {
                assert_ne!(
                    overlay_glyph(marks[i]),
                    overlay_glyph(marks[j]),
                    "marks {:?} and {:?} must be distinct",
                    marks[i],
                    marks[j]
                );
            }
        }
        for p in ['\u{27E6}', '\u{27E7}', '\u{2014}'] {
            assert!(
                overlay_glyph(p).is_some_and(|g| g.iter().any(|r| *r != 0)),
                "punctuation {p:?} renders on the overlay path"
            );
        }
        // The marks pass through `to_overlay` untouched in every script (never remapped to a
        // stave/SGA rune) — the world and HUD draw the same enclosure/damage cells.
        for script in Script::ALL {
            for mark in marks {
                assert_eq!(
                    to_overlay(&mark.to_string(), script),
                    mark.to_string(),
                    "mark {mark:?} must survive to_overlay in {script:?}"
                );
            }
        }
    }
}
