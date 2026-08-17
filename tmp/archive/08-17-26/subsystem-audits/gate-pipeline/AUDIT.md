# Gate Pipeline Subsystem Audit

Comprehensive analysis of the 7-rung verification system: 16 concrete gates
(10 rung-dispatched + 6 standalone), 3 composition wrappers, adaptive thresholds,
SPC detectors, Hotelling joint anomaly detection, process reward model, and 3
execution paths with different capabilities and caller contexts.

Last verified: 2026-04-29 against source in `crates/roko-gate/src/` (40 files, ~20.1K LOC).

---

## 1. Architecture Overview

The gate pipeline implements a two-tier verification architecture:

**Tier 1: Rung-dispatched gates** — 7 rungs in canonical order (compile, lint,
test, symbol, generated-test, property-test, integration). Selected by plan
complexity and executed sequentially. This is the core verification spine.

**Tier 2: Standalone gates** — 6 gates invoked outside the rung pipeline for
scenario-specific checks (diff review, sandboxed code execution, benchmarks,
formatting, security scanning, arbitrary shell commands).

**Composition layer** — 3 algebraic combinators (ParallelGate, VotingGate,
FallbackGate) plus a ComposedGatePipeline with 4 composition modes (Sequential,
Parallel, Voting, Fallback). These allow any gate to be combined into parallel,
majority-vote, or fallback topologies.

**Statistical layer** — EMA-based adaptive thresholds, SPC detector ensemble
(CUSUM + EWMA Control Chart + BOCPD), Hotelling T-squared joint anomaly
detection, process reward model (Promise/Progress signals), and PELT offline
change-point detection.

### Gate Trait

All gates implement `Verify` (from `roko-core`):

```rust
#[async_trait]
pub trait Verify: Cell + Send + Sync {
    async fn verify(&self, signal: &Engram, ctx: &Context) -> Verdict;
    fn name(&self) -> &str;
}
```

Verdicts carry: `passed: bool`, `gate: String`, `reason: String`,
`detail: Option<String>`, `error_digest: Option<String>`,
`test_count: Option<TestCount>`, `duration_ms: u64`.

### Unified Gate Service

`GateService` (`gate_service.rs`, 680 LOC) is the canonical gate runner.
It implements the `GateRunner` trait from `roko-core::foundation`:

```rust
#[async_trait]
impl GateRunner for GateService {
    async fn run_gates(&self, config: GateConfig) -> Result<GateReport>;
}
```

GateService:
- Maps gate names to rung indices (compile=0, clippy=1, test=2, diff=3, fmt=4, custom=5, judge=6)
- Orders gates by rung index regardless of config order
- Filters by `max_rung` when configured
- Applies adaptive skip decisions (never skips rung 0 / compile)
- Records pass/fail outcomes back to AdaptiveThresholds
- Short-circuits on first non-skipped failure
- Handles shell/custom gates via ShellGateCommand from GateConfig
- Skips LLM judge gate with a clear message (not yet implemented in GateService)

---

## 2. The 7-Rung Pipeline

### Rung Definitions (rung_selector.rs, 560 LOC)

| Rung | Index | Gates | Input Required |
|------|-------|-------|----------------|
| Compile | 0 | `CompileGate` | `GatePayload` (working dir) |
| Lint | 1 | `ClippyGate` | `GatePayload` |
| Test | 2 | `TestGate` | `GatePayload` |
| Symbol | 3 | `SymbolGate` | `SymbolManifest` signal |
| GeneratedTest | 4 | `GeneratedTestGate` + `VerifyChainGate` | Artifact store + verify script |
| PropertyTest | 5 | `PropertyTestGate` + `FactCheckGate` | `SearchOracle` (Perplexity) |
| Integration | 6 | `LlmJudgeGate` + `IntegrationGate` | `JudgeOracle` + test pattern |

### Rung Dispatch (rung_dispatch.rs, 249 LOC)

`run_rung()` and `run_canonical_rung()` execute a single rung using:

- `RungExecutionInputs` — per-gate signals (symbol_signal, fact_check_signal,
  llm_judge_signal, code_intel_hints)
- `RungExecutionConfig` — oracle instances, threshold overrides, artifact stores,
  verdict publisher

When inputs are missing, gates return stub verdicts that pass but note the
missing input. This is a deliberate design decision: missing inputs do not
fail the pipeline, they degrade it gracefully.

### Complexity-Based Selection (rung_selector.rs)

| Complexity | Compile | Lint | Test | Symbol | GenTest | PropTest | Integration |
|---|---|---|---|---|---|---|---|
| Trivial | Y | | | | | | |
| Simple | Y | Y | | | | | |
| Standard | Y | Y | Y | Y | | | |
| Complex | Y | Y | Y | Y | Y | Y | Y |

**Escalation ladder**: On repeated failure, complexity promotes one tier per
failure (Trivial -> Simple -> Standard -> Complex). Saturates at Complex.
This ensures stubbornly failing tasks face progressively more thorough verification.

**Capability filtering**: `RungCaps` removes rungs the project cannot run
(e.g., no lint tool -> skip Lint). Caps only narrow, never add rungs.

```rust
pub fn select_rungs(complexity: PlanComplexity, caps: &RungCaps, prior_failures: u32) -> Vec<Rung> {
    let effective = complexity.escalate_by(prior_failures);
    base_rungs(effective).iter().copied().filter(|r| caps.allows(*r)).collect()
}
```

---

## 3. Concrete Gate Implementations

### 3.1 CompileGate (compile.rs, 226 LOC)

Build-system-aware compilation check. Reads `GatePayload` from signal body,
runs `cargo check --workspace --message-format=json`, parses output.

Key features:
- Configurable build system (`BuildSystem::detect()` probes for Cargo.toml, package.json, go.mod)
- Extra args passthrough
- 10-minute default timeout with `kill_on_drop`
- Custom target dir via `CARGO_TARGET_DIR` env
- Structured error classification via `compile_errors.rs` (11 error categories,
  12 failure classes, 4 failure actions: Retry/NeedsReplan/Blocked/NeedsHuman)
- Error summary extracts up to 3 error-level diagnostics for concise verdict reason

### 3.2 ClippyGate (clippy_gate.rs, 232 LOC)

Runs `cargo clippy --workspace --no-deps -- -D warnings`. Treats warnings-as-errors.
Same architecture as CompileGate with build-system awareness.

### 3.3 TestGate (test_gate.rs, 404 LOC)

Runs `cargo test --workspace`. Parses test output for pass/fail/ignored counts
via `parse_test_counts()`. Reports `TestCount` in verdict for downstream aggregation.

Test selector support:
- `TestSelector::All` (default)
- `TestSelector::Pattern(String)` — `cargo test <pattern>`
- `TestSelector::ExcludeIntegration` — skips integration tests

### 3.4 SymbolGate (symbol_gate.rs)

Verifies expected symbols exist in source files. Reads a `SymbolManifest`
describing expected struct/function/trait definitions with visibility and
signature constraints. Scans source roots for matching declarations.

Reports missing symbols with `MISSING` tag in error_digest. Enriched by
code-intelligence hints when available (INT-16: code_intel_hint_N tags).

### 3.5 GeneratedTestGate (generated_test_gate.rs)

Writes generated test files from an `ArtifactStore` into the project's
test directory, then runs `cargo test` to verify them pass. This is the
"behavioral specification as tests" gate -- agent generates tests, gate runs them.

`InMemoryArtifactStore` provided for testing; runtime uses filesystem-backed store.

### 3.6 VerifyChainGate (verify_chain_gate.rs)

Runs a verification script (tagged via `verify_script` on the signal).
Supports strict mode (fail on any non-zero exit) and fallback mode (try a
fallback Verify impl if the script is missing).

### 3.7 FactCheckGate (fact_check.rs, 505 LOC)

Validates factual claims by querying a `SearchOracle` (Perplexity API in production).
Extracts claims from signal body, searches for corroboration, computes a
confidence score. Gate passes if confidence exceeds a configurable threshold
(default: `DEFAULT_MIN_CONFIDENCE`).

### 3.8 LlmJudgeGate (llm_judge_gate.rs, 577 LOC)

Sends a `JudgePayload` (task description + diff) to a `JudgeOracle` for scoring.
The oracle returns a float score; gate passes if score >= min_score (default 0.8).

In orchestrate.rs, the oracle is `AgentJudgeOracle` which calls `run_prepared_agent`
(real LLM dispatch). Model falls back to hardcoded `claude-sonnet-4-20250514`.

### 3.9 IntegrationGate (integration_gate.rs)

Runs integration tests matching a specific pattern. Two modes:
- `build_test`: runs `cargo test <pattern>` with a specific build system
- `script`: runs an arbitrary integration test script

### 3.10 Standalone Gates

| Gate | File | Purpose | Status |
|------|------|---------|--------|
| ShellGate | `shell.rs` | Arbitrary shell command verification | Live (roko run) |
| DiffGate | `diff_gate.rs` | Diff analysis; detects stub/vacuous changes | Built, tested |
| CodeExecutionGate | `code_exec.rs` | Sandboxed code execution | Built |
| BenchmarkRegressionGate | `benchmark_gate.rs` | Performance regression detection | Built |
| FormatCheckGate | `format_check_gate.rs` | Code formatting checks | Built |
| SecurityScanGate | `security_scan_gate.rs` | Security vulnerability scanning | Built |

---

## 4. Three Gate Dispatch Paths

### Path 1: `roko run` (crates/roko-cli/src/run.rs)

```
run_gate() -> match GateConfig { Shell | Compile | Clippy | Test }
```

- 4 hardcoded gate types from `roko.toml` config
- No rung selection, no adaptive thresholds, no LLM judge
- No gate feedback to agent, no replan on failure
- **Simplest path -- works but minimal**

### Path 2: ACP Runner (crates/roko-acp/src/runner.rs)

```
run_gates() -> CompileGate -> TestGate (if not skipped) -> ClippyGate (if not skipped)
```

- 3 gates hardcoded (compile, test, clippy)
- Adaptive threshold heuristic: skip test/clippy if EMA pass-rate > threshold
  for 20+ consecutive passes
- Loads/saves `.roko/learn/gate-thresholds.json`
- Runs clippy AFTER test (rung 1 after rung 2) due to inline logic ordering
- No rung selection, no LLM judge, no fact check, no replan

### Path 3: orchestrate.rs (`roko plan run`)

```
ExecutorAction::RunGate -> run_gate_pipeline() -> run_selected_gate_pipeline() or run_gate_rung()
```

- Full 7-rung pipeline via `run_rung()` from `rung_dispatch.rs`
- Complexity-based rung selection via `selected_gate_steps()` using `rung_selector`
- Rich input assembly: symbol manifests, diffs, acceptance criteria
- Oracle wiring: LLM judge (`AgentJudgeOracle` via `run_prepared_agent`),
  fact check (`PerplexitySearchOracle`)
- Adaptive threshold with role-based floor overrides and neuro hints
- Gate feedback to agent context for retry (`feedback_for_agent`)
- Replan on failure (deduped, capped at 2) via `build_gate_failure_plan_revision`
- Pheromone deposition from verdicts
- **Most capable path -- live, called from `roko plan run` executor loop**

### Path 4: Workflow Engine (crates/roko-runtime/src/workflow_engine.rs)

```
EffectDriver -> GateService -> GateReport
```

- Uses the unified GateService with GateConfig from workflow step
- Respects GateConfig.enabled_gates and max_rung
- Emits RuntimeEvents for gate start/pass/fail via EventBus
- Adaptive thresholds via GateService.with_adaptive_thresholds()
- **Newest path -- designed to replace paths 1-3**

### Feature Matrix Across Paths

| Feature | run.rs | ACP | orchestrate.rs | workflow_engine |
|---|---|---|---|---|
| Rung selection | No | No | Yes | Via GateConfig |
| Rungs 3-6 | No | No | Yes | Via GateService |
| Adaptive thresholds | No | Yes (EMA skip) | Yes (full SPC) | Yes (via GateService) |
| LLM judge oracle | No | No | Yes | Stub (skipped) |
| Fact check oracle | No | No | Yes | No |
| Gate feedback to agent | No | No | Yes | No |
| Replan on failure | No | No | Yes | No |
| Pheromone deposition | No | No | Yes | No |
| Domain profiles | No | No | No | No |
| EventBus integration | No | No | Partial | Yes |

---

## 5. Adaptive Thresholds (adaptive_threshold.rs, 957 LOC)

### Core Mechanism

Per-rung EMA of pass rates with alpha=0.1. Each `observe(rung, passed)` call:

1. Updates EMA: `ema = 0.1 * value + 0.9 * ema`
2. Tracks consecutive pass streak (resets on failure)
3. Updates CUSUM accumulators (detects sustained distributional shifts)
4. Feeds observation to SPC detector ensemble
5. Collects any SPC alerts for downstream consumption

### Skip Decision

`should_skip_rung(rung)` returns true when `consecutive_passes >= 20`.
GateService enforces that rung 0 (compile) is **never** skipped regardless.

### CUSUM Change Detection

Two one-sided accumulators track upward and downward shifts from the EMA baseline:
- Sensitivity parameter k=0.25 (slack allowance)
- Decision threshold h=4.0
- On detection: EMA resets to current observation for fast adaptation

### Role-Based Overrides

`override_for_role()` applies a floor from `AgentThresholds.gate_pass_rate_floor`.
This ensures safety-critical roles (e.g., security auditor) never have their
gate thresholds relaxed below a configured minimum.

### Neuro-Informed Hints

`apply_neuro_hints()` biases thresholds using knowledge from the neuro store:
- Known failure rungs: bias EMA downward when few observations exist (<10)
- Known failure rungs: tighten CUSUM sensitivity by 30%
- Known stable rungs: relax CUSUM sensitivity by 30%

### Temperament Adjustments (AGT-06)

Three personality-based modifiers:
- **Conservative**: stricter thresholds (+10%), fewer retries (cap 3), never skip
- **Balanced**: default behavior
- **Aggressive**: relaxed thresholds (-15%), more retries (floor 2), skip earlier (streak/2)

### Residual-Based Tightening (TA-15)

`observe_residual()` adjusts thresholds from prediction residuals. When oracles
over-predict success, the gate threshold tightens proportionally to the absolute
residual magnitude (alpha=0.05).

### Domain Profiles (built, unused at runtime)

Three predefined profiles: coding, research, security. Each sets per-rung
prior pass rates, floor multipliers, retry multipliers, and CUSUM sensitivity
overrides. **Never instantiated at runtime** -- the profiles exist but no
code path constructs or applies them.

### Persistence

`save()` / `load()` / `load_or_new()` for JSON serialization. Atomic write via
temp file + rename. Stored at `.roko/learn/gate-thresholds.json`.

---

## 6. Statistical Process Control (spc.rs, 725 LOC)

Three complementary detectors run in parallel via `SpcDetector`:

### 6.1 CUSUM Detector

Two-sided cumulative sum for sustained shift detection.
- Target: expected in-control pass rate
- Threshold h: alarm when cumsum exceeds this (default 5.0)
- Drift k: allowance parameter, half the smallest shift to detect

### 6.2 EWMA Control Chart

Exponentially weighted moving average with formal UCL/LCL:
- Lambda=0.2 (smoothing factor)
- L=3.0 sigma multiplier for control limits
- Three-zone output: InControl / Warning (2-sigma) / OutOfControl (3-sigma)

More sensitive to small sustained shifts than Shewhart charts because the
exponential weighting carries memory of recent observations.

### 6.3 BOCPD (Bayesian Online Change Point Detection)

Detects abrupt regime changes using a run-length distribution:
- Hazard rate: 0.01 (prior probability of change per step)
- Change threshold: 0.5 (posterior probability to trigger alarm)
- Gaussian predictive model (conjugate normal-inverse-gamma)
- Memory-bounded via trimming low-probability run lengths

Reference: Adams & MacKay (2007), "Bayesian Online Changepoint Detection".

### 6.4 Integration

Alerts from any of the three detectors are collected as `SpcAlert` variants:
- `CusumShift(Upward | Downward)`
- `EwmaOutOfControl { ewma_value }`
- `EwmaWarning { ewma_value }`
- `ChangePoint { observation_index, most_probable_run_length, change_probability }`

Alerts accumulate in `AdaptiveThresholds.pending_spc_alerts` and can be drained
via `drain_spc_alerts()`. Currently only orchestrate.rs drains them.

---

## 7. Hotelling T-Squared (hotelling.rs, 439 LOC)

Joint anomaly detection across the full gate vector. When multiple gates shift
simultaneously, this signals systemic problems (model degradation, environment
change) rather than gate-specific issues.

Formula: T^2 = n * (x - mu)^T * S^(-1) * (x - mu)

Uses Welford's online algorithm extended to multivariate data for numerically
stable incremental mean and covariance updates. Chi-squared critical value
approximated via Wilson-Hilferty.

**Usage**: `observe_pipeline()` is called only from `adaptive_threshold.rs`
unit tests. No runtime caller invokes it.

---

## 8. Gate Feedback (feedback.rs, 393 LOC)

Filters raw gate output into structured `GateFeedback`:

### Classification

| Severity | Patterns |
|----------|----------|
| Error | `error`, `Error:`, `ERROR:`, `FAILED`, `panicked at` |
| Warning | `warning`, `Warning:`, `WARNING:`, `warn[` |
| Suggestion | `help:`, `= help:`, `note:`, `= note:`, `-->`, `hint:` |

### Noise Filtering

Drops: Downloading/Downloaded/Compiling/Checking/Finished/Running/Fresh/
Packaging lines, npm deprecation warnings, progress bars (unicode block characters).

### Fallback

When no lines classify but output is non-empty, the first non-noise line is
surfaced as an error. This prevents silent failures from unrecognized output formats.

### Usage

Called from orchestrate.rs (`roko plan run`). The `roko run` and ACP paths
do not call `feedback_for_agent`.

---

## 9. Additional Gate Infrastructure

### 9.1 Compile Error Classification (compile_errors.rs)

11 error categories: Syntax, UnresolvedImport, TypeMismatch, Lifetime,
MissingMember, Unused, Visibility, Macro, TraitBound, Ownership, Other.

12 failure classes: SyntaxError, ImportError, TypeError, MissingDependencyOrFeature,
BorrowOrLifetime, TestExpectationFailure, ExternalEnvironment,
UnsafeStubOrPassBehavior, PromptContextInsufficiency, RoleToolPermission,
ArchitecturalConflictRequiresReplan, Unknown.

4 failure actions: Retry, NeedsReplan, Blocked, NeedsHuman.

`classify_gate_failure()` maps raw error output to these categories,
enabling agents to take appropriate remediation action.

### 9.2 Process Reward Model (process_reward.rs)

Step-level verification for agent trajectories. Two cybernetic signals derived
from gate verdicts at each agent turn:

- **Promise**: predicts probability of eventual task success given the current
  trajectory (ratchet progression rate, pass history, diff trends)
- **Progress**: measures trajectory delta between turns (rung advancement,
  error reduction, coverage increase)

Enables early termination (low Promise -> abandon) and intervention (stalling
Progress -> change model/strategy).

Reference: Lightman et al. 2023 (PRM800K), AgentPRM (arXiv:2502.10325).

### 9.3 Gate Ratchet (ratchet.rs)

Prevents rung regression: once a gate passes, subsequent failures at lower
rungs trigger escalation rather than regression.

### 9.4 Forensic Replay (forensic.rs)

Content-addressed artifact storage for causal chain reconstruction from gate
verdicts. Enables post-mortem analysis of why a specific gate failed.

### 9.5 Eval Generator (eval_generator.rs)

Dynamically generates verification checks from templates. Supports multiple
evaluation strategies and templates for ad-hoc gate creation.

### 9.6 Verdict Publisher (verdict_publisher.rs)

Broadcasts gate outcomes to interested consumers. Wired into RungExecutionConfig
but currently optional.

### 9.7 Error Patterns (error_patterns.rs)

Extracts failure pattern records from gate classifications and review verdicts.
Used by the learning subsystem to detect recurring failure modes.

### 9.8 Acceptance Contract (acceptance_contract.rs)

Formal acceptance criteria with evidence collection: NoStubRequirement,
ParityLedgerRequirement, ReviewVerdictRequirement, StructuredAgentOutputRequirement.
Each maps to a corresponding evidence type for audit trails.

---

## 10. Composition Wrappers (composition.rs, 569 LOC)

Three standalone combinators:

### ParallelGate
Run N gates concurrently, aggregate by taking minimum score.
If any gate fails, the aggregate fails. Used when inner gates are independent.

### VotingGate
Run all gates, require N-of-M to pass. Mean of passing scores.
Threshold is configurable fraction (0.0 to 1.0).

### FallbackGate
Try primary gate(s); on failure, try fallback gate(s).
First passing verdict wins. Used for degraded-mode verification.

### ComposedGatePipeline (gate_pipeline.rs)

A unified gate pipeline with configurable composition mode:
- `GateComposition::Sequential` — standard short-circuit pipeline
- `GateComposition::Parallel(indices)` — run specified gate indices concurrently
- `GateComposition::Voting { threshold }` — majority-vote with configurable threshold
- `GateComposition::Fallback(indices)` — try primary, fallback on failure

All modes produce aggregate verdicts with test count merging and detailed
step-by-step transcripts.

---

## 11. Anti-Patterns Detected

| AP# | Anti-Pattern | Where | Impact |
|-----|-------------|-------|--------|
| AP-1 | **Stub gates that silently pass** | Rungs 3-6 return stub pass verdicts when inputs missing | Gives false confidence; hides that higher rungs are not running |
| AP-5 | **Hardcoded model fallback** | LlmJudgeGate in orchestrate.rs falls back to `claude-sonnet-4-20250514` | Should route through CascadeRouter |
| AP-6 | **Three separate gate dispatch paths** | run.rs, runner.rs, orchestrate.rs each implement gate dispatch differently | Config drift, feature gaps, maintenance burden |
| AP-7 | **Feedback as afterthought** | `roko run` and ACP paths don't call `feedback_for_agent` | Agents don't learn from gate failures |
| AP-8 | **Built but unused features** | Domain profiles, Hotelling pipeline observation, SPC alert draining | Engineering effort with no runtime benefit |
| AP-9 | **Rung ordering mismatch** | ACP runs clippy (rung 1) after test (rung 2) | Violated canonical order |
| AP-10 | **No cost tracking for LLM judge** | AgentJudgeOracle doesn't record episodes or gate budget | Invisible spend |

---

## 12. Test Coverage

### Unit Tests (inline in source files)

| File | Test Count | Coverage |
|------|-----------|----------|
| gate_service.rs | 14 tests | Rung mapping, ordering, adaptive skip, compile-never-skip |
| gate_pipeline.rs | 19 tests | Empty/single/multi pass, short-circuit, fan-out, composition modes |
| rung_selector.rs | 16 tests | All complexity/cap/escalation combinations |
| adaptive_threshold.rs | 15 tests | EMA, skip, CUSUM, SPC, Hotelling, residual, temperament |
| spc.rs | 8 tests | CUSUM, EWMA, BOCPD, composite detector |
| feedback.rs | 13 tests | Classification, noise filtering, roundtrip |
| compile.rs | 4 tests | Error summarization |
| composition.rs | Tests in gate_pipeline.rs ComposedGatePipeline section |

### Integration Tests (crates/roko-gate/tests/)

| File | Tests | What |
|------|-------|------|
| gate_truth.rs | 6 tests | GateService truthful verdicts (shell, compile, unknown, judge) |
| rungs.rs | 9 tests | Full 7-rung pipeline with mock oracles, real cargo scaffold |
| adaptive_threshold.rs | 2 tests | Persistence roundtrip, neutral defaults |
| compile_real_project.rs | Tests | Real cargo project compile verification |

---

## 13. File Inventory

| File | LOC | Role | Status |
|------|-----|------|--------|
| gate_service.rs | 680 | Unified gate runner (GateRunner impl) | Wired via workflow engine |
| gate_pipeline.rs | 1118 | Sequential + composed pipeline (Verify impl) | Stable |
| rung_dispatch.rs | 249 | 7-rung runtime mapping | Stable |
| rung_selector.rs | 560 | Complexity-based rung selection | Stable |
| adaptive_threshold.rs | 957 | EMA + CUSUM + SPC + temperament | Partially utilized |
| spc.rs | 725 | CUSUM + EWMA + BOCPD detectors | Used via adaptive_threshold |
| hotelling.rs | 439 | Joint anomaly detection | Test-only (no runtime callers) |
| feedback.rs | 393 | Agent feedback filter | Used from orchestrate.rs only |
| compile.rs | 226 | CompileGate | Live (all paths) |
| clippy_gate.rs | 232 | ClippyGate | Live (all paths) |
| test_gate.rs | 404 | TestGate + parse_test_counts | Live (all paths) |
| shell.rs | 225 | ShellGate | Live (roko run + GateService) |
| compile_errors.rs | ~400 | Error classification | Live (CompileGate) |
| llm_judge_gate.rs | 577 | LLM judge gate | Wired in orchestrate.rs |
| fact_check.rs | 505 | Fact-check gate | Wired in orchestrate.rs |
| composition.rs | 569 | ParallelGate, VotingGate, FallbackGate | Test-only |
| process_reward.rs | ~300 | PRM (Promise/Progress signals) | Built |
| diff_gate.rs | ~200 | Diff analysis | Built, tested |
| symbol_gate.rs | ~300 | Symbol manifest verification | Wired in orchestrate.rs |
| generated_test_gate.rs | ~300 | Generated behavioral tests | Wired in orchestrate.rs |
| verify_chain_gate.rs | ~200 | Verification script runner | Wired in orchestrate.rs |
| property_test_gate.rs | ~200 | Property-based test runner | Wired in orchestrate.rs |
| integration_gate.rs | ~200 | Integration test runner | Wired in orchestrate.rs |
| benchmark_gate.rs | ~200 | Performance regression | Built |
| format_check_gate.rs | ~150 | Code formatting | Built |
| security_scan_gate.rs | ~150 | Security scanning | Built |
| acceptance_contract.rs | ~400 | Formal acceptance criteria | Built |
| error_patterns.rs | ~200 | Failure pattern extraction | Built |
| forensic.rs | ~300 | Causal chain reconstruction | Built |
| ratchet.rs | ~150 | Rung regression prevention | Built |
| eval_generator.rs | ~250 | Dynamic check generation | Built |
| env_builder.rs | ~150 | Gate environment construction | Built |
| payload.rs | ~200 | GatePayload, BuildSystem, TestSelector | Stable |
| pelt.rs | ~200 | Offline change-point detection | Built |
| review_verdict.rs | ~300 | Structured review parsing | Built |
| verdict_publisher.rs | ~150 | Verdict broadcasting | Built |
| artifact_store.rs | ~150 | Content-addressed artifacts | Built |
| lib.rs | 182 | Module structure + re-exports | Stable |
| **Total** | **~20.1K** | **40 source files** | |

---

## Sources

All findings verified against source code at the paths listed. Key files:

- `crates/roko-gate/src/lib.rs` -- module structure, gate tier definitions
- `crates/roko-gate/src/gate_service.rs` -- GateService implementation
- `crates/roko-gate/src/gate_pipeline.rs` -- GatePipeline, ComposedGatePipeline
- `crates/roko-gate/src/rung_dispatch.rs` -- run_rung, run_canonical_rung
- `crates/roko-gate/src/rung_selector.rs` -- PlanComplexity, Rung, select_rungs
- `crates/roko-gate/src/adaptive_threshold.rs` -- AdaptiveThresholds, ThresholdProfile
- `crates/roko-gate/src/spc.rs` -- CUSUM, EWMA, BOCPD, SpcDetector
- `crates/roko-gate/src/hotelling.rs` -- HotellingDetector
- `crates/roko-gate/src/feedback.rs` -- feedback_for_agent
- `crates/roko-gate/src/compile_errors.rs` -- ErrorCategory, FailureClass
- `crates/roko-gate/src/process_reward.rs` -- ProcessRewardModel
- `crates/roko-gate/src/composition.rs` -- ParallelGate, VotingGate, FallbackGate
- `crates/roko-gate/tests/gate_truth.rs` -- GateService integration tests
- `crates/roko-gate/tests/rungs.rs` -- Full 7-rung integration tests
- `crates/roko-cli/src/orchestrate.rs` -- Gate pipeline integration in plan runner
- `crates/roko-acp/src/runner.rs` -- ACP gate dispatch
- `crates/roko-cli/src/run.rs` -- Simple gate dispatch
- `crates/roko-runtime/src/workflow_engine.rs` -- Workflow engine gate dispatch
