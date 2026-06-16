# G17 — Handshakes & the expedition economy

> **Status: ready to build — next directive.** The last fork-free milestone in the
> automation-depth pipeline ([`../game-depth.md`](game-depth.md);
> [`../research-automation-depth.md`](research-automation-depth.md) P18/P19/P12): the two
> agents stop sharing a magic pocket and start **coordinating through the world** — a cache
> the walker fills and the ship empties, with both sides of the handshake player-authored.
> Failed handoffs are visible little vignettes, not error messages. Game-side; no engine
> change expected.

## Goal · Demonstrable outcome · What it de-risks

- **Goal.** A **cache** (a placeable/world drop-point) + the `deposit` verb: the walker
  collects into its **carry** (a small cap — the first real per-agent asymmetry) and
  `deposit`s into the cache; the ship `collect`s from a cache it lands near. The G8
  expedition becomes a real logistics handshake the player programs both sides of.
- **Demonstrable outcome.** Author: foot `on-arrive → collect → when(carry full) → deposit`;
  ship `goto(cache) → land → collect → ascend`. Watch the walker fill the cache, the ship
  drain it, yields credit through (G11). Break one side (disable the ship routine) and the
  cache visibly accumulates — the walker waiting at a full cache is an honest, legible
  state (`blocked: cache full`), not an error.
- **De-risks.** The research's strongest two-agent finding (Factorio trains, P18): agents
  coordinating **only via world state** is what makes multi-agent automation feel authored
  rather than scripted — and carry/cache capacities are the first real scarcities (P1) the
  later economy tuning leans on.

## Scope

**In:**
- **Walker carry cap** (small, e.g. 8 items): collect fills carry; full carry blocks
  further collect (honest G11 reason `blocked: carry full`). Shards/finds carried as
  counts; deposit transfers them to the cache; the *bank/research intake* happens on
  **ship pickup** (the handshake is what moves value home — this is the point).
- **The cache:** one given cache per expedition site v1 (spawned at the expedition target /
  the landing point — no free placement yet; placement UX is a later polish). World-visible
  (a small emissive marker via the splat path, budgeted per charter rule 2). Persistent in
  the session; contents survive agents leaving (`pg=` next version, append-only).
- **`deposit` (foot) + cache-aware `collect` (ship)** blocks: `deposit` moves carry →
  cache; a ship `collect` within reach of a cache drains it into the bank/research intake
  (the existing CollectShard/credit path — same canonical events, so D11's harness covers
  it). Both glyph-named (G12), discoverable/researchable like any block? **No — v1 they're
  Tier-0-adjacent given vocabulary** (the expedition already exists; gating the handshake
  behind research would orphan G8 — pin in Decisions, vetoable).
- **`when(carry ≥ %)` / `when(cache ≥ N)` states** for the wiring (the G14-deferred state
  expansion, now with real referents).
- **Asymmetry recorded:** ship has no carry cap v1 (it's the hauler); the walker's cap is
  the constraint. Per-agent routine-slot asymmetry (P19) stays deferred (a *discovered*
  property later, per P12 — not a menu number now).
- **Telemetry/teaching:** G11 reasons (`carry full` / `cache full` / `cache empty`); the
  failed-handoff vignette is just honest state + the visible cache marker.
- **E2E:** extend the D11 scripted expedition to assert the full handshake (fill → deposit
  → ship drain → credit) — the harness exists precisely so new systems land with
  integration coverage.

**Out:** free cache placement UX; multiple caches/routing networks; ship carry caps;
routine-slot budgets; any new engine primitive (markers ride the splat path).

## Design sketch

- `progress`/`expedition`: `carry: u32` (cap const) on the walker agent state; `cache:
  Vec<(Domain,Rarity,count)>`-ish compact store keyed to the expedition site; `deposit`
  drains carry→cache; ship-collect within reach drains cache→the canonical CollectShard
  events (so banking/research/credit all just work).
- `console`: `Block::Deposit` (foot), cache-awareness in the ship `collect` dispatch,
  `State::{Carry, Cache}` for `when`. Glyph names via the existing transliteration.
- `pg=` v7 append-only: carry, cache contents; old payloads → empty.

## Decisions to resolve (pinned defaults — veto via the channel)

1. **`deposit` + cache are given vocabulary** (not research-gated) so the existing G8
   expedition isn't orphaned; they're the expedition's tier. *Pinned; vetoable.*
2. **Value lands on ship pickup** (not on deposit) — the handshake moves value home; a
   stranded full cache is real risk/drama. *Pinned.*
3. **One cache per expedition site, auto-spawned** v1; placement UX later. *Pinned.*
4. **Walker carry cap 8** (placeholder; feel pass tunes).

## Tests

Carry fill/block + honest reason; deposit transfers; ship drain emits canonical events
(bank/research/credit verified); `when(carry/cache)` edges; persistence round-trip (v7,
old payloads); the D11 expedition scenario extended to the full handshake; golden
voxel-hash unchanged (cache markers content-keyed/budgeted); CI green; boundary intact;
roadmap G17.

## Risks & mitigations

- **Orphaning the simple expedition** → the old direct-collect expedition keeps working
  when no cache wiring exists (`deposit` unused = today's behaviour); the handshake is
  opt-in depth (P4 optionality audit).
- **Scope creep into logistics networks** → one cache, one site, v1 (Decision 3).
- **Splat budget** → marker consumer budget declared (charter rule 2), tiny.

## Acceptance checklist

- [x] Walker carry (cap, honest `blocked: carry full`), `deposit` → cache, ship
      cache-collect → canonical events (bank/research/credit flow); old expedition still
      works with no cache wiring.
- [x] `when(carry ≥ %)` / `when(cache ≥ N)`; glyph-named blocks; cache world-marker
      (budgeted).
- [x] `pg=` v7 append-only round-trip; old payloads load.
- [x] D11 scenario extended to the full handshake; golden voxel-hash unchanged; CI green;
      boundary intact; roadmap G17.

## How it landed (2026-06-16, two green commits)

**1/2 (`dc190a2`) — the walker side + the language + storage.** `progress`: `carry`/`cache`
domain×rarity tallies held in transit; `CARRY_CAP` = 8; `carry_shard` (honest-blocks at the cap),
`deposit`, `drain_cache`; `pg=` v7 (old payloads load empty). `console`: `Block::Deposit` (foot,
**given** — Decision 1), `State::{Carry, Cache}` + `when(carry ≥ %)` / `when(cache ≥ N)` (threaded
through `tick`/`Cond::holds`, the editor cycle, and the `co=` codec), `BlockReason::{CarryFull,
CacheFull}` + `note_blocked`. Foot `collect` routes shards into carry at every dispatch site (Walk,
away-walker, expedition harvest, on-scan); finds still bank immediately.

**2/2 (this commit) — the ship side + the payoff.** `drain_cache_if_near`: a ship `collect` within
the hauler's reach of the cache drains it home via canonical `CollectShard` events (value lands —
Decision 2; bank + research fill + routine credit all flow). `cache_pos` (session) is placed at the
expedition site / first deposit and cleared when the cache empties; the world-visible marker is a
budgeted emissive cluster (`CACHE_MARKER_CAP` = 14) that grows with the cache. The D11 harness gains
two handshake tests (full carry → deposit → ship-drain → bank/credit; the loop runs with a cache),
plus a console codec/vocab/edge test. **Decisions honoured:** 1 given vocab, 2 value-on-pickup, 3
one auto-spawned cache, 4 carry cap 8. The simple direct expedition is untouched when no `deposit`
is wired (optionality). Golden voxel-hash + headless render byte-identical; boundary intact.
