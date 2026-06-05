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
