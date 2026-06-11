# G15 — Comprehension-as-research (the unified economy)

> **Status: ready to build — next directive (LARGE; expect a split).** The economy reframe
> the human designed and the babysitter recorded as fully fork-resolved
> ([`../open-questions.md`](open-questions.md) §C; [`../game-depth.md`](game-depth.md) G15):
> **discovery + shards + decode are ONE pipe.** It **retrofits** G9's unlock gate and G10's
> `spend` into a single research model. The biggest game-design change of the run — split it
> (suggested split below) and build the parts green one at a time. Game-side
> (`scraped-again` console/progress); no engine change expected.

## The model (what we're building)

**Find a block's glyph-name in the world → it becomes a research target (locked) → the
player allocates auto-collected shards into it → it fills over time → comprehended (usable)
→ keep feeding for levels.** "Decoding" and "spending" become the same act: comprehension
is shard-funded research.

Fully-decided rules (do not re-litigate — these are the human's calls):
- **Allocate-and-fill, player-directed (Decision):** the player **chooses which target**
  their shards flow into; the bar fills as shards arrive (even while flying/idle). It's an
  explicit *what to research next* decision — **not** a passive trickle into everything,
  **not** bank-then-instant-buy.
- **Domain-matched + rarity-gated cost (Decision):** a block's research consumes (mostly)
  its **stratum's domain** of shards, and **rarer blocks demand rarer shards** (G10's
  common/uncommon/rare) and/or larger totals — the deepest vocabulary is the hard late
  frontier, gated behind exploring rarer strata. This gives G10's rarity tiers a real sink.
- **Faculties fold in (Decision):** G10's `sensing`/`reach`/`drive` become **ordinary
  research targets** in the same pipe — no separate bank-then-spend subsystem.
- **Progression buys verbs, not numbers (Decision):** research unlocks **vocabulary**
  (blocks / argument options / triggers); faculties are the only modest +% and stay capped.
  No hedonic treadmill.
- **G11 integration:** the "one lit goal" points at the **nearest-to-complete research**;
  per-target progress is shown.

## What it retrofits (preserve behaviour where noted)

- **G9 unlock gate:** today *decode-a-stratum unlocks all its blocks at once*. Replace with
  *per-block research-fill*: a discovered block has a research bar; filling it comprehends
  (unlocks) **that block**. The stratum still determines the block's **shard domain**.
  Discovery (find name → listed, locked) is unchanged; the *unlock step* changes from
  decode-stratum to research-fill.
- **G10 `spend`:** `Block::Spend(Faculty)` / bank-then-buy becomes **allocate-to-research**
  on a faculty target (same fill mechanic as blocks). Keep faculties + their effects
  (the tested multipliers at the three call sites) — only the *acquisition* changes.
- **Opening parity:** the starter set stays pre-comprehended (Tier-0 already usable); the
  given routines + autopilot are unchanged. The first *new* block the player researches is
  the onboarding beat. Don't regress the opening.

## Suggested split (build green one at a time)

1. **G15a — research runtime + allocation + the retrofit.** The `Research` model (per-target
   progress, domain+rarity cost), the **allocate** action (player directs shards into a
   chosen target; fill on shard intake), unlock-on-complete; **delete the decode-stratum
   unlock and the bank-then-spend path**, routing both through research; `co=`/`pg=`
   migration (append-only; old payloads → sensible default: starters comprehended, no
   in-progress research). Console shows targets + bars; G11 lit-goal points at nearest.
2. **G15b — levels + faculties-as-targets + pacing.** Multi-level research (parameter
   unlocks for blocks; capped +% for faculties); faculties fully in the pipe; cost pacing
   numbers (placeholders, tuned at the human pass). The `when(research ≥ %)` / research-state
   trigger if cheap.

*(Build 1 fully green before 2. If even 1 is too large, split allocation-UI from
runtime — but land a runnable research→unlock loop before moving on.)*

## Design sketch

- `progress`/`console`: a `Research { target, filled, cost }` per discovered-but-locked
  target; `cost = f(stratum domain, rarity, tier)`; **allocate** sets/append the player's
  chosen active target(s); shard intake credits the active target's `filled` (allocate-and-
  fill); on `filled ≥ cost` → mark comprehended (the existing unlocked path). Shards stay
  typed (G10) — a block draws its domain; surplus/other-domain handling pinned in Decisions.
- Faculties: a faculty is a research target with levels; effect read from progress as today.
- Persistence: research progress + comprehension set in `co=`/`pg=` (whichever owns it —
  G14a established routines live in `co=`; comprehension/economy likely `pg=`), next
  version, append-only; old payloads load (starters comprehended, economy zeroed).
- Golden-neutral: economy is game-logic; golden voxel-hash + headless render unchanged.

## Decisions to resolve (pinned defaults — veto via the channel)

1. **Active-target model:** one active research target at a time (simplest; "what next") vs
   a small set splitting intake. *Pinned: one active target* (clearest decision; expandable).
   Veto → a few parallel with split allocation.
2. **Off-domain shards:** a block draws its **own domain**; other-domain shards either
   don't apply, or apply at a discount. *Pinned: own-domain only* (keeps domains meaningful;
   makes rare-domain blocks genuinely gated). Veto → discounted cross-domain.
3. **Cost shape:** `cost = base(tier) × rarity-weight`, rarer blocks needing rarer shards
   (a Signals-tier block wants rare Signals shards) — exact numbers are placeholders for the
   feel pass; don't agonise.
4. **Levels:** block levels = parameter/option unlocks; faculty levels = capped +%. *Pinned.*
5. **Migration:** old payloads → starters comprehended, no in-progress research, economy
   zeroed; append-only codec bump.

## Tests

- Research fill: allocate → shard intake credits the active target → completes at cost →
  block becomes usable (unlock path); off-domain shards don't fill (Decision 2);
  rarity/domain cost correct.
- Retrofit parity: a Tier-0 starter is usable from the start; the given routines/opening
  behave as before (opening-parity tests stay green); decode-stratum-unlock is **gone**
  (no path unlocks a whole stratum at once).
- Faculties via research: allocate → fills → level up → the tested multiplier applies.
- Codec: research progress + comprehension round-trip; old payloads load to the migration
  default.
- Golden voxel-hash + headless render unchanged; CI green (fmt / clippy -D / tests / wasm);
  boundary intact; roadmap G15 entry.

## Risks & mitigations

- **Big retrofit touching G9/G10** → split (G15a runtime+retrofit, G15b levels); land a
  green research→unlock loop first; keep opening-parity tests as the guard.
- **Codec migration** → append-only + explicit old-payload default (Decision 5), tested.
- **Pacing feel** → numbers are placeholders, flagged for the human pass; don't block on
  balance.
- **Treadmill creep** → progression buys verbs; faculties the only +%, capped (the human's
  standing rule).

## Acceptance checklist (full G15)

- [ ] Discovered blocks are **research targets**; the player **allocates** shards into a
      chosen target (allocate-and-fill, player-directed); fill → comprehended (usable).
- [ ] Domain-matched + rarity-gated cost (own-domain shards; rarer blocks need rarer
      shards); G10 rarity tiers now have a real sink.
- [ ] **Decode-stratum-unlock removed**; **bank-then-spend removed** — both routed through
      research; faculties are research targets (effects preserved); levels = verbs (+ capped
      faculty %).
- [ ] Opening parity (starters comprehended; given routines/autopilot unchanged); G11
      lit-goal → nearest research.
- [ ] Research + comprehension persist (append-only codec; old payloads → migration
      default); tested.
- [ ] Golden voxel-hash + headless render unchanged; CI green; boundary intact; roadmap G15.
