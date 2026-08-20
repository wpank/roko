# 35 — CLI Output Redesign

**Priority**: P2 — hundreds of raw `println!`/`eprintln!` calls bypass the structured output layer, producing inconsistent formatting for scripting and CI consumers
**Size**: L (3-5 days)
**Crates**: `crates/roko-cli/` (primary), no other crates
**Depends on**: None

---

## Background

Roko already has a structured output system: `output_format.rs` provides clack-style primitives (`intro`, `step`, `bar`, `success`, `error`, `warning`, etc.), and `cli_output.rs` wraps those primitives in a `CliOutput` struct that respects a `--quiet` flag. The CLI also has a global `--json` flag that signals machine-readable output is desired, and a `--quiet` flag that suppresses non-error output. Both flags are declared on the `Cli` struct in `main.rs` at line 271 (`json: bool`) and line 280 (`quiet: bool`).

Despite this infrastructure, most commands bypass it entirely. A survey of `crates/roko-cli/src/` finds approximately 1,447 raw `println!` and `eprintln!` calls. Commands like `commands/think.rs`, `commands/feed.rs`, and many others use ad-hoc `println!` directly instead of routing through `output_format` or `CliOutput`. The result is inconsistent output: some paths print with color and structure, others emit plain text, and the `--json` flag has no effect on most subcommands even though it is documented.

There is also a second gap: `roko plan run` produces unstructured text during execution. The runner has a `RunOutputSink` trait in `runner/output_sink.rs` with implementations including `FormattedStderrSink` (the default) and `NoopSink`. However, these implementations write to stderr without indicatif-style per-task progress bars, and there is no `MultiProgress` usage in the non-TUI path. The `indicatif` dependency is already present (`crates/roko-cli/Cargo.toml` line 80: `indicatif = "0.17"`) and the `spinner.rs` module uses it for single-bar spinners, but multi-progress for concurrent plan tasks is not wired.

The `--json` flag has no `JsonOutputSink` implementation in the runner: when `--json` is passed, `roko plan run` still emits the same human-readable stderr text. A `JsonReporter` that emits one JSON object per event on stdout is needed for CI and scripting consumers.

## Current State

1. `crates/roko-cli/src/output_format.rs` — the canonical output primitives: `intro`, `step`, `bar`, `note`, `success`, `error`, `warning`, `divider`, `branch`, `end`, `print_gate_result`, `print_agent_table`, `print_replay_timeline`, `print_cost_prediction`, `print_cost_actual`, `print_knowledge_loaded`. These emit ANSI escape codes directly and use `println!` internally.

2. `crates/roko-cli/src/cli_output.rs` — `CliOutput { quiet: bool }` wraps `output_format` primitives and gates each method on `self.quiet`. The `error` method is always shown regardless of quiet. This struct exists but is not used in most command handlers.

3. `crates/roko-cli/src/main.rs` lines 271-284 — the `Cli` struct has `json: bool` and `quiet: bool` fields. The `--quiet` flag is plumbed into a few paths (e.g., `cmd_oneshot_inline` at line 2618, `cmd_unified_chat` at line 2625) but not into most subcommand handlers.

4. `crates/roko-cli/src/runner/output_sink.rs` — the `RunOutputSink` trait (defined at line 54) with methods: `task_started`, `task_completed`, `task_failed`, `agent_started`, `agent_text_delta`, `flush_agent_text`, `tool_call`, `tool_output`, `token_usage`, `agent_turn_completed`, `agent_error`, `gate_result`, `gate_retry`, `warm_cache_started`, `warm_cache_completed`, `diff_block`, `plan_summary`. Existing implementations: `StderrSink`, `FormattedStderrSink`, `FanOutSink`, `AcpProgressSink`, `NoopSink`.

5. `crates/roko-cli/src/spinner.rs` — `cli_spinner(msg)` builds a single indicatif `ProgressBar` spinner; no `MultiProgress` usage anywhere in the non-TUI code path.

6. `crates/roko-cli/Cargo.toml` line 80 — `indicatif = "0.17"` is already a dependency. No `owo-colors` or `console` crate is present; color is done via raw ANSI escape codes in `output_format.rs`.

7. ~1,447 raw `println!`/`eprintln!` calls in `crates/roko-cli/src/` — concentrated in `commands/think.rs`, `commands/feed.rs`, `commands/mod.rs`, and many other handler files. These bypass `CliOutput` entirely.

## Implementation Plan

### Step 1: Audit scope

Run `grep -rn 'println!\|eprintln!' crates/roko-cli/src/ --include='*.rs' | grep -v 'target/'` to produce the full list. Group by file. Prioritize: (a) commands with `--json` flag mentioned in docs or help text, (b) `roko status`, `roko plan list`, `roko agent list` as high-visibility output paths, (c) `commands/think.rs` and `commands/feed.rs` as the heaviest offenders.

### Step 2: Add `JsonOutputSink` to the runner

In `crates/roko-cli/src/runner/output_sink.rs`, add a new struct `JsonOutputSink` that implements `RunOutputSink`. Each event method should serialize a JSON object to stdout using `serde_json::json!` and print it with `println!`. Fields: `type` (event kind), `plan_id`, `task_id`, `timestamp` (RFC3339 from `chrono::Utc::now()`), and event-specific data.

Example for `task_started`:
```rust
fn task_started(&self, plan_id: &str, task_id: &str, role: &str, title: &str, attempt: u32) {
    println!("{}", serde_json::json!({
        "type": "task_started",
        "plan_id": plan_id,
        "task_id": task_id,
        "role": role,
        "title": title,
        "attempt": attempt,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }));
}
```

### Step 3: Wire `--json` into runner sink selection

In `crates/roko-cli/src/commands/plan.rs`, where the `RunOutputSink` is constructed for `roko plan run`, check `cli.json` and select `JsonOutputSink` instead of `FormattedStderrSink`. The `FanOutSink` can combine both if needed.

### Step 4: Add `MultiProgress` plan-level progress to `FormattedStderrSink`

Add a `MultiProgress` field to `FormattedStderrSink` (from indicatif). In `task_started`, create a child `ProgressBar` for the task and store it keyed by `task_id`. In `task_completed`, mark it done with a green checkmark. In `task_failed`, mark it red. Add a top-level bar updated in `plan_summary` or incrementally in `task_completed`.

Use `indicatif::MultiProgress` for concurrent tasks. Use `ProgressStyle` with `{spinner:.cyan} {task_id} {msg} ({elapsed})` for per-task bars.

### Step 5: Migrate high-priority commands to `CliOutput`

For each command handler file, replace direct `println!` calls with `CliOutput` methods, gating on `cli.quiet`. For commands that produce tabular data (`roko agent list`, `roko plan list`), call `output_format::print_agent_table` or similar.

For `--json` in standalone commands (e.g., `roko status --json` documented at `main.rs` line 447), add JSON serialization that writes a serde-serializable struct to stdout and exits. The human path continues to use `output_format` primitives.

### Step 6: Error deduplication in `FormattedStderrSink`

Add a `HashMap<String, u32>` to `FormattedStderrSink` tracking seen error messages. In `agent_error`, check the map: if the message is new, print in full; if repeated, increment the counter and print "(repeated N more times)" only at the end (in `plan_summary`). Use `Mutex<HashMap<String, u32>>` for thread safety since the trait uses `&self`.

### Step 7: Verify tracing routing

In `main.rs`, the tracing subscriber is initialized around line 3280. Confirm that without `--verbose`, tracing output goes only to `.roko/roko.log` (file appender), not to stderr. With `--verbose`, it should also write to stderr. This is already the documented intent; verify it is actually implemented and not accidentally sending tracing to stderr in the default path.

## Acceptance Criteria

1. `cargo test -p roko-cli` passes with no regressions.
2. `roko plan run plans/ --engine runner-v2 --json` produces one JSON object per line on stdout parseable by `jq '.type'`.
3. `roko plan run plans/ --engine runner-v2` shows a per-task indicatif progress bar for each running task.
4. `roko status --json` produces a JSON object with plan/signal counts.
5. `roko plan run --quiet` suppresses all non-error output.
6. Repeated identical agent error messages are collapsed in `plan_summary` output.
7. Running without `--verbose` produces no tracing output on stderr (only in `.roko/roko.log`).

## Verification Checklist

- [ ] `roko plan run plans/ --json 2>/dev/null | jq '.type'` prints one event type per line without parse errors
- [ ] `roko plan run plans/` (no flags) shows per-task progress bars updating in real-time
- [ ] `roko plan run plans/ --quiet 2>&1` prints only error lines
- [ ] `roko status --json | jq .` is valid JSON
- [ ] `roko plan run plans/ 2>&1 | grep 'DEBUG\|TRACE\|INFO'` produces no output in default mode
- [ ] `cargo test -p roko-cli 2>&1 | tail -5` shows all tests passed

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/output_sink.rs` | Add `JsonOutputSink` struct implementing `RunOutputSink`; add `MultiProgress` to `FormattedStderrSink`; add error dedup map |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/plan.rs` | Wire `cli.json` to select `JsonOutputSink` vs `FormattedStderrSink` in `roko plan run` |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/think.rs` | Replace `println!` calls with `CliOutput` methods |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/feed.rs` | Replace `println!`/`eprintln!` calls with `CliOutput` methods |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/mod.rs` | Replace `println!` calls with `CliOutput` methods |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/status.rs` | Add `--json` serialization path |
