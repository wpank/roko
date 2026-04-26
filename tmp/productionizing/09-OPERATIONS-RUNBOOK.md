# Operations Runbook — Roko + demo-app end-to-end

Companion to **00-INDEX.md**. Use this after deploy to verify behavior, debug failures, and align load balancers with the right HTTP paths.

---

## 1. What you actually deployed

Two different “Railway” stories exist in this repo; **confirm which image you are running**.

| Stack | Typical image / Dockerfile | Primary process | Demo UI |
|--------|----------------------------|-----------------|---------|
| **Mirage demo** (current `railway.toml`) | `docker/mirage-demo.Dockerfile` | Mirage + agent-relay + static Mirage dashboard | Mirage dashboard assets, not necessarily `demo-app` |
| **Roko control plane + React demo** (target of this folder) | Root Dockerfile per **04-DOCKERFILE-FIX.md** or GHCR `roko` image | `roko serve` | Embedded / disk `demo/demo-app/dist` |

If health checks or SPA routes “don’t match the docs,” you are usually on the **wrong** stack for that doc section.

---

## 2. HTTP surface (`roko serve`)

Mounted in `crates/roko-serve/src/routes/mod.rs`:

| Method | Path | Auth | Purpose |
|--------|------|------|---------|
| `GET` | `/health` | Public | **Liveness**: minimal `{"status":"ok"}` — use for “is the process up?” |
| `GET` | `/api/health` | Matches API router rules | **Service health** handler under `/api` |
| `GET` | `/api/status` | Matches API router rules | Session / dashboard status payload |
| `GET` | `/api/...` | Per-route | All other API endpoints live under `/api` |
| `GET` | `/ws`, `/roko-ws` | Per config | Event bus WebSocket (aliases) |
| `GET` | `/api/ws` | Per config | Aggregator stream |
| `GET` | `/api/workflow/ws` | Per config | Workflow WebSocket |
| `GET` | `/ws/terminal/{id}` | Gated | PTY bridge when terminal feature enabled |
| *fallback* | `/*` not matched above | — | **Embedded SPA** (`demo-app`) |

**Railway / Fly health checks**

- Today’s `railway.toml` uses `healthcheckPath = "/health"` — valid for **minimal liveness** if the running app exposes that route (Mirage or roko).
- For “can this instance serve the API + SPA meaningfully?”, you may want **`/api/health`** and to treat non-2xx or degraded JSON as unhealthy (see audit **M7** in **06-AUDIT-FINDINGS.md**).

**Fly.io (`fly.toml`)**

- Often uses `internal_port` that differs from local `6677`; set **`PORT`** in the container to match what Fly expects, or align `[http_service] internal_port` with your `roko serve --port`.

---

## 3. Environment and config quick reference

| Concern | Mechanism |
|---------|-----------|
| Override any `roko.toml` field | `ROKO__SECTION__SUBSECTION__KEY` (double underscore = dotted path) |
| Interpolation in TOML strings | `${VAR}` / `${VAR:-default}` |
| Dotenv | `~/.roko/.env` then `.roko/.env` (see **01-ENV-VARS.md**) |
| Provider keys | Each `[providers.*]` entry’s `api_key_env` names the variable (e.g. `GEMINI_API_KEY`) |
| SPA from disk instead of embed | `ROKO_SPA_DIR=/path/to/demo-app/dist` |
| Secrets for client → server | `ROKO_API_KEY`, `ROKO_SERVER_AUTH_TOKEN` (see **01**) |

**Privy / JWT**

- Auth integrates with JWKS / app id from **config**, not `PRIVY_*` env vars in Rust (see **01-ENV-VARS.md** expanded section).

---

## 4. Stateful directories (production)

| Path | Contents | Notes |
|------|----------|--------|
| `.roko/` under workspace | Episodes, learning, PRD, snapshots | Needs a **volume** on Railway for durability |
| `ROKO_WORKSPACE_ROOT`, worker template vars | Cloud worker / MCP | See **01** for worker-related env |

**Concurrency**

- Multiple processes writing the same `.roko/` can corrupt JSONL / router state (**C2**). Run **one writer** per volume until **P5** (file locking) ships.

---

## 5. End-to-end verification checklist

Run in order after deploy.

### 5.1 Liveness

```bash
BASE="https://YOUR_HOST"
curl -sS "$BASE/health"
curl -sS "$BASE/api/health"
curl -sS "$BASE/api/status"
```

Expect JSON bodies; compare `/health` vs `/api/health` semantics before wiring alerts.

### 5.2 SPA

Open `/` in a browser. If the shell is empty or 404:

- Confirm **`demo/demo-app/dist`** existed at **image build** time, or set **`ROKO_SPA_DIR`**.
- For root Dockerfile-only builds without Node, **`roko-serve/build.rs`** tries **npm** during `cargo build`; inside a slim Rust image **`npm` is often missing**, so the embed can be empty — use **04-DOCKERFILE-FIX.md**.

### 5.3 Provider routing (pre-P1/P4)

Until provider gating is fully enforced:

```bash
roko config check-secrets
# If implemented in your branch:
roko config providers health
```

Strip **05-ROKO-TOML-PRODUCTION.md** so cascade cannot pick providers without keys.

### 5.4 Auth surface (**C3**)

If `serve.auth.enabled` is false and bind is `0.0.0.0`, **all API routes including dangerous ones are world-readable**. Before sharing a public URL:

- Enable auth and set API keys per **10-IMPLEMENTATION-PLAN.md** task **P6** guidance.

### 5.5 Logs

```bash
railway logs
# or
fly logs
```

Search for provider errors, “API key”, lock failures, and panic traces (pre-**P10**).

---

## 6. Troubleshooting matrix

| Symptom | Likely cause | Next step |
|---------|----------------|-----------|
| SPA blank / static 404 | No `dist` at build; wrong stack image | Rebuild with **04**; set `ROKO_SPA_DIR` |
| `502` / health failures | Wrong port; process not binding `0.0.0.0` | Use `$PORT`; match Fly `internal_port` |
| `/api/health` OK but LLM calls fail | Missing key; cascade chose bad model | **02**, **05**, provider health |
| Intermittent corrupt state | Two writers; no flock | **P5**; single replica or external lock |
| Silent failures in logs | Swallowed errors | **P8**, **07** §3 |

---

## 7. Build tool note (frontend)

- `demo/demo-app` ships **`package-lock.json`**; **`crates/roko-serve/build.rs`** invokes **npm** for `install` / `run build`.
- Docker examples in **04** / **08** use **`npm ci`** for reproducible CI. If you standardize on **Yarn**, commit **`yarn.lock`** and align `build.rs` / Docker stages — until then, mixed lockfiles will confuse CI.

---

## 8. Cross-links

| Topic | Doc |
|-------|-----|
| Env inventory | **01-ENV-VARS.md** |
| Routing failures | **02-MODEL-ROUTING-FIX.md** |
| Railway steps + volume | **03-RAILWAY-DEPLOY.md** |
| Image with SPA | **04-DOCKERFILE-FIX.md** |
| Minimal config | **05-ROKO-TOML-PRODUCTION.md** |
| Risk register | **06-AUDIT-FINDINGS.md** |
| Conventions | **07-ANTI-PATTERNS.md** |
| Fast deploy tiers | **08-FAST-BUILD-DEPLOY.md** |
| Implementation tasks | **10–12** |
