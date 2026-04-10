# Gate Pipeline Subsystem Audit

7-rung verification system with 12 gates, adaptive thresholds, and 3 independent execution paths that don't share configuration.

## The Problem

The gate pipeline itself is well-designed (clean rung abstraction, complexity-based selection, adaptive thresholds). The problems are:
1. **Three separate gate dispatch paths** with different configs and capabilities
2. **Rungs 3-6 often return stubs** because their inputs aren't wired in most paths
3. **Sophisticated features** (SPC detectors, Hotelling T-squared, domain profiles) are built but unused at runtime
4. **LLM judge gate bypasses ModelCallService** — calls Claude directly with hardcoded model
5. **Gate replan only works from dead orchestrate.rs**

---

## 1. Gate Types

### 7-Rung Pipeline (rung_dispatch.rs)

| Rung | Gate | File | Status |
|------|------|------|--------|
| 0 | CompileGate | `compile.rs` | Live |
| 1 | ClippyGate | `clippy_gate.rs` | Live |
| 2 | TestGate | `test_gate.rs` | Live |
| 3 | SymbolGate | `symbol_gate.rs` | Stub — needs SymbolManifest input |
| 4 | GeneratedTestGate + VerifyChainGate | `generated_test_gate.rs` + `verify_chain_gate.rs` | Stub — needs artifact store |
| 5 | PropertyTestGate + FactCheckGate | `property_test_gate.rs` + `fact_check.rs` | Stub — needs Perplexity oracle |
| 6 | LlmJudgeGate + IntegrationGate | `llm_judge_gate.rs` + `integration_gate.rs` | Stub — needs agent oracle |

### 6 Standalone Gates (Not Rung-Dispatched)

| Gate | File | Status |
|------|------|--------|
| ShellGate | `shell.rs` | Live (roko run only) |
| DiffGate | `diff_gate.rs` | Built, no callers |
| CodeExecutionGate | `code_exec.rs` | Built, no callers |
| BenchmarkRegressionGate | `benchmark_gate.rs` | Built, no callers |
| FormatCheckGate | `format_check_gate.rs` | Built, no callers |
| SecurityScanGate | `security_scan_gate.rs` | Built, no callers |

### 3 Composition Wrappers (Test-Only)

ParallelGate, VotingGate, FallbackGate — built in `composition.rs`, only used in tests.

---

## 2. Complexity-Based Rung Selection

`rung_selector.rs` (350 LOC) — determines which rungs execute based on plan complexity:

| Complexity | Compile | Lint | Test | Symbol | GenTest | PropTest | Integration |
|---|---|---|---|---|---|---|---|
| Trivial | ✓ | | | | | | |
| Simple | ✓ | ✓ | | | | | |
| Standard | ✓ | ✓ | ✓ | ✓ | | | |
| Complex | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |

**Escalation**: On repeated failure, complexity promotes (Trivial→Simple→Standard→Complex).

**Capability filtering**: `RungCaps` can remove rungs if project lacks required capability (e.g., no symbol manifest → skip rung 3).

---

## 3. Three Gate Dispatch Paths

### Path 1: `roko run` (run.rs)

```
run_gate() → match GateConfig { Shell | Compile | Clippy | Test }
```
- 4 hardcoded gate types from `roko.toml` config
- No rung selection, no adaptive thresholds, no LLM judge
- No gate feedback to agent, no replan on failure
- **Simplest path — works but minimal**

### Path 2: ACP Runner (roko-acp/runner.rs)

```
run_gates() → CompileGate → TestGate (if not skipped) → ClippyGate (if not skipped)
```
- 3 gates hardcoded (compile, test, clippy)
- Adaptive threshold heuristic: skip test/clippy if EMA pass-rate > threshold for 20+ consecutive passes
- Loads/saves `.roko/learn/gate-thresholds.json`
- **Note**: Runs clippy AFTER test (rung 1 after rung 2) due to inline logic ordering
- No rung selection, no LLM judge, no fact check, no replan

### Path 3: Orchestrate.rs (DEAD)

```
run_gate_rung() → resolve config → enrich config → run_canonical_rung()
```
- Full 7-rung pipeline via `run_rung()` from `rung_dispatch.rs`
- Rich input assembly: symbol manifests, diffs, acceptance criteria
- Oracle wiring: LLM judge (AgentJudgeOracle), fact check (PerplexitySearchOracle)
- Adaptive threshold with role-based floor overrides
- Gate feedback → agent context for retry
- Replan on failure (deduped, capped)
- Pheromone deposition from verdicts
- **Most capable path — but dead code**

### What's Missing From Live Paths

| Feature | run.rs | ACP | orchestrate.rs (dead) |
|---|---|---|---|
| Rung selection | No | No | Yes |
| Rungs 3-6 | No | No | Yes (with inputs) |
| Adaptive thresholds | No | Yes (basic) | Yes (full SPC) |
| LLM judge oracle | No | No | Yes |
| Fact check oracle | No | No | Yes |
| Gate feedback to agent | No | No | Yes |
| Replan on failure | No | No | Yes |
| Pheromone deposition | No | No | Yes |
| Domain profiles | No | No | No (built, unused) |

---

## 4. Adaptive Thresholds

`adaptive_threshold.rs` (600 LOC) — sophisticated but underutilized.

**What's built:**
- EMA pass-rate tracking per rung (alpha=0.1)
- Consecutive pass streak tracking
- CUSUM accumulators (detect sustained shifts)
- SPC detector ensemble (CUSUM + EWMA + BOCPD)
- Hotelling T-squared (multi-gate joint anomaly)
- Domain profiles (coding/research/security presets)
- Role-based floor overrides
- Neuro-informed hints

**What's actually used:**
- ACP runner: EMA pass-rate for skip heuristic (basic)
- orchestrate.rs (dead): Full SPC + role floors + neuro hints

**What's never used:**
- Domain profiles (ThresholdProfile) — built, never instantiated
- Hotelling T-squared — `observe_pipeline()` exists but never called from live paths
- SPC alerts — collected but never acted upon

---

## 5. LLM Judge Gate — Anti-Pattern

The LLM judge oracle (`AgentJudgeOracle` in orchestrate.rs:2928-2995) **bypasses the provider system**:

```rust
// Hardcoded model, hardcoded command, direct spawn
AgentJudgeOracle {
    command: "claude".to_string(),
    model: "claude-sonnet-4-20250514".to_string(),
    timeout_ms: 120_000,
    skip_permissions: true,
}
```

This violates Anti-Pattern #1 (just shell out to Claude). The judge call:
- Doesn't go through CascadeRouter
- Doesn't record an episode
- Doesn't track cost
- Uses hardcoded model instead of configured routing

**Should use:** `ModelCallService::complete()` with `caller: ModelCallCaller::gate_judge`.

---

## 6. Gate Failure Handling

### What Exists (orchestrate.rs, dead)

**Replan ladder:**
1. Gate fails → `gate_failure_count++`
2. If `learning_config.replan_on_gate_failure` → emit `PlanReplan` event
3. Deduplication via hash of (plan_id, task_id, rung, gate)
4. Cap at 2 replans per task to avoid infinite loops
5. Gate feedback filtered into `GateFeedback { errors, warnings, suggestions }`
6. Feedback injected into agent's next prompt

**What live paths do on gate failure:**
- `roko run`: Print failure, exit
- ACP: Return failure to Zed, auto-fixer may retry
- Neither has replan, feedback injection, or escalation

---

## 7. Gate Feedback

`feedback.rs` (200 LOC) — filters raw gate output into actionable items:

**Classification:**
- Error: lines starting with "error", "Error:", "panicked"
- Warning: lines starting with "warning", "Warning:"
- Suggestion: lines starting with "help:", "note:", "-->"

**Noise filtering:** Removes cargo progress, npm deprecation warnings, progress bars.

**Usage:** Only from orchestrate.rs (dead). Live paths don't feed gate results back to agents.

---

## 8. Anti-Patterns In This Subsystem

| Anti-Pattern | Where |
|---|---|
| **#7 Copy between runtimes** | 3 separate gate dispatch paths with different capabilities |
| **#1 Shell out to Claude** | LLM judge oracle hardcodes `Command::new("claude")` |
| **#6 Feedback as afterthought** | Live paths don't record gate verdicts to episodes |
| **#4 Features in wrong layer** | ACP runner has inline adaptive skip logic instead of using rung_selector |

---

## 9. File Inventory

| File | LOC | Status |
|---|---|---|
| `roko-gate/src/rung_dispatch.rs` | 249 | Core — stable |
| `roko-gate/src/rung_selector.rs` | 350 | Core — stable |
| `roko-gate/src/gate_pipeline.rs` | 225 | Core — stable |
| `roko-gate/src/adaptive_threshold.rs` | 600 | Sophisticated — underutilized |
| `roko-gate/src/spc.rs` | 400 | Built — never read in live paths |
| `roko-gate/src/hotelling.rs` | 200 | Built — never called |
| `roko-gate/src/feedback.rs` | 200 | Good — only used from dead path |
| `roko-gate/src/llm_judge_gate.rs` | 200 | Stub inputs, bypasses provider |
| `roko-gate/src/fact_check.rs` | 300 | Stub inputs, needs Perplexity key |
| `roko-gate/src/compile.rs` | 150 | Live |
| `roko-gate/src/clippy_gate.rs` | 100 | Live |
| `roko-gate/src/test_gate.rs` | 150 | Live |
| `roko-gate/src/shell.rs` | 100 | Live (roko run) |
| `roko-gate/src/composition.rs` | 200 | Test-only |
| 6 standalone gates | ~900 | Built, no callers |
| **Total roko-gate** | ~19K | **39 files** |

---

## 10. Grep Gates

```bash
# Gate dispatch paths should converge to one
rg 'fn run_gate|fn run_gates|fn run_gate_rung|fn run_verify_gate' crates/ --type rust
# Should return 1 after unification

# LLM judge should use ModelCallService
rg 'AgentJudgeOracle|claude-sonnet' crates/roko-gate/ crates/roko-cli/src/orchestrate.rs --type rust
# Should return 0 after migration

# Gate feedback should be used by all paths
rg 'feedback_for_agent' crates/ --type rust | grep -v test
# Should be called from unified GateService, not just orchestrate.rs
```
