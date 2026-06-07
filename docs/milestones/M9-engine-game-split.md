# M9 — Engine / Game split (Cargo workspace)

> **Status: planned brief, not started.** This is the executable plan for separating
> brickmap-the-**engine** (a reusable, content-agnostic voxel rendering engine) from
> the **game** that has grown on top of it (the fallen-colossi exploration world). It
> is written to be picked up by a fresh agent **after** the currently-active branch
> lands, so it is self-contained and assumes nothing about in-flight work.
>
> Pairs with [`game-mechanics.md`](../game-mechanics.md) (the game's design) and the
> crate plan in [`architecture.md`](architecture.md) §3 (which this milestone finally
> executes — and extends with the engine/game cut).

## Goal · Demonstrable outcome · What it de-risks

- **Goal.** Turn the single `brickmap` crate into a **Cargo workspace** with a clean,
  graph-enforced boundary: **engine library crates** that know nothing about the
  specific world, and a **game binary crate** that supplies all content + rules +
  fiction and is the thing you actually run.
- **Demonstrable outcome.** Two things build and run from the workspace:
  1. the **engine on its own** renders a minimal demo (streamed raw terrain, no
     colossi / inscriptions / doom-drone) — proof it's genuinely content-free;
  2. the **game** builds on the engine and is **pixel-identical to today** on every
     target (desktop, web/WASM, Android, headless).
- **What it de-risks.** The project's core identity claim (design §3: "a rendering
  engine, not a game") becomes *true in the code*, not just aspirational. It also
  makes the engine independently reusable, gives the game a home to grow the core
  loop without bloating the engine, and the crate graph **mechanically prevents**
  content from leaking back into the engine.

## Why now (context)

The "engine, not a game" line in design §3 is currently **half-fiction**: the one
crate already contains a character controller (`player`), entities (`creatures`),
persistence-shaped codecs (`share`, `edit`), movement modes, and a per-seed
instrument (`audio`) — intermixed with the renderer. Meanwhile the planned 7-crate
workspace in architecture §3 has **never been executed** (`lib.rs` ≈ 107 KB,
`gfx.rs` ≈ 65 KB, all one crate). So neither "clean engine" nor "game on an engine"
is true today. This milestone makes the second one true on purpose.

The good news (see the audit below): the recent content was, perhaps by luck, built
as a **thin policy layer over generic mechanisms** — most of it already flows through
two clean engine APIs (`foliage::SplatInstance` and `edit::Edit`/`apply`). The split
is real work but the boundaries are mostly already visible. The genuine hazard is the
**four fused files** where a mechanism and a baked-in art-direction live in one module.

## The boundary principle

**Engine = mechanism. Game = policy / content / fiction.** Concretely:

- The engine provides *capabilities* and *extension points* (traits, callbacks, data
  contracts). It never decides what the world contains, how it looks, what it sounds
  like, or what the player is trying to do.
- The game decides all of that by *implementing the engine's extension points* and
  *feeding the engine data*. The game owns the binary / event loop.

Three failure modes to avoid: (a) a speculative engine *framework* with trait towers
(design/architecture both warn against this — keep extension points minimal and
earned); (b) leaving art-direction baked into the engine (the palette set, the doom
voicing); (c) the engine depending, even transitively, on the game crate.

## Module audit → target placement

Every current `src/*.rs` module and where it lands. **Engine** = a `bm-*` library
crate; **Game** = the game binary crate; **Fused** = the cut runs *inside* the file
and the file must be split (see "Fused files" below).

| Module | Today | Verdict | Target |
|---|---|---|---|
| `world.rs` | voxel data, 32³ sections, palette storage, coords | **Engine** | `bm-world` |
| `mesh.rs` | binary greedy mesher, packed vertices, draw contract | **Engine** | `bm-mesh` |
| `visibility.rs` | chunk connectivity graph (cave-cull input) | **Engine** | `bm-mesh` (build) / `bm-scene` (policy) |
| `scene.rs` | camera, frustum, controller | **Engine** | `bm-scene` |
| `gfx.rs` | wgpu device/pipelines/passes, chunk+structure draws, splat pass, uniforms | **Engine** | `bm-render` |
| `post.rs` (+`.wgsl`) | post chain (palette/dither/pixel-scale present, bloom) | **Engine** | `bm-render` |
| `particles.rs` (+`.wgsl`) | instanced particle system | **Engine** | `bm-render` |
| `textures.rs` | procedural material tile generator | **Engine** | `bm-render` |
| `sim.rs` | cellular-automata substrate | **Engine** | `bm-world` (substrate); sand *rules* are thin content → game |
| `edit.rs` | `Edit` + `apply` command seam | **Engine** | `bm-world` (or `bm-edit`) |
| `share.rs` | seed/URL codec | **Engine** | `bm-core`/`bm-share`; the **toggle-bit layout** is app-specific → game-supplied payload |
| `hud.rs` (+`.wgsl`) | bitmap-font overlay renderer | **Engine** | `bm-render`; *what's shown* → game |
| `text.rs` (+`.wgsl`) | `font8x8` rasteriser + `Script` enum + world-text billboards | **Engine** | `bm-render`/`bm-text`; *the strings* → game |
| `map.rs` (+`.wgsl`) | explored-map overlay renderer | **Engine** | `bm-render`; *markers/content* → game |
| `gamepad.rs` | controller input (web/native/android) | **Engine** | `bm-platform` |
| `audio_native.rs` | cpal output | **Engine** | `bm-platform` |
| `headless.rs` | render-to-PNG tooling | **Engine** | `bm-render` tooling; the fixed *demo scene* it composes → game/example |
| `main.rs` | native bin entry | **Game** | game bin |
| `lib.rs` | event loop + winit `ApplicationHandler` + all wiring + wasm/android entry + live-dial setters | **Fused → mostly Game** | engine systems extracted to crates; the **app/runtime + mode machine** → game |
| `worldgen.rs` | noise primitives **+** the terrain recipe (block ids, sea level, crystal chance) | **Fused** | noise → `bm-world`; recipe → game |
| `biome.rs` | two-nearest blend machinery **+** the full biome preset table | **Fused** | field/blend helper → engine; **presets** → game |
| `palette.rs` (+`.wgsl`) | gradient-map/dither apply **+** the 20 curated palettes | **Fused** | apply machinery → `bm-render`; **curated set** → game (engine ships a tiny default) |
| `audio.rs` | synth DSP **+** the Sleep/*Dopesmoker* `Drone` instrument | **Fused** | synth toolkit (optional `bm-audio`) → engine; **the dirge** → game |
| `relic.rs` | capsule-union geometry generator **+** "tube-tech giant" content | **Fused** | generator could promote to `bm-geom` later; **start whole in game** |
| `model.rs` | OBJ loader + surface sampler **+** "human-figure giant" content | **Fused** | loader/sampler could promote later; **start whole in game** |
| `creatures.rs` | swarm sim **+** decorative-wisp content | **Game** (sim is thin) | game |
| `structures.rs` | seed-grid placement of colossi + inscriptions | **Game** | game |
| `player.rs` | walking/gravity/voxel-collision character controller | **Game** | game |
| `bin/` (`drone`) | WAV-render dev tool for the dirge | **Game** | game dev tool (or engine audio example if synth promoted) |

Everything that touches `assets/base-human.obj` and the `[package.metadata.android]`
block moves with the game crate (the game is the APK).

## Target architecture

### Crate graph

Library-style (recommended, see Decision 2): **the game owns the binary** and *calls*
the engine; the engine is libraries with no `main`. `brickmap` is repurposed as the
**engine facade** so the name keeps meaning "the rendering engine."

```
                      ┌───────────────────────────────────┐
                      │   <game>  (bin + cdylib)           │  ← the binary you run
                      │   app/event-loop, mode machine,    │     (desktop/web/android)
                      │   worldgen recipe, biomes, colossi,│
                      │   inscriptions, doom dirge, player,│
                      │   creatures, codex/cipher loop …   │
                      └───────────────┬───────────────────┘
                                      │ depends on
                      ┌───────────────▼───────────────────┐
                      │   brickmap  (engine facade, rlib)  │  ← re-exports the bm-* API
                      └───────────────┬───────────────────┘
        ┌──────────────┬──────────────┼───────────────┬──────────────┐
   ┌────▼────┐   ┌─────▼─────┐   ┌────▼─────┐   ┌──────▼──────┐  ┌────▼──────┐
   │bm-render│   │ bm-scene  │   │ bm-mesh  │   │  bm-world   │  │bm-platform│
   │ wgpu,   │   │ camera,   │   │ greedy   │   │ voxels,     │  │ winit,    │
   │ passes, │   │ frustum,  │   │ mesher,  │   │ sections,   │  │ input,    │
   │ splat,  │   │ cull      │   │ vis-graph│   │ palette,    │  │ gamepad,  │
   │ post,   │   │ policy    │   │ contract │   │ noise, edit,│  │ timing,   │
   │ text,map│   └─────┬─────┘   └────┬─────┘   │ sim, WorldGen  │ audio I/O │
   │ hud,    │         │              │         │ *trait*     │  └────┬──────┘
   │ particle│         └──────────────┴────┬────┴─────────────┘       │
   └────┬────┘                             │                          │
        └──────────────────────────────────┴──────────────────────────┘
                                  ┌────▼────┐
                                  │ bm-core │  math glue, ids, prng/hash, errors
                                  └─────────┘
```

(The internal engine granularity — full 7-crate split vs. a coarser `bm-engine` —
is Decision 5. The **engine/game cut is the load-bearing boundary**; the internal
subdivision is architecture §3's existing plan and can land in the same effort or
just after.)

### The extension seams (how the game plugs in)

These are the *entire* contract surface between game and engine. Keep it this small.

1. **World generation — `WorldGen` trait (in `bm-world`).** The engine streams/meshes
   sections but does not know their contents:
   ```rust
   pub trait WorldGen: Send + Sync {
       fn fill(&self, coord: ChunkCoord, out: &mut Section);
       fn solid(&self, x: i32, y: i32, z: i32) -> bool; // foot collision + DDA pick
   }
   ```
   The game implements it with the terrain recipe + biome presets. **This single
   trait dissolves the `worldgen.rs` and `biome.rs` fusion.**
2. **Splat feed.** `bm-render` consumes `&[SplatInstance]` each frame; the game
   produces them (foliage, creatures, relics, human figures, world-text). *Already
   the de-facto API* — formalise it.
3. **Structure draws.** `bm-render` consumes chunk-instance draw lists (the existing
   `structure_draws` path) for solid relics; the game produces them.
4. **Edits/events.** `Edit` + `apply` stay the engine mutation seam; the game issues
   edits (sand, brushes, future gameplay writes). Keep `serde`-derived (multiplayer
   groundwork, roadmap N1).
5. **Look params.** `bm-render` post takes a `LookParams` (palette set + dither +
   wobble + sun + pixel-scale…); the game/biome supplies it. Engine ships a tiny
   default palette so the demo isn't blank.
6. **Audio source.** `bm-platform` audio output pulls from a `trait AudioSource {
   fn next_frame(&mut self) -> [f32; 2]; }`; the game provides the `Drone`.
7. **Runtime.** The game owns the winit `ApplicationHandler`, constructs an engine
   `Renderer`/`Scene`/streaming `World`, and drives them. The engine exposes
   *constructors + per-frame calls*, **not** a `Game` trait the engine calls back
   into (avoid the framework inversion unless Decision 2 says otherwise).

### The four fused files — extraction plan

- **`worldgen.rs`** → move the value-noise / domain-warp / ridge / hash helpers into
  `bm-world` (pure, reusable). The recipe (`STONE/DIRT/GRASS/…`, `SEA_LEVEL`,
  `CRYSTAL_CHANCE`, the height→material logic) becomes the game's `WorldGen::fill`.
- **`biome.rs`** → the low-freq field sampling + the generic "blend two presets by N
  scalars" helper can sit in engine; the **preset table** (names, palettes, spawn
  densities, drone mix, lighting, wobble) is game data feeding `LookParams` + spawn
  decisions. Cleanest: the whole `Biome`/`Blended` lives in the game, using engine
  noise.
- **`audio.rs`** → optionally extract a small synth toolkit (oscillators, waveshaper,
  one-pole/SVF filter, LFO) into `bm-audio`; the `Drone` instrument (the gains, the
  Phrygian ♭2, the chain) is the game's. **Default: keep `Drone` in the game and
  don't build `bm-audio` yet** (one consumer = no engine API earned).
- **`palette.rs`** → the gradient-map + Bayer-dither apply is engine post; the **20
  curated palettes** are art-direction → game. Engine keeps a minimal built-in ramp.

## Design sketch / build sequence

Bottom-up, **green at every step** (`cargo fmt && clippy -D warnings && test --all`
+ the wasm build, per `development.md`). Each phase is independently committable.

- **Phase 0 — Decide + document (no code move).** Resolve the Decisions below. Update
  the **stated intention**: design §3 ("the *engine* has no gameplay; gameplay lives
  in the game crate"), the architecture §3 crate table (add the engine/game cut + the
  game row), and a roadmap rung pointing here. Add the `WorldGen`/seam signatures as
  doc'd contracts. *(These edits were deliberately deferred out of the planning branch
  to avoid colliding with concurrent work — they are this phase's job.)*
- **Phase 1 — Stand up the workspace, code unmoved.** Create the workspace
  `Cargo.toml`; move the existing crate under `crates/` unchanged; confirm all four
  build targets + headless golden render are byte-identical. Pure plumbing.
- **Phase 2 — Carve the engine library crates (bottom-up).** `bm-core` → `bm-world`
  (incl. noise extracted from worldgen, `edit`, `sim` substrate, the `WorldGen`
  trait) → `bm-mesh` → `bm-scene` → `bm-render` → `bm-platform`. After each carve the
  game still compiles as the same fat top crate that now `use`s the new crate.
  Engine-internal only; no game separation yet. The crate graph starts enforcing "no
  upward deps."
- **Phase 3 — Carve the game crate + introduce the seams.** Create `<game>` crate;
  move `structures`/`relic`/`model`/`creatures`/`player` + the terrain recipe + biome
  presets + the `Drone` + the curated palettes + the app/event-loop/mode-machine +
  the wasm/android entry + the live-dial setters into it. Wire the seven seams. Gut
  `lib.rs`: engine bits already left in Phase 2; the residue is the game app.
- **Phase 4 — Prove content-freedom.** Add a tiny **engine demo** (an `examples/`
  binary or `bm-demo`) that streams flat/raw terrain via a trivial `WorldGen` with no
  game content — it must build + render against the engine crates *without depending
  on the game*. This is the real proof the boundary holds (and a permanent engine
  smoke test).
- **Phase 5 — CI + distribution.** Update the GitHub Actions (Pages WASM, Android
  APK, desktop binaries), `wasm-bindgen` target, and `cargo apk` metadata to point at
  the **game** crate. Confirm the live preview + APK + desktop builds still ship.

## Decisions to resolve (with recommended defaults)

> **Resolution (Phase 0, 2026-06-07).** Decisions **2–6 take their recommended
> defaults** (library-style; keep `bm-audio`/`bm-geom` generators in the game for
> now — extract only noise + palette apply; **monorepo** path-dep crate, Decision 4
> already firmed; do the full §3 7-crate split in Phase 2; engine keeps the runnable
> Phase 4 demo). **Decision 1 (the game's name) — RESOLVED 2026-06-07: the game is
> *Scraped Again*** (crate `scraped-again`, lib `scraped_again`). The engine stays
> `brickmap`. Phase 3 creates `crates/scraped-again` as the binary/cdylib.

1. **Game name. → Resolved: *Scraped Again*.** brickmap stays the engine; the game is
   the Cargo package `scraped-again` (lib `scraped_again`) — the binary you run, the
   APK, the Pages app. The fiction is "a lonely surveyor of a dead world of fallen
   giants."
2. **Library-style vs framework-style.** Game-owns-`main` and calls the engine
   (library) **vs** engine-owns-`main` and calls a `Game` trait (framework).
   *Default: **library-style*** — simpler, matches "a game that uses the engine," and
   dodges the speculative-trait-tower the design/architecture docs warn against.
   Revisit only if a second game appears.
3. **Where do the fused *mechanisms* land?** Extract `bm-audio` (synth) and `bm-geom`
   (capsule/OBJ samplers) now, or keep those generators in the game until a second
   consumer exists? *Default: **keep them in the game***; extract **only** the noise
   helpers (engine genuinely needs them) and the palette/dither apply. Promote others
   when earned.
4. **Monorepo vs separate repo for the game.** **Resolved: one workspace (monorepo),
   game as a path-dep crate** — `crates/bm-* + crates/<game>`, with `<game>`
   depending on the `brickmap` engine facade via a `path` dependency. Rationale:
   **crate boundaries enforce *layering*; repo boundaries enforce independent *release
   cadence + ownership*.** We need the layering (the workspace DAG + a no-upward-dep CI
   check enforce it as hard as separate repos would), but the engine and game co-evolve
   tightly with a single consumer and one author — exactly where a monorepo wins and a
   multi-repo's publish-tag-bump dance hurts on every change. The game still genuinely
   "consumes the engine as a crate"; it just lives in the same workspace. This also
   keeps the atomic cross-boundary refactors that Decision 3 anticipates trivial, and
   one home for the golden-image + voxel-hash tests that span both halves.
   - **It doesn't lock us in.** A monorepo keeps extraction cheap (`git filter-repo`
     the `crates/bm-*` out, switch the game to a git/registry dep); a premature split
     spends real friction now and is harder to undo.
   - **Flip-triggers — revisit a separate engine repo only when one of these is true:**
     (a) a genuine **second consumer** (another game, or an external user) appears;
     (b) we want to **publish the engine on crates.io with its own semver**; or
     (c) the engine goes **stable / low-churn while the game churns fast**, so a pinned
     released engine + a separate game repo stops being friction and starts being
     hygiene.
5. **Engine crate granularity.** Full 7-crate split (architecture §3) **vs** a single
   `bm-engine` library first, subdivided later. *Default: **do the §3 split** as part
   of Phase 2* (you're moving everything anyway), but landing a single `bm-engine`
   first and subdividing later is acceptable if Phase 2 gets heavy — the **engine/game
   cut is the milestone; the internal granularity is secondary.**
6. **Does the engine keep a runnable demo?** *Default: **yes*** (Phase 4) — it makes
   "rendering engine, not a game" literally true and doubles as the engine's smoke
   test + golden-image host.

## Tests

The split is behaviour-preserving, so the tests are mostly **"prove nothing changed"**
plus **"prove the boundary exists"**:

- **Golden-image headless render** identical (within tolerance) before vs after, for a
  fixed seed/camera — the primary regression guard (D1).
- **Golden voxel-hash determinism** (E12) unchanged across the worldgen extraction —
  the recipe must produce bit-identical worlds after moving behind `WorldGen`.
- **All existing unit tests pass** in their new crate homes (mesher correctness,
  palette pack/unpack, packed-vertex round-trip, visibility graph, player physics,
  share codec, audio finiteness, creature determinism, etc.).
- **All four build targets green:** desktop native, web/WASM (`wasm-bindgen`),
  Android (`cargo apk`), headless. CI updated to build the workspace.
- **Boundary enforced by the crate graph:** no engine crate depends on the game
  (checked by the DAG itself); add a CI assertion / `cargo-deny`-style check or a
  simple test that the engine crates have no game dep.
- **Engine-alone demo builds** without the game crate present (Phase 4) — the
  strongest content-freedom proof.

## Risks & mitigations

- **Concurrent churn / merge hell.** This touches nearly every file. *Mitigation:*
  start **only after** the active branch lands; do it as one focused effort on a
  dedicated branch; land Phases 1–2 fast to minimise the divergence window.
- **The fused-file extractions change behaviour.** Pulling the terrain recipe behind
  `WorldGen` or the look behind `LookParams` can subtly shift output. *Mitigation:*
  golden-image + voxel-hash tests gate every fused extraction; do them one at a time.
- **wasm/android entry points.** The `cdylib`, `#[wasm_bindgen]` `start` + live-dial
  setters, and `android_main` must end up in the **game** crate (the thing each
  platform loads), and `[lib] crate-type` / `cargo-apk` metadata move with it.
  *Mitigation:* Phase 5 explicitly re-targets CI; verify the deployed Pages build +
  a built APK + a desktop binary, not just `cargo build`.
- **Over-abstraction.** Easy to grow an engine framework with hooks nobody needs.
  *Mitigation:* the seven seams in this brief are the *whole* contract; anything more
  must be justified against a real need (design §"don't over-abstract").
- **Compile-time / circular-dep regressions.** *Mitigation:* enforce the DAG; keep
  `bm-core` dependency-light; measure clean-build time before/after.
- **Engine "owns the look" question.** The palette/dither *capability* is engine, but
  the *curated palettes* are content. *Mitigation:* Decision 3's default (apply in
  engine, set in game, tiny built-in default) — revisit if it feels wrong in the demo.

## Acceptance checklist

- [x] Decisions 1–6 resolved and recorded; design §3 + architecture §3 + a roadmap
      rung updated to describe the engine/game split (Phase 0). *(Decision 1 — game
      name — deferred to the human before Phase 3, per the resolution note.)*
- [ ] Cargo **workspace** in place; engine is library crates with **no `main`**; the
      game crate owns the binary + wasm/android entry.
- [ ] The **seven extension seams** implemented; the four fused files split per plan
      (`worldgen`/`biome`/`palette` extracted; `audio`/`relic`/`model` placed per
      Decision 3).
- [ ] **Crate graph proves the boundary**: no engine crate depends on the game
      (CI-checked).
- [ ] **Engine-alone demo** renders streamed terrain with zero game content (Phase 4).
- [ ] Game is **behaviour-identical to today**: golden image + voxel-hash unchanged.
- [ ] **All four targets** build + run; CI (fmt/clippy `-D warnings`/test/wasm/apk/
      desktop) green; the **live Pages preview, APK, and desktop binary** verified.
- [ ] `architecture.md` §7 "current vs target" updated to reflect the realised
      workspace.

## Out of scope / follow-ups

- The **game's core loop** (codex/cipher/objectives) — that's the
  [`game-mechanics.md`](../game-mechanics.md) milestone, built *after* this split
  gives it a home.
- Promoting `bm-audio` / `bm-geom` to engine crates (do when a second consumer
  appears — Decision 3).
- Publishing the engine crates to crates.io / a stable public API — not needed for an
  internal monorepo; revisit only if the engine is reused elsewhere.
- Any rename of the GitHub repo / Pages URL — cosmetic; decide alongside the game name.
