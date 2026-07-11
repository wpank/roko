# Agent ID contract drops task and output

- Severity: critical
- Status: code-confirmed and reproduced
- Area: runner-to-dashboard event reduction

## Observation

Runner-v2 creates agent IDs in `plan/task` form (`crates/roko-cli/src/runner/event_loop.rs:4994`). The dashboard reducer associates an agent with a task only when its ID begins with `plan:` (`crates/roko-core/src/dashboard_snapshot.rs:952-958`). The live registry contained three PIDs, but the TUI showed one row with task `-`.

Agent output is retained only when `current_task` is non-empty (`dashboard_snapshot.rs:1046-1058`). The failed identity match therefore also explains the blank Output pane even while Codex subprocesses are active.

## Expected

Agent identity must use one canonical structured plan/task key. Output and usage events should carry explicit plan and task fields rather than parsing an opaque display ID.

