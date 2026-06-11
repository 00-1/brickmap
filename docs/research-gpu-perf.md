# Research — weak-hardware GPU performance (vendor-doc pass)

> 2026-06-11 research pass (web; vendor docs + primary repos). The operational findings for
> our reference targets (Mali-G78 phone · Intel Iris Xe · browsers), each with a concrete
> action for the engine. Feeds [`performance.md`](performance.md) and the M-series. Siblings:
> [`research-voxel-rendering.md`](research-voxel-rendering.md),
> [`research-points-splatting.md`](research-points-splatting.md).

## 1. The cross-vendor invariant (Arm + Qualcomm + Apple agree)

A render pass should **clear (not load) on entry and store only what's read later**. Arm
measured the costs on G76-class hardware: `LOAD_OP_LOAD` ≈ **600 MiB/s** of read traffic at
1080p60; storing an unread depth attachment ≈ **555 MiB/s** of writes; in-pass clear
commands cost ~6 M extra fragment cycles/s vs a free `loadOp=CLEAR`. Qualcomm's GMEM
"unresolves" and Apple's load/store actions are the same physics. **We already do
clear+depth-Discard** — the residual risks are the *silent* disablers below.
*(Sources: Arm-authored Khronos Vulkan-Samples render_passes/subpasses/afbc READMEs
[fetched]; Qualcomm Game Developer Guide + "Avoid GMEM loads" profiler rule; Apple Metal
Best Practices + WWDC19/20 sessions.)*

**Actions:**
- **A1 — usage-flag audit (cheap, real):** AFBC (Mali) is disabled by `STORAGE` usage /
  mutable-format/alias flags — worth ~33–50% of color write bandwidth; UBWC (Adreno) and
  Apple lossless compression have the same trap; Intel likewise ("do not set UAV bind flags
  you don't need" — kills CCS compression). **Audit every render-target's
  `TextureUsages`** — declare only `RENDER_ATTACHMENT | TEXTURE_BINDING`.
- **A2 — keep the frame one uninterrupted pass** (no mid-frame target switches/readbacks);
  Qualcomm flags intra-frame GMEM resolves as a named profiler violation.
- **A3 — the budget number:** Arm's planning figure is ~80–100 mW per GB/s of DRAM traffic
  against a ~1 W mobile GPU budget ⇒ **aim well under ~100 MB/frame total** at 60 fps.
  (Pixel 6a ground truth = Streamline "Output External Read/Write Bytes".) This belongs in
  `performance.md` §6 as the bandwidth budget M8b measures against.

## 2. Fragment discard / alpha-test — the highest-value finding (partially contradicts us)

Vendor-doc specifics, per family:
- **Mali (our Pixel 6a)**: a draw using `discard` / alpha-to-coverage / `gl_FragDepth`
  **skips early-ZS and falls back to late-ZS**, with deferred depth writes that also weaken
  culling of *subsequent* draws; FPK can't kill fragments behind unknown coverage. Arm's
  best-practice list literally says avoid shader-discard draws. **Alpha-to-coverage is NOT
  an escape hatch on Mali** (same late-ZS path) — the desktop advice doesn't transfer. New
  Mali (G925 fragment-prepass HSR) makes the opaque/discard split matter *more*.
- **Adreno**: Qualcomm calls discard "very expensive"; LRZ *writes* are disabled for
  discard draws (they still test against prior opaque depth — so discard draws don't
  poison the pass, but they themselves shade fully).
- **PowerVR**: the vendor doc is literally titled "Do Not Use Discard" (HSR feedback loop).
- **Intel Xe**: mildest — early-Z lost for those draws, no HSR to break.

**Implications for us:** the *blend-avoidance* rationale stays sound (blending also kills
FPK/LRZ-writes + costs RMW bandwidth), but our dither/discard splats are **not free**:
every dithered fragment shades at late-ZS. The documented mitigations, in fit order:
1. **A4 — draw order is the whole game:** all opaque first (front-to-back — we do), the
   discard/dither material **last in the same pass**, never interleaved. This confines the
   damage and is free.
2. **Tight billboards:** shrink splat quads to the dither footprint — fewer late-ZS
   fragments is the only universal win (a measurable M8b item).
3. **Do NOT reach for alpha-to-coverage or a depth-equal prepass** — A2C is a no-op on
   Mali, and Arm advises against prepass techniques on tilers (doubled binning/vertex work
   usually nets negative). Keep depth-tested discard and accept late-ZS.
4. The `melt` terrain mode puts discard in the *terrain* shader while active — an opt-in
   look with a now-known early-Z tax; note it in the toggle docs, measure at M8b.

Also: practical mobile draw budgets remain low (~200/frame low-end, ~500 high-end before
submission+binning overhead bites; WebGL2 chokes around low hundreds) — our
one-draw-per-chunk + M10's mesh-draw counter (≤1,200 budget) should be checked against
this on-device; the forest scene's 663 colossus sections say merging is the first lever.

## 3. Intel Iris Xe specifics

- **Hybrid tiler:** Gen11+ has an optional position-only binning pipe (PTBR) using L3 for
  tile data; binning-friendly = pure tri-lists, no mid-pass hazards, store-op discipline,
  **position split into its own vertex-buffer slot**.
- **fp16 is double-rate** on Xe-LP; FP64 removed. Mali Valhall is also 2× fp16, and
  **mediump halves register pressure** (occupancy halves above 32 registers/thread).
- **The internal buffer can ride the cache:** Xe-LP has up to 16 MB GPU L3 (TGL ships
  3.8 MB) at 128 B/cycle — a 960×540 RGBA8 target (~2 MB) substantially fits, converting
  DRAM writes to on-die traffic. Validates the pixel-scale dial beyond fragment savings.
- **Bandwidth reality:** Iris Xe class = 51–68 GB/s theoretical *shared with the CPU*;
  measured ~57 GB/s all-cores; budget ~40–50 GB/s for GPU while our meshing threads run ⇒
  ~0.7–0.8 GB/frame ceiling at 60 — the M10/M8b context numbers.
- **Fast clears are metadata-only** when you use the pass `LoadOp::Clear` (we do) — never a
  separate clear.

**Actions:** **A5 — two-stream vertex layout** (position+AO stream / rest stream) serves
both Intel's binner and Mali IDVS (positions-first shading pulls only position bytes; ~50%
of varying work culled). **A6 — fp16-first WGSL** for fragment color/fog/dither math with
an f32 fallback variant (WebGPU `shader-f16` is a feature; absent on Adreno browsers — must
be dual-path, never baked in).

## 4. WebGPU / wgpu specifics

- **wgpu-core overhead** is ~5–10% over raw Vulkan in realistic draw mixes (maintainer
  figure) — per-chunk draws are fine natively; keep one bind-group set per draw max, group 0
  = per-frame data (bind-group *setting* is not optimized away).
- **`Queue::write_buffer` allocates a staging buffer per call** (an extra copy — and issue
  #3698: the skip-staging-on-UMA optimization our iGPU targets would love is still open);
  unthrottled buffer creation caused measured ~25 ms hitches (issue #1242). For static chunk
  meshes prefer **`mappedAtCreation`** (one write, no staging hop); `StagingBelt` only for
  many small writes. **A7 — audit our upload path** (see the voxel-rendering doc §3 — same
  conclusion from the Sodium/Veloren side).
- **Web is a different cost model:** every call crosses a serialize→IPC→validate boundary
  (Chrome Dawn wire); 500 k individual indirect draws ran 17 fps vs 55 native *without*
  validation (gpuweb #5175). **Render bundles** skip re-validation/re-encoding —
  Babylon.js measured up to ~10× submission speed on static scenes. **Multi-draw-indirect
  is NOT portable** (Chrome experimental flag only). **A8 — web path: rebuild a render
  bundle only on chunk-set change**; on WebGL2 fallback, instancing/merging is the only
  batching tool.
- **Timestamp queries are quantized to 100 µs in Chrome** and pass-level GPU timestamps
  mislead across machines — M10's wall-clock+counts choice is the right one; treat browser
  timing as debug-only.
- **Occlusion queries are core** (boolean) but tiler-hostile same-frame; previous-frame
  reuse only, and only if profiling demands (matches the voxel-doc occlusion verdict).
- **Browser floor 2026:** WebGPU in Chrome 113+, Safari/iOS 26+, Firefox 141+ (Windows
  first; Firefox *is* wgpu). WebGL2 fallback still earns its keep (pre-26 iOS, non-Windows
  Firefox).

## 5. Formats & upscaling (vindication + refinements)

- **WebGPU has no 16-bit packed color formats** (no 565/4444) — RGBA8 is the cheapest sane
  LDR target; our bandwidth win must come from low resolution (it does). `rg11b10ufloat`
  as a render target is feature-gated (Adreno 5xx/6xx can't) — only relevant if we ever
  want pre-palette HDR.
- **FSR1 is wrong for us by AMD's own spec:** EASU's input contract bans banding and
  structured noise — a Bayer-dithered buffer violates it twice; and naive FSR1 costs
  ~6.3 ms on a phone GPU (~2–2.7 ms heavily optimized). Temporal upscalers (FSR2/SGSR2/Arm
  ASR) require per-frame sub-pixel jitter + history — which either erases or ghosts a
  screen-locked dither, and cost ≥1 ms on *flagship* mobile. **The builder's FSR rejection
  is now fully vindicated on technical grounds, not just aesthetic.** Nearest blit is
  ~free and correct; record this as settled in `design.md` §8 territory when convenient.
- **Vertex formats:** Arm explicitly endorses 4–8-byte packed vertices (8/16-bit
  norm + 10-10-10-2) — our packed vertex is state-of-practice; the two-stream split (A5)
  is the remaining refinement.

## 6. Distilled action list (ranked, all measurable-here or M8b-cheap)

1. **A7 upload-path audit** (mappedAtCreation / belt / no mid-frame buffer creation) —
   latent multi-ms hitch class.
2. **A1 render-target usage-flag audit** — up to ~33–50% color bandwidth on Mali for one
   line of code.
3. **A5 two-stream vertex split** — vertex-fetch bandwidth on both mobile families.
4. **A6 fp16-first shaders (dual-path)** — 2× ALU + occupancy headroom on Mali/Xe.
5. **A8 web render bundles keyed on chunk-set** — the one big web-submission lever.
6. **A2/A4 pass discipline + discard hygiene** — keep what we have; document the melt tax.
7. **A3 adopt the ~100 MB/frame mobile bandwidth budget** into the charter table.

*(Source quality: Arm numbers from Arm-authored Khronos samples [fetched primary]; Intel
from the Xe-LP/Arc guides + patents [search-extract, vendor]; wgpu from maintainer
discussions/issues [fetched]; WebGPU from gpuweb spec issues [fetched]; upscaler costs from
vendor repos [fetched] + the atyuwen mobile-FSR measurements [search-extract].)*
