# SOLID Tasks -- Properly Implemented

These tasks fulfill their specs, are wired end-to-end, have meaningful tests, and follow
Rust idioms. This document provides verification evidence, implementation file references,
pattern analysis, and a quality benchmark for future work.

---

## Per-Task Verification

### Wave 0 (Foundation)

#### Task 003: TimeoutConfig Wiring
- **Spec**: Replace all hardcoded `Duration::from_secs()` calls in the runner with values from
  `[timeouts]` in `roko.toml`. 6 helper functions, 3 test categories (config roundtrip, rung
  mapping, RunConfig propagation).
- **Delivered**: All runner operational timeouts now derive from `TimeoutConfig`. The helpers
  `agent_dispatch_timeout()`, `plan_total_timeout()`, `llm_call_timeout()`, and
  `gate_timeout()` in the event loop read from `config.roko_config`. Gate timeout correctly maps
  rung 0 -> compile, rung 1 -> clippy, rung >= 2 -> test. `RunConfig::from_roko_config()` copies
  `roko_config.timeouts.agent_dispatch()` and `roko_config.timeouts.plan_total()` into legacy
  scalar fields for backward compatibility.
- **Key files**:
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/timeouts.rs` -- TimeoutConfig struct
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs` -- timeout helpers
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/types.rs` -- RunConfig propagation
- **What makes it solid**: Config struct was already correct; the task was pure wiring. Existing
  tests (`run_config_uses_timeout_config_from_roko_toml`) prove config propagation. No new
  abstractions introduced -- just calls to what already existed.

---

### Wave 1 (Parallel Fixes)

#### Task 009: SafetyLayer Universal Coverage
- **Spec**: Ensure all agent backends that bypass `ToolDispatcher` still call
  `SafetyLayer.check_pre_execution()` or `check_exec_command()`. Audit every backend, classify
  which are covered by ToolDispatcher and which need direct checks.
- **Delivered**: Verified via grep -- every backend's tool execution path now calls safety checks:
  - `ExecAgent` (exec.rs:123) calls `self.safety.check_exec_command()` before subprocess spawn.
  - `CursorAgent` (cursor_agent.rs:457) calls `self.safety.check_pre_execution()` on tool calls.
  - `GeminiNativeAgent` (gemini/native.rs:422) calls `self.safety.check_pre_execution()` on
    function calls.
  - Tool-loop backends are covered by `ToolDispatcher::dispatch()`.
- **Key files**:
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/exec.rs` -- ExecAgent safety
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/cursor_agent.rs` -- Cursor safety
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/gemini/native.rs` -- Gemini safety
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/safety/mod.rs` -- SafetyLayer core
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/provider/mod.rs` -- thread-local scoping
- **What makes it solid**: Systematic coverage audit. Every backend was classified (tool-loop
  covered vs. direct check needed vs. text-only). Thread-local `current_safety_layer()` pattern
  correctly used before await points. Tests use sentinel patterns for dangerous commands.

#### Task 012: Schema Validation Wiring
- **Spec**: Wire `validate_against_schema()` into both `plan_loader.rs` (runtime) and
  `plan_validate.rs` (CLI validate command). Integration test asserts PLAN_035 error rule and
  nonzero exit.
- **Delivered**: Dual-path validation confirmed via grep:
  - `plan_loader.rs:44` calls `tasks.validate_against_schema()` after parsing TOML
  - `plan_validate.rs:319` calls `tasks_file.validate_against_schema()` in the validation command
  - Both share the same `TasksFile::parse()` -> `validate_against_schema()` path
- **Key files**:
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/plan_loader.rs` -- runtime validation
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/plan_validate.rs` -- CLI validate
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/task_parser.rs` -- validate_against_schema()
- **What makes it solid**: Schema errors are caught before agent dispatch (not after). Two
  independent code paths both converge on the same validator. Distinct error codes: PLAN_034 for
  TOML parse failures, PLAN_035 for schema failures.

#### Task 027: Engram Balance Field
- **Spec**: Add `balance: f64` to `Engram` for demurrage support. Serde backwards-compatible with
  `default = 1.0`. `touch()` resets to 1.0. Excluded from `content_hash()`. 5 specific tests.
- **Delivered**: Field exists at engram.rs:97 with `#[serde(default = "default_balance")]`.
  `touch()` method at line 151. Builder support via `.balance(val)`. All 5 required tests present
  including `touch_resets_balance_to_one`, content hash exclusion, and serde defaults.
- **Key files**:
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/engram.rs` -- struct + builder + tests
- **What makes it solid**: Backward-compatible serde defaults. Content hash correctly excludes
  mutable metadata. Builder pattern follows existing codebase conventions. No downstream changes
  needed -- `Signal` type alias automatically inherits the new field.

#### Task 045: Bound Streaming Channels
- **Spec**: Replace `mpsc::UnboundedSender<StreamChunk>` with bounded
  `mpsc::channel(DEFAULT_CHANNEL_BUFFER)` in all LLM streaming paths. Bridge pattern at WS
  boundary. All callsites use the shared constant.
- **Delivered**: Grep confirms bounded channels at all streaming creation sites:
  - `openai_compat_backend.rs:1241,1397` -- bounded channel creation
  - `testutil.rs:464` -- test streaming channel
  - `tool_loop/mod.rs:1659` -- tool loop streaming
  - All use `roko_core::defaults::DEFAULT_CHANNEL_BUFFER` constant
- **Key files**:
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/openai_compat_backend.rs` -- provider streaming
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/tool_loop/mod.rs` -- tool loop streaming
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/agent.rs` -- Agent trait signatures
- **What makes it solid**: Single constant for buffer size across all sites. Bridge pattern
  correctly adapts public unbounded `DispatchLike` trait to internal bounded channels. Natural
  backpressure under slow consumers instead of unbounded memory growth.

#### Task 046: PRD Promote Atomicity
- **Spec**: Replace write-then-delete with atomic writes. Replace manual line-scanner frontmatter
  parser with `serde_yaml_ng`. Convert all critical PRD writes to `atomic_write_str`.
- **Delivered**: Grep confirms 7 `atomic_write_str` callsites in prd.rs (lines 355, 823, 1296,
  1307, 1319, 1889, and the import at line 37). The promote path at line 823 uses
  `atomic_write_str(&dst, &content)`. Plan generation uses it for `tasks.toml` and `plan.md`.
  PRD metadata updates use it for the frontmatter rewrite.
- **Key files**:
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/prd.rs` -- PRD lifecycle + frontmatter
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/io.rs` -- atomic_write_str implementation
- **What makes it solid**: YAML frontmatter parser handles edge cases that broke the old line
  scanner (colons in values, quoted strings, block lists). Tests specifically cover these edge
  cases. Write atomicity prevents partial-file corruption on crash.

#### Task 052: Atomic Writes (PRD/Learn)
- **Spec**: Convert `std::fs::write` calls in PRD plan writes and cascade_router save to
  `atomic_write_str`.
- **Delivered**: PRD plan writes (tasks.toml, plan.md) and cascade router persistence all use
  `atomic_write_str`. This task overlaps with 046 and both were implemented correctly.
- **Key files**:
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/prd.rs` -- PRD writes
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/` -- cascade router persistence
- **What makes it solid**: Consistent application of the same pattern across two subsystems.

#### Task 053: Workspace Persistence
- **Spec**: Persist workspace registry to `.roko/workspaces.json` so workspaces survive server
  restart. Load on startup with validation. Save on create/delete with rollback on failure.
  Configurable GC interval.
- **Delivered**: Full registry implementation confirmed via grep:
  - `state.rs:510` -- `workspace_registry_path_for()`
  - `state.rs:516` -- `load_workspace_registry()`
  - `state.rs:677` -- startup loads from registry
  - `state.rs:894` -- `persist_workspace_registry()`
  - `state.rs:921,940,978` -- save on insert/remove/touch
  - `routes/workspaces.rs:274,285,303` -- save on create/delete API calls
  - `lib.rs:1660` -- GC persists after cleanup
- **Key files**:
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/state.rs` -- registry + AppState
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/workspaces.rs` -- CRUD routes
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/lib.rs` -- GC integration
- **What makes it solid**: Rollback on failure (reinsert entry if persist fails). Startup recovery
  with path validation. 5 persistence tests including restart survival, missing-path recreation,
  and delete cleanup. Configurable GC interval via `[server].workspace_gc_interval_secs`.

---

### Wave 2 (V2 Core + CLI)

#### Task 076: Tool Dispatch Safety Redesign
- **Spec**: Merge duplicate denylists, add path confinement, add env scrubbing, add `__truncated`
  detection, add `#[cfg(test)]` permissive mode. Five coordinated changes across safety and
  dispatch layers.
- **Delivered**: All five components verified:
  - `bash.rs:77` -- `allowed_path_prefixes` field on BashPolicy
  - `bash.rs:204-205` -- `check_path_confinement()` wired into `check_command_with_policy()`
  - `bash.rs` -- 4+ path confinement tests (empty allows all, prefix matching, rejection, metachar)
  - `bash.rs:96-104` (roko-std) -- `cmd.env_clear()` + `safe_env_keys` whitelist
  - `dispatcher/mod.rs:236-238` -- `__truncated` detection at top of dispatch
  - `dispatcher/mod.rs:128-136` -- `new_unguarded()` with `SafetyLayer::permissive()`
  - All test dispatchers migrated to `new_unguarded()`
- **Key files**:
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/safety/bash.rs` -- BashPolicy + path confinement
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/dispatcher/mod.rs` -- dispatch safety
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-std/src/tool/builtin/bash.rs` -- env scrubbing
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/safety/mod.rs` -- SafetyLayer::permissive()
- **What makes it solid**: Defense in depth -- three independent layers (denylist, path confinement,
  env scrubbing) each catch different attack vectors. `#[cfg(test)]` gating prevents permissive
  mode from leaking into production. Single canonical denylist eliminates divergence risk. Clear
  error messages for truncated arguments enable model self-correction.

#### Task 078: Learning Loop Completeness
- **Spec**: Wire `persist_capture_episode` into every PRD dispatch surface. Bootstrap
  `ProviderHealthTracker` from persisted state so circuit breaker survives process restarts.
- **Delivered**:
  - `prd.rs` has 4 `persist_capture_episode` calls (lines 1156, 1176, 1345, 1470), all with
    `"prd-plan-generate"` task kind. Canonical helper used, not the duplicate
    `commands::util::persist_capture_episode`.
  - `runtime_feedback.rs:1277` -- `provider_health_tracker_from_persisted()` loads from disk
  - `runtime_feedback.rs:1344,1414` -- both `open()` and `open_with_models()` use the bootstrap
  - `ProviderHealthRegistry::load_or_new()` called from multiple test and production paths
- **Key files**:
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/prd.rs` -- PRD episode persistence
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/runtime_feedback.rs` -- health bootstrap
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/agent_exec.rs` -- canonical persist helper
- **What makes it solid**: Every PRD dispatch surface now feeds the learning loop. The provider
  health bootstrap closes the "manual success doesn't fix circuit breaker" gap. Regression test
  (`dispatch_surfaces_provide_episodes`) catches new unlearned dispatch paths.

#### Task 080: Unwrap Elimination
- **Spec**: Replace `.unwrap()` and `.expect()` in 8 production code clusters with proper error
  handling. Preserve compile-time infallible expects (regex literals, type invariants).
- **Delivered**: Targeted replacement across serve/lib.rs, provider/openai_compat.rs,
  provider/mod.rs, chain/marketplace.rs, cli/prd.rs. Marketplace now uses
  `.ok_or(MarketplaceError::NotFound)?` consistently (14 callsites confirmed). The prd.rs
  fuzzy-matcher uses `best.map_or(true, |(_, best_dist)| dist < best_dist)` instead of
  `best.unwrap().1`. Safety hooks' `TaintedString::as_str()` expect was correctly preserved as
  a type invariant.
- **Key files**:
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/lib.rs` -- server state, chain watcher
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/provider/openai_compat.rs` -- MCP runtime
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/provider/mod.rs` -- semaphore, HTTP client
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/marketplace.rs` -- HashMap safety
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/prd.rs` -- fuzzy matcher
- **What makes it solid**: Nuanced judgment -- didn't blindly replace all unwraps. Preserved
  infallible patterns (regex, type invariants). Each replacement matches the enclosing function's
  error type (anyhow, concrete enum, logged fallback). No new `Box<dyn Error>` introduced.

#### Task 085: Config Architecture Redesign
- **Spec**: Fix `ConfigCache` ArcSwap duplication bug (watcher and `get()` diverge), wire
  file-watch invalidation, surface diagnostics on hot path, ACP live reload, config export.
- **Delivered**:
  - `cache.rs` -- `ConfigCache` stores `config: Arc<ArcSwap<RokoConfig>>`, watcher closure
    captures `Arc::clone(&config)`, `get()` calls `self.config.load_full()`. The duplication bug
    is fixed -- watcher and reader share the same swap.
  - Regression test at cache.rs:193+ specifically documents and prevents the bug.
  - Watched reload test at cache.rs:210+ writes, edits, and polls for config update.
  - `config/mod.rs:37` -- `pub use cache::ConfigCache` re-exported
  - Diagnostics flow through `load_from_resolved_path()` so both hot-path and validated-path see
    warnings.
  - ACP `config_sources()` has 6+ unit tests covering explicit global, workspace, missing files,
    ordering, and implicit global paths.
- **Key files**:
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/cache.rs` -- ConfigCache + tests
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/loader.rs` -- diagnostics on load
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/mod.rs` -- re-export
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/config.rs` -- config_sources + tests
- **What makes it solid**: Found and fixed a real production bug (ArcSwap readers getting stale
  data). Regression test prevents recurrence. 6+ focused config_sources tests. The cache design is
  correct: single `Arc<ArcSwap>` shared between watcher closure and `get()`.

#### Task 088: ACP Architecture Sweep
- **Spec**: Wire effort to dispatch, remove dead config fields (temperament, routing_mode), add
  `--global-config` flag, add `configSources` to InitializeResult, add missing-workdir warning.
- **Delivered**:
  - `bridge_events.rs` captures `session.config_state.effort` and passes it through
    `config_with_session_effort()` to set `config.agent.default_effort` before dispatch.
  - Dead fields removed: no `temperament` or `routing_mode` in `SessionConfigState`.
  - `--global-config` flag at main.rs:600, forwarded at both ACP dispatch sites.
  - `configSources` populated from `config.config_sources()` in handler.rs:135.
  - `config_with_session_effort()` function at bridge_events.rs:1828.
- **Key files**:
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/bridge_events.rs` -- effort wiring
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/session.rs` -- SessionConfigState
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/handler.rs` -- initialize, config watch
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/config.rs` -- AcpConfig + config_sources
  - `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/main.rs` -- --global-config flag
- **What makes it solid**: Dead code removed instead of left rotting. Effort wiring flows through
  existing config infrastructure rather than adding a parallel path. Multiple remaining gaps are
  documented in the task's status log rather than silently ignored.

---

### Additional Solid Tasks (Summary Verification)

| ID | Title | Verification |
|----|-------|-------------|
| 008 | AdaptiveBudget | All 7 compose templates implement `sections_with_context_window()`. Budget scales with model context window. |
| 013 | SSE keepalive | `.take(256)` replay bound, 8s keepalive interval, correct SSE headers. Minor: missing unit test for the 256 cap. |
| 024 | agents_instructions | Single canonical `common::agents_instructions_section()` called from all 7 templates. No duplication. |
| 028 | Orchestrate feature gate | `legacy-orchestrate` feature flag cleanly gates orchestrate.rs. Default build excludes it. |
| 029 | Delete roko-calc | Clean deletion. Zero references remain. No orphaned imports or Cargo.toml entries. |
| 033 | PostGateReflection | Lessons loaded into retry context. 5 test cases. Minor: dedup is insertion-order, not sorted. |
| 043 | Sync mutex audit | Correctly diagnosed as no-op. `roko-serve` already uses `tokio::sync::Mutex`. Learn locks are sync-correct because they guard CPU-only operations. |
| 044 | MCP transport timeout | 5s write + 30s response timeouts in `roundtrip()`. Lock held inside timeout future, not outside. |
| 049 | roko dev command | PID file management, port probe, SIGINT handling. Minor: missing SIGTERM handler. |
| 051 | Integration tests | 3 test files: config roundtrip (core), PRD pipeline (cli), serve lifecycle (serve). Random port, health polling, tempdir fixtures. |
| 055 | Docker multistage | 3-stage build (frontend, builder, runtime). No Rust toolchain in runtime image. CORS cleanup. |
| 061 | IDE max_output | `effective_max_output()` correctly computed, wired into config options and diagnostics. |
| 063 | IDE MCP notification | All 7 `McpInitStatus` variants, per-server timeout, mapped to `SessionUpdate`. |
| 064 | IDE default fallback | 7-step fallback rule for default model/provider selection. `IndexMap` preserves insertion order. |
| 072 | CLI boot sequence | `session_banner_label()` replaces removed `auth.label()`. Debug log in place. |
| 073 | ACP startup resilience | Graceful degradation for missing/malformed config. Provider readiness check. Config warnings in InitializeResult. |
| 075 | Provider translator | Gemini URL idempotency, Ollama tool sanitization, assistant message override. |
| 077 | Model identity | `tool_loop_max_iterations_for_profile()` wired to all 5 providers. Old API deprecated. Per-model override tested. |

---

## Pattern Analysis

### Common Traits of Solid Implementations

**1. Wiring over building.**
Solid tasks connect existing code to existing runtime paths. Task 003 didn't redesign
`TimeoutConfig` -- it already existed. The task traced the runtime call chain and added 6 helper
functions. Task 012 didn't rewrite the schema validator -- it called `validate_against_schema()`
from two paths that previously skipped it.

**2. Defense in depth, not single-point.**
Task 076 is the best example: three independent safety layers (denylist, path confinement, env
scrubbing) where any one catches different classes of attack. Task 085 shares this: the regression
test for the ArcSwap bug is independent of the fix itself.

**3. Canonical helpers, not local duplicates.**
Task 078 replaced `commands::util::persist_capture_episode` (a local duplicate) with the canonical
`agent_exec::persist_capture_episode` at every PRD site. Task 024 created one
`agents_instructions_section()` in `common.rs` called from all 7 templates instead of 7 inline
copies.

**4. Nuanced judgment about scope.**
Task 043 correctly diagnosed "no change needed" instead of making unnecessary changes. Task 080
preserved compile-time infallible expects while replacing genuine runtime panics. Task 088
documented remaining gaps in a status log instead of claiming completeness.

**5. Tests that catch regressions, not just verify presence.**
Task 085's regression test specifically names the ArcSwap duplication bug. Task 078's
`dispatch_surfaces_provide_episodes` test breaks if someone adds a new PRD dispatch path without
wiring learning. Task 012's integration test spawns the actual binary and asserts exit code.

---

### What Solid Tasks Did That NEEDS_WORK/DUCT_TAPE Tasks Did Not

| Solid Pattern | NEEDS_WORK Anti-Pattern | Example Contrast |
|---|---|---|
| Traced the runtime call chain before editing | Changed struct fields without tracing callers | Task 003 (traced `plan run` -> RunConfig -> event_loop) vs Task 004 (added Workspace but 95 callsites still use RokoLayout) |
| Used existing helpers/infrastructure | Built new parallel infrastructure | Task 078 (used canonical `persist_capture_episode`) vs Task 017 (built rotation but no config fields -- the actual task) |
| Verified observable CLI behavior | Verified "code exists" only | Task 012 (binary exits nonzero for bad schema) vs Task 035 (test file won't compile because `Substrate` is not re-exported) |
| Documented remaining gaps honestly | Marked done without checking | Task 088 (status log lists 6 remaining gaps) vs Task 014 (removed suppression from main.rs but lib.rs still suppresses everything) |
| Single canonical implementation | Duplicated across callsites | Task 024 (one function, 7 callers) vs Task 034 (section_id uses wrong format, collides across retries) |
| Tested edge cases explicitly | Happy-path only or no tests | Task 046 (colons in YAML values, block lists) vs Task 026 (missing serde roundtrip, bus integration, property tests) |

---

### Reusable Patterns Worth Applying to NEEDS_WORK Tasks

**Pattern A: "Trace before touch."**
Before editing any file, trace the full runtime call chain from CLI command to the code being
changed. Write down the chain. If the chain doesn't reach the code being changed, the change is
dead code.
```
Example from Task 003:
  roko plan run -> commands/plan.rs -> load roko_config -> construct RunConfig
  -> runner::event_loop::run() -> gate_timeout(config, rung)
```

**Pattern B: "Canonical helper, not local duplicate."**
When the same logic is needed in N places, create one helper function and call it N times. If a
helper already exists elsewhere, use it -- don't create a local copy with subtle differences.
```
Example from Task 078:
  BAD:  commands::util::persist_capture_episode  (local duplicate)
  GOOD: agent_exec::persist_capture_episode      (canonical, tested)
```

**Pattern C: "Regression test names the bug."**
When fixing a specific bug, write a test whose name and comment describe the bug. Future readers
instantly understand what the test protects against.
```
Example from Task 085:
  // Regression: ConfigCache::new() previously created two independent
  // ArcSwap instances -- watcher updated one, get() read the other.
```

**Pattern D: "cfg(test) gating for permissive modes."**
When adding a permissive/unguarded mode for testing, gate it with `#[cfg(test)]` so it can never
leak into production. The compiler enforces the boundary, not code review.
```
Example from Task 076:
  #[cfg(test)]
  pub fn new_unguarded(...) -> Self { ... safety: SafetyLayer::permissive() ... }
```

**Pattern E: "Observable verification, not code inspection."**
Verify via CLI behavior (exit codes, file creation, log output), not by reading source code. If
the feature isn't observable from `cargo run -p roko-cli -- <command>`, it may not be wired.
```
Example from Task 012:
  cargo run -p roko-cli -- plan validate /tmp/bad-plan/
  # Must exit nonzero and print PLAN_035
```

---

## Quality Benchmark

### What "Solid" Means in This Codebase

A task is solid when it satisfies ALL of these criteria:

1. **Spec adherence**: Every requirement in the task spec has a corresponding code change.
2. **Runtime reachability**: The change is reachable from a CLI command or serve endpoint. Not
   just compiled -- actually called during normal operation.
3. **Test coverage**: At least one test exercises the changed behavior. Integration tests that
   run the actual binary are preferred over unit tests that test internal functions.
4. **Error handling**: No new `.unwrap()` or `.expect()` in production paths. Errors propagate
   via `?`, use explicit fallbacks with logging, or use concrete error enum variants.
5. **No duplication**: Uses existing helpers and infrastructure. Doesn't introduce a parallel
   implementation of something that already exists.
6. **Honest documentation**: Remaining gaps are documented (in status logs, GAPS.md, or task
   notes), not silently omitted.

### Checklist for Future Implementations

Before marking a task done, verify:

- [ ] Trace the runtime call chain from CLI/serve to the changed code. Is the chain complete?
- [ ] Run the Wire Target commands from the task spec. Do they produce expected output?
- [ ] Run all Verification checklist items from the task spec. Do they pass?
- [ ] `cargo build --workspace` -- clean?
- [ ] `cargo test --workspace` -- passing?
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` -- clean?
- [ ] No new `.unwrap()` in production code paths?
- [ ] No duplicate helpers introduced? Used canonical versions?
- [ ] Any remaining gaps documented in task Status Log or `.roko/GAPS.md`?
- [ ] At least one test exercises the changed runtime path?
- [ ] Feature is observable via CLI command or API endpoint?

### Exemplary Code Patterns to Follow

**Config wiring** (Task 003 pattern):
```
// Helper reads from config with clear fallback chain
fn gate_timeout(config: &RunConfig, rung: usize) -> Duration {
    config.roko_config.as_deref()
        .map(|cfg| match rung {
            0 => cfg.timeouts.gate_compile(),
            1 => cfg.timeouts.gate_clippy(),
            _ => cfg.timeouts.gate_test(),
        })
        .unwrap_or(Duration::from_secs(300))
}
```

**Safety-first dispatch** (Task 076 pattern):
```
// Truncation detection BEFORE validation -- model gets actionable error
if call.arguments.get("__truncated").and_then(|v| v.as_bool()) == Some(true) {
    return ToolResult::err(ToolError::Other(format!(
        "tool `{}` received truncated arguments ({} chars)",
        call.name, raw_len
    )));
}
```

**Canonical episode persistence** (Task 078 pattern):
```
// Every dispatch surface flows through the same helper
let _ = persist_capture_episode(
    workdir, &agent_command, &model_key,
    "prd-plan-generate",
    &format!("prd:plan:{slug}"),
    success, elapsed, &capture,
);
```

---

## Concerns Even in Solid Tasks

### Potential Fragility Points

| Task | Concern | Risk |
|------|---------|------|
| 003 | Gate timeout rung mapping uses numeric indices (0=compile, 1=clippy, 2+=test). If rung ordering changes, timeouts silently mismap. | Medium -- no enum, just magic numbers |
| 009 | Thread-local safety layer (`ACTIVE_SAFETY_LAYER`) must be set before any `.await`. If a backend restructures its async flow, safety checks silently stop running. | Medium -- correctness depends on code structure |
| 045 | Bridge pattern at WS boundary still uses unbounded channel. If the websocket consumer stalls, memory grows without bound at that layer. | Low -- WS is user-interactive, natural rate limiting |
| 053 | Workspace registry is a JSON file with no locking. If two `roko serve` processes share a workdir, they can corrupt the registry. | Low -- single-server assumption is documented |
| 076 | Path confinement uses whitespace-split heuristic. Commands with paths in quotes (`cat "/etc/passwd"`) bypass the check. | Medium -- denylist catches worst cases, this is depth-of-defense |
| 078 | `persist_capture_episode` count assertion in `dispatch_surfaces_provide_episodes` relies on string counting. A refactor that moves the function could break the test without breaking the feature. | Low -- test will fail noisily |
| 085 | `ConfigCache` watcher uses `notify::RecommendedWatcher` which is platform-dependent. File system events may be delayed or batched differently on Linux vs macOS. | Low -- degraded latency, not correctness |
| 088 | Effort wiring flows through `config.agent.default_effort` -- a global setting. If two sessions with different effort levels share a config, the last one wins. | Medium -- per-session config is a workaround, not a solution |

### Missing Edge Cases

| Task | Missing Edge Case |
|------|-------------------|
| 013 | No unit test for the `.take(256)` replay bound. If the bound is accidentally removed, SSE streams grow without limit. |
| 046 | YAML frontmatter parser falls back to `None` on any parse error. A single malformed field in otherwise-valid frontmatter silently loses all metadata. |
| 049 | Missing SIGTERM handler. On `docker stop` (which sends SIGTERM before SIGKILL), child processes may be orphaned. |
| 076 | No test for env scrubbing with a variable whose name contains "KEY" (the case-insensitive pattern match). |
| 085 | No test for watcher failure recovery (e.g., watched file deleted and recreated). |

### Areas That Might Regress

| Area | Regression Vector | Protection |
|------|-------------------|------------|
| Safety layer coverage | New backend added without `check_pre_execution()` | No automated check -- requires manual audit |
| Bounded channels | New streaming path uses `unbounded_channel()` | Grep-based verification in CI would catch this |
| Atomic writes | New PRD/learn write uses `std::fs::write` | No automated check -- task 046/052 set the pattern |
| Episode persistence | New agent dispatch path skips `persist_capture_episode` | `dispatch_surfaces_provide_episodes` test catches for PRD, not other surfaces |
| Config cache | New code calls `load_config_unified()` instead of `ConfigCache::get()` in a hot path | No automated check -- both work, but the hot-path version re-reads from disk |
