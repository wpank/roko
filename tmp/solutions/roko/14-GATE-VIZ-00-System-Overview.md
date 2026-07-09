# PRD-00 — Unified Evaluation Architecture: System Overview

**Status**: Draft
**Author**: Will (architect) + Claude (synthesis)
**Date**: 2026-04-25 (revised 2026-04-29)
**Supersedes**: `tmp/visual-gate/prd/PRD-00` through `PRD-06`, `tmp/visual-gate/PRD.md`

---

## 1. What This Is

A ground-up redesign of how roko evaluates artifacts. Today roko has five disconnected evaluation mechanisms: the 7-rung gate pipeline, the LLM judge gate, the `quality_judge` free function, the vision loop prototype, and the process reward model. Each uses different types, different score ranges, different composition rules, and none are user-extensible.

This PRD set replaces all of them with a single composable evaluation framework built on three primitives:

```
EvidenceCollector  ->  produces structured evidence from any artifact
Criterion          ->  scores one dimension given evidence
Profile            ->  composes criteria into a named, shareable evaluation strategy
```

Every evaluation in the system -- compile checks, visual quality, accessibility audits, LLM judgments, performance benchmarks, security scans, research quality -- becomes a composition of these three primitives. Users author, share, fork, and compose evaluation criteria the same way DAW users work with presets and plugins.

---

## 2. Why Redesign

### 2.1 The Five Evaluation Systems Problem

| System | Location | Score Type | Composable? | User-Extensible? | Learnable? |
|---|---|---|---|---|---|
| 7-rung gate pipeline | `roko-gate` | `Verdict { score: f32 }` [0..1] | Via `GatePipeline`, `ParallelGate`, `VotingGate` | No -- hardcoded `match rung` dispatch | Via `AdaptiveThresholds` |
| `LlmJudgeGate` | `roko-gate/llm_judge_gate.rs` | `f32` [0..1] via `JudgeOracle` | No -- standalone gate | No | No |
| `judge_quality()` | `roko-learn/quality_judge.rs` | `f64` [0..1] | No -- free function | No | No |
| Vision loop | `roko-cli/src/vision_loop/` | `f64` [1..10] | No -- entirely parallel system | No | Via iteration history |
| `ProcessRewardModel` | `roko-gate/process_reward.rs` | `f64` [0..1] | No -- standalone | No | Accumulates snapshots |

Five mechanisms, three different score ranges, zero shared infrastructure for composition, judging, or learning.

### 2.2 The Gate Service Bottleneck

The current `GateService` (at `crates/roko-gate/src/gate_service.rs`) demonstrates the architectural pain. It maps gate names to concrete implementations via a hardcoded `match`:

```rust
// Current: gate_service.rs lines 67-83 -- brittle dispatch
fn gate_for_name(&self, name: &str, build_system: BuildSystem) -> Option<Box<dyn Verify>> {
    match name {
        "compile" | "compile:cargo" => Some(Box::new(CompileGate::new(build_system))),
        "clippy" | "clippy:cargo" => Some(Box::new(ClippyGate::new(build_system))),
        "test" | "test:cargo" => Some(Box::new(TestGate::new(build_system))),
        // ...every new gate requires modifying this match
        _ => None,
    }
}
```

Adding a new gate means editing `gate_service.rs`, `rung_dispatch.rs`, and updating the rung mapping. The new system replaces this with a registry-driven approach where criteria are discovered and composed at configuration time.

### 2.3 What's Good (Keep)

- **`Verify` trait** -- `async fn verify(&self, engram: &Engram, ctx: &Context) -> Verdict` is the right shape. We extend it, not replace it.
- **Composition wrappers** -- `ParallelGate`, `VotingGate`, `FallbackGate`, `GatePipeline` (in `crates/roko-gate/src/composition.rs` and `gate_pipeline.rs`) are well-designed. They become implementations of `Profile` composition strategies.
- **`AdaptiveThresholds`** -- EMA + CUSUM + SPC + Hotelling is sophisticated (in `crates/roko-gate/src/adaptive_threshold.rs`). It becomes the default learning strategy for `Profile`s.
- **`Score` (7-axis)** -- `confidence x (1+novelty) x (1+utility) x reputation x salience x coherence` is rich. `Criterion` outputs are projected into `Score` space.
- **`Engram` as universal carrier** -- content-addressed, scored, decaying, with lineage. Evidence and criterion results are both engrams.
- **`ProcessRewardModel`** -- step-level Promise/Progress signals (in `crates/roko-gate/src/process_reward.rs`) become a `TrajectoryScorer` criterion.
- **Structured compile errors** -- `compile_errors.rs` with `ErrorCategory`, `FailureClass`, `GateFailureClassification` carries over directly as the structured finding model for `CompileCriterion`.

### 2.4 What Changes

- **Rung dispatch becomes a registry** -- `match rung { Compile => CompileGate::cargo(), ... }` (in `rung_dispatch.rs`) becomes `HashMap<RungId, Vec<CriterionRef>>` loaded from config + plugins.
- **Gates become Criterion compositions** -- `CompileGate` becomes a `Criterion` that requires `ProcessEvidence`. `LlmJudgeGate` becomes a `Criterion` that requires `DiffEvidence` + `TaskEvidence`.
- **Vision loop merges into the framework** -- `VisionEvaluator` becomes an `EvidenceCollector` (screenshot) + a `Criterion` (visual quality judge). Its iteration/regression logic becomes a `Profile` execution strategy.
- **Score ranges unify to [0..1]** -- all criterion outputs are `f64` in `[0..1]`. The vision loop's 1-10 scale is normalized.
- **Everything is user-extensible** -- criteria, profiles, evidence collectors are authored in TOML + optional scripts, published to a marketplace, installed and forked.

---

## 3. Architecture

### 3.1 The Three Primitives

```
+----------------------------------------------------------------------+
|                        Artifact Under Test                           |
|  (URL, screenshot, HTML, diff, process output, API response, ...)    |
+-------------------------------+--------------------------------------+
                                |
                    +-----------v-----------+
                    |  EvidenceCollector(s)  |
                    |  browser, screenshot,  |
                    |  process, http, static |
                    |  AST, runtime trace    |
                    +-----------+-----------+
                                |
                    +-----------v-----------+
                    |    Evidence Bag        |
                    |  screenshots, DOM,     |
                    |  a11y tree, console,   |
                    |  network, perf, diff,  |
                    |  stdout, AST, traces,  |
                    |  semantic diff, ...    |
                    +-----------+-----------+
                                |
              +-----------------+------------------+
              |                 |                   |
    +---------v------+  +------v-------+  +--------v--------+
    |  Criterion A   |  | Criterion B  |  |  Criterion C    |
    |  (deterministic|  | (LLM judge)  |  |  (computed      |
    |   e.g. APCA)   |  |              |  |   e.g. CWV)     |
    +---------+------+  +------+-------+  +--------+--------+
              |                |                    |
    +---------v----------------v--------------------v---------+
    |                    Profile                               |
    |  composition strategy: conjunctive hard + Pareto soft    |
    |  thresholds, retry policy, feedback format               |
    +---------------------------+------------------------------+
                                |
                       +--------v--------+
                       |    Verdict      |
                       |  passed, score, |
                       |  findings,      |
                       |  trace, feedback|
                       +-----------------+
```

### 3.2 How It Maps to the Existing System

| Old Concept | New Concept | Migration Path |
|---|---|---|
| `Verify` trait | `Criterion` trait (+ `EvidenceCollector`) | `Verify` gains a default impl that wraps the new system. Existing gates keep working via `LegacyCriterion` adapter. |
| `GatePipeline` | `Profile` with sequential strategy | 1:1 mapping |
| `ParallelGate` | `Profile` with parallel strategy | 1:1 mapping |
| `VotingGate` | `Profile` with voting strategy | 1:1 mapping |
| `FallbackGate` | `Profile` with fallback strategy | 1:1 mapping |
| `Rung` enum (7 variants) | `RungId` (string-based, extensible) | Existing rungs become built-in `RungId`s |
| `run_canonical_rung` | Registry lookup + criterion instantiation | Hardcoded match -> config-driven |
| `AdaptiveThresholds` | `ProfileLearner` (per-profile, per-criterion) | Preserved, generalized |
| `VisionEvaluator` | `BrowserCollector` + `VisualQualityCriterion` | Absorbed |
| `LlmJudgeGate` | `JudgePanelCriterion` | Upgraded to pairwise BT |
| `quality_judge()` | `SingleJudgeCriterion` | Absorbed |
| `ProcessRewardModel` | `TrajectoryScorer` criterion | Absorbed |
| `GateService` | `EvalService` | Registry-driven dispatch replaces `gate_for_name` match |
| `GateConfig` | `EvalConfig` | Extends with profile references, evidence config |
| `GateReport` | `EvalReport` | Carries `EvalTrace` per-criterion |
| `GatePayload` | `ArtifactRef` | Unified artifact reference |
| `CompileGate::summarize_errors` | `CompileCriterion` + `Finding` | Structured findings replace flat strings |
| `RungStats` | `CriterionStats` | Per-criterion instead of per-rung |
| `GateVerdict` | `CriterionResult` + `EvalVerdict` | Richer finding model |

### 3.3 Integration with GateService (Bridge Architecture)

The migration is incremental. A `BridgeGateService` wraps the new `EvalService` behind the existing `GateRunner` trait so the orchestrator (`crates/roko-cli/src/orchestrate.rs`) continues to call `run_gates()` without changes.

```rust
/// Bridge: wraps EvalService behind the existing GateRunner trait.
///
/// Lives in `crates/roko-gate/src/bridge.rs`. This is the migration adapter
/// that lets orchestrate.rs call the new evaluation system through the old
/// GateRunner interface.
///
/// File: crates/roko-gate/src/bridge.rs
pub struct BridgeGateService {
    eval_service: EvalService,
    /// Fallback to old GateService for gates not yet migrated.
    legacy: GateService,
    /// Which gate names have been migrated to criteria.
    migrated: HashSet<String>,
}

#[async_trait]
impl GateRunner for BridgeGateService {
    async fn run_gates(&self, config: GateConfig) -> Result<GateReport> {
        let mut verdicts = Vec::new();

        for gate_name in GateService::ordered_gate_names(&config) {
            if self.migrated.contains(&gate_name) {
                // Route through new EvalService
                let artifact = ArtifactRef::from_gate_config(&config, &gate_name);
                let result = self.eval_service
                    .evaluate_criterion(&gate_name, &artifact)
                    .await?;
                verdicts.push(result.into_gate_verdict(gate_name));
            } else {
                // Fallback to legacy GateService for unmigrated gates
                let legacy_report = self.legacy.run_gates(GateConfig {
                    enabled_gates: vec![gate_name],
                    ..config.clone()
                }).await?;
                verdicts.extend(legacy_report.verdicts);
            }

            // Preserve short-circuit semantics
            if let Some(last) = verdicts.last() {
                if !last.passed && !last.skipped {
                    break;
                }
            }
        }

        Ok(GateReport { verdicts })
    }
}
```

### 3.4 Crate Structure

```
crates/
  roko-eval/                    # Layer 1: traits + core types
    src/
      lib.rs
      evidence.rs               # EvidenceCollector trait, Evidence bag types
      criterion.rs              # Criterion trait, CriterionResult, Finding
      profile.rs                # Profile, CompositionStrategy, Verdict
      artifact.rs               # ArtifactRef (URL, path, screenshot, diff, etc.)
      registry.rs               # CriterionRegistry, ProfileRegistry
      trace.rs                  # EvalTrace for flywheel
      config.rs                 # TOML schema for profiles and criteria
      bridge.rs                 # LegacyCriterion adapter for existing Verify impls
      service.rs                # EvalService: the new runtime
      stats.rs                  # CriterionStats (replaces per-rung RungStats)

  roko-eval-browser/            # Browser evidence collector
    src/
      lib.rs
      playwright.rs             # Playwright backend
      chromiumoxide.rs          # Rust-native CDP backend (target)
      dev_server.rs             # DevServerHandle (RAII)
      journey.rs                # Step execution, locator resolution
      metrics.rs                # 15 computational metrics
      accessibility.rs          # axe-core + IBM Equal Access

  roko-eval-judge/              # LLM judge criteria
    src/
      lib.rs
      panel.rs                  # Disjoint-family judge panel
      pairwise.rs               # Bradley-Terry MLE, position swap
      absolute.rs               # Fallback absolute scoring
      rubric.rs                 # Rubric definition, 7-dimension visual
      aggregation.rs            # Trimmed mean, bootstrap CI
      prompt.rs                 # Judge prompt templates

  roko-eval-metrics/            # Built-in deterministic criteria
    src/
      lib.rs
      apca.rs                   # APCA contrast
      cwv.rs                    # Core Web Vitals
      aim.rs                    # AIM computational aesthetics
      token_adherence.rs        # Design token adherence
      regression.rs             # odiff + dssim visual regression
      layout.rs                 # Grid, alignment, density, balance
      color.rs                  # Colorfulness, palette compactness
      saliency.rs               # DeepGaze IIE + UMSI++
      reduced_motion.rs         # Reduced-motion compliance
      a11y.rs                   # Accessibility criteria (WCAG, ARIA)
      compile.rs                # CompileCriterion (absorbs CompileGate)
      lint.rs                   # LintCriterion (absorbs ClippyGate)
      test.rs                   # TestCriterion (absorbs TestGate)
      security.rs               # SecurityCriterion (absorbs SecurityScanGate)
      diff.rs                   # DiffCriterion (absorbs DiffGate)
      symbol.rs                 # SymbolCriterion (absorbs SymbolGate)
      format.rs                 # FormatCriterion (absorbs FormatCheckGate)
      ast_analysis.rs           # AST-based structural analysis
      semantic_diff.rs          # Semantic diff (AST-level change detection)
      runtime_trace.rs          # Runtime tracing evidence analysis

  roko-eval-community/          # Marketplace + registry
    src/
      lib.rs
      publish.rs                # Publish criteria/profiles
      install.rs                # Install/fork
      registry_client.rs        # Registry API client
      attribution.rs            # Fork chains, provenance
      validation.rs             # Criterion validation before publish

  roko-viz/                     # CLI commands (roko viz fb, roko viz insp)
    src/
      lib.rs
      feedback.rs               # roko viz fb
      inspiration.rs            # roko viz insp
      report.rs                 # Markdown/JSON report generation
      batch.rs                  # Multi-page batch analysis
```

### 3.5 Relationship to Existing Crates

```
roko-eval (new)
  +-- depends on: roko-core (Engram, Score, Verdict, Context)
  +-- consumed by: roko-gate (bridge trait, migration adapter)
  +-- consumed by: roko-cli (viz commands, orchestrate.rs integration)
  +-- consumed by: roko-serve (HTTP routes)

roko-eval-browser (new)
  +-- depends on: roko-eval
  +-- consumed by: roko-eval-metrics, roko-viz

roko-eval-judge (new)
  +-- depends on: roko-eval, roko-agent (for LLM dispatch)
  +-- consumed by: profiles that include judge criteria

roko-eval-metrics (new)
  +-- depends on: roko-eval
  +-- contains: migrated versions of CompileGate, ClippyGate, TestGate, etc.
  +-- consumed by: roko-gate (via registry), profiles

roko-eval-community (new)
  +-- depends on: roko-eval
  +-- consumed by: roko-cli, roko-serve, nunchi-dashboard

roko-gate (existing, modified)
  +-- adds: bridge.rs (BridgeGateService wrapping EvalService)
  +-- adds: legacy_adapter.rs (LegacyCriterion wrapping existing Verify impls)
  +-- existing gates: untouched during migration, deprecated after
  +-- GateService: continues to work for unmigrated gates

roko-viz (new)
  +-- depends on: roko-eval, roko-eval-browser, roko-eval-judge, roko-eval-metrics
  +-- consumed by: roko-cli
```

### 3.6 Layer Dependencies (Must Pass layer_check.rs)

```
Layer 0: roko-primitives, roko-core
Layer 1: roko-eval                        (depends only on Layer 0)
Layer 2: roko-eval-metrics, roko-eval-browser  (depends on Layer 0-1)
Layer 3: roko-eval-judge                  (depends on Layer 0-2 + roko-agent)
Layer 4: roko-gate (bridge)               (depends on Layer 0-3)
Layer 5: roko-cli, roko-serve             (depends on Layer 0-4)
```

---

## 4. Design Principles

### 4.1 Deterministic First, Subjective Last

From Song et al. (ICLR 2025): self-improvement only works when verification exceeds generation difficulty. Browser evidence (DOM, computed styles, network logs) is a stronger verifier than any LLM. Deterministic criteria run first and gate access to expensive subjective criteria.

The sequential composition strategy enforces this ordering. A `Profile` with `Sequential` strategy runs criteria in declared order and short-circuits on the first hard failure, matching the existing `GatePipeline` behavior.

### 4.2 Conjunctive Hard, Pareto Soft (Never Weighted-Sum)

From Moskovitz et al. (ICLR 2024): weighted-sum amplifies Goodhart's Law. Hard criteria (accessibility, console errors, compile) are conjunctive -- all must pass independently. Soft criteria (visual polish, layout aesthetics, saliency) use Pareto composition -- no single score compensates for another's failure.

There is no `WeightedSum` variant in `CompositionStrategy`. This is deliberate and load-bearing.

### 4.3 Disjoint Judges, Pairwise Comparison

From Chen et al. (ICML 2024, MLLM-as-a-Judge): pairwise comparison achieves ~0.6-0.7 human agreement vs Pearson ~0.49 for absolute scoring. From Verga et al. (2024, PoLL): panel of diverse-family models outperforms single large judge at 7x lower cost. Judge families must be disjoint from the generator family (Wataoka et al.: self-preference correlates with low perplexity-on-self).

### 4.4 Every Evaluation Produces a Trace

Every criterion evaluation emits an `EvalTrace` -- input evidence, criterion config, score, findings, duration, cost. Traces feed the flywheel: preference mining, curriculum-from-failures, MIPROv2 optimization, RFT post-fixer training. The framework's value compounds over time.

### 4.5 The Same Abstraction Everywhere

The same `Criterion` runs in:
- The gate pipeline (per-task verification during plan execution)
- `roko viz fb` (standalone feedback on any artifact)
- `roko viz insp` (design system extraction)
- The arena (competitive benchmarking)
- The dashboard (Evals Library, Runner, Calibration)
- CI/CD (`--check` flag for nonzero exit on failure)

One abstraction, many contexts. Improvements to a criterion propagate to all contexts.

### 4.6 DAW-Level Composability

From Will's design philosophy: "Things should be reusable, modular, composable, so that people can create their own primitives to reuse and share, similar to presets or templates, so that things become like modular plugins that you can use as lego pieces."

Criteria are plugins. Profiles are presets. The registry is the marketplace. Fork is the fundamental operation.

### 4.7 Evidence Is Separate from Judgment

This is the key architectural insight that the current gate system lacks. In `CompileGate`, evidence collection (running `cargo check`) and judgment (interpreting the exit code) are fused in a single `verify()` call. In the new system, `ProcessCollector` runs the command and `CompileCriterion` interprets the result. This separation enables:

1. **Evidence reuse** -- multiple criteria share the same process output.
2. **Evidence caching** -- unchanged artifacts reuse cached evidence.
3. **Failure isolation** -- infrastructure failures (can't run the process) are distinguished from evaluation failures (the process reported errors).
4. **Testability** -- criteria can be tested with synthetic evidence without running real processes.

---

## 5. Implementation Plan

### 5.1 Phase 1: Core Types + Bridge (Week 1-2)

**Goal**: `roko-eval` crate compiles and the bridge adapter lets `orchestrate.rs` use it through the existing `GateRunner` interface.

| Step | File | What |
|---|---|---|
| 1 | `crates/roko-eval/Cargo.toml` | New crate, depends on `roko-core` |
| 2 | `crates/roko-eval/src/evidence.rs` | `EvidenceKind`, `Evidence`, `EvidenceData`, `EvidenceBag` |
| 3 | `crates/roko-eval/src/criterion.rs` | `Criterion` trait, `CriterionResult`, `Finding`, `CriterionKind`, `Severity` |
| 4 | `crates/roko-eval/src/artifact.rs` | `ArtifactRef`, `ProcessCapture`, `HttpEndpoint` |
| 5 | `crates/roko-eval/src/profile.rs` | `Profile`, `CompositionStrategy`, `RetryPolicy`, `EvalVerdict` |
| 6 | `crates/roko-eval/src/trace.rs` | `EvalTrace` |
| 7 | `crates/roko-eval/src/registry.rs` | `CriterionRegistry`, `ProfileRegistry` |
| 8 | `crates/roko-eval/src/bridge.rs` | `LegacyCriterion` (wraps `Verify` -> `Criterion`) |
| 9 | `crates/roko-eval/src/service.rs` | `EvalService` runtime |
| 10 | `crates/roko-gate/src/bridge.rs` | `BridgeGateService` (wraps `EvalService` -> `GateRunner`) |

### 5.2 Phase 2: Code Criteria Migration (Week 3-4)

**Goal**: CompileGate, ClippyGate, TestGate, DiffGate, SymbolGate, FormatCheckGate, SecurityScanGate all have criterion equivalents in `roko-eval-metrics`.

| Step | File | Migrates |
|---|---|---|
| 1 | `crates/roko-eval-metrics/src/compile.rs` | `CompileGate` |
| 2 | `crates/roko-eval-metrics/src/lint.rs` | `ClippyGate` |
| 3 | `crates/roko-eval-metrics/src/test.rs` | `TestGate` |
| 4 | `crates/roko-eval-metrics/src/diff.rs` | `DiffGate` |
| 5 | `crates/roko-eval-metrics/src/symbol.rs` | `SymbolGate` |
| 6 | `crates/roko-eval-metrics/src/format.rs` | `FormatCheckGate` |
| 7 | `crates/roko-eval-metrics/src/security.rs` | `SecurityScanGate` |

### 5.3 Phase 3: Novel Evidence Collectors (Week 5-6)

**Goal**: AST analysis, semantic diff, and runtime tracing collectors produce structured evidence that criteria can consume.

| Step | File | What |
|---|---|---|
| 1 | `crates/roko-eval-metrics/src/ast_analysis.rs` | Tree-sitter-based AST structural analysis |
| 2 | `crates/roko-eval-metrics/src/semantic_diff.rs` | AST-level semantic change detection |
| 3 | `crates/roko-eval-metrics/src/runtime_trace.rs` | Runtime tracing evidence analysis |

### 5.4 Phase 4: Judge + Visual Criteria (Week 7-8)

**Goal**: Judge panel criteria and visual computational metrics.

### 5.5 Phase 5: Community + Dashboard (Week 9-10)

**Goal**: Marketplace, profiles, dashboard integration.

---

## 6. Document Index

| PRD | Title | What It Covers |
|---|---|---|
| **00** (this) | System Overview | Architecture, motivation, crate structure, design principles, implementation plan |
| **01** | Core Abstractions | `EvidenceCollector`, `Criterion`, `Profile`, `Verdict` traits and types with full Rust signatures |
| **02** | Evidence Collectors | Browser, Screenshot, HTTP, Process, Static-Analysis, AST, Semantic Diff, Runtime Tracing backends |
| **03** | Criterion Library | Built-in criteria, authoring format, the 15 visual metrics, code gates, novel AST criteria |
| **04** | Judge Methodology | Pairwise BT, disjoint panels, statistical rigor, Goodhart resistance |
| **05** | Self-Improvement Flywheel | Preference mining, curriculum, MIPROv2, RFT post-fixer |
| **06** | Community Marketplace | Publish/install/fork criteria & profiles, registry, attribution |
| **07** | Dashboard Integration | Evals Library, Runner, Calibration, Arena, viz result views, component specs |
| **08** | Migration & Orchestration | How existing gates migrate, rung dispatch redesign, CLI commands |
| **09** | Research Appendix | Full citations, statistical methodology, Goodhart taxonomy |

---

## 7. Non-Goals

- **Pixel-perfect visual regression** as the primary evaluation method -- LPIPS and dssim are criteria, not the framework's identity
- **Custom judge model training in year one** -- use frontier models and open-weight judges; RFT post-fixer is the training target
- **Non-web artifact evaluation in MVP** -- the framework supports it by design, but MVP focuses on web UIs and code
- **Replacing application test suites** -- criteria verify agent output quality, not application correctness
- **Real-time streaming evaluation** -- batch evaluation per-task or per-artifact; streaming is a future extension
- **Breaking the existing GateRunner interface** -- the bridge adapter ensures backward compatibility throughout migration

---

## 8. Success Metrics

| Metric | Target | How Measured |
|---|---|---|
| Time to first evaluation | < 5 min from `roko init` | Onboarding funnel |
| Gate migration coverage | 100% of existing gates wrapped as criteria | Unit tests per criterion matching existing gate behavior |
| Evaluation consistency | Judge panel Krippendorff alpha >= 0.7 | Canary set monitoring |
| False positive rate | < 5% on canary set | Quarterly canary evaluation |
| Evidence collection latency | < 2s for process evidence, < 10s for browser evidence | Trace duration histograms |
| Criteria in marketplace | 100+ within 6 months | Registry count |
| Profiles in marketplace | 50+ within 6 months | Registry count |
| Community forks | 500+ within 6 months | Registry fork count |
| Flywheel data generation | 10k+ preference triples in 6 months | Trace collection |
| Dashboard eval completion | All 6 measurement pages functional | Feature flag rollout |
| Bridge backward compat | Zero regressions in orchestrate.rs gate behavior | Existing gate tests pass through bridge |

---

## 9. Risks and Mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| Bridge adapter introduces latency | Gate pipeline slows down | Benchmark bridge vs direct GateService; bridge adds only artifact conversion |
| Evidence collection infrastructure is fragile | Browser crashes, dev server hangs | RAII handles (DevServerHandle), process group isolation, retry with exponential backoff |
| LLM judge cost scales with evaluation volume | Budget overrun | Sequential composition: deterministic criteria gate access to judge criteria; per-profile budget caps |
| Criterion registry becomes a security surface | Malicious criteria | Sandboxed execution for community criteria, signature verification, audit trail |
| Migration takes longer than planned | Old and new systems coexist too long | Each phase is independently valuable; bridge adapter makes coexistence safe |
