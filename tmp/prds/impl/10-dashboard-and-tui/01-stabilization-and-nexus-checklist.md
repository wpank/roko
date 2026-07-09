# Roko Stabilization And Nexus Checklist

## Scope

Use this file for backend stabilization inside the Roko repo: file watchers, websocket/event parity, auth cleanup, persisted state, jobs backend, and the Nexus relay boundary.

## Implementation checklist

- [ ] Audit current backend truth sources.
  - `.roko/` state files;
  - serve routes;
  - websocket stream;
  - TUI direct file reads.
- [ ] Reduce duplicated state paths before adding more UI features.
  - prefer `roko-serve` and explicit state/projection routes;
  - leave file reads only where no API exists yet.
  - include `StateHub` or projection-layer ownership explicitly where `crates/roko-serve/src/routes/projections.rs` is the intended truth surface.
- [ ] Stabilize auth first.
  - middleware boundary in `crates/roko-serve/src/routes/middleware.rs`;
  - CLI auth flow in `crates/roko-cli`;
  - dashboard bearer-token expectations.
- [ ] Add jobs backend only with durable storage and state transitions.
  - typed job model;
  - durable store under `.roko/jobs/` or equivalent;
  - route coverage in `roko-serve`;
  - websocket/server events for lifecycle changes.
- [ ] Define Nexus as a relay boundary, not a hidden second backend.
  - connection/auth model;
  - room/subscription model;
  - aggregate heartbeat outputs;
  - fallback when Nexus is unavailable.

## Agent-ready task sequence

1. `SERVE-BOOT-01` Backend truth-source audit and projection ownership
   - Scope: inventory every state source currently read by CLI, TUI, websocket consumers, and routes, then assign a single owner for each entity.
   - Touches: `.roko/` state layout, `crates/roko-serve/src/routes/status.rs`, `crates/roko-serve/src/routes/plans.rs`, `crates/roko-serve/src/routes/projections.rs`, TUI readers.
   - Deliverable: one backend truth map covering files, projections, routes, and websocket entities.
   - Done when: every operator-visible entity has a documented source of truth and file-read exceptions are explicit.

2. `SERVE-BOOT-02` Auth boundary consolidation
   - Scope: unify middleware, CLI token flow, and dashboard bearer-token expectations around one auth contract.
   - Touches: `crates/roko-serve/src/routes/middleware.rs`, `crates/roko-cli`, dashboard auth client assumptions.
   - Deliverable: one auth contract with clear success/failure behavior.
   - Depends on: `SERVE-BOOT-01`.
   - Done when: unauthenticated, expired-token, and misconfigured-token cases all resolve deterministically across surfaces.

3. `SERVE-BOOT-03` Durable jobs backend and lifecycle model
   - Scope: add a typed jobs model, durable storage, route coverage, and explicit state transitions before any more surface work depends on jobs.
   - Touches: `crates/roko-serve/src/routes/`, durable job storage under `.roko/jobs/` or equivalent, shared job types.
   - Deliverable: durable jobs backend with documented lifecycle states.
   - Depends on: `SERVE-BOOT-01`, `SERVE-BOOT-02`.
   - Done when: create/list/detail/update flows mutate durable state through a documented state machine.

4. `SERVE-BOOT-04` Websocket parity and subscription contract
   - Scope: ensure websocket events, subscription semantics, and route-backed state all describe the same entities and lifecycle changes.
   - Touches: `crates/roko-serve/src/routes/ws.rs`, `crates/roko-serve/src/events.rs`, subscription routes, TUI websocket client.
   - Deliverable: one event contract aligned with route semantics.
   - Depends on: `SERVE-BOOT-03`.
   - Done when: a job or plan transition can be observed equivalently by polling and by streaming.

5. `SERVE-BOOT-05` Nexus relay boundary and degraded-mode fallback
   - Scope: define Nexus as a relay layer with explicit connection, auth, subscription, aggregation, and fallback behavior.
   - Touches: Nexus boundary docs, relay code, `crates/roko-cli/src/tui/ws_client.rs`, dashboard consumers where applicable.
   - Deliverable: one Nexus contract with degraded-mode rules.
   - Depends on: `SERVE-BOOT-04`.
   - Done when: a Nexus outage produces stale-state warnings or fallback behavior instead of silent operator drift.

## Relevant current files

- `crates/roko-serve/src/routes/ws.rs`
- `crates/roko-serve/src/routes/status.rs`
- `crates/roko-serve/src/routes/plans.rs`
- `crates/roko-serve/src/routes/projections.rs`
- `crates/roko-serve/src/routes/middleware.rs`
- `crates/roko-cli/src/tui/fs_watch.rs`
- `crates/roko-cli/src/tui/ws_client.rs`

## Verification checklist

- [ ] Backend routes and websocket events cover the same entities.
- [ ] Auth failure modes are explicit and testable.
- [ ] Jobs can move through a documented state machine.
- [ ] Nexus disconnects degrade to clear stale-state behavior, not silent failure.

## Acceptance criteria

- Backend truth is converging toward API/WS projections.
- Jobs and state persistence are real backend features, not dashboard-only assumptions.
- Nexus is specified as a relay layer with explicit fallback behavior.
