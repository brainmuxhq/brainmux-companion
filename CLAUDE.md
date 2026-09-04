# brainmux-companion — çalışma kuralları

> Global tarz `~/.claude/CLAUDE.md` · üst mimari/metodoloji: product `brainmux/CLAUDE.md` (Control/Data Plane, Vertical Slice, Zero-State, Vibe Coding). Bu repo = **ürünün kendisi: masaüstü uygulaması** (Tauri v2; yerel Data Plane, ADR-0011 §7).

- **Ne bu:** **Tauri v2 desktop app** (`src-tauri/`). Native pencere (webview) → **aynı Next.js konsolu** gösterir; motor+modüller **gömülü** (portable-python sidecar + manifest resource'ları) → tek-kurulum, yerelde koşar. Eski tiny-launcher (browser açan) SÖKÜLDÜ; Tauri kapsıyor.
- **İnce kabuk:** runtime mantığı `brainmux-core`'dan **gelir, KOPYALANMAZ** (SSoT). Kabuk orkestre eder: provizyon + gömülü core spawn + konsol + (sonra) tray/pairing. Mantık `src-tauri/src/launcher.rs`.
- **Zero-State:** ilk açılışta eksikler otomatik (core `bootstrap`). Ollama = prereq (app kontrol/kurar).
- **Temiz kapanış:** çocuğu process-group'ta başlat, çıkışta `killpg` (RunEvent::Exit) → orphan YOK.
- **İsimlendirme/stealth:** vendored motor adı geçmez (generic: çekirdek/runtime).
- **Paketleme:** `packaging/build-desktop.sh` — core bundle'ı (`apps/core/packaging/build-bundle.sh`) + modülleri stage'ler, `cargo tauri build` ile `.AppImage`/`.deb` üretir (FUSE-bağımsız). Dev: `cargo build` (placeholder resource) / `npx tauri dev`. Marka ikonu: `packaging/linux/brainmux.png`.
- Commit/push sadece Ali isteyince.
