# Wave B: Dead Code Wiring

## Root Cause

40% of 46 shipped batches created code with zero runtime callers. These are capabilities
the system was supposed to have (configurable timeouts, adaptive budgets, typed paths)
that were implemented but never connected.

**Key audit finding**: Most wiring is SIMPLER than originally estimated because:
- `RunConfig` already holds `Arc<RokoConfig>` — config fields are already accessible
- Path decision superseded by taskrunner on 2026-05-05: `Workspace` (roko-core) is the
  public workspace path boundary; `RokoLayout` (roko-fs) remains a lower-level layout catalog
  during migration
- Gate pipeline already has `Verify` trait + `ComposedGatePipeline` — use it
- No new parameter threading needed for most config access

---

## Task B1: Wire TimeoutConfig

**Root cause**: `TimeoutConfig` (9 Duration fields) is defined in roko-core, deserialized
from `[timeouts]` in roko.toml, and is a field on `RokoConfig`. But all timeouts are
hardcoded `Duration::from_secs()`.

**What already exists**:
- `TimeoutConfig` struct with all fields
- Field on `RokoConfig`
- `RokoConfig` is in `RunConfig` as `Arc<RokoConfig>` (field `roko_config`)

**What to do** (simpler than originally scoped):
1. Add accessor methods to `TimeoutConfig`:
   ```rust
   impl TimeoutConfig {
       pub fn agent_dispatch(&self) -> Duration { self.agent_dispatch }
       pub fn gate_compile(&self) -> Duration { self.gate_compile }
       // etc.
   }
   ```
2. In call sites, replace `Duration::from_secs(300)` with `config.timeouts.gate_compile()`
3. **NO new parameter threading** — `RunConfig.roko_config` already provides access

**Target call sites** (~12):
- `crates/roko-agent/src/dispatcher/` (agent call timeout)
- `crates/roko-cli/src/runner/` (task timeout, plan timeout)
- `crates/roko-gate/` (gate execution timeout — compile 600s, clippy 300s, test 900s)
- `crates/roko-serve/src/routes/health.rs` (health check timeout)

**Verification**:
```toml
# roko.toml
[timeouts]
agent_dispatch_secs = 30
```
```bash
cargo run -p roko-cli -- run "complex task" 2>&1 | grep -i timeout
# Should timeout at 30s, not the hardcoded 120s default
```

---

## Task B2: Wire InlineTerminal as Output Pipeline

**Root cause**: `agent_events.rs` has 10 inline `if stream_to_stderr { eprintln!(...) }` blocks.
Meanwhile, `crates/roko-cli/src/inline/` has a FULLY BUILT `InlineTerminal` with 11 primitives
(ToolCallBlock, DiffBlock, CostMeter, etc.) that are never used in production.

**Audit finding**: The original plan had B2 (RunOutputSink) and C5 (Clack-style output) as
separate tasks. They are the SAME system. Skip the intermediate `StderrSink` abstraction
and wire InlineTerminal directly.

**What already exists** (from audit):
- `InlineTerminal` — real ratatui `Viewport::Inline` with `insert_before` for scrollback
- 11 primitives in `inline/primitives/`: ToolCallBlock, DiffBlock, CostMeter, SpinnerBar, etc.
- `ResponseRenderer` trait in inline module

**What to do**:
1. Replace 10 `if stream_to_stderr { eprintln!(...) }` blocks in `agent_events.rs` with
   calls to `InlineTerminal` methods
2. Add `InlineTerminal` (or `Arc<dyn ResponseRenderer>`) to the runner's event handling context
3. Wire primitives: ToolCallBlock for tool calls, CostMeter for running cost, DiffBlock for file changes
4. Construct `InlineTerminal` at runner init when `stream_to_stderr` is true, `NoopRenderer` when false

**Design note**: This replaces BOTH the old B2 (RunOutputSink) AND C5 (Clack-style output).
One task, one system, one abstraction.

**Verification**:
```bash
cargo run -p roko-cli -- plan run plans/test/
# Should show structured ◆/│/└ format with tool calls, costs, diffs
# NOT raw eprintln debug output
```

---

## Task B3: Wire GatePipeline Configuration

**Root cause**: `rung_dispatch.rs` has a hardcoded 7-arm `match rung { Rung::Compile => ... }`
while `GateRungConfig` and `GatesConfig::effective_rungs()` exist but are never called.

**What already exists** (from audit):
- `GateRungConfig` struct (name, command, timeout, required, skip_conditions)
- `GatesConfig::effective_rungs() -> Vec<GateRungConfig>`
- `Verify` trait — the gate plugin interface: `async fn verify(&self, engram: &Engram, ctx: &Context) -> Verdict`
- `ComposedGatePipeline` — Sequential/Parallel/Voting/Fallback composition
- `PlanComplexity` enum: Trivial/Simple/Standard/Complex
- `select_rungs()` mapping PlanComplexity → rung set
- Per-rung adaptive thresholds (EMA in `.roko/learn/gate-thresholds.json`)

**What to do** (use existing plugin system, NOT dual-path):
1. Create a `GatePipelineBuilder` that constructs a `ComposedGatePipeline` from config:
   ```rust
   impl GatePipelineBuilder {
       pub fn from_config(config: &GatesConfig, complexity: PlanComplexity) -> ComposedGatePipeline {
           if config.has_custom_rungs() {
               // Build from [[gates.rungs]] TOML entries
               Self::from_custom_config(config.effective_rungs())
           } else {
               // Use default select_rungs(complexity)
               Self::from_defaults(complexity)
           }
       }
   }
   ```
2. Each `GateRungConfig` becomes a `Box<dyn Verify>` (either a known Rung or a custom command)
3. Use `GateRungConfig.timeout` to override per-gate hardcoded defaults
4. Wire into `run_canonical_rung()` call site — replace it with pipeline.run()

**Verification**:
```toml
# roko.toml — custom gate config
[[gates.rungs]]
name = "compile"
command = "cargo check --workspace"
timeout_secs = 120
required = true

[[gates.rungs]]
name = "test"
command = "cargo test --workspace"
timeout_secs = 300
required = true

# NOTE: clippy intentionally omitted — should be skipped
```
```bash
cargo run -p roko-cli -- plan run plans/test/
# Confirm: runs compile and test gates, does NOT run clippy
```

---

## Task B4: Wire AdaptiveBudget

**Root cause**: `adaptive_budget_for(role, context_window)` exists but all templates call
`budget_for(role)` which returns static constants (always 8K regardless of model).

**What already exists**:
- `AdaptiveBudget` struct with per-section allocations
- `adaptive_budget_for(role: &str, context_window: usize) -> AdaptiveBudget`
- Tests verifying scaling logic

**What to do**:
1. In `system_prompt_builder.rs`, replace `budget_for(role)` with
   `adaptive_budget_for(role, model_profile.context_window)`
2. Model context window is available from CascadeRouter's model selection
3. Update ~5 call sites in template rendering

**Verification**:
```bash
# Small model — compact prompt
ROKO_MODEL=haiku cargo run -p roko-cli -- run "fix typo" 2>&1 | grep -i "prompt.*tokens"

# Large model — fuller prompt
ROKO_MODEL=opus cargo run -p roko-cli -- run "fix typo" 2>&1 | grep -i "prompt.*tokens"
# Second should report higher token count
```

---

## Task B5: Consolidate Layout Types + Wire

**Root cause**: Two path abstraction types exist:
- `Workspace` (roko-core/src/workspace.rs) — public workspace path boundary with live callers
- `RokoLayout` (roko-fs/src/layout.rs) — 30+ accessors, many existing callers via `for_project()`

Meanwhile, 600+ sites use `workdir.join(".roko/...")` string concatenation.

**Audit finding, revised**: Don't do 600 mechanical edits. Instead:
1. Use `Workspace` as the public boundary for new workspace-bound paths
2. Add missing accessors to `Workspace` before migrating callers
3. Wire into key paths (runner, commands, serve) by subsystem
4. Keep `RokoLayout` for roko-fs internals and documented migration exceptions

**Phase 1** (this task): Wire `Workspace` into runner + serve initialization where it owns
the workspace boundary:
- `runner/event_loop.rs` and `runner/` modules (~20 sites)
- `commands/plan.rs` (~14 sites)
- `serve_runtime.rs` (~10 sites)

**Phase 2** (follow-up): Remaining crates (~300 sites, mechanical)

**Verification**: `grep -rn '\.join(".roko' crates/roko-cli/src/runner/ crates/roko-cli/src/commands/plan.rs crates/roko-cli/src/serve_runtime.rs --include='*.rs' | grep -v target/` → empty

---

## Task B6: Wire SafetyLayer on All Backends

**Root cause**: `ToolDispatcher` requires `SafetyLayer`. But `ExecAgent`, `GeminiBackend`,
and `CursorBackend` have their own dispatch paths that bypass it entirely.

**What to do**:
1. Add `SafetyLayer` (non-optional) to `ExecAgent`, `GeminiBackend`, `CursorBackend`
2. Each constructor takes `SafetyLayer` (no default, no option)
3. Pre/post checks run before/after every tool call regardless of backend

**Verification**: Run with GeminiBackend, trigger a dangerous tool call (`rm -rf /`),
confirm it's blocked.

---

## Task B7: Wire validate_against_schema()

**Root cause**: `task_parser.rs:830` has `validate_against_schema()` on `TasksFile`.
Never called from `plan_loader.rs` or any validation path. Schema violations are
silently accepted.

**What to do**:
1. Call `tasks_file.validate_against_schema()` in `plan_loader.rs` after parsing
2. If validation fails, return clear error listing invalid fields
3. Wire into `roko plan validate` command

**Verification**:
```bash
# Invalid task with missing 'id' field
cargo run -p roko-cli -- plan validate plans/broken/
# Should fail with schema validation error mentioning missing 'id'
```

---

## Dependency Graph

```
B1 (TimeoutConfig)    ─── independent
B2 (InlineTerminal)   ─── independent (merges old B2+C5)
B3 (GatePipeline)     ─── independent
B4 (AdaptiveBudget)   ─── independent
B5 (Layout consolidate) ── independent
B6 (SafetyLayer)      ─── independent
B7 (validate_schema)  ─── independent
```

ALL tasks are independent and can run in parallel. Different files, different subsystems.

---

## What This Wave Achieves

After Wave B:
- Gates are configurable via `roko.toml` (GatePipelineBuilder)
- Timeouts are configurable (accessor methods on TimeoutConfig)
- Prompts scale to model capability (adaptive budget)
- Output is structured inline terminal (not raw eprintln)
- Paths are typed (`Workspace` public boundary, `RokoLayout` only for internals/migration)
- Safety is universal (not bypassed on some backends)
- Schema violations are caught at load time

Total new code: ~500-800 lines (mostly replacing inline conditionals with method calls)
