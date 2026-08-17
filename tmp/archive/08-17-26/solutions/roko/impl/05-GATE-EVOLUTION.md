# 05 -- Gate Evolution: Unified Evaluation Framework

**Scope**: Evolve the existing `roko-gate` 7-rung pipeline into a unified evaluation
framework with typed evidence, composable criteria, LLM judge panels, a self-improvement
flywheel, community marketplace, and multi-surface dashboard integration.

**Source PRDs**: 14-GATE-VIZ-01 through 14-GATE-VIZ-07, 14-GATE-VIZ-09

**New crates**: `roko-eval`, `roko-eval-metrics`, `roko-eval-judge`, `roko-eval-community`

**Modified crates**: `roko-gate` (bridge adapters), `roko-learn` (flywheel wiring),
`roko-core` (event bus extensions), `roko-cli` (eval subcommands + TUI), `roko-serve`
(eval API routes), `roko-runtime` (new event variants)

**Target**: 48 tasks across 8 phases

---

## Phase 1 -- Core Abstractions (`roko-eval` kernel)

The trait definitions, type system, and contracts that every other phase builds on.
Everything in `crates/roko-eval/src/`. Depends on `roko-core` for `Engram`, `Score`,
`Verdict`, `Context`, `Verify`, `ContentHash`.

### T01: Scaffold `roko-eval` crate with core types

**File**: `crates/roko-eval/Cargo.toml`, `crates/roko-eval/src/lib.rs`

Create the crate. Declare dependencies: `roko-core`, `async-trait`, `serde`,
`serde_json`, `thiserror`, `uuid`, `chrono`. Define the core enums and structs:

- `EvidenceKind` enum (ProcessOutput, ProcessStatus, Diff, SemanticDiff, Ast,
  RuntimeTrace, Dom, ComputedStyles, Screenshot, LayoutMetrics, ConsoleLog,
  PerformanceTrace, HttpResponse, StaticAnalysis, DesignTokens, RegressionDiff,
  SaliencyMap, Custom)
- `ArtifactRef` struct (id, path, url, artifact_type, content_hash, metadata)
- `EvidenceBag` struct wrapping `Vec<EvidenceItem>` with typed accessors
  (`get_one(kind)`, `get_all(kind)`, `has(kind)`, `insert()`)
- `EvidenceItem` struct (kind, data as `serde_json::Value`, source collector name,
  collected_at timestamp, content_hash)
- `Severity` enum (Critical, Hard, Soft, Info)
- `CriterionKind` enum (Deterministic, Computed, Heuristic, JudgePanel, Script)
- `Finding` struct (criterion, severity, summary, detail, source_location,
  ast_path, rule_id, source_tool, fix_hint, bbox, confidence)
- `CriterionResult` struct (criterion name, kind, score, passed, findings,
  metadata as `Option<serde_json::Value>`, duration_ms, cost_usd)
- `EvalVerdict` struct (passed, score, hard_failures, soft_scores, findings,
  criteria_passed, criteria_total)
- `EvalError` enum (EvidenceUnavailable, Evaluation, CollectorFailed, Timeout,
  Configuration, Internal)

Register in workspace `Cargo.toml`.

**Verify**: `cargo build -p roko-eval`, `cargo test -p roko-eval`

---

### T02: Define `EvidenceCollector` trait

**File**: `crates/roko-eval/src/collector.rs`

```rust
#[async_trait]
pub trait EvidenceCollector: Send + Sync {
    fn name(&self) -> &str;
    fn produces(&self) -> &[EvidenceKind];
    fn requires(&self) -> CollectorRequirements;
    async fn collect(
        &self,
        artifact: &ArtifactRef,
        ctx: &Context,
    ) -> Result<Vec<EvidenceItem>, EvalError>;
}

pub struct CollectorRequirements {
    pub needs_browser: bool,
    pub needs_network: bool,
    pub needs_filesystem: bool,
    pub timeout_ms: u64,
}
```

Implement `ProcessCollector` (spawns a shell command, captures stdout/stderr/exit code
into ProcessOutput + ProcessStatus evidence items) with factory methods:
`for_compile(build_system)`, `for_lint(build_system)`, `for_test(build_system)`,
`for_format(build_system)`. This collector replaces the subprocess spawning that is
currently baked into each gate.

Add `DiffCollector` (runs `git diff`, produces Diff evidence). Add
`CompositeCollector` that wraps `Vec<Box<dyn EvidenceCollector>>` and merges results.

**Verify**: Unit tests for ProcessCollector with `echo` and `false` commands.

---

### T03: Define `Criterion` trait

**File**: `crates/roko-eval/src/criterion.rs`

```rust
#[async_trait]
pub trait Criterion: Send + Sync {
    fn name(&self) -> &str;
    fn criterion_kind(&self) -> CriterionKind;
    fn is_hard(&self) -> bool;
    fn required_evidence(&self) -> &[EvidenceKind];
    fn optional_evidence(&self) -> &[EvidenceKind] { &[] }
    fn default_threshold(&self) -> f64 { 0.5 }
    async fn evaluate(
        &self,
        artifact: &ArtifactRef,
        evidence: &EvidenceBag,
        ctx: &Context,
    ) -> Result<CriterionResult, EvalError>;
}
```

Implement evidence availability checking: `fn check_evidence(criterion, bag) ->
Result<(), EvalError>` that short-circuits if required evidence is missing.

**Verify**: Unit test with a mock criterion and a pre-populated EvidenceBag.

---

### T04: Define `Profile` and `EvalService`

**File**: `crates/roko-eval/src/profile.rs`, `crates/roko-eval/src/service.rs`

`Profile` composes criteria into a named evaluation strategy:

```rust
pub struct Profile {
    pub id: String,
    pub name: String,
    pub tags: Vec<String>,
    pub strategy: EvalStrategy,
    pub criteria: Vec<CriterionRef>,
}

pub enum EvalStrategy {
    Sequential,
    ConjunctiveHardParetoSoft,
    WeightedSum { weights: Vec<f64> },
}

pub struct CriterionRef {
    pub name: String,
    pub hard: Option<bool>,
    pub threshold: Option<f64>,
    pub params: serde_json::Value,
}
```

`EvalService` orchestrates evaluation:

1. Resolve profile to concrete criteria.
2. Collect evidence via collectors (only what criteria need).
3. Run criteria in order (short-circuit on hard failure for Sequential strategy).
4. Aggregate into `EvalVerdict`.
5. Emit `EvalTrace`.

```rust
pub struct EvalService {
    pub collectors: Vec<Box<dyn EvidenceCollector>>,
    pub criteria: Vec<Box<dyn Criterion>>,
}

impl EvalService {
    pub async fn evaluate(
        &self,
        artifact: &ArtifactRef,
        profile: &Profile,
        ctx: &Context,
    ) -> Result<EvalTrace, EvalError>;
}
```

**Verify**: Integration test: ProcessCollector -> CompileCriterion -> EvalService.

---

### T05: Define `EvalTrace` and JSONL storage

**File**: `crates/roko-eval/src/trace.rs`, `crates/roko-eval/src/trace_store.rs`

`EvalTrace` is the durable record of every evaluation run:

```rust
pub struct EvalTrace {
    pub id: String,
    pub timestamp: String,
    pub artifact: ArtifactRef,
    pub profile_id: String,
    pub evidence_phase: Vec<CollectorPhaseRecord>,
    pub criterion_results: Vec<CriterionResult>,
    pub verdict: EvalVerdict,
    pub pipeline_context: PipelineContext,
    pub cost: EvalCost,
    pub duration_ms: u64,
}

pub struct PipelineContext {
    pub model: String,
    pub backend: String,
    pub prompt_variant: Option<String>,
    pub agent_role: String,
    pub generation_cost_usd: f64,
    pub generation_tokens: u64,
}

pub struct EvalCost {
    pub total_usd: f64,
    pub evidence_usd: f64,
    pub criteria_usd: f64,
    pub judge_usd: f64,
}
```

`TraceStore` appends to `.roko/eval/traces.jsonl`. Crash-tolerant: tolerates
malformed trailing lines. Read methods: `recent(limit)`, `by_id(id)`,
`by_task(task_id)`.

**Verify**: Write/read round-trip test.

---

### T06: Bridge adapter -- `BridgeGateService`

**File**: `crates/roko-gate/src/bridge.rs`

The migration bridge. `BridgeGateService` wraps the existing `GateService` and
intercepts gate names that have been migrated to the new system:

```rust
pub struct BridgeGateService {
    legacy: GateService,
    eval_service: Option<Arc<EvalService>>,
    migrated: HashSet<String>,
}

impl BridgeGateService {
    pub fn new(legacy: GateService) -> Self;
    pub fn with_eval_service(self, svc: Arc<EvalService>) -> Self;
    pub fn migrate_gate(mut self, name: &str) -> Self;
}
```

For migrated gates, run through `EvalService` and project the `EvalTrace` back
to `GateVerdict` for backward compatibility:

```rust
impl From<&EvalTrace> for Vec<GateVerdict> { ... }
```

For non-migrated gates, delegate to the inner `GateService` unchanged.

Add to `roko-gate/src/lib.rs` exports: `pub mod bridge;`

**Verify**: Test that BridgeGateService with no migrations behaves identically
to GateService. Test that a migrated gate produces the same pass/fail as the
legacy gate.

---

## Phase 2 -- Code Criteria (`roko-eval-metrics`)

Migrate existing gates to the new criterion model. Each criterion consumes
evidence from collectors rather than spawning its own subprocess.

### T07: Scaffold `roko-eval-metrics` crate

**File**: `crates/roko-eval-metrics/Cargo.toml`, `crates/roko-eval-metrics/src/lib.rs`

Dependencies: `roko-eval`, `roko-core`, `roko-gate` (for parse functions), `async-trait`,
`serde`, `serde_json`, `tree-sitter`, `tree-sitter-rust`. Re-export all criteria.
Register in workspace Cargo.toml.

**Verify**: `cargo build -p roko-eval-metrics`

---

### T08: `CompileCriterion`

**File**: `crates/roko-eval-metrics/src/compile.rs`

Migrates `CompileGate` from `crates/roko-gate/src/compile.rs`. Consumes
`EvidenceKind::ProcessOutput` + `EvidenceKind::ProcessStatus` from
`ProcessCollector::for_compile()`. Reuses `roko_gate::parse_cargo_json` and
`roko_gate::parse_plain_stderr` for structured error extraction. Binary scoring:
0.0 or 1.0. Hard severity. Threshold 1.0. Emits up to `max_error_findings`
Finding items with source_location, rule_id, fix_hint.

**Verify**: Unit test with mock evidence bag containing a failing cargo build.

---

### T09: `LintCriterion`

**File**: `crates/roko-eval-metrics/src/lint.rs`

Migrates `ClippyGate`. Two modes: strict (binary pass/fail matching exit code) and
graduated (weighted score: errors * 0.2 + warnings * 0.05, subtracted from 1.0).
Parses clippy/eslint/golangci-lint diagnostic output into `LintDiagnostic` structs.
Emits Findings with rule_id and suggestion.

**Verify**: Unit tests for both strict and graduated modes.

---

### T10: `TestCriterion`

**File**: `crates/roko-eval-metrics/src/test.rs`

Migrates `TestGate`. Reuses `roko_gate::parse_test_counts`. Score = pass_rate.
Extracts failing test names and their stdout blocks as Findings. Supports
`TestSelector` (All, Changed, Specific).

**Verify**: Unit test with mock test output containing 2 failures out of 14.

---

### T11: `FormatCriterion`, `SecurityCriterion`, `DiffCriterion`

**File**: `crates/roko-eval-metrics/src/format.rs`, `security.rs`, `diff.rs`

Three smaller criteria following the same pattern:

- `FormatCriterion`: runs format check, extracts unformatted file paths.
- `SecurityCriterion`: runs `cargo audit` or equivalent, emits info-level
  findings when audit tool is missing.
- `DiffCriterion`: analyzes git diff stats, optionally consumes SemanticDiff
  evidence for richer analysis.

**Verify**: Unit tests for each.

---

### T12: `CriterionStats` -- per-criterion adaptive tracking

**File**: `crates/roko-eval/src/stats.rs`

Extends the existing `AdaptiveThresholds` concept from per-rung to per-criterion
granularity. Tracks: EMA pass rate, consecutive passes, CUSUM accumulators,
score history (last 50), average duration and cost. Provides `should_skip()`
based on consecutive pass streak. Persists to `.roko/eval/criterion-stats.json`.

Wire into `EvalService`: after each criterion evaluation, call
`stats.observe(passed, score, duration_ms, cost_usd)`.

**Verify**: Test that 20+ consecutive passes triggers skip suggestion.

---

## Phase 3 -- AST/Semantic/Runtime Evidence and Criteria

Novel analysis capabilities that go beyond text-level gates.

### T13: `AstCollector` -- tree-sitter AST extraction

**File**: `crates/roko-eval-metrics/src/ast_collector.rs`

Uses `tree-sitter` + `tree-sitter-rust` to parse source files and produce
`EvidenceKind::Ast` evidence. Output: `Vec<FileAst>` where `FileAst` contains
`path`, `items: Vec<AstItem>`, `complexity: Vec<FunctionComplexity>`. Each
`AstItem` has kind, name, visibility, span, children, body_text.

Factory method: `AstCollector::for_changed_files(workdir)` -- parses only files
that appear in git diff.

**Verify**: Parse a small Rust file, assert item count and kinds.

---

### T14: `StructuralCompletenessCriterion`

**File**: `crates/roko-eval-metrics/src/structural_completeness.rs`

AST-based replacement for `SymbolGate`. Takes a list of `StructuralExpectation`
(kind, name pattern, path, visibility, impl_trait, substantive_body). Matches
expectations against flattened AST items. Score = met/total. Hard severity.
Handles nested items, generics, macro-generated items.

Detects `todo!()` and `unimplemented!()` bodies when `substantive_body = true`.

**Verify**: Test against a file with 3 expected functions, 1 missing.

---

### T15: `ComplexityCriterion`

**File**: `crates/roko-eval-metrics/src/complexity.rs`

Checks cyclomatic complexity, cognitive complexity, and body line count per
function from AST evidence. Default thresholds: cyclomatic 15, cognitive 20,
body lines 100. Score = fraction of functions within thresholds. Soft severity.
Threshold 0.9.

**Verify**: Test with a function containing 5 nested if-else branches.

---

### T16: `SemanticDiffCollector` and `SubstanceCriterion`

**File**: `crates/roko-eval-metrics/src/semantic_diff.rs`, `substance.rs`

`SemanticDiffCollector`: Compares before/after ASTs to classify changes at the
structural level. Each `SemanticChange` has a `kind` (FunctionAdded,
FunctionModified, TypeChanged, FormattingOnly, DocumentationChanged,
ImportChanged, etc.) and a `significance` score (0.0-1.0).

`SubstanceCriterion`: Scores the substantive content of changes. Catches
"did nothing" failure mode with higher precision than DiffGate's forbidden
token matching. Average significance below threshold = fail.

**Verify**: Test that a diff adding only comments scores near 0.0.

---

### T17: `RuntimeTraceCollector` and `CoverageCriterion`

**File**: `crates/roko-eval-metrics/src/runtime_trace.rs`, `coverage.rs`

`RuntimeTraceCollector`: Runs tests with coverage instrumentation (cargo-tarpaulin
for Rust, c8/istanbul for JS). Parses coverage output into `RuntimeTraceData`
with `CoverageData` (line_coverage, branch_coverage, files with per-file stats).

`CoverageCriterion`: Checks minimum line coverage threshold (default 0.7).
Optional `diff_only` mode: only consider coverage for changed files. Reports
files with lowest coverage as Findings.

**Verify**: Test with mock coverage JSON output.

---

## Phase 4 -- LLM Judge Panel (`roko-eval-judge`)

Bradley-Terry pairwise comparison, disjoint-family panels, position bias
mitigation, and the full judge methodology.

### T18: Scaffold `roko-eval-judge` crate

**File**: `crates/roko-eval-judge/Cargo.toml`, `crates/roko-eval-judge/src/lib.rs`

Dependencies: `roko-eval`, `roko-core`, `roko-gate` (for JudgeOracle trait),
`async-trait`, `serde`, `serde_json`, `rand`, `nalgebra` (for BT MLE).

**Verify**: `cargo build -p roko-eval-judge`

---

### T19: Bradley-Terry MLE with Davidson ties

**File**: `crates/roko-eval-judge/src/bt_model.rs`

Implement BT MLE via logistic regression with high regularization (C=10^6).
Davidson tie parameter (nu). BCa bootstrap confidence intervals (B=1000
resamples). Elo scale mapping: `Elo_i = theta_i * 400 / ln(10)`.

Structs: `BtResult` (elo_scores BTreeMap, confidence_intervals, tie_parameter,
comparison_count, log_likelihood), `BtConfidenceInterval` (lower, point, upper,
n_bootstrap, confidence).

Input: `Vec<ComparisonTriple>` where each triple is (candidate_a, candidate_b,
outcome: APreferred|BPreferred|Tie).

**Verify**: Known-answer test: feed 10 comparisons where A always beats B,
verify Elo_A > Elo_B by >200 points.

---

### T20: Judge panel construction with family exclusion

**File**: `crates/roko-eval-judge/src/panel.rs`

`JudgePanelConfig`: min_panel_size (default 3), preferred_panel_size (default 3),
exclude_generator_family (mandatory).

`construct_panel(providers, generator_family, config)`: collect available model
families, exclude generator, select strongest vision-capable model per family,
sort by priority, take top N. Error if fewer than min_panel_size available.

`JudgeSpec`: model_id, family, endpoint, max_tokens, temperature (default 0).

Wire into `roko-learn` CascadeRouter: extract generator family from
`AgentEfficiencyEvent.model` via `providers.family_for_model(slug)`.

**Verify**: Test with 4 providers, exclude one family, assert panel = 3.

---

### T21: Position swap-and-discard

**File**: `crates/roko-eval-judge/src/position_swap.rs`

For every pairwise comparison by every judge:
1. Present (A=candidate, B=anchor), record V_AB.
2. Present (A=anchor, B=candidate), record V_BA.
3. Consistency check: same underlying artifact preferred regardless of position?
   If flipped, discard this judge's result.

`PositionSwapResult`: judge, verdict_ab, verdict_ba, consistent, effective_verdict.

Intra-Pair Instability (IPI) metric:
`IPI(pair) = |P(A>B | order=AB) - P(A>B | order=BA)|`

**Verify**: Test consistent case (PreferA, PreferB) -> consistent=true.
Test inconsistent case (PreferA, PreferA) -> consistent=false, discarded.

---

### T22: Anchor store and rotation

**File**: `crates/roko-eval-judge/src/anchor.rs`

`JudgeAnchor`: content_hash, established_at_ms, provenance (HumanApproved,
GatePassed, ArenaWinner, Bootstrapped), artifact, evidence_path, elo,
comparison_count.

`AnchorStore`: persists to `.roko/eval/anchors.json`. Methods: `get(task_id,
viewport)`, `set(anchor)`, `rotate_if_due(config)`.

`AnchorRotationConfig`: max_anchor_age_days (30), rotation_win_rate (0.8),
rotation_min_evals (20), bootstrap_rotation_evals (10).

Bootstrapping protocol: when no prior anchor exists, use absolute rubric scoring
with N=3 per judge. If passes, candidate becomes anchor with provenance
Bootstrapped. After 10 evals, auto-promote best candidate.

**Verify**: Test bootstrap -> rotate lifecycle.

---

### T23: Aggregation and disagreement detection

**File**: `crates/roko-eval-judge/src/aggregation.rs`,
`crates/roko-eval-judge/src/disagreement.rs`

Trimmed mean aggregation (10-20% trim). For 3-judge panel, no trimming (too few).
For 5-judge panel with per-run scores, trim lowest and highest.

`LearnedJudgeWeights`: BTreeMap<family, weight>, fit_at_ms, n_canary, r_squared.
Active when >= 500 human-rated canary examples. Ridge regression fit.

`PanelDisagreement`: agreement_rate, score_spread, krippendorff_alpha,
needs_human_review, reason. Flags for review when agreement < 0.5,
spread > 0.3, or alpha < 0.4.

**Verify**: Test trimmed mean with 5 scores. Test disagreement detection.

---

### T24: Judge prompt templates and rubrics

**File**: `crates/roko-eval-judge/src/prompts/mod.rs`, `code.rs`, `visual.rs`,
`crates/roko-eval-judge/src/rubric.rs`

Code evaluation 7-dimension rubric: correctness (0.30), maintainability (0.20),
safety (0.15), performance (0.10), test_coverage (0.10), api_design (0.10),
documentation (0.05). Findings grounded with file:line:col.

Visual evaluation 7-dimension rubric: task_completion (0.25),
layout_integrity (0.20), responsive_quality (0.15), interaction_clarity (0.10),
visual_polish (0.10), design_system_fit (0.10), accessibility_affordance (0.10).
Findings grounded with bounding boxes.

All prompts use analyze-before-rate format. JSON-only output.
Configurable weight customization per profile.

**Verify**: Test prompt rendering with sample artifacts.

---

### T25: `JudgePanelCriterion` and `PanelJudgeOracle` gate adapter

**File**: `crates/roko-eval-judge/src/criterion.rs`,
`crates/roko-eval-judge/src/gate_adapter.rs`

`JudgePanelCriterion` implements `Criterion` trait. Full evaluation flow:
1. Construct panel excluding generator family.
2. For each judge: run pairwise comparison with position swap (6 calls total
   per judge: 3 runs x 2 positions).
3. Discard inconsistent results.
4. Aggregate via trimmed mean.
5. Feed to BT model for Elo scoring against anchor.
6. Return CriterionResult with per-dimension findings.

`PanelJudgeOracle` implements `roko_gate::JudgeOracle` for backward compatibility
with the existing gate pipeline. Wraps the panel flow and returns a normalized
f32 score.

Wire into adaptive thresholds: report aggregate score as rung-6 observation.

**Verify**: Integration test with mock judge oracles.

---

### T26: Sampling strategy and adaptive budget

**File**: `crates/roko-eval-judge/src/sampling.rs`

N=3 at T=0 per position (base). Adaptive increase when panel disagrees:
agreement >= 0.8 -> base_samples; >= 0.5 -> 2x; < 0.5 -> max_samples.

**Verify**: Unit test for budget calculation.

---

## Phase 5 -- Self-Improvement Flywheel

Every evaluation produces compounding value: traces, preferences, patterns,
curricula, and experiment data feed back into the system.

### T27: Preference mining and PreferenceTriple

**File**: `crates/roko-eval/src/preference.rs`

`PreferenceTriple`: id, prompt, candidate_a, candidate_b, preferred
(A|B|Tie), source (UserEdit, UserSelection, JudgePanel, ExternalBenchmark,
RegressionComparison), trace_id_a, trace_id_b, task_id, timestamp,
criterion_deltas, confidence. Append-only JSONL at
`.roko/learn/preferences.jsonl`.

Mining from judge panel: every pairwise comparison produces a triple.
Mining from user edits: pre-edit = negative, post-edit = positive (confidence 1.0).

**Verify**: Write/read round-trip test.

---

### T28: Auto-grade bridge to existing FeedbackService

**File**: `crates/roko-eval/src/feedback_bridge.rs`

Convert EvalTrace verdicts into `KnowledgeOutcome` records for the existing
`FeedbackService` at `crates/roko-learn/src/feedback_service.rs`. Record outcome
for each knowledge entry and prompt section used during generation.

Wire into ExperimentStore: when trace carries a `prompt_variant`, report
pass/fail to `VariantStats` for UCB1 convergence.

**Verify**: Test that a passing trace records Success, failing records Failure.

---

### T29: Pattern library and neuro store promotion

**File**: `crates/roko-eval/src/pattern_library.rs`,
`crates/roko-eval/src/neuro_bridge.rs`

`PatternEntry`: id, name, category, fingerprint, polarity (Positive|Negative),
support_count, avg_score, template, anti_pattern_description, tags, updated_at.

Nightly batch: cluster successful evaluation traces by AST/DOM shape. Diff
passing vs. failing pairs sharing the same artifact to extract positive and
negative patterns.

`neuro_bridge`: promote patterns into `roko-neuro` knowledge store as engrams
with `Kind::Pattern`. HDC fingerprint for approximate nearest-neighbor queries
at dispatch time. Tier: Transient -> Episodic -> Semantic via natural promotion.

**Verify**: Test pattern creation and neuro engram promotion.

---

### T30: Curriculum from failures (WebRL pattern)

**File**: `crates/roko-eval/src/curriculum.rs`

Cluster failed evaluation traces by judge rationale text. For each cluster with
3+ members, generate synthetic task prompt exercising the failure mode with
edge-case variants.

`CurriculumTask`: id, source_cluster_id, cluster_size, prompt,
acceptance_criteria, eval_profile, variants, priority, status
(Pending|InProgress|Promoted|Retained|Superseded).

Promoted tasks (succeeded) join the regression eval set. Integrate with existing
`roko-learn/src/curriculum.rs`.

Wire post-gate reflections from `roko-learn/src/post_gate_reflection.rs` into
cluster enrichment.

**Verify**: Test cluster creation from 5 failing traces with similar rationales.

---

### T31: Pipeline arm bandits (CascadeRouter extension)

**File**: `crates/roko-eval/src/pipeline_arm.rs`

`PipelineArm`: model, retrieval_k, with_clarifying_turn, with_post_fixer.
Feature vector for LinUCB contextual bandit. Register as additional context
dimensions in the existing `CascadeRouter` at
`crates/roko-learn/src/cascade_router.rs`.

Flywheel event types: `FlywheelEvent` with `FlywheelEventType` enum
(TraceEmitted, AutoGradeCompleted, PreferenceMined, PatternExtracted,
CurriculumGenerated, ExperimentCreated, CanaryEvaluated, AnchorRotated,
DriftDetected). Emit through existing runtime event bus.

**Verify**: Test feature vector generation and CascadeRouter integration.

---

### T32: Anti-Goodhart safeguards

**File**: `crates/roko-eval/src/canary.rs`, `crates/roko-eval/src/anti_goodhart.rs`

Canary set management: 200-500 frozen human-rated prompts with Krippendorff
alpha >= 0.8. Re-evaluated every release. Track Spearman rho between inner-loop
judge and canary evaluation. rho < 0.6 = drift detected.

Rubric rotation: quarterly schedule for rubric emphasis changes per LiveBench.
Tracked as experiments in ExperimentStore.

Integrate with `roko-learn/src/drift.rs` for canary correlation as drift signal.

**Verify**: Test Spearman rho calculation with known ranked lists.

---

## Phase 6 -- Community Marketplace (`roko-eval-community`)

Publish, discover, fork, and compose evaluation artifacts.

### T33: Scaffold `roko-eval-community` crate

**File**: `crates/roko-eval-community/Cargo.toml`,
`crates/roko-eval-community/src/lib.rs`

Dependencies: `roko-eval`, `roko-core`, `serde`, `serde_json`, `reqwest`,
`blake3`, `ed25519-dalek`, `zip`, `semver`, `toml`.

**Verify**: `cargo build -p roko-eval-community`

---

### T34: Package format and manifest

**File**: `crates/roko-eval-community/src/package.rs`,
`crates/roko-eval-community/src/namespace.rs`

`.rokoeval` ZIP archive format: manifest.toml (name, namespace, version, type,
roko_version_min, signature, dependencies, checksums), artifact definition
(criterion.toml or profile.toml), README.md, LICENSE, optional scripts, sample/.

Namespace model: `roko/` (official), `@username/` (user), `@orgname/` (org).
Version specifiers: exact (`=1.2.3`), compatible (`^1.2`), patch (`~1.2`),
range (`>=1.0, <2.0`). Parse and resolve with `semver` crate.

Bundle creation: `create_bundle(dir) -> PathBuf` computes BLAKE3 checksums
and packages into ZIP. Bundle extraction: `extract_bundle(path) -> TempDir`.

**Verify**: Create bundle, verify checksums, extract and compare contents.

---

### T35: Dependency resolution and lock file

**File**: `crates/roko-eval-community/src/resolver.rs`,
`crates/roko-eval-community/src/install.rs`

Dependency types: evidence dependency, criterion reference, knowledge attachment.
Resolution: expand transitive deps, unify version ranges (intersect), resolve
to highest matching stable, verify roko_version_min. Produce `eval.lock` file.

Conflict resolution: exact pins win over ranges, lock file pins win over range
resolution, local overrides win over everything.

Local installation layout: `.roko/eval/installed/<ns>/<name>/<version>/`,
`.roko/eval/local/` for forked artifacts. `eval.lock` and `overrides.toml`.

**Verify**: Test resolution with diamond dependency (A depends on B and C,
both depend on D at compatible versions).

---

### T36: Ed25519 signing and trust system

**File**: `crates/roko-eval-community/src/signature.rs`,
`crates/roko-eval-community/src/trust.rs`

Sign bundles with ed25519. Verify signatures on install. Key generation and
storage in `.roko/keys/`.

Trust signals: `VerifiedRunReport` (anonymous team_hash, artifact_ref, run_count,
pass_rate). Reputation score weighted by install count (0.20), active runs (0.25),
fork count (0.15), canary correlation (0.25), comment sentiment (0.10), yank
rate (0.05). Trust badges: Verified (100+ runs from 5+ teams), Battle-tested
(1000+ runs, >90% pass), Calibrated (rho >= 0.7), Official.

**Verify**: Sign and verify round-trip test.

---

### T37: Fork lineage and attribution

**File**: `crates/roko-eval-community/src/lineage.rs`

`ForkLineage`: parent (Option<ArtifactRef>), chain (Vec<ArtifactRef>),
changes (Vec<ForkChange>). Fork change types: ParameterChanged, CriterionAdded,
CriterionRemoved, EvidenceChanged, ScoringChanged, etc.

Fork operation: create mutable copy preserving full lineage chain. Display
attribution: "Forked from @alice/criterion v1.2.0, changes: +threshold (75->95)".

**Verify**: Test fork chain through 3 generations.

---

### T38: Registry client (CLI operations)

**File**: `crates/roko-eval-community/src/registry.rs`

REST client for `https://registry.roko.dev/v1` (configurable in roko.toml).
Methods: `publish(bundle)`, `search(query, filters)`, `download(ref, version)`,
`fork(ref)`, `yank(ref, version)`. Bearer token auth for write operations,
anonymous for reads. Fallback to private registry URL from config.

CLI integration stubs for: `roko eval publish`, `roko eval search`,
`roko eval install`, `roko eval fork`, `roko eval list`, `roko eval update`,
`roko eval uninstall`.

**Verify**: Mock HTTP server test for search and download.

---

### T39: Gate and learn bridges for community artifacts

**File**: `crates/roko-eval-community/src/gate_bridge.rs`,
`crates/roko-eval-community/src/learn_bridge.rs`,
`crates/roko-eval-community/src/neuro_bridge.rs`

`gate_bridge`: Convert installed community criterion into `Verify` impl.
Route through DeterministicCriterionGate, ScriptCriterionGate, or
JudgePanelCriterionGate based on `CriterionKind`.

`learn_bridge`: installed criterion refs recorded in episodes via EpisodeLogger.
Criterion outcomes feed FeedbackService. Published rubric variants become
ExperimentStore experiments.

`neuro_bridge`: ingest knowledge bundles into neuro store as `KnowledgeEntry`
with `Kind::ExternalBundle`, tier Transient, confidence 0.6.

**Verify**: Test criterion -> Verify conversion for a deterministic criterion.

---

## Phase 7 -- Dashboard Integration

Eval results surfaced across TUI, web dashboard, and CLI.

### T40: Runtime event bus extensions

**File**: modify `crates/roko-core/src/runtime_event.rs`,
modify `crates/roko-runtime/src/lib.rs`

Add `RokoEvent` variants: `EvalStarted`, `EvalCriterionCompleted`,
`EvalCompleted`. Each carries plan_id, task_id, and criterion-specific fields.

Add `DashboardEvent` variants: `EvalResult`, `EvalCriterionResult`.

Wire: `orchestrate.rs` emits `EvalCompleted` alongside existing
`GateCompleted` events. SSE adapter streams both `eval.*` and `gate.*` events.

**Verify**: Test event emission and SSE serialization.

---

### T41: `roko eval` CLI command family

**File**: `crates/roko-cli/src/commands/eval.rs`,
modify `crates/roko-cli/src/commands/mod.rs`

Commands:
- `roko eval run <path> [--profile <name>] [--judge] [--budget <usd>] [--output text|json]`
- `roko eval list` -- list available profiles
- `roko eval show <profile>` -- profile detail with criteria
- `roko eval history [--limit N]` -- recent eval traces
- `roko eval trace <id>` -- full trace detail
- `roko eval compare <id1> <id2>` -- side-by-side comparison
- `roko eval calibrate` -- run judge calibration suite

Enhanced `plan run` output: replace "gate" prefix with "eval", add score values,
finding counts, judge panel agreement, cost in summary line. Sub-criteria
indented under parent.

**Verify**: Smoke test that `roko eval list` parses and runs.

---

### T42: `roko eval` serve routes

**File**: `crates/roko-serve/src/routes/eval.rs`,
`crates/roko-serve/src/routes/eval_artifacts.rs`,
`crates/roko-serve/src/routes/eval_judges.rs`,
modify `crates/roko-serve/src/routes/mod.rs`

Eval trace endpoints:
- `GET /api/eval/traces` -- list recent, supports limit, offset, filter by verdict
- `GET /api/eval/traces/{id}` -- full trace with evidence
- `GET /api/eval/traces/{id}/artifacts` -- list artifacts
- `GET /api/eval/traces/{id}/artifacts/{n}` -- download artifact (screenshot, etc.)
- `GET /api/eval/summary` -- aggregate stats
- `GET /api/eval/criteria` -- list registered criteria
- `GET /api/eval/profiles` -- list profiles
- `POST /api/eval/run` -- trigger ad-hoc evaluation

Judge endpoints:
- `GET /api/eval/judges` -- configured judge models
- `GET /api/eval/judges/calibration` -- panel calibration metrics
- `POST /api/eval/judges/compare` -- run pairwise comparison

Artifact serving with proper MIME types (image/png for screenshots,
text/plain for logs). Existing `/api/gates/*` endpoints remain unchanged.

**Verify**: API integration test for trace listing and artifact serving.

---

### T43: TUI eval trace widget and evidence browser

**File**: `crates/roko-cli/src/tui/widgets/eval_trace.rs`,
`crates/roko-cli/src/tui/widgets/criterion_bar.rs`,
`crates/roko-cli/src/tui/widgets/judge_panel.rs`,
`crates/roko-cli/src/tui/modals/evidence_browser.rs`

`EvalTraceWidget`: compact table rendering an EvalTrace -- criterion name,
pass/fail, duration, evidence count, score bar, finding count. Visual evidence
shows filepath + link to web dashboard artifact viewer.

`CriterionBarWidget`: horizontal bar for 0.0-1.0 scores with color coding
(red < 0.5, yellow 0.5-0.8, green > 0.8).

`JudgePanelWidget`: panel agreement indicator showing per-judge verdicts.

`EvidenceBrowserModal`: accessible via Enter on any gate row. Lists ArtifactRef
entries. Text artifacts rendered inline with syntax highlighting. Image artifacts
show file path + open in system viewer on Enter. Navigation: j/k scroll,
Enter open, q close.

Modify `verdicts.rs` to accept `EvalTrace` alongside `GateVerdict`.
Modify `pages/operations.rs` for visual gate rows and judge rows.

**Verify**: Widget render test for correct line count.

---

### T44: Web dashboard -- Arena pages (React)

**File**: `demo/demo-app/src/pages/ArenaOverview.tsx`,
`demo/demo-app/src/pages/EvalsLibrary.tsx`,
`demo/demo-app/src/pages/EvalRunner.tsx`,
`demo/demo-app/src/pages/EvalHistory.tsx`

ArenaOverview: four glass-1 metric cards (total evals, pass rate, mean duration,
mean cost) + recent eval timeline (left 60%) + gate waterfall heatmap (right 40%).
Live SSE updates.

EvalsLibrary: two-column master-detail. Left: criteria/profiles browser with
search and category filter. Right: detail view with description, evidence
requirements, threshold config.

EvalRunner: split view. Left: profile selector, target path, options. Right:
live results via SSE -- criteria completing one by one, screenshots inline,
final verdict card.

EvalHistory: table with expandable rows. Columns: timestamp, task, profile,
verdict, duration, cost, criteria_passed/total. Filters: date range, verdict,
profile.

ROSEDUST design system: void-black backgrounds, rose accent family, jade/amber/
crimson semantic colors, glass morphism (blur 8/12/16px), spring physics via
Framer Motion.

Zustand store: `evalStore.ts` with traces, activeTrace, criteria, profiles,
summary, SSE subscription.

**Verify**: Component renders without errors (Vitest or manual).

---

### T45: `roko status` eval enhancement

**File**: modify `crates/roko-cli/src/commands/util.rs` (or wherever status is)

When eval traces exist, add a summary section:

```
Evaluation Summary (last 24h):
  Runs: 47 (43 pass, 3 fail, 1 error)
  Pass rate: 91.5% (up from 87.2% prior 24h)
  Mean duration: 8.4s
  Mean cost: $0.038
  Top failing criteria: responsive_quality (3 failures)
  Judge panel agreement: 94.2%
```

Read from `TraceStore::recent(limit=1000)` and aggregate.

**Verify**: Test output format with mock trace data.

---

## Phase 8 -- Integration and Migration

Wire everything together through the orchestrator and run the end-to-end path.

### T46: Wire EvalService into orchestrate.rs

**File**: modify `crates/roko-cli/src/orchestrate.rs`

After agent dispatch produces output, construct `ArtifactRef` from task workdir.
Select evaluation profile from task config or workspace default. Run
`EvalService::evaluate()`. Project `EvalTrace` to `GateVerdict` for existing
consumers. Emit both `RokoEvent::EvalCompleted` and `RokoEvent::GateCompleted`.

Feed eval outcomes to:
- `EpisodeLogger` (gate_verdicts + eval_trace_id)
- `FeedbackService` (via feedback_bridge)
- `ExperimentStore` (via prompt_variant)
- `CascadeRouter` (via pipeline arm observation)
- `AdaptiveThresholds` (via per-criterion stats)
- `PreferenceTriple` logger

Profile selection logic: check task.eval_profile, then plan.eval_profile, then
roko.toml `[eval.default_profile]`, then built-in "rust-strict" for Rust projects.

**Verify**: End-to-end: `roko plan run` with a simple task produces an EvalTrace
in `.roko/eval/traces.jsonl` and emits events to the runtime bus.

---

### T47: Built-in profiles (TOML)

**File**: `crates/roko-eval/src/builtin_profiles.rs`,
`.roko/eval/profiles/rust-strict.toml`,
`.roko/eval/profiles/code-review.toml`

`rust-strict`: compile -> lint -> test -> format -> diff -> substance (soft).
Sequential strategy. All deterministic, no judge panel.

`code-review`: compile -> lint -> test -> format -> diff -> substance ->
complexity -> judge_panel. Sequential + judge panel at end.

`fullstack-web`: code gates first (compile, lint, test), then AST analysis
(structural_completeness, complexity), then visual gates (requires browser),
then judge panel last. Sequential.

Profile loader: read `.roko/eval/profiles/*.toml` and merge with built-in
profiles. Community-installed profiles from `.roko/eval/installed/`.

**Verify**: Test profile loading and criterion resolution.

---

### T48: Criterion authoring format (user TOML criteria)

**File**: `crates/roko-eval/src/custom_criterion.rs`

Users author custom criteria in TOML at `.roko/criteria/*.toml`. Two modes:

1. Shell command (deterministic): runs a command, exit 0 = pass. Evidence
   available via environment variables (`$EVAL_ARTIFACT_PATH`, etc.).

2. LLM judge (stochastic): delegates to a judge panel with custom rubric.
   Model selection, position swap, panel size configurable.

Parser: read TOML into `CustomCriterionDef`, construct either
`ShellCriterion` or `JudgePanelCriterion` at runtime.

```toml
[criterion]
name = "no_unwrap"
kind = "deterministic"
severity = "hard"

[criterion.evidence]
required = ["diff"]

[criterion.check]
type = "shell"
command = "grep -rn '.unwrap()' ${EVAL_ARTIFACT_PATH}/src/ && exit 1 || exit 0"
```

**Verify**: Parse and execute a custom shell criterion.

---

## Dependency Graph

```
Phase 1 (T01-T06): Core abstractions
  |
  +-- Phase 2 (T07-T12): Code criteria       \
  |                                            +-- Phase 5 (T27-T32): Flywheel
  +-- Phase 3 (T13-T17): AST/semantic/runtime /
  |
  +-- Phase 4 (T18-T26): Judge panel
  |
  +-- Phase 6 (T33-T39): Community marketplace
  |
  Phase 7 (T40-T45): Dashboard integration
  |
  Phase 8 (T46-T48): Integration + migration
```

Phases 2, 3, and 4 can run in parallel after Phase 1 completes. Phase 5 depends
on Phases 2 and 3. Phase 6 depends on Phase 1 only. Phase 7 depends on Phases 1
and 4. Phase 8 depends on all prior phases.

---

## Migration Strategy

The migration from `roko-gate` to `roko-eval` is incremental:

1. **Phase 1**: `BridgeGateService` wraps `GateService` with zero migrated gates.
   Everything works exactly as before.

2. **Phases 2-3**: Individual gates are migrated one at a time. Each migration
   adds the gate name to `BridgeGateService.migrated`. The bridge routes migrated
   gates through `EvalService` and projects results back to `GateVerdict`.
   Non-migrated gates continue through the legacy path.

3. **Phase 4**: Judge panel replaces `StubJudgeGate` at rung 6.

4. **Phase 8**: orchestrate.rs switches from `GateService` to
   `BridgeGateService`. At this point, gates can be migrated one-by-one by
   adding them to the migrated set. The old and new systems coexist.

5. **Future**: when all gates are migrated, `GateService` becomes a thin
   backward-compatibility shim and `BridgeGateService` becomes the primary.

---

## Key Integration Points

| Existing Crate | File | Integration |
|---|---|---|
| `roko-gate` | `gate_service.rs` | BridgeGateService wraps GateService |
| `roko-gate` | `adaptive_threshold.rs` | CriterionStats extends per-rung to per-criterion |
| `roko-gate` | `llm_judge_gate.rs` | PanelJudgeOracle implements JudgeOracle |
| `roko-gate` | `compile_errors.rs` | CompileCriterion reuses parse functions |
| `roko-learn` | `cascade_router.rs` | Pipeline arm features extend routing context |
| `roko-learn` | `prompt_experiment.rs` | Auto-grade closes experiment loop |
| `roko-learn` | `feedback_service.rs` | Eval verdicts -> KnowledgeOutcome |
| `roko-learn` | `episode_logger.rs` | Traces cross-referenced by task_id |
| `roko-learn` | `playbook.rs` | Curriculum successes -> new playbooks |
| `roko-learn` | `post_gate_reflection.rs` | Reflections enrich curriculum clusters |
| `roko-learn` | `drift.rs` | Canary correlation as drift signal |
| `roko-neuro` | `knowledge_store.rs` | Patterns stored as engrams |
| `roko-core` | `runtime_event.rs` | Eval events on the event bus |
| `roko-cli` | `orchestrate.rs` | EvalService called per-task |
| `roko-serve` | `routes/mod.rs` | New eval API routes |
| `roko-cli` | `tui/verdicts.rs` | Accepts EvalTrace alongside GateVerdict |

---

## Risk Mitigations

1. **tree-sitter compilation**: tree-sitter C bindings can be slow to compile.
   Mitigate by making `roko-eval-metrics` optional in the default workspace build.
   Feature-gate tree-sitter behind `ast` feature flag.

2. **Judge panel cost**: at $0.78/evaluation with 3 judges, costs compound
   quickly. Mitigate by making judge panels opt-in per profile and only running
   them for visual/high-stakes evaluations.

3. **Bradley-Terry numerical stability**: MLE can diverge with sparse data.
   Mitigate with high regularization (C=10^6) and minimum comparison count
   check before fitting.

4. **Backward compatibility**: the BridgeGateService ensures zero-regression
   during migration. Every EvalTrace projects cleanly to GateVerdict. Existing
   `/api/gates/*` endpoints are never modified.

5. **Community registry availability**: offline and air-gapped deployments
   must work without the registry. All marketplace features degrade gracefully
   to local-only operation.
