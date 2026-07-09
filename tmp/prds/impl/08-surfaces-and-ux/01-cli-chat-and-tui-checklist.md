# CLI, Chat, And TUI Checklist

## Scope

Use this file for agent lifecycle CLI commands, persistent chat, and TUI feature work inside the Roko repo.

## Implementation checklist

- [ ] Audit existing CLI coverage first.
  - `roko run`
  - `roko chat`
  - `roko dashboard`
  - `roko serve`
  - existing config/agent-related commands
- [ ] Add agent lifecycle commands only on top of a real runtime identity model.
  - start
  - list
  - stop
  - status
- [ ] Keep persistent chat aligned with the current websocket/event surfaces.
  - use `roko-serve` and the agent-side execution path already present;
  - do not add a second incompatible chat transport.
- [ ] Continue TUI work in the existing modules.
  - `crates/roko-cli/src/tui/tabs.rs`
  - `views/`
  - `widgets/`
  - `state.rs`
  - `dashboard.rs`
- [ ] Treat F8/F9 as real surfaces now that they exist.
  - marketplace flow;
  - atelier/PRD-plan workspace flow;
  - modal/detail parity where appropriate.
- [ ] Add CLI tests or integration tests for any new top-level command.

## Agent-ready task sequence

1. `CLI-TUI-01` CLI and TUI command-surface inventory
   - Scope: audit existing top-level commands, TUI tabs, modal flows, and missing operator actions before adding new entry points.
   - Touches: `crates/roko-cli/src/main.rs`, `crates/roko-cli/src/chat.rs`, `crates/roko-cli/src/tui/`.
   - Deliverable: one command-surface inventory with gaps called out by workflow.
   - Done when: start/list/stop/status, chat, serve, dashboard, marketplace, and atelier flows all have an explicit current-state note.

2. `CLI-TUI-02` Runtime-backed agent lifecycle commands
   - Scope: add or finish `start`, `list`, `stop`, and `status` only against real runtime or backend identity/state.
   - Touches: CLI command registration, orchestrator/runtime integration, status rendering.
   - Deliverable: one lifecycle command set backed by real runtime state.
   - Depends on: `CLI-TUI-01`.
   - Done when: lifecycle commands do not fabricate state and can be exercised against a live local backend.

3. `CLI-TUI-03` Persistent chat transport convergence
   - Scope: keep `roko chat` on the same backend truth and event transport as other live surfaces.
   - Touches: `crates/roko-cli/src/chat.rs`, websocket/event client code, serve chat/event endpoints.
   - Deliverable: one persistent chat path with reconnect and degraded-mode behavior.
   - Depends on: `CLI-TUI-02`.
   - Done when: chat reconnects cleanly or surfaces explicit degraded behavior when streaming is unavailable.

4. `CLI-TUI-04` Marketplace and Atelier TUI completion
   - Scope: finish the existing F8/F9 and tab/subview operator workflows before inventing additional top-level concepts.
   - Touches: `crates/roko-cli/src/tui/tabs.rs`, `views/`, `widgets/`, `state.rs`, `dashboard.rs`.
   - Deliverable: one usable TUI operator path for marketplace and atelier work.
   - Depends on: `CLI-TUI-03`.
   - Done when: an operator can inspect work, move through existing subviews, and complete the intended workflow without dropping to manual file editing.

5. `CLI-TUI-05` CLI fallback and regression coverage
   - Scope: add tests and verification for help output, command behavior, keybindings, and degraded-mode CLI fallback.
   - Touches: CLI tests, integration tests, TUI verification docs.
   - Deliverable: one verification layer for operator-critical CLI/TUI actions.
   - Depends on: `CLI-TUI-04`.
   - Done when: critical operator actions remain possible from CLI even when TUI or web surfaces are unavailable.

## Relevant current files

- `crates/roko-cli/src/main.rs`
- `crates/roko-cli/src/chat.rs`
- `crates/roko-cli/src/tui/tabs.rs`
- `crates/roko-cli/src/tui/views/marketplace_view.rs`
- `crates/roko-cli/src/tui/views/atelier_view.rs`
- `crates/roko-cli/src/tui/dashboard.rs`

## Verification checklist

- [ ] New commands appear in `--help`.
- [ ] TUI keybindings and tab labels remain consistent.
- [ ] Persistent chat can reconnect or degrade cleanly when websocket transport is unavailable.
- [ ] Critical actions remain doable from CLI even if TUI/web are down.

## Acceptance criteria

- Agent lifecycle commands map to real runtime behavior.
- Chat and TUI reuse the same backend truth where possible.
- Surface changes extend the current UX instead of fragmenting it.
