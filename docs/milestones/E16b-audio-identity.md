# E16b — Audio identity: the lament bass, the phone voice & per-stratum earcons

> **Status: ready to build — dispatch after G23 lands** (serialize on the trunk; E16b
> touches `audio.rs`/`audio_native.rs`/`lib.rs` wiring only). The 2026-06-11 audio research
> ([`../research-audio.md`](research-audio.md)) cashed at its top of the ranked list: the
> drone grows from a reactive pad into a **composed grief signature** — and gains a
> soundtrack on phones at all (the sub is literally absent below the phone-speaker
> rolloff today). All on the existing dependency-free game-side synth
> (`crates/scraped-again/src/audio.rs`); **no engine change**; every new behavior behind a
> **runtime toggle** (the D6 norm) so cost and taste are both A/B-able. The human tunes by
> ear later — build to the research's verified parameters, test what's testable headless
> (determinism, boundedness, band energy), and record listening notes for the eye/ear pass.

## Goal · Demonstrable outcome · What it de-risks

- **Goal.** Research Top-10 items 1–6: **(1)** lament-bass ostinato in the sub; **(2)**
  phone-mode virtual bass; **(3)** incommensurate per-voice cycles + session meta-LFOs;
  **(4)** a per-stratum earcon family at finds + stratum-biased drone color; **(5)** FDN
  tune-up + vastness; **(6)** the smoothing/asymmetric-envelope/hysteresis craft pass.
- **Demonstrable outcome.** Headless-rendered audio (the existing `render(seed, sr, secs)`
  seam) shows: the sub's fundamental *steps* through the descending tetrachord on its
  ~45 s period (Goertzel energy tracks the step sequence); phone mode carries the pitch in
  the 300 Hz–1.5 kHz band with the true sub attenuated; two same-seed renders are
  identical, two seeds differ; a find event triggers a stratum's earcon (contour + timbre
  variant of one family motive); no parameter jump produces a zipper discontinuity.
- **De-risks.** Whether the audio layer can carry the game's melancholy identity without
  any readable content (it is the *other* half of the archive's voice); the phone target's
  viability (weak-hardware-first applies to sound too).

## Scope

**In (each item toggleable; sensible defaults on):**
1. **Lament-bass ostinato (the flagship).** The sub voice's static root becomes the
   descending minor tetrachord (root→♭7→♭6→5), ~10–12 s per step (≈45 s period), per-seed
   transposition; a **chromatic variant for darker strata** (bias by the local stratum
   signal, see item 4); the ostinato **surfaces and recedes** on one of item 3's long
   cycles (it should not always be foreground); body+fifth **hold the tonic** so the
   ground moves beneath a held world (the Dido texture); the ♭2 voice resolves downward at
   phrase ends (the pianto). Change steps only at swell troughs (horizontal logic at
   harmonic level only).
2. **Phone virtual bass.** A companion voice playing **harmonics 2–5** of the current sub
   note (lament transposition included), band-passed **~300 Hz–1.5 kHz**; in phone mode
   (platform default: web/mobile) the true sub attenuates and the harmonic voice carries
   the fundamental percept; native desktop keeps the real sub (harmonic shadow quiet or
   off). Duck the drone's low-mids where the earcon/console accents live (upward masking).
3. **Incommensurate cycles + meta-layer.** Per-voice swell/gate LFOs with seeded mutually
   incommensurate periods (≈23/31/41/53 s class), **one near-equal pair** (e.g. 41 vs
   41.7 s) drifting through alignment over ~30 min (the perceptible process), plus 1–2
   session-scale meta-LFOs (10–40 min) reweighting density/brightness/key-color. ~4 phase
   accumulators; replaces/augments the existing per-voice drift where they overlap.
4. **Per-stratum earcons + drone color.** One shared **two-note family motive**; five
   stratum variants = interval-vs-root × timbre signature (Records unison/pure/washed →
   Signals tritone/bright/jittery), **fixed per session** (seeded); played as a localized
   accent at **finds** (collect/discovery events — the Ape Out accent role, volume-ducked
   into the bed); and the ambient drone's **partial emphasis biases by the local stratum**
   so players hear where they are. Event path: a small lock-free event slot beside the
   existing atomic params (native) / per-frame set (web). The console **prosody voice is
   OUT** (v2 — it's a feature, not a variant).
5. **FDN tune-up + vastness.** On the existing FDN: mutually-prime log-spread line lengths
   (~30–150 ms), **Householder feedback**, per-line Jot gain + one-pole damping (lows
   decay ~2× longer than highs), **±2-sample ~1 Hz modulation on half the lines**;
   vastness config: pre-delay 80–120 ms, sparse earlies, long dark tail; scale
   pre-delay/T60 by stratum depth. Verify the shaper topology while in there: **sub+fifth
   jointly shaped, ♭2 outside the shaper** (mix after, faint) per Lilja — fix if not.
6. **The craft pass.** One-pole smoothing (~10–50 ms) on every reactive parameter that
   lacks it; asymmetric envelopes (fast attack on threat/dive, slow release); hysteresis +
   minimum-dwell on any discrete mode change (incl. the new ostinato steps and phone-mode
   switch). Reactivity felt, not noticed.

**Out:** the console prosody voice (v2); JI lock↔beat intensity axis, spectral-fusion
partial restructure, doom rests (banked — items 7–9 of the ranked list; revisit after the
human's ear pass); band-limited oscillators (explicitly skipped per research — document
the rationale in code); Android audio backend (still its own follow-up); any audio
assertion that amounts to "it sounds good" (that's the human's).

## Design sketch

- `audio.rs`: `Ostinato { steps: [f32; 4], step_secs, phase, surface_lfo }` modulating the
  sub voice's root; `VirtualBass { harmonics: [Voice; 4], bp: Svf }` (add the Simper SVF —
  ~9 ops, replaces recalculated biquads where present, free BP output); `Earcon` one-shot
  voice + `EarconEvent { stratum }` atomic slot; cycle/meta LFO phase accumulators; FDN
  feedback matrix → Householder + line modulation. All new constants named, at the top,
  with the research citation in a comment.
- `lib.rs`/`audio_native.rs` wiring: phone-mode flag (platform default + toggle), find
  events → earcon slot, local-stratum signal → drone color + ostinato variant.
- Toggles: extend the existing runtime-toggle surface (one per item 1–5; item 6 is a fix,
  not a feature). Headless `render()` grows optional event/mode injection for tests.
- Budget: stay well under the ~100 ops/sample chain budget; assert voice caps still hold.

## Decisions to resolve (pinned defaults — veto via the channel)

1. Ostinato period ≈45 s (10–12 s/step), diatonic default, chromatic variant keyed to
   deep-strata presence; surfacing cycle ≈ one of the long LFOs.
2. Phone mode = platform default (web/mobile on, desktop off), overridable by toggle.
3. Earcons only at finds v1 (collect/discovery); family motive fixed per session.
4. FDN goes Householder + modulated (the "N=8 sounds like N=16" recipe).
5. Everything toggleable; defaults on except phone-mode-on-desktop.

## Tests

Determinism (same seed ⇒ identical render; different seeds differ) across all new paths;
boundedness/finiteness with all toggles on (the existing test pattern); ostinato step
sequence + period (Goertzel energy at expected fundamentals over a rendered window);
virtual-bass band energy (in-band ≫ out-of-band; true sub attenuated in phone mode);
earcon determinism + five-variant distinctness (contour/interval assertions on rendered
buffers) + triggering via the event slot; no-zipper (bounded sample-to-sample delta under
worst-case parameter steps); FDN stability (impulse energy decays; no line blows up);
voice-cap and ops-budget sanity; toggles idle at ~zero cost when off (op-count or timing
proxy); golden voxel-hash untouched (audio-only change); four-way CI; boundary intact
(no `bm-*` diffs); roadmap E16b + brief as-built with **listening notes** (what to listen
for, per item, for the human's ear pass).

## Acceptance checklist

- [ ] Lament ostinato (steps at troughs, tonic held, pianto resolution, surfacing cycle,
      chromatic deep-strata variant) behind a toggle; step/period asserted headless.
- [ ] Phone virtual bass (harmonics 2–5, 300 Hz–1.5 kHz, platform-default mode, sub
      attenuated) behind a toggle; band-energy asserted.
- [ ] Incommensurate cycles + drifting near-pair + session meta-LFOs.
- [ ] Per-stratum earcon family at finds + stratum-biased drone color; event path wired
      native + web.
- [ ] FDN Householder + modulation + vastness + damping; shaper topology verified (♭2
      outside), fixed if wrong.
- [ ] Craft pass: smoothing/asymmetric envelopes/hysteresis everywhere reactive.
- [ ] All toggleable; determinism/boundedness/no-zipper suites green; four-way CI green;
      boundary intact; roadmap E16b + brief as-built with listening notes.

## Standing notes for the build agent

- **Push per commit.** Splittable: (1) ostinato + cycles + craft pass, (2) phone voice +
  SVF, (3) earcons + stratum color + FDN tune-up + docs. Land each green.
- The synth is dependency-free and must stay that way (no FFT crates — Goertzel in tests
  is a ~10-line function).
- Audio is the one layer the babysitter cannot eyeball headless — the **listening notes**
  in the as-built are the review artifact; write them like you expect to be graded on
  falsifiability ("at 0:45 the sub should have stepped down a whole tone") rather than
  vibes.
