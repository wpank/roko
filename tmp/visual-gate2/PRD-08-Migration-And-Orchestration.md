# PRD-08 -- Migration, Orchestration Wiring, and CLI Commands

**Status**: Draft v2 (expanded)
**Author**: Will (architect) + Claude (synthesis)
**Date**: 2026-04-29
**Prerequisites**: PRD-00 (System Overview), PRD-01 (Core Abstractions)
**Depends on**: `roko-gate` (existing), `roko-eval` (PRD-01), `roko-cli` (orchestrate.rs)

---

## 0. Scope

This document covers three things:

1. **Migration** -- how the existing 7-rung gate pipeline (`roko-gate`) migrates to the new eval framework (`roko-eval`) without breaking existing workflows at any point.
2. **Orchestration wiring** -- how `roko-eval` integrates with `orchestrate.rs`, the plan executor, retry feedback, learning, and task specs.
3. **CLI commands** -- the `roko eval` and `roko viz` command families.

The migration is progressive, not big-bang. At every intermediate state, `cargo test --workspace` passes, `roko plan run` works identically to before, and no user-visible behavior changes without opt-in.

---

## 1. Current Gate Architecture (What We Are Migrating From)

### 1.1 Gate Pipeline Structure

The existing gate system is implemented across several files:

| File | Purpose | Key Types |
|---|---|---|
| `crates/roko-gate/src/gate_pipeline.rs` | Sequential gate composition | `GatePipeline`, `ComposedGatePipeline`, `GateComposition` |
| `crates/roko-gate/src/gate_service.rs` | Concrete `GateRunner` implementation | `GateService` |
| `crates/roko-gate/src/rung_dispatch.rs` | Per-rung execution with enrichment | `RungExecutionConfig`, `RungExecutionInputs`, `run_rung` |
| `crates/roko-gate/src/rung_selector.rs` | Select rungs based on task complexity | `select_rungs`, `Rung`, `RungCaps`, `PlanComplexity` |
| `crates/roko-gate/src/adaptive_threshold.rs` | EMA-based adaptive gate skipping | `AdaptiveThresholds` |
| `crates/roko-gate/src/compile.rs` | Rung 0: `cargo build` | `CompileGate` |
| `crates/roko-gate/src/clippy_gate.rs` | Rung 1: `cargo clippy` | `ClippyGate` |
| `crates/roko-gate/src/test_gate.rs` | Rung 2: `cargo test` | `TestGate` |
| `crates/roko-gate/src/diff_gate.rs` | Rung 3: `git diff --stat` | `DiffGate` (via `ShellGate`) |
| `crates/roko-gate/src/format_check_gate.rs` | Rung 4: `cargo fmt --check` | `FormatCheckGate` |
| `crates/roko-gate/src/shell.rs` | Rung 5: custom shell commands | `ShellGate` |
| `crates/roko-gate/src/llm_judge_gate.rs` | Rung 6: LLM judge (stub) | `JudgeOracle`, `JudgePayload` |
| `crates/roko-gate/src/symbol_gate.rs` | Symbol manifest verification | `SymbolGate`, `SymbolManifest` |
| `crates/roko-gate/src/generated_test_gate.rs` | Run generated test suites | `GeneratedTestGate` |
| `crates/roko-gate/src/benchmark_gate.rs` | Performance regression detection | `BenchmarkGate` |
| `crates/roko-gate/src/security_scan_gate.rs` | Security vulnerability scanning | `SecurityScanGate` |

### 1.2 Core Traits

The gate system is built on two traits from `roko-core`:

```rust
// crates/roko-core/src/lib.rs
#[async_trait]
pub trait Verify: Send + Sync {
    async fn verify(&self, signal: &Engram, ctx: &Context) -> Verdict;
    fn name(&self) -> &str;
}

pub struct Verdict {
    pub gate: String,
    pub passed: bool,
    pub reason: String,
    pub detail: Option<String>,
    pub duration_ms: u64,
    pub test_count: Option<TestCount>,
    pub error_digest: Option<String>,
}
```

And the `GateRunner` trait from `roko-core::foundation`:

```rust
#[async_trait]
pub trait GateRunner: Send + Sync {
    async fn run_gates(&self, config: GateConfig) -> Result<GateReport>;
}

pub struct GateConfig {
    pub workdir: PathBuf,
    pub enabled_gates: Vec<String>,
    pub shell_gates: Vec<ShellGateCommand>,
    pub max_rung: Option<u8>,
}

pub struct GateReport {
    pub verdicts: Vec<GateVerdict>,
}
```

### 1.3 Gate Composition Modes

The `ComposedGatePipeline` in `gate_pipeline.rs` supports four composition modes:

- **Sequential**: Run gates in order, short-circuit on first failure (default)
- **Parallel**: Run specified gate indices concurrently, collect all verdicts
- **Voting**: Aggregate verdicts, pass if more than threshold fraction agree
- **Fallback**: Try primary gates, then fallback gates if primary fails

These composition modes map directly to eval framework equivalents and must
be preserved through migration.

### 1.4 Orchestrate.rs Integration Points

The orchestrator (`crates/roko-cli/src/orchestrate.rs`) interfaces with gates at
these specific points:

```
Line ~8175:  run_gate_pipeline()       # Main gate invocation per task
Line ~16578: run_gate_pipeline() impl  # Builds pipeline, runs, stores results
Line ~17254: run_selected_gate_pipeline()  # Per-rung dispatch via rung_dispatch
Line ~2676:  last_gate_verdicts        # PlanTracker field, stores results
Line ~2678:  last_gate_verdict_summaries  # GateVerdictSummary for event bus
Line ~3248:  gate_verdict_signature()  # Extract failure signature for learning
Line ~3266:  is_stub_gate_verdict()    # Detect stub judge verdicts
Line ~3279:  positive_learning_withhold_reason()  # Gate verdicts -> learning filter
```

Key functions that touch gate results:

| Function | Purpose | Migration Impact |
|---|---|---|
| `run_gate_pipeline` | Build + run `GatePipeline` | Replace with `EvalRunner` |
| `run_selected_gate_pipeline` | Dispatch per-rung | Replace with `EvalProfile` dispatch |
| `gate_verdict_signature` | Extract error pattern | Extend for `CriterionResult` |
| `is_stub_gate_verdict` | Filter stub verdicts | Remove once judge is real |
| `positive_learning_withhold_reason` | Filter learning signals | Extend for eval verdicts |
| `summarize_runtime_verdicts` | Build `GateVerdictSummary` | Add `EvalTraceSummary` |

### 1.5 Learning Integration Points

Gate verdicts flow into the learning system at:

| Component | File | What it receives |
|---|---|---|
| `EpisodeLogger` | `crates/roko-learn/src/episode_logger.rs` | `gate_verdicts: Vec<GateVerdict>` |
| `ErrorPatternStore` | `crates/roko-learn/src/error_pattern_store.rs` | `GateFailureObservation` |
| `PlaybookStore` | `crates/roko-learn/src/playbook.rs` | Gate pass/fail -> playbook scoring |
| `SectionEffectiveness` | `crates/roko-learn/src/section_effect.rs` | Verdict -> prompt section feedback |
| `SkillLibrary` | `crates/roko-learn/src/skill_library.rs` | `SkillGateResult` |
| `AdaptiveThresholds` | `crates/roko-gate/src/adaptive_threshold.rs` | Per-rung pass/fail EMA |
| `BudgetGuardrail` | `crates/roko-learn/src/budget.rs` | Gate failure -> budget decisions |
| `ConductorBandit` | `crates/roko-learn/src/conductor.rs` | Failure patterns -> retry strategy |

---

## 2. Migration Strategy

### 2.1 Progressive Migration (Not Big Bang)

The migration proceeds in five phases. Each phase is a self-contained PR that
leaves the system in a fully working state. There is no "flag day" where
everything switches at once.

### Phase 1: Coexistence (PR 1)

**Goal**: `roko-eval` crate exists in the workspace, compiles, has tests, but nothing calls it.

**Changes**:

```
Cargo.toml (workspace members):
  + "crates/roko-eval"

New files:
  crates/roko-eval/Cargo.toml
  crates/roko-eval/src/lib.rs           # Re-exports
  crates/roko-eval/src/types.rs         # EvidenceBag, ArtifactRef, CriterionResult,
                                        # Finding, EvalVerdict, EvalTrace
  crates/roko-eval/src/criterion.rs     # Criterion trait
  crates/roko-eval/src/profile.rs       # Profile (ordered list of criteria)
  crates/roko-eval/src/collector.rs     # EvidenceCollector trait
  crates/roko-eval/src/runner.rs        # EvalRunner (executes profile against evidence)
  crates/roko-eval/src/registry.rs      # EvalRegistry (criterion + collector lookup)
```

**Dependencies**:

```toml
[dependencies]
roko-core = { path = "../roko-core" }
async-trait = "0.1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
toml = "0.8"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v4"] }
```

**Acceptance criteria**:
- `cargo build --workspace` succeeds
- `cargo test -p roko-eval` passes (unit tests for types, serialization, registry)
- No changes to any existing file
- `roko plan run` works identically

### Phase 2: Bridge Layer (PR 2)

**Goal**: Bidirectional adapters between `Verify`/`Verdict` and `Criterion`/`CriterionResult`.

**New files**:

```
crates/roko-eval/src/bridge.rs          # LegacyCriterion<V>, CriterionGate<C>
crates/roko-eval/src/bridge_tests.rs    # Parity verification tests
```

**Bridge adapter design**:

```rust
// crates/roko-eval/src/bridge.rs

/// Wraps an existing Verify implementor as a Criterion.
/// This allows existing gates to participate in eval profiles
/// without any modification to the gate itself.
pub struct LegacyCriterion<V: Verify> {
    inner: V,
    category: CriterionCategory,
}

impl<V: Verify> LegacyCriterion<V> {
    pub fn new(inner: V, category: CriterionCategory) -> Self {
        Self { inner, category }
    }
}

#[async_trait]
impl<V: Verify> Criterion for LegacyCriterion<V> {
    async fn evaluate(&self, evidence: &EvidenceBag, ctx: &EvalContext) -> CriterionResult {
        // Convert EvidenceBag to Engram + Context for the legacy gate
        let (engram, core_ctx) = evidence.to_legacy_inputs();
        let verdict = self.inner.verify(&engram, &core_ctx).await;

        CriterionResult {
            criterion_name: self.inner.name().to_string(),
            passed: verdict.passed,
            score: if verdict.passed { Some(1.0) } else { Some(0.0) },
            findings: verdict_to_findings(&verdict),
            evidence_consumed: evidence.artifact_count(),
            duration_ms: verdict.duration_ms,
            detail: verdict.detail,
        }
    }

    fn name(&self) -> &str {
        self.inner.name()
    }

    fn category(&self) -> CriterionCategory {
        self.category
    }
}

/// Wraps a Criterion as a Verify implementor.
/// This allows new criteria to be used in existing GatePipeline
/// compositions without changing the pipeline code.
pub struct CriterionGate<C: Criterion> {
    inner: C,
}

#[async_trait]
impl<C: Criterion> Verify for CriterionGate<C> {
    async fn verify(&self, signal: &Engram, ctx: &Context) -> Verdict {
        let evidence = EvidenceBag::from_legacy(signal, ctx);
        let eval_ctx = EvalContext::from_core(ctx);
        let result = self.inner.evaluate(&evidence, &eval_ctx).await;

        Verdict {
            gate: result.criterion_name,
            passed: result.passed,
            reason: result.summary_reason(),
            detail: result.detail,
            duration_ms: result.duration_ms,
            test_count: result.extract_test_count(),
            error_digest: result.extract_error_digest(),
        }
    }

    fn name(&self) -> &str {
        self.inner.name()
    }
}
```

**Parity test pattern**:

```rust
#[tokio::test]
async fn legacy_criterion_matches_original_gate() {
    let gate = CompileGate::new(BuildSystem::Cargo);
    let criterion = LegacyCriterion::new(
        CompileGate::new(BuildSystem::Cargo),
        CriterionCategory::Deterministic,
    );

    let signal = test_signal_with_valid_rust_code();
    let ctx = Context::at(0);
    let evidence = EvidenceBag::from_legacy(&signal, &ctx);
    let eval_ctx = EvalContext::from_core(&ctx);

    let verdict = gate.verify(&signal, &ctx).await;
    let result = criterion.evaluate(&evidence, &eval_ctx).await;

    assert_eq!(verdict.passed, result.passed);
    assert_eq!(verdict.gate, result.criterion_name);
}
```

**Acceptance criteria**:
- `LegacyCriterion<CompileGate>` produces identical pass/fail to `CompileGate`
- `CriterionGate<CompileCriterion>` produces identical pass/fail to `CompileGate`
- All bridge tests pass
- No changes to existing gate files

### Phase 3: Native Criteria (PR 3-4)

**Goal**: Write native criterion implementations that produce richer evidence.

**New crates**:

```
crates/roko-eval-metrics/               # Deterministic criteria
  src/compile.rs                        # CompileCriterion
  src/lint.rs                           # LintCriterion (clippy equivalent)
  src/test.rs                           # TestCriterion
  src/diff.rs                           # DiffCriterion
  src/format.rs                         # FormatCriterion
  src/security.rs                       # SecurityCriterion
  src/symbol.rs                         # SymbolCriterion
  src/benchmark.rs                      # BenchmarkRegressionCriterion
  src/generated_test.rs                 # GeneratedTestCriterion

crates/roko-eval-judge/                 # LLM judge criteria
  src/panel.rs                          # JudgePanelCriterion
  src/pairwise.rs                       # Pairwise comparison logic
  src/calibration.rs                    # Panel calibration tracking
```

**Native criterion advantages over legacy bridge**:

| Aspect | Legacy Bridge | Native Criterion |
|---|---|---|
| Evidence | Single Engram (text blob) | Typed EvidenceBag with artifacts |
| Output | Binary pass/fail + text reason | Pass/fail + score + typed findings |
| Artifacts | None | Screenshots, diffs, logs as ArtifactRef |
| Structure | Free-text detail | Structured `Finding` with severity, location |
| Composition | Sequential only via GatePipeline | Profile-driven with weights |

**Native CompileCriterion example**:

```rust
// crates/roko-eval-metrics/src/compile.rs

pub struct CompileCriterion {
    build_system: BuildSystem,
    timeout: Duration,
}

#[async_trait]
impl Criterion for CompileCriterion {
    async fn evaluate(&self, evidence: &EvidenceBag, ctx: &EvalContext) -> CriterionResult {
        let workdir = evidence.workdir();
        let started = Instant::now();

        let output = Command::new("cargo")
            .args(["build", "--workspace"])
            .current_dir(workdir)
            .output()
            .await?;

        let passed = output.status.success();
        let stderr = String::from_utf8_lossy(&output.stderr);
        let duration_ms = started.elapsed().as_millis() as u64;

        let mut findings = Vec::new();
        if !passed {
            // Parse compiler errors into structured findings
            for error in parse_cargo_errors(&stderr) {
                findings.push(Finding {
                    severity: FindingSeverity::Critical,
                    category: "compile_error".into(),
                    message: error.message.clone(),
                    location: Some(SourceLocation {
                        file: error.file.clone(),
                        line: error.line,
                        column: error.column,
                    }),
                    suggestion: error.suggestion.clone(),
                });
            }
        }

        // Store compiler output as artifact
        let artifact = evidence.store_artifact(
            "compile_output.txt",
            stderr.as_bytes(),
            ArtifactKind::Log,
        )?;

        CriterionResult {
            criterion_name: "compile".into(),
            passed,
            score: if passed { Some(1.0) } else { Some(0.0) },
            findings,
            evidence_consumed: 1,
            duration_ms,
            detail: Some(stderr.to_string()),
            artifacts: vec![artifact],
        }
    }
}
```

### Phase 4: Gate Reimplementation (PR 5-6)

**Goal**: Existing gates delegate to native criteria internally without changing their public API.

**Strategy**: Each gate is reimplemented one at a time, in rung order. The public
type and `Verify` impl remain identical. The internal logic delegates to the
native criterion via `CriterionGate`.

```rust
// Before (crates/roko-gate/src/compile.rs):
pub struct CompileGate {
    build_system: BuildSystem,
}

impl CompileGate {
    pub fn new(build_system: BuildSystem) -> Self {
        Self { build_system }
    }
}

#[async_trait]
impl Verify for CompileGate {
    async fn verify(&self, signal: &Engram, ctx: &Context) -> Verdict {
        // ... 80 lines of shell invocation and parsing ...
    }
}

// After (crates/roko-gate/src/compile.rs):
pub struct CompileGate {
    inner: CriterionGate<roko_eval_metrics::CompileCriterion>,
}

impl CompileGate {
    pub fn new(build_system: BuildSystem) -> Self {
        Self {
            inner: CriterionGate::new(
                roko_eval_metrics::CompileCriterion::new(build_system)
            ),
        }
    }
}

#[async_trait]
impl Verify for CompileGate {
    async fn verify(&self, signal: &Engram, ctx: &Context) -> Verdict {
        self.inner.verify(signal, ctx).await
    }
    fn name(&self) -> &str { "compile" }
}
```

**Migration order** (one gate per sub-PR):
1. `CompileGate` -- simplest, most critical, validates the pattern
2. `ClippyGate` -- similar to compile, validates lint output parsing
3. `TestGate` -- more complex (test count extraction), validates structured output
4. `FormatCheckGate` -- trivial, validates shell command wrapping
5. `DiffGate` -- trivial shell command
6. `ShellGate` -- preserves user-defined commands
7. `SymbolGate` -- complex, validates manifest evidence
8. `GeneratedTestGate` -- complex, validates artifact store integration
9. `BenchmarkGate` -- complex, validates regression detection
10. `SecurityScanGate` -- complex, validates vulnerability finding structure
11. `LLMJudgeGate` -- replaces `StubJudgeGate` with real `JudgePanelCriterion`

**Parity test for each migration**:

```rust
#[tokio::test]
async fn compile_gate_migration_parity() {
    // Old implementation (saved as CompileGateLegacy in test module)
    let old = CompileGateLegacy::new(BuildSystem::Cargo);
    // New implementation (delegates to CompileCriterion)
    let new = CompileGate::new(BuildSystem::Cargo);

    let fixtures = vec![
        test_fixture_valid_rust(),
        test_fixture_syntax_error(),
        test_fixture_missing_dependency(),
    ];

    for fixture in fixtures {
        let signal = fixture.to_engram();
        let ctx = fixture.to_context();
        let old_v = old.verify(&signal, &ctx).await;
        let new_v = new.verify(&signal, &ctx).await;

        assert_eq!(old_v.passed, new_v.passed, "fixture: {}", fixture.name);
        assert_eq!(old_v.gate, new_v.gate);
    }
}
```

### Phase 5: Orchestrator Switchover (PR 7-8)

**Goal**: `orchestrate.rs` uses `EvalRunner` alongside (then instead of) `GatePipeline`.

**Step 1**: Dual execution (feature-flagged):

```rust
// orchestrate.rs - run_gate_pipeline()

async fn run_gate_pipeline(&mut self, plan_id: &str, rung: u32) -> Result<GateRunOutcome> {
    // Always run legacy pipeline (existing behavior)
    let legacy_outcome = self.run_legacy_gate_pipeline(plan_id, rung).await?;

    // Optionally run eval pipeline in parallel (new behavior)
    if self.config.eval.enabled {
        let eval_outcome = self.run_eval_pipeline(plan_id, rung).await?;

        // Log comparison for validation
        self.log_gate_eval_comparison(&legacy_outcome, &eval_outcome);

        // If eval.primary is set, use eval outcome for task advancement
        if self.config.eval.primary {
            return Ok(eval_outcome.to_gate_run_outcome());
        }
    }

    Ok(legacy_outcome)
}
```

**Step 2**: Config flag in `roko.toml`:

```toml
[eval]
enabled = false         # Phase 5 step 1: dual execution, log comparison
primary = false         # Phase 5 step 2: eval results drive task advancement
visual_gate = false     # Enable browser-based visual evaluation
judge_panel = false     # Enable LLM judge panel
```

**Step 3**: Full switchover (eval.primary = true):

When `eval.primary = true`:
- `run_gate_pipeline` delegates to `run_eval_pipeline`
- `EvalTrace` is stored alongside `GateVerdict` in episode
- Learning feedback receives `CriterionResult` with richer structure
- Retry logic uses `Finding` severity for replan decisions

**Step 4**: Legacy removal (future, not in this migration):

Once all users have migrated and telemetry confirms parity:
- Remove `run_legacy_gate_pipeline`
- Remove `eval.enabled` and `eval.primary` flags
- `run_gate_pipeline` is renamed to `run_eval_pipeline`
- `roko-gate` crate becomes thin re-export layer over `roko-eval-metrics`

---

## 3. Orchestration Wiring

### 3.1 EvalRunner Integration in orchestrate.rs

The `EvalRunner` replaces the ad-hoc gate pipeline construction currently scattered
across `run_gate_pipeline` and `run_selected_gate_pipeline`. The new flow:

```rust
// orchestrate.rs

struct PlanRunner {
    // ... existing fields ...

    /// Eval runner replaces ad-hoc GatePipeline construction.
    eval_runner: Option<EvalRunner>,
}

impl PlanRunner {
    async fn init_eval_runner(&mut self) -> Result<()> {
        if !self.config.eval.enabled {
            return Ok(());
        }

        let profile = self.load_eval_profile()?;
        let registry = self.build_eval_registry()?;

        self.eval_runner = Some(EvalRunner::new(profile, registry));
        Ok(())
    }

    fn load_eval_profile(&self) -> Result<EvalProfile> {
        // Load from .roko/eval/profiles/ or use default
        let profile_path = self.workdir
            .join(".roko/eval/profiles")
            .join(&self.config.eval.profile)
            .with_extension("toml");

        if profile_path.exists() {
            EvalProfile::from_toml(&std::fs::read_to_string(&profile_path)?)
        } else {
            Ok(EvalProfile::default_for_workspace(&self.workdir))
        }
    }

    fn build_eval_registry(&self) -> Result<EvalRegistry> {
        let mut registry = EvalRegistry::new();

        // Register native criteria
        registry.register_criterion("compile", CompileCriterion::cargo());
        registry.register_criterion("lint", LintCriterion::clippy());
        registry.register_criterion("test", TestCriterion::cargo());
        registry.register_criterion("diff", DiffCriterion::git());
        registry.register_criterion("format", FormatCriterion::cargo_fmt());

        // Register visual criteria if enabled
        if self.config.eval.visual_gate {
            registry.register_criterion("visual", VisualCriterion::new(
                self.browser_pool.clone(),
            ));
        }

        // Register judge criteria if enabled
        if self.config.eval.judge_panel {
            registry.register_criterion("judge", JudgePanelCriterion::new(
                self.config.eval.judge_models.clone(),
            ));
        }

        // Register legacy gates via bridge for any not yet migrated
        for gate_name in &self.config.gates.enabled_gates {
            if !registry.has_criterion(gate_name) {
                if let Some(gate) = self.gate_service.gate_for_name(gate_name) {
                    registry.register_criterion(
                        gate_name,
                        LegacyCriterion::new(gate, CriterionCategory::Deterministic),
                    );
                }
            }
        }

        Ok(registry)
    }
}
```

### 3.2 Task Spec Eval Configuration

Task specs in `tasks.toml` can override the default eval profile:

```toml
[[task]]
id = "implement-login-form"
prompt = "Implement the login form component..."

[task.eval]
profile = "web-component-strict"     # Override default profile
visual = true                        # Enable visual gate for this task
judge = true                         # Enable judge panel for this task
criteria_override = [                # Add/remove specific criteria
    "+accessibility",                # Add accessibility criterion
    "-benchmark",                    # Skip benchmark criterion
]
max_rung = 6                         # Allow all rungs including judge
budget_usd = 0.10                    # Per-task eval budget cap
```

### 3.3 Retry and Replan Integration

The existing retry logic in orchestrate.rs uses gate failure information
to build replan prompts. The eval framework provides richer information:

**Current** (gate-based):
```rust
fn build_gate_failure_plan_revision(
    verdict: &Verdict,
    task_id: &str,
) -> PlanRevisionRequest {
    PlanRevisionRequest {
        reason: format!("Gate {} failed: {}", verdict.gate, verdict.reason),
        evidence: PlanRevisionEvidence::GateFailure {
            gate: verdict.gate.clone(),
            error: verdict.reason.clone(),
            detail: verdict.detail.clone(),
        },
        strategy: ReplanStrategy::RetryWithFeedback,
    }
}
```

**Enhanced** (eval-based):
```rust
fn build_eval_failure_plan_revision(
    trace: &EvalTrace,
    task_id: &str,
) -> PlanRevisionRequest {
    let failed_criteria: Vec<&CriterionResult> = trace
        .criterion_results
        .iter()
        .filter(|cr| !cr.passed)
        .collect();

    let critical_findings: Vec<&Finding> = failed_criteria
        .iter()
        .flat_map(|cr| cr.findings.iter())
        .filter(|f| f.severity == FindingSeverity::Critical)
        .collect();

    // Richer replan context:
    // - Which specific criteria failed (not just "gate failed")
    // - Structured findings with file locations
    // - Severity-based prioritization
    // - Previous attempt comparison (if available)
    PlanRevisionRequest {
        reason: format!(
            "{} criteria failed: {}. {} critical findings.",
            failed_criteria.len(),
            failed_criteria.iter().map(|cr| &cr.criterion_name).join(", "),
            critical_findings.len(),
        ),
        evidence: PlanRevisionEvidence::EvalFailure {
            trace_id: trace.id.clone(),
            failed_criteria: failed_criteria.iter().map(|cr| {
                EvalFailureCriterion {
                    name: cr.criterion_name.clone(),
                    score: cr.score,
                    findings: cr.findings.clone(),
                    artifacts: cr.artifacts.clone(),
                }
            }).collect(),
        },
        strategy: infer_replan_strategy(&failed_criteria, &critical_findings),
    }
}

fn infer_replan_strategy(
    failed_criteria: &[&CriterionResult],
    critical_findings: &[&Finding],
) -> ReplanStrategy {
    // If only visual/judge criteria failed but deterministic passed,
    // the code works but the UI needs refinement -> RetryWithFeedback
    let deterministic_failed = failed_criteria.iter()
        .any(|cr| cr.category == CriterionCategory::Deterministic);

    if !deterministic_failed {
        return ReplanStrategy::RetryWithFeedback;
    }

    // If compile failed, no point retrying other criteria
    if failed_criteria.iter().any(|cr| cr.criterion_name == "compile") {
        return ReplanStrategy::RetryWithFeedback;
    }

    // If multiple deterministic criteria failed, consider decomposing
    if failed_criteria.len() >= 3 {
        return ReplanStrategy::DecomposeTask;
    }

    ReplanStrategy::RetryWithFeedback
}
```

### 3.4 Learning System Integration

The learning system receives richer signals from eval traces:

**Episode Logger Enhancement**:
```rust
pub struct Episode {
    // ... existing fields ...

    /// Gate verdicts (legacy, for backward compatibility)
    pub gate_verdicts: Vec<GateVerdict>,

    /// Eval trace (new, richer structure)
    pub eval_trace: Option<EvalTrace>,

    /// Individual criterion scores (for learning aggregation)
    pub criterion_scores: HashMap<String, f64>,
}
```

**Error Pattern Store Enhancement**:
```rust
pub struct GateFailureObservation {
    // ... existing fields ...

    /// Structured findings from eval criterion (new)
    pub findings: Vec<Finding>,

    /// Criterion category (deterministic, statistical, visual, judge)
    pub criterion_category: Option<CriterionCategory>,

    /// File locations from findings (for pattern matching)
    pub affected_files: Vec<String>,
}
```

**Skill Library Enhancement**:
```rust
pub struct SkillGateResult {
    // ... existing fields ...

    /// Per-criterion scores (new, for fine-grained skill assessment)
    pub criterion_scores: HashMap<String, f64>,

    /// Visual quality score (new, if visual gate was enabled)
    pub visual_score: Option<f64>,

    /// Judge panel agreement (new, if judge was enabled)
    pub judge_agreement: Option<f64>,
}
```

### 3.5 Adaptive Threshold Migration

The existing `AdaptiveThresholds` in `crates/roko-gate/src/adaptive_threshold.rs`
uses EMA per rung to skip gates with high consecutive-pass streaks. This migrates
to per-criterion adaptive thresholds:

```rust
// crates/roko-eval/src/adaptive.rs

pub struct AdaptiveCriterionThresholds {
    /// Per-criterion EMA of pass rate
    criterion_ema: HashMap<String, f64>,

    /// Consecutive passes per criterion
    consecutive_passes: HashMap<String, u32>,

    /// Skip threshold (skip if consecutive passes exceed this)
    skip_threshold: u32,

    /// Never-skip set (criteria that are always executed)
    never_skip: HashSet<String>,
}

impl AdaptiveCriterionThresholds {
    pub fn new() -> Self {
        let mut never_skip = HashSet::new();
        // Compile is never skipped (same as rung 0 in legacy system)
        never_skip.insert("compile".to_string());
        // Visual gate is never skipped (too important for UI quality)
        never_skip.insert("visual".to_string());

        Self {
            criterion_ema: HashMap::new(),
            consecutive_passes: HashMap::new(),
            skip_threshold: 20,  // Same as legacy default
            never_skip,
        }
    }

    pub fn should_skip(&self, criterion_name: &str) -> bool {
        if self.never_skip.contains(criterion_name) {
            return false;
        }
        self.consecutive_passes
            .get(criterion_name)
            .map_or(false, |&count| count >= self.skip_threshold)
    }

    pub fn observe(&mut self, criterion_name: &str, passed: bool) {
        // Reset consecutive passes on failure
        if !passed {
            self.consecutive_passes.insert(criterion_name.to_string(), 0);
        } else {
            *self.consecutive_passes
                .entry(criterion_name.to_string())
                .or_insert(0) += 1;
        }

        // Update EMA (alpha = 0.1, same as legacy)
        let alpha = 0.1;
        let current = self.criterion_ema
            .entry(criterion_name.to_string())
            .or_insert(0.5);
        *current = alpha * (if passed { 1.0 } else { 0.0 }) + (1.0 - alpha) * *current;
    }
}
```

---

## 4. CLI Commands

### 4.1 `roko eval` Command Family

```
roko eval run [<path>]                    Run default eval profile against workspace
  --profile <name>                        Evaluation profile (default: auto-detect)
  --visual                                Enable visual gate (requires browser)
  --judge                                 Enable judge panel
  --budget <usd>                          Maximum eval cost
  --output <text|json|jsonl>              Output format
  --no-cache                              Skip artifact cache, re-collect all evidence
  --parallel <n>                          Max concurrent criteria (default: 4)
  --save                                  Persist trace to .roko/eval/traces/

roko eval list                            List available eval profiles
roko eval show <profile>                  Show profile criteria and config
roko eval create <name>                   Create new profile from template
roko eval edit <name>                     Edit profile in $EDITOR

roko eval history [--limit <n>]           Show recent eval traces
roko eval trace <id>                      Show full eval trace detail
roko eval artifacts <trace-id>            List artifacts for a trace
roko eval compare <id1> <id2>             Side-by-side comparison

roko eval calibrate                       Run judge calibration suite
  --judge <model>                         Calibrate specific judge model
  --samples <n>                           Number of calibration samples

roko eval benchmark <suite>               Run benchmark suite (SWE-bench, etc.)
  --model <model>                         Model to benchmark
  --parallel <n>                          Concurrent tasks
  --budget <usd>                          Maximum budget
```

### 4.2 `roko viz` Command Family

```
roko viz screenshot <url>                 Capture screenshot
  --viewport <WxH>                        Viewport size (default: 1280x720)
  --wait <ms>                             Wait before capture (default: 2000)
  --output <path>                         Output file

roko viz diff <before> <after>            Generate visual diff
  --threshold <0.0-1.0>                   Pixel difference threshold
  --output <path>                         Output file

roko viz heatmap <screenshot>             Generate attention heatmap
  --model <model>                         Vision model for analysis
  --output <path>                         Output file

roko viz report <trace-id>                Generate HTML report from eval trace
  --output <path>                         Output file
  --open                                  Open in browser after generation
```

### 4.3 Command Implementation

New files in `crates/roko-cli/src/commands/`:

| File | Purpose |
|---|---|
| `eval.rs` | `roko eval` subcommand handlers |
| `viz.rs` | `roko viz` subcommand handlers |

Registration in `crates/roko-cli/src/commands/mod.rs`:

```rust
mod eval;
mod viz;

pub fn build_cli() -> Command {
    Command::new("roko")
        // ... existing subcommands ...
        .subcommand(eval::subcommand())
        .subcommand(viz::subcommand())
}
```

---

## 5. Configuration

### 5.1 `roko.toml` Additions

```toml
[eval]
# Enable eval framework (dual execution alongside legacy gates)
enabled = false

# Use eval results as primary (replace legacy gate results)
primary = false

# Default eval profile name (loaded from .roko/eval/profiles/)
profile = "default"

# Enable visual gate (requires browser automation)
visual_gate = false

# Enable judge panel
judge_panel = false

# Judge panel model configuration
judge_models = ["claude-opus-4", "gemini-2.5-pro"]

# Maximum eval cost per task (USD)
per_task_budget = 0.10

# Maximum eval cost per plan run (USD)
per_plan_budget = 5.00

# Artifact retention days (0 = keep forever)
artifact_retention_days = 30

# Criteria that are never adaptively skipped
never_skip_criteria = ["compile", "visual"]
```

### 5.2 Default Eval Profile

```toml
# .roko/eval/profiles/default.toml

name = "default"
description = "Standard workspace evaluation profile"

[[criteria]]
name = "compile"
category = "deterministic"
weight = 1.0
required = true

[[criteria]]
name = "lint"
category = "deterministic"
weight = 0.8

[[criteria]]
name = "test"
category = "deterministic"
weight = 1.0
required = true

[[criteria]]
name = "diff"
category = "deterministic"
weight = 0.3

[[criteria]]
name = "format"
category = "deterministic"
weight = 0.5

# Visual criteria (enabled by flag)
[[criteria]]
name = "visual"
category = "visual"
weight = 0.7
enabled_by = "eval.visual_gate"

# Judge criteria (enabled by flag)
[[criteria]]
name = "judge"
category = "judge"
weight = 0.6
enabled_by = "eval.judge_panel"
```

---

## 6. Testing Strategy

### 6.1 Parity Tests

Every migrated gate has a parity test that verifies identical behavior:

| Test Suite | Location | What it verifies |
|---|---|---|
| Bridge adapter parity | `crates/roko-eval/tests/bridge_parity.rs` | `LegacyCriterion<G>` matches `G` |
| Native criterion parity | `crates/roko-eval-metrics/tests/parity.rs` | Native vs. legacy gate |
| Orchestrator dual-mode | `crates/roko-cli/tests/eval_dual_mode.rs` | Legacy + eval agree |
| Learning signal compat | `crates/roko-learn/tests/eval_compat.rs` | Episode with eval trace |

### 6.2 Integration Tests

| Test | Location | What it verifies |
|---|---|---|
| `roko eval run` smoke | `crates/roko-cli/tests/eval_cmd.rs` | Command runs, produces trace |
| Profile loading | `crates/roko-eval/tests/profile_load.rs` | TOML parsing, validation |
| Artifact storage | `crates/roko-eval/tests/artifact_store.rs` | Write, read, GC |
| Gate service compat | `crates/roko-gate/tests/gate_truth.rs` | Existing tests still pass |

### 6.3 Performance Tests

| Test | What it measures | Threshold |
|---|---|---|
| Criterion overhead | Extra latency of criterion vs. raw gate | < 5ms per criterion |
| Evidence bag allocation | Memory overhead of EvidenceBag | < 1MB per evaluation |
| Bridge round-trip | Legacy -> Criterion -> Legacy verdict | Identical |

---

## 7. Rollback Plan

If any phase introduces regressions:

1. **Phase 1-3**: Revert the `roko-eval` crate additions. No existing code was changed.
2. **Phase 4**: Each gate migration is a separate sub-PR. Revert individual gate to
   restore original implementation. The `CriterionGate` bridge means the revert is
   one file per gate.
3. **Phase 5**: Set `eval.primary = false` in `roko.toml`. The orchestrator
   immediately falls back to legacy gate pipeline. No code revert needed.

The feature flag approach means Phase 5 is instantly reversible via config change,
which is critical for a system that develops itself.

---

## 8. Implementation Order and Timeline

| Phase | PR | Dependencies | Estimated Size |
|---|---|---|---|
| Phase 1: Coexistence | PR 1 | None | ~500 LOC new |
| Phase 2: Bridge | PR 2 | Phase 1 | ~400 LOC new |
| Phase 3a: Basic criteria | PR 3 | Phase 2 | ~800 LOC new |
| Phase 3b: Judge criteria | PR 4 | Phase 3a | ~600 LOC new |
| Phase 4a: Gates 1-5 | PR 5 | Phase 3a | ~200 LOC changed |
| Phase 4b: Gates 6-11 | PR 6 | Phase 3b | ~300 LOC changed |
| Phase 5a: Dual execution | PR 7 | Phase 4a | ~400 LOC changed |
| Phase 5b: Primary switchover | PR 8 | PR 7 validated | ~100 LOC changed |
| CLI commands | PR 9 | Phase 5a | ~600 LOC new |

Total: approximately 3,900 LOC across 9 PRs. Each PR is independently reviewable
and revertible. The system works identically at every intermediate state.

---

## 9. Risk Assessment

| Risk | Impact | Mitigation |
|---|---|---|
| Parity drift | Gate and criterion produce different results | Automated parity tests run in CI |
| Performance regression | Criterion wrapper adds latency | Performance test threshold < 5ms |
| Learning signal change | Episode logger receives different data | Backward-compat Episode fields |
| Config complexity | Too many flags confuse users | Sensible defaults, `roko eval` is opt-in |
| Judge cost explosion | Panel evaluation costs too much | Per-task and per-plan budget caps |
| Artifact disk usage | Screenshots accumulate | 30-day retention + GC in `roko knowledge gc` |
