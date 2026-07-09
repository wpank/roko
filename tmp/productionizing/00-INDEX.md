# Productionizing Roko

Guide to deploying roko + demo-app as a full self-hosting loop on Railway.

## Files

| # | File | What |
|---|------|------|
| 01 | [ENV-VARS.md](01-ENV-VARS.md) | Complete env var inventory (~70 vars) — what to set, what's optional |
| 02 | [MODEL-ROUTING-FIX.md](02-MODEL-ROUTING-FIX.md) | The model/provider routing problem and how to fix it |
| 03 | [RAILWAY-DEPLOY.md](03-RAILWAY-DEPLOY.md) | Step-by-step Railway deployment |
| 04 | [DOCKERFILE-FIX.md](04-DOCKERFILE-FIX.md) | Updated multi-stage Dockerfile (builds demo-app + roko) |
| 05 | [ROKO-TOML-PRODUCTION.md](05-ROKO-TOML-PRODUCTION.md) | Production roko.toml — stripped to real providers only |
| 06 | [AUDIT-FINDINGS.md](06-AUDIT-FINDINGS.md) | Full production audit: 4 critical, 6 high, 8 medium findings |
| 07 | [ANTI-PATTERNS.md](07-ANTI-PATTERNS.md) | 13 things NOT to do — read before making changes |
| 08 | [FAST-BUILD-DEPLOY.md](08-FAST-BUILD-DEPLOY.md) | 3 speed tiers: 45s (zigbuild), 2min (cargo-chef), 0s (tunnel) |
| 09 | [OPERATIONS-RUNBOOK.md](09-OPERATIONS-RUNBOOK.md) | **End-to-end ops:** health routes, deploy paths, volumes, troubleshooting |
| **10** | **[IMPLEMENTATION-PLAN.md](10-IMPLEMENTATION-PLAN.md)** | **Master plan: 18 tasks with full context, code, verification** |
| **11** | **[FRONTIER-CAPABILITIES-PLAN.md](11-FRONTIER-CAPABILITIES-PLAN.md)** | **7 tasks: wire ADAS, research pipeline, math, daimon, novelty, spawning, collusion** |
| **12** | **[PRODUCTION-DEPLOYMENT-PLAN.md](12-PRODUCTION-DEPLOYMENT-PLAN.md)** | **9 tasks: budget enforcement, semantic caching, cost metrics, compliance, OTEL** |

## Reading Order

1. **07-ANTI-PATTERNS.md** — Read first. Know what not to do.
2. **06-AUDIT-FINDINGS.md** — Understand what's broken and why.
3. **09-OPERATIONS-RUNBOOK.md** — How the running system behaves (URLs, health checks, Railway vs `roko serve`).
4. **10-IMPLEMENTATION-PLAN.md** — The concrete tasks to fix production blockers (P1-P18).
5. **11-FRONTIER-CAPABILITIES-PLAN.md** — Wire unwired frontier code: ADAS, research pipeline, math primitives, novelty search (F1-F7).
6. **12-PRODUCTION-DEPLOYMENT-PLAN.md** — Production economics: budget enforcement, semantic caching, cost metrics, compliance export (D1-D9).
7. Reference 01-05, 08 as needed during implementation.

## Repo status (verify in-tree)

These docs target **self-hosted `roko serve` + `demo-app`**. In the tree today:

| Item | In repo now | Docs / plan |
|------|----------------|-------------|
| `railway.toml` | Builds **`docker/mirage-demo.Dockerfile`** (Mirage demo stack), `healthcheckPath = "/health"` | **03** + **P14**: point Railway at a **roko** image and set health to the path your binary exposes |
| Root `Dockerfile` | Rust-only; no Node stage; SPA embed relies on `roko-serve/build.rs` running **npm** inside the image (often absent) | **04** multi-stage Node → Rust + `COPY dist` |
| `Dockerfile.runtime` / `Dockerfile.optimized` | **Not committed** | **08** — add when adopting fast deploy tiers |
| Health | **`GET /health`** (minimal) and **`GET /api/health`** (API) | Load balancers: pick the one that matches your readiness definition (**09**) |
| `roko.toml` | **`schema_version = 2`**: `[providers.*]`, `[models.*]` tables | **05** must match this shape (not legacy `[[providers]]`) |

## TL;DR

### Critical blockers (must fix before deploy)

| # | What | Impact |
|---|------|--------|
| C1 | Model routing falls back to unavailable providers | Dispatch fails mid-task |
| C2 | No file locking for concurrent state writes | State corruption |
| C3 | Auth disabled by default, no warning on public bind | Everything exposed |

### Quick deploy (after fixing blockers)

```bash
# 1. Set your secrets on Railway
railway variables set ANTHROPIC_API_KEY=sk-ant-...
railway variables set OPENAI_API_KEY=sk-...
railway variables set PERPLEXITY_API_KEY=pplx-...
railway variables set GEMINI_API_KEY=AI...

# 2. Fix the Dockerfile (see 04-DOCKERFILE-FIX.md)
# 3. Fix railway.toml (see 10-IMPLEMENTATION-PLAN.md, task P14)

# 4. Deploy
railway up

# 5. Verify
curl https://your-app.up.railway.app/api/health
```

## Available providers (your keys)

| Provider | Env Var | Models |
|----------|---------|--------|
| Anthropic | `ANTHROPIC_API_KEY` | claude-opus-4-6, claude-sonnet-4-6, claude-haiku-4-5 |
| OpenAI | `OPENAI_API_KEY` | gpt-5.4, gpt-5-mini, o3, o4-mini |
| Perplexity | `PERPLEXITY_API_KEY` | sonar-deep-research, sonar-reasoning-pro |
| Gemini | `GEMINI_API_KEY` | gemini-2.5-pro, gemini-2.5-flash |
| Ollama | (local, no key) | depends on installed models |

## Task execution order

See `10-IMPLEMENTATION-PLAN.md` for full details. Summary:

```
CRITICAL (do first):
  P1 → P2 → P3 → P4  (model routing validation)
  P5                   (file locking)
  P6                   (auth + path traversal)

HIGH (do next):
  P7  (HTTP timeouts + limits)
  P8  (error logging)
  P9  (context overflow)
  P10 (expect/unwrap cleanup)

MEDIUM (before or after deploy):
  P11 (log rotation)
  P12 (eprintln → tracing)
  P13 → P14 (Dockerfile + deploy configs)
  P15 (production roko.toml)
```

Parallel-safe groups: {P1-P4}, {P5, P6, P7, P8}, {P9, P10, P11, P12}, {P13-P14, P15}.

## Frontier capabilities (after P1-P15)

```
INDEPENDENT (all parallel-safe):
  F1 (ADAS wiring), F2 (research pipeline), F3 (tropical/sheaf math)
  F4 (daimon phase 2), F6 (dynamic spawning), F7 (collusion detection)

DEPENDS ON F1:
  F5 (novelty search)
```

## Production economics (after P1-P15)

```
CRITICAL:
  D1 (budget enforcement)

HIGH:
  D2 (semantic caching), D6 (predictive cost estimation → depends on D1)

MEDIUM:
  D3 (cost per feature), D4 (bench regression), D5 (cache stats)
  D9 (competitive benchmarks → depends on D4)

LOW:
  D7 (compliance export), D8 (OpenTelemetry)
```
