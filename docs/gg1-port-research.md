# GG1-on-brickmap — port research (Builder B, 2026-06-23)

> **Strand 3 of the architecture pivot.** Research-only answer to
> `halves:docs/agent/BRICKMAP-GG1.md`. Builder B has `00-1/brickmap` access (the
> Babysitter does not); this report is the evidence base for `BRICKMAP-GG1-SPEC.md`.
> **No engine or game code was changed.** Findings are cited to live files at
> `main`/`claude/halves-visual-fx-engine-yvflpa` (even with `origin/main`).
>
> "GG-web" = the shipped Goblin Gold v1 in `00-1/halves` (no-build vanilla-JS PWA;
> 46 topics / 959 questions; modules incl. `fxgl.js`, `synth.js`, `emblems.js`,
> `main.js`, `glyphs.js`, `sound.js`, `scenery.js`).

---

## TL;DR — verdict

**Brickmap is a strong, *honest* re-platform target for GG1 — but GG1 is a 2-D
text+UI+audio game, and brickmap is a *voxel* engine, so GG1 would use brickmap's
*presentation half* (render overlays, palette/dither post, particles, text, audio I/O,
input, platform) and almost none of its *voxel half* (world-gen, sections, greedy
mesher, visibility culling).** That is the key framing: this is not "GG drives the voxel
renderer", it's "GG is a flat game on the engine's 2-D/FX/platform services."

Three things land in our favour, three are real new work:

**Tailwinds**
1. **The FX is coming home.** GG-web's `fxgl.js` was *ported FROM* brickmap's WGSL
   (Bayer-dither palette gradient-map, instanced splats, xorshift). The engine renders
   GG's exact visual identity natively — `palette.wgsl`, `splat.wgsl`, `particles.wgsl`
   are the originals. Don't port `fxgl.js`; consume the recipes.
2. **The audio DSP already exists in Rust.** `scraped-again`'s `Drone` is a dependency-
   free generative synth (detuned oscillators, PolyBLEP saw, state-variable filter, a
   stable Hadamard **FDN reverb** — the same DSP family as GG's `synth.js`) feeding
   cpal (native/Android) + a Web-Audio ScriptProcessor (web) as f32 sample blocks.
3. **Headless render-to-PNG works today** (llvmpipe software Vulkan). The "self-verify
   pixels instead of DOM whack-a-mole" bet is real — the capture path exists; only the
   *golden-diff assertion* layer is missing (and GG already has that discipline in JS).

**Headwinds (new *engine* work — exactly the "bank reusable text+menus" the owner asked for)**
1. **Text legibility.** Today text is `font8x8` (8×8 bitmap, integer nearest-neighbour,
   no AA/SDF/TrueType). Fine for HUD numbers/short labels; **marginal for GG's dense
   guide/explain prose** at 11+ reading sizes. A legible font path (SDF or a baked
   TrueType atlas) is the #1 dependency and must land first.
2. **UI/menus.** There is **no** retained/immediate UI framework — only raw text +
   filled-rect primitives + a touch hit-test layout. GG's menus, settings, and the
   **numeric keypad** are from-scratch (build them *into the engine*, per owner).
3. **Persistence & a11y.** No localStorage/file/SharedPreferences save abstraction;
   no screen-reader story (opaque GPU canvas). Both are new, and a11y is a genuine
   *regression risk* vs the accessible DOM of GG-web for a school audience.

**Bottom line:** feasible and strategically right (engine hardened by a 2nd, very
different game; GG2 intended brickmap-native). It is a **re-platform, not a port** —
budget for it as such. Recommended next step: a tightly-scoped spike (below) that
proves crisp text + one drill + one self-verified FX moment + a clean APK launch,
gated before any full port.

---

## A. Engine architecture & capability inventory

### A1. Architecture, render pipeline, platform targets

**Cargo workspace** (the M9 "engine/game split" is *done* and on `main`):

```
crates/
  bm-core      math/POD glue (glam, bytemuck), IDs, errors
  bm-world     voxel data model: 32³ sections, palette storage, coords, noise worldgen, edits, CA sim — exposes the WorldGen trait
  bm-mesh      binary greedy mesher (packed ~4-byte vertices) + per-chunk visibility graph
  bm-scene     camera, view/proj, frustum extraction, culling policy
  bm-render    wgpu device/pipelines/passes; splat + post chain; text/hud/map overlays; particles; materials  ← GG uses THIS crate
  bm-platform  platform edges: winit + input (touch/gamepad) across native/web/android  ← and THIS
  brickmap     facade lib (rlib) — re-exports the bm-* DAG as one import surface
  scraped-again the game (cdylib+rlib) — owns the binary + web/android entry + the winit loop
```
Render: **wgpu 29**, forward rendering, rasterised greedy meshing. Targets, **one code
path**: native desktop (winit), **web** (`wasm32-unknown-unknown`, WebGPU with a
**WebGL2 fallback** via `wgpu` `webgl` feature), **Android APK** (winit
`android-native-activity`, built with `cargo-apk`). Auto-deploys to GitHub Pages on
`main`. *(refs: root `Cargo.toml`; each `crates/*/Cargo.toml`; `crates/brickmap/src/lib.rs`; `docs/architecture.md` §2–3, §6.)*

### A2. Capability matrix (exists / partial / missing)

| Subsystem | Status | Evidence & notes |
|---|---|---|
| **Text / font** | **PARTIAL (the crux)** | `bm-render/src/{text,hud}.rs` + `font8x8` 8×8 bitmap. HUD scale `= (surface_h/360).max(2)` → ~24px @1080p; in-world billboards (`text.wgsl`, 5 scripts). **No SDF/TrueType/AA**; integer nearest-neighbour only. HUD numbers/short labels legible; **dense prose marginal** (G12 brief itself says "sizing is eye-pass"). |
| **UI / menus** | **MISSING (framework) / PARTIAL (primitives)** | No retained or immediate-mode GUI, no layout, no button-with-label, no keypad widget. Primitives exist: `HudOverlay` (text), `RectOverlay` (alpha-blended filled rects, `hud_rect.wgsl`), `WorldText`. The `scraped-again` "console" is a text list, not a widget toolkit. |
| **Input** | **PARTIAL→good** | `bm-platform`: touch (`touch.rs`, normalised `TouchPoint`), gamepad (`gilrs`/web Gamepad API/android), keyboard via winit. `scraped-again/touch.rs` has a hit-tested overlay `Layout` (sliders+buttons) + `overlay_rects()` — directly reusable to build a numeric keypad. Digit keys already wired (currently to debug toggles). **No keypad/settings widget yet.** |
| **Audio** | **EXISTS (mature, generative)** | `scraped-again/src/audio.rs` `Drone`: detuned oscillators, PolyBLEP saw, **SVF** filter, LFOs, **4-line Hadamard FDN reverb**, per-seed PRNG → stereo `next_frame()->[f32;2]`. I/O: `audio_native.rs` (cpal; Android AAudio, minSDK 26) + web (ScriptProcessor pulls blocks via wasm). Lock-free reactive params. **Synth only — no sample player.** |
| **Persistence / save** | **MISSING (only URL/seed codec)** | `share.rs` encodes seed+camera+toggles to a URL fragment (E12). **No** localStorage, **no** native file/prefs, **no** Android SharedPreferences. Settings/progress are not saved. (Each is small to add per target.) |
| **Game loop / scene / entity** | **EXISTS (no ECS)** | Game owns the `winit::ApplicationHandler` loop; per-frame `run_frame(dt)` → gather inputs/state → call `state.render(...)`. Data-driven with manual borrows; "scenes" = mode switches, not a scene graph. |
| **Particles / FX** | **EXISTS (this is GG's identity, natively)** | `bm-render/src/*.wgsl`: `palette` (Bayer-4×4 ordered dither + small-palette luminance gradient-map — **the exact recipe `fxgl.js` borrowed**), `splat` (instanced billboards, wind sway, dither alpha), `particles` (emissive instanced cubes), `post` (bloom), `sky`, `shader` (terrain w/ dissolve), `overlay` (lines/rects), `text`, `map`, `ship`. |
| **Asset / texture pipeline** | **PARTIAL (procedural-friendly)** | 2-D texture-array materials, procedural materials, palette post. No generic "rasterise a CPU buffer → texture for a 2-D sprite/emblem" helper yet, but `hud.rs` already builds an RGBA buffer → GPU texture (the pattern GG's generative emblems/icons need). |

### A3. "Scraped Again" — the engine↔game boundary (the template for GG1)

The boundary is enforced by the crate graph (no `bm-*` may depend on the game; CI-checked
via `cargo tree`) and realised as **7 seams** (per `docs/milestones/M9-engine-game-split.md`):

1. **WorldGen trait** (`bm-world`) — game fills 32³ sections + answers `solid()`.
2. **Splat feed** — game passes `&[SplatInstance]{offset,size,color,sway,alpha}` per frame.
3. **Structure draws** — game passes solid meshes (`ChunkInstance`).
4. **Edit command** (`bm-world::Edit`) — voxel edits / CA sim.
5. **LookParams** — `render(...)` takes per-frame aesthetic dials `aesthetic:[wobble,steps]`,
   `toggles`, `sun`, `ink`, `murk`.
6. **Audio** — kept wholly in the game (no engine trait yet).
7. **Runtime** — the game owns the winit loop and calls engine fns.

`crates/brickmap/examples/engine_demo.rs` is the minimal skeleton (a `FlatGen` WorldGen +
an `ApplicationHandler` that streams a few chunks and calls `state.render(...)` with default
dials) and doubles as the CI proof the engine builds with **zero game content**.

> **GG1 implication:** seams **1–4** are *voxel* seams GG1 barely needs (GG is flat). GG1's
> real boundary is **seam 7 (runtime) + a 2-D presentation surface**: `state.render` 2-D
> overlay layers (`hud`/`overlay`/`rect`/`text`/`particles`) over a cheap background +
> `palette` post, plus audio (seam 6) and input. So GG1 exercises a *different, thinner*
> slice of the engine than Scraped Again — which is exactly why it hardens the engine's
> presentation half (text/menus/2-D), the part the owner wants banked as reusable.

### A3b. Workspace & the future `gg-kit` slot

Already a workspace, so a second game is **additive**: add `crates/goblin-gold`
(`cdylib`+`rlib`, `brickmap` path dep). No restructuring; no crates.io. The existing
`brickmap` facade + `scraped-again` pattern is the template. `gg-kit` (the GG-genre
framework) should **not** be a crate yet — build GG1 honouring the PACK contract so the
seam is already real, and extract `gg-kit` later (when GG2 is the 2nd data point) as a
file-move. The five design-for-extraction rules in `BRICKMAP-GG1.md` are achievable as-is
(one-way deps + the no-leakage grep gate are already how `scraped-again` is built).

### A3c. Which old-CORE "engine half" items brickmap ALREADY provides (sizes how thin gg-kit gets)

| old-CORE engine-half item | brickmap today | Sinks into brickmap? |
|---|---|---|
| FX / particles / dither-palette (`fxgl`) | ✅ `palette`/`splat`/`particles` WGSL | **Yes — already there** |
| Audio engine+synth (`sound`+`synth`) | ✅ `Drone` DSP + cpal/web I/O | **Yes** (re-author GG instruments onto it) |
| Icons / generative images (`emblems`,`icons`) | ⚠ partial (RGBA→texture pattern, procedural materials) | Yes, with a small sprite/texture helper |
| Text rendering | ⚠ `font8x8` only — **needs an upgrade** | **Yes (new engine work)** |
| Menus / UI shell | ❌ primitives only | **Yes (new engine work)** |
| Input dispatch | ✅ touch/gamepad/keyboard edges | Yes |
| Save / persistence | ❌ URL codec only | **Yes (new engine work)** |

Net: brickmap already eats most of the engine half; the new sinks (text, menus, save) are
the owner's explicit "bank reusable" targets. `gg-kit` stays thin (round loop, progression/
mastery/unlock chain, collection-ladder math, T218 nav-badges, the PACK contract).

---

## B. Reuse feasibility

### B4. Can the web target run JS? Embed a JS engine on native? — straight verdict

- **Web:** there is heavy **wasm-bindgen ↔ JS glue** (`scraped-again` exports ~15
  `#[wasm_bindgen]` fns; the web page drives a Web-Audio callback into wasm). That is
  Rust↔browser interop, **not** "run arbitrary JS logic." There is **no** embedded JS
  runtime and **no** Node anywhere; native/Android have zero JS.
- **Embedding a JS engine (quickjs/boa/deno_core) on native to run GG-web's JS as-is:**
  technically possible, **but a bad idea** — verdict: **don't.** It bloats the APK, drags
  a second language + GC into a Rust binary, kills the headless-pixel/parity-test win
  (you'd be testing JS-in-Rust), and forks behaviour between web (real JS) and native
  (embedded JS). It buys nothing the shared **data export + parity vectors** doesn't.

**→ Re-implement GG logic in Rust against the exported data + parity vectors. Share DATA, not code.**

### B5. Best port path per GG-web asset

| GG-web asset | Recommended path | Why |
|---|---|---|
| **Game logic** (transforms / modes / events / collector / drill loop / arena) | **Re-implement in Rust** against **T229's parity vectors** (`input → {p,a}`) + content data. | Logic ≠ content (content is the data export). Rust port is small, fully testable, and the parity vectors *prove* behaviour parity exactly. Embedding JS is rejected (B4). |
| **Audio** (`synth.js` / `sound.js`, Web-Audio node graph) | **Re-author as a Rust sample synth onto brickmap's audio** (`Drone`-style `next_frame()` + cpal/web I/O). | brickmap's model is *block-of-samples* DSP, not a node graph; but the primitives GG uses (oscillators, filters, FDN reverb, envelopes) already exist in `audio.rs`. Re-cast GG's instruments/sequencer as a deterministic Rust synth driven by a **music/SFX data spec**. (Parity is perceptual, not vector-testable → port to spec, A/B by ear; render-to-WAV bin already exists for iteration.) |
| **Generative images** (`emblems.js`, icon/hero/monster art) | **Engine sprite/texture helper + game-side generators.** Rasterise the deterministic cell-grids to an RGBA buffer → upload as a texture (the `hud.rs` pattern), drawn via the 2-D overlay; or bake at build time. Belongs in the **game** (content), with a thin **engine** "CPU-buffer → sprite" helper banked as reusable. | Keeps generators as data-driven game content; the only engine add is a generic sprite path (small). |
| **FX / backdrop** (`fxgl.js`) | **Do not port — use `bm-render` directly.** Author GG scenes against `palette`/`splat`/`particles`. | `fxgl.js` *is* a JS re-implementation of these exact recipes; going to brickmap reverts to the originals. |

---

## C. Tooling, verification, a11y

### C6. Build toolchain (web + native APK) + CI

- **Web:** `cargo build -p <game> --release --lib --target wasm32-unknown-unknown` →
  `wasm-bindgen --target web` → static `pkg/` served from a hand-written `index.html`
  (no bundler). Auto-deploys to GitHub Pages on `main`. *(`.github/workflows/deploy.yml`, `web/index.html`.)*
- **Android APK:** **`cargo-apk`** (NDK r26d, `aarch64-linux-android`, minSDK 26 for
  AAudio, target 34), `[package.metadata.android]` in the game `Cargo.toml`, signed with a
  committed keystore + zipalign/apksigner. *(`.github/workflows/android.yml`, `scraped-again/Cargo.toml`.)*
- **Desktop:** `cargo build --release` (default member = the game).
- **CI is real and green-gated for all three:** `ci.yml` runs `fmt --check`, `clippy
  --workspace --all-targets -D warnings`, `cargo test --all`, the **engine↔game boundary
  check**, the `engine_demo` build, and a **wasm build**; separate workflows build+publish
  the **APK** and desktop binaries to a rolling Release. **A GitHub-Actions APK build is
  not just realistic — it already exists.**

### C7. Self-verifying rendering (the workflow bet) — confirmed, with a gap

- **It works:** `scraped-again/src/headless.rs` (`capture_view(...)`) renders offscreen via
  **wgpu on software Vulkan (Mesa llvmpipe)** and writes a **PNG** (`png` crate). Driven by
  the `screenshot` bin (`cargo run --bin screenshot -- out.png W H ...`) and exercised by
  `e2e.rs` (a 2000-tick headless game-loop test + a `render_robustness_sweep` that asserts
  non-empty PNGs across camera angles). Needs `mesa-vulkan-drivers` on the runner.
- **The gap:** there is **no golden-image pixel-diff** yet — current asserts are
  "render didn't panic / file non-empty," not "pixels match a blessed golden." **This is
  the single highest-leverage thing GG1 brings:** GG-web already runs a golden-image
  discipline (`UPDATE_GOLDEN`, blessed JSON, teeth tests). Porting that as a brickmap
  golden-PNG harness turns "render headless + assert on pixels" from possible into routine —
  and it's the explicit reason for the pivot. **Recommend it be in the spike.**

### C8. Accessibility — the honest gap

Brickmap renders to an **opaque GPU canvas**: **no** `aria-*`, **no** DOM mirror on web,
**no** AccessKit/TalkBack on native (grep: zero hits). GG-web, being DOM/text, was far more
screen-reader-amenable. For an **11+ school audience this is a real regression risk**, not a
nice-to-have. Options (both **new work**): a **web DOM-mirror** (hidden semantic elements +
ARIA-live mirroring the current question/answer/result alongside the canvas) and **native
AccessKit** (Rust a11y → exposes UIA/AT-SPI/TalkBack). Neither is trivial; flag for an owner
decision in the spec (it may gate "ship to schools").

---

## D. Synthesis

### D1. Capability matrix → one line
Render/FX/dither-palette ✅ · audio DSP ✅ · headless render ✅ · input edges ✅ · workspace/boundary ✅ ·
**text (legible prose) ⚠→new · UI/menus/keypad ❌→new · persistence ❌→new · golden-diff ❌→new · a11y ❌→new.**

### D2. GG1 subsystems → brickmap (what exists / new *engine* work / *game* work)

| GG1 subsystem | Lands as | Bucket |
|---|---|---|
| Backdrop FX, dither/palette, particles, coin shower/hoard | `bm-render` palette/splat/particles (author GG scenes) | **exists** (engine) |
| Music + SFX | re-author onto `Drone`-style Rust synth + cpal/web I/O | game (engine I/O exists) |
| Generative emblems/icons/art | CPU-buffer→sprite (engine helper) + Rust generators | small engine + **game** |
| Legible question/guide/explain text | **SDF or baked-TTF font path** | **new ENGINE work (do first)** |
| Menus, settings, **numeric keypad** | retained/immediate UI layer + widgets | **new ENGINE work** (banked reusable) |
| Save (progress, settings, collection) | localStorage / fs / SharedPreferences abstraction | **new ENGINE work** (`bm-platform`) |
| Modes / transforms / arena / collector / drill loop / mastery / unlocks / nav-badges | re-implement in Rust vs parity vectors | game now → **`gg-kit` later** |
| Content (46 topics / 959 Qs, guides, balance, collectibles) | consume T229 data export + parity vectors (one-way) | data (`gg1-pack`) |
| Golden-image self-verify | extend `headless.rs` with pixel-diff goldens | **new ENGINE work** (high leverage) |
| a11y (DOM-mirror / AccessKit) | new, per owner decision | **new ENGINE work** (risk) |

### D3. Proposed engine↔game boundary + where it lives
- **Lives in `00-1/brickmap`** as `crates/goblin-gold` (`cdylib`+`rlib`, `brickmap` path dep),
  alongside `scraped-again`. No new repo; Rust stays out of `halves`.
- **Boundary:** GG1 owns the winit loop (seam 7) + game state + the PACK data; it calls the
  engine's **2-D presentation** services (`hud`/`overlay`/`rect`/`text`/`particles` + `palette`
  post), **audio**, **input**, and the **new** text/UI/save services. It does **not** use the
  voxel seams (WorldGen/mesh/edit) beyond, optionally, a cheap voxel/scene backdrop. Honour the
  PACK contract + one-way deps + the no-leakage grep gate from day one so `gg-kit` extracts cleanly later.

### D4. Proposed spike scope + effort + blockers

**Spike goal (smallest slice that de-risks the whole bet): "one legible drill, self-verified, on a phone."**
A `crates/goblin-gold` skeleton that, on **native + web + APK**:
1. **Crisp text** — renders one question + a short guide paragraph in a **new legible font
   path** (SDF or baked-TTF atlas) at reading size. *(Proves the #1 blocker.)*
2. **Drill loop** — accepts a numeric answer via an **on-screen keypad** (touch) + digit keys,
   marks right/wrong. *(Proves UI/keypad + input.)*
3. **One self-verified FX moment** — a correct-answer palette/particle flourish, **asserted by
   a headless golden-PNG test**. *(Proves the self-verify win + FX reuse.)*
4. **Clean native-APK launch** — boots fullscreen on a real phone, no voxel world required.

**Effort (rough, engineering-days, excludes the full content/logic port):**
- Font/SDF path (engine): **4–7 d** (the long pole; pick SDF-atlas vs baked-TTF first).
- Minimal UI + keypad widget (engine): **3–5 d**.
- `goblin-gold` crate skeleton + 2-D scene + one drill + input wiring (game): **3–4 d**.
- Golden-PNG harness extension to `headless.rs` (engine): **2–3 d**.
- APK/web/CI wiring for the new crate (mostly copy `scraped-again`): **1–2 d**.
- **Spike total ≈ 13–21 engineering-days** (~3–4 weeks of focused work).

**Blockers / risks (call before committing to the full port):**
1. **Font legibility** is a hard dependency — if SDF/TTF in this stack is fussier than
   expected, the whole text-heavy game wobbles. *Spike must clear this first.*
2. **a11y regression** for schools (opaque canvas vs accessible DOM) — owner decision; may
   require the DOM-mirror/AccessKit work to even ship to the audience.
3. **Audio parity is perceptual**, not vector-provable — re-author to a spec + A/B by ear;
   accept "faithful, not bit-identical."
4. **Scope honesty** — this is a **re-platform**, not a port: re-implement logic in Rust,
   re-author audio, build text+UI+save+a11y. The payoff (engine hardened by 2 games,
   self-verifiable rendering, native APK, GG2-native foundation) is real, but budget it as
   weeks-to-months, not days. Recommend a **go/no-go gate after the spike.**

---

*Builder B · research-only pass · no engine/game code changed · for `BRICKMAP-GG1-SPEC.md`.*
