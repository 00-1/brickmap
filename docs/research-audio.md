# Research — generative audio, drone theory & psychoacoustics (E16 evolution)

> 2026-06-11 research pass (web; composer primary texts, DSP literature, vendor/GDC
> sources). How the doom-drone grows from a reactive pad into a *composed generative
> system* — on the existing dependency-free Rust synth, within budget, including the
> phone-speaker problem. Feeds E16 follow-ups and [`game-depth.md`](game-depth.md)
> (per-stratum identity pairs with G9/G11).

## 1. Generative structure — incommensurate cycles (Eno, verified primary)

Eno's *Music for Airports* mechanism, his own numbers: vocal loops of **23½ s, 25⅞ s,
~29 15/16 s** — "incommensurable… not likely to come back into sync again"; six fixed
elements, all complexity from shifting alignment; *Discreet Music* = two harmonically
compatible phrases of unequal length + a long echo system. The two rules that make it
music, not noodling: (a) **the pitch set is pre-curated so every vertical alignment is
consonant** (the "2/1" material is one Db maj7add9 set — sequencing can then be left
entirely to drift); (b) **the process is perceptible** (Reich: "I want to be able to hear
the process happening"; gradual like a minute hand). Long-form shape comes from a *slower
meta-layer*, never the note process: Reflection's rules change with time of day (Chilvers);
Longplayer's six staggered transpositions.

**Application (the cheapest deep win in the whole audio plan):** per-voice swell/gate LFOs
with seeded mutually-incommensurate periods (≈23/31/41/53 s class), one near-equal pair
(41 vs 41.7 s) drifting through alignment over ~30 min as the perceptible process, plus
1–2 session-scale meta-LFOs (10–40 min) reweighting density/brightness/key-color. Cost:
~4 phase accumulators. *(Sources: Eno 1996 talk [full text verified]; Discreet Music liner
notes [verified]; Reich "Music as a Gradual Process" [full text verified]; longplayer.org;
Reverb Machine reconstructions. Caveat: the oft-cited "seven loops" conflates Tamm's seven
pitches; "76 notes" wind-chime framing is apocryphal — don't quote either.)*

## 2. Grief musicology — the lamento bass (the flagship feature)

The **descending minor tetrachord** (8–♭7–♭6–5) as ground-bass ostinato is the codified
Western grief emblem (Rosand: "an emblem of lament" — by the 1640s the bass line *alone*
signified it; Purcell's Dido = the chromatic variant, a 5-bar ground repeated 11 times with
the voice deliberately out of phase). The ♭2→1 sigh (*pianto*) is the oldest grief figure —
our Phrygian ♭2 voice already *is* it. Why it reads as grief (Juslin & Laukka meta-analysis,
104+41 studies: speech and music share one acoustic code): sadness = slow, quiet, low,
small intervals, **falling contours**.

**Application:** the sub voice's static root becomes a 4-step descending ostinato
(root→♭7→♭6→5, ~10–12 s/step ≈ 45 s period, per-seed transposition, chromatic variant for
darker strata), surfacing/receding on one of §1's long cycles; body+fifth hold the tonic so
the ground moves *beneath* a held world (the Dido texture); the ♭2 resolves downward at
phrase ends. ~20 lines of Rust. *(Sources: Rosand 1979; Wikipedia Lament bass/Dido's
Lament/Pianto + Monelle; Juslin & Laukka 2003 [PDF]; Shea EMR 2020 corpus study; Huron
2011.)*

## 3. Tuning & spectrum (Young, Grisey, Lilja)

- **Just intonation = the lock/beat axis**: a true 3:2 fifth phase-locks (eternity/stasis);
  cents of detune = beat rate in Hz (10 cents at 55 Hz ≈ 0.32 Hz free slow movement). Map
  game intensity to lock↔beat (rest = locked/just; flight/finds push detune outward) —
  more *felt* than volume, costs nothing.
- **Spectral fusion (Grisey *Partiels*)**: place the stack's voices on partials 2/3/5/7 of
  the sub with staggered, amplitude-scaled entries → the stack reads as **one enormous
  instrument**, not an organ chord; let the ♭2 sit a few cents inharmonic as the
  grief-colored impurity; the filter sweep = the spectral-centroid arc.
- **Lilja's power-chord acoustics**: distortion intermodulation of root+fifth lands on the
  harmonic series of a sub-octave virtual fundamental ("one huge note"); thirds land
  off-grid (mud). → **Waveshape sub+fifth jointly; keep the ♭2 outside the shaper**
  (mix after, faint) or it intermodulates into garbage. We may already be half-doing this
  — verify the synth's shaper topology.
*(Sources: Gann on Young/WTP; Dream House docs; Hasegawa on Grisey; Lilja diss. 2009;
Sunn O)))/Dopesmoker practice notes [A1≈55 Hz / C standard; "never identical twice" —
jitter each cycle from the seed].)*

## 4. The phone-speaker problem (weak-hardware audio, priority fix)

Phone speakers roll off below ~300–500 Hz: **our sub voice is literally absent on
phones** — and equal-loudness means low frequencies need tens of dB more SPL even when
reproducible. The fix is the **missing fundamental** (virtual bass): the brain infers the
fundamental from harmonics 2–5. Since we *synthesize* the sub, skip all analysis: add a
companion voice playing harmonics 2–5 of the current sub note (lament transposition
included), band-passed ~300 Hz–1.5 kHz, true sub attenuated in phone mode (platform
default web/mobile; or always mix a quiet harmonic shadow — inaudible next to a real sub,
carries the pitch on phones). This is the difference between the game *having* a
soundtrack on phones or not. Watch upward masking: carve/duck the drone's low-mids where
the console voice lives. *(Sources: Audiokinetic phone-speaker measurements; ISO 226
contours [~values]; virtual-bass literature (Oo & Gan ATSR, MaxxBass lineage); auditory
masking refs.)*

## 5. Adaptive craft (the shipped systems)

Vertical layering is the only sane model for a continuous drone (our voices = stems);
horizontal logic applies only at harmonic level (change lament steps/transpositions at
swell troughs). From the documented systems: **Spore** — "composing in probabilities,"
tiny musical responses to *player gestures* (console clicks = our editor clicks);
**No Man's Sky Pulse** — "instruments" with eligibility rules + value ranges per context;
the band's material "dismantled into the system"; **Mini Metro (Vreeland)** — serialism +
sonification: game data indexes into *authored* pitch/duration series, never raw mapping;
**Ape Out** — discrete accents (kill-cymbals, screen-spatialized) over a continuous bed +
density/similarity-rated pattern stepping. Craft rules: one-pole smoothing on every
parameter (~10–50 ms — the audibility floor for zipper noise), **asymmetric envelopes**
(fast attack on threat/dive, slow lingering release), hysteresis + minimum-dwell on any
discrete mode change, reactivity *felt not noticed*. Proteus/Kanaga: make part of the
drone positional — the weather term loudest where the storm is; a find's shimmer localized
at the find. *(Sources: GDC Spore 2008/NMS 2017; Vreeland GDC 2018 [the "Jamie Churchman"
attribution sometimes seen is a confabulation]; Ape Out MusicTech interview; Wwise docs;
CCRMA one-pole.)*

## 6. Per-stratum audio identity (pairs with G9/G11)

Earcon research (Blattner 1989; Brewster's experimental guidelines): **timbre is the
strongest distinguisher** (grossly different, multi-harmonic); pitch alone never absolute
(pair register with another parameter, 2–3 octave spreads); rhythm distinguishes subtypes;
keep sets ≈5–7; **family structure is what makes them learnable**. Melody memory is
**contour-first** (Dowling) — a stable 3–8-note up-down silhouette survives timbre/register
variation (the Zelda-jingle lesson). Burtt's R2-D2 method: emotion lives in **pitch contour
+ rhythm, not phonemes** (rise = query, fall = loss/finality, bell rise-fall = warm
acknowledgment, short-low-flat = prohibition — Fernald's verified contour→intent set).

**Application:** one shared two-note family motive; five stratum variants = interval vs
drone root × timbre signature (Records unison/sine/deep-reverb → Signals tritone/bright/
jittery), **fixed per session** (consistency = learnability); played at finds (the Ape Out
accent role), quoted by a console "prosody voice" (BP-filtered, contours from seeded
templates, pitches drawn from the drone's tonality — the dead machine answering in its own
voice, P15/P18 of the games research); bias the ambient drone's partial emphasis by the
local stratum so players *hear where they are*. Sonification rules: pitch for ordered data
(consistent polarity, log-scaled), timbre for categories. *(Sources: Blattner et al. 1989;
Brewster HCI'95 guidelines [PDF]; Dowling 1978; Gaver SonicFinder; Burtt/designingsound;
Fernald 1989; Sonification Handbook ch. 15.)*

## 7. The DSP cookbook (verified parameters, dependency-free)

- **FDN tune-up** (the highest-value upgrade if absent): 8 lines, mutually-prime
  log-spread lengths ~30–150 ms; **Householder feedback** (one sum + N subs — gentler than
  Hadamard in the loop; Hadamard butterflies in 3–4 input-diffuser stages, ~10/20/30/50 ms,
  Schroeder allpass coeff ≈0.7); per-line Jot gain `g=10^(−3·M·T/T60)` + one-pole damping
  so **lows decay ~2× longer than highs**; **±2-sample ~1 Hz modulation on half the lines**
  (makes N=8 sound like N=16, kills metallic modes). *Vastness recipe:* pre-delay
  80–120 ms, sparse/no earlies, long dark tail, low direct-to-reverb for "far away";
  scale pre-delay/T60 by stratum depth.
- **Filters:** Simper trapezoidal SVF (~9 ops, stable under audio-rate sweeps — replaces
  any recalculated biquad; free BP output for the console voice). One-pole everywhere else.
- **Oscillators:** naive is *fine* at drone pitches — fundamentals ≤415 Hz need no
  anti-aliasing at 44.1 kHz (Välimäki); worst aliases at 55–80 Hz fold in at −50…−60 dB
  under a dark LPF'd reverb-washed mix. Skip PolyBLEP; spend the CPU on FDN modulation +
  the phone voice.
- **Detune:** fixed asymmetric offsets (the supersaw lesson — avoid coincident beat rates),
  specified as beat-Hz not cents; ≤5 cents shimmer / 5–20 thick.
- **Dynamics:** no lookahead needed — mix gain-staged to |x|≲0.5 → existing waveshaper →
  FDN → cubic soft clip (`1.5a(1−a²/3)`) at −0.5 dB. Asymmetric shapers need a DC blocker
  (`y=x−x₁+0.995y₁`).
- **Budget:** the whole chain ≈ <100 ops/sample — trivially real-time native and
  comfortable in a Web Audio worklet.
*(Sources: Jot & Chaigne 1991; JOS PASP FDN chapters; Schlecht & Habets 2016; Signalsmith
reverb example code [verified constants]; Valhalla diffusion notes; KVR FDN-modulation
consensus; Simper SvfLinearTrapOptimised2 [algorithm verified vs implementations]; Szabo
supersaw thesis [verified constants]; Välimäki & Huovilainen 2007 + JASA 2012; Signalsmith
limiter writeups.)*

## Top-10 (ranked)

1. **Lament-bass ostinato** in the sub (≈45 s period, seeded, surfacing on a long cycle).
2. **Phone-mode virtual bass** (harmonics 2–5, 300 Hz–1.5 kHz band) — soundtrack-or-not
   on phones.
3. **Incommensurate per-voice cycles** + one drifting near-equal pair + session meta-LFOs.
4. **Per-stratum earcon family** + console prosody voice in the drone's tonality.
5. **FDN tune-up + vastness configuration** (pre-delay, dark long tail, line modulation).
6. **Smoothing/asymmetric-envelope/hysteresis pass** over every existing reactive mapping.
7. **JI lock↔beat axis** mapped to game intensity.
8. **Spectral fusion** of the stack (partials 2/3/5/7, staggered entries); ♭2 outside the
   waveshaper (verify topology).
9. **Doom rests** — near-silent 5–15 s gaps on a long cycle; "never identical twice"
   jitter per cycle.
10. **Skip band-limited oscillators** (documented rationale) — spend nothing there.
