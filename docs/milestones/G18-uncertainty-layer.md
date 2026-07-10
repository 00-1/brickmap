# G18 — The uncertainty layer (provisional readings, erosion, Leiden display)

> **Status: ✅ done** (2026-07-10, one green commit; as-built notes under the checklist). The Archive
> tranche's first *feature* milestone, on the G16 lexicon substrate: **readings stop being
> instant**. A found reading is a **hypothesis** until confirmed; inscriptions carry seeded
> **condition** (erosion, and rare **⟦deliberate erasure⟧** marks); the survey log/codex
> displays it all in the epigraphers' real **Leiden grammar**. Everything structural — the
> no-readable-lore constraint holds absolutely (brackets and dots annotate *glyphs*, never
> prose). Sources: [`../research-decipherment.md`](research-decipherment.md) P1/P3/P5,
> [`../research-material-text.md`](research-material-text.md) §1–2. Game-side; no engine
> change expected (world-text content + console/codex rendering ride existing paths).

## Goal · Demonstrable outcome · What it de-risks

- **Goal.** Three additions that together make collecting feel like *deciphering*:
  (1) **provisional → confirmed** reading states for discovered block names;
  (2) **inscription condition** — intact / worn (eroded glyphs) / an ⟦erased⟧ mark class;
  (3) the **Leiden display grammar** in the codex/log (underdots = provisional, `[..]` =
  lost glyphs, `⟦ ⟧` = deliberately erased, clean = confirmed).
- **Demonstrable outcome.** Collect a name-bearer: the block appears in the console with a
  **provisional mark** (underdotted glyphs). Confirm it either by **seeing the same name a
  second time** anywhere, or **behaviorally** — comprehend it and *click it once*; the
  machine answering IS the confirmation (the world-as-oracle rule; no UI judgment, no
  softlock). Worn inscriptions render with missing glyphs in the world and `[..]` lacunae
  in the codex, yielding less. A rare ⟦erased⟧ inscription shows a visible gouge-mark
  whose content is *unrecoverable for now* (the hook the sensing ladder, G20, later pays
  off). The codex re-renders entries as their state improves.
- **De-risks.** Whether "comprehension through structure" plays as *deciphering* rather
  than collecting — the tranche's central bet, at its cheapest testable size.

## Scope

**In:**
- **Reading states (names only, v1).** `Attestation { Provisional, Confirmed }` per
  discovered block. Provisional on first collect; **confirmed by**: (a) collecting/being
  in collect-range of a *second* inscription bearing the same block's name, or (b) the
  first successful *execution* of the block after comprehension (behavioral confirmation).
  **Mechanically light by design:** provisional blocks CAN be allocated/researched (no
  gate — no softlock); the state is display + one gentle hook — the lit-goal may nudge an
  unconfirmed-but-comprehended block ("use it once"). Persist in `pg=` (v8, append-only;
  old payloads → all-confirmed for already-comprehended, provisional for merely-discovered).
- **Inscription condition (seeded, deterministic).** Per-cell hash → intact (majority) /
  **worn** (a seeded subset of glyph positions replaced by a lacuna mark in the world
  billboard; stratum-data yield reduced proportionally; a worn *name-bearer* still
  discovers — the name is recoverable from partial glyphs v1) / **⟦erased⟧** (rare, ~1 in
  40: the billboard renders a distinct gouge/strike mark over a blank cluster; collecting
  yields nothing now but **logs the erasure event** in the codex — content unrecoverable
  until G20's sensing ladder; the codex entry is the tease).
- **Leiden display in the codex/log:** provisional names underdotted (or an equivalent
  distinct sub-glyph mark on the HUD path); worn entries show `[..]` per lost glyph;
  erased entries show `⟦——⟧`; confirmed entries clean. Re-render on state change (the
  codex reads live state — no stored snapshots). All marks are **structural UI** (the G12
  scope line: instrumentation, not vocabulary).
- **`when` → `while` rename (the pinned §I.2 rider):** the state-trigger's display label
  becomes `while(…)` (codec/codes untouched — display only); event triggers keep `on-…`.
  One-line + docs; the measured TAP event/state fix completes the shape work G11 started.
- **E2E:** extend D11 — a scenario driving provisional→confirmed via both paths, worn-yield
  reduction, and an ⟦erased⟧ collect logging without yield.

**Out:** the sensing ladder / recovering erased or worn content (G20); formulaic-frame
cribs + cartouches (G19); palimpsest under-layers (G20); hypothesis states for *ambient*
(non-name) readings (later, if ever — names are the load-bearing case); any change to
research/economy mechanics beyond the lit-goal nudge.

## Design sketch

- `progress`: `attested: HashMap<block-code, Attestation>` (or a second HashSet);
  second-sighting check at the existing discovery funnel (a name-bearer collect for an
  already-discovered block upgrades Provisional→Confirmed — the "dupes yield normally"
  path gains one line); behavioral confirmation at `dispatch_block`/interpreter first-run
  of a comprehended block. `pg=` v8 append-only.
- `structures`: `compose`/`inscriptions_near` gain a condition arm from the cell hash
  (independent bits from the name-gate — remember the G10 correlation lesson; add a
  distribution test). Worn = a per-inscription mask of lacuna positions; erased = a
  distinct `Inscription` kind rendering the gouge mark (a reserved overlay glyph or a
  strike cluster — reuse the five-script rasterizer's machinery, no new engine text
  capability; if a tiny generic mark-glyph addition to the shared font is unavoidable,
  it's content-agnostic and boundary-safe — document it).
- `console`/codex: reading-state rendering via the existing glyph/overlay path (underdot =
  a combining mark under the glyph cluster or a dot-row beneath — pick the cheapest that
  reads at HUD size; headless A/B it).
- Golden: worn/erased inscriptions change world-text content at spawn → **update the
  golden inscription image once, noted** (the established G9 policy); voxel-hash untouched.

## Decisions to resolve (pinned defaults — veto via the channel)

1. **No mechanical gate on provisional** (allocate/research allowed); confirmation =
   second sighting OR first use. *Pinned — the no-softlock invariant.*
2. **Condition mix:** ~intact 70% / worn 27% / erased ~3% of inscriptions (deterministic,
   independent bits; placeholder ratios, feel pass tunes).
3. **Worn name-bearers still discover** v1 (partial-name recovery is a G20 refinement).
4. **Erased content is unrecoverable in G18** — the codex logs the *event* (site, stratum,
   the gouge) as the G20 hook. Rare enough to be a curiosity, not a wall.
5. **`while` rename is display-only** (codec untouched).

## Tests

Attestation transitions (both confirmation paths; idempotent; codec v8 round-trip + old-
payload migration); condition determinism + distribution + **independence from the
name-gate bits** (the G10 lesson, as a test); worn yield reduction; erased yields-nothing-
but-logs; codex rendering states (unit-testable string/glyph-run level); the `while`
label; D11 scenario extension; golden voxel-hash unchanged + the one noted inscription-
image update; CI green (fmt / clippy -D / tests / wasm); boundary intact; roadmap G18.

## Risks & mitigations

- **Friction creep** (the human's known concern): provisional gates nothing (Decision 1);
  worn is a minority; erased is rare. The layer is *texture + tease*, not a wall.
- **Correlation bugs** (the G9/G10 class): condition bits independent by construction +
  the distribution test.
- **Codex clutter:** three marks only (underdot / `[..]` / `⟦ ⟧`); anything more waits for
  the eye-pass.

## Acceptance checklist

- [x] Provisional→Confirmed per block (second-sighting + behavioral paths, tested); no
      mechanical gate; lit-goal nudge; `pg=` v8 append-only + migration.
- [x] Seeded inscription condition (intact/worn/⟦erased⟧; independent bits + distribution
      test); worn renders lacunae in-world + `[..]` in codex + reduced yield; erased
      renders the gouge, yields nothing, logs the event (the G20 hook).
- [x] Codex/log renders the Leiden states live (underdot/lacuna/erased/clean); headless
      A/B shows them; all marks structural-UI (no lore, no English vocabulary).
- [x] `when` → `while` display rename (codec untouched) + docs.
- [x] D11 scenario extended; golden voxel-hash unchanged (one noted inscription-image
      update allowed); CI green; boundary intact (any shared mark-glyph addition
      content-agnostic + documented); roadmap G18.

## How it landed (as-built notes, 2026-07-10)

- **Attestation** lives in `progress` as a `confirmed: HashSet<Block>` beside `discovered`
  (provisional = discovered ∧ ¬confirmed; starters implicitly confirmed — their names were
  never field hypotheses). Second sighting rides the existing `Event::Discover` funnel: a
  re-apply on an already-discovered gated block now upgrades once (the "dupes yield
  normally" path gained its one line). Behavioral confirmation is
  `Progress::confirm_block_use` called from the app's *executing* dispatch sites — manual
  console clicks that actually fire an effect, interpreter acts (`wired()` only — an
  unwired `goto` act attests nothing), live nav intents, and on-scan acts. A manual click
  that answers with "no longer a direct action" deliberately does **not** attest
  (world-as-oracle: only the machine answering confirms). `pg=` v8 appends the confirmed
  codes; pre-v8 payloads migrate as confirmed = discovered ∩ comprehended.
- **Condition** is picked in `structures` from a **fresh-salt cell hash** (`^0x51AB/^0x2E77`,
  seed+`0xC05D17`) — independent of the name-gate bits by construction, with a
  distribution + independence test over 40k cells (the G10 lesson as a test). Worn masks
  ~1/3 of glyph positions (always ≥1 lost, ≥1 surviving; a 1-glyph text stays intact);
  the lost positions render the engine's lacuna mark in the world billboard, and yield
  drops automatically because `progress::glyph_count` no longer counts damage marks.
  Erased renders a constant **3-cell gouge cluster** (even the original length is hidden),
  strips its `name` (no discovery from a gouge), and collects through a new
  `Event::CollectErased` — codex entry logged, nothing banked, dedup as usual.
- **Deviations from the sketch**, all recorded: (1) **colossus monument labels stay
  intact** — the deep names are the no-softlock coverage guarantee (the
  every-name-findable test), and eroding monuments would gamble it per-seed; grid inscriptions
  carry the full condition mix. (2) A **legible-script ambient inscription only translates
  when intact** — worn/erased content stays glyphs+marks even after comprehension (worn
  recovery is exactly what G20's sensing ladder pays off). (3) There is no golden
  *image* byte-compare test for scraped-again (only goblin-gold has one, untouched), so
  the "one noted image update" reduces to: the headless render's world-text content
  changed where worn/erased inscriptions now show marks — intentional, verified by eye;
  the golden **voxel-hash test is untouched and green**.
- **Leiden display**: the codex re-renders live from state — `⟦——⟧` for an erased event,
  `[..]` per lost glyph, and a **dot-row beneath** a provisional name's glyph cluster
  (the monospace HUD grid aligns it; chosen over a combining mark as the cheapest that
  reads at HUD size). The console palette marks a provisional block with a trailing
  **underdot sub-mark cell** on the same HUD glyph path (headless `SCRAPED_CONSOLE=1`
  A/B verified: the marks show after the Greek/Runic clusters). Name↔entry matching is
  `structures::name_of_text` (inverse transliteration, lacuna-tolerant).
- **Engine touch (documented, content-agnostic):** `bm-render::text` gained a tiny mark
  PUA range (U+E620 lacuna, U+E621 gouge, U+E622 underdot) rendered identically in every
  script + on the overlay path, plus overlay bitmaps for the generic epigraphic
  punctuation ⟦ ⟧ — (U+27E6/27E7/2014). Pure typographic damage/annotation cells; no game
  concept crossed the boundary.
- **`while` rename**: display-only in `Trigger::label` (`while data ≥ 10`); the `co=`
  codec (`w:`/`wS`/`wY`/`wK`) and rising-edge semantics untouched, tested.
- **Tests added** (12): attestation transitions/no-gate, v8 round-trip + v7 migration,
  worn-yield + erased-logs (progress); condition distribution/independence, weathering
  properties, `name_of_text`, erased-strips-name (structures); `while` label, underdot
  palette mark, lit-goal nudge (console); mark glyphs across scripts/overlay (bm-render);
  and the D11 scenario `uncertainty_attestation_condition_and_erasure` driving both
  confirmation paths, worn yield, erased logging, the live codex re-render, and the v8
  share round-trip through the real collect/frame seams. 176 game-crate tests green;
  full workspace + wasm + fmt + clippy green.
