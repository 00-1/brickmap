//! GG1 generative **music** (full-port phase 4 — audio) — the deterministic *score generator* from
//! `synth.js`, re-implemented in Rust and **proven against the `synth_score_*.json` goldens**. Unlike
//! the SFX, the music's per-step note schedule IS a vector contract: each scene yields a sequence of
//! 16th-note steps, every step a list of event tokens (`p:`pad / `b:`bass / `l:`lead / `d:`drum), and
//! a correct re-impl reproduces those byte-for-byte.
//!
//! Ported: the `mulberry32` PRNG (exact u32 arithmetic), `hashStr`/`phraseSeed` seeding, the modal
//! harmony with voice-leading, Euclidean drum grooves, and the density-gated Markov-ish lead walk —
//! everything `stepEvents` needs. The *synthesis* of these tokens (instrument patches, FDN reverb) is
//! the perceptual, by-ear half; this module is the provable note schedule that drives it.

/// `mulberry32` — the exact PRNG `synth.js` seeds per phrase. Reproduces the JS `f64` stream via u32
/// wrapping arithmetic (`Math.imul` → `wrapping_mul`, `>>>` → `>>`).
struct Rng {
    a: u32,
}
impl Rng {
    fn new(seed: u32) -> Rng {
        Rng { a: seed }
    }
    fn next(&mut self) -> f64 {
        self.a = self.a.wrapping_add(0x6D2B_79F5);
        let a = self.a;
        let mut t = (a ^ (a >> 15)).wrapping_mul(1 | a);
        t = t.wrapping_add((t ^ (t >> 7)).wrapping_mul(61 | t)) ^ t;
        ((t ^ (t >> 14)) as f64) / 4_294_967_296.0
    }
}

/// FNV-1a over the scene name → the music seed (matches `synth.js` `hashStr`).
fn hash_str(s: &str) -> u32 {
    let mut h: u32 = 2_166_136_261;
    for b in s.bytes() {
        h ^= b as u32;
        h = h.wrapping_mul(16_777_619);
    }
    h
}

/// A stable per-phrase seed (matches `synth.js` `phraseSeed`).
fn phrase_seed(seed: u32, phrase: i32) -> u32 {
    let mut h = seed ^ (phrase.wrapping_add(1) as u32).wrapping_mul(2_654_435_761);
    h ^= h >> 15;
    h = h.wrapping_mul(2_246_822_519);
    h ^= h >> 13;
    if h == 0 {
        1
    } else {
        h
    }
}

/// The diatonic modes (semitone offsets), keyed by `synth.js`'s mode names.
fn mode_scale(mode: &str) -> &'static [i32] {
    match mode {
        "ionian" | "major" => &[0, 2, 4, 5, 7, 9, 11],
        "dorian" => &[0, 2, 3, 5, 7, 9, 10],
        "phrygian" => &[0, 1, 3, 5, 7, 8, 10],
        "lydian" => &[0, 2, 4, 6, 7, 9, 11],
        "mixolydian" => &[0, 2, 4, 5, 7, 9, 10],
        "aeolian" | "minor" => &[0, 2, 3, 5, 7, 8, 10],
        "pentatonic" => &[0, 2, 4, 7, 9],
        "pentminor" => &[0, 3, 5, 7, 10],
        _ => &[0, 2, 4, 5, 7, 9, 11], // major
    }
}

const TRIAD: [i32; 3] = [0, 2, 4];

/// Scale degree → MIDI (octave-aware; degrees beyond the scale wrap octaves), matching `degToMidi`.
fn deg_to_midi(root: i32, mode: &str, degree: i32, oct: i32) -> i32 {
    let sc = mode_scale(mode);
    let len = sc.len() as i32;
    let o = (degree as f64 / len as f64).floor() as i32;
    let idx = (((degree % len) + len) % len) as usize;
    root + (oct + o) * 12 + sc[idx]
}

fn chord_midi(root: i32, mode: &str, deg: i32, oct: i32) -> [i32; 3] {
    [
        deg_to_midi(root, mode, deg + TRIAD[0], oct),
        deg_to_midi(root, mode, deg + TRIAD[1], oct),
        deg_to_midi(root, mode, deg + TRIAD[2], oct),
    ]
}

fn bass_midi(root: i32, mode: &str, deg: i32, oct: i32) -> i32 {
    deg_to_midi(root, mode, deg, oct)
}

/// Move each previous voice to the nearest tone of the new chord (≤6 semitones) — `voiceLead`.
fn voice_lead(prev: &[i32], chord: &[i32]) -> Vec<i32> {
    let pcs: Vec<i32> = chord.iter().map(|m| m.rem_euclid(12)).collect();
    prev.iter()
        .map(|&m| {
            let (mut best, mut best_d) = (m, 99);
            for &pc in &pcs {
                let mut cand = m + ((((pc - (m % 12)) % 12) + 12) % 12);
                if cand - m > 6 {
                    cand -= 12;
                }
                let d = (cand - m).abs();
                if d < best_d {
                    best_d = d;
                    best = cand;
                }
            }
            best
        })
        .collect()
}

/// One realised chord in the progression.
struct Chord {
    chord: [i32; 3],
    voiced: Vec<i32>,
    bass: i32,
}

/// Realise a progression into per-chord voicings (matches `harmonyFor`). `pad_oct`/`bass_oct`
/// default to `0` / `-2`.
fn harmony_for(root: i32, mode: &str, prog: &[i32]) -> Vec<Chord> {
    let (pad_oct, bass_oct) = (0, -2);
    let mut voiced: Option<Vec<i32>> = None;
    let mut out = Vec::new();
    for &deg in prog {
        let chord = chord_midi(root, mode, deg, pad_oct);
        voiced = Some(match voiced {
            Some(v) => voice_lead(&v, &chord),
            None => chord.to_vec(),
        });
        out.push(Chord {
            chord,
            voiced: voiced.clone().unwrap(),
            bass: bass_midi(root, mode, deg, bass_oct),
        });
    }
    out
}

/// Euclid(k,n): spread k onsets evenly over n steps — `euclid`.
fn euclid(k: i32, n: i32) -> Vec<u8> {
    let n = n.max(1);
    let k = k.clamp(0, n);
    let mut out = vec![0u8; n as usize];
    let mut bucket = 0;
    for o in out.iter_mut() {
        bucket += k;
        if bucket >= n {
            bucket -= n;
            *o = 1;
        }
    }
    out
}

fn rotate(a: &[u8], by: i32) -> Vec<u8> {
    let n = a.len() as i32;
    if n == 0 {
        return a.to_vec();
    }
    let by = (((by % n) + n) % n) as usize;
    a[by..].iter().chain(a[..by].iter()).copied().collect()
}

/// A weighted stepwise interval (a Markov-ish walk) — `leadStep`.
fn lead_step(rnd: &mut Rng) -> i32 {
    let r = rnd.next();
    if r < 0.34 {
        -1
    } else if r < 0.68 {
        1
    } else if r < 0.80 {
        -2
    } else if r < 0.92 {
        2
    } else if r < 0.97 {
        0
    } else if r < 0.985 {
        -3
    } else {
        3
    }
}

/// Snap a note to the nearest chord tone (≤ a tritone) — `nearestChordTone`.
fn nearest_chord_tone(midi: i32, chord: &[i32]) -> i32 {
    let pcs: Vec<i32> = chord.iter().map(|m| m.rem_euclid(12)).collect();
    let (mut best, mut best_d) = (midi, 99);
    for &pc in &pcs {
        let mut cand = midi + ((((pc - (midi % 12)) % 12) + 12) % 12);
        if cand - midi > 6 {
            cand -= 12;
        }
        let d = (cand - midi).abs();
        if d < best_d {
            best_d = d;
            best = cand;
        }
    }
    best
}

/// A scene's score-relevant config (the subset of `CONTEXTS` that affects the note schedule — tempo,
/// reverb, swing, wobble and patch *names* don't change the tokens).
struct Scene {
    root: i32,
    mode: &'static str,
    progression: &'static [i32],
    density: f64,
    lead_oct: i32,
    kick_k: i32,
    hat_k: i32,
    snare_k: i32,
    lead_k: i32,
}

/// The 12 launcher styles (`STYLE_IDS`).
pub const STYLE_IDS: [&str; 12] = [
    "menu",
    "arena",
    "lofi",
    "ambient",
    "chiptune",
    "synthwave",
    "dubstep",
    "dnb",
    "bigroom",
    "boss8bit",
    "tropical",
    "techno",
];

fn context(name: &str) -> Option<Scene> {
    // (root, mode, progression, density, leadOct, kickK, hatK, snareK, leadK) — from CONTEXTS.
    let s = |root, mode, progression, density, lead_oct, kick_k, hat_k, snare_k, lead_k| Scene {
        root,
        mode,
        progression,
        density,
        lead_oct,
        kick_k,
        hat_k,
        snare_k,
        lead_k,
    };
    Some(match name {
        "menu" => s(60, "ionian", &[0, 3, 4, 0], 0.34, 2, 4, 6, 2, 6),
        "arena" => s(45, "phrygian", &[0, 5, 6, 4], 0.62, 1, 6, 12, 2, 9),
        "lofi" => s(55, "mixolydian", &[0, 5, 3, 0], 0.24, 2, 1, 3, 0, 4),
        "ambient" => s(55, "lydian", &[0, 3, 0, 4], 0.14, 1, 0, 0, 0, 3),
        "chiptune" => s(60, "pentatonic", &[0, 4, 5, 3], 0.60, 2, 4, 8, 2, 11),
        "synthwave" => s(50, "aeolian", &[0, 5, 3, 4], 0.42, 2, 4, 8, 4, 7),
        "dubstep" => s(36, "pentminor", &[0, 0, 5, 3], 0.40, 1, 4, 6, 2, 6),
        "dnb" => s(43, "aeolian", &[0, 5, 3, 4], 0.44, 1, 4, 10, 6, 8),
        "bigroom" => s(57, "lydian", &[0, 3, 4, 5], 0.56, 2, 4, 10, 4, 8),
        "boss8bit" => s(48, "phrygian", &[0, 1, 0, 5], 0.50, 1, 4, 6, 4, 9),
        "tropical" => s(57, "mixolydian", &[0, 4, 5, 2], 0.40, 2, 3, 8, 2, 7),
        "techno" => s(45, "aeolian", &[0, 0, 5, 5], 0.34, 1, 4, 8, 0, 5),
        _ => return None,
    })
}

/// The scheduler's runtime spec (matches `normalizeMusic`, score-relevant fields).
struct Spec {
    seed: u32,
    root: i32,
    mode: &'static str,
    harmony: Vec<Chord>,
    density: f64,
    lead_oct: i32,
    kick: Vec<u8>,
    hat: Vec<u8>,
    snare: Vec<u8>,
    lead_euclid: Vec<u8>,
}

fn normalize(name: &str, scene: &Scene) -> Spec {
    Spec {
        seed: {
            let h = hash_str(name);
            if h == 0 {
                1
            } else {
                h
            }
        },
        root: scene.root,
        mode: scene.mode,
        harmony: harmony_for(scene.root, scene.mode, scene.progression),
        density: scene.density,
        lead_oct: scene.lead_oct,
        kick: euclid(scene.kick_k, 16),
        hat: euclid(scene.hat_k, 16),
        snare: rotate(&euclid(scene.snare_k, 16), 4),
        lead_euclid: euclid(scene.lead_k, 16),
    }
}

fn density_at(spec: &Spec, step: usize) -> f64 {
    let phrase_len = 16 * spec.harmony.len();
    let pos = (step % phrase_len) as f64 / phrase_len as f64;
    spec.density * (0.55 + 0.6 * pos)
}

/// The role of a scheduled note (its instrument lane).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Pad,
    Bass,
    Lead,
    Drum,
}

/// A scheduled note within a step: its lane + pitch (MIDI; ignored for drums) + drum piece.
#[derive(Clone, Debug, PartialEq)]
pub struct Voiced {
    pub role: Role,
    pub midi: i32,
    pub piece: &'static str,
}

/// The events on one 16th-note step, structured — the exact order + gating of `stepEvents`
/// (intensity 0). The score tokens and the audio renderer both derive from this single source.
fn step_voiced(spec: &Spec, step: usize, rnd: &mut Rng, deg: &mut i32) -> Vec<Voiced> {
    let s = step % 16;
    let bar = step / 16;
    let chord = &spec.harmony[bar % spec.harmony.len()];
    let dens = density_at(spec, step).min(1.0);
    let mut ev = Vec::new();
    let v = |role, midi, piece| Voiced { role, midi, piece };
    if s == 0 {
        for &m in &chord.voiced {
            ev.push(v(Role::Pad, m, ""));
        }
    }
    if s == 0 || s == 8 {
        ev.push(v(Role::Bass, chord.bass, ""));
    }
    if spec.lead_euclid[s] == 1 && rnd.next() < dens {
        *deg += lead_step(rnd);
        *deg = (*deg).clamp(-6, 6);
        let mut midi = deg_to_midi(spec.root, spec.mode, *deg, spec.lead_oct);
        if s.is_multiple_of(4) {
            midi = nearest_chord_tone(midi, &chord.chord);
        }
        ev.push(v(Role::Lead, midi, ""));
    }
    if spec.kick[s] == 1 {
        ev.push(v(Role::Drum, 0, "kick"));
    }
    if spec.hat[s] == 1 {
        ev.push(v(Role::Drum, 0, "hat"));
    }
    if !spec.snare.is_empty() && spec.snare[s] == 1 {
        ev.push(v(Role::Drum, 0, "snare"));
    }
    ev
}

/// The score token for a voiced note (`p:`/`b:`/`l:`/`d:`).
fn token(v: &Voiced) -> String {
    match v.role {
        Role::Pad => format!("p:{}", v.midi),
        Role::Bass => format!("b:{}", v.midi),
        Role::Lead => format!("l:{}", v.midi),
        Role::Drum => format!("d:{}", v.piece),
    }
}

/// The structured per-step voiced events for a scene — the audio renderer's input. Same generation
/// as [`score`] (the goldens cover it): reseeds the PRNG per phrase from `phraseSeed`, the melodic
/// `deg` state persists. `None` for an unknown scene.
pub fn voiced_score(name: &str, steps: usize) -> Option<Vec<Vec<Voiced>>> {
    let scene = context(name)?;
    let spec = normalize(name, &scene);
    let phrase_len = 16 * spec.harmony.len();
    let mut rnd = Rng::new(0);
    let mut phrase: i32 = -1;
    let mut deg: i32 = 0;
    let mut out = Vec::with_capacity(steps);
    for step in 0..steps {
        let ph = (step / phrase_len) as i32;
        if ph != phrase {
            phrase = ph;
            rnd = Rng::new(phrase_seed(spec.seed, phrase));
        }
        out.push(step_voiced(&spec, step, &mut rnd, &mut deg));
    }
    Some(out)
}

/// Generate a scene's score: the first `steps` 16th-notes as token lists (`p:`/`b:`/`l:`/`d:`).
/// Returns `None` for an unknown scene.
pub fn score(name: &str, steps: usize) -> Option<Vec<Vec<String>>> {
    Some(
        voiced_score(name, steps)?
            .iter()
            .map(|step| step.iter().map(token).collect())
            .collect(),
    )
}

/// A scene's tempo in BPM (from `CONTEXTS`). The renderer's 16th-note grid step is `(60/tempo)/4` s.
pub fn tempo_of(name: &str) -> Option<f64> {
    Some(match name {
        "menu" => 96.0,
        "arena" => 124.0,
        "lofi" => 78.0,
        "ambient" => 60.0,
        "chiptune" => 150.0,
        "synthwave" => 112.0,
        "dubstep" => 140.0,
        "dnb" => 174.0,
        "bigroom" => 128.0,
        "boss8bit" => 140.0,
        "tropical" => 104.0,
        "techno" => 126.0,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each scene's 32-step score reproduces the committed `synth_score_*.json` golden exactly.
    #[test]
    fn every_scene_score_matches_its_golden() {
        for name in STYLE_IDS {
            let golden_json = std::fs::read_to_string(format!(
                concat!(env!("CARGO_MANIFEST_DIR"), "/data/gg1/synth-scores/{}.json"),
                name
            ))
            .unwrap_or_else(|e| panic!("read golden {name}: {e}"));
            let golden: Vec<Vec<String>> = serde_json::from_str(&golden_json).expect("golden json");
            let got = score(name, 32).expect("known scene");
            assert_eq!(got, golden, "score for `{name}` drifted from its golden");
        }
    }

    #[test]
    fn scores_are_deterministic_and_mutually_distinct() {
        // Same seed → byte-identical (re-derivation is stable).
        assert_eq!(score("arena", 32), score("arena", 32));
        // No two scenes collapse to the same notes (the "samey" guard).
        let scores: Vec<_> = STYLE_IDS.iter().map(|n| score(n, 32).unwrap()).collect();
        for i in 0..scores.len() {
            for j in (i + 1)..scores.len() {
                assert_ne!(
                    scores[i], scores[j],
                    "{} and {} render identically",
                    STYLE_IDS[i], STYLE_IDS[j]
                );
            }
        }
    }

    #[test]
    fn unknown_scene_is_none() {
        assert!(score("nope", 8).is_none());
    }
}
