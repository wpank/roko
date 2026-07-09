# ISFR Implementation Tasks — Master Index

## Build Order & Dependency Graph

```
Phase A: Relay Pub/Sub (can run in parallel with D)
  A1 → A2 → A3 → A4
                → A5
  A1 → A6

Phase B: Feed Integration (requires A6 + existing Feed trait)
  A6 → B1

Phase C: ISFR Keeper + Tools (independent of A until runtime wiring)
  C1 → C2 → C3

Phase D: Contracts & Chain Profile (independent, parallel with A)
  D1 → D2

Phase E: Integration (requires C + E1)
  E1 (config schema — can start early, no deps)
  E2 (CLI command — requires C1, C2, E1)

Phase F: Demo-App UI (requires E2 + F1 for data)
  F1 → F2 → F3 → F4
                → F5
```

## Task Summary

| ID | Title | Crate/App | ~Lines | Depends On |
|----|-------|-----------|--------|------------|
| **A1** | Add pub/sub frame types to relay protocol | agent-relay | ~50 | — |
| **A2** | Implement topic pub/sub Bus module | agent-relay | ~150 | A1 |
| **A3** | Wire Bus into relay WebSocket handler | agent-relay | ~100 | A1, A2 |
| **A4** | Add chain event watcher to relay | agent-relay | ~130 | A2, A3 |
| **A5** | Add feed metadata HTTP endpoints | agent-relay | ~60 | A2, A3 |
| **A6** | Upgrade relay client with pub/sub | roko-agent-server | ~120 | A1 |
| **B1** | Implement ISFRFeed (relay-backed) | roko-core or roko-chain | ~150 | A6 |
| **C1** | Define ISFRSource trait + MockSource | roko-chain | ~200 | — |
| **C2** | Implement ISFRKeeper orchestrator | roko-chain | ~220 | C1 |
| **C3** | Implement ISFR domain tools | roko-std | ~200 | C1, C2 |
| **D1** | Implement ChainProfile abstraction | roko-chain | ~100 | — |
| **D2** | Contract ABIs + bootstrap function | roko-chain | ~80 | D1 |
| **E1** | Add ISFR config to roko.toml schema | roko-core | ~80 | — |
| **E2** | Add `roko isfr` CLI subcommand | roko-cli | ~120 | C1, C2, E1 |
| **F1** | Add ISFR REST API to roko-serve | roko-serve | ~180 | C2, E1, E2 |
| **F2** | Add ISFR SSE stream to roko-serve | roko-serve | ~60 | F1 |
| **F3** | Add ISFR data slice to DataHub + transport types | demo-app (TS) | ~200 | F1, F2 |
| **F4** | Create ISFR Dashboard page | demo-app (TS) | ~250 | F3 |
| **F5** | Add ISFR route + navigation entry | demo-app (TS) | ~300 | F3, F4 |

**Total: ~2,700 lines across 18 tasks**

## Parallel Execution Plan

**Wave 1** (no dependencies — start immediately):
- A1, C1, D1, E1

**Wave 2** (after Wave 1):
- A2, A6 (need A1)
- C2 (needs C1)
- D2 (needs D1)

**Wave 3** (after Wave 2):
- A3 (needs A1, A2)
- C3 (needs C1, C2)
- E2 (needs C1, C2, E1)

**Wave 4** (after Wave 3):
- A4, A5 (need A2, A3)
- B1 (needs A6)

**Wave 5** (after Wave 4 — UI layer):
- F1 (needs E2 for keeper state access)
- F2 (needs F1)

**Wave 6** (after Wave 5 — frontend):
- F3 (needs F1+F2 endpoints for DataHub wiring)
- F4, F5 (need F3 data slice + selectors)

## Before Starting: Common Pitfalls

1. **Always verify Cargo.toml deps before writing code**. Several crates (roko-chain, agent-relay)
   may be missing deps listed as "already present" in task files. Run the pre-check commands.
2. **Serde format**: The relay protocol uses **internally-tagged JSON** (`{"type":"subscribe",...}`),
   NOT externally-tagged (`{"Subscribe":{...}}`). All test assertions must use this format.
3. **Variable names**: In `handle_agent_frame()`, the outbound sender is `outbound_tx` (not `tx`).
4. **Config access**: `load_roko_config()` on AppState is **sync** (returns `Arc<RokoConfig>`).
5. **EventBus**: The method is `event_bus.publish(event)`, NOT `.emit(event)`.
6. **ToolError**: Has no `NotFound` variant — use `ToolError::Other(msg)`.
7. **TileCanvas**: Uses if/else-if chain, NOT switch statement.
8. **App.tsx**: Uses if/else-if chain for pane rendering, NOT switch.
9. **RelayOutboundFrame**: Must remove `Eq` derive when adding `serde_json::Value` payload field.
10. **tokio features**: roko-chain's tokio dep may only have `["sync"]` — C2 needs `["sync","time"]`.
11. **Relay topic naming**: Use `"isfr:rates"` (no `feed:` prefix). C2 keeper must publish to
    `"isfr:rates"` so B1's `relay_topics()` subscription matches.
12. **CompositeRate is flat**: Backend serializes as `{"composite_bps":580,"lending_bps":620,...}`.
    Frontend `IsfrRate` type must match this shape (flat fields, NOT nested `components` object).

## Key Design Decisions

1. **ChainConfig already exists in roko-core** — D1 creates ChainProfile as a RUNTIME
   resolution layer on top of it; E1 adds a `profile` field to the existing ChainConfig.
   Do NOT create a second ChainConfig.
2. **RelayConfig already exists in roko-core** — do not redefine it anywhere. The existing
   `pub relay: RelayConfig` field on RokoConfig is what E2 reads for `config.relay.url`.
3. **ISFRSection and ISFRSourceConfig are new** — added to `chain.rs` by E1, then exported
   from the config module and accessed as `config.isfr` in E2.
4. **ISFRKeeper uses a callback for publishing** — not coupled to a relay client directly.
   E2 sets a logging stub; full relay wiring happens in A6.
5. **MockSource is the only source impl for now** — real sources (Aave, Compound) are Phase 2.
6. **commands/mod.rs has a merge conflict** — when resolving, keep the HEAD section (the
   full module list at lines 3–24) and add `pub mod isfr;` in alphabetical position.
7. **ISFRState in roko-serve** — keeper publishes rates via callback → updates shared state
   in AppState → broadcasts ServerEvent → SSE/WS clients receive automatically.
8. **demo-app calls roko-serve directly** — plain React SPA (Vite + React 19 + Zustand).
   REST via `api` singleton, SSE via `SseAdapter`, WebSocket via `WsAdapter`. No Tauri layer.
9. **DataHub pattern** — REST fetch on mount → 30s poll fallback → SSE event triggers
   `useDebouncedRefetch`. Ring buffer for history (max 256). See CostDashboard for reference.
10. **Relay topic viewer** — connects to relay events WebSocket for raw pub/sub message
    inspection. Only active when the "Relay" tab is open (lazy connection).

## Existing Code That Must NOT Be Duplicated

```bash
# ChainConfig and RelayConfig already exist in roko-core:
grep -rn "struct ChainConfig\|struct RelayConfig" \
  /Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/ --include="*.rs"

# roko-chain already has isfr.rs (the registry/protocol layer) and pub use isfr::{...}:
grep -n "isfr" /Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/lib.rs

# roko-cli already has roko-chain, tokio-util, uuid in Cargo.toml:
grep -n "roko-chain\|tokio-util\|uuid" \
  /Users/will/dev/nunchi/roko/roko/crates/roko-cli/Cargo.toml

# Bus trait and PulseBus already exist in roko-core (in-process, NOT the relay TopicBus):
grep -rn "trait Bus\|struct PulseBus" \
  /Users/will/dev/nunchi/roko/roko/crates/roko-core/src/ --include="*.rs"
```

## Existing Code That Must NOT Be Duplicated (demo-app)

```bash
# Existing DataHub store (add ISFR slice here, DON'T create a new store):
grep -n "interface DataHub" demo/demo-app/src/app/DataHub.ts

# Existing selectors (add ISFR hooks here):
grep -n "export const use" demo/demo-app/src/data/selectors.ts | head -10

# Existing ServerEvent union (add ISFR variants here):
grep -n "ServerEvent" demo/demo-app/src/transport/types.ts | head -5

# Existing api singleton (use this, DON'T create a new one):
grep -n "export.*api" demo/demo-app/src/transport/api.ts | head -5

# Existing SSE known event types (add ISFR types here):
grep -n "KNOWN_SSE" demo/demo-app/src/transport/sse.ts

# Existing dashboard pattern (follow CostDashboard):
grep -rn "useDebouncedRefetch\|useCountUp" demo/demo-app/src/ --include="*.tsx" | head -5
```

## Validation After All Tasks Complete

```bash
# ─── Backend (roko workspace) ─────────────────────────────────
cargo build --workspace
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace

# Verify CLI commands work
cargo run -p roko-cli -- isfr status
cargo run -p roko-cli -- isfr sources

# Start relay with pub/sub (separate terminal)
cargo run -p agent-relay -- --rpc-ws-url ws://localhost:8545

# Start keeper
cargo run -p roko-cli -- isfr start

# Verify API endpoints
curl http://localhost:6677/api/isfr/status
curl http://localhost:6677/api/isfr/current
curl http://localhost:6677/api/isfr/sources

# ─── Frontend (demo-app) ──────────────────────────────────────
cd demo/demo-app

# Type-check
npx tsc --noEmit

# Dev mode verification:
npm run dev
# → Navigate to /dashboard/isfr → page renders with 4 tabs
# → Overview tab shows live rates when keeper is running
# → Sources tab lists source health
# → Relay tab connects when relay is up
# → Keyboard shortcut: g i → navigates to ISFR page
```
