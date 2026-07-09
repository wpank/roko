# Wiring Audit: Created But Not Called

## Summary
Of 46 "completed" batches (W0-W15), approximately 40% created code that has ZERO runtime callers. This document catalogs each instance and provides wiring instructions.

## Category A: Structs/Traits Created, Never Instantiated

### 1. RunOutputSink (W15-B)
- **Created**: `crates/roko-cli/src/runner/output_sink.rs`
- **What**: Trait `RunOutputSink` + `StderrSink` + `NoopSink` implementations
- **Current callers**: Zero. `agent_events.rs` still uses inline `if stream_to_stderr { eprintln!(...) }` blocks
- **To wire**: Replace all `if stream_to_stderr` blocks in `agent_events.rs` with `sink.emit(event)` calls. Pass `&dyn RunOutputSink` through the event handling chain.
- **Effort**: ~1 hour (mechanical replacement, 8-10 call sites)

### 2. Workspace (W15-E)
- **Created**: `crates/roko-core/src/workspace.rs`
- **What**: `Workspace` struct with typed path accessors (`.plans_dir()`, `.state_dir()`, `.episodes_path()`, etc.)
- **Current callers**: Zero. Exported from roko-core's lib.rs but never imported anywhere else.
- **To wire**: Replace all `workdir.join(".roko/state/")`, `workdir.join(".roko/episodes.jsonl")`, etc. with `workspace.state_dir()`, `workspace.episodes_path()`. Start with runner code (biggest win).
- **Effort**: ~2 hours (many call sites across 6+ crates, but mechanical)

### 3. GateRungConfig + effective_rungs() (W15-E)
- **Created**: `crates/roko-core/src/config/gates.rs`
- **What**: `GateRungConfig` struct and `GatesConfig::effective_rungs()` method that returns data-driven gate pipeline from TOML config
- **Current callers**: Zero. Gate pipeline in `crates/roko-gate/` uses hardcoded rung integer constants.
- **To wire**: In the gate dispatch function, call `config.gates.effective_rungs()` and iterate over the returned configs instead of hardcoded `match rung { 0 => ..., 1 => ..., }` blocks.
- **Effort**: ~2 hours (gate pipeline logic needs restructuring to be data-driven)

### 4. AdaptiveBudget (W15-E)
- **Created**: `crates/roko-compose/src/templates/common.rs`
- **What**: `AdaptiveBudget` struct and `adaptive_budget_for(role, context_window)` that scales token budgets to model's context window
- **Current callers**: Zero. All templates still call `budget_for(role)` which returns static constants.
- **To wire**: In `system_prompt_builder.rs`, replace `budget_for(role)` calls with `adaptive_budget_for(role, model_profile.context_window)`. Need to pass model profile through to the builder.
- **Effort**: ~1 hour (change budget_for → adaptive_budget_for at ~5 call sites, thread context_window through)

### 5. TimeoutConfig (W15-C)
- **Created**: `crates/roko-core/src/config/timeouts.rs`
- **What**: Struct with 9 `Duration` fields (agent_dispatch, gate_compile, gate_test, plan_overall, etc.) + helper methods
- **Current callers**: Struct is IN the `RokoConfig` schema (field `pub timeouts: TimeoutConfig`), but no runtime code reads `config.timeouts.*`. All timeouts are hardcoded constants.
- **To wire**: In dispatcher (roko-agent), replace `Duration::from_secs(120)` with `config.timeouts.agent_dispatch`. In gate pipeline, replace hardcoded timeouts with `config.timeouts.gate_compile`, etc.
- **Effort**: ~1.5 hours (find all hardcoded Duration::from_secs, replace with config access)

## Category B: Functions Created, Never Called

### 6. workspace_context() (W15-A)
- **Created**: `crates/roko-cli/src/orchestrate.rs` line ~1278
- **What**: Function that generates workspace context string (crate layout, key files) for agent prompts
- **Current callers**: Zero in production path. Lives in the LEGACY PlanRunner (orchestrate.rs). The production v2 runner (event_loop.rs) has its own `generate_workspace_map()` which IS called.
- **To wire**: Either (a) delete it (the v2 equivalent already works), or (b) port its logic into the v2 path. Option (a) is correct — this is dead legacy code.
- **Effort**: 0 (just delete it, v2 already has the working equivalent)

### 7. dispatch_and_record() (W15-B)
- **Created**: `crates/roko-cli/src/orchestrate.rs`
- **What**: Helper that centralizes post-dispatch bookkeeping (episode recording, cost tracking, efficiency logging)
- **Current callers**: Unclear — may be called from within orchestrate.rs itself (legacy path only). Not called from v2 runner.
- **To wire**: If v2 runner (event_loop.rs) needs centralized post-dispatch bookkeeping, port this logic there. Otherwise, legacy-only code that will be deleted with orchestrate.rs convergence.
- **Effort**: ~30 min to verify and port, or 0 if deleting legacy path

### 8. ImplementerTemplate wiring (W9-A)
- **Created**: Template struct added to `crates/roko-compose/src/templates/implementer.rs`
- **What**: The template STRUCT exists and renders correctly. The runtime dispatch was supposed to SELECT this template based on task role.
- **Current callers**: Template renders in tests. At dispatch time, system prompt assembly DOES use role-based template selection — but the "implementer" role path may not be exercised because most tasks use "default" role.
- **To wire**: Verify that tasks with `role = "implementer"` actually trigger this template in production runs. May need to add "implementer" to the role→template mapping if missing.
- **Effort**: ~30 min to trace and verify/fix the mapping

## Category C: Partially Wired (Works in One Path Only)

### 9. SafetyLayer required (W15-B)
- **Where**: `crates/roko-agent/src/dispatcher/mod.rs` — changed from `Option<SafetyLayer>` to `SafetyLayer`
- **Problem**: Only the main `ToolDispatcher` was updated. `ExecAgent`, `GeminiProvider`, and other backends still have `Option<SafetyLayer>` or no safety layer at all.
- **To wire**: Apply the same `SafetyLayer` (non-optional) pattern to all agent backends. Each backend should accept a `SafetyLayer` in its constructor.
- **Effort**: ~1 hour (3-4 backends to update)

### 10. Health 503 (W14-B)
- **Where**: `crates/roko-serve/src/routes/health.rs`
- **Problem**: Returns 503 only when ALL providers are fully down. If providers are degraded/slow, still returns 200.
- **To wire**: Add degraded state detection (response time > threshold, error rate > threshold) and return 503 or a custom status for degraded.
- **Effort**: ~1 hour (define degradation thresholds, update health check logic)

### 11. LinUCB State Persistence (W14-C)
- **Where**: `crates/roko-learn/src/cascade_router.rs`
- **Problem**: `LinUCBSnapshot` field added to `CascadeSnapshot` and serializes/deserializes correctly. But the LinUCB algorithm's `update()` method may not be called during production runs (only during the cascade router's internal exploration which requires enough successful dispatches).
- **To wire**: Verify that production plan runs produce enough successful episodes to trigger LinUCB learning. May need to lower the exploration threshold.
- **Effort**: ~30 min to trace and verify

## Category D: Config Schema Added, Never Read

### 12. [gates] TOML section
- Config schema has `GatesConfig` with fields for custom rungs, thresholds, and skip conditions
- No code reads these fields at runtime — gate pipeline uses hardcoded logic
- Same problem as #3 (GateRungConfig)

### 13. [timeouts] TOML section
- Same as #5 — in schema, never read

### 14. Various RokoConfig fields
- Several config fields added by batch implementations exist in the schema but have no runtime readers
- Need a systematic audit: for each field in RokoConfig, verify at least one runtime code path reads it

## Priority Wiring Order

If doing this work, prioritize by impact:

1. **TimeoutConfig** (#5) — highest value, prevents hardcoded timeouts everywhere
2. **RunOutputSink** (#1) — enables streaming output abstraction
3. **Workspace** (#2) — reduces path bugs, improves code clarity
4. **AdaptiveBudget** (#4) — prompts scale to model capability
5. **GateRungConfig** (#3) — gates become configurable without code changes
6. **SafetyLayer** (#9) — security gap, all backends should have safety
7. **Delete dead legacy code** (#6, #7) — reduce confusion

Total effort to wire everything: ~10-12 hours
