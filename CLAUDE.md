# brainmux-companion — çalışma kuralları

> Global tarz `~/.claude/CLAUDE.md` · üst mimari/metodoloji: product `brainmux/CLAUDE.md` (Control/Data Plane, Vertical Slice, Zero-State, Vibe Coding). Bu repo = **yerel Data Plane** (ADR-0011).

- **İnce kabuk:** tüm runtime mantığı `brainmux-core`'dan **import edilir, KOPYALANMAZ** (SSoT, drift yok). Companion sadece orkestre eder: provizyon + core + konsol + (sonra) tray/pairing.
- **Zero-State:** ilk çalıştırmada bağımlılıklar otomatik (core `bootstrap` motoru). Kullanıcı manuel komut çalıştırmaz.
- **Temiz kapanış:** çocuğu process-group'ta başlat, Ctrl+C'de `killpg` → orphan YOK (Ali'nin sabit kuralı).
- **İsimlendirme/stealth:** vendored motor adı geçmez (generic: çekirdek/runtime).
- **Paketleme:** Faz-1 = Nuitka + pystray (native, no Docker); Tauri-sidecar sonra. (product CLAUDE.md/ADR-0011.)
- **Dev:** `uv run brainmux`. Commit/push sadece Ali isteyince.
