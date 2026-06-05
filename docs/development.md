# brickmap — Development & code quality

How we work on this repo. Kept deliberately lightweight — this is an informal,
single-driver project — but with the quality gates that actually pay off early.

## Source control

- **Trunk-based on `main`.** Changes land on `main` in small, logically-scoped,
  well-described commits. `main` is always green (CI passes) and always
  deployable (the live preview tracks it — see below).
- **PRs are the exception, not the rule.** We open one only when a change is large
  or risky enough to be worth previewing/reviewing in isolation before it goes
  live. Routine work goes straight to `main`.
- **Commit messages:** imperative subject with a area prefix (`render:`, `mesh:`,
  `world:`, `docs:`, `ci:`, `spike:`), then a body explaining *why* when it isn't
  obvious. Group related changes; avoid drive-by mixing.

## Toolchain

Pinned in [`rust-toolchain.toml`](../rust-toolchain.toml) (stable + `rustfmt`,
`clippy`, and the `wasm32-unknown-unknown` target). `rustup` picks it up
automatically, so local and CI use the same tools.

## The quality gates (enforced in CI)

Run these before pushing; CI runs the same on every push and PR.

```sh
cargo fmt --all --check        # formatting (rustfmt defaults, no bikeshedding)
cargo clippy --all-targets -- -D warnings   # lint; warnings are errors
cargo test --all               # tests
cargo build --lib --target wasm32-unknown-unknown   # the web target still builds
```

A convenient local pre-flight: `cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test --all`.

- **Formatting:** `rustfmt` with default config. Not negotiable, not configurable —
  it removes a whole class of diffs and arguments.
- **Linting:** `clippy` with `-D warnings`. Warnings are failures. If a lint is
  genuinely wrong for a case, suppress it *locally* with `#[allow(...)]` **and a
  comment** explaining why — never blanket-allow.
- **Unsafe:** allowed where it earns its place (vertex bit-packing, GPU upload),
  but it must be localized, commented with the invariant it relies on, and
  ideally wrapped behind a safe API. Prefer `bytemuck` over hand-rolled transmutes.

## Testing strategy

We test **where the logic and the risk are**, and we don't write theatre.

**What we unit-test** (colocated `#[cfg(test)]` modules, pure and deterministic):
- meshing correctness (greedy-quad merging, face culling between voxels),
- palette storage round-trips (pack → unpack is identity; bit-widths),
- vertex bit-packing round-trips (every field survives encode/decode),
- visibility-graph / culling logic,
- coordinate & floating-origin math.

**What we _don't_ unit-test:** wgpu plumbing, windowing, and the event loop. These
are validated by the spike actually running, and later by **golden-image / headless
GPU tests** (render a known scene with headless wgpu, compare against a committed
reference image) once there's a scene worth pinning. Mocking the GPU to assert we
called it would test our test, not the engine.

**Bigger tools, added when the matching code lands:**
- **Integration tests** in `tests/` for cross-module behavior (e.g. world → mesh).
- **`criterion` benchmarks** for the hot paths (meshing throughput, packing) — tied
  directly to the frame/throughput budgets in [`design.md`](design.md) §8 so a
  perf regression is visible, not a surprise on the reference phone.
- **`wasm-bindgen-test`** for any browser-specific logic, run headless in CI.

The seed test in `src/gfx.rs` (cube-geometry invariants) exists mainly to
establish the harness; real coverage begins with the first pure-logic module
(the mesher — Spike 2).

## Continuous integration & preview

- **`.github/workflows/ci.yml`** — runs the quality gates above on every push/PR.
- **`.github/workflows/deploy.yml`** — on push to `main`, builds the release WASM
  bundle, runs `wasm-bindgen`, and deploys the `web/` app to **GitHub Pages**, so
  the latest `main` is always previewable at the project Pages URL.

> One-time setup: in the repo, **Settings → Pages → Source = "GitHub Actions"** must
> be enabled for the deploy to publish.

### Cache-busting (always-fresh preview)

The unhashed wasm bundle is busted with `?v=<sha>` (the deploy seds `__CACHE_BUST__`
→ commit SHA into `/latest/index.html`). But the **HTML page itself** is served by
Pages with a short cache lifetime, so a plain refresh can still hand you a stale
page that points at the old build. To self-heal that:

- The deploy writes a `version.json` sentinel (`{"sha": "<full sha>"}`) next to the
  page, and stamps the same SHA into the page (`__CACHE_BUST__`).
- On load, `index.html` fetches `version.json` **uncached**; if its SHA differs from
  the one baked into the page, the page is stale, so it reloads through a fresh
  `?v=<sha>` URL (a new query key forces the browser past its cached HTML). A
  `sessionStorage` guard prevents reload loops.

The visible **build SHA** stamped on the page (`__BUILD_SHA__`) is the tell for which
build you're on. Note the self-heal only kicks in once you're on a build that *has*
the check — upgrading *from* an older cached page still needs one manual hard refresh.

## Preview gallery & snapshots

The deployed site is a small **build gallery** so we can keep a visible history of
how the engine looks as it evolves (this is also the "look journal" from
[`design.md`](design.md) §11):

- **root** — the gallery index ([`web/gallery/index.html`](../web/gallery/index.html)).
- **`/latest/`** — the current `main` build, rebuilt on every push.
- **`/archive/<id>/`** — frozen, immutable snapshots of notable milestones.

Snapshots are **committed static bundles** (a few MB each), so the deploy stays a
fast copy rather than rebuilding old code — keep them to genuine milestones. To
capture one:

```sh
scripts/snapshot.sh <id> <git-ref>     # e.g. scripts/snapshot.sh 01-first-chunk HEAD
```

That writes `web/archive/<id>/` and tags the source commit `snapshot/<id>`. Then
add a card to `web/gallery/index.html` and commit both (push the tag too).
