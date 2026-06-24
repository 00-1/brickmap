//! `music_proto` — phase-4 audio evidence: render a few seconds of each generative-music scene
//! ([`goblin_gold::music`]) to a 16-bit mono WAV, for the owner's A/B-vs-web-GG by-ear check
//! (`OWNER-EYEBALL.md`). The note schedule is vector-proven (`synth.rs`); the timbre is a first cut.
//!
//! Run:  cargo run -p goblin-gold --bin music_proto -- <out_dir> [seconds]

use goblin_gold::music::render;
use goblin_gold::synth::STYLE_IDS;
use std::io::Write;

const SR: u32 = 44_100;

fn main() {
    let out_dir = std::env::args().nth(1).unwrap_or_else(|| ".".to_string());
    let secs: f32 = std::env::args()
        .nth(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(8.0);
    std::fs::create_dir_all(&out_dir).ok();

    for name in STYLE_IDS {
        let buf = render(name, SR, secs, 1.0);
        let path = format!("{out_dir}/gg-music-{name}.wav");
        write_wav(&path, SR, &buf);
        println!("wrote {path} ({:.1}s)", buf.len() as f32 / SR as f32);
    }
}

/// Write a mono 16-bit PCM WAV.
fn write_wav(path: &str, sample_rate: u32, samples: &[f32]) {
    let mut f = std::fs::File::create(path).expect("create wav");
    let data_len = (samples.len() * 2) as u32;
    let mut h = Vec::with_capacity(44 + data_len as usize);
    h.extend_from_slice(b"RIFF");
    h.extend_from_slice(&(36 + data_len).to_le_bytes());
    h.extend_from_slice(b"WAVE");
    h.extend_from_slice(b"fmt ");
    h.extend_from_slice(&16u32.to_le_bytes());
    h.extend_from_slice(&1u16.to_le_bytes()); // PCM
    h.extend_from_slice(&1u16.to_le_bytes()); // mono
    h.extend_from_slice(&sample_rate.to_le_bytes());
    h.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    h.extend_from_slice(&2u16.to_le_bytes());
    h.extend_from_slice(&16u16.to_le_bytes());
    h.extend_from_slice(b"data");
    h.extend_from_slice(&data_len.to_le_bytes());
    for &s in samples {
        h.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
    }
    f.write_all(&h).expect("write wav");
}
