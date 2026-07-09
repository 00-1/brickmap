# Pacing analysis — playtest by numbers (2026-06-16)

> Measured by driving the **real loop** (`App::headless` + `run_frame`, the D11 seam) under
> pure autopilot (given routines only), 6 seeds × 60 sim-min at 60 fps; income window
> minutes 10–60 (stable, <15% half-window drift). Probe preserved at
> [`probes/pacing_probe.rs`](probes/pacing_probe.rs) (648 lines; a `cfg(test)` module +
> one-line lib hook). Measured at `68c5ff7`-era main (pre-G18). Consequences dispatched as
> the **G19 economy-truing** milestone + an urgent bug directive.

## Measured (means across seeds; spreads noted)

| Metric | Result | Envelope | Verdict |
|---|---|---|---|
| First gated discovery | **0.9 min** (0.3–1.8) | 2–6 min | too fast |
| First comprehension | **15.3 min** (6.2–26.3) | 5–15 min | shape wrong — 4× seed roulette (Relics-first seeds 25–26 min) |
| Shard income | **11.5 shards/min**, 15.9 yield/min (12.7–18.0 = 1.41× spread) | <3× outliers | good |
| Rarity mix | ≈85/13/2 (as designed) | — | good; but **rares gate nothing** |
| Per-domain yield | ~3.2/min, uniform across 5 domains | — | uniform; "rare stratum" feel inverted (REL/SIG biggest HUD numbers via yield bases) |
| Faculties L1 / all-maxed | **0.3–0.9 min / 33–42 min** | ~10 min / 1–2 h | 10–30× too fast |
| Deepest block (runfoot) | **87 min** | multiple hours | too fast + wrong: **no Signals/Rites/Records-gated block exists** |
| Full ladder | **2.03 h** | a real arc | too short; after it, all income = dead numbers |
| Per-block discovery spread | seek: 0.3 min (seed 42) vs 54.8/63.1 min (2024/99999) | <3× | >3× outlier (colossus label table is RunFoot×3-heavy) |

## Bugs found (wiring, not tuning — babysitter-verified in code)

1. **`arrived_at` can never fire in flight** (`lib.rs:1718`): full-3D distance, `ARRIVE_RADIUS=12`,
   ship at `ground + CRUISE_HEIGHT(22)`, sites at ~ground+1.8 → vertical offset ≈20 alone
   exceeds the radius; Seek's turn clamp also forces a ≥17 u orbit. **Measured: 0 automated
   expeditions/hour on every seed.** `on-arrive`, automated `run(foot)`, and the G8c/G17
   expedition-from-air are dead features. Fix: horizontal distance + radius 20.
2. **Seek+collect deadlock:** under `seek` nav, once every site in the scan cone is known,
   no scan event fires → on-scan `collect` never runs → the ship orbits one uncollected
   site forever; economy flatlines (~+14 yield/h vs ~950 under drift). Fix: scan re-hits
   known-but-uncollected, or collect dispatches on arrival.

## Structural findings (routed to Archive briefs)

- **The expedition is never economically rational** (best handshake ≈ 80–96 shards/h vs
  ~700/h pure ship drift; running one *loses* income). The walker needs on-foot-*only*
  value → routed to **G21 sensing ladder** (worn/⟦erased⟧ recovery on foot) + G20.
- **Strata data has no sink** (research fills only from shards; strata = display +
  `when()` fuel) → candidate sink in **G22 proto-language** (cognate work consumes data).
- **Rarity gates nothing** — the human's explicit "rarer blocks demand rarer shards" was
  half-implemented (larger totals only) → rare-count requirements in **G19**.
- **Vocabulary gap:** tiers Records/Rites/Signals gate zero blocks → filled naturally by
  Archive milestones (G21 sensing faculties = Signals-tier targets; commitment recorded).
- Faculties cap at ~40 min → after ~2 h nothing consumes income at all (G19 costs + the
  Archive vocabulary extend the arc to ~5–6 h).

## Caveats

Autopilot is a floor for attentive play, a ceiling for idle; biome-path drives the
low-income seed (99999); headless measures mechanics, not felt pacing — the human's play
pass remains the final word. Pre-G18 baseline (erosion will trim income slightly).
