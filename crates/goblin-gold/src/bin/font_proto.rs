//! `font_proto` — BRICKMAP-GG1 mini-gate #1 evidence generator.
//!
//! Renders a representative Goblin-Gold **guide + question screen** (heading, a guide
//! *paragraph* — the legibility test — a worked example, and a drill question) through the
//! REAL wgpu path (offscreen, software-Vulkan ok) using the AA TTF atlas, one PNG per reading
//! size. The on-device/web render is the same wgpu pipeline, so at 1:1 these pixels equal the
//! phone's — the legibility verdict carries.
//!
//! Run:  VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json \
//!         cargo run -p goblin-gold --bin font_proto -- <out_dir>

use ab_glyph::FontRef;
use goblin_gold::render::{Painter, TextRun};
use goblin_gold::text::Atlas;

const BG: [f32; 3] = [20.0 / 255.0, 12.0 / 255.0, 34.0 / 255.0]; // GG deep violet panel
const GOLD: [f32; 4] = [1.0, 214.0 / 255.0, 110.0 / 255.0, 1.0];
const BODY: [f32; 4] = [226.0 / 255.0, 222.0 / 255.0, 238.0 / 255.0, 1.0];
const DIM: [f32; 4] = [150.0 / 255.0, 140.0 / 255.0, 172.0 / 255.0, 1.0];

fn main() {
    let out_dir = std::env::args().nth(1).unwrap_or_else(|| ".".to_string());
    std::fs::create_dir_all(&out_dir).ok();
    let font = FontRef::try_from_slice(goblin_gold::FONT_INSTRUMENT_SANS).expect("font");

    let topic = "Halving";
    let guide = "To halve a number, split it into two equal parts that add back up to the whole. \
Halve each digit in turn; when a digit is odd, carry a 5 into the next column. So half of 38 is 19, \
and half of 174 is 87. Halving twice is the same as dividing by four.";
    let example = "Worked example:  half of 48  =  24";
    let question = "Half of 168  =  ?";
    let hint = "Tap the number, then press =";

    let width: u32 = 1080;
    let height: u32 = 1600;
    let margin = 72.0;
    let col_w = width as f32 - margin * 2.0;

    let painter = Painter::new();

    for &body_px in &[28.0_f32, 34.0, 42.0] {
        let head_px = (body_px * 1.7).round();
        let big_px = (body_px * 1.9).round();
        let a_body = Atlas::bake(&font, body_px);
        let a_head = Atlas::bake(&font, head_px);
        let a_big = Atlas::bake(&font, big_px);

        let mut texts: Vec<TextRun> = Vec::new();
        let mut y = margin;
        let (q, h) = a_head.layout(topic, margin, y, col_w);
        texts.push(TextRun {
            atlas: &a_head,
            quads: q,
            rgba: GOLD,
        });
        y += h + body_px * 0.6;
        let (q, h) = a_body.layout(guide, margin, y, col_w);
        texts.push(TextRun {
            atlas: &a_body,
            quads: q,
            rgba: BODY,
        });
        y += h + body_px * 0.8;
        let (q, h) = a_body.layout(example, margin, y, col_w);
        texts.push(TextRun {
            atlas: &a_body,
            quads: q,
            rgba: GOLD,
        });
        y += h + body_px * 1.4;
        let (q, h) = a_big.layout(question, margin, y, col_w);
        texts.push(TextRun {
            atlas: &a_big,
            quads: q,
            rgba: GOLD,
        });
        y += h + body_px * 0.6;
        let (q, _h) = a_body.layout(hint, margin, y, col_w);
        texts.push(TextRun {
            atlas: &a_body,
            quads: q,
            rgba: DIM,
        });

        let path = format!("{out_dir}/gg-prose-{}px.png", body_px as u32);
        painter.paint(width, height, BG, &[], &texts, &path);
        println!("wrote {path}");
    }
}
