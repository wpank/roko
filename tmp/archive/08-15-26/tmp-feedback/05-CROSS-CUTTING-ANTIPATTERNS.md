# Cross-Cutting Anti-Patterns

These issues span multiple tasks and crates. They represent systemic problems
rather than isolated task failures. Each section includes concrete evidence,
root cause analysis, a proper architectural solution with migration path,
and a prevention strategy.

---

## 1. "STATUS: NOT WIRED" Floating Modules (8,246 LOC)

### Severity: CRITICAL

15 modules explicitly tagged as disconnected from any runtime path:

**roko-learn** (8 modules, ~2,365 LOC):

| Module | LOC | What it does | Why it matters |
|--------|-----|--------------|----------------|
| `verdict_scorer.rs` | 621 | Scores gate verdicts for quality trends | Cannot detect degrading gate performance |
| `error_enrichment.rs` | 333 | Classifies errors for retry strategy | Retries are blind to error type |
| `event_subscriber.rs` | 430 | Subscribes to runtime events for learning | Learning pipeline has no event source |
| `active_inference.rs` | 257 | Bayesian tier selection for routing | Cascade router uses fixed priors |
| `quality_judge.rs` | 86 | Judges agent output quality | No quality signal feeds back |
| `bayesian_confidence.rs` | 288 | Updates routing confidence from outcomes | Confidence is static |
| `calibration_policy.rs` | 264 | Predict-correct calibration | Router cannot self-correct |
| `oracles/mod.rs` | 13 | Oracle module stub | No oracle system connected |

**roko-runtime** (7 modules, ~5,954 LOC):

| Module | LOC | What it does | Why it matters |
|--------|-----|--------------|----------------|
| `heartbeat_probes.rs` | 1,545 | Agent health probing with rolling stats | Dead agents go undetected |
| `heartbeat_attention.rs` | 2,146 | Attention/priority system with bidders | All tasks equal priority |
| `theta_consumer.rs` | 477 | Rhythm-based scheduling | No cadence optimization |
| `task_scheduler.rs` | 380 | Priority queue dispatch | FIFO only |
| `demurrage_consumer.rs` | 474 | Knowledge decay scheduling | Timer works but consumer is dead |
| `energy.rs` | 508 | Cognitive metabolism / load tracking | No backpressure signals |
| `delta_consumer.rs` | 424 | Differential state updates | Full snapshots only |

### Concrete evidence

Every one of these files starts with an identical banner:

```
//! STATUS: NOT WIRED -- built but no non-test runtime caller.
```

The v2 runner event loop (`crates/roko-cli/src/runner/event_loop.rs`) is the
actual runtime. Its imports show zero references to any of these 15 modules.
It imports from `roko_orchestrator`, `roko_gate`, `roko_learn` (only
`post_gate_reflection` and `section_outcome`), `roko_neuro` (only
`KnowledgeStore`), and `roko_daimon` -- but none of the above modules.

Similarly, the legacy `PlanRunner` in `orchestrate.rs` does not import any of
these modules either.

### Root cause analysis

These were built during early architecture phases (pre-v2 runner) when the
codebase was designed around a pub/sub event bus model. When the v2 runner
replaced the original orchestration loop with a simpler `tokio::select!`
approach, these modules were never reconnected. The `STATUS: NOT WIRED` tag
was added as a triage marker but nothing followed up.

The fundamental issue: **no integration tracking**. Modules get built, pass
`cargo test` (with self-contained unit tests), and appear "done" -- but
nobody verifies they have a live runtime caller.

### Architectural solution

**Triage into three buckets: wire, defer, delete.**

**Bucket A -- Wire now (high value, clear integration point):**

1. `event_subscriber.rs` (430 LOC) -- The runner event loop already emits
   `RuntimeEvent` via `runtime_event_bus`. Wire `run_learning_subscriber()`
   as a `tokio::spawn` task consuming the bus receiver:

   ```rust
   // In runner/event_loop.rs, after constructing runtime_event_bus:
   let learning_rx = runtime_event_bus.subscribe();
   let learning_handle = tokio::spawn(
       roko_learn::event_subscriber::run_learning_subscriber(learning_rx, layout.clone())
   );
   // ... on shutdown:
   learning_handle.abort();
   ```

   **Files to change:** `crates/roko-cli/src/runner/event_loop.rs`

2. `error_enrichment.rs` (333 LOC) -- The runner already has retry logic in
   `runner/state.rs` (lines 93-96: `retry_backoff_until`, `last_failure_kind`).
   Wire `classify_error()` into the retry decision:

   ```rust
   // In runner/event_loop.rs handle_agent_error:
   let enriched = roko_learn::error_enrichment::classify_error(&raw_error);
   let retry_action = match enriched.class {
       ErrorClass::Transient => RetryAction::Backoff(enriched.suggested_delay),
       ErrorClass::Permanent => RetryAction::Fail,
       ErrorClass::Degraded => RetryAction::DegradeAndRetry,
   };
   ```

   **Files to change:** `crates/roko-cli/src/runner/agent_events.rs`

3. `verdict_scorer.rs` (621 LOC) -- The runner already calls gate dispatch
   and records results. Add scoring as a post-gate hook:

   **Files to change:** `crates/roko-cli/src/runner/gate_dispatch.rs`

**Bucket B -- Defer (useful but needs design work):**

- `heartbeat_probes.rs` -- Needs agent process handle plumbing
- `energy.rs` -- Needs backpressure mechanism in executor
- `active_inference.rs`, `bayesian_confidence.rs`, `calibration_policy.rs` -- Need CascadeRouter integration points
- `task_scheduler.rs` -- Needs DAG executor changes

**Bucket C -- Evaluate for deletion:**

- `heartbeat_attention.rs` (2,146 LOC) -- Defines its own `ContextBidder`
  trait that conflicts with `roko-compose`'s (see Anti-Pattern #5).
  The compose version is the one actually used. This module's bidder
  system would need to be reconciled before wiring.
- `theta_consumer.rs`, `demurrage_consumer.rs`, `delta_consumer.rs` --
  These implement a pub/sub consumer model that the v2 runner does not use.
  Either adapt them to the select-loop model or delete them.

### Migration path

1. Add `#[cfg(test)]` to all NOT WIRED modules so they compile only in test
   (prevents accidental coupling while triaging).
2. Wire Bucket A modules one at a time, with integration tests proving the
   runtime path works.
3. For Bucket B, file issues with concrete integration designs.
4. For Bucket C, move to a `deprecated/` directory or delete after 30 days.

### Prevention strategy

- **Compile-time wiring check:** Add a `#[cfg_attr(not(test), deprecated)]`
  attribute to any module without a runtime caller. This makes accidental
  use produce warnings and intentional use require acknowledgment.
- **CI gate:** A script that greps for `STATUS: NOT WIRED` and fails if the
  count increases from the previous commit.
- **Architectural rule:** Every new module must have a runtime integration
  test that proves it is called from a live code path (not just unit tests).

### Impact on self-hosting if left unfixed

The learning pipeline operates without event-driven feedback. Gate verdicts
are not scored for trends. Errors are not classified for smart retry. Agent
health is not monitored. This means: roko can execute plans but cannot
learn from them, detect degradation, or self-correct -- defeating the
"develops itself" goal.

### Estimated effort

- Bucket A (3 modules): ~3 days
- Bucket B (5 modules): ~5 days design + 5 days implementation
- Bucket C triage: ~1 day
- CI gate: ~0.5 days

---

## 2. Gate Pipeline is a 2-Rung Facade

### Severity: CRITICAL

### Concrete evidence

`crates/roko-gate/src/rung_dispatch.rs` has 10 `stub_verdict()` calls across
rungs 3-7. The `stub_verdict` function (line 290) creates a `Verdict::pass()`
-- meaning stubs always pass silently:

```rust
// crates/roko-gate/src/rung_dispatch.rs:290
fn stub_verdict(gate: &str, detail: impl Into<String>) -> Verdict {
    let message = format!("stub gate; {}", detail.into());
    let mut verdict = Verdict::pass(gate.to_string());
    verdict.reason.clone_from(&message);
    verdict.detail = Some(message);
    verdict
}
```

Stub sites:

| Rung | Gate | Line | Guard condition |
|------|------|------|-----------------|
| 3 | `symbol` | 304 | `inputs.symbol_signal.is_none()` |
| 3 | `symbol` | 307 | `config.source_roots.is_none()` |
| 3 | `generated_test:cargo` | 331 | `config.generated_test_artifacts.is_none()` |
| 4 | `verify_chain` | 346 | no verify script tag AND no fallback |
| 5 | `fact_check` | 366 | `inputs.fact_check_signal.is_none()` |
| 5 | `fact_check` | 369 | `config.fact_check_oracle.is_none()` |
| 6 | `llm_judge` | 397 | `inputs.llm_judge_signal.is_none()` |
| 6 | `llm_judge` | 400 | `config.llm_judge_oracle.is_none()` |
| 6 | `integration:build_test` | 414 | `config.integration_test_pattern.is_none()` |

The actual runtime never provides `symbol_signal`, `fact_check_signal`,
`llm_judge_signal`, `generated_test_artifacts`, `fact_check_oracle`,
`llm_judge_oracle`, or `integration_test_pattern` in the
`RungExecutionInputs` / `RungExecutionConfig` structs. This means rungs 3-7
always stub.

**Effective pipeline**: compile (rung 1) -> clippy/test (rung 2) ->
`stub_verdict("pass")` for everything else.

### Root cause analysis

The gate infrastructure was built before the dispatch layer that would
populate its inputs. The `RungExecutionInputs` struct requires data that
only the orchestrator can provide (symbol manifests from parsing agent
output, fact-check content from the task prompt, etc.), but the orchestrator
only populates `base_signal` and `ctx`. The disconnect: **gate inputs
require orchestrator awareness of gate needs**, but the orchestrator treats
gates as a black box.

### Architectural solution

**Phase 1: Wire the inputs the gates already accept.**

The gates are fully implemented -- they just need populated inputs. Create a
`GateInputBuilder` that the orchestrator calls after agent dispatch:

```rust
// crates/roko-gate/src/input_builder.rs (new file)

pub struct GateInputBuilder {
    workdir: PathBuf,
    source_roots: Vec<PathBuf>,
}

impl GateInputBuilder {
    /// Build rung inputs from agent output.
    pub fn build(
        &self,
        agent_output: &str,
        task_prompt: &str,
        diff: &str,
    ) -> RungExecutionInputs {
        RungExecutionInputs {
            // Rung 3: Parse agent output for declared symbols
            symbol_signal: self.extract_symbol_manifest(agent_output, diff),
            // Rung 5: Use task prompt as fact-check content
            fact_check_signal: Some(self.build_fact_check_signal(task_prompt, agent_output)),
            // Rung 6: Use diff + prompt as LLM judge payload
            llm_judge_signal: Some(self.build_judge_signal(diff, task_prompt)),
            code_intel_hints: vec![],
        }
    }
}
```

**Phase 2: Wire `RungExecutionConfig` population.**

The config needs oracles (LLM endpoints for judge/fact-check). These can
use the existing `ModelCallService` or `PerplexitySearchClient`:

```rust
// In runner/gate_dispatch.rs:
let config = RungExecutionConfig {
    source_roots: Some(vec![workdir.clone()]),
    generated_test_artifacts: extract_test_artifacts(agent_output),
    fact_check_oracle: Some(Arc::new(LlmFactCheckOracle::new(model_call_service.clone()))),
    llm_judge_oracle: Some(Arc::new(LlmJudgeOracle::new(model_call_service.clone()))),
    integration_test_pattern: task.integration_pattern.clone(),
    ..Default::default()
};
```

**Phase 3: Change stub_verdict to fail, not pass.**

```rust
fn stub_verdict(gate: &str, detail: impl Into<String>) -> Verdict {
    // CHANGED: stubs are now SKIP (not pass), so they don't inflate the pass count
    Verdict::skip(gate.to_string(), detail.into())
}
```

This requires adding a `Verdict::skip()` variant, which is a one-line
addition to the Verdict enum.

### Files to change

1. `crates/roko-gate/src/rung_dispatch.rs` -- Change stub_verdict to skip
2. `crates/roko-gate/src/input_builder.rs` -- New: build inputs from agent output
3. `crates/roko-cli/src/runner/gate_dispatch.rs` -- Populate inputs/config
4. `crates/roko-gate/src/lib.rs` -- Add `Verdict::Skip` variant

### Prevention strategy

- **Invariant:** Any `Verdict::pass()` must come from actual gate execution,
  never from a stub. The `Verdict` type should enforce this via a
  `source: VerdictSource` field (`Executed | Stubbed | Skipped`).
- **Metrics:** Track stub ratio per run. If >50% of verdicts are stubs,
  warn in the TUI and CLI summary.

### Impact on self-hosting if left unfixed

Agent output is validated only by "does it compile?" and "do existing tests
pass?" There is no symbol manifest checking, no generated test validation,
no fact checking, no LLM judge, and no integration testing. This means an
agent can claim to implement a function, pass compilation by adding a stub,
and the gate pipeline will approve it.

### Estimated effort

- Phase 1 (input builder): ~2 days
- Phase 2 (config wiring + oracles): ~3 days
- Phase 3 (stub -> skip): ~0.5 days

---

## 3. Excessive .unwrap() / .expect() in Production

### Severity: HIGH

### Concrete evidence

**Counts** (non-test source files in `crates/`):

- `.unwrap()`: **3,215 instances**
- `.expect()`: **3,937 instances**
- `#[allow(clippy::unwrap_used)]`: 34 module-scope suppressions

**Top 10 files by .unwrap() count:**

| File | Count | Risk |
|------|-------|------|
| `roko-cli/src/main.rs` | 123 | CLI entry point -- any panic = user-facing crash |
| `roko-learn/src/skill_library.rs` | 100 | Learning persistence -- crash = lost learning data |
| `roko-learn/src/runtime_feedback.rs` | 91 | Feedback loop -- crash = learning regression |
| `roko-cli/src/config.rs` | 84 | Config loading -- crash on malformed TOML |
| `roko-fs/src/file_substrate.rs` | 81 | Signal storage -- crash on disk I/O issues |
| `roko-cli/src/prd.rs` | 59 | PRD commands -- crash on bad YAML |
| `roko-cli/src/orchestrate.rs` | 59 | Plan execution -- crash mid-run |
| `roko-orchestrator/src/worktree.rs` | 56 | Git operations -- crash on git failures |
| `roko-orchestrator/src/dag.rs` | 56 | DAG execution -- crash on edge cases |
| `roko-compose/src/prompt.rs` | 50 | Prompt assembly -- crash on malformed input |

**Top 10 files by .expect() count:**

| File | Count | Risk |
|------|-------|------|
| `roko-neuro/src/knowledge_store.rs` | 134 | Durable knowledge -- crash = knowledge corruption |
| `roko-cli/src/tui/dashboard.rs` | 125 | TUI rendering -- crash = dashboard dead |
| `roko-learn/src/playbook.rs` | 100 | Playbook persistence -- crash = lost playbooks |
| `roko-cli/src/orchestrate.rs` | 89 | Orchestrator -- crash mid-plan |
| `roko-mcp-github/src/main.rs` | 78 | MCP server -- crash = tool unavailable |
| `roko-learn/src/playbook_rules.rs` | 75 | Playbook rules -- crash = rule regression |
| `roko-learn/src/episode_logger.rs` | 66 | Episode logging -- crash = lost episodes |
| `roko-agent/src/provider/openai_compat.rs` | 66 | LLM provider -- crash = agent dead |
| `roko-serve/src/routes/providers.rs` | 62 | HTTP routes -- crash = 500 error |
| `roko-cli/src/repo_context.rs` | 62 | Context building -- crash = no context |

**Total: 7,152 potential panic sites** in production code.

### Root cause analysis

Three contributing factors:

1. **Rapid agent-driven development.** Claude agents generating code
   default to `.unwrap()` for brevity. With 15+ modules built in parallel,
   nobody reviewed for error handling.

2. **Blanket lint suppression.** `roko-cli/src/lib.rs` suppresses all clippy
   lints including `clippy::unwrap_used` (see Anti-Pattern #6), so clippy
   cannot catch new additions.

3. **No error type per crate.** Many crates lack a unified error enum,
   making proper error propagation verbose. Developers reach for `.unwrap()`
   because the alternative is boilerplate.

### Architectural solution

**Do not attempt to fix all 7,152 at once.** Prioritize by blast radius.

**Phase 1: Define per-crate error types for the 5 worst offenders.**

Each crate needs a `Error` enum that wraps its failure modes:

```rust
// crates/roko-learn/src/error.rs
#[derive(Debug, thiserror::Error)]
pub enum LearnError {
    #[error("playbook I/O: {0}")]
    PlaybookIo(#[from] std::io::Error),
    #[error("skill library parse: {0}")]
    SkillParse(#[from] serde_json::Error),
    #[error("episode log: {0}")]
    EpisodeLog(String),
    #[error("feedback: {0}")]
    Feedback(String),
}
```

Priority order for crate error types:
1. `roko-learn` (4 files, 357 unwraps) -- learning data loss
2. `roko-neuro` (1 file, 134 expects) -- knowledge corruption
3. `roko-fs` (1 file, 81 unwraps) -- data storage
4. `roko-orchestrator` (2 files, 112 unwraps) -- execution failure
5. `roko-compose` (1 file, 50 unwraps) -- prompt assembly

**Phase 2: Mechanical replacement in each priority file.**

Pattern replacements:

```rust
// BEFORE:
let data = serde_json::from_str(&text).unwrap();

// AFTER:
let data = serde_json::from_str(&text)
    .map_err(|e| LearnError::SkillParse(e))?;
```

For methods that currently return `()` or a concrete type:

```rust
// BEFORE:
pub fn persist(&self) {
    let json = serde_json::to_string(&self.data).unwrap();
    std::fs::write(&self.path, json).unwrap();
}

// AFTER:
pub fn persist(&self) -> Result<(), LearnError> {
    let json = serde_json::to_string(&self.data)?;
    std::fs::write(&self.path, &json)?;
    Ok(())
}
```

**Phase 3: Enable `clippy::unwrap_used` lint per crate.**

After each crate is cleaned up, add to its `lib.rs`:
```rust
#![deny(clippy::unwrap_used)]
```

This prevents regression.

### Files to change (Phase 1 priority)

1. `crates/roko-learn/src/error.rs` (new)
2. `crates/roko-learn/src/skill_library.rs` (100 unwraps -> Result)
3. `crates/roko-learn/src/runtime_feedback.rs` (91 unwraps -> Result)
4. `crates/roko-learn/src/playbook.rs` (100 expects -> Result)
5. `crates/roko-learn/src/episode_logger.rs` (66 expects -> Result)
6. `crates/roko-neuro/src/knowledge_store.rs` (134 expects -> Result)
7. `crates/roko-fs/src/file_substrate.rs` (81 unwraps -> Result)

### Prevention strategy

- **Lint configuration:** Enable `clippy::unwrap_used` as `deny` in each
  cleaned crate. Enable `clippy::expect_used` as `warn`.
- **CLAUDE.md rule:** Add "Never use `.unwrap()` in production code. Use `?`
  with the crate's error type."
- **CI diff check:** A pre-commit hook that counts `.unwrap()` in changed
  files and warns if new instances are added.

### Impact on self-hosting if left unfixed

Any `.unwrap()` failure during `plan run` kills the entire process. Given
`PlanRunner` holds 77 fields of in-flight state (see Anti-Pattern #4), a
crash loses all accumulated learning, routing feedback, and execution
progress since the last snapshot. During a multi-hour self-hosting run, this
means potentially hours of wasted compute and API costs.

### Estimated effort

- Phase 1 (error types for 5 crates): ~2 days
- Phase 2 (mechanical replacement for top 7 files): ~3 days
- Phase 3 (lint enablement): ~0.5 days per crate

---

## 4. God Structs

### Severity: HIGH

### Concrete evidence

| Struct | Fields | File | File LOC |
|--------|--------|------|----------|
| `TuiState` | 131 | `crates/roko-cli/src/tui/state.rs:986` | 4,955 |
| `PlanRunner` | 80 | `crates/roko-cli/src/orchestrate.rs:2633` | 23,269 |
| `AppState` | 52 | `crates/roko-serve/src/state.rs:344` | 1,317 |
| `RunState` | 38 | `crates/roko-cli/src/runner/state.rs:22` | ~200 |

**TuiState (131 fields)** is organized into 16 sections (core state,
navigation, animation, input, scroll positions, approval, git, cost/tokens,
system metrics, timing, config editor, push-path state, view data). Every
widget function takes `&TuiState` or `&mut TuiState` and can read/write any
of the 131 fields.

Example field categories in TuiState:
- 8 scroll positions (`agent_scroll`, `diff_scroll`, `task_scroll`, ...)
- 7 git fields (`git_branch`, `git_commit_short`, `git_age`, ...)
- 8 cost/token fields (`cost_per_plan`, `token_total`, `token_rate`, ...)
- 6 navigation indices (`selected_plan_idx`, `selected_agent`, ...)
- 5 config editor fields (`config_cursor`, `config_pending`, ...)

**PlanRunner (80 fields)** in `orchestrate.rs` holds everything from
learning runtime to chain clients to MCP state. Key field groups:

- Learning/feedback: `learning`, `skill_library`, `playbook`,
  `knowledge_store`, `feedback_service`, `efficiency_events`, ...
- Execution tracking: `executor`, `event_log`, `task_trackers`,
  `per_plan_agents`, `per_plan_gates`, ...
- Infrastructure: `supervisor`, `cancel`, `conductor`, `health_monitor`,
  `metrics`, `obs_sinks`, `health_probes`, ...
- Routing: `latency_registry`, `router_calibration`, `attribution_tracker`,
  `crate_familiarity_tracker`, `format_bandit`, ...
- Context: `code_index_cache`, `search_client`, `extension_chain`, ...

**AppState (52 fields)** has a documented lock acquisition order spanning
17 numbered locks, indicating the concurrency complexity:

```rust
// crates/roko-serve/src/state.rs:381-392
//  1. active_runs          7. discovered_agents     13. cascade_router
//  2. active_plans         8. aggregator_cache      14. gateway_model_counters
//  3. operations           9. heartbeats            15. batch_progress
//  4. templates           10. connectors            16. active_bench_runs
//  5. deployments         11. feeds                 17. active_matrix_runs
//  6. template_runs       12. ephemeral_workspaces
```

### Root cause analysis

**Incremental accretion.** Each new feature adds 1-3 fields to PlanRunner
or TuiState rather than creating a focused sub-struct. This is the natural
path of least resistance when a struct already exists and every method
takes `&mut self`.

**No module boundary enforcement.** Rust's visibility rules allow any method
on `PlanRunner` to access all 80 fields. There is no architectural boundary
between "learning" fields and "execution" fields.

Note: the v2 runner (`crates/roko-cli/src/runner/`) has already partially
solved this by decomposing into `RunState` (38 fields), plus separate
modules for gate dispatch, agent events, persistence, etc. This is the
right direction. The problem is that `PlanRunner` (the legacy runner) and
`TuiState` have not been decomposed.

### Architectural solution

**For TuiState: Decompose into domain-specific sub-states.**

```rust
// Target structure:
pub struct TuiState {
    pub orchestrator: OrchestratorView,   // plans, phases, waves, tasks
    pub agents: AgentView,                // roster, topology, streams, output
    pub navigation: NavigationState,      // tabs, focus, scroll positions
    pub input: InputState,                // mode, buffers, filter
    pub git: GitView,                     // branch, commits, worktrees
    pub costs: CostView,                  // per-plan, per-task, rates
    pub learning: LearningView,           // efficiency, experiments, trends
    pub system: SystemView,               // metrics, timing, process info
    pub modals: ModalState,               // approvals, confirms, overlays
    pub config_editor: ConfigEditorState, // cursor, pending edits
}
```

Each sub-state contains 8-15 fields. Widget functions take only the
sub-state they need:

```rust
// BEFORE:
fn render_agent_pool(state: &TuiState, area: Rect, buf: &mut Buffer) { ... }

// AFTER:
fn render_agent_pool(agents: &AgentView, nav: &NavigationState, area: Rect, buf: &mut Buffer) { ... }
```

**For PlanRunner: Already superseded by v2 runner.**

The v2 runner in `crates/roko-cli/src/runner/` already decomposes the
monolith into focused modules: `state.rs` (RunState), `gate_dispatch.rs`,
`agent_events.rs`, `persist.rs`, etc. The solution is to complete the
migration from `orchestrate.rs` to the runner module, then delete PlanRunner.

If PlanRunner must remain (for the `legacy-orchestrate` feature flag):
- Extract learning fields into `LearningRuntime` (already partially done)
- Extract gate fields into `GateRuntime`
- Extract routing fields into `RoutingRuntime`
- PlanRunner holds `Arc<LearningRuntime>`, `Arc<GateRuntime>`, etc.

**For AppState: Group by access pattern.**

AppState's 17-lock ordering comment reveals natural groupings:

```rust
pub struct AppState {
    pub core: AppCore,             // workdir, layout, cancel, metrics, started_at
    pub agents: AgentRegistry,     // supervisor, discovered_agents, agent_count, templates
    pub execution: ExecutionState, // active_runs, active_plans, operations
    pub learning: LearningState,   // cascade_router, event_bus, state_hub
    pub infra: InfraState,         // deploy_backend, deployments, http_client, scrubber
    pub chain: ChainState,         // chain_client, chain_wallet, connectors
    pub config: ConfigState,       // roko_config, provider_health, latency_registry
}
```

### Migration path

1. **TuiState:** Create sub-state structs alongside existing fields (both
   can coexist). Migrate one widget at a time to use sub-states. Delete
   flattened fields when all consumers migrate.
2. **PlanRunner:** Complete migration to v2 runner. Track remaining
   orchestrate.rs callers via `#[deprecated]`.
3. **AppState:** Group fields into sub-structs. Route handlers already
   receive `Arc<AppState>`, so sub-struct access is just
   `state.execution.active_runs` instead of `state.active_runs`.

### Prevention strategy

- **Struct field budget:** Any struct exceeding 20 fields triggers a review
  requirement. Add a `// FIELDS: N` comment that CI checks.
- **CLAUDE.md rule:** "New features must use sub-structs. Never add fields
  to TuiState, PlanRunner, or AppState directly."

### Impact on self-hosting if left unfixed

God structs make refactoring extremely risky. Every change to PlanRunner's
80 fields potentially affects dozens of methods across 23,000 lines. This
means: agents modifying orchestrate.rs (which is the self-hosting bottleneck)
will frequently introduce subtle bugs because they cannot reason about which
fields interact with which methods.

### Estimated effort

- TuiState decomposition: ~5 days (131 fields -> 10 sub-states)
- PlanRunner -> v2 runner migration: ~8 days (23K LOC file)
- AppState decomposition: ~3 days (52 fields -> 7 sub-states)

---

## 5. Duplicate Systems

### Severity: MEDIUM

### Concrete evidence

**Config loading (10+ entry points in crates/):**

| Function | File:Line | What it does |
|----------|-----------|--------------|
| `load_config_unified()` | `roko-core/src/config/loader.rs:105` | Canonical: global merge + env overrides |
| `load_config_with_options()` | `roko-core/src/config/loader.rs:110` | Canonical: custom options |
| `load_config_file()` | `roko-core/src/config/loader.rs:123` | From explicit path |
| `load_config_validated()` | `roko-core/src/config/loader.rs:130` | With provenance tracking |
| `load_config_validated_with_options()` | `roko-core/src/config/loader.rs:139` | Provenance + custom options |
| `load_config()` | `roko-core/src/config/mod.rs:118` | Wrapper -> validated |
| `load_config_strict()` | `roko-core/src/config/mod.rs:131` | Strict mode wrapper |
| `load_roko_config()` | `roko-cli/src/orchestrate.rs:863` | OnceLock-cached wrapper |
| `load_roko_config_models()` | `roko-cli/src/run.rs:3048` | Inline, extracts only model list |
| `load_config_or_defaults()` | `roko-cli/src/unified.rs:250` | Returns defaults on missing file |
| `load_roko_config()` | `roko-serve/src/state.rs:783` | ArcSwap-cached read |
| `load_roko_config_with_warning()` | `roko-acp/src/config.rs:120` | Warning on missing file |
| `load_roko_config()` | `roko-acp/src/config.rs:209` | Another ACP-specific loader |
| `load_roko_config_file()` | `roko-cli/src/serve_runtime.rs:490` | Yet another loader |

5 are in `roko-core` (the canonical location), 9 are scattered across
`roko-cli`, `roko-serve`, and `roko-acp`. Each downstream loader adds its
own caching, error handling, or fallback behavior.

**ContextBidder trait (2 incompatible definitions):**

1. `roko-compose/src/context_provider.rs:682`:
   ```rust
   pub trait ContextBidder: Send + Sync {
       fn bidder_id(&self) -> &str;
       fn propose_context(&self, ...) -> Vec<ContextCandidate>;
   }
   // ContextCandidate has `relevance: f32`
   ```
   4 concrete implementations: `TaskRequirementsBidder`,
   `DocsSourceMapBidder`, `RecentFailurePatternsBidder`,
   `RolePromptPolicyBidder`. **Actually used** by the runtime.

2. `roko-runtime/src/heartbeat_attention.rs:665`:
   ```rust
   pub trait ContextBidder: Send + Sync {
       fn generate_candidates(&self, ctx: &BidderContext) -> Vec<ContextCandidate>;
       fn subsystem_id(&self) -> &str { ... }
   }
   // ContextCandidate has `expected_value: f64`
   ```
   2 concrete implementations: `NeuroBidder`, `DaimonBidder`.
   **NOT WIRED** (this entire module is STATUS: NOT WIRED).

These two traits have different method names, different return types, and
different candidate scoring fields (`relevance: f32` vs `expected_value: f64`).

**RetryPolicy (2 independent implementations):**

1. `roko-core/src/error/retry.rs:27`:
   ```rust
   pub struct RetryPolicy {
       max_attempts: u32,
       base_delay_ms: u64,
       max_delay_ms: u64,
       jitter: bool,  // deterministic jitter (no rand dependency)
   }
   ```
   Has `should_retry()` and `delay_for_attempt()` methods. **Zero runtime
   callers** -- nobody uses this implementation.

2. `roko-agent/src/retry.rs:45`:
   ```rust
   pub struct RetryPolicy {
       pub max_attempts: u32,
       pub base_delay_ms: u64,
       pub max_delay_ms: u64,
       pub retryable_errors: Vec<ErrorClass>,  // error-class-aware
   }
   ```
   Has `ErrorClass` enum with 8 variants mapping from `ProviderError`.
   **Actually used** by the agent dispatch layer.

### Root cause analysis

**Parallel development without coordination.** When agents build modules
in isolation (one builds `roko-core/error/retry.rs`, another builds
`roko-agent/retry.rs`), they create duplicate abstractions. The CLAUDE.md
rule "search before writing" is insufficient because agents search for
*exact names* but not *semantic equivalents*.

**No canonical module registry.** There is no single place that lists "the
retry policy is at X" or "config loading goes through Y". Each crate
independently decides where its abstractions live.

### Architectural solution

**Config loading: Converge on `load_config_unified()` as the single entry.**

All downstream callers should call `load_config_unified()` or
`load_config_with_options()`. Remove the 9 scattered wrappers:

```rust
// BEFORE (roko-cli/src/orchestrate.rs:863):
fn load_roko_config(workdir: &Path) -> Result<RokoConfig> {
    static CFG: OnceLock<RokoConfig> = OnceLock::new();
    // ... custom caching logic ...
}

// AFTER:
// Just call roko_core::config::load_config_unified(workdir)
// Caching belongs at the call site, not in a wrapper function.
```

For call sites that need caching, use `ArcSwap<RokoConfig>` (already done
in `roko-serve`). For call sites that need fallback-on-missing, use
`LoadOptions { allow_missing: true }`.

**ContextBidder: Delete the NOT WIRED version.**

Since `heartbeat_attention.rs` is STATUS: NOT WIRED (Anti-Pattern #1), its
`ContextBidder` trait should be deleted. If the `NeuroBidder` and
`DaimonBidder` concepts are valuable, reimplement them against
`roko-compose`'s trait.

**RetryPolicy: Delete the unused version.**

`roko-core/src/error/retry.rs` has zero runtime callers. Delete it. The
`roko-agent/src/retry.rs` version is the canonical one.

### Files to change

**Config consolidation:**
1. Delete `roko-cli/src/orchestrate.rs:863` (`load_roko_config`)
2. Delete `roko-cli/src/run.rs:3048` (`load_roko_config_models`)
3. Delete `roko-cli/src/unified.rs:250` (`load_config_or_defaults`)
4. Delete `roko-cli/src/serve_runtime.rs:490` (`load_roko_config_file`)
5. Simplify `roko-acp/src/config.rs:120,209`
6. Update all call sites to use `roko_core::config::load_config_unified()`

**Trait dedup:**
1. Delete `roko-runtime/src/heartbeat_attention.rs:665-730` (ContextBidder)
2. Delete `roko-core/src/error/retry.rs` (RetryPolicy)

### Prevention strategy

- **Canonical module index:** Add a `MODULES.md` to the repo root listing
  the canonical location for every cross-cutting concern (config loading,
  retry, context bidding, error types, etc.).
- **CLAUDE.md rule:** "Before creating a new trait or struct, search for
  existing implementations with similar purpose, not just matching names."
- **CI check:** Lint for multiple definitions of the same trait name across
  crates.

### Impact on self-hosting if left unfixed

Duplicate config loaders mean config changes may not propagate uniformly.
An agent updating config loading behavior must find and update all 14 entry
points -- or introduce silent inconsistency.

### Estimated effort

- Config consolidation: ~2 days
- Trait/struct dedup: ~1 day
- MODULES.md: ~0.5 days

---

## 6. Blanket Lint Suppression

### Severity: MEDIUM

### Concrete evidence

`crates/roko-cli/src/lib.rs` (the primary source crate, ~90% of CLI code):

```rust
// crates/roko-cli/src/lib.rs:6-23
#![allow(dead_code, unused_imports, unused_variables)]
#![allow(clippy::module_name_repetitions)]
#![allow(missing_docs)]
#![cfg_attr(
    clippy,
    allow(
        clippy::all,
        clippy::pedantic,
        clippy::nursery,
        clippy::restriction,
        missing_docs
    )
)]
```

This disables **every clippy lint category**: `all` (default lints),
`pedantic` (style lints), `nursery` (experimental lints), and
`restriction` (opt-in strictness lints). Combined with the `dead_code` and
`unused_imports` allows, this means the compiler and linter cannot catch:

- Dead code accumulation
- Unused imports (stale dependencies)
- Unused variables (logic errors)
- Every clippy warning and error

Additionally, **15 command modules** each have their own suppression:

```rust
// Every file in crates/roko-cli/src/commands/*.rs:
#![allow(unused_imports)]
```

Files: `bench.rs`, `dashboard.rs`, `prd.rs`, `learn.rs`, `tune.rs`,
`agent.rs`, `knowledge.rs`, `plan.rs`, `server.rs`, `auth.rs`,
`research.rs`, `config_cmd.rs`, `util.rs`, `job.rs`, `think.rs`

Other crates with broad suppressions:

| Crate | File | Suppression |
|-------|------|-------------|
| `roko-compose` | `lib.rs:17-18` | `clippy::pedantic`, `clippy::nursery` |
| `roko-dreams` | `phase2/*.rs` (4 files) | `dead_code` |
| `roko-learn` | `routing_extras.rs:6` | `dead_code` |

### Root cause analysis

These suppressions were added to unblock rapid development. When agents
produce code with hundreds of warnings, suppressing them all at the crate
level is faster than fixing each one. Once suppressed, the warnings become
invisible and accumulate.

The `cfg_attr(clippy, allow(clippy::all, ...))` in `lib.rs` is particularly
harmful because it uses `cfg_attr(clippy, ...)` which activates only when
clippy runs -- meaning `cargo clippy` is silenced specifically when it
should be reporting.

### Architectural solution

**Phase 1: Remove the nuclear suppression from `roko-cli/src/lib.rs`.**

```rust
// REMOVE these lines entirely:
// #![allow(dead_code, unused_imports, unused_variables)]
// #![cfg_attr(clippy, allow(clippy::all, ...))]

// KEEP only intentional, documented suppressions:
#![allow(clippy::module_name_repetitions)]  // Roko naming convention
```

This will produce thousands of warnings. Do not try to fix them all.
Instead:

**Phase 2: Move suppressions to individual items.**

For each warning, add `#[allow(...)]` on the specific item with a comment
explaining why:

```rust
#[allow(dead_code)]  // Used by legacy-orchestrate feature
fn dispatch_legacy(...) { ... }

#[allow(unused_imports)]  // Re-export for downstream crates
pub use roko_core::Signal;
```

**Phase 3: Fix or delete the actual dead code.**

The `dead_code` warnings will reveal which code is truly unused. Delete it
(see Anti-Pattern #1 for the NOT WIRED modules) or gate it behind features
(see Anti-Pattern #7).

**Phase 4: Remove per-command `#![allow(unused_imports)]`.**

In each `commands/*.rs` file, delete the blanket allow and fix the handful
of actual unused imports.

### Files to change

1. `crates/roko-cli/src/lib.rs` -- Remove blanket suppressions
2. 15 files in `crates/roko-cli/src/commands/` -- Remove `unused_imports`
3. `crates/roko-compose/src/lib.rs` -- Remove pedantic/nursery suppression
4. 4 files in `crates/roko-dreams/src/phase2/` -- Remove or scope `dead_code`
5. `crates/roko-learn/src/routing_extras.rs` -- Remove `dead_code`

### Prevention strategy

- **CI rule:** Fail CI if any `#![allow(clippy::all)]` or
  `#![allow(clippy::pedantic)]` appears at crate root scope.
- **CLAUDE.md rule:** "Never add crate-level lint suppressions. Use
  item-level `#[allow(...)]` with a comment explaining the reason."
- **Periodic lint audit:** Run `cargo clippy` with no suppressions quarterly
  and triage new warnings.

### Impact on self-hosting if left unfixed

Clippy cannot detect bugs in the CLI crate. Every `cargo clippy --workspace
-- -D warnings` check passes vacuously for `roko-cli`, which contains
~90% of the runtime code. Agents making changes to the orchestrator,
runner, or TUI get zero static analysis feedback. This makes the
self-hosting loop's code quality gate meaningless for its most critical crate.

### Estimated effort

- Phase 1 (remove nuclear suppression): ~0.5 days
- Phase 2 (move to item-level): ~3-5 days (depending on warning count)
- Phase 3 (fix/delete dead code): ~2-3 days
- Phase 4 (command files): ~0.5 days

---

## 7. Feature Flag as Dead Code Marker

### Severity: LOW-MEDIUM

### Concrete evidence

**`legacy-orchestrate` feature in `roko-cli`:**

`crates/roko-cli/Cargo.toml:13-20`:
```toml
[features]
default = []
legacy-orchestrate = ["legacy-direct-dispatch"]
legacy-direct-dispatch = []
```

41 `#[cfg(feature = "legacy-orchestrate")]` or
`#[cfg(feature = "legacy-direct-dispatch")]` guards across CLI src files.

`crates/roko-cli/src/run.rs` is 3,662 lines. The majority of this file is
gated behind `legacy-orchestrate`, which is not in `default`. This means
the entire file is dead code under a default build. However, the file is
still compiled (feature checks happen at the item level, not file level),
still maintained, and still contributes to compile times.

**`hdc` feature across 5 crates:**

| Crate | Guard Count | What it gates |
|-------|-------------|---------------|
| `roko-neuro` | 29 | HDC fingerprint computation in KnowledgeStore |
| `roko-compose` | gated via `roko-neuro/hdc` | HDC in context assembly |
| `roko-serve` | gated via dep | HDC in HTTP routes |
| `roko-fs` | gated via dep | HDC in signal storage |
| `roko-index` | 1 | HDC in code index |

The `hdc` feature is the neuro store's key differentiator -- without it,
knowledge entries lack similarity-based retrieval. But it is off by default,
meaning the default build cannot do knowledge-aware routing.

**Other feature flags:**

| Feature | Crate | Default? | Impact of being off |
|---------|-------|----------|---------------------|
| `alloy-backend` | `roko-chain` | No | No on-chain operations |
| `tree-sitter` | `roko-lang-rust` | No | No AST-based code analysis |
| `sqlite` | `roko-index` | No | No persistent index |
| `rkyv` | `roko-core`, `roko-primitives`, `roko-index` | No | No zero-copy deserialization |
| `integration` | `roko-agent` | No | Integration tests skipped |

### Root cause analysis

Feature flags are being used for two distinct purposes:

1. **Legitimate conditional compilation:** `alloy-backend` pulls in heavy
   blockchain dependencies; `tree-sitter` requires native C libraries.
   These should be optional.

2. **Dead code preservation:** `legacy-orchestrate` keeps the old runner
   alive "just in case." `hdc` keeps the differentiating feature disabled
   by default. These are not real feature flags -- they are a way to avoid
   deleting or committing to code.

### Architectural solution

**Audit each feature flag for purpose:**

| Feature | Action | Rationale |
|---------|--------|-----------|
| `legacy-orchestrate` | **Remove** | v2 runner is production. Delete `run.rs` or inline what is still needed. |
| `legacy-direct-dispatch` | **Remove** | Same -- migrate remaining callers to `ModelCallService`. |
| `hdc` | **Enable by default** | This is core functionality, not optional. |
| `alloy-backend` | **Keep optional** | Heavy dep, only needed with chain integration. |
| `tree-sitter` | **Keep optional** | Native C dep, reasonable to gate. |
| `sqlite` | **Keep optional** | Storage backend choice. |
| `rkyv` | **Keep optional** | Performance optimization, not required. |
| `integration` | **Keep** | Test-only flag, standard practice. |

**For `legacy-orchestrate` removal:**

1. Identify any code in `run.rs` that is still needed by the v2 runner.
2. Extract those functions into appropriate runner modules.
3. Delete `run.rs` entirely.
4. Remove the feature from `Cargo.toml`.
5. Remove all 41 `#[cfg(feature = "legacy-orchestrate")]` guards.

**For `hdc` default enablement:**

```toml
# crates/roko-neuro/Cargo.toml
[features]
default = ["hdc"]
hdc = ["dep:roko-primitives"]
```

Then remove 29+ `#[cfg(feature = "hdc")]` guards from `knowledge_store.rs`
since the code is now always compiled.

### Files to change

**legacy-orchestrate removal:**
1. `crates/roko-cli/Cargo.toml` -- Remove features
2. `crates/roko-cli/src/run.rs` -- Delete or extract residual code
3. All files with `#[cfg(feature = "legacy` guards

**hdc default enablement:**
1. `crates/roko-neuro/Cargo.toml` -- Change default
2. `crates/roko-neuro/src/knowledge_store.rs` -- Remove 29 guards
3. `crates/roko-compose/Cargo.toml` -- Update dep
4. `crates/roko-fs/Cargo.toml` -- Update dep

### Prevention strategy

- **Feature flag policy:** Document in CLAUDE.md:
  - Features for **optional heavy deps** (native libs, blockchain, databases): OK
  - Features for **dead code preservation**: Not OK -- delete the code or commit
  - Features for **test gating** (integration tests): OK
- **CI check:** Warn on feature flags that are never enabled in CI.

### Impact on self-hosting if left unfixed

3,662 lines of `run.rs` contribute to compile times and cognitive load but
are never executed. New contributors (and agents) may accidentally modify
this dead code thinking it is active. The disabled `hdc` feature means
knowledge-informed routing (item #13 on the priority list) cannot work
without a non-default build configuration that nobody currently uses.

### Estimated effort

- `legacy-orchestrate` removal: ~2 days
- `hdc` default enablement: ~1 day
- Feature flag policy + CI check: ~0.5 days

---

## 8. Backwards Compatibility Shims Still Active

### Severity: LOW

### Concrete evidence

`crates/roko-core/src/config/compat.rs` (439 LOC) contains a full Mori-format
TOML migration reader. It defines a `MoriConfig` struct that deserializes
`.mori/config.toml` fields and converts them to `RokoConfig` via a
`from_mori_toml()` function.

The function is re-exported from `roko-core/src/config/mod.rs:38`:
```rust
pub use compat::from_mori_toml;
```

**However, it has zero runtime callers.** The search for `from_mori_toml`
outside of `compat.rs` itself shows only:
- The `pub use` re-export in `mod.rs`
- Test calls within `compat.rs` itself (10 test functions)

The config loader (`loader.rs`) does not call `from_mori_toml`. The
`find_config_path()` function searches for `roko.toml` and `ROKO_CONFIG`
env var -- it does not look for `.mori/config.toml`.

This file is the **only dirty file in `git status`** at the time of writing,
meaning it was being actively modified (for the `schema_version` field
changes in this branch).

### Root cause analysis

The compat layer was built during the mori-to-roko migration as a safety
net. The migration is complete (roko has its own config format, its own
data directory, and its own CLI). But the file persists because:

1. It has tests that pass, so nobody flags it for deletion.
2. It is `pub`, so removing it is a breaking API change (even though no
   external crate uses it).
3. It was being modified alongside schema changes for consistency.

### Architectural solution

**Delete it.**

Since `from_mori_toml` has zero runtime callers and the mori codebase lives
at a completely different path (`/Users/will/dev/uniswap/bardo/`), this
code serves no purpose.

Steps:

1. Remove `pub use compat::from_mori_toml;` from `config/mod.rs`.
2. Delete `crates/roko-core/src/config/compat.rs`.
3. Remove `mod compat;` from `config/mod.rs`.
4. Run `cargo test --workspace` to confirm nothing breaks.

If there is concern about preserving the migration logic for historical
reference, move it to `tmp/archive/compat.rs` (not compiled, not tested,
just documentation).

### Files to change

1. `crates/roko-core/src/config/mod.rs` -- Remove `mod compat` and `pub use`
2. `crates/roko-core/src/config/compat.rs` -- Delete

### Prevention strategy

- **Rule:** Migration shims get a `// TODO: remove after YYYY-MM-DD`
  comment with a 90-day expiry.
- **CI check:** Flag any `compat` or `migration` or `legacy` named module
  older than 90 days.

### Impact on self-hosting if left unfixed

Minimal. The file is 439 LOC of code that never runs. Its main cost is
cognitive -- it must be kept in sync with `RokoConfig` schema changes
(which is why it was the dirty file in `git status`), consuming maintenance
effort for zero runtime value.

### Estimated effort

- Deletion: ~0.5 days (including verification)

---

## Summary: Priority and Dependencies

| # | Anti-Pattern | Severity | Effort | Dependencies |
|---|-------------|----------|--------|-------------|
| 1 | NOT WIRED modules (8,246 LOC) | CRITICAL | ~10 days | None |
| 2 | Gate pipeline facade | CRITICAL | ~6 days | None |
| 3 | .unwrap()/.expect() density (7,152) | HIGH | ~8 days | #6 (lint suppression removal) |
| 4 | God structs (TuiState, PlanRunner, AppState) | HIGH | ~16 days | None |
| 5 | Duplicate systems (config, bidders, retry) | MEDIUM | ~4 days | None |
| 6 | Blanket lint suppression | MEDIUM | ~6 days | None |
| 7 | Feature flags as dead code markers | LOW-MEDIUM | ~4 days | None |
| 8 | Backwards compatibility shims | LOW | ~0.5 days | None |

**Recommended execution order:**

1. **#8 (compat shims)** -- Quick win, removes maintenance burden.
2. **#6 (lint suppression)** -- Unblocks #3 and reveals actual dead code.
3. **#5 (duplicate systems)** -- Reduces confusion for all subsequent work.
4. **#3 (unwrap density)** -- Now that lints catch regressions.
5. **#7 (feature flags)** -- Enable `hdc` default, delete legacy runner.
6. **#1 (NOT WIRED modules)** -- Wire Bucket A, triage rest.
7. **#2 (gate facade)** -- Wire gate inputs (depends on clean error handling).
8. **#4 (god structs)** -- Largest refactor, benefits from all prior cleanup.

**Total estimated effort: ~55 days** (can be parallelized to ~25 days with
2 agents working on independent tracks).
