//! `drill_proto` — BRICKMAP-GG1 mini-gate #2 evidence generator.
//!
//! Renders the **keypad + one drill loop** for a single topic through the REAL wgpu path
//! (offscreen, software-Vulkan ok). The questions come from the **T229 content data seam**
//! (`data/gg1/*.json`) via [`goblin_gold::drill::Drill`]; the on-screen keypad geometry comes
//! from the data-free [`goblin_gold::keypad::Keypad`] widget. The verdict (right/wrong) is the
//! real model's — we drive it with key presses, then snapshot the screen state.
//!
//! Three states are captured: a fresh question mid-entry, a correct (green) **auto-accepted**
//! answer, and a **skipped** one (the action bar reveals the answer). GG1 has no wrong state — the
//! answer auto-checks as you type. The on-device/web render is the same wgpu pipeline, so at 1:1
//! these pixels equal the phone's.
//!
//! Run:  VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json \
//!         cargo run -p goblin-gold --bin drill_proto -- <out_dir>

use ab_glyph::FontRef;
use goblin_gold::drill::{Drill, Mark};
use goblin_gold::headless::{Painter, RectRun, TextRun};
use goblin_gold::keypad::{Key, Keypad};
use goblin_gold::text::Atlas;

const BG: [f32; 3] = [20.0 / 255.0, 12.0 / 255.0, 34.0 / 255.0]; // GG deep violet
const PANEL: [f32; 4] = [34.0 / 255.0, 22.0 / 255.0, 54.0 / 255.0, 1.0];
const KEYBG: [f32; 4] = [46.0 / 255.0, 32.0 / 255.0, 72.0 / 255.0, 1.0];
const INK: [f32; 4] = [16.0 / 255.0, 10.0 / 255.0, 28.0 / 255.0, 1.0]; // dark text on gold
const GOLD: [f32; 4] = [1.0, 214.0 / 255.0, 110.0 / 255.0, 1.0];
const BODY: [f32; 4] = [232.0 / 255.0, 228.0 / 255.0, 244.0 / 255.0, 1.0];
const DIM: [f32; 4] = [150.0 / 255.0, 140.0 / 255.0, 172.0 / 255.0, 1.0];
const GREEN: [f32; 4] = [120.0 / 255.0, 222.0 / 255.0, 142.0 / 255.0, 1.0];

const W: u32 = 1080;
const H: u32 = 1920;
const MARGIN: f32 = 64.0;

/// One captured screen: what to draw on it (the model has already produced `mark`).
struct Shot {
    label: String, // the framed question, e.g. "Half of 30"
    typed: String, // what's in the answer box
    mark: Option<Mark>,
    solved: u32,
    total: usize,
}

fn main() {
    let out_dir = std::env::args().nth(1).unwrap_or_else(|| ".".to_string());
    std::fs::create_dir_all(&out_dir).ok();
    let font = FontRef::try_from_slice(goblin_gold::FONT_INSTRUMENT_SANS).expect("font");

    // Atlases (baked once). Key atlas asks for ✓/⌫ but falls back if the face lacks them.
    let a_head = Atlas::bake(&font, 88.0);
    let a_tag = Atlas::bake(&font, 40.0);
    let a_q = Atlas::bake(&font, 84.0);
    let a_entry = Atlas::bake(&font, 76.0);
    let a_banner = Atlas::bake(&font, 46.0);
    let a_key = Atlas::bake_chars(&font, 66.0, "✓⌫");

    let painter = Painter::new();

    // State A — fresh question, mid-entry (one digit tapped, no submit yet).
    let mut d = Drill::from_seam("halves");
    let label_a = format!("Half of {}", d.prompt());
    d.press(Key::Digit(1));
    let shot_a = Shot {
        label: label_a,
        typed: d.typed().to_string(),
        mark: None,
        solved: d.solved(),
        total: d.len(),
    };

    // State B — a correct answer (green). Typing the value auto-accepts (no submit key); show the
    // value that triggered it.
    let mut d = Drill::from_seam("halves");
    let label_b = format!("Half of {}", d.prompt());
    let correct = format!("{}", d.expected());
    for c in correct.chars() {
        d.press(Keypad::key_for_char(c).expect("digit/dot"));
    }
    let shot_b = Shot {
        label: label_b,
        typed: correct,
        mark: d.last_mark(), // Mark::Right (auto-accepted on the final digit)
        solved: d.solved(),
        total: d.len(),
    };

    // State C — a skip. The action bar reveals the answer and moves on (counts as a skip).
    let mut d = Drill::from_seam("halves");
    let label_c = format!("Half of {}", d.prompt());
    d.press(Key::Enter); // the bottom bar is SKIP
    let shot_c = Shot {
        label: label_c,
        typed: d.revealed().unwrap_or("").to_string(),
        mark: d.last_mark(), // Mark::Skipped
        solved: d.solved(),
        total: d.len(),
    };

    let name = d.name.clone();
    let tag = d.tag.clone();
    let atlases = Atlases {
        head: &a_head,
        tag: &a_tag,
        q: &a_q,
        entry: &a_entry,
        banner: &a_banner,
        key: &a_key,
    };

    for (shot, file) in [
        (&shot_a, "gg-drill-entry.png"),
        (&shot_b, "gg-drill-correct.png"),
        (&shot_c, "gg-drill-skip.png"),
    ] {
        let path = format!("{out_dir}/{file}");
        render(&painter, &atlases, &name, &tag, shot, &path);
        println!("wrote {path}");
    }
}

struct Atlases<'a> {
    head: &'a Atlas,
    tag: &'a Atlas,
    q: &'a Atlas,
    entry: &'a Atlas,
    banner: &'a Atlas,
    key: &'a Atlas,
}

/// Lay `text` centred horizontally on `cx` and vertically within [`top`, `top+h`].
fn centered(atlas: &Atlas, text: &str, cx: f32, top: f32, h: f32) -> Vec<goblin_gold::text::Quad> {
    let w = atlas.text_width(text);
    let x0 = cx - w / 2.0;
    let y0 = top + h / 2.0 - 0.59 * atlas.px; // visual-centre fudge (see text.rs metrics)
    atlas.layout(text, x0, y0, f32::INFINITY).0
}

/// Draw `outer` (the accent frame), then an inner fill inset by `t` on every side.
fn framed(rects: &mut Vec<RectRun>, outer: RectRun, t: f32, fill: [f32; 4]) {
    rects.push(outer);
    rects.push(RectRun {
        x: outer.x + t,
        y: outer.y + t,
        w: outer.w - 2.0 * t,
        h: outer.h - 2.0 * t,
        rgba: fill,
    });
}

fn render(painter: &Painter, a: &Atlases, name: &str, tag: &str, shot: &Shot, path: &str) {
    let mut rects: Vec<RectRun> = Vec::new();
    let mut texts: Vec<TextRun> = Vec::new();
    let col_w = W as f32 - MARGIN * 2.0;
    let cx = W as f32 / 2.0;

    // Heading (mode name) + tag line.
    let (q, _h) = a.head.layout(name, MARGIN, 64.0, col_w);
    texts.push(TextRun {
        atlas: a.head,
        quads: q,
        rgba: GOLD,
    });
    let (q, _h) = a.tag.layout(tag, MARGIN, 176.0, col_w);
    texts.push(TextRun {
        atlas: a.tag,
        quads: q,
        rgba: DIM,
    });

    // Progress, right-aligned on the heading row.
    let prog = format!("{} / {}", shot.solved, shot.total);
    let pw = a.tag.text_width(&prog);
    let (q, _h) = a.tag.layout(&prog, W as f32 - MARGIN - pw, 96.0, pw + 4.0);
    texts.push(TextRun {
        atlas: a.tag,
        quads: q,
        rgba: DIM,
    });

    // Question card.
    let card_y = 280.0;
    let card_h = 300.0;
    framed(
        &mut rects,
        RectRun {
            x: MARGIN,
            y: card_y,
            w: col_w,
            h: card_h,
            rgba: [GOLD[0], GOLD[1], GOLD[2], 0.5],
        },
        4.0,
        PANEL,
    );
    texts.push(TextRun {
        atlas: a.q,
        quads: centered(a.q, &shot.label, cx, card_y, card_h),
        rgba: GOLD,
    });

    // Answer box — frame colour reflects the verdict (no wrong state).
    let (frame, ink) = match shot.mark {
        Some(Mark::Right) => (GREEN, GREEN),
        Some(Mark::Skipped) => (DIM, DIM),
        None => (DIM, BODY),
    };
    let box_y = 620.0;
    let box_h = 150.0;
    let box_w = col_w * 0.7;
    let box_x = cx - box_w / 2.0;
    framed(
        &mut rects,
        RectRun {
            x: box_x,
            y: box_y,
            w: box_w,
            h: box_h,
            rgba: frame,
        },
        4.0,
        [28.0 / 255.0, 18.0 / 255.0, 44.0 / 255.0, 1.0],
    );
    let shown = if shot.typed.is_empty() {
        "·".to_string()
    } else {
        shot.typed.clone()
    };
    texts.push(TextRun {
        atlas: a.entry,
        quads: centered(a.entry, &shown, cx, box_y, box_h),
        rgba: ink,
    });

    // Verdict banner (no wrong state — it auto-checks; the action bar skips).
    let (msg, colour) = match shot.mark {
        Some(Mark::Right) => ("Correct!", GREEN),
        Some(Mark::Skipped) => ("Skipped", DIM),
        None => ("Tap the digits — it checks itself", DIM),
    };
    let mw = a.banner.text_width(msg);
    let (q, _h) = a.banner.layout(msg, cx - mw / 2.0, 800.0, mw + 4.0);
    texts.push(TextRun {
        atlas: a.banner,
        quads: q,
        rgba: colour,
    });

    // Keypad.
    let kp_y = 920.0;
    let kp_h = H as f32 - kp_y - MARGIN;
    let kp = Keypad::layout(MARGIN, kp_y, col_w, kp_h, 18.0);
    let back_label = if a.key.glyphs.contains_key(&'⌫') {
        "⌫"
    } else {
        "<"
    };
    let mut key_quads: Vec<goblin_gold::text::Quad> = Vec::new(); // BODY-coloured labels share one run
    for cell in &kp.cells {
        let is_enter = cell.key == Key::Enter;
        let fill = if is_enter { GOLD } else { KEYBG };
        rects.push(RectRun {
            x: cell.x,
            y: cell.y,
            w: cell.w,
            h: cell.h,
            rgba: fill,
        });
        if is_enter {
            continue; // the action bar (Skip) label is inked dark on gold — its own run below
        }
        let s = match cell.key {
            Key::Digit(d) => ((b'0' + d) as char).to_string(),
            Key::Dot => ".".to_string(),
            Key::Back => back_label.to_string(),
            Key::Enter => unreachable!(),
        };
        key_quads.extend(centered(a.key, &s, cell.x + cell.w / 2.0, cell.y, cell.h));
    }
    texts.push(TextRun {
        atlas: a.key,
        quads: key_quads,
        rgba: BODY,
    });
    // Action-bar label (Skip, dark on gold).
    if let Some(enter) = kp.cells.iter().find(|c| c.key == Key::Enter) {
        let q = centered(a.key, "Skip", enter.x + enter.w / 2.0, enter.y, enter.h);
        texts.push(TextRun {
            atlas: a.key,
            quads: q,
            rgba: INK,
        });
    }

    painter.paint(W, H, BG, &rects, &texts, path);
}
