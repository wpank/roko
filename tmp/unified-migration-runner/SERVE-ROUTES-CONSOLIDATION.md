# roko-serve Route Consolidation — Implementation Prompt

> **Goal**: Split oversized route files in `crates/roko-serve/src/routes/`. Six files
> exceed 1,800 lines. Split by domain for testability and discoverability.

## Context

`roko-serve` has 33 route modules. The largest ones conflate too many endpoints:

| File | Lines | Issue |
|---|---|---|
| `status.rs` | 2,490 | 85+ endpoints: health, metrics, episodes, signals, gates, cascades, experiments, dashboard |
| `agents.rs` | 2,086 | Agent CRUD + topology + stream management |
| `learning.rs` | 1,885 | Router state, experiments, efficiency, c-factor, playbooks |
| `plans.rs` | 1,860 | Plan CRUD + task execution + status |
| `jobs.rs` | 1,747 | Job marketplace + execution + cancellation |
| `gateway.rs` | 1,338 | Model routing + streaming + batch |

### Files to read first
```
crates/roko-serve/src/routes/mod.rs      — build_router() with all merges
crates/roko-serve/src/routes/status.rs   — 2,490 lines, worst offender
```

---

## Tasks

### SR001 — Split status.rs by domain

**Objective**: `status.rs` → 5 focused files.

**Steps**:
1. Create `crates/roko-serve/src/routes/status/mod.rs`
2. Extract: `status/health.rs` (health, relay_health, parity)
3. Extract: `status/metrics.rs` (metrics, metrics_summary, success_rate, engagement, prometheus, etc.)
4. Extract: `status/episodes.rs` (episodes endpoint)
5. Extract: `status/signals.rs` (signals endpoint)
6. Extract: `status/gates.rs` (gates_summary, gates_history, gate_history)
7. Keep `status/mod.rs` with `pub fn routes()` that merges all sub-routers
8. Update `routes/mod.rs` import

---

### SR002 — Split learning.rs by concern

**Objective**: `learning.rs` → 3 files.

**Steps**:
1. Extract: `learning/router.rs` (cascade router state, routing decisions)
2. Extract: `learning/experiments.rs` (A/B experiments, model experiments)
3. Keep: `learning.rs` with efficiency, c-factor, playbooks, episodes

---

### SR003 — Split plans.rs

**Objective**: `plans.rs` → 2 files.

**Steps**:
1. Extract: `plans/execution.rs` (plan execution, task dispatch, status polling)
2. Keep: `plans.rs` with CRUD (list, show, create, delete)

---

### SR004 — Split agents.rs

**Objective**: `agents.rs` → 2 files.

**Steps**:
1. Extract: `agents/topology.rs` (agent discovery, topology, stream management)
2. Keep: `agents.rs` with CRUD (list, register, deregister, status)

---

### SR005 — Add the missing endpoints from dogfood

**From**: `tmp/dogfood/01-endpoint-audit.md`

**Add**:
- `GET /api/plans/:id` → return plan state from executor snapshot
- `GET /api/plans/:id/tasks` → return tasks from tasks.toml
- `GET /api/knowledge` → query neuro store entries
- `GET /api/knowledge?query=<topic>` → search by topic
- `GET /api/learn/router` → cascade router snapshot
- `GET /api/executor/state` → executor.json contents

**Verification**:
```bash
cargo check -p roko-serve
cargo test -p roko-serve
```
