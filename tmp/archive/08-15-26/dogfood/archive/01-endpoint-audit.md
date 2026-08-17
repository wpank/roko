# Dogfood Endpoint Audit — 2026-04-26

Running `roko plan run .roko/plans/unified-migration-phase0` while querying `roko serve --tui` on :6677.

## Endpoint Status

| Endpoint | HTTP | Size | Verdict |
|---|---|---|---|
| `/health` | 404 | 0B | **MISSING** — no health endpoint |
| `/api/plans` | 200 | 391B | Shows 3 old plans, **NOT the running plan** |
| `/api/status` | 200 | 233B | episode_count=2, signal_count=255 — **stale, not updating** |
| `/api/agents` | 200 | 9.7KB | Lists configured agents from roko.toml, **NOT running agents** |
| `/api/episodes` | 200 | 2B | `[]` — **empty**, plan runner not writing episodes |
| `/api/signals` | 200 | 613KB | 255 signals from prior runs, **nothing from current run** |
| `/api/learn/efficiency` | 200 | 101B | All zeros — **not tracking current run** |
| `/api/learn/router` | 404 | 0B | **MISSING** |
| `/api/connectors` | 200 | 543B | 2 seeded connectors (filesystem, neuro-store) ✓ |
| `/api/feeds` | 200 | 812B | 4 seeded feeds ✓ |
| `/api/knowledge` | 404 | 0B | **MISSING** — neuro store has 14 entries but no route |
| `/api/jobs` | 200 | 44KB | 53 marketplace jobs ✓ |
| `/api/dashboard` | 200 | 1.7KB | Scaffold text only, **no live data** |
| `/api/statehub` | 404 | 0B | **MISSING** — StateHub not exposed via HTTP |
| `/api/executor/state` | 404 | 0B | **MISSING** — no way to query executor state via HTTP |
| `/api/plans/unified-migration-phase0` | 404 | 74B | **MISSING** — can't query individual plan |
| `/api/plans/unified-migration-phase0/tasks` | 404 | 0B | **MISSING** — can't query plan tasks |

## Critical Gaps

### 1. Running plan not visible anywhere via HTTP
The plan runner (`roko plan run`) executes in a separate process. The serve endpoints have **zero visibility** into what it's doing. No executor state, no task progress, no agent activity.

### 2. No executor state endpoint
`.roko/state/executor.json` doesn't exist during the run. The executor state is only in-memory. There's no HTTP endpoint to query it.

### 3. Episodes not written during enrichment
The enrichment phase spawns multiple claude agents but writes 0 episodes. The episode log still has only 2 entries from April 24-25.

### 4. Efficiency metrics not tracked
`/api/learn/efficiency` returns all zeros. The enrichment agents are consuming tokens but nothing is recorded.

### 5. StateHub not exposed
The `state_hub` is used internally (TUI reads it), but there's no HTTP endpoint. External dashboards can't subscribe to plan progress.

### 6. No health endpoint
`/health` returns 404. Basic liveness check missing.

### 7. Knowledge endpoint missing
14 knowledge entries exist in the neuro store but there's no GET route to retrieve them.

### 8. Plan detail routes missing
Can list plans but can't GET `/api/plans/:id` or `/api/plans/:id/tasks`. No way to inspect a specific plan's state via API.

## Data Files During Run

| File | State |
|---|---|
| `.roko/episodes.jsonl` | 2 lines (stale, not updating) |
| `.roko/signals.jsonl` | 0 lines |
| `.roko/learn/efficiency.jsonl` | missing |
| `.roko/state/executor.json` | **does not exist** |
| `.roko/state/server-state.json` | exists, 12KB (TUI state) |
| `.roko/state/mirage-snapshot.json` | 343KB (TUI snapshot) |

## Enrichment Artifacts Written

The enrichment phase DID produce artifacts in `.roko/plans/unified-migration-phase0/`:
- `brief.md` — minimal, just the plan description repeated
- `research.md` — minimal, "10 tasks defined"
- `dependency-manifest.toml` — unknown content
- `fixture-manifest.toml` — unknown content
- `integration.md` — unknown content
- `prd-extract.md` — unknown content

Several enrichment steps **failed**:
- `decompose`: timed out (120s)
- `verify`: TOML parse error (LLM output wrapped in markdown fences)
- `reviews`: exit signal (claude crashed)
- `tests`: timed out
- `invariants`: timed out
- `scribe`: exit signal

## TUI Observations

The TUI shows:
- "no parallel agents" on Agents tab — plan runner agents not visible
- "no agent output yet" — enrichment output not piped to TUI
- Efficiency panel: "waiting for data..."
- Routes panel: "no agent route metrics"
- Diagnosis panel: "no conductor diagnoses yet"
