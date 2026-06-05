# Working notes for Claude

Project conventions and collaboration norms for this repo. Read the `docs/` for
the actual design; this file is about *how we work*.

## Collaboration style
- The human drives informally via chat; Claude makes the code changes.
- **Ask questions in plain prose. Do not use multiple-choice question dialogs.**
- Default to acting and reporting, rather than asking permission for routine work.
  Only stop to ask when a decision is genuinely the human's to make.

## Source control
- **Trunk-based on `main`.** Commit straight to `main` in small, well-described
  steps. Keep `main` green and deployable.
- Open a PR only for a large/risky change worth previewing in isolation — and say
  why. Don't add PR ceremony to routine work.
- Commit subjects use an area prefix: `render:`, `mesh:`, `world:`, `docs:`,
  `ci:`, `spike:`.

## Quality gates (run before pushing; CI enforces the same)
```sh
cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test --all
```
- rustfmt defaults; clippy warnings are errors.
- Test pure logic (meshing, palette, packing, culling, math); don't write tests
  that just assert we called wgpu. GPU/visual correctness is covered by the spike
  running and later golden-image tests.

## Project shape (where things are headed)
- A voxel **rendering** engine in Rust on `wgpu`; weak-hardware-first (bandwidth
  bound). Rasterized greedy meshing, forward rendering. Full rationale in
  `docs/design.md`; module/crate plan in `docs/architecture.md`.
- Visual identity is **emergent and low-fi (exposing the tech)** — not retro
  pastiche, not photoreal, not the stock-engine look. See `docs/design.md` §11.
- Web is a low-priority, cheap-to-keep target; never let it compromise native.

## Preview
- Push to `main` auto-deploys the WASM app to GitHub Pages (the live preview).
  See `docs/development.md`.
