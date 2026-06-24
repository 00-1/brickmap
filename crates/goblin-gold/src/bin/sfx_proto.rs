//! `sfx_proto` — phase-4 audio evidence: render each GG1 sound effect ([`goblin_gold::sfx`]) to a
//! 16-bit mono WAV, so the owner can A/B them by ear against web-GG (`OWNER-EYEBALL.md`). Audio
//! parity is perceptual, not vector-provable, so this is the by-ear deliverable.
//!
//! Run:  cargo run -p goblin-gold --bin sfx_proto -- <out_dir>

use goblin_gold::sfx::{render, Rarity, Sfx};
use std::io::Write;

const SR: u32 = 44_100;

fn main() {
    let out_dir = std::env::args().nth(1).unwrap_or_else(|| ".".to_string());
    std::fs::create_dir_all(&out_dir).ok();

    let events: &[(&str, Sfx)] = &[
        ("correct-combo0", Sfx::Correct { combo: 0 }),
        ("correct-combo6", Sfx::Correct { combo: 6 }),
        ("correct-combo12", Sfx::Correct { combo: 12 }),
        ("skip", Sfx::Skip),
        (
            "item-common",
            Sfx::Item {
                rarity: Rarity::Common,
            },
        ),
        (
            "item-legendary",
            Sfx::Item {
                rarity: Rarity::Legendary,
            },
        ),
        ("gold", Sfx::Gold { big: false }),
        ("gold-big", Sfx::Gold { big: true }),
        ("topic-unlock", Sfx::TopicUnlock),
        ("mastery", Sfx::Mastery),
        ("topic100", Sfx::Topic100),
        ("round-start", Sfx::RoundStart),
        ("round-complete", Sfx::RoundComplete),
    ];

    for (name, e) in events {
        let buf = render(e, SR, 1.0);
        let path = format!("{out_dir}/gg-sfx-{name}.wav");
        write_wav(&path, SR, &buf);
        println!("wrote {path} ({} samples)", buf.len());
    }
}

/// Write a mono 16-bit PCM WAV (a minimal 44-byte header + samples).
fn write_wav(path: &str, sample_rate: u32, samples: &[f32]) {
    let mut f = std::fs::File::create(path).expect("create wav");
    let data_len = (samples.len() * 2) as u32;
    let byte_rate = sample_rate * 2;
    let mut h = Vec::with_capacity(44 + data_len as usize);
    h.extend_from_slice(b"RIFF");
    h.extend_from_slice(&(36 + data_len).to_le_bytes());
    h.extend_from_slice(b"WAVE");
    h.extend_from_slice(b"fmt ");
    h.extend_from_slice(&16u32.to_le_bytes()); // PCM fmt chunk size
    h.extend_from_slice(&1u16.to_le_bytes()); // PCM
    h.extend_from_slice(&1u16.to_le_bytes()); // mono
    h.extend_from_slice(&sample_rate.to_le_bytes());
    h.extend_from_slice(&byte_rate.to_le_bytes());
    h.extend_from_slice(&2u16.to_le_bytes()); // block align
    h.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    h.extend_from_slice(b"data");
    h.extend_from_slice(&data_len.to_le_bytes());
    for &s in samples {
        let v = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
        h.extend_from_slice(&v.to_le_bytes());
    }
    f.write_all(&h).expect("write wav");
}
