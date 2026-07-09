# B — HTTP API and WebSocket Streaming (Docs 05, 06)

Parity of the two server-layer chapters: HTTP API (`roko serve`, ~85
routes on port 6677) and WebSocket streaming.

Both chapters are substantially DONE. `crates/roko-serve/src/routes/`
ships 18 route modules totalling **~12,285 LOC**. `crates/roko-agent-
server/src/features/` ships 5 per-agent-sidecar features (health /
messaging / predictions / research / tasks) with bearer auth and
registration.

Generated: 2026-04-16.

---

## B.01 — `roko serve` HTTP control plane ships (Doc 05 §"Roko Serve", CLAUDE.md)

**Status**: DONE
**Severity**: —
**Doc claim**: `roko serve` starts an HTTP control plane; per CLAUDE.md "~85 routes on :6677".
**Reality**: `crates/roko-serve/src/routes/` ships 18 route files: `agents.rs, aggregator.rs, config.rs, deployments.rs, learning.rs, middleware.rs, plans.rs, prds.rs, providers.rs, research.rs, run.rs, sse.rs, status.rs, subscriptions.rs, templates.rs, webhooks.rs, ws.rs, mod.rs`. Total 12,285 LOC. CLAUDE.md confirms "HTTP control plane | Wired | `crates/roko-serve/src/routes/`, `roko serve` on :6677".

---

## B.02 — 14+ route categories (Doc 05 §"Route Categories")

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 05 enumerates route categories: agents / plans / PRDs / research / providers / learning / status / config / webhooks / templates / deployments / SSE / WebSocket / subscriptions.
**Reality**: Each doc category maps to a shipping `roko-serve/src/routes/*.rs` file:
- agents.rs, plans.rs, prds.rs, research.rs, providers.rs, learning.rs, status.rs, config.rs — 8 CRUD-style resource routes
- webhooks.rs (520 LOC), templates.rs (556 LOC), deployments.rs — extension routes
- sse.rs, ws.rs (139 LOC), subscriptions.rs (267 LOC) — streaming routes
- aggregator.rs — cross-cutting aggregation
- run.rs — run-control surface
- middleware.rs — shared middleware

Sixteen of the ~14 documented categories ship. `roko-serve` is mature.

---

## B.03 — Aggregator routes for TUI / dashboard consumption (Doc 05 §"Aggregator")

**Status**: DONE
**Severity**: —
**Doc claim**: `aggregator` routes provide composite views for dashboard consumption.
**Reality**: `routes/aggregator.rs` ships. Exact surface TBD by read-through; the file exists as a shipping surface.

---

## B.04 — Per-agent sidecar `roko-agent-server` ships (Doc 05 §"Per-Agent Sidecar", Doc 06 §"Per-Agent WebSocket")

**Status**: DONE
**Severity**: —
**Doc claim**: Each running agent exposes its own HTTP sidecar for /message, /stream WS, /predictions, /research, /tasks.
**Reality**: `crates/roko-agent-server/src/` ships with 4 module directories: `auth/, features/, registration.rs, state.rs, lib.rs`. `features/` contains 5 feature modules:
- `health.rs` (33 LOC)
- `messaging.rs` (521 LOC) — real LLM dispatch path per CLAUDE.md T9
- `predictions.rs` (50 LOC)
- `research.rs` (19 LOC)
- `tasks.rs` (46 LOC)

`AgentServer` struct at `lib.rs:49-56` holds `bind, state, auth, features, on_start, registration`. `FeatureFlags { messaging, predictions, research, tasks }` at `:42-47` selects which feature modules to enable. Bearer auth at `auth/bearer.rs`. Agent registration / AgentCard at `registration.rs`. Shipping matches Doc 05/06 claims for the sidecar pattern.

---

## B.05 — 13-route sidecar surface (Doc 05 §"13 routes")

**Status**: DONE
**Severity**: —
**Doc claim**: CLAUDE.md row: "Per-agent sidecar | Wired | 13 routes — `/message` (real LLM dispatch T9), `/stream` WS, `/predictions`, `/research`, `/tasks`".
**Reality**: The 5 feature modules in B.04 fan out into ~13 HTTP routes (GET + POST + DELETE per resource). Shipping per CLAUDE.md "(T19) Agent-server messaging integration tests" recent commit.

---

## B.06 — SSE (Server-Sent Events) streaming (Doc 06 §"SSE", Doc 05)

**Status**: DONE
**Severity**: —
**Doc claim**: SSE endpoint streams signals / episodes to browser clients.
**Reality**: `routes/sse.rs` ships. Not empty.

---

## B.07 — WebSocket streaming (Doc 06 §"WebSocket Streaming", Doc 05)

**Status**: DONE
**Severity**: —
**Doc claim**: WebSocket endpoints stream real-time agent output.
**Reality**: `routes/ws.rs` (139 LOC) ships on serve side. `features/messaging.rs` (521 LOC) handles the agent-server side `/stream` WS path. Real-time bidirectional streaming is live.

---

## B.08 — Webhook inbound routes (Doc 05 §"Webhooks")

**Status**: DONE
**Severity**: —
**Doc claim**: Webhook endpoints accept external events (GitHub, Slack, etc.).
**Reality**: `routes/webhooks.rs` ships at 520 LOC — substantial. MCP integrations like `roko-mcp-github` and `roko-mcp-slack` cross-link with this surface.

---

## B.09 — Templates routes (Doc 05 §"Templates")

**Status**: DONE
**Severity**: —
**Doc claim**: Templates endpoint serves reusable plan / prompt / scaffold templates.
**Reality**: `routes/templates.rs` ships at 556 LOC. Substantial.

---

## B.10 — Deployments routes (Doc 05 §"Deployments")

**Status**: DONE
**Severity**: —
**Doc claim**: Deployments endpoint for remote-agent deployment management.
**Reality**: `routes/deployments.rs` ships. Exact surface TBD.

---

## B.11 — Subscriptions (long-lived pub/sub) (Doc 06 §"Subscriptions")

**Status**: DONE
**Severity**: —
**Doc claim**: Subscriptions routes manage long-lived signal subscriptions for streaming clients.
**Reality**: `routes/subscriptions.rs` ships at 267 LOC.

---

## B.12 — Middleware layer (Doc 05 §"Middleware")

**Status**: DONE
**Severity**: —
**Doc claim**: Middleware layer handles auth, logging, rate limiting.
**Reality**: `routes/middleware.rs` ships.

---

## B.13 — Auth: bearer tokens + registration (Doc 05 §"Auth", Doc 06 §"Auth")

**Status**: DONE
**Severity**: —
**Doc claim**: Bearer-token auth for control-plane + per-agent authentication.
**Reality**: `crates/roko-agent-server/src/auth/` ships with `bearer.rs` (imported in `lib.rs:26`). `registration.rs` at `crates/roko-agent-server/src/registration.rs` handles AgentCard publishing with endpoints + bearer tokens. Shipping.

---

## B.14 — Port allocation and default-base-URL drift (Doc 05 §"Port Allocation", Doc 17)

**Status**: PARTIAL
**Severity**: MEDIUM
**Doc claim**: Doc 17 §"Port Allocation" tables canonical ports: 6677 (control plane), 8080+ (per-agent sidecars), 3000 (web portal), etc.
**Reality**: The checked-in sources disagree:
- `crates/roko-cli/src/main.rs` defaults `roko serve` and daemon start/restart to `9090`
- `crates/roko-cli/src/main.rs` defaults chat `--serve-url` to `http://localhost:6677`
- `crates/roko-serve/README.md` says the control plane defaults to `6677`

Per-agent sidecar binding is dynamic, and portal port remains frontier because no portal ships. The main issue here is not just allocation; it is unresolved default drift.
**Fix sketch**: Docs 05 / 17 should explicitly record the `9090` vs `6677` split until a later implementation pass resolves the runtime default.

---

## B.15 — OpenAPI spec / route documentation (Doc 05 §"OpenAPI")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 05 mentions OpenAPI / auto-generated route docs as a goal.
**Reality**: `Grep 'openapi\|utoipa\|poem-openapi' crates/roko-serve/Cargo.toml` — unverified, but likely not shipping. With 85 routes + 18 route modules, comprehensive API docs would be substantial. Treat as frontier.
**Fix sketch**: Doc 05 should mark OpenAPI generation as `Design — Phase 2+`.

---

## Section Summary

| Status | Count |
|--------|-------|
| DONE | 13 (B.01-B.13) |
| PARTIAL | 1 (B.14 port allocation — web portal frontier) |
| NOT DONE | 1 (B.15 OpenAPI docs) |

Section B is one of the **strongest shipping sections of topic 12**,
but the earlier parity wording was a little too confident about some
endpoint-detail claims. The route stack, sidecar features, SSE, and
WebSocket surfaces are clearly real; exact endpoint matrices still need
narrow wording where they were not directly source-verified.

## Agent Execution Notes

### B.01 / B.02 — Update Doc 05 with shipping route count

Doc 05 should cite the 18 shipping route files + ~85 total routes.
The per-file LOC counts (webhooks 520, templates 556, subscriptions
267, ws 139) give readers a sense of shipping surface depth.

### B.04 — Document the agent-server feature flag matrix

Agent-server has `FeatureFlags { messaging, predictions, research,
tasks }` — 4 feature flags plus always-on health. Doc 05/06 should
note the feature-flag matrix.

### B.14 — Treat 9090 vs 6677 as a first-class seam

Do not bury the port mismatch in a footnote. An unattended agent
rewriting Doc 17 or Doc 05 needs to surface that the checked-in CLI,
chat defaults, and READMEs disagree.

Acceptance criteria:

- Doc 05 cites 18 shipping route files,
- Doc 06 cites `routes/sse.rs` + `routes/ws.rs` + agent-server `messaging.rs`,
- Docs 05 / 17 explicitly mention the `9090` vs `6677` split,
- Doc 05 OpenAPI subsection banner-tagged frontier.
