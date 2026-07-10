# G20 — Cartouches, formulaic frames & true names

> **Status: built ✅ (2026-07-10, three green commits — names → cartouches → frames).** See the as-built note at the end. The Archive tranche continues on G18's substrate. Three
> pieces that belong together because they all touch *naming and recognition*:
> **(1) true names** — the fix for the G18-review finding that Records/Latin-script block
> names leak readable English; **(2) cartouches** — the visual name-enclosure that was the
> first foothold of every real decipherment; **(3) formulaic frames as cribs** — crack a
> recurring frame and it *restores* worn inscriptions for you (the Leiden `[restored]`
> semantics made mechanical). Sources: [`../research-linguistics.md`](research-linguistics.md)
> §1/§5, [`../research-material-text.md`](research-material-text.md) §1/§3,
> [`../research-decipherment.md`](research-decipherment.md). Game-side; the only permitted
> engine touch is content-agnostic cartouche mark glyphs in the G18 PUA range.

## Goal · Demonstrable outcome · What it de-risks

- **Goal.** Names become *true names* (seeded lexicon nonsense, never English), visually
  **cartouched** in world and console alike, and the corpus's recurring frames become a
  *mechanic*: known frames restore worn text.
- **Demonstrable outcome.** The console palette shows glyph clusters with **no readable
  English anywhere** (a Records-stratum block is a Latin-script *nonsense* word, e.g.
  "nauzin", not "collect"); every name-bearing inscription in the world carries a visible
  **cartouche** around the name — the same enclosure the console draws — so you learn to
  *spot* names at a glance; after collecting 3 instances of the same recurring frame, the
  codex records the frame (its glyph skeleton, slot highlighted), and from then on a *worn*
  inscription matching it shows its lost glyphs **restored in Leiden brackets** (`[abc]`)
  and yields full data.
- **De-risks.** The no-readable-lore constraint's last leak; and the tranche's second bet —
  that *structural* pattern-cracking (frames) is a satisfying mechanic.

## Scope

**In:**
1. **True names (lexicon-sourced display names).** Each block's display name becomes a
   **seeded lexicon word** (deterministic per block + world seed, distinct across the
   vocabulary — collision-tested), rendered via the existing per-stratum transliteration.
   English internal names/codes/codecs unchanged (internal `name()` stays for
   tests/codecs). World name-inscriptions, console palette/rows, codex, discovery toast
   all use the lexicon name; the **world↔console pixel-identity test re-targets** (the
   G12 invariant must survive the renaming). Worldgen-version policy applies (name
   inscriptions change content; golden voxel-hash untouched; coverage/uniformity/
   `name_of_text` tests re-target). Parameters (`(shards)`, `(sensing)`) too — every
   vocabulary word goes lexicon.
2. **Cartouches.** A content-agnostic **cartouche-open/close mark pair** (extend the G18
   PUA marks; renders in all scripts + overlay path) encloses the name cluster on
   **name-bearing world inscriptions** and on **console/codex name renders** — the same
   marks both places (the recognition loop extends to "I can spot a name before I can read
   anything"). Ambient (non-name) text never cartouched. Worn name-bearers keep the
   cartouche (the frame survives even when glyphs are lost); ⟦erased⟧ stays a gouge (no
   cartouche — that's a G21 sensing tease).
3. **Formulaic frames as cribs.** G16 already emits a recurring one-varying-slot frame.
   Mechanize: track per-frame *sightings* (collect events on frame-matching inscriptions);
   at **3 sightings** the frame becomes **known** (codex entry: the frame's glyph skeleton
   with the slot marked — structural, no translation). Thereafter, a **worn** inscription
   matching a known frame is **restored**: its lost glyphs render in the codex as
   Leiden-bracketed `[restored]` glyphs (visually distinct from plain lacunae `[..]`) and
   the collect **yields full (unreduced) data**. Restoration applies to *worn* only —
   ⟦erased⟧ recovery is G21's sensing ladder, explicitly out.
4. **Persistence + E2E:** frame-knowledge in `pg=` v10 (append-only; old → none known);
   a D11 scenario: learn a frame via 3 sightings → a worn match restores + yields full.

**Out:** erased recovery / the sensing ladder (G21); proto-language/script shapes (G22);
any English gloss anywhere; frame *authoring* by the player.

## Design sketch

- Lexicon names: a `lexicon::block_name(seed, block) -> String` drawing from the G16
  generator's content-root space (stable, distinct — reuse its morphology so names are
  Kober-able later); `Block::glyphs()` routes through it. The transliteration/overlay
  pipeline is unchanged downstream.
- Cartouche marks: two PUA glyphs (e.g. U+E623/E624) with 8×8 bitmaps (bracket-like frame
  ends), emitted around the name run by the same code that composes name inscriptions and
  console name renders.
- Frames: G16's frame emitter gains an identity (`frame_id` from its skeleton hash);
  `progress` tracks `frame_sightings: HashMap<frame_id, u8>` + `frames_known: HashSet`;
  the codex render + yield path consult it for worn frame-matching inscriptions (match =
  same frame_id, recoverable because the frame skeleton survives outside the lost slots —
  if a *worn frame instance's* id is ambiguous from surviving glyphs, pin: only inscriptions
  whose surviving glyphs uniquely match a known frame restore; ambiguous stays lacunae).
- All three pieces are independent enough to split (names → cartouches → frames) if size
  demands; land each green.

## Decisions to resolve (pinned defaults — veto via the channel)

1. **3 sightings** to learn a frame (placeholder; feel pass tunes).
2. **Restored = full yield** (the payoff must be felt); restored glyphs render bracketed
   `[abc]`-style, distinct from `[..]` lacunae (Leiden: restoration ≠ loss).
3. **Names change with the world seed** (per-seed lexicon names — share-link determinism
   holds; names are *per-world*, which is on-theme: each dead world had its own tongue).
4. Ambiguous worn frame-matches don't restore (no false restorations).

## Tests

Lexicon-name determinism/distinctness (collision test re-targeted); world↔console
pixel-identity under lexicon names; cartouche marks render in all scripts + overlay; only
name-bearers cartouched; frame sighting-count → known at 3; worn frame-match restores
(render state + full yield) vs ambiguous doesn't; `pg=` v10 round-trip + migration; D11
scenario; envelope pacing test still green (frames slightly *raise* worn yield — if the
income band shifts, re-pin with a note); golden voxel-hash unchanged; CI green; boundary
intact (marks content-agnostic); roadmap G20.

## Acceptance checklist

- [x] No readable English anywhere in the vocabulary layer: block display names are seeded
      lexicon words in all five scripts (internal names/codecs unchanged; recognition
      invariant re-proven).
- [x] Cartouche marks enclose name renders in world + console + codex (same marks);
      ambient never cartouched; worn keeps it; erased doesn't.
- [x] Frames: 3 sightings → known (codex skeleton + slot); worn matches restore
      (`[restored]` render distinct from `[..]`; full yield); ambiguous doesn't; erased out.
- [x] `pg=` v10 append-only; D11 scenario; envelope test green (re-pinned if shifted, noted).
- [x] Golden voxel-hash unchanged; four-way CI green; boundary intact; roadmap G20 + brief
      as-built.

## As-built (2026-07-10)

Landed as three green commits on `main` (names → cartouches → frames), each with the full
gate. Deviations + pins beyond the brief, recorded honestly:

1. **Frames are visible in raw glyphs.** The brief's sketch kept the frame in the lexicon
   (translated) layer, but a worn instance's "surviving glyphs" only exist player-side if
   the frame is in the *world text* — so frame-shaped ambient cells now **spell the frame
   verbatim, transliterated into their cell's script** (`transliterate` keeps spaces so the
   word-shape survives). This is *more* faithful to the research (§1's Grotefend foothold is
   spotted in raw glyphs, pre-comprehension); the translated display of a frame cell is the
   same frame phrase. Content change to ambient frame cells (find-ids move with it) — same
   worldgen-version policy as the name inscriptions; the golden voxel-hash is untouched.
2. **Restoration is skeleton-only (extra pin).** A worn match restores only when **every**
   lacuna falls in a fixed (skeleton) position — the varying slot is not formulaic, so a
   slot lacuna keeps the whole text lacunae (no data minted from nothing; Decision 2's
   full-yield payoff still holds because a restored text *is* complete). Also pinned: the
   match must be anchored by ≥1 surviving fixed-position glyph (word-shape alone doesn't
   restore), and only **intact** instances count as sightings.
3. **Restored render rides ASCII `[` `]`** (the literal Leiden restoration brackets) stored
   in the codex text — visually distinct from `[..]` lacunae, excluded from glyph/yield
   counts like every structural mark. The codex frame entry renders the skeleton in the
   machine's romanized transcription with the slot marked `__` (structural, no English).
4. **One engine touch beyond the two mark bitmaps:** `to_overlay` now passes *all* generic
   mark glyphs through unmapped (Runic/Galactic previously remapped any char — a latent
   bug that would have turned enclosure marks into staves). Mark-scoped; tested.
5. **Faculty research labels went lexicon too** (`ResearchTarget::glyphs`) — the research
   bar would otherwise re-leak the English word (`sensing`) that `spend(…)` hides.
6. Kept per the G12 review (out of scope here, noted for a future review): routine
   **author labels** (the given `drift`/`survey`/`prospect`/`collect` row names) and
   trigger/state instrumentation (`when(shards ≥ N)`) remain minimal-English.
