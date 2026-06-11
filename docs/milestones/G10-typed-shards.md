# G10 — Typed shards (the collectible currency)

> **Status: planned — do NOT build until the channel directs it** (G9 first; this brief is
> staged on the steering branch). The human-decided direction: "add shards… auto collectible,
> probably with different types and rarities." Today **shards don't exist** — `scan(shards)`
> is a label that actually senses inscription sites, and `spend` is a stub with no currency
> and no targets. G10 makes shards **real world items**: typed, rarity-graded, scannable,
> auto-collectible through the existing routine pipeline, and **spendable on the first
> faculties**. This is what makes `match`/`priority`/selective-collection *matter* — the
> management game gets real decisions. In `scraped-again`; no engine changes expected.

## Goal · Demonstrable outcome · What it de-risks

- **Goal.** A real shard economy: seed-scattered shard items in the world (5 domain types ×
  3 rarities), sensed by `scan(shards)`, collected by every collect route, counted+persisted,
  and spent via `spend(faculty)` on the first passive faculties.
- **Demonstrable outcome.** Fly the given routine and watch the shard counter climb as the
  ship auto-collects the shards it scans; rare shards glint visibly brighter; author
  `scan(shards) → on-scan → match(rare) → collect` and only the rare ones get taken; bank 50
  and click `spend(sensing)` — the scan radius visibly grows. Counts survive a share-link
  round-trip.
- **De-risks.** The whole upgrade economy: that a typed/rare currency + filters turns
  auto-collection from a checkbox into *tuning decisions* (chase rare Signals shards vs bulk
  Records shards) — the heart of "interesting purely through menus."

## Scope

**In:**
- **Shard items in the world.** Seed-scattered on their own grid (denser than inscriptions —
  they're the bulk currency), deterministic, streamed near the camera like
  inscriptions/colossi. Rendered cheap + on-aesthetic: a small emissive cluster through the
  existing splat path, **tinted by domain**, with rarity scaling the glow/size (rare = a
  visible glint at distance).
- **Types & rarities.** Type = the five strata domains (Records/Schematics/Rites/Relics/
  Signals — same tint family as their scripts); rarity = common/uncommon/rare with seeded
  weights (~85/13/2) and yield 1/3/9. *(Whether spend-targets later require domain-matched
  shards is a parked human fork — v1 spend takes a generic total; types exist to feed
  filters + future matching.)*
- **Scan honesty.** `scan(shards)` now senses *shard items* (matching the design intent —
  game-system §11 calls shards "the basic upgrade currency / starter item"). Inscription
  sites become their own scan item — `ScanItem::Sites` — **kept in the Tier-0 given** so the
  opening still auto-collects both: the given routine becomes (or is joined by)
  `scan(sites) → on-scan → collect` alongside shards. **Do not nerf the opening** — the
  G3 cruiser autoscan + map pins must keep working for sites exactly as today.
- **Collection.** All routes (T / beam-along-path / routine auto-collect) pick up shards in
  reach; `match` grows shard-aware fields — `match(shard-type)` and rarity applies to shards
  too (reuse the existing rarity field). Field unlocks follow the existing stratum gating.
- **Spend becomes real.** `spend(faculty)` parameterised like `scan(item)`; first faculties
  (passive, modest): `sensing` (scan radius +25%/level), `reach` (collect/beam reach
  +20%/level), `drive` (cruise speed +15%/level); 3 levels each, escalating cost (e.g.
  25/75/200 shards). Clickable once affordable; wireable (`when(shards ≥ N) → spend(…)`)
  if `when` already supports a shards state — add that state if cheap, else note it.
- **Counts + persistence + HUD.** Per type×rarity counts in the progress state; `pg=`
  next version, append-only; HUD shard readout (total + a compact per-domain line); the
  codex/console shows faculty levels.

**Out:** domain-matched spend costs (parked fork); gradual shard-fill vs one-time-spend
models (parked fork); shard-driven block *discovery* (names are G9's job); any new engine
capability; economy *tuning* beyond the pinned numbers (end-of-run play).

## Design sketch

- `shards.rs` (new): `Domain` (= reuse `Stratum`), `Rarity`, seeded `shards_near(cam, seed,
  radius, ground)` like `inscriptions_near` (own grid spacing + cell hash → presence/type/
  rarity/offset); a `collect_at`/in-reach predicate for the collect paths.
- `progress`: `Event::CollectShard(domain, rarity)` (+ `Event::Spend(faculty)`), counts +
  faculty levels in the derived state; versioned `pg=`.
- `console`: `ScanItem::{Shards, Sites}`; `Block::Spend(Faculty)` parameterised; `match`
  field for shard type; the given routines updated to keep opening parity.
- App: faculty effects applied where scan radius / reach / cruise speed are read (plumb as
  multipliers from progress state).

## Decisions to resolve (pinned defaults — veto via the channel)

1. **Types = the five strata domains**, tinted to match their scripts. Rarity 3-tier,
   ~85/13/2, yield 1/3/9.
2. **Spend takes the generic total** in v1 (types feed filters, not costs).
3. **Faculties v1:** sensing / reach / drive, 3 levels, 25/75/200. Numbers are placeholders
   to tune at the human pass — don't agonise.
4. **Opening parity:** sites stay auto-collected by the given wiring; shards join, not
   replace. If one given routine can't express both cleanly, two given routines is fine
   (they're ordinary data since G7).

## Tests

- Seeded scatter: deterministic; type/rarity distribution within tolerance; density bounded.
- Collect: in-reach predicate; counts accumulate by type×rarity; events round-trip `pg=`
  (old payloads load).
- Spend: gated on affordability; levels cap; cost ladder; effects expressed as multipliers
  (pure fn tested).
- Routine integration: `match(rare)` filters shard collection in an authored pipeline
  (interpreter-level test, like G7's).
- Golden voxel-hash unchanged (terrain untouched); headless render — shards are new visible
  content: keep them **outside the golden default** if the golden image would change (flag-
  gate like `SCRAPED_BEAM` only if needed; otherwise update the golden once, noted).
- CI green (fmt / clippy -D / tests / wasm); boundary intact.

## Risks & mitigations

- **Opening regression** (sites no longer auto-collected) → explicit parity requirement +
  the G3 autoscan tests must stay green.
- **Currency feels noisy** (too many pickups) → density pinned conservative; rarity does the
  excitement work, not volume.
- **Faculty plumbing sprawl** (multipliers threaded everywhere) → one `Faculties` struct
  computed from progress, read at the three call sites.

## Acceptance checklist

- [ ] Shard items: seeded, streamed, domain-tinted, rarity-glinted, rendered via the splat
      path; deterministic + density-bounded (tested).
- [ ] `scan(shards)` senses shards; `scan(sites)` senses inscriptions; **opening parity**
      (sites + shards both auto-collected by the givens; G3 autoscan/map unchanged).
- [ ] All collect routes pick up shards; counts by type×rarity persist (`pg=` versioned,
      old payloads load); HUD readout.
- [ ] `match` filters shards by rarity (+ type field); an authored selective-collect
      pipeline works (tested at the interpreter level).
- [ ] `spend(faculty)`: sensing/reach/drive, 3 levels, escalating costs; effects applied as
      tested multipliers; clickable + wireable.
- [ ] Golden voxel-hash unchanged; CI green (fmt / clippy -D / tests / wasm); boundary intact.
- [ ] Roadmap G10 entry + this checklist ticked on `main`.
