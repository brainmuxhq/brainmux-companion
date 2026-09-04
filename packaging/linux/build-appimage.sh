#!/usr/bin/env bash
# brainmux Companion → tek-dosya Linux AppImage (çift-tık çalışır, kurulum yok).
# Kullanım: packaging/linux/build-appimage.sh   (repo kökünden veya herhangi bir yerden)
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"          # repo kökü (brainmux-companion)
DIST="$ROOT/dist"
APPDIR="$DIST/brainmux.AppDir"
ARCH="${ARCH:-x86_64}"
OUT="$DIST/brainmux-${ARCH}.AppImage"
TOOL="$DIST/appimagetool-${ARCH}.AppImage"
# FUSE-bağımsız statik runtime: hedef makinede libfuse2 yoksa AppImage otomatik extract-and-run
# yapar → çift-tık her yerde çalışır (Arch/CachyOS gibi fuse2 gelmeyen dağıtımlarda bile).
RUNTIME="$DIST/runtime-${ARCH}"

echo "==> 1/4 Rust release derleniyor…"
( cd "$ROOT" && cargo build --release )

echo "==> 2/4 AppDir hazırlanıyor…"
rm -rf "$APPDIR" "$OUT"
mkdir -p "$APPDIR/usr/bin" "$APPDIR/usr/share/applications" \
         "$APPDIR/usr/share/icons/hicolor/256x256/apps"
install -m755 "$ROOT/target/release/brainmux"       "$APPDIR/usr/bin/brainmux"
install -m755 "$HERE/AppRun"                         "$APPDIR/AppRun"
install -m644 "$HERE/brainmux.desktop"               "$APPDIR/brainmux.desktop"
install -m644 "$HERE/brainmux.desktop"               "$APPDIR/usr/share/applications/brainmux.desktop"
install -m644 "$HERE/brainmux.png"                   "$APPDIR/brainmux.png"
install -m644 "$HERE/brainmux.png"                   "$APPDIR/usr/share/icons/hicolor/256x256/apps/brainmux.png"

echo "==> 3/4 appimagetool + statik runtime alınıyor…"
if [ ! -x "$TOOL" ]; then
  wget -q -O "$TOOL" \
    "https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-${ARCH}.AppImage"
  chmod +x "$TOOL"
fi
if [ ! -x "$RUNTIME" ]; then
  wget -q -O "$RUNTIME" \
    "https://github.com/AppImage/type2-runtime/releases/download/continuous/runtime-${ARCH}"
  chmod +x "$RUNTIME"
fi

echo "==> 4/4 AppImage paketleniyor (FUSE-bağımsız runtime ile)…"
# appimagetool kendisi FUSE yoksa --appimage-extract-and-run ile çalışır (CI/temiz makine güvenli).
# --runtime-file → paketlenen AppImage FUSE-bağımsız statik runtime taşır.
ARCH="$ARCH" "$TOOL" --appimage-extract-and-run --runtime-file "$RUNTIME" "$APPDIR" "$OUT" 2>&1 | tail -3

chmod +x "$OUT"
echo "==> HAZIR: $OUT"
ls -lh "$OUT" | awk '{print "    boyut:", $5}'
