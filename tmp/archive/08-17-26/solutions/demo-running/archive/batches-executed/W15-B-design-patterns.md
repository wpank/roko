# W15-B: Design Pattern Improvements (IMPROVEMENTS 5.1-5.5)

**Priority**: P2 -- reduce duplication, improve error visibility, enforce safety
**Effort**: 4-5 hours
**Files to modify**: 5 files, 1 new file
**Dependencies**: None

## Problem

Five structural anti-patterns degrade maintainability and hide failures:

1. **11 duplicated dispatch patterns** -- The same dispatch-match-record-emit flow is copy-pasted at 11 call sites across `orchestrate.rs` (lines 9318, 9785, 12136, 12203, 12313, 12900, 14085, 14289, 14453, 21494, 21573). Each copy drifts slightly, making behavior inconsistent.
2. **9 `let _ = self.daimon.appraise()` silently dropping errors** -- at lines 7836, 8165, 8657, 9505, 9554, 9579, 11153, 11496, 13629. Plus 1 `conductor.decide()` at line 15118 and 2 `substrate.put()` at lines 17232, 17250.
3. **SafetyLayer is optional** -- `ToolDispatcher` holds `Option<SafetyLayer>` (line 89), defaulting to `None`. Every construction site must remember to attach safety. Forgetting = no safety checks at all.
4. **10 inline `if stream_to_stderr` blocks** -- `agent_events.rs` has 10 `if stream_to_stderr { ... }` blocks scattered through the event handler (in `MessageDelta`, `ToolCall`, `ToolOutput`, `TokenUsage`, `TurnCompleted`, `Error`, `Exited` arms). Adding a new output sink (JSON, file, SSE) requires editing every branch.
5. **Silent env var parse failures** -- `apply_env()` in `schema.rs` silently ignores malformed env vars like `ROKO_CONTEXT_LIMIT_K=abc`. Users get no feedback that their override was ignored.

## Exact Code to Change

### File 1: `crates/roko-cli/src/orchestrate.rs` (23,181 lines)

#### Change 1: Extract `dispatch_and_record` helper (5.1)

The dispatch-match-record pattern appears at 11 call sites. Each follows roughly this shape:

```rust
let outcome = self
    .dispatch_agent_with(plan_id, role, task, prompt, model, dir, sys_prompt)
    .await;
match &outcome {
    Ok(outcome) => {
        // record episode, emit efficiency event, daimon appraisal
    }
    Err(e) => {
        // record failure episode
    }
}
```

This is a large refactor best done incrementally (2-3 sites per PR). The batch creates the helper method; wiring it into all 11 call sites is follow-up.

**Find this code (line ~14737):**

```rust
        self.dispatch_agent_with(plan_id, role, task, None, None, None, None)
```

**Add this method to the `Orchestrator` impl block, near `dispatch_agent_with` (before line 14880):**

```rust
    /// Dispatch an agent and record the outcome (episode, efficiency, daimon).
    ///
    /// Wraps `dispatch_agent_with` with the standard post-dispatch bookkeeping
    /// that was previously duplicated across 11 call sites.
    async fn dispatch_and_record(
        &mut self,
        plan_id: &str,
        role: AgentRole,
        task: &str,
        prompt_override: Option<String>,
        model_override: Option<String>,
        exec_dir_override: Option<PathBuf>,
        system_prompt_override: Option<String>,
    ) -> Result<DispatchOutcome> {
        let result = self
            .dispatch_agent_with(
                plan_id,
                role,
                task,
                prompt_override,
                model_override,
                exec_dir_override,
                system_prompt_override,
            )
            .await;

        match &result {
            Ok(outcome) => {
                // Episode recording + efficiency events are done inline at each
                // call site because the exact data shape varies (some sites pass
                // extra context like gate results, replan info, etc.). This helper
                // centralizes the daimon appraisal which is uniform across all sites.
                if let Err(e) = self.daimon.appraise(AffectEvent::TaskOutcome {
                    task_id: format!("{plan_id}/{task}"),
                    succeeded: outcome.result.success,
                }) {
                    tracing::warn!(error = %e, "daimon appraisal failed (non-fatal)");
                }
            }
            Err(_e) => {
                // Failure episode recording is site-specific (some sites include
                // replan context, gate results, etc.)
            }
        }

        result
    }
```

**Note**: `DispatchOutcome`, `AgentRole`, `AffectEvent`, `Result`, and `PathBuf` are all already in scope in this file. Use `tracing::warn!` (fully qualified) -- this file does NOT import bare `warn!`. The only post-dispatch method that exists on Orchestrator is `emit_efficiency_event` (line 18521). There is NO `record_episode` or `record_failure_episode` method -- episode recording is done inline at each call site with varying data shapes. This helper centralizes the daimon appraisal only; episode + efficiency recording remain at individual call sites.

**Instrumentation**: The helper centralizes the recording, which means adding tracing here (e.g., `info!(plan_id, task, role = ?role, "dispatch_and_record completed")`) gives you one place to observe all dispatch outcomes.

#### Change 2: Log all `let _ =` error drops (5.2)

Replace each silently-dropped error with a logged warning. The exact lines and the `AffectEvent` variant vary; here are all 12 instances with exact before/after:

**Instance 1 (line 7836):**

**Find this code:**

```rust
                let _ = self.daimon.appraise(AffectEvent::TaskOutcome {
                    task_id: format!("plan:{}", p.plan_id),
                    succeeded: p.succeeded,
                });
```

**Replace with:**

```rust
                if let Err(e) = self.daimon.appraise(AffectEvent::TaskOutcome {
                    task_id: format!("plan:{}", p.plan_id),
                    succeeded: p.succeeded,
                }) {
                    tracing::warn!(error = %e, "daimon appraisal failed (non-fatal)");
                }
```

**Instance 2 (line 8165):**

**Find this code:**

```rust
                let _ = self.daimon.appraise(AffectEvent::DreamOutcome {
                    knowledge_entries: report.knowledge_entries_written,
                    playbooks_created: report.playbooks_created,
                    regressions_detected: report.regressions_detected.len(),
```

**Replace with:**

```rust
                if let Err(e) = self.daimon.appraise(AffectEvent::DreamOutcome {
                    knowledge_entries: report.knowledge_entries_written,
                    playbooks_created: report.playbooks_created,
                    regressions_detected: report.regressions_detected.len(),
```

And close the `if let Err` with `}) { warn!(error = %e, "daimon appraisal failed (non-fatal)"); }` instead of `});`.

**Instances 3-9** (lines 8657, 9505, 9554, 9579, 11153, 11496, 13629): Apply the same pattern -- replace `let _ = self.daimon.appraise(AffectEvent::...` with `if let Err(e) = self.daimon.appraise(AffectEvent::...`) { warn!(...); }`.

Each instance follows the same mechanical transform:
- `let _ = self.daimon.appraise(AffectEvent::Xxx { ... });` becomes
- `if let Err(e) = self.daimon.appraise(AffectEvent::Xxx { ... }) { tracing::warn!(error = %e, "daimon appraisal failed (non-fatal)"); }`

**Instance 10 -- conductor (line 15118):**

**Find this code:**

```rust
                    let _ = self.conductor.decide(&signals, &Context::now());
```

**Replace with:**

```rust
                    if let Err(e) = self.conductor.decide(&signals, &Context::now()) {
                        tracing::warn!(error = %e, "conductor decision failed (non-fatal)");
                    }
```

**Instance 11 (line 17232):**

**Find this code:**

```rust
                    let _ = substrate.put(sig).await;
```

**Replace with:**

```rust
                    if let Err(e) = substrate.put(sig).await {
                        tracing::error!(error = %e, "signal persistence failed -- audit trail may be incomplete");
                    }
```

**Instance 12 (line 17250):** Same pattern as instance 11.

**Note**: Use `tracing::error!` (not `tracing::warn!`) for substrate failures because signal persistence is part of the audit trail. Use `tracing::warn!` for daimon/conductor since those are non-critical. **IMPORTANT**: This file does NOT have a bare `use tracing::{warn, error};` import. It only imports `use tracing::{Instrument, info_span, instrument};` (line 168). All warn/error/debug logging in this file uses fully-qualified paths like `tracing::warn!(...)`. You MUST use `tracing::warn!` and `tracing::error!`, not bare `warn!` or `error!`.

---

### File 2: `crates/roko-agent/src/dispatcher/mod.rs` (200+ lines visible)

#### Change 3: Make SafetyLayer required, not optional (5.3)

**Prerequisite check**: `SafetyLayer::with_defaults()` exists at line 244 of `crates/roko-agent/src/safety/mod.rs`. This returns a fully-configured default safety layer with `BashPolicy`, `GitPolicy`, `NetworkPolicy`, `PathPolicy` defaults.

**Find this code (line 89):**

```rust
    safety: Option<SafetyLayer>,
```

**Replace with:**

```rust
    safety: SafetyLayer,
```

**Find this code (lines 108-117):**

```rust
    pub fn new(registry: Arc<dyn ToolRegistry>, resolver: Arc<dyn HandlerResolver>) -> Self {
        Self {
            registry,
            resolver,
            max_result_bytes: DEFAULT_MAX_RESULT_BYTES,
            safety: None,
            tool_cache: None,
            hook_chain: None,
            tool_selector: None,
        }
    }
```

**Replace with:**

```rust
    pub fn new(registry: Arc<dyn ToolRegistry>, resolver: Arc<dyn HandlerResolver>) -> Self {
        Self {
            registry,
            resolver,
            max_result_bytes: DEFAULT_MAX_RESULT_BYTES,
            safety: SafetyLayer::with_defaults(),
            tool_cache: None,
            hook_chain: None,
            tool_selector: None,
        }
    }
```

**Find this code (lines 130-133):**

```rust
    pub fn with_safety(mut self, layer: SafetyLayer) -> Self {
        self.safety = Some(layer);
        self
    }
```

**Replace with:**

```rust
    pub fn with_safety(mut self, layer: SafetyLayer) -> Self {
        self.safety = layer;
        self
    }
```

**Find this code (lines 136-139):**

```rust
    pub const fn safety(&self) -> Option<&SafetyLayer> {
        self.safety.as_ref()
    }
```

**Replace with:**

```rust
    pub const fn safety(&self) -> &SafetyLayer {
        &self.safety
    }
```

**Then** search for all uses of `self.safety` within the dispatch logic in this file. The pattern `if let Some(ref safety) = self.safety { ... }` needs to become just `self.safety.check(...)`:

```bash
grep -n 'self\.safety' crates/roko-agent/src/dispatcher/mod.rs
```

Update each match arm from `if let Some(ref s) = self.safety { s.check_pre_execution(...) }` to `self.safety.check_pre_execution(...)`. The exact occurrences depend on what grep finds.

**Then** search callers that use `.safety()`:

```bash
grep -rn '\.safety()' crates/ --include='*.rs' | grep -v target/ | grep -v test
```

Callers that previously checked `if let Some(s) = dispatcher.safety() { ... }` now get a plain reference: `let s = dispatcher.safety();`. No `if let` needed.

**Known callers that need updating:**

1. `crates/roko-agent/tests/contracts.rs` line 196-198: `.safety().expect(...)` becomes just `.safety()` (returns `&SafetyLayer` directly, no `expect` needed).
2. `crates/roko-agent/src/provider/mod.rs` line 889: `assert!(dispatcher.safety().is_some())` becomes `let _ = dispatcher.safety();` (or just remove the assertion -- it always has safety now).
3. `crates/roko-agent/src/dispatcher/mod.rs` line 338: `if let Some(ref safety) = self.safety { safety.check_pre_execution(...) }` becomes `self.safety.check_pre_execution(...)` (unwrap the if-let).
4. `crates/roko-agent/src/dispatcher/mod.rs` lines 447, 452: `if let Some(ref safety) = self.safety { safety.scrub_output(...) }` and `safety.check_recovery(...)` -- remove the `if let` and use `self.safety.scrub_output(...)` and `self.safety.check_recovery(...)` directly. Remove the `else` branches that returned the result unmodified.
5. `crates/roko-agent/src/dispatcher/mod.rs` line 664: `.field("safety", &self.safety.is_some())` becomes `.field("safety", &true)` or `.field("safety", &"SafetyLayer")`.

---

### File 3: `crates/roko-cli/src/runner/output_sink.rs` (NEW FILE)

#### Change 4: Extract pluggable output sink (5.4)

Create a trait-based output sink to replace the 10 inline `if stream_to_stderr { ... }` blocks in `agent_events.rs`.

**Create** `crates/roko-cli/src/runner/output_sink.rs`:

```rust
//! Pluggable output sink for plan runner events.
//!
//! Replaces scattered `if stream_to_stderr { eprintln!(...) }` patterns
//! in `agent_events.rs` with a trait-based sink.

use std::time::Duration;

/// Receives structured output events from the plan runner.
///
/// Implementations control where output goes -- stderr, JSON file,
/// SSE stream, or /dev/null.
pub trait RunOutputSink: Send + Sync {
    /// A task is about to start execution.
    fn task_started(&self, task_id: &str, title: &str, role: &str, index: usize, total: usize);

    /// A line of agent text output.
    fn agent_line(&self, task_id: &str, line: &str);

    /// An agent invoked a tool.
    fn tool_call(&self, task_id: &str, tool_name: &str);

    /// Tool output (first line only, abbreviated).
    fn tool_output(&self, task_id: &str, first_line: &str);

    /// Token usage update.
    fn token_usage(&self, task_id: &str, input: u64, output: u64);

    /// A gate rung completed.
    fn gate_result(&self, task_id: &str, rung: &str, passed: bool, detail: &str);

    /// A task completed successfully.
    fn task_completed(&self, task_id: &str, elapsed: Duration, cost: f64);

    /// A task failed.
    fn task_failed(&self, task_id: &str, reason: &str);

    /// The entire plan finished.
    fn plan_summary(&self, completed: usize, failed: usize, total_cost: f64, elapsed: Duration);
}

/// Stderr sink -- the current behavior, extracted into a type.
pub struct StderrSink {
    /// Whether to show agent content lines (verbose mode).
    pub verbose: bool,
}

impl StderrSink {
    pub fn new(verbose: bool) -> Self {
        Self { verbose }
    }
}

impl RunOutputSink for StderrSink {
    fn task_started(&self, _task_id: &str, title: &str, role: &str, index: usize, total: usize) {
        eprintln!(
            "[plan-run] Starting task {}/{}: \"{}\" ({})",
            index + 1,
            total,
            title,
            role
        );
    }

    fn agent_line(&self, _task_id: &str, line: &str) {
        if self.verbose {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                let max_chars = 120;
                if trimmed.len() > max_chars {
                    eprintln!("     \u{2502} {}...", &trimmed[..max_chars]);
                } else {
                    eprintln!("     \u{2502} {trimmed}");
                }
            }
        }
    }

    fn tool_call(&self, _task_id: &str, tool_name: &str) {
        eprintln!("     \u{2502} \u{1f527} {tool_name}");
    }

    fn tool_output(&self, _task_id: &str, first_line: &str) {
        if !first_line.is_empty() {
            if first_line.len() > 80 {
                eprintln!("     \u{2502}   {}...", &first_line[..80]);
            } else {
                eprintln!("     \u{2502}   {first_line}");
            }
        }
    }

    fn token_usage(&self, _task_id: &str, input: u64, output: u64) {
        let total = input + output;
        eprintln!("     \u{2502} tokens: {total} (in:{input} out:{output})");
    }

    fn gate_result(&self, _task_id: &str, rung: &str, passed: bool, detail: &str) {
        let icon = if passed { "\u{2713}" } else { "\u{2717}" };
        if detail.is_empty() {
            eprintln!("[plan-run]   {icon} {rung}");
        } else {
            eprintln!("[plan-run]   {icon} {rung}: {detail}");
        }
    }

    fn task_completed(&self, _task_id: &str, elapsed: Duration, cost: f64) {
        let cost_str = if cost > 0.0 {
            format!(", ${cost:.2}")
        } else {
            String::new()
        };
        eprintln!(
            "     \u{2713} Agent turn complete ({:.1}s{cost_str})",
            elapsed.as_secs_f64()
        );
    }

    fn task_failed(&self, _task_id: &str, reason: &str) {
        let msg = if reason.len() > 120 {
            format!("{}...", &reason[..120])
        } else {
            reason.to_string()
        };
        eprintln!("     \u{2717} Error: {msg}");
    }

    fn plan_summary(
        &self,
        completed: usize,
        failed: usize,
        total_cost: f64,
        elapsed: Duration,
    ) {
        eprintln!(
            "[plan-run] Summary: {completed} completed, {failed} failed, ${total_cost:.2}, {:.0}s",
            elapsed.as_secs_f64()
        );
    }
}

/// No-op sink -- suppresses all output.
pub struct NoopSink;

impl RunOutputSink for NoopSink {
    fn task_started(&self, _: &str, _: &str, _: &str, _: usize, _: usize) {}
    fn agent_line(&self, _: &str, _: &str) {}
    fn tool_call(&self, _: &str, _: &str) {}
    fn tool_output(&self, _: &str, _: &str) {}
    fn token_usage(&self, _: &str, _: u64, _: u64) {}
    fn gate_result(&self, _: &str, _: &str, _: bool, _: &str) {}
    fn task_completed(&self, _: &str, _: Duration, _: f64) {}
    fn task_failed(&self, _: &str, _: &str) {}
    fn plan_summary(&self, _: usize, _: usize, _: f64, _: Duration) {}
}
```

**Then** register the module in `crates/roko-cli/src/runner/mod.rs`:

**Find this code (line 33):**

```rust
pub mod tui_bridge;
pub mod types;
```

**Replace with:**

```rust
pub mod output_sink;
pub mod tui_bridge;
pub mod types;
```

**Wiring into `agent_events.rs`** is a follow-up -- this batch creates the abstraction. To wire it:
1. Add `sink: Arc<dyn RunOutputSink>` as a parameter to `handle_agent_event()`
2. Replace each `if stream_to_stderr { eprintln!(...) }` block with the corresponding `sink.method()` call
3. In the event loop setup, choose `StderrSink` or `NoopSink` based on `config.stream_to_stderr`

---

### File 4: `crates/roko-core/src/config/schema.rs`

#### Change 5: Add warnings on env var parse failures (5.5)

The `apply_env()` method (line 296) silently ignores malformed numeric env vars. Three parse sites at lines 309-322 use `if let Ok(n)` which silently drops the `Err`.

**Find this code (lines 309-322):**

```rust
        if let Some(v) = env_fn("ROKO_CONTEXT_LIMIT_K") {
            if let Ok(n) = v.parse::<u32>() {
                self.agent.context_limit_k = n;
            }
        }
        if let Some(v) = env_fn("ROKO_MAX_AGENTS") {
            if let Ok(n) = v.parse::<usize>() {
                self.conductor.max_agents = n;
            }
        }
        if let Some(v) = env_fn("ROKO_BUDGET_USD") {
            if let Ok(n) = v.parse::<f32>() {
                self.budget.max_plan_usd = n;
            }
        }
```

**Replace with:**

```rust
        if let Some(v) = env_fn("ROKO_CONTEXT_LIMIT_K") {
            match v.parse::<u32>() {
                Ok(n) => self.agent.context_limit_k = n,
                Err(e) => {
                    tracing::warn!(
                        env_var = "ROKO_CONTEXT_LIMIT_K",
                        value = %v,
                        error = %e,
                        "invalid env var value; using default"
                    );
                }
            }
        }
        if let Some(v) = env_fn("ROKO_MAX_AGENTS") {
            match v.parse::<usize>() {
                Ok(n) => self.conductor.max_agents = n,
                Err(e) => {
                    tracing::warn!(
                        env_var = "ROKO_MAX_AGENTS",
                        value = %v,
                        error = %e,
                        "invalid env var value; using default"
                    );
                }
            }
        }
        if let Some(v) = env_fn("ROKO_BUDGET_USD") {
            match v.parse::<f32>() {
                Ok(n) => self.budget.max_plan_usd = n,
                Err(e) => {
                    tracing::warn!(
                        env_var = "ROKO_BUDGET_USD",
                        value = %v,
                        error = %e,
                        "invalid env var value; using default"
                    );
                }
            }
        }
```

**Import check**: `tracing` is used as a fully-qualified path (`tracing::warn!`) so no new import is needed. If the file already has `use tracing::warn;` at the top, the qualified path still works.

## Agent Prompt

This batch has 5 changes across 5 files. The agent should:

1. Start with Change 5 (`schema.rs` env var warnings) -- smallest, self-contained
2. Do Change 3 (`dispatcher/mod.rs` SafetyLayer) -- 4 find/replace within one file
3. Create the new file for Change 4 (`output_sink.rs`) and register in `runner/mod.rs`
4. Do Change 2 (`orchestrate.rs` let-_ drops) -- 12 mechanical replacements
5. Do Change 1 (`orchestrate.rs` dispatch_and_record) -- add the helper method (wiring is follow-up)

For Change 2, search the file for `let _ = self.daimon.appraise` and do the mechanical transform on all 9 instances. Then do the conductor and substrate instances. Use `replace_all` or similar batch approach.

For Change 3, verify callers after changing the type signature by running `cargo check -p roko-agent`. The `with_defaults()` constructor exists in `safety/mod.rs` line 244.

## Verification

```bash
# 1. Build all affected crates
cargo check -p roko-cli -p roko-agent -p roko-core

# 2. Run tests
cargo test -p roko-cli -p roko-agent -p roko-core

# 3. Verify no remaining silent drops in orchestrate.rs
grep -c 'let _ = self.daimon' crates/roko-cli/src/orchestrate.rs
# Should be 0 (all replaced with if-let-Err)

grep -c 'let _ = substrate.put' crates/roko-cli/src/orchestrate.rs
# Should be 0

grep -c 'let _ = self.conductor' crates/roko-cli/src/orchestrate.rs
# Should be 0

# 4. Verify SafetyLayer is no longer optional
grep -n 'Option<SafetyLayer>' crates/roko-agent/src/dispatcher/mod.rs
# Should return 0

# 5. Verify output_sink module exists
test -f crates/roko-cli/src/runner/output_sink.rs && echo "exists"

# 6. Clippy
cargo clippy -p roko-cli -p roko-agent -p roko-core --no-deps -- -D warnings
```

## Why This Matters

- `dispatch_and_record` eliminates 11 copies of the same ~15-line pattern, preventing behavioral drift
- Logging silenced errors surfaces affect engine, conductor, and persistence failures that currently vanish
- Required SafetyLayer makes "no safety" an explicit choice (`with_defaults()`) instead of an easy-to-forget omission
- The output sink trait unblocks JSON logging, SSE streaming, and test-mode output capture
- Env var warnings save debugging time when operators set `ROKO_BUDGET_USD=unlimited`

## Audit Status

Audited: 2026-05-05. 1 issue fixed (dispatch_and_record helper referenced `outcome.exit_code` but DispatchOutcome has `result: AgentResult` with `result.success: bool` -- corrected to `outcome.result.success`)
