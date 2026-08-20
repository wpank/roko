# 154 — HTTP Monitoring Workflow Documentation

**Priority**: P3 — documentation-only task; all endpoints already serve real data
**Size**: XS (half day)
**Crates**: None (documentation only)
**Depends on**: None
**Sources**: `tmp/mori-old/IMPLEMENTATION-CHECKLIST.md` §4.4, `tmp/mori-old/16-ROKO-HTTP-ROUTES-AUDIT.md`

---

## Background

Roko has an HTTP control plane (`roko serve` on port 6677) with ~365 routes that serve real data — plans, agents, learning state, gate results, SSE event streams, system metrics, etc. An audit (`tmp/mori-old/16-ROKO-HTTP-ROUTES-AUDIT.md`) confirmed ~97% of routes return real data.

However, there is no documentation describing which endpoints are most useful for monitoring a live roko session, how to subscribe to real-time events via SSE, or how Claude (or another agent) should use the HTTP API to observe and diagnose a running plan execution.

The endpoints exist and work. What's missing is a focused operator/agent monitoring guide that tells you: "When roko is running, here's how to watch it via HTTP."

## Current State

- `crates/roko-serve/src/routes/` — all route modules exist with real implementations
- `roko serve` starts on port 6677 (configurable)
- Routes include: health, plans, agents, learning (episodes, routing, playbook), gates, metrics, signals, feeds, triggers, surfaces, and SSE event streams
- `tmp/mori-old/16-ROKO-HTTP-ROUTES-AUDIT.md` — comprehensive route catalog from an external audit
- No existing monitoring-focused documentation exists
- OpenAPI spec exists for named surfaces (E37) but not as a general monitoring guide

## Implementation Plan

1. **Create `docs/monitoring.md`**: A single focused document covering the HTTP monitoring workflow. Organize by use case:

   a. **Health check**: `GET /api/health` — is roko healthy?
   b. **Plan status**: `GET /api/plans` — list all plans with status. `GET /api/plans/{id}` — detail for one plan including task states.
   c. **Active agents**: `GET /api/agents` — list running agents with model, role, status
   d. **Learning state**:
      - `GET /api/learning/episodes` — episode history (pass/fail)
      - `GET /api/learning/routing` — cascade router state (model selection stats)
      - `GET /api/learning/playbook` — learned playbook rules
   e. **Gate results**: `GET /api/gates/results` — recent gate pass/fail with error details
   f. **Real-time events (SSE)**: `GET /api/events` — subscribe to live event stream (task start/complete, gate results, agent lifecycle, errors)
   g. **System metrics**: `GET /api/metrics` — CPU, memory, disk, token burn
   h. **Cost tracking**: How to derive total cost from learning/efficiency data

2. **Include curl examples**: Every endpoint should have a runnable `curl` example:
   ```bash
   # Check if roko is running
   curl -s localhost:6677/api/health | jq .

   # Watch real-time events
   curl -N localhost:6677/api/events

   # Get plan status
   curl -s localhost:6677/api/plans | jq '.[] | {id, status, tasks_done, tasks_total}'
   ```

3. **Document the SSE event format**: Show the event types, their JSON payloads, and which events are most useful for monitoring plan execution:
   - `task_started` — new task beginning execution
   - `task_completed` — task finished (with gate result)
   - `gate_result` — gate pass/fail with error details
   - `agent_spawned` / `agent_stopped` — agent lifecycle
   - `plan_completed` — plan finished execution
   - `error` — runtime errors

4. **Add a "Claude monitoring recipe"**: A step-by-step recipe for how an agent should monitor a running plan execution:
   a. Start `roko serve` in background
   b. Start `roko plan run` in background
   c. Poll `/api/plans` every 30s for status
   d. On `task_completed` events, check gate results
   e. On gate failure, hit `/api/gates/results` for error details
   f. Use `roko diagnose <plan-id>` for structured failure analysis
   g. After completion, check `/api/learning/routing` for model quality stats

5. **Verify route availability**: Before documenting each endpoint, verify it exists and returns real data by checking the route modules in `crates/roko-serve/src/routes/`. Note any endpoints that return stub/placeholder data.

## Acceptance Criteria

1. `docs/monitoring.md` exists with all sections listed above
2. Every documented endpoint has a runnable `curl` example
3. SSE event format is documented with example payloads
4. The Claude monitoring recipe is complete and executable
5. All documented endpoints are verified to exist in the route modules

## Verification Checklist

- [ ] `docs/monitoring.md` is well-structured and readable
- [ ] Run `roko serve` and verify each documented `curl` example returns data
- [ ] SSE event stream connects and delivers events during `roko plan run`
- [ ] The Claude monitoring recipe can be followed step-by-step

## Files to Modify

| File | Change |
|---|---|
| `docs/monitoring.md` | New: complete HTTP monitoring workflow guide |
