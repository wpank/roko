# 22: Plan Run TUI + Execution Broken

## Symptoms

1. `roko plan run plans/self-dev-ux --engine runner-v2 --approval` exits immediately with no TUI
2. No visible error — stderr is redirected to `.roko/runner-stderr.log`
3. Graph Engine path (`--engine graph`, the default) is a dry-run stub that does nothing

## Root Causes

### Problem 1: Stale executor snapshot blocks startup

**File**: `.roko/runner-stderr.log`
```
error: resume validation failed: plan `P07-autofix-retry` is in snapshot but not in the current run
```

Runner v2 auto-resumes from `.roko/state/executor.json` if it exists. When the snapshot references plans that aren't in the current run, it hard-errors. The TUI thread hasn't initialized yet, so the process exits silently.

The `--fresh` flag only cleaned `executor.json`, `orchestrator.json`, `run-state.json` — but the
actual resume file is `state-snapshot.json` (line 152 of `crates/roko-cli/src/runner/resume.rs`).

**Fix applied**: Added `state-snapshot.json` to the `--fresh` cleanup list in
`crates/roko-cli/src/commands/plan.rs` line 279.

**Additional fixes needed**:
- Make the runner skip/ignore plans in the snapshot that aren't in the current run
- Add `hint: use --fresh to start a new execution` to the resume validation error

**Location**: `crates/roko-cli/src/commands/plan.rs` ~line 279 (--fresh cleanup),
`crates/roko-cli/src/runner/resume.rs:123` (PlanMissing error)

### Problem 2: Graph Engine `TaskExecutorCell` is a dry-run stub

**File**: `crates/roko-graph/src/cells/task_executor.rs`

The default `--engine graph` path uses `TaskExecutorCell` which always returns synthetic success without dispatching any agents:

```rust
if self.dry_run {  // always true
    return Ok(vec![synthetic_engram]);
}
```

Even `dry_run: false` falls back to stub behavior with a warning.

**Fix options**:
- **Option A**: Inject a dispatch callback (`Box<dyn Fn(TaskDef) -> AgentResult>`) into the cell registry. `cmd_plan_run_engine()` provides the callback before execution. ~150 LOC across 3 files.
- **Option B**: Move dispatch into roko-graph by adding `roko-agent` as a dependency. ~200 LOC.
- **Option C**: Make `legacy-runner-v2` a default feature. 1 line in Cargo.toml, gets running immediately.

**Location**:
- `crates/roko-graph/src/cells/task_executor.rs` (the stub)
- `crates/roko-graph/src/engine.rs:356` (registry always uses `TaskExecutorCell::default()`)
- `crates/roko-cli/src/commands/plan.rs:246-254` (graph engine path)

### Problem 3: Errors hidden by stderr redirect

**File**: `crates/roko-cli/src/commands/plan.rs:563-574`

```rust
// Redirect stderr to a log file so runner tracing doesn't corrupt TUI
#[cfg(unix)]
if let Ok(log_file) = std::fs::File::create(&stderr_log_path) {
    unsafe { libc::dup2(log_file.as_raw_fd(), 2); }
}
```

ALL stderr (including fatal errors) goes to `.roko/runner-stderr.log`. User sees nothing. This makes debugging impossible without knowing to check that file.

**Fix**: After runner exits, if exit code != 0, print the last N lines of the log file to stdout. Or only redirect stderr AFTER the TUI is confirmed initialized.

### Problem 4: TUI thread has no startup synchronization

**File**: `crates/roko-cli/src/commands/plan.rs:578-590`

```rust
std::thread::Builder::new()
    .name("roko-plan-approval-tui")
    .spawn(move || { /* TUI init + run */ })
    .context("spawn approval TUI thread")?;
// Thread handle is DISCARDED — no join, no sync
```

The TUI thread starts asynchronously. The main thread immediately continues to the event loop. If the event loop errors (as it does with the stale snapshot), the process exits before the TUI thread enters raw terminal mode.

**Fix**: Add a barrier or channel so the main thread waits for TUI initialization before proceeding to the event loop.

### Problem 5: No `--fresh` flag awareness

The `--fresh` flag exists but the user may not know about it. When auto-resume fails, the error should suggest `--fresh`.

**Fix**: Change the resume validation error to:
```
error: resume validation failed: plan 'P07-autofix-retry' is in snapshot but not in the current run.
hint: use --fresh to start a new execution (deletes old state)
```

## Immediate Workaround

```bash
# Delete stale state, then run
rm -f .roko/state/executor.json
cargo run -p roko-cli --bin roko --features legacy-runner-v2 -- \
  plan run plans/self-dev-ux --engine runner-v2 --approval --fresh
```

## Workaround: Use Graph Engine default (no execution)

```bash
# Shows plan structure but doesn't execute (dry-run stub)
cargo run -p roko-cli --bin roko -- plan run plans/self-dev-ux
```

## Architecture Notes

Two execution engines exist:

| Engine | Flag | Status |
|--------|------|--------|
| Graph Engine | `--engine graph` (default) | Stub — returns synthetic success, no agent dispatch |
| Runner v2 | `--engine runner-v2` | Works but feature-gated behind `legacy-runner-v2` |

To use Runner v2, must pass `--features legacy-runner-v2` to `cargo run`.

The Graph Engine needs `TaskExecutorCell` to be wired to actual agent dispatch (see Problem 2 fix options above). This is tracked in `.roko/GAPS.md` line 8.

## Files Involved

| File | What |
|------|------|
| `crates/roko-graph/src/cells/task_executor.rs` | Dry-run stub cell |
| `crates/roko-graph/src/engine.rs` | Graph engine, cell registry |
| `crates/roko-cli/src/commands/plan.rs:246-590` | Plan run command, TUI spawn, stderr redirect |
| `crates/roko-cli/src/orchestrate.rs` | Runner v2 dispatch (the working path) |
| `crates/roko-cli/src/agent_spawn.rs` | SpawnAgentSpec, create_agent_for_model |
| `.roko/state/executor.json` | Executor snapshot (stale = blocks startup) |
| `.roko/runner-stderr.log` | Hidden error output |
