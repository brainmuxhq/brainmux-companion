# brainmux Companion

The local **Data Plane** (ADR-0011). The one tool a user downloads + runs on their PC. It:

1. **Zero-State provision** — checks + auto-installs deps (Ollama daemon, bge-m3, dirs). No manual steps.
2. **Runs the core** on `127.0.0.1:8787` (the engine: RAG, doc-gen — the user's data never leaves).
3. **Opens the console** at `127.0.0.1:3100/console/evrak`.

The control plane (auth/billing) stays in our cloud; work data + compute stay local (BYOC hybrid).

**Thin shell:** all runtime logic is **imported from `brainmux-core`** — nothing is copied (no drift).

## Run (dev / dogfood)
```
cd brainmux-companion
uv run brainmux
```
Ctrl+C tears down core + console cleanly (no orphans).

## Roadmap
v1 = launcher (this). Next fills: Nuitka one-file (true download) + pystray tray + hosted
console via pairing token (ADR-0011). "Walking skeleton → doldura doldura."
