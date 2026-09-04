#!/usr/bin/env bash
# brainmux desktop app (Tauri, ADR-0011 §7) — stage the self-contained core + modules, then bundle.
# Output = installers (.AppImage/.deb) with the ENGINE EMBEDDED → fresh-machine one-install (Ollama = prereq).
# Cross-repo: needs the product repo for the core bundle + module manifests.
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/.." && pwd)"                                        # brainmux-companion
REPO="${BRAINMUX_REPO:-$HOME/Development/Projects/brainmux/brainmux}" # product repo (apps/core, modules)
STAGE="$ROOT/src-tauri/bundle"                                        # gitignored; embedded as Tauri resources

echo "==> 1/3 core bundle (portable python + core; relocatable)"
( cd "$REPO" && bash apps/core/packaging/build-bundle.sh "$STAGE/core-bundle" )

echo "==> 2/3 modules (manifest + templates)"
rm -rf "$STAGE/modules"; mkdir -p "$STAGE/modules"
cp -a "$REPO/modules/." "$STAGE/modules/"

echo "==> 3/3 Tauri bundle (FUSE-independent)"
( cd "$ROOT/src-tauri" && APPIMAGE_EXTRACT_AND_RUN=1 npx --yes @tauri-apps/cli@latest build )

echo "==> HAZIR → $ROOT/src-tauri/target/release/bundle/"
ls -lh "$ROOT/src-tauri/target/release/bundle/appimage/"*.AppImage 2>/dev/null | awk '{print "    ", $9, $5}'
