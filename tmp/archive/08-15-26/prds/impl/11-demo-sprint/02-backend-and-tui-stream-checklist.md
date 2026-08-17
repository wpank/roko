# Demo Streams B And C: Backend And TUI Checklist

## Scope

Use this file for the Roko repo stream: jobs backend, state/event wiring, TUI tabs/subviews, and visible operator feedback.

## Checklist

- [ ] Add a typed jobs model only if it is backed by durable storage and real route handlers.
- [ ] Add serve routes for job list/detail/create/state transitions and emit matching server events.
- [ ] Keep state-machine transitions explicit and test-covered.
- [ ] Reuse existing websocket/event infrastructure in `roko-serve`.
- [ ] Wire the TUI marketplace/atelier tabs to the real backend data or durable local store.
- [ ] Finish the subviews already enumerated in TUI code before adding new conceptual ones.
- [ ] Ensure visible demo metrics exist.
  - heartbeats;
  - plan/task progress;
  - agent status;
  - cost or provider health where available.

## Agent-ready task sequence

1. `DEMO-BT-01` Typed jobs model and durable state
   - Scope: introduce the jobs model only with durable storage and explicit state transitions.
   - Touches: backend job types, durable storage under `.roko/`, route handlers.
   - Deliverable: one durable jobs model that survives process restarts.
   - Done when: job state is persisted and reloadable without manual reconstruction.

2. `DEMO-BT-02` Route and websocket lifecycle parity
   - Scope: add route coverage for list/detail/create/update and emit matching server events for the same transitions.
   - Touches: `crates/roko-serve/src/routes/`, `crates/roko-serve/src/events.rs`.
   - Deliverable: one job lifecycle contract available through both HTTP and websocket streams.
   - Depends on: `DEMO-BT-01`.
   - Done when: route responses and stream events describe the same state machine.

3. `DEMO-BT-03` Marketplace and Atelier backend wiring
   - Scope: bind the existing TUI marketplace and atelier tabs to real backend data or the same durable local store used by the backend.
   - Touches: `crates/roko-cli/src/tui/`, `crates/roko-cli/src/orchestrate.rs`.
   - Deliverable: one end-to-end operator flow from backend state into TUI rendering.
   - Depends on: `DEMO-BT-02`.
   - Done when: TUI surfaces render backend state changes without manual file edits.

4. `DEMO-BT-04` Demo-visible telemetry and progress signals
   - Scope: surface heartbeats, task progress, agent status, and provider-health or cost signals where available.
   - Touches: backend event payloads, TUI panels/widgets, operator telemetry rendering.
   - Deliverable: one visible telemetry layer suitable for live coding/demo operation.
   - Depends on: `DEMO-BT-03`.
   - Done when: an operator can watch real progress and health signals while work is running.

## Verification checklist

- [ ] Job routes can be exercised from curl or a small integration test.
- [ ] Matching server events appear on the websocket stream.
- [ ] TUI renders updated job or plan state without manual file surgery.

## Relevant files

- `crates/roko-serve/src/routes/`
- `crates/roko-serve/src/events.rs`
- `crates/roko-cli/src/tui/`
- `crates/roko-cli/src/orchestrate.rs`

## Acceptance criteria

- Creating or updating a job changes durable backend state.
- TUI surfaces can see and render the same state changes.
- Demo-visible progress comes from real backend events, not timers pretending to be execution.
