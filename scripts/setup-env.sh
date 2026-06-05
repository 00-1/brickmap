#!/usr/bin/env bash
# Ensure this environment can run the headless renderer (milestone D1): wgpu needs
# a Vulkan ICD, and the container has no GPU, so we use Mesa's software rasterizer
# (llvmpipe) from `mesa-vulkan-drivers`. Idempotent; safe to run on every session.
set -euo pipefail

if [ -f /usr/share/vulkan/icd.d/lvp_icd.json ]; then
  exit 0
fi

echo "setup-env: installing mesa-vulkan-drivers (software Vulkan / llvmpipe)..."
apt-get install -y --no-install-recommends mesa-vulkan-drivers >/dev/null 2>&1 \
  || echo "setup-env: could not install mesa-vulkan-drivers (headless render will be unavailable)"
