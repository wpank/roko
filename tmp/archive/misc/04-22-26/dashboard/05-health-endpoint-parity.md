# Task 05: Health Endpoint Parity

**Priority**: P1
**Crate**: `roko-serve`
**File**: `crates/roko-serve/src/routes/status.rs`

## Problem

Minor shape differences between roko-serve's health response and what the dashboard expects.

### GET /api/health

**What roko-serve returns:**
```json
{
  "status": "ok",
  "version": "0.x.x",
  "uptime_secs": 12345,
  "active_plans": 2,
  "active_agents": 3,
  "active_runs": 1,
  "providers": {
    "total": 5,
    "healthy": 4,
    "unhealthy": 1
  }
}
```

**What the dashboard expects:**
```json
{
  "status": "ok" | "degraded" | "down",
  "version": "string",
  "uptime_secs": 12345,
  "active_plans": 2,
  "active_agents": 3
}
```

**Differences:**
1. Dashboard doesn't use `active_runs` or `providers` — but extra fields are harmless (JS ignores them)
2. Dashboard expects `status` to be one of `"ok"`, `"degraded"`, `"down"` — verify roko-serve uses these exact strings
3. Dashboard uses `active_agents` — verify this counts the right thing (discovered agents? supervised processes?)

This is low risk because extra fields in JSON are safely ignored by the dashboard. The
main concern is ensuring `status` values and `active_agents` semantics match.

## Implementation

### Step 1: Verify status values

Read the health handler in `status.rs` (around lines 53-85). Check what logic determines
the `status` field:

- If all providers healthy → `"ok"`
- If some providers unhealthy → `"degraded"`
- If server is shutting down or critical failure → `"down"`

The dashboard uses these values to set a status badge color. Verify the exact strings match.

### Step 2: Verify active_agents counts discovered agents

The dashboard shows `active_agents` as a top-level metric. Check whether roko-serve counts:
- `state.discovered_agents.len()` — agents that have registered via heartbeat
- `state.supervisor.running_count()` — locally managed processes

The dashboard likely wants the former (total agents communicating with serve, not just
locally spawned ones). Verify and adjust if needed.

### Step 3: No-op if already correct

If the response already matches (status strings are correct, active_agents is sensible),
this task is done — just document the verification.

## Files to modify

| File | Change |
|------|--------|
| `crates/roko-serve/src/routes/status.rs` | Fix status strings or agent count if needed |

## Verification

### Manual

```bash
cargo run -p roko-cli -- serve &

HEALTH=$(curl -s http://127.0.0.1:6677/api/health)

# Verify status is one of expected values
STATUS=$(echo "$HEALTH" | jq -r '.status')
echo "Status: $STATUS"
[[ "$STATUS" == "ok" || "$STATUS" == "degraded" || "$STATUS" == "down" ]] && \
  echo "PASS: valid status" || echo "FAIL: unexpected status value '$STATUS'"

# Verify required fields exist
echo "$HEALTH" | jq 'has("status", "version", "uptime_secs", "active_plans", "active_agents")'
# MUST be true

# Verify types
echo "$HEALTH" | jq '.uptime_secs | type'  # "number"
echo "$HEALTH" | jq '.active_plans | type'  # "number"
echo "$HEALTH" | jq '.active_agents | type' # "number"
```

## Acceptance criteria

- [ ] `GET /api/health` returns `status` as one of `"ok"`, `"degraded"`, `"down"`
- [ ] Response includes `version` (string), `uptime_secs` (number), `active_plans` (number), `active_agents` (number)
- [ ] `active_agents` counts agents communicating with serve (not just local processes)
- [ ] All existing tests still pass
