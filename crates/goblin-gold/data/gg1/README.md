# GG1 content data (one-way synced from `00-1/halves:content/gg1/`)

A snapshot of Goblin Gold v1's T229 content-as-data export — the cross-repo **data seam**
(research §"share DATA not code"). The brickmap port CONSUMES this; it never embeds GG's JS.

- `modes.json` — per-mode metadata (id/name/tag/group/expr/unlock) + the JS `transform`
  string (NOT executed here — see the JS-rejected decision) + the raw `pool`.
- `parity-vectors.json` — the deterministic `{p,a}` set per mode (the behavioural contract).
  A re-implementation is correct iff it reproduces these exactly.

**Sync:** regenerate in halves (`node tools/content-export.js`) then re-copy here. Halves'
`test/content-parity.test.js` keeps the halves copy locked to the live runtime.
