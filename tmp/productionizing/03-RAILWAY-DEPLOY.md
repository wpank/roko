# Railway Deployment Guide

## Prerequisites

- Railway CLI installed, e.g. `npm i -g @railway/cli && railway login` or `yarn global add @railway/cli && railway login`
- Docker installed (for building the image)
- API keys for your LLM providers

## Step 1: Create Railway Project

```bash
railway init
# or link existing:
railway link
```

## Step 2: Set Secrets

```bash
# Required
railway variables set ANTHROPIC_API_KEY=sk-ant-...
railway variables set OPENAI_API_KEY=sk-...

# Optional (if using these providers)
railway variables set PERPLEXITY_API_KEY=pplx-...
railway variables set GEMINI_API_KEY=AI...

# Optional integrations
railway variables set GITHUB_TOKEN=ghp_...

# Server config
railway variables set RUST_LOG=info,roko=debug
```

## Step 3: Align `railway.toml` with what you are deploying

**In-tree today:** `railway.toml` builds **`docker/mirage-demo.Dockerfile`** (Mirage demo stack) and uses `healthcheckPath = "/health"`. That is **not** the same as “`roko serve` + `demo-app` SPA” described in **04-DOCKERFILE-FIX.md**.

Choose one:

| Goal | Set `dockerfilePath` to | Health path |
|------|-------------------------|-------------|
| Mirage demo (current) | `docker/mirage-demo.Dockerfile` | `/health` (or whatever that binary exposes) |
| Roko + embedded SPA | Dockerfile matching **04** (often repo root after you add the multi-stage file) | `GET /health` for liveness, or **`/api/health`** if you want API-level checks (**09-OPERATIONS-RUNBOOK.md**) |

Example **target** shape for a **roko** image:

```toml
[build]
builder = "DOCKERFILE"
dockerfilePath = "Dockerfile"

[deploy]
healthcheckPath = "/health"
healthcheckTimeout = 300
restartPolicyType = "ON_FAILURE"
restartPolicyMaxRetries = 3
```

Use `/api/health` instead if your ops standard requires hitting the nested API router (see **09**).

## Step 4: Fix the Dockerfile

See [04-DOCKERFILE-FIX.md](04-DOCKERFILE-FIX.md). The root `Dockerfile` does not run a Node stage; without a pre-seeded `demo/demo-app/dist/`, the SPA embed is often empty in container builds.

## Step 5: Deploy

```bash
railway up
```

Or use the CLI subcommand (if fixed):
```bash
cargo run -p roko-cli -- deploy railway
```

## Step 6: Verify

```bash
BASE=https://your-app.up.railway.app
# Liveness (minimal) — always defined for `roko serve`
curl -sS "$BASE/health"
# API health + status (under /api nest)
curl -sS "$BASE/api/health"
curl -sS "$BASE/api/status"

# Demo app (SPA fallback on same origin when deployed correctly)
open "$BASE"
```

## Persistent Storage

For the full self-hosting loop, roko needs persistent storage for `.roko/` state (episodes, plans, signals, learning data).

Railway supports volumes:

```bash
railway volume create --name roko-state --mount /workspace/.roko
```

Or set `ROKO_WORKSPACE_ROOT` to point at the volume mount.

## Full Self-Hosting Requirements

For agents to actually execute plans (compile, test, clippy gates), the container needs:

1. **Rust toolchain** — `rustup`, `cargo`, `clippy`, `rustfmt`
2. **Git** — for repo operations
3. **The roko source code** — either clone at startup or bake into image
4. **Node.js** — if any tasks involve frontend work

The Dockerfile in [04-DOCKERFILE-FIX.md](DOCKERFILE-FIX.md) covers option 1-2. For option 3, you can either:

- Mount a volume with the repo
- Clone at container startup via entrypoint script
- Bake a snapshot into the image (stale but fast)

### Entrypoint Script (clone at startup)

```bash
#!/bin/bash
set -e

# Clone repo if not present
if [ ! -d /workspace/roko ]; then
    git clone https://github.com/nunchi-trade/roko.git /workspace/roko
fi

cd /workspace/roko
git pull --ff-only

# Start server
exec roko serve --bind 0.0.0.0 --port ${PORT:-6677}
```

## Port Mapping

Railway assigns a random port via `$PORT`. The start command must use it:

```bash
roko serve --bind 0.0.0.0 --port $PORT
```

Or via env override:
```bash
ROKO__SERVER__PORT=$PORT roko serve --bind 0.0.0.0
```

## Architecture on Railway

```
┌─────────────────────────────────────────┐
│  Railway Container                       │
│                                          │
│  roko serve --bind 0.0.0.0 --port $PORT │
│  ├── /api/*     → roko-serve routes     │
│  ├── /ws        → WebSocket stream      │
│  └── /*         → demo-app SPA          │
│                                          │
│  Volume: /workspace/.roko/               │
│  ├── state/     → executor snapshots    │
│  ├── episodes.jsonl                      │
│  ├── learn/     → cascade router, etc   │
│  └── prd/       → PRD documents         │
└─────────────────────────────────────────┘
         │
         ▼ HTTPS (:443)
    Railway proxy
         │
         ▼
    Public URL: https://your-app.up.railway.app
```

## Monitoring

- Dashboard: `https://your-app.up.railway.app` (demo-app SPA when that stack is deployed)
- Logs: `railway logs`
- Liveness: `curl https://your-app.up.railway.app/health`
- API health / status: `curl …/api/health` and `curl …/api/status`
