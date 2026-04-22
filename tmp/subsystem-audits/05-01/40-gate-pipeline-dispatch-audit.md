# 40 -- Gate Pipeline & Agent Dispatch Audit

**Date**: 2026-05-01
**Scope**: `roko-gate` rung pipeline, `orchestrate.rs` gate execution, agent dispatch paths, adaptive thresholds
**Method**: Static code analysis of runtime call paths

---

## 1. Gate Rung Wiring

### 1.1 Rung definitions vs runtime execution

The 7-rung enum is defined at `crates/roko-gate/src/rung_selector.rs:96-117`:

| Rung | Index | Declared gates | Runtime status in orchestrate.rs |
|------|-------|----------------|----------------------------------|
| Compile | 0 | `CompileGate` | **REAL** -- `selected_gate_steps()` line 17281 |
| Lint | 1 | `ClippyGate` | **REAL** -- `selected_gate_steps()` line 17284 |
| Test | 2 | `TestGate` | **REAL** -- `selected_gate_steps()` line 17287 |
| Symbol | 3 | `SymbolGate` | **SILENTLY DROPPED** -- see finding 1.2 |
| GeneratedTest | 4 | `GeneratedTestGate` + `VerifyChainGate` | **CONDITIONAL** -- only if generated test store exists (line 17290) |
| PropertyTest | 5 | `PropertyTestGate` + `FactCheckGate` | **SILENTLY DROPPED** -- see finding 1.2 |
| Integration | 6 | `LlmJudgeGate` + `IntegrationGate` | **SILENTLY DROPPED** -- see finding 1.2 |

### 1.2 [CRITICAL] `_ =>` catch-all silently drops rungs 3, 5, 6

**File**: `crates/roko-cli/src/orchestrate.rs:17296-17298`

```rust
_ => {
    skipped_count = skipped_count.saturating_add(1);
}
```

The `selected_gate_steps()` method at line 17240 has a `match rung` block that only handles `Compile`, `Lint` (with cap guard), `Test`, and `GeneratedTest`. Everything else hits the `_ =>` catch-all and is silently counted as skipped. This means:

- **Rung 3 (Symbol)**: Even if `select_rungs()` includes it (Standard/Complex complexity), the orchestrator skips it. The `rung_dispatch` module has a real `SymbolGate` implementation, but `selected_gate_steps()` never instantiates it.
- **Rung 5 (PropertyTest)**: Same -- `PropertyTestGate` is fully implemented in `crates/roko-gate/src/property_test_gate.rs` with proptest integration, but the catch-all drops it.
- **Rung 6 (Integration)**: Same -- both `LlmJudgeGate` and `IntegrationGate` are implemented, `gate_rung_config()` even wires their oracles (lines 17491-17508), but `selected_gate_steps()` never creates them.

**Impact**: For the primary gate execution path (rung=0, which is what `run_gate_pipeline()` uses at line 16712), only rungs 0-2 and optionally 4 ever run. The 7-rung pipeline is effectively a 3-rung pipeline in production. Rungs 3, 5, and 6 exist only in the secondary `run_gate_rung()` code path (line 17546) which is used when rung > 0 is explicitly requested.

### 1.3 `gate_rung_caps()` hard-codes caps to false

**File**: `crates/roko-cli/src/orchestrate.rs:17209-17223`

```rust
fn gate_rung_caps(&self, exec_dir: &Path, generated_tests: Option<&Arc<dyn GeneratedArtifactStore>>) -> RungCaps {
    RungCaps {
        has_lint_tool: gate_config.clippy_enabled && build_system != BuildSystem::Make,
        has_symbol_manifest: false,       // <-- always false
        has_generated_tests: generated_tests.is_some(),
        has_property_tests: false,        // <-- always false
        has_integration_scenario: false,  // <-- always false
    }
}
```

Even before the `_ =>` catch-all, caps filtering would already exclude Symbol, PropertyTest, and Integration from `select_rungs()` output. This double-disabling (caps + catch-all) makes it impossible for these rungs to execute through the primary path.

**Severity**: CRITICAL -- These rungs have real, working gate implementations that are unreachable.

### 1.4 [MEDIUM] Lint guard is redundant

At line 17283: `Rung::Lint if caps.has_lint_tool =>` -- but `select_rungs()` already filters by caps (line 17272 in `rung_selector.rs`), so this guard can never trigger on a false value. The cap was already checked.

---

## 2. Gate Threshold Adaptation

### 2.1 [LOW] `observe()` receives real data -- path is healthy

**File**: `crates/roko-cli/src/orchestrate.rs:16859-16862`

```rust
for recorded in &recorded_verdicts {
    self.adaptive_thresholds.observe(recorded.rung.as_index(), recorded.verdict.passed);
}
```

This correctly feeds per-rung pass/fail results into the adaptive EMA system. The adaptive thresholds receive real gate data from every pipeline execution.

### 2.2 [HIGH] `observe_pipeline()` and `drain_spc_alerts()` are NEVER called from production code

**File**: `crates/roko-gate/src/adaptive_threshold.rs:468` and `446`

A grep across the entire codebase for `observe_pipeline` and `drain_spc_alerts` returns results **only** in:
- The `adaptive_threshold.rs` implementation and its `#[cfg(test)]` module

Neither `observe_pipeline()` nor `drain_spc_alerts()` is called from `orchestrate.rs` or any other production code path. This means:

- **Hotelling T-squared joint anomaly detection (GATE-08)**: Built but never invoked. The `HotellingDetector` is never initialized at runtime.
- **SPC alerts (GATE-01)**: The per-rung `SpcDetector` ensemble runs inside `observe()` and accumulates alerts, but those alerts are never drained or acted upon. They accumulate in `pending_spc_alerts` forever and are lost when the process exits.

**Impact**: The statistical process control infrastructure (CUSUM via `observe()` works, but the higher-level SPC detectors and joint anomaly detection are dead at runtime.

### 2.3 [LOW] Adaptive threshold feedback into gate config is limited to rungs 5-6

**File**: `crates/roko-cli/src/orchestrate.rs:17470-17478`

```rust
if rung == 5 {
    config.fact_check_min_confidence = Some(nominal);
}
if rung == 6 {
    config.llm_judge_min_score = Some(nominal as f32);
}
```

The adaptive EMA pass rate is used as a threshold parameter only for FactCheckGate (rung 5) and LlmJudgeGate (rung 6). Rungs 0-4 use the adaptive data only for skip/retry decisions (via `should_skip_rung` and `suggested_max_retries`), not for adjusting internal gate strictness. Since rungs 5-6 don't actually execute in the primary path (see finding 1.2), the threshold adaptation has no practical effect.

---

## 3. Agent Dispatch Paths

### 3.1 [HIGH] `dispatch_agent_with()` is 2,059 lines long

**File**: `crates/roko-cli/src/orchestrate.rs:14554-16613`

The function spans from line 14554 to line 16613 -- **2,059 lines** of a single async function. It contains:

1. Budget check and pressure calculation (~30 lines)
2. Task definition loading (~50 lines)
3. Domain resolution and git state capture (~30 lines)
4. Base model selection via `resolve_effective_model()` (~50 lines)
5. CascadeRouter adaptive model routing (~250 lines, lines 14754-15003)
6. Lookahead router post-filter (~35 lines)
7. Budget guardrail re-check (~25 lines)
8. Skill/playbook/search enrichment (~50 lines)
9. Provider health check (~25 lines)
10. Approval gate (~15 lines)
11. MCP server acquisition (~5 lines)
12. Daimon somatic signal + dispatch params (~30 lines)
13. Routing explanation + log record construction (~100 lines)
14. Context provider resolution (~90 lines)
15. Neuro context assembly (~20 lines)
16. Tool allowlist construction (~40 lines)
17. Format bandit selection (~20 lines)
18. System prompt building (~80 lines)
19. Prompt composition with scorers (~100 lines)
20. Signal persistence (~10 lines)
21. Safety pre-dispatch check (~30 lines)
22. Extension pre-inference hook (~15 lines)
23. Agent invocation record creation (~30 lines)
24. **Three separate agent spawn/run blocks** (~330 lines):
    - Claude CLI path (line 15804-15909)
    - Ollama tool loop path (line 15910-16011)
    - Generic subprocess path with known-protocol vs unknown sub-paths (line 16012-16136)
25. Safety post-dispatch check (~30 lines)
26. Learning feedback recording (~15 lines)
27. Cost accounting (~40 lines)
28. Output persistence (~10 lines)
29. Ghost turn detection (~30 lines)
30. Context attribution feedback (~100 lines)
31. Efficiency event emission (~80 lines)
32. DispatchOutcome construction (~10 lines)

**Extractable responsibilities** (should be separate functions or structs):
- Model routing pipeline (steps 4-6): ~335 lines of model selection could be a `ModelRouter::select()` method
- Context/prompt assembly (steps 14-19): ~350 lines could be a `PromptAssembler::build()` method
- Agent spawn/run (step 24): ~330 lines of three-way branching could be an `AgentLauncher::run()` method
- Post-dispatch processing (steps 25-31): ~295 lines could be a `DispatchPostProcessor::record()` method

### 3.2 [MEDIUM] Four agent dispatch paths with inconsistent capability coverage

The dispatch_agent_with function has four distinct agent execution paths:

| Path | Condition | Backend | Tool loop | Budget guard | Cost table |
|------|-----------|---------|-----------|--------------|------------|
| 1. Claude CLI | `command == "claude"` | `spawn_agent_with_layer` + `TaskRunner` | N/A (CLI handles) | `RunnerBudgetGuardrail` | `task_runner_cost_table()` |
| 2. Ollama | `command == "ollama"` | `OllamaLlmBackend` + `ToolLoop` | Full tool loop | None | None (uses `total_usage`) |
| 3a. Known protocol | `is_known_protocol_command()` | `spawn_agent_with_layer` + `TaskRunner` | N/A | `RunnerBudgetGuardrail` | `RunnerCostTable::default()` |
| 3b. Generic subprocess | else | `spawn_agent_with_layer` + `TaskRunner` | N/A | `RunnerBudgetGuardrail` | `RunnerCostTable::default()` |

**Inconsistencies**:
- **Ollama path has no RunnerBudgetGuardrail**: The Claude and generic paths wrap the agent in a `TaskRunner` with budget tracking, but the Ollama path runs a raw `ToolLoop` with no per-turn budget enforcement.
- **Known protocol and generic paths get `RunnerCostTable::default()`**: Only the Claude path gets `task_runner_cost_table()` with actual cost-per-token data. All other paths use a default cost table, meaning cost tracking is approximate.
- **Known protocol path passes `system_prompt: None`**: Unlike Claude which gets the full system prompt, known-protocol agents get no system prompt in their spawn spec.

### 3.3 [MEDIUM] Codex backend is handled but via OpenAI-compat, not native protocol

**File**: `crates/roko-core/src/agent.rs:138-140`

```rust
} else {
    Self::Codex
}
```

`AgentBackend::from_model()` falls through to `Codex` for any model slug that doesn't match Claude, Ollama, Perplexity, Cerebras, or Cursor patterns. The `Codex` backend maps to `ProviderKind::OpenAiCompat` (line 148), so it dispatches via OpenAI-compatible HTTP, not Codex's native JSON-RPC protocol. The `is_known_protocol_command()` check may or may not recognize "codex" as a known protocol command. This is adequate for most use cases but means Codex-specific features (file apply, project state) are not surfaced.

### 3.4 [LOW] `ModelCallService` is NOT used in the main orchestration loop

`ModelCallService` is defined in `crates/roko-agent/src/model_call_service.rs` and implements the `ModelCaller` trait. However, it is **not referenced** anywhere in `orchestrate.rs`. The orchestration loop uses the agent subprocess model (spawn claude/ollama/generic process) rather than the HTTP-based `ModelCallService`.

`ModelCallService` is used in:
- `roko-agent-server` (per-agent sidecar HTTP routes)
- The `roko chat` inline surface
- Episode completion (`episode_completion.rs`)

This is not necessarily a bug -- the subprocess model and the HTTP service model are alternative dispatch strategies. But it means `ModelCallService`'s sophisticated provider routing, cascade router integration, and cost tracking are not available in the main plan execution path.

---

## 4. Gate Oracle Enrichment

### 4.1 [MEDIUM] `enrich_rung_config()` enriches rungs 4 and 6 -- which don't execute in primary path

**File**: `crates/roko-cli/src/orchestrate.rs:17515-17544`

```rust
fn enrich_rung_config(&self, config: &mut RungExecutionConfig, rung: u32, ...) {
    // GATE-07: wire generated_test_artifacts for rung 4.
    if (rung == 4 || rung > 6) && config.generated_test_artifacts.is_none() { ... }
    // GATE-07: wire integration_test_pattern from task verify steps for rung 6.
    if (rung == 6 || rung > 6) && config.integration_test_pattern.is_none() { ... }
}
```

`enrich_rung_config()` only enriches rungs 4 and 6. But it's only called from `run_gate_rung()` (line 17546), not from `run_selected_gate_pipeline()` (line 17371). The primary execution path (`run_gate_pipeline` with rung=0) calls `run_selected_gate_pipeline()`, which constructs gates directly in `selected_gate_steps()` and never calls `enrich_rung_config()`.

For `run_gate_rung()` (the secondary path, used when rung > 0), it does enrich:
- Rung 4: wires `generated_test_artifacts` from the exec dir
- Rung 6: wires `integration_test_pattern` from task verify steps

Note that `gate_rung_config()` (line 17453) separately wires:
- Rung 3: `source_roots` for SymbolGate
- Rung 5: `fact_check_oracle` (Perplexity), `fact_check_min_confidence`
- Rung 6: `llm_judge_oracle` (AgentJudgeOracle), `llm_judge_min_score`

So the secondary path (explicit rung > 0) has complete oracle wiring for all rungs. The primary path (rung=0) has none of it, because it builds gates directly.

### 4.2 `run_gate_rung()` builds proper signals for rungs 3, 5, 6

**File**: `crates/roko-cli/src/orchestrate.rs:17567-17633`

The secondary execution path (`run_gate_rung()`) correctly constructs:
- `symbol_signal` from task context symbols (for rung 3)
- `fact_check_signal` from task acceptance criteria (for rung 5)
- `llm_judge_signal` from task description + git diff (for rung 6)
- `code_intel_hints` from the code index (for symbol/LLM-judge enrichment)

These are well-implemented but only accessible through the non-default path.

---

## 5. Model Routing in Dispatch

### 5.1 [LOW] CascadeRouter IS consulted during agent dispatch -- properly wired

**File**: `crates/roko-cli/src/orchestrate.rs:14680-15001`

The model selection pipeline in `dispatch_agent_with` is comprehensive and well-wired:

1. **Base selection** via `resolve_effective_model()` (line 14680): 6-step precedence chain (CLI > Task > Role > CascadeRouter > ProjectDefault > BuiltIn)
2. **CascadeRouter consultation** (lines 14777-15001): When task_def exists and no hard override:
   - Builds full `CascadeRoutingContext` with conductor load, budget pressure
   - Filters healthy models via `ProviderHealthTracker`
   - Scores models with capability matching via `score_model_for_task()`
   - Applies knowledge routing boost from neuro store
   - Applies conductor bias (deprioritize, prefer_cheaper)
   - Applies cost pressure filtering
   - Runs `select_for_frequency_among()` with LinUCB and C-Factor
   - Applies experiment overrides from `ModelExperimentStore`
3. **Lookahead router** (lines 15010-15043): Optional tier downgrade based on calibration data
4. **Budget guardrail** (lines 15047-15069): Final override to cheaper model if budget pressure is critical
5. **Provider health** (lines 15106-15130): Fallback if selected provider is unhealthy
6. **Daimon modulation** (lines 15158-15161): Affect-engine can override model selection

This is one of the most thoroughly wired subsystems in the codebase. The CascadeRouter is genuinely consulted with real context.

---

## 6. Summary of Findings

### CRITICAL

| # | Finding | File:Line | Impact |
|---|---------|-----------|--------|
| C1 | `_ =>` catch-all in `selected_gate_steps()` silently drops rungs 3, 5, 6 | `orchestrate.rs:17296-17298` | 7-rung pipeline is effectively 3 rungs. Symbol, PropertyTest, FactCheck, LlmJudge, and Integration gates never execute in production. |
| C2 | `gate_rung_caps()` hard-codes `has_symbol_manifest`, `has_property_tests`, `has_integration_scenario` to false | `orchestrate.rs:17218-17221` | Even without the catch-all, caps would exclude these rungs from `select_rungs()` output. |

### HIGH

| # | Finding | File:Line | Impact |
|---|---------|-----------|--------|
| H1 | `observe_pipeline()` never called from production code | `adaptive_threshold.rs:468` | Hotelling T-squared joint anomaly detection (GATE-08) is dead code at runtime. |
| H2 | `drain_spc_alerts()` never called from production code | `adaptive_threshold.rs:446` | SPC alerts from CUSUM/EWMA/BOCPD accumulate in memory and are never acted upon. |
| H3 | `dispatch_agent_with()` is 2,059 lines | `orchestrate.rs:14554-16613` | Unmaintainable; 4+ responsibilities in one function. Extraction into ModelRouter, PromptAssembler, AgentLauncher, and DispatchPostProcessor would reduce to ~200 lines each. |

### MEDIUM

| # | Finding | File:Line | Impact |
|---|---------|-----------|--------|
| M1 | `enrich_rung_config()` enriches rungs not reachable from primary gate path | `orchestrate.rs:17515-17544` | Oracle wiring for rungs 4, 6 exists but only in secondary `run_gate_rung()` path. |
| M2 | Ollama dispatch path has no `RunnerBudgetGuardrail` | `orchestrate.rs:15910-16011` | Ollama agents can run without per-turn budget enforcement. |
| M3 | Known-protocol and generic paths use `RunnerCostTable::default()` | `orchestrate.rs:16100` | Cost tracking for non-Claude backends is approximate. |
| M4 | Adaptive threshold feedback only affects rungs 5-6 which don't execute | `orchestrate.rs:17470-17478` | The EMA pass rate feeds into `fact_check_min_confidence` and `llm_judge_min_score`, but those rungs never run in the primary path. |

### LOW

| # | Finding | File:Line | Impact |
|---|---------|-----------|--------|
| L1 | Lint guard in `selected_gate_steps()` is redundant | `orchestrate.rs:17283` | `select_rungs()` already filters by caps; the `if caps.has_lint_tool` guard adds no value. |
| L2 | `ModelCallService` not used in orchestration loop | `model_call_service.rs` | Two parallel dispatch strategies exist (subprocess vs HTTP service); not a bug but increases maintenance surface. |
| L3 | Codex backend dispatches via generic OpenAI-compat, not native protocol | `agent.rs:138-140` | Codex-specific features are not surfaced, but basic dispatch works. |

---

## 7. Recommended Fixes (Priority Order)

### P0: Enable rungs 3, 5, 6 in `selected_gate_steps()`

Add match arms for `Symbol`, `PropertyTest`, and `Integration` rungs in `selected_gate_steps()`, constructing the appropriate gate objects (mirroring what `rung_dispatch::run_canonical_rung()` already does). Also update `gate_rung_caps()` to dynamically detect whether symbol manifests, property tests, and integration scenarios are available, rather than hard-coding false.

### P1: Wire `observe_pipeline()` and `drain_spc_alerts()`

After the per-rung `observe()` loop in `run_gate_pipeline()` (line 16862), add:
```rust
let pass_rates: Vec<f64> = recorded_verdicts.iter()
    .map(|r| if r.verdict.passed { 1.0 } else { 0.0 })
    .collect();
self.adaptive_thresholds.observe_pipeline(&pass_rates);
let spc_alerts = self.adaptive_thresholds.drain_spc_alerts();
for (rung, alert) in &spc_alerts {
    tracing::warn!(rung, alert = ?alert, "SPC alert detected");
    // Emit conductor signal or adjust routing
}
```

### P2: Extract `dispatch_agent_with()` into composable units

Split the 2,059-line function into:
1. `ModelRouter::select()` -- model selection pipeline
2. `PromptAssembler::build()` -- context + prompt composition
3. `AgentLauncher::run()` -- the 4-way agent dispatch
4. `DispatchPostProcessor::record()` -- learning, attribution, cost

### P3: Add budget guardrail to Ollama path

Wrap the Ollama `ToolLoop` in `TaskRunner` or add equivalent budget checking.
