# PRD-01 — Core Abstractions: Traits, Types, and Contracts

**Status**: Draft
**Author**: Will (architect) + Claude (synthesis)
**Date**: 2026-04-25 (revised 2026-04-29)
**Crate**: `roko-eval` (new, Layer 1)
**Prerequisites**: PRD-00 (System Overview)
**Implementation path**: `crates/roko-eval/src/`

---

## 0. Scope

This document defines the kernel of the unified evaluation framework: the trait
definitions, type system, and contracts that every other PRD builds on. Everything
lives in `crates/roko-eval/`. No external dependencies beyond `roko-core` (for
`Engram`, `Score`, `Verdict`, `Context`, `Verify`, `ContentHash`).

The three primitives:

1. **`EvidenceCollector`** -- produces structured evidence from any artifact
2. **`Criterion`** -- scores one evaluative dimension given evidence
3. **`Profile`** -- composes criteria into a named, shareable evaluation strategy

Plus the supporting types: `EvidenceBag`, `ArtifactRef`, `CriterionResult`,
`Finding`, `EvalVerdict`, `EvalTrace`, registry references, TOML authoring
format, bridge adapters to the existing `Verify` trait, and the `EvalService`
runtime that orchestrates evaluation.

---

## 1. The EvidenceCollector Trait

### 1.1 Design Motivation

Evidence collection is the bottleneck that determines what criteria can run. A
criterion that needs DOM snapshots cannot function if the only evidence available
is a process exit code. By making evidence collection an explicit, typed phase,
we achieve three properties:

1. **Declarative evidence requirements** -- criteria declare what they need;
   the runtime can short-circuit if evidence is unavailable.
2. **Evidence sharing** -- multiple criteria reuse the same DOM snapshot or
   screenshot without redundant collection.
3. **Infrastructure failure isolation** -- a browser crash is an evidence
   collection failure, not a criterion failure. The distinction matters for
   scoring (Section 4).

This separation addresses a fundamental limitation of the current `Verify` trait:
`CompileGate::verify()` fuses evidence collection (spawning `cargo check`) with
judgment (interpreting the exit code). When the process fails to spawn (missing
binary, disk full), the gate reports a low quality score rather than an
infrastructure failure. The new system distinguishes these cases.

### 1.2 EvidenceKind Enum

Every piece of evidence has a kind. Criteria declare which kinds they require.
The runtime matches available evidence to criterion requirements.

```rust
// File: crates/roko-eval/src/evidence.rs

/// The kind of evidence that can be collected from an artifact.
///
/// Evidence kinds form a closed set at the framework level. Collector
/// implementations may produce evidence of any kind; criteria declare
/// which kinds they require via [`Criterion::required_evidence`].
///
/// New kinds can be added without breaking existing criteria -- criteria
/// that don't require the new kind simply ignore it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    /// Full-page or element screenshot (PNG bytes + dimensions).
    Screenshot,
    /// Serialized DOM tree (HTML string).
    Dom,
    /// Accessibility tree snapshot (axe-core + IBM Equal Access format).
    AccessibilityTree,
    /// Browser console log entries (errors, warnings, info).
    ConsoleLog,
    /// Network request/response log (HAR-like).
    NetworkLog,
    /// Performance trace (Lighthouse, CWV, LoAF).
    PerformanceTrace,
    /// Computed styles for visible elements (getComputedStyle dumps).
    ComputedStyles,
    /// Code diff (unified diff format, before/after).
    Diff,
    /// Process stdout/stderr capture.
    ProcessOutput,
    /// Process exit code and timing.
    ProcessStatus,
    /// HTTP response (status, headers, body).
    HttpResponse,
    /// Static analysis results (AST, symbol table, lint output).
    StaticAnalysis,
    /// Design token file (W3C DTCG 2025.10 format).
    DesignTokens,
    /// Saliency heatmap (DeepGaze IIE + UMSI++ ensemble output).
    SaliencyMap,
    /// Visual regression diff image and metrics.
    RegressionDiff,
    /// Layout metrics (element positions, grid adherence, density).
    LayoutMetrics,
    /// AST (Abstract Syntax Tree) structural representation.
    /// Produced by tree-sitter parsing for language-aware analysis.
    Ast,
    /// Semantic diff: AST-level change classification.
    /// Distinguishes structural changes (new functions, changed signatures)
    /// from cosmetic changes (renames, reformatting, comment edits).
    SemanticDiff,
    /// Runtime execution trace (function calls, allocations, I/O).
    /// Produced by instrumented test execution or profiling.
    RuntimeTrace,
    /// Type-checker output (type errors, inferred types, constraint violations).
    TypeCheckOutput,
    /// Dependency graph (crate/module/import relationships).
    DependencyGraph,
    /// Arbitrary JSON payload (escape hatch for custom collectors).
    CustomJson,
    /// Arbitrary binary payload (escape hatch for custom collectors).
    CustomBytes,
}

impl EvidenceKind {
    /// Whether this evidence kind is typically expensive to collect.
    ///
    /// Used by the runtime to schedule cheap evidence first and skip
    /// expensive evidence when criteria that need it are already
    /// known to be irrelevant (e.g., all hard criteria failed).
    pub fn is_expensive(&self) -> bool {
        matches!(
            self,
            Self::Screenshot
                | Self::AccessibilityTree
                | Self::PerformanceTrace
                | Self::SaliencyMap
                | Self::RuntimeTrace
        )
    }

    /// Whether this evidence kind requires a running browser.
    pub fn requires_browser(&self) -> bool {
        matches!(
            self,
            Self::Screenshot
                | Self::Dom
                | Self::AccessibilityTree
                | Self::ConsoleLog
                | Self::NetworkLog
                | Self::PerformanceTrace
                | Self::ComputedStyles
                | Self::LayoutMetrics
                | Self::SaliencyMap
        )
    }
}
```

### 1.3 Evidence Type

Individual evidence entries carry their kind, raw data, and optional metadata.

```rust
// File: crates/roko-eval/src/evidence.rs

/// A single piece of collected evidence.
///
/// Evidence is produced by [`EvidenceCollector`] implementations and
/// stored in an [`EvidenceBag`]. Criteria consume evidence by kind,
/// extracting the data they need from the `data` field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    /// What kind of evidence this is.
    pub kind: EvidenceKind,
    /// The evidence payload.
    pub data: EvidenceData,
    /// When this evidence was collected (Unix ms).
    pub collected_at_ms: i64,
    /// How long collection took (milliseconds).
    pub collection_duration_ms: u64,
    /// Which collector produced this evidence.
    pub collector: String,
    /// Optional metadata (viewport name, journey ID, step index, etc.).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, String>,
}

/// The payload of a piece of evidence.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EvidenceData {
    /// UTF-8 text (stdout, diff, HTML).
    Text { content: String },
    /// Structured JSON (DOM, a11y tree, network log, metrics).
    Json { value: serde_json::Value },
    /// Raw bytes (screenshot PNG, binary artifacts).
    /// Serialized as base64 in JSON.
    Bytes {
        #[serde(with = "base64_serde")]
        content: Vec<u8>,
        /// MIME type hint (e.g., "image/png", "application/octet-stream").
        #[serde(default)]
        mime: Option<String>,
    },
    /// Reference to an artifact on disk (avoids embedding large blobs).
    /// The path is relative to the eval output directory.
    FileRef {
        path: PathBuf,
        /// MIME type hint.
        #[serde(default)]
        mime: Option<String>,
        /// File size in bytes (for budget tracking).
        #[serde(default)]
        size_bytes: Option<u64>,
    },
}

impl Evidence {
    /// Extract text content from this evidence entry.
    ///
    /// Returns the content for `Text` data, serialized JSON for `Json` data,
    /// and an error for binary data.
    pub fn extract_text(&self) -> Result<String, EvalError> {
        match &self.data {
            EvidenceData::Text { content } => Ok(content.clone()),
            EvidenceData::Json { value } => Ok(serde_json::to_string_pretty(value)
                .map_err(|e| EvalError::Evaluation {
                    criterion: "evidence".into(),
                    message: format!("failed to serialize JSON evidence: {e}"),
                })?),
            EvidenceData::Bytes { .. } | EvidenceData::FileRef { .. } => {
                Err(EvalError::Evaluation {
                    criterion: "evidence".into(),
                    message: "cannot extract text from binary evidence".into(),
                })
            }
        }
    }

    /// Extract a typed JSON value from this evidence entry.
    pub fn extract_json<T: serde::de::DeserializeOwned>(&self) -> Result<T, EvalError> {
        match &self.data {
            EvidenceData::Json { value } => serde_json::from_value(value.clone())
                .map_err(|e| EvalError::Evaluation {
                    criterion: "evidence".into(),
                    message: format!("failed to deserialize JSON evidence: {e}"),
                }),
            EvidenceData::Text { content } => serde_json::from_str(content)
                .map_err(|e| EvalError::Evaluation {
                    criterion: "evidence".into(),
                    message: format!("failed to parse text as JSON: {e}"),
                }),
            _ => Err(EvalError::Evaluation {
                criterion: "evidence".into(),
                message: "cannot extract JSON from binary evidence".into(),
            }),
        }
    }

    /// Extract the process exit code from ProcessStatus evidence.
    pub fn extract_exit_code(&self) -> Result<i32, EvalError> {
        let value: serde_json::Value = self.extract_json()?;
        value.get("exit_code")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32)
            .ok_or_else(|| EvalError::Evaluation {
                criterion: "evidence".into(),
                message: "ProcessStatus evidence missing exit_code field".into(),
            })
    }
}
```

### 1.4 EvidenceBag

The evidence bag is the typed container that flows from collectors to criteria.
It is keyed by `EvidenceKind` and may contain multiple entries per kind (e.g.,
multiple screenshots for different viewports).

```rust
// File: crates/roko-eval/src/evidence.rs

/// A typed container of evidence collected from an artifact.
///
/// The bag is the primary input to [`Criterion::evaluate`]. Criteria
/// declare which [`EvidenceKind`]s they require; the runtime populates
/// the bag from one or more [`EvidenceCollector`]s before invoking
/// criteria.
///
/// # Multiple entries per kind
///
/// A bag may contain multiple entries of the same kind. For example,
/// a browser collector might produce screenshots for desktop and mobile
/// viewports -- both are `EvidenceKind::Screenshot` but with different
/// metadata (`viewport=desktop` vs `viewport=mobile`). Criteria that
/// need a specific entry should filter by metadata.
///
/// # Design note
///
/// The bag is intentionally not generic over evidence types. All evidence
/// is dynamically typed via `EvidenceData` to keep the trait signatures
/// simple and to allow criteria to gracefully degrade when optional
/// evidence is absent. Type-safe extraction is provided by helper methods
/// that parse `EvidenceData::Json` into concrete Rust types.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EvidenceBag {
    /// All collected evidence, in collection order.
    entries: Vec<Evidence>,
    /// Index: kind -> indices into `entries`.
    #[serde(skip)]
    index: HashMap<EvidenceKind, Vec<usize>>,
}

impl EvidenceBag {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a piece of evidence to the bag.
    pub fn insert(&mut self, evidence: Evidence) {
        let idx = self.entries.len();
        self.index
            .entry(evidence.kind)
            .or_default()
            .push(idx);
        self.entries.push(evidence);
    }

    /// Get all evidence of a specific kind.
    pub fn get(&self, kind: EvidenceKind) -> Vec<&Evidence> {
        self.index
            .get(&kind)
            .map(|indices| indices.iter().filter_map(|&i| self.entries.get(i)).collect())
            .unwrap_or_default()
    }

    /// Get the first (or only) evidence entry of a specific kind.
    pub fn get_one(&self, kind: EvidenceKind) -> Option<&Evidence> {
        self.index
            .get(&kind)
            .and_then(|indices| indices.first())
            .and_then(|&idx| self.entries.get(idx))
    }

    /// Get evidence by kind, filtered by metadata key-value pair.
    pub fn get_by_metadata(
        &self,
        kind: EvidenceKind,
        key: &str,
        value: &str,
    ) -> Vec<&Evidence> {
        self.get(kind)
            .into_iter()
            .filter(|e| e.metadata.get(key).is_some_and(|v| v == value))
            .collect()
    }

    /// Check whether evidence of a specific kind is available.
    pub fn has(&self, kind: EvidenceKind) -> bool {
        self.index.contains_key(&kind)
    }

    /// Check whether all required kinds are present.
    pub fn has_all(&self, kinds: &[EvidenceKind]) -> bool {
        kinds.iter().all(|k| self.has(*k))
    }

    /// List all evidence kinds present in the bag.
    pub fn available_kinds(&self) -> Vec<EvidenceKind> {
        self.index.keys().copied().collect()
    }

    /// Total number of evidence entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Evidence> {
        self.entries.iter()
    }

    /// Merge another bag into this one (used when composing collectors).
    pub fn merge(&mut self, other: EvidenceBag) {
        for entry in other.entries {
            self.insert(entry);
        }
    }

    /// Total collection cost in milliseconds.
    pub fn total_collection_ms(&self) -> u64 {
        self.entries.iter().map(|e| e.collection_duration_ms).sum()
    }

    /// Rebuild the index after deserialization.
    ///
    /// The index is `#[serde(skip)]` so it must be rebuilt when
    /// loading from disk. Call this after deserialization.
    pub fn rebuild_index(&mut self) {
        self.index.clear();
        for (idx, entry) in self.entries.iter().enumerate() {
            self.index.entry(entry.kind).or_default().push(idx);
        }
    }
}
```

### 1.5 ArtifactRef

An artifact reference describes what is being evaluated. Artifacts are the
input to evidence collectors.

```rust
// File: crates/roko-eval/src/artifact.rs

/// A reference to the artifact under evaluation.
///
/// Artifacts are the things agents produce: a running web page, a code diff,
/// a compiled binary, an API endpoint, a generated document. Evidence
/// collectors use the artifact ref to know what to collect from.
///
/// # Multiple representations
///
/// An artifact may have multiple representations simultaneously. A web UI
/// task has a URL (for browser collection), a code diff (for static
/// analysis), and process output (from the dev server). The artifact ref
/// captures all of these so that different collectors can each find what
/// they need.
///
/// # Conversion from GatePayload
///
/// The existing `GatePayload` (in `crates/roko-gate/src/payload.rs`) maps
/// to `ArtifactRef` via `From<GatePayload>`. The `workdir` field becomes
/// `path`, and the `BuildSystem` is preserved in `context`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactRef {
    /// Unique identifier for this artifact (typically task_id or content hash).
    pub id: String,

    /// Human-readable label (task title, file name, endpoint path).
    #[serde(default)]
    pub label: Option<String>,

    /// Live URL to evaluate (for browser-based evidence collection).
    #[serde(default)]
    pub url: Option<String>,

    /// Filesystem path to the artifact (source file, binary, screenshot).
    #[serde(default)]
    pub path: Option<PathBuf>,

    /// Code diff in unified diff format.
    #[serde(default)]
    pub diff: Option<String>,

    /// Screenshot image path (pre-captured, e.g., from a previous pass).
    #[serde(default)]
    pub screenshot: Option<PathBuf>,

    /// Process output (stdout + stderr from a build/test/run command).
    #[serde(default)]
    pub process_output: Option<ProcessCapture>,

    /// HTTP endpoint to probe (for API evaluation).
    #[serde(default)]
    pub http_endpoint: Option<HttpEndpoint>,

    /// Source files affected by this artifact (for AST analysis).
    #[serde(default)]
    pub source_files: Vec<PathBuf>,

    /// Additional context provided to criteria (task description,
    /// acceptance criteria, visual goal, etc.).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub context: BTreeMap<String, String>,

    /// Content hash of the primary artifact (for cache keying and lineage).
    #[serde(default)]
    pub content_hash: Option<ContentHash>,
}

impl ArtifactRef {
    /// Create an artifact ref from a GateConfig, for bridge compatibility.
    ///
    /// This converts the existing gate infrastructure's `GateConfig` into
    /// the new `ArtifactRef` format so that the `BridgeGateService` can
    /// route through the new evaluation system.
    pub fn from_gate_config(config: &GateConfig, gate_name: &str) -> Self {
        Self {
            id: format!("gate:{gate_name}"),
            label: Some(gate_name.to_string()),
            path: Some(config.workdir.clone()),
            context: {
                let mut ctx = BTreeMap::new();
                ctx.insert("gate_name".into(), gate_name.into());
                ctx.insert("workdir".into(), config.workdir.to_string_lossy().into());
                ctx
            },
            ..Default::default()
        }
    }
}

impl Default for ArtifactRef {
    fn default() -> Self {
        Self {
            id: String::new(),
            label: None,
            url: None,
            path: None,
            diff: None,
            screenshot: None,
            process_output: None,
            http_endpoint: None,
            source_files: Vec::new(),
            context: BTreeMap::new(),
            content_hash: None,
        }
    }
}

/// Captured process output for process-based evidence collection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessCapture {
    /// The command that was run.
    pub command: String,
    /// Process exit code (0 = success).
    pub exit_code: i32,
    /// Captured stdout.
    pub stdout: String,
    /// Captured stderr.
    pub stderr: String,
    /// Wall-clock duration in milliseconds.
    pub duration_ms: u64,
}

/// HTTP endpoint description for API-based evidence collection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpEndpoint {
    pub url: String,
    #[serde(default = "default_http_method")]
    pub method: String,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    #[serde(default)]
    pub body: Option<String>,
}

fn default_http_method() -> String {
    "GET".to_string()
}
```

### 1.6 EvidenceCollector Trait

```rust
// File: crates/roko-eval/src/evidence.rs

/// Collects structured evidence from an artifact.
///
/// Evidence collectors are the bridge between the artifact under test and
/// the criteria that evaluate it. Each collector knows how to extract one
/// or more kinds of evidence from specific artifact representations.
///
/// # Error semantics
///
/// Collector errors are **infrastructure failures**, not evaluation
/// failures. A browser crash, a network timeout, a missing binary -- these
/// prevent evaluation from happening at all. The distinction matters for
/// scoring: infrastructure failures produce `score = 0.0` with a special
/// `InfrastructureFailure` finding, not a low quality score.
#[async_trait]
pub trait EvidenceCollector: Send + Sync {
    /// Collect evidence from the given artifact.
    async fn collect(
        &self,
        artifact: &ArtifactRef,
        ctx: &Context,
    ) -> Result<EvidenceBag, EvalError>;

    /// Which evidence kinds this collector can produce.
    fn produces(&self) -> &[EvidenceKind];

    /// Human-readable name (for traces and debugging).
    fn name(&self) -> &str;

    /// Estimated collection time in milliseconds.
    ///
    /// Used by the runtime to schedule collectors and enforce
    /// time budgets. Returns 0 for instant collectors.
    fn estimated_duration_ms(&self) -> u64 {
        0
    }
}
```

### 1.7 CompositeCollector

```rust
// File: crates/roko-eval/src/evidence.rs

/// Runs multiple evidence collectors against the same artifact and
/// merges their output into a single `EvidenceBag`.
///
/// Collectors run sequentially by default. Use `with_parallel(true)` to
/// run independent collectors concurrently.
pub struct CompositeCollector {
    collectors: Vec<Box<dyn EvidenceCollector>>,
    name: String,
    parallel: bool,
}

impl CompositeCollector {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            collectors: Vec::new(),
            name: name.into(),
            parallel: false,
        }
    }

    pub fn with(mut self, collector: impl EvidenceCollector + 'static) -> Self {
        self.collectors.push(Box::new(collector));
        self
    }

    pub fn with_parallel(mut self, parallel: bool) -> Self {
        self.parallel = parallel;
        self
    }
}

#[async_trait]
impl EvidenceCollector for CompositeCollector {
    async fn collect(
        &self,
        artifact: &ArtifactRef,
        ctx: &Context,
    ) -> Result<EvidenceBag, EvalError> {
        let mut bag = EvidenceBag::new();
        if self.parallel {
            // Run all collectors concurrently via tokio::join_all.
            // Infrastructure errors are logged but do not abort other collectors.
            let futures: Vec<_> = self.collectors.iter()
                .map(|c| c.collect(artifact, ctx))
                .collect();
            let results = futures::future::join_all(futures).await;
            for result in results {
                match result {
                    Ok(sub_bag) => bag.merge(sub_bag),
                    Err(EvalError::Infrastructure { component, message, .. }) => {
                        tracing::warn!(
                            collector = %component,
                            "infrastructure failure during parallel collection: {message}"
                        );
                    }
                    Err(e) => return Err(e),
                }
            }
        } else {
            // Sequential: infrastructure errors are logged, collection continues.
            for collector in &self.collectors {
                match collector.collect(artifact, ctx).await {
                    Ok(sub_bag) => bag.merge(sub_bag),
                    Err(EvalError::Infrastructure { component, message, .. }) => {
                        tracing::warn!(
                            collector = %component,
                            "infrastructure failure during collection: {message}"
                        );
                    }
                    Err(e) => return Err(e),
                }
            }
        }
        Ok(bag)
    }

    fn produces(&self) -> &[EvidenceKind] {
        // Computed at construction time: union of all sub-collector produces().
        &[]
    }

    fn name(&self) -> &str {
        &self.name
    }
}
```

### 1.8 EvalError

```rust
// File: crates/roko-eval/src/lib.rs

/// Errors that can occur during evaluation.
///
/// The distinction between `Infrastructure` and `Evaluation` errors is
/// load-bearing: they produce different scores and different feedback.
#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EvalError {
    /// Infrastructure failure -- evaluation could not happen at all.
    ///
    /// Scoring: `score = 0.0`, finding severity = `infrastructure`.
    /// These are NOT quality judgments -- they indicate the verifier
    /// itself failed.
    #[error("infrastructure failure in {component}: {message}")]
    Infrastructure {
        component: String,
        message: String,
        retriable: bool,
    },

    /// Evaluation failure -- the criterion ran but produced an error.
    #[error("evaluation error in {criterion}: {message}")]
    Evaluation {
        criterion: String,
        message: String,
    },

    /// Configuration error -- invalid criterion or profile config.
    #[error("configuration error: {message}")]
    Configuration { message: String },

    /// Budget exceeded -- evaluation aborted to stay within cost/time limits.
    #[error("budget exceeded: {message}")]
    BudgetExceeded { message: String },

    /// Evidence not available -- a required evidence kind was not collected.
    /// This is distinct from infrastructure failure: the collector succeeded
    /// but the artifact did not produce the required evidence kind.
    #[error("evidence {kind:?} not available for criterion {criterion}")]
    EvidenceUnavailable {
        criterion: String,
        kind: EvidenceKind,
    },
}
```

---

## 2. The Criterion Trait

### 2.1 Design Motivation

A criterion evaluates one dimension of quality given evidence. The key design
decisions:

1. **Single-dimension scoring** -- each criterion produces exactly one `f64`
   score in `[0.0, 1.0]`. Multi-dimensional evaluation is achieved by
   composing criteria in a profile, not by making criteria multi-valued.

2. **Evidence-driven** -- criteria declare what evidence they need. The runtime
   skips criteria whose evidence requirements are not met.

3. **Findings with grounding** -- following Liang et al. (ICML 2024, UICrit),
   criteria produce findings grounded in the evidence: bounding boxes on
   screenshots, selectors in DOM, line numbers in code.

4. **Criterion kinds** -- deterministic criteria (APCA, compile check) are
   distinguished from subjective criteria (LLM judge) because they have
   different reliability profiles and different roles in composition.

### 2.2 CriterionKind

```rust
// File: crates/roko-eval/src/criterion.rs

/// The kind of evaluation a criterion performs.
///
/// Affects how results are composed in profiles and whether the
/// criterion can serve as a hard gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CriterionKind {
    /// Deterministic: exact computation with no stochastic component.
    /// Examples: APCA contrast, compile gate, test pass/fail.
    /// Properties: reproducible, can be hard gates.
    Deterministic,

    /// Computed: algorithmic with known error bounds.
    /// Examples: CWV measurement noise, visual regression anti-aliasing.
    /// Properties: mostly reproducible, small confidence interval.
    Computed,

    /// Judge panel: LLM-based evaluation with pairwise comparison.
    /// Examples: visual quality, code review, design system fit.
    /// Properties: stochastic, requires confidence interval.
    JudgePanel,

    /// Heuristic: advisory signal with unknown error bounds.
    /// Examples: saliency analysis, colorfulness, AIM feature congestion.
    /// Properties: informational, must NEVER be a hard gate.
    Heuristic,
}

impl CriterionKind {
    /// Whether this kind can serve as a hard gate.
    ///
    /// Only deterministic and computed criteria have the reliability
    /// profile required for hard gating. Judge panels and heuristics
    /// must be soft-only.
    pub fn can_be_hard(&self) -> bool {
        matches!(self, Self::Deterministic | Self::Computed)
    }
}
```

### 2.3 Severity

```rust
// File: crates/roko-eval/src/criterion.rs

/// Severity of a finding.
///
/// Maps to the existing `roko_gate::feedback::Severity` but adds the
/// `Infrastructure` level for evidence collection failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Infrastructure,
    Hard,
    Soft,
    Info,
}

impl From<Severity> for roko_gate::feedback::Severity {
    fn from(s: Severity) -> Self {
        match s {
            Severity::Infrastructure | Severity::Hard => roko_gate::feedback::Severity::Error,
            Severity::Soft => roko_gate::feedback::Severity::Warning,
            Severity::Info => roko_gate::feedback::Severity::Info,
        }
    }
}
```

### 2.4 Finding

```rust
// File: crates/roko-eval/src/criterion.rs

/// A grounded finding from a criterion evaluation.
///
/// Every finding is grounded in evidence: a bounding box on a screenshot,
/// a CSS selector in the DOM, a line number in code. Grounded findings
/// are actionable; vague findings are not.
///
/// Findings serve two purposes:
/// 1. **Agent feedback** -- structured, actionable instructions.
/// 2. **Flywheel data** -- findings with bounding boxes become training
///    signal for the preference model (PRD-05).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Finding {
    pub criterion: String,
    pub severity: Severity,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bounding_box: Option<BoundingBox>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selector: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_location: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wcag_sc: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fix_hint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_kind: Option<EvidenceKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rule_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_tool: Option<String>,
    /// AST node path for AST-grounded findings (e.g., "mod::struct::method").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ast_path: Option<String>,
    /// Confidence in this finding (0.0 to 1.0).
    /// Deterministic findings have confidence 1.0.
    /// Judge-derived findings carry the judge panel's agreement level.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
}

impl Default for Severity {
    fn default() -> Self {
        Self::Info
    }
}

/// A bounding box for visual grounding of findings.
/// Coordinates are normalized to [0.0, 1.0] relative to screenshot dimensions.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BoundingBox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl BoundingBox {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x: x.clamp(0.0, 1.0),
            y: y.clamp(0.0, 1.0),
            width: width.clamp(0.0, 1.0),
            height: height.clamp(0.0, 1.0),
        }
    }

    pub fn from_pixels(
        px_x: f64, px_y: f64, px_w: f64, px_h: f64,
        img_width: f64, img_height: f64,
    ) -> Self {
        Self::new(
            px_x / img_width,
            px_y / img_height,
            px_w / img_width,
            px_h / img_height,
        )
    }

    pub fn area(&self) -> f64 {
        self.width * self.height
    }
}
```

### 2.5 CriterionResult

```rust
// File: crates/roko-eval/src/criterion.rs

/// The result of a single criterion evaluation.
///
/// Scores are `f64` in `[0.0, 1.0]`:
/// - `0.0` = complete failure or infrastructure error
/// - `0.0..0.5` = failing range
/// - `0.5..0.8` = marginal
/// - `0.8..1.0` = good
/// - `1.0` = perfect
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriterionResult {
    pub criterion: String,
    pub kind: CriterionKind,
    pub score: f64,
    pub passed: bool,
    pub findings: Vec<Finding>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence_interval: Option<ConfidenceInterval>,
    pub duration_ms: u64,
    #[serde(default)]
    pub cost_usd: f64,
    /// Arbitrary metadata from the criterion (test counts, vulnerability
    /// counts, etc.).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl Default for CriterionResult {
    fn default() -> Self {
        Self {
            criterion: String::new(),
            kind: CriterionKind::Deterministic,
            score: 0.0,
            passed: false,
            findings: Vec::new(),
            confidence_interval: None,
            duration_ms: 0,
            cost_usd: 0.0,
            metadata: None,
        }
    }
}

impl CriterionResult {
    /// Convert this CriterionResult into a GateVerdict for bridge compatibility.
    ///
    /// Maps the new structured result back to the flat GateVerdict format
    /// used by orchestrate.rs.
    pub fn into_gate_verdict(self, gate_name: String) -> GateVerdict {
        let output = if self.findings.is_empty() {
            if self.passed { "passed".into() } else { "failed".into() }
        } else {
            self.findings.iter()
                .map(|f| f.summary.as_str())
                .collect::<Vec<_>>()
                .join("\n")
        };

        GateVerdict {
            gate_name,
            passed: self.passed,
            skipped: false,
            skip_reason: None,
            output,
            duration_ms: self.duration_ms,
        }
    }
}

/// A confidence interval on a criterion score.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ConfidenceInterval {
    pub lower: f64,
    pub upper: f64,
    pub level: f64,
    pub bootstrap_n: u32,
}
```

### 2.6 Criterion Trait

```rust
// File: crates/roko-eval/src/criterion.rs

/// Evaluates one dimension of quality given collected evidence.
///
/// Criteria are the atomic unit of evaluation. Each criterion:
/// 1. Declares what evidence it requires.
/// 2. Declares its kind (deterministic, computed, judge, heuristic).
/// 3. Declares whether it is a hard gate.
/// 4. Evaluates the evidence and produces a `CriterionResult`.
///
/// Criteria do not compose with each other directly. Composition happens
/// at the `Profile` level.
#[async_trait]
pub trait Criterion: Send + Sync {
    /// Evaluate the evidence and produce a result.
    ///
    /// The caller guarantees that `evidence.has_all(self.required_evidence())`
    /// is true. Implementations may unwrap evidence lookups for required kinds.
    async fn evaluate(
        &self,
        artifact: &ArtifactRef,
        evidence: &EvidenceBag,
        ctx: &Context,
    ) -> Result<CriterionResult, EvalError>;

    /// Which evidence kinds this criterion requires.
    fn required_evidence(&self) -> &[EvidenceKind];

    /// What kind of criterion this is.
    fn criterion_kind(&self) -> CriterionKind;

    /// Whether this criterion is a hard gate (conjunctive semantics).
    fn is_hard(&self) -> bool;

    /// Human-readable name.
    fn name(&self) -> &str;

    /// Optional evidence kinds that improve evaluation quality.
    fn optional_evidence(&self) -> &[EvidenceKind] {
        &[]
    }

    /// The default threshold for pass/fail.
    fn default_threshold(&self) -> f64 {
        0.5
    }
}
```

### 2.7 LegacyCriterion (Bridge Adapter)

This adapter wraps an existing `Verify` implementation as a `Criterion`,
enabling incremental migration. The bridge constructs a `GatePayload` engram
from the `ArtifactRef` and interprets the `Verdict` as a `CriterionResult`.

```rust
// File: crates/roko-eval/src/bridge.rs

/// Wraps an existing `Verify` implementation as a `Criterion`.
///
/// This is the migration adapter that allows existing gates (CompileGate,
/// ClippyGate, etc.) to participate in the new evaluation framework
/// without modification.
///
/// The adapter:
/// 1. Constructs a `GatePayload` engram from the `ArtifactRef`
/// 2. Calls `verify()` on the inner gate
/// 3. Converts the `Verdict` to a `CriterionResult`
///
/// # Limitations
///
/// - Evidence requirements are always `[ProcessOutput, ProcessStatus]`
///   because legacy gates handle their own evidence collection.
/// - The `is_hard` flag defaults to `true` because all existing rung gates
///   are hard gates.
/// - No structured findings -- the verdict's `reason` and `detail` are
///   used as a single finding summary.
pub struct LegacyCriterion {
    gate: Box<dyn Verify>,
    name: String,
    kind: CriterionKind,
}

impl LegacyCriterion {
    pub fn new(gate: Box<dyn Verify>, name: impl Into<String>) -> Self {
        Self {
            gate,
            name: name.into(),
            kind: CriterionKind::Deterministic,
        }
    }

    pub fn with_kind(mut self, kind: CriterionKind) -> Self {
        self.kind = kind;
        self
    }
}

#[async_trait]
impl Criterion for LegacyCriterion {
    async fn evaluate(
        &self,
        artifact: &ArtifactRef,
        _evidence: &EvidenceBag,
        ctx: &Context,
    ) -> Result<CriterionResult, EvalError> {
        let payload = GatePayload::in_dir(
            artifact.path.clone().unwrap_or_else(|| PathBuf::from("."))
        );
        let signal = Engram::builder(Kind::Task)
            .body(Body::from_json(&payload).map_err(|e| EvalError::Configuration {
                message: format!("failed to serialize GatePayload: {e}"),
            })?)
            .build();

        let started = std::time::Instant::now();
        let verdict = self.gate.verify(&signal, ctx).await;
        let duration_ms = started.elapsed().as_millis() as u64;

        let findings = if !verdict.passed {
            vec![Finding {
                criterion: self.name.clone(),
                severity: Severity::Hard,
                summary: verdict.reason.clone(),
                detail: verdict.detail.clone(),
                ..Default::default()
            }]
        } else {
            vec![]
        };

        Ok(CriterionResult {
            criterion: self.name.clone(),
            kind: self.kind,
            score: verdict.score as f64,
            passed: verdict.passed,
            findings,
            duration_ms,
            ..Default::default()
        })
    }

    fn required_evidence(&self) -> &[EvidenceKind] {
        // Legacy gates collect their own evidence; no external requirements.
        &[]
    }

    fn criterion_kind(&self) -> CriterionKind {
        self.kind
    }

    fn is_hard(&self) -> bool {
        true
    }

    fn name(&self) -> &str {
        &self.name
    }
}
```

---

## 3. The Profile Type

### 3.1 Design Motivation

A profile is a named, shareable composition of criteria. Profiles are **not a
trait** -- they are a configuration/data structure that the runtime interprets.
This is intentional: profiles are authored in TOML by users, not implemented
in Rust by developers.

### 3.2 CompositionStrategy

```rust
// File: crates/roko-eval/src/profile.rs

/// How criteria are composed within a profile.
///
/// All strategies enforce the invariant that hard criteria are conjunctive
/// (all must pass), regardless of the soft criterion composition mode.
///
/// There is no `WeightedSum` variant. This is deliberate.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CompositionStrategy {
    Conjunctive,
    Pareto,
    Sequential,
    Parallel,
    Voting { required: u32 },
    Fallback,
    Custom { function: String },
}

impl Default for CompositionStrategy {
    fn default() -> Self {
        Self::Conjunctive
    }
}
```

### 3.3 Profile Definition

```rust
// File: crates/roko-eval/src/profile.rs

/// A named, shareable evaluation strategy.
///
/// Profiles compose criteria into a complete evaluation. They are the
/// "presets" of the system -- users create profiles for their specific
/// use case and share them via the marketplace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    /// Unique identifier (e.g., "startup-mvp", "wcag-aa-strict").
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Description of what this profile evaluates.
    #[serde(default)]
    pub description: Option<String>,
    /// How criteria are composed.
    #[serde(default)]
    pub strategy: CompositionStrategy,
    /// Criteria to evaluate, with per-criterion config overrides.
    pub criteria: Vec<CriterionConfig>,
    /// Retry policy for iterative evaluation.
    #[serde(default)]
    pub retry: RetryPolicy,
    /// Evidence collectors to run (if empty, inferred from criteria requirements).
    #[serde(default)]
    pub collectors: Vec<CollectorConfig>,
    /// Tags for marketplace discovery.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Version for marketplace publishing.
    #[serde(default)]
    pub version: Option<String>,
    /// Parent profile (for fork chains).
    #[serde(default)]
    pub forked_from: Option<String>,
}

/// Configuration for a criterion instance within a profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriterionConfig {
    /// Reference to this criterion in the registry.
    pub ref_: CriterionRef,
    #[serde(default)]
    pub threshold: Option<f64>,
    #[serde(default)]
    pub hard: Option<bool>,
    #[serde(default)]
    pub params: BTreeMap<String, toml::Value>,
}

/// Reference to a criterion (built-in name or registry path).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CriterionRef {
    BuiltIn(String),
    Registry { registry: String, name: String, version: Option<String> },
}

/// Configuration for a collector within a profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectorConfig {
    pub name: String,
    #[serde(default)]
    pub params: BTreeMap<String, toml::Value>,
}
```

### 3.4 RetryPolicy

```rust
// File: crates/roko-eval/src/profile.rs

/// Retry policy for profile evaluation.
///
/// Uses control-theory primitives: dead-band (hysteresis),
/// anti-windup (max retries), and derivative term (improvement rate).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    #[serde(default = "default_dead_band")]
    pub dead_band: f64,
    #[serde(default = "default_min_improvement_rate")]
    pub min_improvement_rate: f64,
    #[serde(default = "default_improvement_window")]
    pub improvement_window: u32,
    #[serde(default = "default_cooldown_ms")]
    pub cooldown_ms: u64,
    #[serde(default = "default_carry_forward")]
    pub carry_forward_passes: bool,
}

fn default_max_retries() -> u32 { 3 }
fn default_dead_band() -> f64 { 0.05 }
fn default_min_improvement_rate() -> f64 { 0.02 }
fn default_improvement_window() -> u32 { 3 }
fn default_cooldown_ms() -> u64 { 1000 }
fn default_carry_forward() -> bool { true }

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: default_max_retries(),
            dead_band: default_dead_band(),
            min_improvement_rate: default_min_improvement_rate(),
            improvement_window: default_improvement_window(),
            cooldown_ms: default_cooldown_ms(),
            carry_forward_passes: default_carry_forward(),
        }
    }
}
```

---

## 4. EvalVerdict and EvalTrace

### 4.1 EvalVerdict

```rust
// File: crates/roko-eval/src/profile.rs

/// The aggregate result of evaluating a profile.
///
/// Contains per-criterion results plus the aggregate pass/fail and score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalVerdict {
    /// Which profile was evaluated.
    pub profile_id: String,
    /// Per-criterion results, in evaluation order.
    pub criterion_results: Vec<CriterionResult>,
    /// Aggregate pass/fail.
    pub passed: bool,
    /// Aggregate score (composition-strategy-dependent).
    pub score: f64,
    /// All findings from all criteria, sorted by severity.
    pub findings: Vec<Finding>,
    /// Total evaluation duration (wall-clock, ms).
    pub duration_ms: u64,
    /// Total evaluation cost (USD).
    pub cost_usd: f64,
    /// Retry attempt number (0 = first attempt).
    pub attempt: u32,
    /// Score history across retry attempts (for derivative term).
    #[serde(default)]
    pub score_history: Vec<f64>,
}
```

### 4.2 EvalTrace

```rust
// File: crates/roko-eval/src/trace.rs

/// Complete trace of an evaluation for the flywheel.
///
/// Every evaluation produces a trace. Traces feed:
/// - Preference mining (PRD-05)
/// - Curriculum from failures (PRD-05)
/// - MIPROv2 optimization (PRD-05)
/// - Adaptive threshold learning
/// - Dashboard visualization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalTrace {
    /// Unique trace ID.
    pub trace_id: String,
    /// Timestamp (Unix ms).
    pub timestamp_ms: i64,
    /// The artifact that was evaluated.
    pub artifact: ArtifactRef,
    /// Which profile was used.
    pub profile_id: String,
    /// Evidence collected (kinds + collection durations, not raw data).
    pub evidence_summary: Vec<EvidenceSummary>,
    /// Per-criterion results.
    pub criterion_results: Vec<CriterionResult>,
    /// Aggregate verdict.
    pub verdict: EvalVerdict,
    /// Runtime context (agent ID, task ID, plan ID, etc.).
    pub context: BTreeMap<String, String>,
}

/// Summary of evidence collection (for traces without embedding raw data).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceSummary {
    pub kind: EvidenceKind,
    pub collector: String,
    pub duration_ms: u64,
    pub size_bytes: Option<u64>,
}
```

---

## 5. The EvalService Runtime

```rust
// File: crates/roko-eval/src/service.rs

/// The evaluation runtime.
///
/// `EvalService` is the new runtime that replaces `GateService` for
/// evaluation. It orchestrates evidence collection and criterion
/// evaluation according to a profile's composition strategy.
///
/// The service:
/// 1. Resolves the profile's criteria from the registry.
/// 2. Determines which evidence collectors are needed.
/// 3. Runs collectors (respecting budget/timeout constraints).
/// 4. Evaluates criteria according to the composition strategy.
/// 5. Produces an `EvalVerdict` and `EvalTrace`.
/// 6. Records outcomes for adaptive learning.
pub struct EvalService {
    criterion_registry: CriterionRegistry,
    profile_registry: ProfileRegistry,
    collector_registry: CollectorRegistry,
    adaptive: Option<Arc<Mutex<AdaptiveThresholds>>>,
    trace_sink: Option<Box<dyn TraceSink>>,
}

/// Trait for sinks that receive evaluation traces.
#[async_trait]
pub trait TraceSink: Send + Sync {
    async fn record(&self, trace: EvalTrace) -> Result<(), EvalError>;
}

impl EvalService {
    pub fn new(
        criterion_registry: CriterionRegistry,
        profile_registry: ProfileRegistry,
        collector_registry: CollectorRegistry,
    ) -> Self {
        Self {
            criterion_registry,
            profile_registry,
            collector_registry,
            adaptive: None,
            trace_sink: None,
        }
    }

    /// Evaluate an artifact against a profile.
    pub async fn evaluate(
        &self,
        profile_id: &str,
        artifact: &ArtifactRef,
        ctx: &Context,
    ) -> Result<EvalVerdict, EvalError> {
        let profile = self.profile_registry.get(profile_id)
            .ok_or_else(|| EvalError::Configuration {
                message: format!("profile not found: {profile_id}"),
            })?;

        // Phase 1: Collect evidence
        let required_kinds = self.compute_required_evidence(&profile);
        let evidence = self.collect_evidence(artifact, &required_kinds, ctx).await?;

        // Phase 2: Evaluate criteria
        let results = self.evaluate_criteria(&profile, artifact, &evidence, ctx).await?;

        // Phase 3: Compose verdict
        let verdict = self.compose_verdict(&profile, results)?;

        // Phase 4: Record trace
        if let Some(sink) = &self.trace_sink {
            let trace = EvalTrace {
                trace_id: uuid::Uuid::new_v4().to_string(),
                timestamp_ms: chrono::Utc::now().timestamp_millis(),
                artifact: artifact.clone(),
                profile_id: profile_id.to_string(),
                evidence_summary: evidence.iter()
                    .map(|e| EvidenceSummary {
                        kind: e.kind,
                        collector: e.collector.clone(),
                        duration_ms: e.collection_duration_ms,
                        size_bytes: None,
                    })
                    .collect(),
                criterion_results: verdict.criterion_results.clone(),
                verdict: verdict.clone(),
                context: BTreeMap::new(),
            };
            let _ = sink.record(trace).await;
        }

        Ok(verdict)
    }

    /// Evaluate a single criterion (for bridge compatibility).
    pub async fn evaluate_criterion(
        &self,
        criterion_name: &str,
        artifact: &ArtifactRef,
    ) -> Result<CriterionResult, EvalError> {
        let criterion = self.criterion_registry.get(criterion_name)
            .ok_or_else(|| EvalError::Configuration {
                message: format!("criterion not found: {criterion_name}"),
            })?;

        let ctx = Context::now();
        let evidence = self.collect_evidence(
            artifact,
            &criterion.required_evidence().to_vec(),
            &ctx,
        ).await?;

        criterion.evaluate(artifact, &evidence, &ctx).await
    }

    fn compute_required_evidence(&self, profile: &Profile) -> Vec<EvidenceKind> {
        let mut kinds = Vec::new();
        for config in &profile.criteria {
            if let Some(criterion) = self.criterion_registry.get_by_ref(&config.ref_) {
                kinds.extend_from_slice(criterion.required_evidence());
                kinds.extend_from_slice(criterion.optional_evidence());
            }
        }
        kinds.sort();
        kinds.dedup();
        kinds
    }

    async fn collect_evidence(
        &self,
        artifact: &ArtifactRef,
        required: &[EvidenceKind],
        ctx: &Context,
    ) -> Result<EvidenceBag, EvalError> {
        let collectors = self.collector_registry.collectors_for_kinds(required);
        let composite = CompositeCollector::new("eval-service")
            .with_parallel(true);
        // Build composite from matched collectors
        let mut bag = EvidenceBag::new();
        for collector in collectors {
            match collector.collect(artifact, ctx).await {
                Ok(sub_bag) => bag.merge(sub_bag),
                Err(EvalError::Infrastructure { .. }) => {
                    // Log and continue
                }
                Err(e) => return Err(e),
            }
        }
        Ok(bag)
    }

    async fn evaluate_criteria(
        &self,
        profile: &Profile,
        artifact: &ArtifactRef,
        evidence: &EvidenceBag,
        ctx: &Context,
    ) -> Result<Vec<CriterionResult>, EvalError> {
        let mut results = Vec::new();
        for config in &profile.criteria {
            let Some(criterion) = self.criterion_registry.get_by_ref(&config.ref_) else {
                continue;
            };

            // Skip if required evidence is missing
            if !evidence.has_all(criterion.required_evidence()) {
                let missing: Vec<_> = criterion.required_evidence().iter()
                    .filter(|k| !evidence.has(**k))
                    .collect();
                tracing::info!(
                    criterion = criterion.name(),
                    ?missing,
                    "skipping criterion: missing required evidence"
                );
                continue;
            }

            let result = criterion.evaluate(artifact, evidence, ctx).await?;
            let is_hard = config.hard.unwrap_or(criterion.is_hard());

            // Short-circuit on hard failure in Sequential strategy
            if matches!(profile.strategy, CompositionStrategy::Sequential)
                && is_hard
                && !result.passed
            {
                results.push(result);
                break;
            }

            results.push(result);
        }
        Ok(results)
    }

    fn compose_verdict(
        &self,
        profile: &Profile,
        results: Vec<CriterionResult>,
    ) -> Result<EvalVerdict, EvalError> {
        let all_hard_passed = results.iter()
            .all(|r| {
                let criterion = self.criterion_registry.get(&r.criterion);
                let is_hard = criterion.map(|c| c.is_hard()).unwrap_or(true);
                !is_hard || r.passed
            });

        let aggregate_score = match &profile.strategy {
            CompositionStrategy::Conjunctive | CompositionStrategy::Sequential => {
                results.iter()
                    .map(|r| r.score)
                    .fold(f64::INFINITY, f64::min)
                    .min(1.0)
            }
            CompositionStrategy::Parallel => {
                let sum: f64 = results.iter().map(|r| r.score).sum();
                if results.is_empty() { 1.0 } else { sum / results.len() as f64 }
            }
            CompositionStrategy::Voting { required } => {
                let passing = results.iter().filter(|r| r.passed).count() as u32;
                if passing >= *required { 1.0 } else { passing as f64 / *required as f64 }
            }
            _ => results.iter().map(|r| r.score).fold(f64::INFINITY, f64::min).min(1.0),
        };

        let findings: Vec<Finding> = results.iter()
            .flat_map(|r| r.findings.clone())
            .collect();

        let duration_ms: u64 = results.iter().map(|r| r.duration_ms).sum();
        let cost_usd: f64 = results.iter().map(|r| r.cost_usd).sum();

        Ok(EvalVerdict {
            profile_id: profile.id.clone(),
            criterion_results: results,
            passed: all_hard_passed && aggregate_score > 0.0,
            score: aggregate_score,
            findings,
            duration_ms,
            cost_usd,
            attempt: 0,
            score_history: vec![aggregate_score],
        })
    }
}
```

---

## 6. Registry Types

```rust
// File: crates/roko-eval/src/registry.rs

/// Registry of criterion implementations.
///
/// Replaces the hardcoded `match name` dispatch in `GateService::gate_for_name`.
/// Criteria are registered by name and can be looked up by name or by
/// `CriterionRef`.
pub struct CriterionRegistry {
    criteria: HashMap<String, Arc<dyn Criterion>>,
}

impl CriterionRegistry {
    pub fn new() -> Self {
        Self { criteria: HashMap::new() }
    }

    /// Register a criterion implementation.
    pub fn register(&mut self, criterion: Arc<dyn Criterion>) {
        self.criteria.insert(criterion.name().to_string(), criterion);
    }

    /// Look up a criterion by name.
    pub fn get(&self, name: &str) -> Option<&Arc<dyn Criterion>> {
        self.criteria.get(name)
    }

    /// Look up a criterion by reference.
    pub fn get_by_ref(&self, ref_: &CriterionRef) -> Option<&Arc<dyn Criterion>> {
        match ref_ {
            CriterionRef::BuiltIn(name) => self.get(name),
            CriterionRef::Registry { name, .. } => self.get(name),
        }
    }

    /// Create a registry with all built-in criteria.
    pub fn with_builtins() -> Self {
        let mut reg = Self::new();
        // Register all built-in criteria from roko-eval-metrics
        reg.register(Arc::new(CompileCriterion::cargo()));
        reg.register(Arc::new(LintCriterion::cargo_strict()));
        reg.register(Arc::new(TestCriterion::cargo()));
        reg.register(Arc::new(FormatCriterion::cargo()));
        reg.register(Arc::new(SecurityCriterion::critical_only()));
        reg.register(Arc::new(DiffCriterion::default()));
        reg
    }
}

/// Registry of evidence collectors.
pub struct CollectorRegistry {
    collectors: Vec<Arc<dyn EvidenceCollector>>,
}

impl CollectorRegistry {
    pub fn new() -> Self {
        Self { collectors: Vec::new() }
    }

    pub fn register(&mut self, collector: Arc<dyn EvidenceCollector>) {
        self.collectors.push(collector);
    }

    /// Find collectors that can produce the requested evidence kinds.
    pub fn collectors_for_kinds(&self, kinds: &[EvidenceKind]) -> Vec<&Arc<dyn EvidenceCollector>> {
        self.collectors.iter()
            .filter(|c| c.produces().iter().any(|p| kinds.contains(p)))
            .collect()
    }
}

/// Registry of profiles.
pub struct ProfileRegistry {
    profiles: HashMap<String, Profile>,
}

impl ProfileRegistry {
    pub fn new() -> Self {
        Self { profiles: HashMap::new() }
    }

    pub fn register(&mut self, profile: Profile) {
        self.profiles.insert(profile.id.clone(), profile);
    }

    pub fn get(&self, id: &str) -> Option<&Profile> {
        self.profiles.get(id)
    }
}
```

---

## 7. TOML Authoring Format

Profiles are authored in TOML files that users create, share, and fork.

```toml
# File: .roko/profiles/rust-strict.toml

[profile]
id = "rust-strict"
name = "Rust Strict"
description = "Strict evaluation for Rust projects: compile, lint, test, format, security"
version = "1.0.0"
tags = ["rust", "strict", "ci"]

[profile.strategy]
kind = "sequential"

[[profile.criteria]]
ref_ = "compile"
hard = true

[[profile.criteria]]
ref_ = "lint"
hard = true
[profile.criteria.params]
strict = true

[[profile.criteria]]
ref_ = "test"
hard = true

[[profile.criteria]]
ref_ = "format"
hard = true

[[profile.criteria]]
ref_ = "security"
hard = false
threshold = 0.8

[[profile.criteria]]
ref_ = "diff"
hard = false
threshold = 0.5

[profile.retry]
max_retries = 3
dead_band = 0.05
carry_forward_passes = true
```

---

## 8. Implementation Checklist

| # | File | Type | What |
|---|---|---|---|
| 1 | `crates/roko-eval/Cargo.toml` | New | Crate manifest, depends on roko-core |
| 2 | `crates/roko-eval/src/lib.rs` | New | Crate root, re-exports, EvalError |
| 3 | `crates/roko-eval/src/evidence.rs` | New | EvidenceKind, Evidence, EvidenceData, EvidenceBag, EvidenceCollector, CompositeCollector |
| 4 | `crates/roko-eval/src/criterion.rs` | New | CriterionKind, Severity, Finding, BoundingBox, CriterionResult, ConfidenceInterval, Criterion trait |
| 5 | `crates/roko-eval/src/artifact.rs` | New | ArtifactRef, ProcessCapture, HttpEndpoint |
| 6 | `crates/roko-eval/src/profile.rs` | New | Profile, CompositionStrategy, RetryPolicy, CriterionConfig, EvalVerdict |
| 7 | `crates/roko-eval/src/trace.rs` | New | EvalTrace, EvidenceSummary |
| 8 | `crates/roko-eval/src/registry.rs` | New | CriterionRegistry, ProfileRegistry, CollectorRegistry |
| 9 | `crates/roko-eval/src/bridge.rs` | New | LegacyCriterion adapter |
| 10 | `crates/roko-eval/src/service.rs` | New | EvalService runtime |
| 11 | `crates/roko-gate/src/bridge.rs` | New | BridgeGateService (EvalService -> GateRunner) |
| 12 | `Cargo.toml` (workspace) | Modify | Add `roko-eval` member |
