# G12 — Glyph console (de-Anglicization)

> **Status: ready to build — next directive.** The foundational consequence of the human's
> 2026-06-11 decision ([`../open-questions.md`](../open-questions.md) §A): **block names
> stay unreadable — no English layer at all.** Today G9/G11 print English block labels in
> the console + HUD (`scan(shards)`, routine rows, "NAME RECOVERED — `priority`"). This
> renders **block identity as its glyph-name everywhere** (the G9 `transliterate` output,
> in the block's stratum script) so the console reads as the dead machine's own language —
> *before* more English UI accretes. Game-side (`scraped-again` console/HUD on the E17 text
> path); no engine change.

## Goal · Demonstrable outcome · What it de-risks

- **Goal.** A block is shown by its **glyph-name** (stable per block, in its stratum's
  script — the same glyph-cluster the player finds carved in the world), never an English
  word, throughout the console, palette, codex, and the discovery toast. Function is
  learned by **clicking and observing** (ladder L0), not by reading a label.
- **Demonstrable outcome.** Open the console: the palette and routine rows show glyph
  clusters, not `scan`/`collect`; collecting a name-bearer flashes **NAME RECOVERED** + the
  recovered glyph cluster (no English block word); the codex lists discovered blocks by
  glyph. The block you click to fire `collect` shows the *same* glyphs you saw on the
  inscription that discovered it — closing the world↔console recognition loop G9 opened.
- **De-risks.** That the "comprehension, not conquest / lore implied, never read" identity
  holds at the UI layer — the console stops being a labelled menu and becomes a recovered
  terminal you learn by operating. And it's cheap to do now, expensive to retrofit after
  G13+ pile more labelled surfaces on.

## Scope

**In:**
- **Block identity → glyph-name** wherever a block is currently shown by `label()`: the
  palette/insert list, routine step rows, the selected-routine detail line, the codex (`J`),
  and the discovery toast. Render via the existing E17 text path in the block's
  **stratum script** (the `transliterate(name, script_for(stratum))` machinery G9 already
  has — reuse it; the console already draws on the text path).
- **Stable + recognizable:** a block always renders the same glyph cluster (deterministic,
  per G9), so it's recognizable *as a symbol* even though unreadable *as a word* — icon
  literacy, not reading literacy. Starter (Tier-0) blocks render in their stratum's script
  like any other.
- **Discovery toast:** "NAME RECOVERED" (or an equivalent non-block-naming marker) + the
  glyph cluster — no English block word.
- **Structural / meta UI stays minimal-English** (the pinned scope default — veto if
  wrong): numbers, gauges (`37/50`), `×fires` · `y` · rate, the `▶` executing-step marker,
  state glosses (`running`/`waiting`/`blocked: nothing in reach`), faculty levels, the
  HUD `◆` goal line's *non-name* parts. These read as the machine's **instrumentation**;
  its **vocabulary** is the dead language. (Where the goal line names a block, that part
  goes glyph.)
- **Parameters:** a parameterised block's argument (`scan(shards)` → the shards glyph;
  `match(rare)`, `spend(sensing)`) renders glyph too where the argument is a
  world-vocabulary item; pure quantities (thresholds, `≥ 50`) stay numeric.
- **Docs in lockstep:** update `game-system.md` §1/§6 (blocks are glyph-named, learned by
  clicking — drop the readable-name framing) and any `game-mechanics.md` line that shows
  readable block names; roadmap G12 entry.

**Out:** a glyph-composition / derive-a-name verb (the superseded knowledge-gate — not
needed); full-glyph-everything including structural UI (a later eye-pass call); any new
text/render capability (the five scripts already render).

## Design sketch

- The console renderer already composes block rows via `Block::label()`. Add a
  `Block::glyphs()` (or render-time `transliterate(self.name(), self.stratum_script())`)
  and route the console/HUD/codex/toast draws through it. `name()`/`label()` may stay for
  *internal* use (codes, tests, the `co=` codec) — only the **player-facing** draw changes.
- Stratum→script is the inverse map G9 built (`progress::script_for`); a starter block's
  stratum is whatever it's gated by (Tier-0 = the on-ramp stratum).
- Keep the glyph cluster compact enough to sit in a phone-width row (it already must, as
  world inscriptions); truncation/scaling is a feel detail for the eye-pass.

## Decisions to resolve (pinned defaults — veto via the channel)

1. **Scope line:** vocabulary (block + world-item names) → glyph; instrumentation (numbers,
   gauges, state, markers) → minimal-English. (As above.)
2. **Learn-by-clicking is the teacher** — no English tooltip/gloss. Onboarding leans on the
   given pre-wired routine + the 1–2 starter blocks (vocabulary grows one block at a time,
   so opacity is bounded at any moment).
3. **Toast** keeps a short English *event* marker ("NAME RECOVERED") — that's
   instrumentation, not a block name — plus the glyph. Veto → fully glyph.

## Tests

- `glyphs()` deterministic + stable per block; renders in the correct stratum script;
  distinct across the vocabulary (reuse the G9 collision test's spirit).
- Console-state tests still pass (identity is unchanged; only its *rendering* changed —
  `co=`/`pg=` codecs untouched, routine equality untouched).
- Golden voxel-hash unchanged (console/HUD aren't in it); a headless A/B of the console
  (opt-in flag, like `SCRAPED_TOUCH`) shows glyph blocks.
- CI green (fmt / clippy -D / tests / wasm); engine boundary intact (text path already generic).

## Risks & mitigations

- **Too opaque?** Pure-glyph with no gloss is the human's explicit call; bounded by
  starting at 1–2 blocks + learn-by-clicking. **Mitigation (note, don't build):** if a
  playtest finds it impenetrable, an *optional after-use gloss* (a player-facing note that
  appears only once you've fired a block, in structural-UI English) is the fallback — flag
  for the human's eye-pass, do not build now.
- **Recognition vs reading:** the win depends on glyph clusters being visually distinct at
  row size — the same constraint world inscriptions already meet; final sizing is eye-pass.

## As built (2026-06-11, two commits)

The brief assumed "game-side only, no engine change; the five scripts already render." That
held for the **world-text billboards** (`text::WorldText`) but **not** the console, which
draws through the **ASCII-only HUD overlay** (`hud::rasterize` rendered `0x20..0x7f`, else a
dot) — and `transliterate` stores Latin-letter stand-ins for Galactic/Runic, so a flat string
isn't script-self-describing. So G12 needed a small **generic** engine addition: wire the
existing five-script glyph rasterizer into the HUD overlay (no *new* scripts invented). Split:

- **G12 (1/2)** `a6a898d` — identity machinery (`Block::glyphs`/`glyph_label`,
  `MatchField::glyphs`, `Step::glyph_label`) + the **world-text de-Anglicization** (a
  name-bearer keeps its glyph cluster even when its script is legible; only ambient text
  translates) + docs (game-system §1/§4/§5, game-mechanics §8.2).
- **G12 (2/2)** *(this commit)* — the engine HUD capability + console/HUD wiring.

The chosen design avoids a per-segment "runs" API: block glyphs are emitted as
**self-identifying overlay codepoints** (`text::to_overlay`: Greek/Hiragana already are;
Galactic→SGA PUA; Runic→a dedicated PUA range), and the HUD detects each char's script by
codepoint (`text::overlay_glyph`). ASCII still routes through the HUD's legacy font, so existing
HUD output is **byte-identical** (verified: `BASIC_LEGACY == BASIC_FONTS` for ASCII, and a
clean-HEAD render `cmp`s identical). A unit test proves the overlay codepoints reproduce the
*exact* bitmaps the world billboard draws (the recognition loop, at the pixel level).

## Acceptance checklist

- [x] Blocks render as glyph-names (stratum script, stable/deterministic) in palette,
      routine rows, the editor body, and the discovery toast; no English block words.
      *(Selected-detail line is counters only — no block name there; codex already shows the
      raw inscription glyphs.)*
- [x] World↔console recognition holds: a console block's glyphs are the overlay form of the
      exact cluster its world inscription spells (`block_glyphs_match_world_inscriptions` +
      `overlay_codepoints_reproduce_world_bitmaps` — bitmap-identical).
- [x] Structural/meta UI stays minimal-English per the scope line (numbers, gauges, state,
      `▶`, the `(locked: decode SCH)` tag, `decode … ready`); parameters render glyph for
      vocabulary items (scan target, faculty, match field), quantities stay numeric.
- [x] Learn-by-clicking intact; given routines + opening behaviour unchanged (only rendering
      changed; `co=`/`pg=` codecs + routine equality untouched).
- [x] `game-system.md` §1/§4/§5 + `game-mechanics.md` §8.2 updated; roadmap G12 entry added.
- [x] Golden voxel-hash unchanged; headless console A/B (`SCRAPED_CONSOLE`) shows glyph blocks;
      CI green (fmt / clippy -D / tests / wasm); engine boundary intact (the HUD addition is
      generic — `text::Script`/`overlay_glyph`, no game concept).
