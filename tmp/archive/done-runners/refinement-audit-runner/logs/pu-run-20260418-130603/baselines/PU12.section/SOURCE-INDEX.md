# SOURCE-INDEX — Code Anchors for 12-Interfaces Parity

Verified code references for batch `12`, organized by surface.

Generated: 2026-04-16

---

## Important Corrections First

- The shipping interface core is real: CLI, TUI, HTTP control plane, and per-agent sidecar all exist.
- `roko new` does **not** appear in the checked-in top-level CLI command enum in `crates/roko-cli/src/main.rs`.
- Doc 03’s standalone `roko explain` command does **not** appear in the checked-in top-level CLI command enum. A verified `--explain` flag exists on `model route`, which is a different surface.
- The serve-port story is internally inconsistent:
  - `crates/roko-cli/src/main.rs` defaults `roko serve` and daemon ports to `9090`
  - `crates/roko-cli/src/main.rs` defaults chat `--serve-url` to `http://localhost:6677`
  - `crates/roko-serve/README.md` says `6677` by default
- `roko-agent-server` really does ship WebSocket streaming at `/stream`; that was worth verifying directly.
- Spectre, web portal frontend, A2UI runtime, sonification, ACP runtime, and VS Code extension remain absent.

---

## crates/roko-cli/src/main.rs — Live Command Tree

### Verified top-level subcommands

| Command | What | Section |
|---------|------|---------|
| `init` | initialize `.roko/` and config | A.08 |
| `run` | universal loop entry | A.02 |
| `status` | status + optional C-Factor | A.06 |
| `replay` | DAG/lineage walk | A.06 |
| `dream` | run/report/schedule | A.01 |
| `config` | init/show/path/edit/set/set-secret | A.07, A.11 |
| `inject` | inject signal into running session | A.13 |
| `plan` | list/show/create/validate/run/generate/regenerate | A.03 |
| `prd` | idea/list/status/draft/plan/consolidate | A.04 |
| `research` | topic/enhance/analyze/list/search | A.05 |
| `chat` | talk to agent via serve URL | A.06 |
| `neuro` | query/stats/gc | A.01 |
| `subscription` | list/add/remove/enable/disable | A.13 |
| `event-sources` | list configured sources | A.13 |
| `provider` | list/health/test | A.12 |
| `model` | list/route with optional `--explain` | A.10 |
| `experiment` | model experiments | A.01 |
| `deploy` | railway/fly/docker | A.01 |
| `daemon` | start/stop/status/logs/reload/restart/install/uninstall | A.13 |
| `dashboard` | TUI entry with text fallback | C.01 |
| `serve` | HTTP API server | B.01 |
| `worker` | deployed worker mode | A.01 |

### Important negatives

| Claimed Surface | Reality | Section |
|-----------------|---------|---------|
| `roko new` | no verified top-level command in `main.rs` | A.09 |
| `roko explain` | no verified standalone top-level command in `main.rs` | A.10 |
| custom progressive-help flow | no verified command-level implementation from source pass | A.10 |

---

## crates/roko-cli/src/tui/ — Shipping TUI

### Core framing

| File | What | Section |
|------|------|---------|
| `tui/mod.rs` | module entry | C.01 |
| `tui/tabs.rs` | 7 F1-F7 tabs | C.02 |
| `tui/app.rs` | main app state | C.01 |
| `tui/state.rs` | shared TUI state | C.01 |
| `tui/layout.rs` | layout primitives | C.01 |
| `tui/input.rs`, `tui/hit_test.rs`, `tui/scroll.rs` | routing + scroll handling | C.09 |

### Effects / theme / config surfaces

| File | What | Section |
|------|------|---------|
| `tui/atmosphere.rs` | atmosphere effects | C.08 |
| `tui/postfx.rs` | post-processing | C.08 |
| `tui/postfx_pipeline.rs` | effect pipeline | C.08 |
| `tui/effects_config.rs` | effect config | C.08 |
| `tui/config_meta.rs` | config editor surface | C.11 |
| `tui/approval_ipc.rs` | approval bridge | C.10 |

### High-value widgets

| File | What | Section |
|------|------|---------|
| `widgets/plan_tree.rs` | major plan renderer | C.04 |
| `widgets/task_progress.rs` | task progress | C.04 |
| `widgets/token_sparkline.rs` | token display | C.04, D.04 |
| `widgets/status_bar.rs` | status display | C.04, D.04 |
| `widgets/rosedust.rs` | narrow palette/theme constants, not full design language | C.06, C.07 |

### View structure notes

- `find crates/roko-cli/src/tui -type f | wc -l` returned `59`
- `find crates/roko-cli/src/tui/modals -type f | wc -l` returned `14` including `mod.rs`
- `find crates/roko-cli/src/tui/widgets -type f | wc -l` returned `14` including `mod.rs`

Working interpretation:

- shipping TUI is broad and real,
- but Doc 09’s flat “29 screens” model should be reconciled to tabs,
  modals, widgets, and sub-views.

---

## crates/roko-serve/src/routes/ — HTTP Control Plane

### Route wiring

| File | What | Section |
|------|------|---------|
| `routes/mod.rs` | assembles status, plans, prds, run, research, subscriptions, templates, aggregator, agents, learning, config, deployments, providers, sse, ws, webhooks | B.01-B.03 |
| `routes/status.rs` | includes `/api/dashboard` scaffold JSON | B.02 |
| `routes/providers.rs` | `/api/models`, `/api/routing/explain` | B.02 |
| `routes/sse.rs` | `/api/events` SSE | B.06 |
| `routes/ws.rs` | top-level WebSocket routes | B.07 |
| `routes/aggregator.rs` | includes `/api/ws` usage in tests | B.03, B.07 |

### Route inventory

Present route modules:

- `agents.rs`
- `aggregator.rs`
- `config.rs`
- `deployments.rs`
- `learning.rs`
- `middleware.rs`
- `plans.rs`
- `prds.rs`
- `providers.rs`
- `research.rs`
- `run.rs`
- `sse.rs`
- `status.rs`
- `subscriptions.rs`
- `templates.rs`
- `webhooks.rs`
- `ws.rs`
- `mod.rs`

Important caution:

- route-module presence is verified,
- not every doc-described endpoint behavior was verified in this pass,
- do not treat every table cell in Docs 05/06 as source-proven without a second route-level pass.

---

## crates/roko-agent-server/src/ — Per-Agent Sidecar

### High-value surfaces

| File | What | Section |
|------|------|---------|
| `lib.rs` | `AgentServer`, protected/public router split, feature flags | B.04, B.13 |
| `registration.rs` | agent card + websocket field + registration | B.04, B.13 |
| `auth/bearer.rs` | bearer auth | B.13 |
| `features/health.rs` | `/health` | B.04 |
| `features/messaging.rs` | `/message` and `/stream` WebSocket | B.04, B.07 |
| `features/predictions.rs` | predictions routes | B.04 |
| `features/research.rs` | research route | B.04 |
| `features/tasks.rs` | tasks routes | B.04 |

### Verified sidecar routes

`rg` confirmed:

- `/message`
- `/stream`
- `/health`
- `/research`
- `/predictions/{id}`
- `/predictions/residuals`
- `/tasks`
- `/tasks/{id}/accept`
- `/tasks/{id}/complete`

This is stronger proof than the earlier parity note had.

---

## Port / Default Drift

| Surface | Verified Default |
|---------|------------------|
| `roko serve` in `main.rs` | `9090` |
| daemon start/restart in `main.rs` | `9090` |
| chat `--serve-url` in `main.rs` | `http://localhost:6677` |
| `roko-serve/README.md` | `6677` |
| `crates/roko-cli/README.md` examples | mixed `6677` and `9090` |
| docs/12 interface config/CLI | several docs already use `9090` |

Working rule:

Treat this as an explicit docs/status inconsistency until a future pass
resolves the runtime default.

---

## Missing / Absent

| Surface | Evidence | Section |
|---------|----------|---------|
| Spectre renderer/runtime | no meaningful hits in crates | D.* |
| web portal frontend | no portal/web/frontend app directory | E.01-E.03 |
| A2UI runtime/schema | no A2UI schema/renderer types | E.07-E.10 |
| sonification/audio cues | no audio-related runtime | F.01-F.02 |
| ACP runtime / `roko acp` | no verified CLI/runtime surface | F.10-F.11 |
| VS Code extension | no extension project | F.13-F.14 |

---

## Practical Search Priorities

```bash
sed -n '180,760p' crates/roko-cli/src/main.rs
rg -n "9090|6677|serve_url" crates/roko-cli crates/roko-serve docs/12-interfaces tmp/docs-parity/12 --glob '*.rs' --glob '*.md'
rg -n 'route\\(\"/(stream|message|predictions|research|tasks|health)' crates/roko-agent-server/src --glob '*.rs'
rg -n "/api/events|/api/dashboard|/api/ws|/api/models|/api/routing/explain" crates/roko-serve/src/routes --glob '*.rs'
rg -n "Spectre|A2UI|sonif|ACP|VS Code" crates docs/12-interfaces --glob '*.rs' --glob '*.md'
```
