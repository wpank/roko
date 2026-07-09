# Task 03: Agents Endpoint Parity

**Priority**: P0
**Crate**: `roko-serve`
**Files**: `crates/roko-serve/src/routes/agents.rs`, `crates/roko-serve/src/state.rs`

## Problem

The dashboard expects rich agent metadata that roko-serve doesn't currently return.

### GET /api/managed-agents — missing fields

**What roko-serve returns now:**
```json
[
  { "id": "12345", "label": "agent-label" }
]
```

**What the dashboard expects** (type `AgentSummary` in `rokoApi.ts`):
```json
[
  {
    "id": 12345,
    "label": "agent-name",
    "status": "running",
    "role": "implementer",
    "model": "claude-sonnet-4-20250514",
    "tier": "T2",
    "current_task": "T3: Wire safety checks"
  }
]
```

Missing: `status`, `role`, `model`, `tier`, `current_task`.

The dashboard uses these in the Agents tab to show agent cards with health indicators,
model badges, and current work assignment. Without them, agents render as empty/broken cards.

### GET /api/agents/{id} — verify response matches

**What the dashboard expects** (type `Agent`):
```json
{
  "agent_id": "agent-1",
  "label": "implementer-1",
  "process_id": 12345,
  "owner": "user",
  "endpoints": {
    "rest": "http://127.0.0.1:9001",
    "websocket": "ws://127.0.0.1:9001/stream",
    "a2a": null,
    "mcp": null
  },
  "card_uri": null,
  "capabilities": ["coding", "testing"],
  "domain_tags": ["rust", "backend"]
}
```

This endpoint reads from the `DiscoveredAgent` registry in AppState. Verify the field
names match — particularly `agent_id` vs `id`, and the `endpoints` nested object.

## Implementation

### Step 1: Enrich managed-agents response

The GET `/api/managed-agents` handler (around line 35-48 in `agents.rs`) currently returns
minimal data from `ProcessSupervisor`. Enrich it by cross-referencing:

1. **status**: From `ProcessSupervisor` process state — is the process alive?
   - `"running"` if process is alive
   - `"stopped"` if process has exited
   - `"starting"` if recently spawned
   - `"error"` if exited with non-zero

2. **role**: From the agent's configuration. When an agent is spawned via `plan run`,
   the task assigns a role (e.g., `"implementer"`, `"reviewer"`, `"researcher"`).
   Check `PlanHandle` or the agent manifest in `.roko/agents/`.

3. **model**: From the agent's dispatch config. The `CascadeRouter` or agent config
   specifies which model the agent uses. Check `roko.toml` agent definitions or
   the dispatch config passed at spawn time.

4. **tier**: From the cascade router tier. The tier routing system in `roko-primitives`
   assigns agents to tiers (T1-T5). Check if the agent's model maps to a tier.

5. **current_task**: From active plan execution state. If the agent is currently executing
   a task, `PlanHandle` tracks which task ID. Format as `"T3: <task description>"`.

**Approach**: The `AppState` struct has all of these data sources:
- `state.supervisor` → process status
- `state.active_plans` → current task assignments
- `state.discovered_agents` → capabilities, endpoints
- `state.config` → agent model/role config

Cross-reference by agent ID across these sources.

**Updated struct:**
```rust
#[derive(Serialize)]
struct ManagedAgentSummary {
    id: u32,           // Note: dashboard expects number, not string
    label: String,
    status: String,
    role: Option<String>,
    model: Option<String>,
    tier: Option<String>,
    current_task: Option<String>,
}
```

**Important**: The dashboard expects `id` as a number (process ID), not a string.
Check the current serialization.

### Step 2: Verify GET /api/agents/{id}

Read the handler for `GET /api/agents/{id}` (in `agents.rs`) and compare its serialization
against the dashboard's `Agent` type. Key fields to verify:

- `agent_id` (not just `id`)
- `process_id` (nullable number)
- `endpoints` as nested object with `rest`, `websocket`, `a2a`, `mcp` keys
- `capabilities` as string array
- `domain_tags` as string array

The `DiscoveredAgent` struct in `state.rs` has an `endpoints` field — verify it serializes
with the exact key names the dashboard expects.

### Step 3: Handle missing data gracefully

Not all fields will always be available (e.g., an agent spawned manually won't have a
role or current_task from a plan). Use `Option<String>` for nullable fields and serialize
them as `null` in JSON. The dashboard handles null gracefully for optional fields.

## Files to modify

| File | Change |
|------|--------|
| `crates/roko-serve/src/routes/agents.rs` | Enrich managed-agents handler, verify single-agent handler |
| `crates/roko-serve/src/state.rs` | May need helper to query agent metadata across data sources |

## Verification

### Automated

```bash
cargo build -p roko-serve
cargo test -p roko-serve
cargo clippy -p roko-serve --no-deps -- -D warnings
```

### Manual — managed-agents

```bash
cargo run -p roko-cli -- serve &

AGENTS=$(curl -s http://127.0.0.1:6677/api/managed-agents)

# Verify response is an array
echo "$AGENTS" | jq 'type'
# MUST be "array"

# If agents exist, verify fields
echo "$AGENTS" | jq '.[0] | keys'
# MUST include at minimum: id, label, status
# SHOULD include: role, model, tier, current_task (may be null)

# Verify id is a number
echo "$AGENTS" | jq '.[0].id | type'
# MUST be "number"

# Verify status is a valid string
echo "$AGENTS" | jq '.[0].status'
# MUST be one of: "running", "stopped", "starting", "error"
```

### Manual — with a running agent

```bash
# Start an agent to have data
cargo run -p roko-cli -- agent start --name test-agent &

# Wait a moment for registration
sleep 3

# Check managed-agents again
curl -s http://127.0.0.1:6677/api/managed-agents | jq .
# Should show the agent with status "running"

# Check single-agent detail
AGENT_ID=$(curl -s http://127.0.0.1:6677/api/managed-agents | jq -r '.[0].id')
curl -s "http://127.0.0.1:6677/api/agents/${AGENT_ID}" | jq .
# Should include: agent_id, endpoints {rest, websocket, a2a, mcp}, capabilities

# Clean up
cargo run -p roko-cli -- agent stop --name test-agent
```

### Manual — during plan execution

```bash
# Execute a plan (must have one available)
PLAN_ID=$(curl -s http://127.0.0.1:6677/api/plans | jq -r '.[0].id')
curl -s -X POST "http://127.0.0.1:6677/api/plans/${PLAN_ID}/execute"

# Check agents while plan runs
curl -s http://127.0.0.1:6677/api/managed-agents | jq '.[] | {label, status, current_task}'
# Active agents should show current_task during execution
```

## Acceptance criteria

- [ ] `GET /api/managed-agents` returns `status` (string) for every agent
- [ ] `GET /api/managed-agents` returns `id` as a number (not string)
- [ ] Optional fields (`role`, `model`, `tier`, `current_task`) are included (as null if unknown)
- [ ] `GET /api/agents/{id}` returns `agent_id`, `endpoints` (nested), `capabilities`, `domain_tags`
- [ ] Agent status reflects real process state (running/stopped/error)
- [ ] `current_task` shows the task being executed during plan runs (when applicable)
- [ ] All existing tests still pass
- [ ] No new clippy warnings
