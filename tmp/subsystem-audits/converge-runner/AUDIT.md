# Converge Runner — Full Audit Findings

Audit performed 2026-04-28 after merging 83 converge commits into `wp-arch2`.
Four parallel audit agents reviewed all 13 tracks.

## Severity Levels

- **CRITICAL**: Runtime panics, data corruption, incorrect behavior, security holes
- **WARNING**: Built but not wired, dead code, missing integration, design issues
- **NOTE**: Style, documentation, naming, minor improvements

---

## CRITICAL Issues (10)

### CRIT-01: `block_on` panic in affect policy persist — **FIXED**
- **File**: `crates/roko-cli/src/run.rs` (~line 382)
- **Track**: D04 (Wire DaimonPolicy into CLI run path)
- **Problem**: `RuntimeAffectPolicyAdapter::persist()` used
  `futures::executor::block_on(policy.persist())` inside a `BoxFuture` polled
  by tokio. This panics at runtime with "Cannot start a runtime from within a
  runtime" because `block_on` creates a new runtime inside the existing tokio
  runtime.
- **Fix**: Replaced with `tokio::task::spawn_blocking` +
  `tokio::runtime::Handle::current().block_on()` pattern, which correctly bridges
  sync and async contexts.

### CRIT-02: Layer violation — roko-runtime at layer 0 — **FIXED**
- **File**: `crates/roko-runtime/Cargo.toml` (line 33)
- **Track**: L01 (Add layer metadata)
- **Problem**: L01 batch set `roko-runtime` to `layer = 0` but roko-runtime
  depends on `roko-core` which is layer 1. The layer-check binary (L02) would
  flag this as a violation.
- **Fix**: Changed `layer = 0` → `layer = 1`.

### CRIT-03: Wrong PipelineInput variant on commit error — **FIXED**
- **File**: `crates/roko-runtime/src/effect_driver.rs` (~line 312-376)
- **Track**: E04 (EffectDriver commit effect)
- **Problem**: `commit()` returned `PipelineInput::AgentFailed` when `git add` or
  `git commit` failed. But the state machine is in `Phase::Committing` which only
  handles `CommitDone` transitions. An `AgentFailed` input in this phase would
  cause an unhandled state transition.
- **Fix**: Changed all error returns to `PipelineInput::CommitDone` with
  error-prefixed hash strings (e.g., `"error: git add failed: ..."`).

### CRIT-04: Silent stub judge gate pass — **FIXED**
- **File**: `crates/roko-gate/src/gate_service.rs` (~line 184-187)
- **Track**: S12 (GateService remaining rungs)
- **Problem**: `StubJudgeGate` returned `Verdict::pass(...)`, silently passing
  verification when the LLM judge is not implemented. If a user enables "judge"
  in `enabled_gates`, they would get false confidence that their code was reviewed.
- **Fix**: Changed to `Verdict::fail("stub-llm-judge", "LLM judge gate not yet
  implemented — enable a real judge or remove from enabled_gates")`.

### CRIT-05: Duplicate `AffectPolicy` trait — OPEN
- **Files**: `crates/roko-runtime/src/effect_driver.rs` (lines 69-87) vs
  `crates/roko-core/src/foundation.rs`
- **Tracks**: D01 + D03
- **Problem**: D01 added `AffectPolicy` to `foundation.rs`, D03 added its own
  version in `effect_driver.rs` with incompatible types (different `BoxFuture`,
  different `Result` type). The two traits cannot be used interchangeably. The
  `EffectServices` struct uses the local version, so the foundation trait is
  dead code.
- **Impact**: Any code trying to pass a `foundation::AffectPolicy` impl to
  `EffectDriver` won't compile. The D04 adapter in `run.rs` works around this
  by implementing the local trait directly.

### CRIT-06: DispatchModulation computed but never applied — OPEN
- **File**: `crates/roko-runtime/src/effect_driver.rs` (~line 133-138)
- **Track**: D03
- **Problem**: `spawn_agent()` calls `policy.modulate_dispatch(role, &mut modulation)`
  to get tier_bias, turn_limit_factor, and exploration_rate. But these values are
  never used — the `ModelCallRequest` is constructed with hardcoded defaults
  (empty model string, no temperature override). The modulation is dead computation.
- **Impact**: The affect engine's behavioral modulation has zero effect on actual
  model calls.

### CRIT-07: `flush_async` TOCTOU race — OPEN
- **File**: `crates/roko-learn/src/feedback_service.rs`
- **Track**: S10/S11
- **Problem**: `flush_async` reads the buffer length, then acquires a lock and
  drains. Between the length check and the drain, another thread could have
  modified the buffer, leading to partial flushes or missed events.
- **Impact**: Low in practice (single-threaded runtime), but architecturally unsound.

### CRIT-08: `--share` flag is no-op — OPEN
- **File**: `crates/roko-cli/src/run.rs`
- **Track**: C07
- **Problem**: The `--share` CLI flag was added but only works with `--engine legacy`.
  The default engine (`v2`) ignores the flag entirely. Users who pass `--share`
  get no error and no share URL.
- **Impact**: Silent feature failure for the default code path.

### CRIT-09: `resume()` missing first PhaseTransition event — OPEN
- **File**: `crates/roko-runtime/src/workflow_engine.rs`
- **Track**: E08
- **Problem**: When resuming from checkpoint, the engine doesn't emit a
  `RuntimeEvent::PhaseTransition` for the phase it resumes into. Observers
  (JsonlLogger, TUI, SSE) miss the initial state.
- **Impact**: Log gaps on resume — the first event after resume is from the
  middle of execution with no context about the starting phase.

### CRIT-10: `resume()` fires StateCheckpointed with empty path — OPEN
- **File**: `crates/roko-runtime/src/workflow_engine.rs`
- **Track**: E08
- **Problem**: On resume, `StateCheckpointed` event is emitted with
  `path: String::new()` instead of the actual checkpoint file path.
- **Impact**: Misleading log entries.

---

## WARNING Issues (25)

### W-01: GatewayEventWriter built but never instantiated — OPEN
- **Track**: G02
- **File**: `crates/roko-agent/src/gateway_events.rs`
- Gateway event writer + projection types exist but are never constructed or
  called from any live code path.

### W-02: force_backend doesn't feed CascadeRouter learning — OPEN
- **Track**: G09
- **File**: `crates/roko-agent/src/model_call_service.rs`
- When a user sets `force_backend`, the manual override result is not recorded as
  a bandit observation. The router can't learn from forced choices.

### W-03: Knowledge-aware routing method stubbed — OPEN
- **Track**: K01
- **File**: `crates/roko-learn/src/cascade_router.rs`
- `knowledge_route()` or similar method exists but returns hardcoded defaults.
  Knowledge store is never actually consulted during model selection.

### W-04: Knowledge injection in prompts is no-op — OPEN
- **Track**: K03
- **File**: `crates/roko-compose/src/prompt_assembly_service.rs`
- `ContextSource` for neuro/knowledge was added but the implementation returns
  empty context. Prompts don't actually get knowledge-enriched content.

### W-05: Adaptive gate thresholds not loaded in WorkflowEngine path — OPEN
- **Track**: S13
- Gate thresholds are loaded and used in the legacy orchestrate.rs path but not
  wired into the new WorkflowEngine `run_gates()` flow. The GateConfig passed
  from EffectDriver doesn't include adaptive threshold state.

### W-06: Episode distillation not triggered — OPEN
- **Track**: K04/K05
- Episode metadata records knowledge usage but the feedback loop to update
  knowledge confidence scores is never triggered at runtime.

### W-07: Output format functions exist but not all wired — OPEN
- **Track**: C01-C06
- `output_format.rs` has identity line, cost prediction, cost actual, gate
  results formatters. Some are called from the v2 engine path, others are only
  used in tests.

### W-08: StateHub type split across crates — OPEN
- **Files**: `roko-core/src/state_hub.rs` included via `#[path]` in roko-cli and
  roko-serve
- Both crates include the same source file but produce distinct types. A
  `StateHubSender` from roko-serve cannot be passed where roko-cli's version is
  expected. Workaround: create local hub instances per crate.

### W-09: Hardcoded model in EffectDriver spawn — OPEN
- **Track**: E03
- **File**: `crates/roko-runtime/src/effect_driver.rs` (~line 177)
- `ModelCallRequest.model` is set to `String::new()` (empty). The ModelCallService
  is expected to fill in a default, but if it doesn't, the request has no model.

### W-10: Hand-rolled TOML parser in pipeline_state — OPEN
- **Track**: E01
- **File**: `crates/roko-runtime/src/pipeline_state.rs`
- Config loading parses TOML manually instead of using serde + the existing
  roko-core config types.

### W-11: ACP bridge wiring is minimal — OPEN
- **Track**: W05
- `bridge_events.rs` connects to WorkflowEngine events but only forwards a
  subset. The ACP protocol doesn't get gate results or commit events.

### W-12: roko-serve background tasks don't use WorkflowEngine — OPEN
- **Track**: W07
- The serve crate's background task spawning still uses its own ad-hoc async
  pattern instead of routing through WorkflowEngine.

### W-13: SSE endpoint streams incomplete event set — OPEN
- **Track**: W08
- SSE events from roko-serve only include a subset of RuntimeEvent variants.
  Dashboard consumers don't receive all event types.

### W-14: CascadeRouter bandit observation incomplete — OPEN
- **Track**: S11
- FeedbackService records model call events but the bandit observation for
  CascadeRouter doesn't include latency or quality metrics — only success/failure.

### W-15: Section effectiveness scoring is a stub — OPEN
- **Track**: S09
- PromptAssemblyService has a scoring method but it returns constant values.
  No actual effectiveness measurement is implemented.

### W-16: MCP config passthrough not verified — OPEN
- **Track**: S05
- ModelCallService accepts MCP config but whether it actually reaches the agent
  spawn is untested. The `--mcp-config` flag may be dropped in the new path.

### W-17: Cost prediction is hardcoded estimates — OPEN
- **Track**: S04
- Cost tracking records actual costs but prediction uses hardcoded per-model
  estimates rather than learning from historical data.

### W-18: Thinking cap + convergence detection are stubs — OPEN
- **Track**: G08
- These "cells" in ModelCallService exist as types but their implementation
  is a passthrough that doesn't actually detect convergence or apply thinking caps.

### W-19: Custom gate reads no config — OPEN
- **Track**: S12
- `gate_for_name("custom")` returns `ShellGate::new("true", vec![])` — a no-op.
  There's a TODO comment about reading from GateConfig but no `custom_command` field
  exists in GateConfig yet.

### W-20: format check gate only supports Cargo — OPEN
- **Track**: S12
- `FormatCheckGate::cargo()` is the only implementation. No npm/prettier, go fmt,
  or other format checkers.

### W-21: Research/dreams/neuro domain callers not migrated — OPEN
- **Track**: G06
- These subsystems still use their own LLM call mechanisms instead of routing
  through ModelCallService.

### W-22: Cache cell returns cached results without staleness check — OPEN
- **Track**: G07
- ModelCallService cache cell caches responses but has no TTL or staleness
  detection. Old cached results may be returned indefinitely.

### W-23: Budget cell has no persistence — OPEN
- **Track**: G07
- Budget tracking resets on process restart. No file or database persistence
  for budget state.

### W-24: Test T05 (share URL) may be flaky — OPEN
- **Track**: T05
- The share endpoint integration test uses a hardcoded port. If the port is
  in use, the test fails with a confusing bind error.

### W-25: CLI progress printer uses terminal width detection that may fail — OPEN
- **Track**: O04/O05
- Falls back to 80 columns but some formatting assumes wider terminals.

---

## NOTE Issues (20)

### N-01: C01 was success_noop — output_format.rs already existed
### N-02: Many prompts reference "Clack-style" without defining it
### N-03: Status TSV has no header row
### N-04: Runner doesn't clean up worktree on completion
### N-05: Some batch prompts reference files that don't exist yet (dep ordering)
### N-06: Structural checks are grep-based, not semantic
### N-07: R-track failure expected but not documented in BATCHES.md
### N-08: Some converge commits have very long commit messages
### N-09: deny.toml (L03) allows all licenses — needs tightening
### N-10: CI workflow (L04) layer-check step may need build first
### N-11: layer_check.rs reads Cargo.toml metadata but doesn't handle workspace inheritance
### N-12: Several test files import both old and new trait paths
### N-13: `uuid_short()` in effect_driver uses millisecond timestamp — not globally unique
### N-14: `truncate_message` doesn't handle multi-byte UTF-8 boundaries correctly
  - Actually fixed: manual `floor_char_boundary` helper added for MSRV < 1.91
### N-15: `count_changed_files` shells out to git — could use git2 crate
### N-16: FeedbackService buffer capacity is not configurable
### N-17: Several `#[allow(clippy::too_many_lines)]` annotations added as workarounds
### N-18: Error messages in PipelineInput::CommitDone encode error type in hash string
### N-19: Some import paths still reference old module locations
### N-20: Demo app dashboard pages have minimal error handling for API failures
