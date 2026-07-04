# Dashboard ↔ Roko Backend Wiring — Task Index

**Created**: 2026-04-22
**Context**: The nunchi-dashboard (Atelier Chat + CLI surface) expects specific API contracts
from roko-serve that don't fully match what roko-serve currently returns. These tasks close
those gaps so the dashboard can operate against a live roko-serve instance.

**Source PRD**: `Nunchi-trade/nunchi-dashboard` repo, branch `wp-demo-dashboard`,
file `docs/atelier-chat.md`

**Dashboard repo**: `/Users/will/dev/nunchi/nunchi-dashboard/`
**Roko repo**: `/Users/will/dev/nunchi/roko/roko/`

## Also in this directory

Files `00-overview.md` through `05-integration-test.md` cover a separate PRD
(**agent-matchmaking**). Those tasks are independent of the ones below.

## Atelier Chat — Task files

| # | File | Summary | Priority |
|---|------|---------|----------|
| 01 | [01-prds-endpoint-parity.md](01-prds-endpoint-parity.md) | Fix GET /api/prds to return title, section, has_plan; fix POST /api/prds/ideas to return slug | P0 |
| 02 | [02-plans-endpoint-parity.md](02-plans-endpoint-parity.md) | Add completed_task_count to GET /api/plans; add status field to plan tasks | P0 |
| 03 | [03-agents-endpoint-parity.md](03-agents-endpoint-parity.md) | Enrich GET /api/managed-agents with status, role, model, tier, current_task | P0 |
| 04 | [04-websocket-path-alias.md](04-websocket-path-alias.md) | Add /roko-ws alias for WebSocket; normalize event type casing to snake_case | P0 |
| 05 | [05-health-endpoint-parity.md](05-health-endpoint-parity.md) | Verify /api/health shape matches dashboard expectations (minor) | P1 |
| 06 | [06-missing-http-endpoints.md](06-missing-http-endpoints.md) | Add POST /api/neuro/query, POST /api/prd/consolidate, POST /api/dream/run | P1 |
| 07 | [07-ws-event-format-normalization.md](07-ws-event-format-normalization.md) | Normalize all ServerEvent variants to match dashboard WsEventPayload types | P0 |
| 08 | [08-job-field-alignment.md](08-job-field-alignment.md) | Align job state/field names between serve and dashboard (state vs status, etc.) | P1 |

## Dependency order

```
01, 02, 03 — independent, can run in parallel
04, 07     — related (WS), 04 before 07
05         — independent
06         — independent
08         — independent
```

## How to verify the full integration

After all tasks are complete, run this end-to-end check:

```bash
# 1. Start roko-serve
cd /Users/will/dev/nunchi/roko/roko
cargo run -p roko-cli -- serve

# 2. In another terminal, start the dashboard
cd /Users/will/dev/nunchi/nunchi-dashboard
# Set VITE_ROKO_URL=http://127.0.0.1:6677
pnpm dev

# 3. Verify each endpoint returns expected shape
curl -s http://127.0.0.1:6677/api/health | jq .
curl -s http://127.0.0.1:6677/api/prds | jq .
curl -s http://127.0.0.1:6677/api/plans | jq .
curl -s http://127.0.0.1:6677/api/managed-agents | jq .
curl -s http://127.0.0.1:6677/api/jobs | jq .

# 4. Test WebSocket (wscat or websocat)
websocat ws://127.0.0.1:6677/roko-ws

# 5. Test Atelier commands in dashboard
#    /idea "test idea"   → should create PRD, return slug
#    /plan <slug>        → should generate plan
#    /run <plan-id>      → should execute, WS events flow
```
