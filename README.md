<div align="center">

<img src="packaging/linux/brainmux.png" width="88" height="88" alt="brainmux" />

# brainmux — desktop app

**Your data and compute stay on your machine.**

[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Release](https://img.shields.io/github/v/release/brainmuxhq/brainmux-companion?display_name=tag)](https://github.com/brainmuxhq/brainmux-companion/releases/latest)
[![Platform](https://img.shields.io/badge/platform-Linux%20x86__64-informational.svg)](#install)
[![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri%20v2-24C8DB.svg)](#build-the-installer)

</div>

---

brainmux runs an AI agent fleet on **your own machine**. The desktop app (Tauri) shows the console in
a **native window** and runs the engine **locally** on `127.0.0.1` — nothing about your work data
leaves the device. The cloud is thin: sign-in, billing, and the download only.

```
        Cloud — thin control plane               Your machine — the desktop app
   ┌────────────────────────────┐          ┌──────────────────────────────────┐
   │  brainmux.com  — download   │          │  brainmux (native window)         │
   │  app.brainmux.com — account │          │   ├─ console (webview)            │
   └────────────────────────────┘          │   ├─ engine   127.0.0.1:8787       │
                                            │   ├─ Ollama + model (local)       │
                                            │   └─ your files  (never leave)    │
                                            └──────────────────────────────────┘
```

## Why local-first

- **Your data never leaves the machine.** Documents, embeddings, and generated output live in
  `~/.brainmux`, served only on `127.0.0.1`.
- **Open source.** Apache-2.0 — audit every line before you run it.
- **No telemetry.** The engine runs with telemetry disabled; nothing phones home.

## Install

### One-click (recommended)

1. **Download** the latest installer from
   [Releases](https://github.com/brainmuxhq/brainmux-companion/releases/latest) — Linux `.AppImage`
   (macOS / Windows soon).
2. **Double-click** it — the app opens in its **own window**. On first run it auto-provisions the AI
   engine + model (progress shown as a notification). No terminal.

The AppImage is **FUSE-independent** (runs even without `libfuse2`). A second launch focuses the
existing window instead of starting a duplicate. **Ollama** is a prerequisite (the app checks for it
and provisions the model).

### From source (dev)

```sh
cd src-tauri && cargo build          # compile (uses placeholder resources)
npx @tauri-apps/cli dev              # run the window (dev falls back to the local product repo)
```

## How it works

The native window (webview) shows the **same Next.js console**, which talks to the local engine on
`127.0.0.1`:

1. **Provision** `~/.brainmux/{logs,models,knowledge,output,runtime}`.
2. **Zero-State** — ensure the Ollama daemon is up and `bge-m3` is present (pulled on first run, with a
   progress readout).
3. **Run the embedded engine** (portable-python sidecar bundled in the app) on `127.0.0.1:8787`.
4. **Show the console** in the window. On exit, child process groups are killed (no orphans); the web
   console can also stop it via `POST /shutdown`.

## Architecture

Thin shell: all runtime logic is **imported from `brainmux-core`** (SSoT, no drift). The app only
orchestrates: provision → run the embedded engine → show the console → (later) tray, pairing, updates.
Design record: **ADR-0011 §7-8** (desktop app + thin cloud) in the product repository.

## Build the installer

```sh
packaging/build-desktop.sh    # stage core bundle + modules → cargo tauri build → .AppImage/.deb
```

Stages the relocatable portable-python core (`apps/core/packaging/build-bundle.sh`) + the module
manifests into `src-tauri/bundle/` (embedded as Tauri resources), then bundles a single installer with
the engine inside — a fresh-machine one-install (Ollama = prerequisite).

## Status & roadmap

**v0.2 — Tauri desktop app (WIP).** The old browser-launcher was removed; the Tauri app supersedes it.

- [x] Tauri v2 shell — native window shows the console
- [x] Relocatable portable-python core bundle (proven: runs moved / repo-free)
- [x] Core embedded as a Tauri resource + graceful dev fallback
- [x] Single-instance · desktop notifications · clean teardown · web quit (`POST /shutdown`)
- [ ] Console static-export bundled (fully offline console) — Faz-2b
- [ ] Cross-platform (`.dmg` / `.msi`) + code-sign / notarize + auto-updater
- [ ] Membership / account auth (system-browser OAuth → deep-link)

## License

[Apache-2.0](LICENSE). The vendored engine's attribution is preserved in [`NOTICE`](NOTICE).
