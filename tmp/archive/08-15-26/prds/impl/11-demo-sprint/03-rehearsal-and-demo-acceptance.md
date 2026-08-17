# Demo Rehearsal And Acceptance

## Rehearsal checklist

- [ ] Run the backend locally and confirm health/status routes respond.
- [ ] Run the dashboard locally and confirm websocket/API configuration matches the backend.
- [ ] Verify at least one full research-style flow.
  - create job;
  - assign/start/update;
  - surface results in dashboard and/or TUI.
- [ ] Verify at least one full coding-style flow.
  - create job;
  - show live progress or plan execution state;
  - record and render evaluation/gate outcome.
- [ ] Verify heartbeat and operator telemetry are visible.
- [ ] Capture all known demo fallbacks ahead of time.

## Agent-ready task sequence

1. `DEMO-ACC-01` Backend and surface launch rehearsal
   - Scope: launch backend and relevant surfaces from a clean local environment and verify basic status/health behavior.
   - Touches: local run docs, status routes, dashboard/TUI launch instructions.
   - Deliverable: one reproducible launch-and-smoke-test runbook.
   - Done when: a fresh operator can bring up the backend and at least one surface without hidden setup.

2. `DEMO-ACC-02` Research-flow acceptance
   - Scope: rehearse a full research-style flow from job creation through visible results in a live surface.
   - Touches: job flow docs, dashboard/TUI verification steps, result rendering path.
   - Deliverable: one accepted research walkthrough with evidence of live state changes.
   - Depends on: `DEMO-ACC-01`.
   - Done when: the research path can be executed end to end without hidden mock substitutions.

3. `DEMO-ACC-03` Coding-flow acceptance
   - Scope: rehearse a coding-style job with live progress, plan/task execution state, and visible gate or evaluation output.
   - Touches: coding workflow docs, plan/task telemetry surfaces, gate/evaluation rendering.
   - Deliverable: one accepted coding walkthrough for self-hosting validation.
   - Depends on: `DEMO-ACC-02`.
   - Done when: Roko can demonstrate a credible coding flow inside its own operator surfaces.

4. `DEMO-ACC-04` Fallback and error audit
   - Scope: predeclare all remaining mocks, demo-only fallbacks, and critical console/server errors, then verify they are visible and non-misleading.
   - Touches: fallback notes, error handling docs, rehearsal checklist.
   - Deliverable: one honest demo-risk ledger.
   - Depends on: `DEMO-ACC-03`.
   - Done when: remaining demo shortcuts are clearly marked and no critical hidden failure remains in the rehearsal path.

## Acceptance criteria

- [ ] Landing page explains the system in under two minutes.
- [ ] Marketplace flow is credible end to end.
- [ ] Operator can see live activity in at least one surface while work is running.
- [ ] All remaining mocks are clearly marked and do not pretend to be live backends.
- [ ] No critical console/server errors appear during the rehearsal path.
