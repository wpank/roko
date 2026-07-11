# Graph Engine Issues

## Critical — Blocks all real execution

### TaskExecutorCell is hardwired to dry-run
- `cells/task_executor.rs:31-34`: `Default::default()` sets `dry_run: true`.
- Live-mode branch (line 81-93): falls through to same dry-run behavior with a warning.
- `default_registry()` at `engine.rs:356-358` registers `TaskExecutorCell::default()`. Factory closure ignores node config.
- `commands/plan.rs:1594`: Also hardcodes `let dry_run_stub = true;`.

### AgentCell exists but never registered
- `cells/agent.rs:127-236`: Complete cell with real dispatch, token counting.
- `default_registry()` at `engine.rs:311-371`: Does NOT register `"agent"` or `AgentCell`.
- `ComposeCell` and `GraduationCell` also exist but registry binds `"compose"` to a `NoopCell` stub (line 347-350).

## High

### No state/resume support
- `GraphEngine` has no snapshot, checkpoint, or resume mechanism.
- `GraphOutput` returned at end but never persisted to disk.
- Interrupted graph runs restart from beginning.

### Cross-plan dependencies silently dropped
- `convert.rs:92-99`: `depends_on_plan` logged as warning and skipped.
- `max_parallel` stored as label but never enforced (engine runs all nodes sequentially).

### HotPolicy.persist_tick_state declared but never implemented
- `hot.rs:40`: Field never read anywhere.
- Hot loop (line 133-215): Creates fresh `CellContext::new()` every tick. Outputs from tick N are discarded.
- `CellContext` has no mechanism to carry prior-tick state.

## Medium

### Edge condition evaluation built but never called
- `condition.rs`: Full `evaluate()` function with `OnSuccess`, `OnFailure`, `When` variants.
- `GraphEngine::execute()`: Only checks `has_failed_ancestor()`. Never reads `edge.condition`.
- Conditional branching silently ignored at runtime.

### All 7 cognitive-loop cells are PassthroughCell stubs
- `cells/stubs.rs:69-77`: `signal-reader`, `relevance-scorer`, `system-prompt-builder`, `claude-agent`, `gate-pipeline`, `store-writer`, `event-publisher` — all return input unchanged.
