# 108 — TUI Live Feedback and Plan Run Performance Gaps

**Priority**: P1 — makes plan execution feel broken even when it is working; zero user feedback during 2.5+ minute API calls
**Size**: L (4-6 days)
**Crates**: `roko-cli` (`/Users/will/dev/nunchi/roko/roko/crates/roko-cli/`), `roko-gate` (`/Users/will/dev/nunchi/roko/roko/crates/roko-gate/`)
**Depends on**: None (109 covers the deeper architectural fix; this covers practical quick wins)

---

## Background

Running `roko plan run` with the inline TUI (`--approval`) or watching from the standalone
`roko dashboard` should give the user continuous feedback about what the system is doing. In
practice, during dogfood testing with a z.ai/GLM-5.1 provider, the TUI appeared completely
frozen for 2.5 minutes during the API call, then showed "gating" for 20+ minutes while
`cargo check/test/clippy` ran on the full 35-crate workspace for a task that only wrote a
markdown file.

The user cannot distinguish between "the system is working slowly" and "the system is hung or
the API key is wrong." All three cases present identically: blank TUI, 0 tokens, no output, no
errors.

This backlog item covers six specific, actionable gaps identified during that session. It
focuses on quick wins and targeted fixes rather than the broader architectural IPC work (covered
in item 109).

## Current State

### Gap 1: Standalone `roko dashboard` is not connected to a running `plan run` process

**Files:** `crates/roko-cli/src/tui/app.rs` lines 535-548

```rust
pub fn new(root: impl AsRef<Path>) -> Self {
    let state_hub = crate::state_hub::SharedStateHub::new_in_process();
    let _ = state_hub.bootstrap_from_workdir(root.as_ref());
    let events_path = root.as_ref().join(".roko").join("events.jsonl");
    let count = state_hub.replay_log_into_snapshot(&events_path);
```

`App::new()` creates a fresh in-process `SharedStateHub` and bootstraps from disk files. It has
no IPC connection to a running `plan run` process. The connected path (used when `--approval` is
passed to `plan run`) receives events via a `watch::channel` in milliseconds. The standalone
`roko dashboard` path falls back to polling `events.jsonl` with a 200ms debounce (defined in
`crates/roko-cli/src/tui/fs_watch.rs` line 19: `const DEBOUNCE_WINDOW: Duration =
Duration::from_millis(200)`).

Result: users who run `roko dashboard` in a second terminal see "no agent output yet" even when
the agent completed 2.5 minutes ago (if the files haven't been flushed to disk yet).

### Gap 2: OpenAI-compatible provider does not stream output

`CodexAgent` (used for OpenAI-compat endpoints like z.ai) calls `agent.run()` synchronously,
which makes a blocking HTTP POST and waits for the full response. It does NOT implement
`run_streaming()`. The streaming path exists in `crates/roko-cli/src/dispatch_v2.rs` at line
1320 (`run_agent_streaming`), but the event loop calls `spawn_shared_agent_bridge` in
`crates/roko-cli/src/dispatch/factory.rs` line 317, which calls
`run_agent_result_bridge_with_tools_and_cli_mcp` → `agent.run()` (the batching path, line
1495).

For a 2.5-minute API call, zero streaming events are emitted. The TUI shows nothing.

### Gap 3: Gate pipeline runs full workspace compile for markdown-only tasks

When a task with tier `"focused"` completes, `gate_plan_complexity_for_task` in
`crates/roko-cli/src/runner/event_loop.rs` at line 8529-8536 maps it to
`PlanComplexity::Simple`, which selects rungs `[Compile, Lint]` per `rung_selector.rs` line 237:

```rust
PlanComplexity::Simple => &[Rung::Compile, Rung::Lint],
```

`Compile` means `cargo check` on the full 35-crate workspace in an isolated worktree with a
cold `target/` cache. For a task that only writes `discovery.md`, this takes 20+ minutes. The
plan's own `roko.toml` can define custom gates (`[[gates.rungs]]`), but these may stack with
workspace gates rather than replace them.

### Gap 4: TUI shows stale/missing data across all tabs during long API calls

During the dogfood session, the following was observed in the inline TUI (with `--approval`):

| Tab | Displayed | Should show |
|-----|-----------|-------------|
| F1 Agents | "no agent output yet" | "agent running (2m30s elapsed)" |
| F1 Output | "waiting for agent output" | At minimum: spinner with elapsed time |
| F1 Efficiency | "tokens 0 cost $0.00 succ 0%" | "in progress, model: glm-5.1" |
| F3 Agents | model "unknown", tokens "-" | "glm-5.1", running cost |

The model name ("unknown") is not populated until after the agent call completes, because the
model slug is returned from the provider response rather than being set at dispatch time.

### Gap 5: No indication of API call in progress

Between the "Spawned" log entry and the eventual result, there are zero log entries, zero TUI
updates, zero progress indicators. The system appears frozen. There is no periodic heartbeat or
"still waiting..." log message. For an API call that takes 2.5 minutes with no streaming, a user
has no way to know if the call is in progress or if the connection dropped.

### Gap 6: Token/cost reporting shows $0.00 for OpenAI-compat providers

All episodes from z.ai show `cost_usd: 0.0` because (a) cost-per-token is not configured in the
model profile for z.ai/glm-5.1, and (b) the OpenAI-compat adapter doesn't compute cost from
usage × configured rates. The TUI always shows "$0.00" for this provider.

## Implementation Plan

### Fix 1: Document and default `--approval` / `--tui` for interactive terminals

(Covered partially in item 107.) The quickest fix is to default `approval = true` when stdout is
a TTY. In `crates/roko-cli/src/commands/plan.rs`, in the `PlanCmd::Run` handler (around line
713 where `if approval {` is checked):

```rust
// Auto-enable inline TUI when stdout is interactive
let approval = approval || (!cli.quiet && !cli.json && std::io::stdout().is_terminal());
```

Add `use std::io::IsTerminal;` at the top. Add a `--no-tui` flag to `PlanCmd::Run` in
`main.rs` so users can opt out:

```rust
/// Disable the inline TUI even in interactive terminals.
#[arg(long)]
no_tui: bool,
```

Then: `let approval = approval || (stdout_is_tty && !no_tui && !cli.quiet && !cli.json);`

### Fix 2: Emit elapsed-time events even when no tokens arrive

In `crates/roko-cli/src/runner/event_loop.rs`, in the agent event processing loop, add a
periodic heartbeat: after the agent task is spawned, spawn a sibling task that sends
`AgentEvent::Heartbeat` (or equivalent) every 10-30 seconds:

```rust
// After spawning the agent bridge:
let heartbeat_tx = routed_tx.clone();
let heartbeat_attempt = attempt.clone();
let heartbeat_handle = tokio::spawn(async move {
    let start = std::time::Instant::now();
    loop {
        tokio::time::sleep(Duration::from_secs(15)).await;
        let elapsed_ms = start.elapsed().as_millis() as u64;
        let event = RoutedAgentEvent::heartbeat(heartbeat_attempt.clone(), elapsed_ms);
        if heartbeat_tx.send(event).await.is_err() { break; }
    }
});
// Cancel the heartbeat when the agent completes
```

In the TUI, the `AgentCard` should display elapsed time using the `started_at` timestamp even
when no token events have arrived. Check `crates/roko-cli/src/tui/state.rs` for the agent card
rendering logic and add `elapsed_secs` field derived from `started_at`.

### Fix 3: Show resolved model name immediately at dispatch time

In `crates/roko-cli/src/dispatch/factory.rs`, when `spawn_shared_agent_bridge` selects a
provider and model (before the blocking `agent.run()` call), emit a
`DashboardEvent::AgentModelResolved { agent_id, model }` event through the TUI bridge so the
model name appears in the TUI immediately, not after completion.

Look for where `AgentRuntimeEvent::Started` is emitted in
`crates/roko-cli/src/dispatch_v2.rs` line 1328-1335 — this event already fires at dispatch
time, but check whether the `model` field is populated correctly:

```rust
let _ = event_tx.send(AgentRuntimeEvent::Started {
    agent_id: request.agent_id.clone(),
    provider: created.target.provider_id.clone(),
    model: created.target.model_slug.clone(),   // ← is this set correctly?
    pid: None,
}).await;
```

If `model_slug` is empty at this point, trace back to where `created.target` is set in
`create_agent()` and ensure the model slug is resolved from config before the HTTP call.

### Fix 4: Skip cargo gates for tasks with no `.rs` file changes

In `crates/roko-cli/src/runner/event_loop.rs`, in `gate_plan_complexity_for_task` (line 8529),
add a changed-files check before selecting the complexity:

```rust
fn gate_plan_complexity_for_task(task_def: Option<&TaskDef>, changed_files: Option<&[PathBuf]>) -> PlanComplexity {
    // If we have file change information and no .rs files changed, downgrade to Trivial
    if let Some(files) = changed_files {
        let has_rust_changes = files.iter().any(|f| f.extension().is_some_and(|e| e == "rs"));
        if !has_rust_changes {
            return PlanComplexity::Trivial;  // Trivial = Compile only, or even skip
        }
    }
    match task_def.map(|task| task.tier.as_str()).unwrap_or("focused") {
        "mechanical" | "fast" => PlanComplexity::Trivial,
        "focused" => PlanComplexity::Simple,
        // ...
    }
}
```

This requires the caller to pass `changed_files`. Check how gate dispatch is invoked (line 9384
and 10763) and pass the list of files changed by the agent's worktree diff. Alternatively,
detect `.rs` changes at gate invocation time using `git diff --name-only` in the worktree.

If detecting file changes is too complex for a first pass, add a plan-level config option:

```toml
# In tasks.toml or plan's roko.toml:
[gates]
skip_if_no_rust_changes = true
```

And add a `--gate-scope=auto` flag to `plan run` that applies this heuristic.

### Fix 5: Add `cost_input_per_m` and `cost_output_per_m` to the OpenAI-compat model profile

This is a config/documentation fix, not a code fix. In the operator's `roko.toml`, add cost
rates to the `[models.glm51]` profile (or whichever OpenAI-compat model is in use):

```toml
[models.glm51]
provider = "openai_compat"
slug = "glm-5.1-air"
cost_input_per_m = 0.14    # example rates in USD per 1M tokens
cost_output_per_m = 0.14
```

In `crates/roko-cli/src/dispatch_v2.rs`, `fill_cost_from_profile` (line 1497) already computes
cost from usage × configured rates if the rates are set. The fix is ensuring the example
`roko.toml` and docs include these fields for OpenAI-compat models.

Additionally, in the TUI efficiency panel, when cost is $0.00 and the model profile lacks cost
rates, show "cost: unknown" instead of "$0.00":

```rust
// In tui/state.rs cost rendering:
if cost_usd == 0.0 && !has_cost_config {
    "cost: unknown".to_string()
} else {
    format!("${cost_usd:.4}")
}
```

### Fix 6: Show gate progress in the TUI (rung name + elapsed time)

In the gate dispatch path, emit a `DashboardEvent::GateRungStarted { rung_name, task_id }` event
when each rung begins. In `crates/roko-cli/src/runner/gate_dispatch.rs`, before executing each
rung, send this event through the TUI bridge. In the TUI, display "Gating: cargo check (3m12s)"
instead of just "gating" on the task status card.

Find the gate rung execution loop in `gate_dispatch.rs` (the `build_rung_execution_inputs`
function referenced in CLAUDE.md and the rung loop that calls into `roko-gate`). Before each
rung call, emit an event.

## Acceptance Criteria

1. In an interactive terminal, `roko plan run plans/` (without `--approval`) shows the inline TUI automatically.
2. During a long API call (>15 seconds), the TUI shows "agent running (Xs elapsed)" even when no token events have arrived.
3. The model name (e.g., "glm-5.1") appears in the F3 Agents tab immediately after dispatch, not after the API call completes.
4. A plan where the task only writes a `.md` file does NOT trigger `cargo check/test/clippy` gates.
5. OpenAI-compat model profiles in example `roko.toml` include `cost_input_per_m` and `cost_output_per_m` fields.
6. The F5 Logs tab shows "Gating: [rung-name] (Xs elapsed)" during gate execution.
7. `cargo test -p roko-cli -p roko-gate` passes.

## Verification Checklist

- [ ] `cargo run -p roko-cli --bin roko -- plan run plans/demo-multistage --engine runner-v2 --fresh` (no `--approval`) — verify TUI appears automatically in interactive terminal
- [ ] During a long z.ai call, verify TUI shows elapsed time counter updating every ~15 seconds
- [ ] Run a plan with a markdown-only task — verify gate does NOT run `cargo check`
- [ ] `curl -s localhost:6677/api/managed-agents | jq .[0].model` — returns model name, not "unknown", immediately after dispatch
- [ ] `cargo test -p roko-cli 2>&1 | tail -5` — all tests pass

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/src/commands/plan.rs` | Auto-enable inline TUI when stdout is a TTY (around line 713); add `--no-tui` flag support |
| `crates/roko-cli/src/main.rs` | Add `no_tui: bool` flag to `PlanCmd::Run` |
| `crates/roko-cli/src/runner/event_loop.rs` | Add periodic heartbeat task after agent bridge spawn; update `gate_plan_complexity_for_task` (line 8529) to skip cargo gates for non-Rust changes |
| `crates/roko-cli/src/dispatch_v2.rs` | Verify `model_slug` is set correctly in `AgentRuntimeEvent::Started` (line 1328); ensure cost is computed from profile rates |
| `crates/roko-cli/src/runner/gate_dispatch.rs` | Emit `DashboardEvent::GateRungStarted` before each gate rung execution |
| `crates/roko-cli/src/tui/state.rs` | Show "cost: unknown" when cost is 0.0 and no cost config; show elapsed time on agent cards even with no token events |
| `roko.toml` (example / docs) | Add `cost_input_per_m` and `cost_output_per_m` to OpenAI-compat model profiles |
