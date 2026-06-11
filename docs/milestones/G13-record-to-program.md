# G13 — Record-to-program ("the console remembers your hands")

> **Status: ready to build — next directive.** The P9 on-ramp from the automation research
> ([`../research-block-language.md`](research-block-language.md)): the strongest bridge from
> *playing by hand* to *authoring routines* for non-programmers — and dead-on-theme (the
> recovered machine recording its lone operator's gestures). The PBD literature pins the
> contract hard: **record literally, generalize manually** — silently-inferred intent is
> what killed every programming-by-demonstration system (Eager, Lieberman post-mortems).
> Game-side (`scraped-again` console + the existing G7 routine model); no engine change.

## Goal · Demonstrable outcome · What it de-risks

- **Goal.** A player who has been acting by hand (clicking blocks / firing the beam) can
  turn the **last N manual actions into a draft routine** with one console action, then
  prune/generalize it in the existing editor — no blank-console authoring required.
- **Demonstrable outcome.** Manually `scan` then `collect` a few finds; open the console;
  pick **"trace → routine"**; a draft routine appears containing the *exact* concrete
  blocks you just ran (identical adjacent repeats folded into `repeat(n)`, nothing else
  inferred); enable it and the agent runs it; open it and generalize a step via the
  existing stepper (e.g. `collect(this)` → `collect(all)`). The trace is visible filling as
  you act (a faint ticker).
- **De-risks.** The "interesting purely through menus" pillar's *on-ramp*: that players who
  never open a blank editor still end up with authored automation — the research's
  single highest-leverage adoption mechanic, and the one most genres get wrong by
  over-inferring.

## Scope

**In:**
- **Action memory.** A rolling per-agent buffer (last ~10) of the player's *manual*
  block-equivalent actions — the blocks they clicked, plus the beam-collects / scans those
  map to (the G4 retrofit already expresses keybind actions as blocks; reuse that mapping).
  Bounded, session-local, cheap.
- **"trace → routine".** A console action that builds a **draft `Routine`** from the memory:
  the *exact* concrete blocks in order, with the **only** transformation being mechanical
  folding of identical adjacent actions into `repeat(n)`. **No dropping "noise", no
  generalization, no inference.** Opens in the existing G7 editor for the player to prune
  and generalize (Victor's create-by-abstracting — the player generalizes via steppers,
  closing the loop PBD couldn't).
- **Playback verifiability (Eager's anticipation):** while a routine runs, **pre-highlight
  the agent's next world target** (the cell/find it's about to act on) so the player can
  confirm intent at a glance / interrupt — reuses the G11 executing-step highlight + the
  existing pick/DDA. *(If the target-prehighlight proves more than a small addition, ship
  trace→routine first and split the prehighlight — but it's the cheap, high-value half of
  the research finding, so try to include it.)*
- **Discoverability.** The console shows the trace **filling as you act** (a faint ticker /
  recent-actions strip — expose-the-tech), so the feature announces itself without a
  tutorial.
- **Glyph-consistent (G12):** the draft's blocks render as glyphs like any routine; the
  ticker too. Trace UI chrome stays minimal-English instrumentation.

**Out:** any *generalization/inference* of the recording (the explicit anti-goal);
cross-agent trace; saving traces as named templates (that's G14 subroutines); new runtime
semantics — a draft is **ordinary G7 routine data**, so it persists/edits/runs through the
existing paths unchanged.

## Design sketch

- `ActionMemory { agent, ring: VecDeque<Block-or-Step, cap 10> }` fed at the manual-action
  sites (console block-click, `T`-collect, beam-collect, manual scan) — the same call sites
  that already dispatch those effects.
- `trace_to_routine(&ActionMemory) -> Routine`: map ring → `Vec<Step>` of `Do(block)`,
  run-length-fold identical adjacent into `Repeat(n, [Do])`, wrap in a `Routine` with a
  default trigger (`continuous`? or `manual/once` — pin in Decisions) and a fresh
  given-style default name. Pure fn → unit-test it directly.
- Editor: the draft enters the **existing** create/edit flow at the cursor; no new editor
  surface. Enable/disable, param-cycle, persist all already work (G7).
- Pre-highlight: the interpreter already knows the next `Do` and its target intent (G11
  surfaces the executing step); extend the world-pick that the act uses to render a faint
  marker one tick ahead.

## Decisions to resolve (pinned defaults — veto via the channel)

1. **Literal record, manual generalize** — non-negotiable (the research's load-bearing
   finding). Only mechanical run-length `repeat`-folding; everything else is the player's.
2. **Draft trigger default:** `continuous` (it starts running once enabled, the common
   intent) — *or* a `manual` trigger so it doesn't auto-fire until the player wires one.
   *Pinned: `continuous`*, since "I did this, now keep doing it" is the modal case; easy to
   change in the editor. Veto if you'd rather drafts be inert until triggered.
3. **Memory size ~10**, per-agent, session-local (not persisted; the dead console's
   working memory — matches G11's stats framing).
4. **Pre-highlight** included if small; else split to a G13b. Trace→routine is the core.

## Tests

- `trace_to_routine` pure: exact-block preservation; run-length fold correctness; no
  spurious generalization (a `scan, collect, scan, collect` does **not** collapse to a loop
  unless adjacent-identical); default trigger/name.
- A recorded then-enabled draft runs via the G7 interpreter with behaviour matching the
  manual actions (interpreter-level, like G7's parity tests).
- Action memory bounds (cap, per-agent isolation, manual-only — autopilot/auto-collect
  actions do **not** pollute the trace).
- Golden voxel-hash + headless render unchanged (console-only; ticker behind the console
  view); `pg=` unaffected (drafts persist via the existing routine path once authored);
  CI green (fmt / clippy -D / tests / wasm); boundary intact.

## Risks & mitigations

- **Inference creep** (the genre's classic failure) → Decision 1 is the acceptance bar:
  the recorder must be *literal*; any "helpful" generalization is a bug.
- **Trace pollution** (autopilot actions sneaking in) → record only *player-manual*
  actions at the manual call sites; tested.
- **Pre-highlight scope** → splittable (G13b) if it's not small; don't block the core on it.

## Acceptance checklist

- [ ] Rolling per-agent action memory of *manual* actions (~10, session-local); not polluted
      by autopilot/auto-collect.
- [ ] "trace → routine" builds a draft of the **exact** blocks (only run-length `repeat`
      folding; no inference), opened in the existing editor for manual generalization.
- [ ] Draft is ordinary G7 data — enables/edits/persists/runs through existing paths; a
      recorded draft reproduces the manual behaviour (tested).
- [ ] The trace is visible filling as you act (ticker); glyph-rendered per G12.
- [ ] Pre-highlight of the agent's next target during playback (or split to G13b with a note).
- [ ] Golden voxel-hash + headless render unchanged; CI green; boundary intact; roadmap G13.
