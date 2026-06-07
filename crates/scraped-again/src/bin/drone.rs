//! Render the seeded doom-drone (E16) to a 16-bit stereo WAV, so we can listen to and
//! iterate on the sound offline — the audio analogue of the `screenshot` dev tool.
//! Usage: `cargo run --bin drone -- [out.wav] [seconds] [seed]`

use std::io::Write;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let path = args
        .first()
        .cloned()
        .unwrap_or_else(|| "drone.wav".to_string());
    let seconds: f32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(30.0);
    let seed: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1337);
    let sr: u32 = 44_100;

    let samples = scraped_again::audio::Drone::render(seed, sr, seconds);
    write_wav(&path, &samples, sr, 2).expect("write wav");
    eprintln!(
        "wrote {path} ({seconds}s, seed {seed}, {} frames @ {sr} Hz)",
        samples.len() / 2
    );
}

/// Write interleaved f32 samples in `[-1, 1]` as a 16-bit PCM WAV (no external crate).
fn write_wav(path: &str, samples: &[f32], sr: u32, channels: u16) -> std::io::Result<()> {
    let bits = 16u16;
    let byte_rate = sr * channels as u32 * (bits / 8) as u32;
    let block_align = channels * (bits / 8);
    let data_len = (samples.len() * (bits / 8) as usize) as u32;

    let mut f = std::io::BufWriter::new(std::fs::File::create(path)?);
    f.write_all(b"RIFF")?;
    f.write_all(&(36 + data_len).to_le_bytes())?;
    f.write_all(b"WAVE")?;
    f.write_all(b"fmt ")?;
    f.write_all(&16u32.to_le_bytes())?; // PCM fmt chunk size
    f.write_all(&1u16.to_le_bytes())?; // audio format = PCM
    f.write_all(&channels.to_le_bytes())?;
    f.write_all(&sr.to_le_bytes())?;
    f.write_all(&byte_rate.to_le_bytes())?;
    f.write_all(&block_align.to_le_bytes())?;
    f.write_all(&bits.to_le_bytes())?;
    f.write_all(b"data")?;
    f.write_all(&data_len.to_le_bytes())?;
    for &s in samples {
        let v = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        f.write_all(&v.to_le_bytes())?;
    }
    Ok(())
}
