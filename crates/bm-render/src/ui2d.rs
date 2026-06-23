//! `ui2d` — the engine's 2-D **UI surface** primitives + widgets. The reusable draw contract for
//! flat-shaded menus/HUD-prose/overlays: filled [`RectRun`]s and runs of atlas text ([`TextRun`],
//! from [`crate::text2d`]), plus a shared quad shader ([`UI_SHADER`]) that any renderer — an
//! on-screen surface or the headless golden painter — builds its pipeline from. Data-free
//! **widgets** (e.g. the numeric [`keypad`]) live here too, so the on-screen and offscreen paths
//! produce identical geometry.
//!
//! The renderer owns the GPU plumbing (it knows its target size, format, and whether it presents
//! to a swapchain or reads back a PNG); this module is just the primitives + geometry, so the same
//! UI is pixel-identical on device and in a golden test.

use crate::text2d::{Atlas, Quad};

pub mod keypad;

/// A run of text: its coverage atlas, the laid-out glyph quads, and an RGBA colour (0..1).
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

/// WGSL for the unified UI quad pipeline: each vertex carries pos (NDC) + uv + rgba; the fragment
/// samples an `R8` coverage texture (a glyph slot, or a 1×1 white texel for solid rects) and
/// multiplies that coverage into the vertex alpha. A renderer fills a vertex buffer of
/// `{pos:[f32;2], uv:[f32;2], rgba:[f32;4]}` and draws with one bind group (texture + sampler).
pub const UI_SHADER: &str = r#"
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
