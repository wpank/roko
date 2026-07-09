# PR #7 Reply — API Architecture Clarification

## Fixed in this branch

### 1. Router ordering bug (the "POST is required" issue)

The endpoints you listed (`/api/agents`, `/api/agents/{id}/trace`, `/api/agents/{id}/heartbeat`, `/api/agents/{id}/stats`, `/api/tasks`) **are all defined as GET** in the HTTP API router (`apps/mirage-rs/src/http_api/mod.rs:217-238`). They exist and are fully implemented.

The problem was a **router ordering issue** in `apps/mirage-rs/src/rpc.rs`. Axum's `.fallback_service()` was registered _before_ the `/api` routes were nested:

```rust
// BEFORE (broken):
let mut app = Router::new()
    .route("/health", get(health_handler))
    .fallback_service(rpc_fallback)      // catches everything
    .with_state(local_state);
if let Some(api) = api_router {
    app = app.nest("/api", api);         // never reached
}

// AFTER (fixed):
let mut app = Router::new()
    .route("/health", get(health_handler))
    .with_state(local_state);
if let Some(api) = api_router {
    app = app.nest("/api", api);         // REST routes matched first
}
app = app.fallback_service(rpc_fallback); // JSON-RPC only for unmatched paths
```

All `/api/*` GET endpoints should now work.

### 2. Heartbeat block number always 0

Three related bugs — the heartbeat handlers (both HTTP REST and JSON-RPC) were passing `0` as the current block number:

- `GET /api/agents/{id}/heartbeat` — `blocks_since` was computed as `0u64.saturating_sub(last_heartbeat_block)` which is always 0
- `POST /api/agents/{id}/heartbeat` — recorded `block: 0` in the heartbeat, so liveness checks were meaningless
- `chain_agentHeartbeat` JSON-RPC — same issue

**Fix:** Added a `current_block` callback to `ApiState` that reads `MirageState.fork.local_block_number` from the live fork. The JSON-RPC handler now also reads the current block from `ctx.state`. All three paths now use the real block number.

### 3. Test infrastructure updated

Updated all test `ApiState` construction sites to use the new `ApiState::new()` constructor. All 51 tests pass.

---

## Two servers, not one

There are **two independent HTTP servers** in this PR:

| | **mirage-rs** | **roko serve** |
|---|---|---|
| **Port** | 8545 | 9090 |
| **Purpose** | Chain simulator (EVM fork + knowledge/pheromone layer) | Orchestration API (plans, PRDs, agents, learning) |
| **API type** | JSON-RPC primary + REST at `/api/*` (conditional on `chain` feature) | REST only at `/api/*` |
| **Binary** | Standalone `mirage-rs` app | Part of `roko` CLI |
| **State** | Chain state, knowledge store, pheromone field, agent registry | Plan execution, episode logs, learning data |

They don't share state, ports, or routes. They are completely decoupled.

### What this means for the dashboard

Some of the endpoints you need live on **mirage-rs** (:8545) and some live on **roko serve** (:9090):

**On mirage-rs (:8545) — chain/knowledge data (all fixed now):**
- `GET /api/health` ✓
- `GET /api/pheromones` ✓
- `GET /api/knowledge/entries` ✓
- `GET /api/knowledge/edges` ✓
- `GET /api/knowledge/search` ✓
- `GET /api/agents/topology` ✓
- `GET /api/agents` ✓
- `GET /api/agents/{id}/trace` ✓
- `GET /api/agents/{id}/heartbeat` ✓ (now with real block numbers)
- `GET /api/agents/{id}/stats` ✓
- `GET /api/tasks` ✓
- `GET /api/stats` ✓
- `WS /api/ws` ✓ (needs `roko` feature flag)

**On roko serve (:9090) — orchestration data:**
- `GET /api/health`
- `GET /api/agents`, `GET /api/agents/{id}`
- `GET /api/plans`, plan lifecycle (start/pause/abort/resume)
- `GET /api/prds`, PRD lifecycle
- Learning data (episodes, playbooks, cascade router, efficiency, experiments)
- Deployment management (`/api/deployments`)
- Config management
- `WS /ws` (plan execution updates)

Run both servers. Don't proxy one through the other.

## Addressing the open issues

### Issue #1 — HTTP API for mirage-rs
**Status: Fixed.** The HTTP API was already implemented and comprehensive. The router ordering bug was the only blocker. Now fixed.

### Issue #5 — Reasoning trace, heartbeat, achievements

**Agent reasoning trace (`/api/agents/{id}/trace`):** Fully implemented. GET returns paginated traces with `cycle`, `phase`, `reads`, `reasoning`, `action`, `action_id`, `timestamp`. POST accepts new entries. Check if the schema matches what you need for CoALA phases.

**Agent heartbeat (`/api/agents/{id}/heartbeat`):** Fully implemented and now returns accurate `blocks_since` and `alive` status using the real block number.

**Agent stats (`/api/agents/{id}/stats`):** Fully implemented. Returns `confirmations_given`, `challenges_given`, `warnings_posted`, `insights_posted`, `delta_cycles`, `total_cost_usd`, `total_tokens`.

**Block timestamps:** The auto-miner already sets `timestamp = now_secs()` on mine (`fork.rs:1256`). If you're seeing `timestamp: 0`, check if you're reading from a block that was created before the auto-miner started, or from the genesis/initial block.

**Insight dual-system:** `chain_postInsight` (JSON-RPC) writes to Rust-side `KnowledgeStore`. `InsightBoard.sol` is separate. They don't sync. For the demo, pick one source of truth. Long term, sync contract events into KnowledgeStore.

### Issue #6 — One-click deploy / agent templates

**For the demo:** Option B (in-process spawn via mirage-rs) is fastest. Add a `POST /api/agents/spawn` endpoint that creates an agent from a template config, starts it in-process, and registers it in the agent registry.

**Deployment endpoints on roko serve:** `POST /api/deployments`, `GET /api/deployments`, `DELETE /api/deployments/{id}` exist in roko serve's routes. Need Railway API credentials. No mock mode yet — for demo, Option B avoids this.

## Cleanest separation going forward

- **mirage-rs** = the **chain**. Knowledge graph, pheromone field, agent registry, task tracking, block production. The world state agents interact with.
- **roko serve** = the **operator**. Plan management, PRD lifecycle, agent orchestration, learning/feedback, deployments. The control plane.

### WebSocket

- mirage-rs: `WS /api/ws` (behind `roko` feature) — streams pheromone events, insight events, and agent events. Supports filtering by channel and agent ID via query params (`?pheromones=true&insights=true&agents=true&agent_id=xxx`).
- roko serve: `WS /ws` — streams plan execution updates.

Dashboard should open both connections.

## Remaining work

1. **`POST /api/agents/spawn`** — new endpoint for in-process agent creation from templates (issue #6)
2. **Decide insight source of truth** — KnowledgeStore vs InsightBoard.sol (issue #5, item 5)
3. Run `roko serve` alongside mirage-rs for the full API surface
