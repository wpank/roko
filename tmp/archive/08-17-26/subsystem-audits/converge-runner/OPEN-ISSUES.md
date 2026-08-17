# Converge Runner — Open Issues Checklist

Issues remaining after audit + fixes. Organized by priority.

## Critical (6 open)

- [ ] **CRIT-05**: Duplicate `AffectPolicy` trait — `effect_driver.rs` defines its own
  version with incompatible `BoxFuture`/`Result` types vs `foundation.rs`. The two
  can't be used interchangeably. Delete the local one and use the foundation trait,
  or vice versa. Affects: D01, D03, D04.
  - `crates/roko-runtime/src/effect_driver.rs:69-87`
  - `crates/roko-core/src/foundation.rs`

- [ ] **CRIT-06**: `DispatchModulation` computed but never applied — `spawn_agent()`
  calls `policy.modulate_dispatch()` but the resulting `tier_bias`,
  `turn_limit_factor`, `exploration_rate` are discarded. The `ModelCallRequest`
  gets empty/default values. Wire modulation into request construction.
  - `crates/roko-runtime/src/effect_driver.rs:133-138`

- [ ] **CRIT-07**: `flush_async` TOCTOU race — reads buffer length then acquires lock
  in separate steps. Another thread could modify between. Use a single lock acquisition.
  - `crates/roko-learn/src/feedback_service.rs`

- [ ] **CRIT-08**: `--share` flag is no-op with default engine — flag added but only
  works with `--engine legacy`. Default v2 engine silently ignores it. Either wire
  it in v2 or error when used with v2.
  - `crates/roko-cli/src/run.rs`

- [ ] **CRIT-09**: `resume()` missing first `PhaseTransition` event — observers
  miss the initial state on resume. Emit a PhaseTransition for the resumed phase
  before continuing execution.
  - `crates/roko-runtime/src/workflow_engine.rs`

- [ ] **CRIT-10**: `resume()` fires `StateCheckpointed` with empty path — should use
  the actual checkpoint file path, not `String::new()`.
  - `crates/roko-runtime/src/workflow_engine.rs`

## Warning — Built but Not Wired (12 open)

- [ ] **W-01**: GatewayEventWriter never instantiated (`crates/roko-agent/src/gateway_events.rs`)
- [ ] **W-02**: force_backend doesn't feed CascadeRouter learning (`crates/roko-agent/src/model_call_service.rs`)
- [ ] **W-03**: Knowledge-aware routing returns hardcoded defaults (`crates/roko-learn/src/cascade_router.rs`)
- [ ] **W-04**: Knowledge injection in prompts returns empty context (`crates/roko-compose/src/prompt_assembly_service.rs`)
- [ ] **W-05**: Adaptive gate thresholds not in WorkflowEngine path
- [ ] **W-06**: Episode distillation never triggered at runtime
- [ ] **W-07**: Output format functions partially wired in v2 engine
- [ ] **W-11**: ACP bridge forwards only subset of WorkflowEngine events
- [ ] **W-12**: roko-serve background tasks bypass WorkflowEngine
- [ ] **W-13**: SSE endpoint streams incomplete event set
- [ ] **W-21**: Research/dreams/neuro still use own LLM call mechanisms
- [ ] **W-22**: Cache cell has no TTL/staleness detection

## Warning — Design Issues (8 open)

- [ ] **W-08**: StateHub type split — same source via `#[path]` creates distinct types
  in roko-cli vs roko-serve. Should be a proper shared crate or module.
- [ ] **W-09**: Hardcoded empty model string in EffectDriver spawn
- [ ] **W-10**: Hand-rolled TOML parser in pipeline_state instead of serde
- [ ] **W-14**: CascadeRouter bandit observation missing latency/quality metrics
- [ ] **W-15**: Section effectiveness scoring returns constant values
- [ ] **W-16**: MCP config passthrough not verified in new path
- [ ] **W-17**: Cost prediction uses hardcoded estimates, not historical data
- [ ] **W-18**: Thinking cap + convergence detection are passthroughs

## Warning — Minor (5 open)

- [ ] **W-19**: Custom gate is a no-op (`ShellGate::new("true", vec![])`)
- [ ] **W-20**: Format check gate only supports Cargo
- [ ] **W-23**: Budget cell has no persistence across restarts
- [ ] **W-24**: Test T05 uses hardcoded port (flaky)
- [ ] **W-25**: CLI progress printer terminal width fallback

## Note (20 open, low priority)

- [ ] N-01: C01 was success_noop (file already existed)
- [ ] N-02: "Clack-style" referenced without definition
- [ ] N-03: status.tsv has no header row
- [ ] N-04: Runner doesn't clean up worktree on completion
- [ ] N-05: Some batch prompts reference not-yet-existing files
- [ ] N-06: Structural checks are grep-based, not semantic
- [ ] N-07: R-track failure expected but undocumented
- [ ] N-08: Some converge commits have very long messages
- [ ] N-09: deny.toml allows all licenses
- [ ] N-10: CI layer-check step may need build first
- [ ] N-11: layer_check.rs doesn't handle workspace Cargo.toml inheritance
- [ ] N-12: Some test files import both old and new trait paths
- [ ] N-13: uuid_short() uses millisecond timestamp (not globally unique)
- [ ] N-14: floor_char_boundary manually implemented (resolved for MSRV)
- [ ] N-15: count_changed_files shells out to git
- [ ] N-16: FeedbackService buffer capacity not configurable
- [ ] N-17: `#[allow(clippy::too_many_lines)]` workarounds
- [ ] N-18: Error encoded in CommitDone hash string
- [ ] N-19: Some imports reference old module locations
- [ ] N-20: Dashboard pages have minimal API error handling

## R-Track: Feature-Gating orchestrate.rs (4 open)

These are the 4 failed converge batches. They need manual implementation
because the 21K-line orchestrate.rs monolith is too complex for single
Codex prompts.

- [ ] **R02**: Feature-gate `orchestrate.rs` module behind `legacy-orchestrate`
- [ ] **R03**: Feature-gate `dispatch_helpers` + `agent_spawn` behind legacy
- [ ] **R04**: Ensure `cargo check` passes WITHOUT `legacy-orchestrate` feature
- [ ] **R05**: Ensure `cargo check` passes WITH `legacy-orchestrate` feature

**Note**: R01 (add feature to Cargo.toml) succeeded. The blockers are R02-R05
which require careful conditional compilation across the entire orchestrate.rs
and its dependents.

## Summary

| Severity | Total | Fixed | Open |
|----------|-------|-------|------|
| Critical | 10 | 4 | 6 |
| Warning | 25 | 0 | 25 |
| Note | 20 | 0 | 20 |
| R-track | 4 | 0 | 4 |
| **Total** | **59** | **4** | **55** |

## What to Work on Next

**Highest impact, lowest effort**:
1. CRIT-10 (empty checkpoint path) — one-line fix
2. CRIT-09 (missing PhaseTransition on resume) — add one event emit
3. CRIT-05 (duplicate AffectPolicy) — delete local trait, use foundation version
4. CRIT-06 (wire DispatchModulation) — use modulation values in ModelCallRequest
5. CRIT-08 (`--share` no-op) — either wire or emit warning
6. CRIT-07 (TOCTOU race) — restructure to single lock acquisition
