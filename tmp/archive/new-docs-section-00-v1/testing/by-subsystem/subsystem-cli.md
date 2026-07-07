# roko-cli — Test Coverage

> 38 tests for the CLI: PRD lifecycle, interactive dashboard state, and command dispatch.

**Status**: Shipping
**Crate**: `roko-cli`
**Section**: 12 — Interfaces
**Last reviewed**: 2026-04-19

---

## Test Count: 38 (PRD lifecycle)

Source: implementation status audit, 2026-04-17 ("38 tests" under `roko prd`). Additional tests for the dashboard and other commands are not separately counted.

| Module | Approx. tests | Focus |
|---|---|---|
| `prd` | ~38 | PRD creation, editing, planning, idea/draft/plan lifecycle |
| `dashboard` | ~? | State machine transitions (F1-F7 tabs) |
| `commands` | ~? | Command dispatch, argument validation |

---

## Key Test Focus Areas

### PRD Lifecycle (3 stages)

The PRD lifecycle has three stages: `Idea → Draft → Plan`.

Tests verify:
- `roko prd idea <description>` creates a new Idea PRD in the substrate.
- `roko prd draft <id>` promotes an Idea to a Draft with a generated spec.
- `roko prd plan <id>` promotes a Draft to a Plan DAG.
- Invalid transitions (e.g., Draft → Idea) are rejected with a helpful error.
- An Idea/Draft/Plan can be listed, viewed, and deleted.

### Interactive Dashboard (ratatui TUI, F1-F7 tabs)

Tests use a headless terminal emulator to verify state machine transitions:
- F1: Overview tab shows current plan status.
- F2: Agents tab shows active agents and their states.
- F3: Gate tab shows current gate pipeline state.
- F4: Substrate tab shows recent Engrams.
- F5: Learning tab shows bandit arms and scores.
- F6: Logs tab shows event stream.
- F7: Settings tab shows configuration.

Tab transitions and keyboard shortcuts are tested against the state machine, not rendered output.

### CLI Command Dispatch

- `roko plan run` invokes the orchestrator with the current plan.
- `roko plan resume <id>` resumes a paused plan.
- `roko run <task>` dispatches a single-task agent run.
- `roko serve` starts the HTTP control plane.
- Commands that require a running orchestrator return an error when the orchestrator is not running.

---

## Known Gaps

- Dashboard test count is not separately reported; the full tab test suite may be untested.
- `roko serve` integration tests (CLI command → running server) are thin.
- No tests for the research agent commands (`roko research topic`, `roko research enhance`).

## See also

- [subsystem-orchestrator.md](subsystem-orchestrator.md) — `roko plan run` exercises the orchestrator
- [subsystem-serve.md](subsystem-serve.md) — `roko serve` starts the HTTP server
