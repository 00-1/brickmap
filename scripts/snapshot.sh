#!/usr/bin/env bash
# Capture an immutable build snapshot for the deployed gallery.
#
# Usage: scripts/snapshot.sh <id> <git-ref>
#   e.g. scripts/snapshot.sh 00-spike-cube f5a93f8
#
# Builds the release WASM bundle at <git-ref> into web/archive/<id>/, which the
# deploy workflow publishes at <pages-url>/archive/<id>/. Commit the result, then
# add a matching entry to web/gallery/index.html. Snapshots are intentionally
# sparse (notable milestones) — each bundle is a few MB.
set -euo pipefail

id="${1:?usage: snapshot.sh <id> <git-ref>}"
ref="${2:?usage: snapshot.sh <id> <git-ref>}"

repo="$(git rev-parse --show-toplevel)"
out="$repo/web/archive/$id"
work="$(mktemp -d)"
trap 'git -C "$repo" worktree remove --force "$work" 2>/dev/null || true; rm -rf "$work"' EXIT

echo "Building snapshot '$id' from $ref ..."
git -C "$repo" worktree add --detach "$work" "$ref" >/dev/null

# Share the main target dir so dependency artifacts are reused.
CARGO_TARGET_DIR="$repo/target" cargo build --release --lib \
  --target wasm32-unknown-unknown --manifest-path "$work/Cargo.toml"

rm -rf "$out"
mkdir -p "$out"
cp "$work/web/index.html" "$out/index.html"
# Freeze the build-id/cache placeholders to this snapshot's id.
sed -i "s/__CACHE_BUST__/$id/g; s/__BUILD_SHA__/$id/g" "$out/index.html"
wasm-bindgen --target web --no-typescript \
  --out-dir "$out/pkg" \
  "$repo/target/wasm32-unknown-unknown/release/brickmap.wasm"

# Tag the source commit so the snapshot's provenance is recorded.
git -C "$repo" tag -f "snapshot/$id" "$ref" >/dev/null

echo "Wrote $out"
echo "Next: add a gallery entry in web/gallery/index.html, then commit + push"
echo "      (and 'git push origin snapshot/$id')."
