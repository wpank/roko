# Runtime Migration, Verification, And Cutover

## Scope

Use this file for the transitional API, CLI migration, and integration-test path that keeps the current product working while the runtime is extracted.

## Migration checklist

- [ ] Add a transitional spawn/dispatch API instead of replacing call sites all at once.
  - `spawn_agent()`
  - `dispatch_task()`
  - `spawn_and_run_task()` as a compatibility layer if needed
- [ ] Migrate one production path first.
  - Recommended first path: plan execution from `crates/roko-cli/src/orchestrate.rs`
  - Avoid migrating chat, serve, and dashboard paths in the same first change.
- [ ] Add CLI surface only after the runtime path is real.
  - `roko agent start`
  - persistent chat hookup
  - status/list/stop commands only when agent identity and lifecycle state are durable
- [ ] Wire runtime events into the surfaces that already exist.
  - TUI and serve should consume runtime events through existing event bus or websocket layers.
  - Avoid adding a second incompatible websocket protocol here.
- [ ] Keep rollback simple.
  - one feature flag or one config flag to force legacy dispatch;
  - one integration test proving both paths still work.

## Required integration tests

- [ ] Full lifecycle test.
  - agent spawn;
  - task dispatch;
  - completion/failure;
  - event emission;
  - cleanup.
- [ ] Domain profile test.
  - profile changes tools, gates, or extensions in an observable way.
- [ ] Type-state or lifecycle guard test.
  - invalid transitions fail at compile time or return a documented runtime error.
- [ ] Extension ordering test.
  - confirm extension hook order is stable.
- [ ] Concurrent access test.
  - runtime state updates do not corrupt shared state under parallel execution.

## Build and test commands

- `cargo check --workspace`
- `cargo test -p roko-runtime`
- `cargo test -p roko-cli`
- `cargo test -p roko-orchestrator`
- `cargo test --workspace -- --nocapture` for end-to-end lifecycle failures that only show in integrated logs

## Verification checklist

- [ ] Transitional and legacy dispatch paths both run in automated tests.
- [ ] CLI entrypoints still reach the expected execution path after migration.
- [ ] Event subscribers used by serve/TUI still receive lifecycle updates.
- [ ] Rollback switch or fallback path is documented and tested once.

## Cutover checklist

- [ ] Update architecture docs once the boundary is real.
- [ ] Update CLI help text and examples.
- [ ] Ensure `roko serve` and TUI consumers still receive the data they expect.
- [ ] Remove dead compatibility code only after the new path has test parity.

## Acceptance criteria

- `roko plan run` continues to work during the migration.
- The runtime extraction reduces direct logic inside `orchestrate.rs`.
- At least one non-CLI consumer can subscribe to runtime lifecycle events.
- A rollback path exists until the new runtime handles the primary execution loop cleanly.
