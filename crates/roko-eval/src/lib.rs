#![allow(clippy::missing_fields_in_debug)]
//! Unified evaluation framework for Roko.
//!
//! This crate separates **evidence collection** from **judgment** via two core
//! traits:
//!
//! - [`EvidenceCollector`] — produces typed evidence from artifacts (e.g., runs
//!   `cargo check`, captures stdout/stderr/exit code).
//! - [`Criterion`] — scores one dimension of quality given an evidence bag.
//!
//! A [`Profile`] composes multiple criteria with a composition strategy.
//!
//! # Bridge adapters
//!
//! The existing gate pipeline in `roko-gate` continues to work unchanged.
//! [`BoxedLegacyCriterion`] wraps any [`Verify`](roko_core::Verify) implementation
//! as a [`Criterion`] (evidence-bag API). [`LegacyCriterion`] provides the
//! generic bridge to the opaque evaluation API.
//!
//! # Phase 1 scope
//!
//! This is the initial Phase 1 implementation. It provides core types, two
//! initial collectors ([`ProcessCollector`] and [`DiffCollector`]), and the
//! bridge adapters. Phases 2-4 (migrating gates to criteria, registry-driven
//! dispatch, user-authored criteria) are follow-on work.

use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::time::Instant;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Errors that can occur during evidence collection.
#[derive(Debug, thiserror::Error)]
pub enum CollectError {
    /// The requested command failed to spawn.
    #[error("failed to spawn process: {0}")]
    SpawnFailed(String),

    /// The process timed out.
    #[error("process timed out after {0} ms")]
    Timeout(u64),

    /// An I/O error occurred.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Any other collection error.
    #[error("{0}")]
    Other(String),
}

/// Errors from running a full evaluation profile.
#[derive(Debug, thiserror::Error)]
pub enum EvalError {
    /// Evidence collection failed for a required kind.
    #[error("evidence collection failed for {kind:?}: {source}")]
    CollectionFailed {
        /// The evidence kind that failed.
        kind: EvidenceKind,
        /// The underlying collection error.
        source: CollectError,
    },

    /// A criterion required evidence that was not present in the bag.
    #[error("missing required evidence {kind:?} for criterion {criterion}")]
    MissingEvidence {
        /// The missing evidence kind.
        kind: EvidenceKind,
        /// The criterion that needed it.
        criterion: String,
    },

    /// Any other evaluation error.
    #[error("{0}")]
    Other(String),
}

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// Identifies the kind of evidence a collector produces.
///
/// Each collector declares one kind; criteria declare which kinds they need.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    /// Output from `cargo check` / `cargo build`.
    Compile,
    /// Output from `cargo clippy`.
    Clippy,
    /// Output from `cargo test`.
    Test,
    /// Output from `cargo fmt --check`.
    Format,
    /// Output from `git diff --stat`.
    Diff,
    /// Output from a custom shell command.
    Shell,
    /// Output from `cargo audit` or similar security scanner.
    SecurityScan,
}

/// A reference to the artifact being evaluated.
///
/// This is distinct from `roko_core::ArtifactRef` (which describes a published
/// artifact with publisher/name/version). An `EvalArtifactRef` points to a
/// workspace directory that is being evaluated by the gate/criterion pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalArtifactRef {
    /// Root path of the artifact being evaluated (typically a workspace root).
    pub path: PathBuf,
    /// Optional human-readable label (e.g., plan name, task id).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

impl EvalArtifactRef {
    /// Construct a new artifact reference from a path.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            label: None,
        }
    }

    /// Attach a label.
    #[must_use]
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

/// Context passed to collectors and criteria during evaluation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EvalContext {
    /// Run identifier (for correlation with runner-v2 state).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    /// Task identifier within the plan.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    /// Arbitrary key-value metadata.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub attrs: HashMap<String, String>,
}

impl EvalContext {
    /// Construct a new context.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the run id.
    #[must_use]
    pub fn with_run_id(mut self, id: impl Into<String>) -> Self {
        self.run_id = Some(id.into());
        self
    }

    /// Set the task id.
    #[must_use]
    pub fn with_task_id(mut self, id: impl Into<String>) -> Self {
        self.task_id = Some(id.into());
        self
    }

    /// Insert an attribute.
    #[must_use]
    pub fn with_attr(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attrs.insert(key.into(), value.into());
        self
    }
}

/// Evidence produced by an [`EvidenceCollector`].
///
/// Wraps the raw output from a subprocess or tool invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    /// The kind of evidence this represents.
    pub kind: EvidenceKind,
    /// Standard output captured from the process.
    pub stdout: String,
    /// Standard error captured from the process.
    pub stderr: String,
    /// Process exit code (0 = success by convention).
    pub exit_code: i32,
    /// Wall-clock duration in milliseconds.
    pub duration_ms: u64,
}

impl Evidence {
    /// Whether the process exited successfully (exit code 0).
    #[must_use]
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }
}

/// A typed bag of evidence keyed by [`EvidenceKind`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EvidenceBag {
    entries: HashMap<EvidenceKind, Evidence>,
}

impl EvidenceBag {
    /// Construct an empty evidence bag.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert evidence into the bag.
    pub fn insert(&mut self, evidence: Evidence) {
        self.entries.insert(evidence.kind, evidence);
    }

    /// Look up evidence by kind.
    #[must_use]
    pub fn get(&self, kind: &EvidenceKind) -> Option<&Evidence> {
        self.entries.get(kind)
    }

    /// Check whether evidence of a given kind is present.
    #[must_use]
    pub fn contains(&self, kind: &EvidenceKind) -> bool {
        self.entries.contains_key(kind)
    }

    /// Number of evidence entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the bag is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterate over all evidence entries.
    pub fn iter(&self) -> impl Iterator<Item = (&EvidenceKind, &Evidence)> {
        self.entries.iter()
    }
}

/// Severity of a finding produced by a criterion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// Informational finding (does not cause failure).
    Info,
    /// Warning (may cause failure depending on policy).
    Warning,
    /// Error (causes failure).
    Error,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Info => write!(f, "info"),
            Self::Warning => write!(f, "warning"),
            Self::Error => write!(f, "error"),
        }
    }
}

/// A structured finding produced by a [`Criterion`].
///
/// Findings carry source location and fix hints so downstream consumers
/// (agents, TUI, feedback loops) can act on them without parsing raw output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    /// Severity of this finding.
    pub severity: Severity,
    /// Human-readable description.
    pub message: String,
    /// Source file, if applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<PathBuf>,
    /// Line number in the source file, if applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    /// Machine-readable rule or error identifier (e.g., "E0599", "clippy::unwrap_used").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rule_id: Option<String>,
    /// Suggested fix or remediation hint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fix_hint: Option<String>,
}

impl Finding {
    /// Construct a new finding.
    #[must_use]
    pub fn new(severity: Severity, message: impl Into<String>) -> Self {
        Self {
            severity,
            message: message.into(),
            file: None,
            line: None,
            rule_id: None,
            fix_hint: None,
        }
    }

    /// Attach a source file.
    #[must_use]
    pub fn with_file(mut self, file: impl Into<PathBuf>) -> Self {
        self.file = Some(file.into());
        self
    }

    /// Attach a line number.
    #[must_use]
    pub fn with_line(mut self, line: u32) -> Self {
        self.line = Some(line);
        self
    }

    /// Attach a rule identifier.
    #[must_use]
    pub fn with_rule_id(mut self, id: impl Into<String>) -> Self {
        self.rule_id = Some(id.into());
        self
    }

    /// Attach a fix hint.
    #[must_use]
    pub fn with_fix_hint(mut self, hint: impl Into<String>) -> Self {
        self.fix_hint = Some(hint.into());
        self
    }
}

/// The result of evaluating one [`Criterion`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriterionResult {
    /// Whether the criterion passed.
    pub passed: bool,
    /// Numeric score in [0.0, 1.0].
    pub score: f64,
    /// Structured findings (errors, warnings, info).
    pub findings: Vec<Finding>,
    /// Wall-clock duration in milliseconds.
    pub duration_ms: u64,
}

impl CriterionResult {
    /// Construct a passing result with full score.
    #[must_use]
    pub fn pass() -> Self {
        Self {
            passed: true,
            score: 1.0,
            findings: Vec::new(),
            duration_ms: 0,
        }
    }

    /// Construct a failing result with zero score.
    #[must_use]
    pub fn fail() -> Self {
        Self {
            passed: false,
            score: 0.0,
            findings: Vec::new(),
            duration_ms: 0,
        }
    }

    /// Set the score.
    #[must_use]
    pub fn with_score(mut self, score: f64) -> Self {
        self.score = score.clamp(0.0, 1.0);
        self
    }

    /// Set the duration.
    #[must_use]
    pub fn with_duration_ms(mut self, ms: u64) -> Self {
        self.duration_ms = ms;
        self
    }

    /// Add a finding.
    #[must_use]
    pub fn with_finding(mut self, finding: Finding) -> Self {
        self.findings.push(finding);
        self
    }

    /// Add multiple findings.
    #[must_use]
    pub fn with_findings(mut self, findings: Vec<Finding>) -> Self {
        self.findings.extend(findings);
        self
    }
}

/// Composition strategy for combining multiple criteria results.
///
/// Mirrors the `GateComposition` variants in `roko-gate` so that profiles
/// can express the same composition semantics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum CompositionStrategy {
    /// Run criteria in order; short-circuit on first failure.
    #[default]
    Sequential,
    /// Run all criteria; pass if all pass.
    Parallel,
    /// Run all criteria; pass if at least `threshold` fraction pass.
    Voting {
        /// Fraction of criteria that must pass (0.0 to 1.0).
        threshold: f64,
    },
    /// Try criteria in order; use the first non-error result.
    Fallback,
}

/// The top-level result from running a [`Profile`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalVerdict {
    /// Name of the profile that was evaluated.
    pub profile_name: String,
    /// Whether the profile passed overall (according to its composition strategy).
    pub passed: bool,
    /// Overall score (composition of individual criterion scores).
    pub score: f64,
    /// Per-criterion results, in evaluation order.
    pub criteria_results: Vec<NamedCriterionResult>,
    /// Total wall-clock duration in milliseconds.
    pub total_duration_ms: u64,
}

/// A criterion result paired with the criterion name.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamedCriterionResult {
    /// Name of the criterion.
    pub criterion_name: String,
    /// The criterion's evaluation result.
    pub result: CriterionResult,
}

// ---------------------------------------------------------------------------
// Core traits
// ---------------------------------------------------------------------------

/// Produces typed evidence from artifacts.
///
/// Evidence collectors are responsible for running external tools (compilers,
/// linters, test runners) and capturing their output in a structured
/// [`Evidence`] value. They do NOT interpret the output -- that is the job of
/// [`Criterion`] implementations.
#[async_trait]
pub trait EvidenceCollector: Send + Sync {
    /// The kind of evidence this collector produces.
    fn kind(&self) -> EvidenceKind;

    /// Collect evidence from the given artifact.
    async fn collect(
        &self,
        artifact: &EvalArtifactRef,
        ctx: &EvalContext,
    ) -> Result<Evidence, CollectError>;
}

/// Scores one dimension of quality given an evidence bag.
///
/// Criteria examine evidence and produce a [`CriterionResult`] with a
/// pass/fail decision, a numeric score, and structured findings. They
/// should NOT spawn subprocesses directly -- use an [`EvidenceCollector`]
/// for that.
pub trait Criterion: Send + Sync {
    /// Human-readable name of this criterion.
    fn name(&self) -> &str;

    /// Which evidence kinds this criterion requires.
    ///
    /// The evaluation engine collects all required evidence before calling
    /// `evaluate`. An empty slice means the criterion is self-contained
    /// (e.g., a legacy bridge that spawns its own subprocess).
    fn required_evidence(&self) -> &[EvidenceKind];

    /// Evaluate the artifact against this criterion using the provided evidence.
    fn evaluate(
        &self,
        artifact: &EvalArtifactRef,
        evidence: &EvidenceBag,
        ctx: &EvalContext,
    ) -> CriterionResult;
}

// ---------------------------------------------------------------------------
// Profile
// ---------------------------------------------------------------------------

/// An ordered list of criteria with a composition strategy.
///
/// Profiles define _what_ to evaluate and _how_ to combine results. They are
/// the top-level entry point for running evaluations.
pub struct Profile {
    /// Human-readable name for this profile.
    pub name: String,
    /// Ordered criteria to evaluate.
    pub criteria: Vec<Box<dyn Criterion>>,
    /// How to combine criterion results into an overall verdict.
    pub strategy: CompositionStrategy,
}

impl fmt::Debug for Profile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Profile")
            .field("name", &self.name)
            .field(
                "criteria",
                &self.criteria.iter().map(|c| c.name()).collect::<Vec<_>>(),
            )
            .field("strategy", &self.strategy)
            .finish()
    }
}

impl Profile {
    /// Construct a new profile.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            criteria: Vec::new(),
            strategy: CompositionStrategy::default(),
        }
    }

    /// Add a criterion.
    #[must_use]
    pub fn with_criterion(mut self, criterion: Box<dyn Criterion>) -> Self {
        self.criteria.push(criterion);
        self
    }

    /// Set the composition strategy.
    #[must_use]
    pub fn with_strategy(mut self, strategy: CompositionStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Run all criteria against the artifact using the provided evidence bag.
    ///
    /// The caller is responsible for collecting evidence into the bag before
    /// calling this method. Missing evidence for a criterion that declares
    /// requirements is treated as a failure (not a panic).
    #[must_use]
    pub fn evaluate(
        &self,
        artifact: &EvalArtifactRef,
        evidence: &EvidenceBag,
        ctx: &EvalContext,
    ) -> EvalVerdict {
        let start = Instant::now();
        let mut criteria_results = Vec::with_capacity(self.criteria.len());

        for criterion in &self.criteria {
            // Check that all required evidence is present.
            let missing = criterion
                .required_evidence()
                .iter()
                .find(|kind| !evidence.contains(kind));

            let result = if let Some(missing_kind) = missing {
                CriterionResult::fail().with_finding(Finding::new(
                    Severity::Error,
                    format!(
                        "Missing required evidence {:?} for criterion {}",
                        missing_kind,
                        criterion.name()
                    ),
                ))
            } else {
                criterion.evaluate(artifact, evidence, ctx)
            };

            let named = NamedCriterionResult {
                criterion_name: criterion.name().to_string(),
                result,
            };

            // Short-circuit for sequential strategy on first failure.
            let failed = !named.result.passed;
            criteria_results.push(named);

            if failed && matches!(self.strategy, CompositionStrategy::Sequential) {
                break;
            }
        }

        let (passed, score) = compose_results(&self.strategy, &criteria_results);
        let elapsed = start.elapsed();

        EvalVerdict {
            profile_name: self.name.clone(),
            passed,
            score,
            criteria_results,
            total_duration_ms: elapsed.as_millis() as u64,
        }
    }
}

/// Combine criterion results according to a composition strategy.
fn compose_results(
    strategy: &CompositionStrategy,
    results: &[NamedCriterionResult],
) -> (bool, f64) {
    if results.is_empty() {
        return (true, 1.0);
    }

    match strategy {
        CompositionStrategy::Sequential | CompositionStrategy::Parallel => {
            let all_passed = results.iter().all(|r| r.result.passed);
            let avg_score =
                results.iter().map(|r| r.result.score).sum::<f64>() / results.len() as f64;
            (all_passed, avg_score)
        }
        CompositionStrategy::Voting { threshold } => {
            let pass_count = results.iter().filter(|r| r.result.passed).count();
            let pass_fraction = pass_count as f64 / results.len() as f64;
            let passed = pass_fraction >= *threshold;
            (passed, pass_fraction)
        }
        CompositionStrategy::Fallback => {
            // Use the first result that passed, or the last result if none passed.
            let first_pass = results.iter().find(|r| r.result.passed);
            match first_pass {
                Some(r) => (true, r.result.score),
                None => {
                    let last = results.last().expect("non-empty results");
                    (false, last.result.score)
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Built-in collectors
// ---------------------------------------------------------------------------

/// Evidence collector that spawns a subprocess and captures its output.
///
/// This is the workhorse collector used by compile, clippy, test, and fmt
/// criteria. It runs an arbitrary command and wraps the result as [`Evidence`].
pub struct ProcessCollector {
    /// The evidence kind this collector produces.
    kind: EvidenceKind,
    /// Program to execute.
    program: String,
    /// Arguments to pass to the program.
    args: Vec<String>,
    /// Timeout in milliseconds (0 = no timeout).
    timeout_ms: u64,
}

impl ProcessCollector {
    /// Construct a new process collector.
    #[must_use]
    pub fn new(kind: EvidenceKind, program: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            kind,
            program: program.into(),
            args,
            timeout_ms: 0,
        }
    }

    /// Set a timeout in milliseconds.
    #[must_use]
    pub fn with_timeout_ms(mut self, ms: u64) -> Self {
        self.timeout_ms = ms;
        self
    }

    /// Construct a collector for `cargo check`.
    #[must_use]
    pub fn cargo_check() -> Self {
        Self::new(
            EvidenceKind::Compile,
            "cargo",
            vec!["check".into(), "--workspace".into()],
        )
        .with_timeout_ms(roko_core::config::timeouts::DEFAULT_AGENT_TIMEOUT_MS)
    }

    /// Construct a collector for `cargo clippy`.
    #[must_use]
    pub fn cargo_clippy() -> Self {
        Self::new(
            EvidenceKind::Clippy,
            "cargo",
            vec![
                "clippy".into(),
                "--workspace".into(),
                "--no-deps".into(),
                "--".into(),
                "-D".into(),
                "warnings".into(),
            ],
        )
        .with_timeout_ms(roko_core::config::timeouts::DEFAULT_AGENT_TIMEOUT_MS)
    }

    /// Construct a collector for `cargo test`.
    #[must_use]
    pub fn cargo_test() -> Self {
        Self::new(
            EvidenceKind::Test,
            "cargo",
            vec!["test".into(), "--workspace".into()],
        )
        .with_timeout_ms(300_000)
    }

    /// Construct a collector for `cargo fmt --check`.
    #[must_use]
    pub fn cargo_fmt_check() -> Self {
        Self::new(
            EvidenceKind::Format,
            "cargo",
            vec!["fmt".into(), "--check".into()],
        )
        .with_timeout_ms(30_000)
    }
}

impl fmt::Debug for ProcessCollector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProcessCollector")
            .field("kind", &self.kind)
            .field("program", &self.program)
            .field("args", &self.args)
            .field("timeout_ms", &self.timeout_ms)
            .finish()
    }
}

#[async_trait]
impl EvidenceCollector for ProcessCollector {
    fn kind(&self) -> EvidenceKind {
        self.kind
    }

    async fn collect(
        &self,
        artifact: &EvalArtifactRef,
        _ctx: &EvalContext,
    ) -> Result<Evidence, CollectError> {
        use std::process::Stdio;

        let start = Instant::now();

        let mut cmd = tokio::process::Command::new(&self.program);
        cmd.args(&self.args)
            .current_dir(&artifact.path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        let child = cmd
            .spawn()
            .map_err(|e| CollectError::SpawnFailed(format!("{}: {e}", self.program)))?;

        let output = if self.timeout_ms > 0 {
            let timeout = std::time::Duration::from_millis(self.timeout_ms);
            match tokio::time::timeout(timeout, child.wait_with_output()).await {
                Ok(result) => result?,
                Err(_) => return Err(CollectError::Timeout(self.timeout_ms)),
            }
        } else {
            child.wait_with_output().await?
        };

        let elapsed = start.elapsed();

        Ok(Evidence {
            kind: self.kind,
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            exit_code: output.status.code().unwrap_or(-1),
            duration_ms: elapsed.as_millis() as u64,
        })
    }
}

/// Evidence collector that runs `git diff --stat`.
pub struct DiffCollector {
    /// Extra args for `git diff` (e.g., `["--cached"]`, `["HEAD~1"]`).
    extra_args: Vec<String>,
}

impl DiffCollector {
    /// Construct a diff collector with `git diff --stat`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            extra_args: Vec::new(),
        }
    }

    /// Add extra arguments to the `git diff` command.
    #[must_use]
    pub fn with_extra_args(mut self, args: Vec<String>) -> Self {
        self.extra_args = args;
        self
    }
}

impl Default for DiffCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for DiffCollector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DiffCollector")
            .field("extra_args", &self.extra_args)
            .finish()
    }
}

#[async_trait]
impl EvidenceCollector for DiffCollector {
    fn kind(&self) -> EvidenceKind {
        EvidenceKind::Diff
    }

    async fn collect(
        &self,
        artifact: &EvalArtifactRef,
        _ctx: &EvalContext,
    ) -> Result<Evidence, CollectError> {
        use std::process::Stdio;

        let start = Instant::now();

        let mut args = vec!["diff".to_string(), "--stat".to_string()];
        args.extend(self.extra_args.clone());

        let child = tokio::process::Command::new("git")
            .args(&args)
            .current_dir(&artifact.path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| CollectError::SpawnFailed(format!("git: {e}")))?;

        let output = child.wait_with_output().await?;
        let elapsed = start.elapsed();

        Ok(Evidence {
            kind: EvidenceKind::Diff,
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            exit_code: output.status.code().unwrap_or(-1),
            duration_ms: elapsed.as_millis() as u64,
        })
    }
}

// ---------------------------------------------------------------------------
// Bridge adapters
// ---------------------------------------------------------------------------

/// Wraps a boxed [`Verify`](roko_core::Verify) implementation as a
/// [`Criterion`].
///
/// This is the backward-compatibility bridge that allows the existing gate
/// pipeline to be used through the evidence-bag-based evaluation framework
/// without changes. The wrapped gate spawns its own subprocess, so
/// `required_evidence()` returns an empty slice.
///
/// For the generic (non-boxed) bridge to the opaque evaluation API, see
/// [`LegacyCriterion`].
pub struct BoxedLegacyCriterion {
    /// The wrapped `Verify` implementation.
    gate: Box<dyn roko_core::Verify>,
    /// Human-readable name for this criterion.
    criterion_name: String,
}

impl BoxedLegacyCriterion {
    /// Wrap a `Verify` implementation as a `Criterion`.
    pub fn new(name: impl Into<String>, gate: Box<dyn roko_core::Verify>) -> Self {
        Self {
            gate,
            criterion_name: name.into(),
        }
    }
}

impl fmt::Debug for BoxedLegacyCriterion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BoxedLegacyCriterion")
            .field("name", &self.criterion_name)
            .finish()
    }
}

impl Criterion for BoxedLegacyCriterion {
    fn name(&self) -> &str {
        &self.criterion_name
    }

    fn required_evidence(&self) -> &[EvidenceKind] {
        // Legacy gates spawn their own subprocesses; no external evidence needed.
        &[]
    }

    fn evaluate(
        &self,
        artifact: &EvalArtifactRef,
        _evidence: &EvidenceBag,
        _ctx: &EvalContext,
    ) -> CriterionResult {
        // Build a minimal Signal and Context for the wrapped gate.
        let payload = serde_json::json!({
            "workdir": artifact.path.to_string_lossy(),
        });
        let signal = roko_core::Signal::builder(roko_core::Kind::Task)
            .body(roko_core::Body::from_json(&payload).unwrap_or_else(|_| roko_core::Body::empty()))
            .build();
        let ctx = roko_core::Context::now().with_attr("workdir", artifact.path.to_string_lossy());

        // Run the gate synchronously using a blocking bridge.
        // This is intentional: `Criterion::evaluate` is synchronous because
        // most criteria are pure functions over evidence. Legacy gates that
        // spawn subprocesses are the exception, and this bridge handles them.
        let start = Instant::now();

        let verdict = std::thread::scope(|_s| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to build tokio runtime for legacy gate bridge");
            rt.block_on(self.gate.verify(&signal, &ctx))
        });

        let elapsed = start.elapsed();

        let mut result = if verdict.passed {
            CriterionResult::pass()
        } else {
            CriterionResult::fail()
        };

        result.score = f64::from(verdict.score);
        result.duration_ms = elapsed.as_millis() as u64;

        // Convert verdict detail/reason to a finding.
        if !verdict.passed {
            let message = verdict
                .error_digest
                .as_ref()
                .filter(|d| !d.is_empty())
                .or(verdict.detail.as_ref().filter(|d| !d.is_empty()))
                .unwrap_or(&verdict.reason);

            result.findings.push(Finding::new(Severity::Error, message));
        }

        result
    }
}

// ---------------------------------------------------------------------------
// Convenience re-exports
// ---------------------------------------------------------------------------

/// Convert a [`CriterionResult`] to a [`Verdict`](roko_core::Verdict) for
/// backward compatibility with callers that expect the core type.
#[must_use]
pub fn criterion_result_to_verdict(
    gate_name: &str,
    result: &CriterionResult,
) -> roko_core::Verdict {
    let mut verdict = if result.passed {
        roko_core::Verdict::pass(gate_name)
    } else {
        let reason = result
            .findings
            .first()
            .map(|f| f.message.clone())
            .unwrap_or_else(|| "criterion failed".to_string());
        roko_core::Verdict::fail(gate_name, reason)
    };

    verdict.score = result.score as f32;
    verdict.duration_ms = result.duration_ms;

    // Aggregate error findings into the error_digest field.
    let errors: Vec<&Finding> = result
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .collect();
    if !errors.is_empty() {
        let digest = errors
            .iter()
            .map(|f| {
                let mut line = f.message.clone();
                if let Some(file) = &f.file {
                    line = format!("{}:{}", file.display(), f.line.unwrap_or(0));
                    if !f.message.is_empty() {
                        line = format!("{line}: {}", f.message);
                    }
                }
                line
            })
            .collect::<Vec<_>>()
            .join("\n");
        verdict.error_digest = Some(digest);
    }

    verdict
}

/// Convert a [`Verdict`](roko_core::Verdict) to a [`CriterionResult`].
#[must_use]
pub fn verdict_to_criterion_result(verdict: &roko_core::Verdict) -> CriterionResult {
    let mut result = if verdict.passed {
        CriterionResult::pass()
    } else {
        CriterionResult::fail()
    };

    result.score = f64::from(verdict.score);
    result.duration_ms = verdict.duration_ms;

    if !verdict.passed {
        let message = verdict
            .error_digest
            .as_ref()
            .filter(|d| !d.is_empty())
            .or(verdict.detail.as_ref().filter(|d| !d.is_empty()))
            .unwrap_or(&verdict.reason);

        result.findings.push(Finding::new(Severity::Error, message));
    }

    result
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    // -- ProcessCollector tests --

    #[tokio::test]
    async fn process_collector_captures_stdout_and_exit_code() {
        let collector =
            ProcessCollector::new(EvidenceKind::Shell, "echo", vec!["hello world".into()]);

        let artifact = EvalArtifactRef::new(".");
        let ctx = EvalContext::new();
        let evidence = collector
            .collect(&artifact, &ctx)
            .await
            .expect("echo should succeed");

        assert_eq!(evidence.kind, EvidenceKind::Shell);
        assert_eq!(evidence.exit_code, 0);
        assert!(evidence.stdout.contains("hello world"));
        assert!(evidence.success());
    }

    #[tokio::test]
    async fn process_collector_captures_stderr_and_nonzero_exit() {
        let collector = ProcessCollector::new(
            EvidenceKind::Shell,
            "sh",
            vec!["-c".into(), "echo err >&2; exit 42".into()],
        );

        let artifact = EvalArtifactRef::new(".");
        let ctx = EvalContext::new();
        let evidence = collector
            .collect(&artifact, &ctx)
            .await
            .expect("command should complete");

        assert_eq!(evidence.exit_code, 42);
        assert!(evidence.stderr.contains("err"));
        assert!(!evidence.success());
    }

    #[tokio::test]
    async fn process_collector_returns_spawn_error_for_missing_program() {
        let collector =
            ProcessCollector::new(EvidenceKind::Shell, "nonexistent-program-xyz-12345", vec![]);

        let artifact = EvalArtifactRef::new(".");
        let ctx = EvalContext::new();
        let err = collector
            .collect(&artifact, &ctx)
            .await
            .expect_err("should fail to spawn");

        assert!(matches!(err, CollectError::SpawnFailed(_)));
    }

    // -- DiffCollector tests --

    #[tokio::test]
    async fn diff_collector_runs_in_a_git_repo() {
        // This test only works when run inside a git repository.
        let collector = DiffCollector::new();
        let artifact = EvalArtifactRef::new(env!("CARGO_MANIFEST_DIR"));
        let ctx = EvalContext::new();

        // The collector should succeed (even if there are no diffs).
        let result = collector.collect(&artifact, &ctx).await;
        // If we're not in a git repo, the collector will fail with a spawn error
        // or a nonzero exit code -- either way, it should not panic.
        if let Ok(evidence) = result {
            assert_eq!(evidence.kind, EvidenceKind::Diff);
        }
    }

    // -- EvidenceBag tests --

    #[test]
    fn evidence_bag_insert_and_lookup() {
        let mut bag = EvidenceBag::new();
        assert!(bag.is_empty());

        bag.insert(Evidence {
            kind: EvidenceKind::Compile,
            stdout: "ok".into(),
            stderr: String::new(),
            exit_code: 0,
            duration_ms: 100,
        });

        assert_eq!(bag.len(), 1);
        assert!(bag.contains(&EvidenceKind::Compile));
        assert!(!bag.contains(&EvidenceKind::Test));

        let ev = bag.get(&EvidenceKind::Compile).expect("should be present");
        assert_eq!(ev.stdout, "ok");
    }

    // -- CriterionResult tests --

    #[test]
    fn criterion_result_builders() {
        let result = CriterionResult::pass()
            .with_score(0.95)
            .with_duration_ms(42)
            .with_finding(Finding::new(Severity::Info, "all good"));

        assert!(result.passed);
        assert!((result.score - 0.95).abs() < f64::EPSILON);
        assert_eq!(result.duration_ms, 42);
        assert_eq!(result.findings.len(), 1);
    }

    #[test]
    fn criterion_result_fail_has_zero_score() {
        let result = CriterionResult::fail();
        assert!(!result.passed);
        assert!((result.score - 0.0).abs() < f64::EPSILON);
    }

    // -- Finding tests --

    #[test]
    fn finding_builder_chain() {
        let finding = Finding::new(Severity::Error, "undefined symbol")
            .with_file("src/main.rs")
            .with_line(42)
            .with_rule_id("E0599")
            .with_fix_hint("did you mean `foo()`?");

        assert_eq!(finding.severity, Severity::Error);
        assert_eq!(finding.file.as_deref(), Some(Path::new("src/main.rs")));
        assert_eq!(finding.line, Some(42));
        assert_eq!(finding.rule_id.as_deref(), Some("E0599"));
        assert_eq!(finding.fix_hint.as_deref(), Some("did you mean `foo()`?"));
    }

    // -- Profile + CompositionStrategy tests --

    /// A test criterion that always passes.
    struct AlwaysPass;

    impl Criterion for AlwaysPass {
        fn name(&self) -> &str {
            "always_pass"
        }
        fn required_evidence(&self) -> &[EvidenceKind] {
            &[]
        }
        fn evaluate(
            &self,
            _artifact: &EvalArtifactRef,
            _evidence: &EvidenceBag,
            _ctx: &EvalContext,
        ) -> CriterionResult {
            CriterionResult::pass()
        }
    }

    /// A test criterion that always fails.
    struct AlwaysFail;

    impl Criterion for AlwaysFail {
        fn name(&self) -> &str {
            "always_fail"
        }
        fn required_evidence(&self) -> &[EvidenceKind] {
            &[]
        }
        fn evaluate(
            &self,
            _artifact: &EvalArtifactRef,
            _evidence: &EvidenceBag,
            _ctx: &EvalContext,
        ) -> CriterionResult {
            CriterionResult::fail().with_finding(Finding::new(Severity::Error, "always fails"))
        }
    }

    #[test]
    fn profile_sequential_passes_when_all_pass() {
        let profile = Profile::new("test-profile")
            .with_criterion(Box::new(AlwaysPass))
            .with_criterion(Box::new(AlwaysPass));

        let artifact = EvalArtifactRef::new(".");
        let evidence = EvidenceBag::new();
        let ctx = EvalContext::new();

        let verdict = profile.evaluate(&artifact, &evidence, &ctx);
        assert!(verdict.passed);
        assert_eq!(verdict.criteria_results.len(), 2);
    }

    #[test]
    fn profile_sequential_short_circuits_on_failure() {
        let profile = Profile::new("test-profile")
            .with_criterion(Box::new(AlwaysFail))
            .with_criterion(Box::new(AlwaysPass));

        let artifact = EvalArtifactRef::new(".");
        let evidence = EvidenceBag::new();
        let ctx = EvalContext::new();

        let verdict = profile.evaluate(&artifact, &evidence, &ctx);
        assert!(!verdict.passed);
        // Sequential strategy short-circuits: only the failing criterion runs.
        assert_eq!(verdict.criteria_results.len(), 1);
    }

    #[test]
    fn profile_voting_passes_at_threshold() {
        let profile = Profile::new("voting")
            .with_criterion(Box::new(AlwaysPass))
            .with_criterion(Box::new(AlwaysPass))
            .with_criterion(Box::new(AlwaysFail))
            .with_strategy(CompositionStrategy::Voting { threshold: 0.5 });

        let artifact = EvalArtifactRef::new(".");
        let evidence = EvidenceBag::new();
        let ctx = EvalContext::new();

        let verdict = profile.evaluate(&artifact, &evidence, &ctx);
        // 2/3 = 0.667 >= 0.5 threshold
        assert!(verdict.passed);
        assert_eq!(verdict.criteria_results.len(), 3);
    }

    #[test]
    fn profile_voting_fails_below_threshold() {
        let profile = Profile::new("voting")
            .with_criterion(Box::new(AlwaysPass))
            .with_criterion(Box::new(AlwaysFail))
            .with_criterion(Box::new(AlwaysFail))
            .with_strategy(CompositionStrategy::Voting { threshold: 0.5 });

        let artifact = EvalArtifactRef::new(".");
        let evidence = EvidenceBag::new();
        let ctx = EvalContext::new();

        let verdict = profile.evaluate(&artifact, &evidence, &ctx);
        // 1/3 = 0.333 < 0.5 threshold
        assert!(!verdict.passed);
    }

    #[test]
    fn profile_fallback_uses_first_pass() {
        let profile = Profile::new("fallback")
            .with_criterion(Box::new(AlwaysFail))
            .with_criterion(Box::new(AlwaysPass))
            .with_strategy(CompositionStrategy::Fallback);

        let artifact = EvalArtifactRef::new(".");
        let evidence = EvidenceBag::new();
        let ctx = EvalContext::new();

        let verdict = profile.evaluate(&artifact, &evidence, &ctx);
        assert!(verdict.passed);
    }

    #[test]
    fn profile_reports_missing_evidence_as_failure() {
        struct NeedsCompile;
        impl Criterion for NeedsCompile {
            fn name(&self) -> &str {
                "needs_compile"
            }
            fn required_evidence(&self) -> &[EvidenceKind] {
                &[EvidenceKind::Compile]
            }
            fn evaluate(
                &self,
                _artifact: &EvalArtifactRef,
                _evidence: &EvidenceBag,
                _ctx: &EvalContext,
            ) -> CriterionResult {
                CriterionResult::pass()
            }
        }

        let profile = Profile::new("missing-evidence").with_criterion(Box::new(NeedsCompile));

        let artifact = EvalArtifactRef::new(".");
        let evidence = EvidenceBag::new(); // empty -- no compile evidence
        let ctx = EvalContext::new();

        let verdict = profile.evaluate(&artifact, &evidence, &ctx);
        assert!(!verdict.passed);
        assert_eq!(verdict.criteria_results.len(), 1);
        assert!(
            verdict.criteria_results[0]
                .result
                .findings
                .iter()
                .any(|f| f.message.contains("Missing required evidence"))
        );
    }

    // -- BoxedLegacyCriterion tests --

    /// A mock Verify implementation for testing the BoxedLegacyCriterion bridge.
    struct MockVerify {
        pass: bool,
    }

    impl roko_core::Cell for MockVerify {
        fn cell_id(&self) -> &str {
            "mock-verify"
        }
        fn cell_name(&self) -> &str {
            "MockVerify"
        }
        fn protocols(&self) -> Vec<roko_core::ProtocolId> {
            vec![roko_core::ProtocolId::Verify]
        }
    }

    #[async_trait]
    impl roko_core::Verify for MockVerify {
        async fn verify(
            &self,
            _signal: &roko_core::Signal,
            _ctx: &roko_core::Context,
        ) -> roko_core::Verdict {
            if self.pass {
                roko_core::Verdict::pass("mock")
            } else {
                roko_core::Verdict::fail("mock", "mock failure reason")
            }
        }

        fn name(&self) -> &str {
            "mock-verify"
        }
    }

    #[test]
    fn legacy_criterion_wraps_passing_verify() {
        let criterion = BoxedLegacyCriterion::new("mock-pass", Box::new(MockVerify { pass: true }));

        assert_eq!(criterion.name(), "mock-pass");
        assert!(criterion.required_evidence().is_empty());

        let artifact = EvalArtifactRef::new(".");
        let evidence = EvidenceBag::new();
        let ctx = EvalContext::new();

        let result = criterion.evaluate(&artifact, &evidence, &ctx);
        assert!(result.passed);
        assert!((result.score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn legacy_criterion_wraps_failing_verify() {
        let criterion =
            BoxedLegacyCriterion::new("mock-fail", Box::new(MockVerify { pass: false }));

        let artifact = EvalArtifactRef::new(".");
        let evidence = EvidenceBag::new();
        let ctx = EvalContext::new();

        let result = criterion.evaluate(&artifact, &evidence, &ctx);
        assert!(!result.passed);
        assert!((result.score - 0.0).abs() < f64::EPSILON);
        assert!(!result.findings.is_empty());
        assert!(result.findings[0].message.contains("mock failure reason"));
    }

    // -- Conversion function tests --

    #[test]
    fn criterion_result_round_trips_through_verdict() {
        let result = CriterionResult::fail()
            .with_score(0.3)
            .with_duration_ms(150)
            .with_finding(
                Finding::new(Severity::Error, "undefined symbol")
                    .with_file("src/lib.rs")
                    .with_line(10),
            );

        let verdict = criterion_result_to_verdict("compile", &result);
        assert!(!verdict.passed);
        assert!((verdict.score - 0.3).abs() < f32::EPSILON);
        assert_eq!(verdict.duration_ms, 150);
        assert!(verdict.error_digest.is_some());

        let back = verdict_to_criterion_result(&verdict);
        assert!(!back.passed);
        assert_eq!(back.duration_ms, 150);
    }

    // -- Serialization tests --

    #[test]
    fn evidence_kind_serializes_as_snake_case() {
        assert_eq!(
            serde_json::to_value(EvidenceKind::Compile).unwrap(),
            "compile"
        );
        assert_eq!(
            serde_json::to_value(EvidenceKind::SecurityScan).unwrap(),
            "security_scan"
        );
    }

    #[test]
    fn criterion_result_serialization_round_trip() {
        let result = CriterionResult::fail()
            .with_score(0.5)
            .with_duration_ms(200)
            .with_finding(
                Finding::new(Severity::Warning, "unused import")
                    .with_file("src/lib.rs")
                    .with_line(3)
                    .with_rule_id("unused_imports"),
            );

        let json = serde_json::to_string(&result).expect("serialize");
        let deserialized: CriterionResult = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(deserialized.passed, result.passed);
        assert!((deserialized.score - result.score).abs() < f64::EPSILON);
        assert_eq!(deserialized.findings.len(), 1);
    }

    #[test]
    fn eval_verdict_serialization_round_trip() {
        let verdict = EvalVerdict {
            profile_name: "test".to_string(),
            passed: true,
            score: 0.9,
            criteria_results: vec![NamedCriterionResult {
                criterion_name: "compile".to_string(),
                result: CriterionResult::pass(),
            }],
            total_duration_ms: 500,
        };

        let json = serde_json::to_string(&verdict).expect("serialize");
        let deserialized: EvalVerdict = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(deserialized.profile_name, "test");
        assert!(deserialized.passed);
        assert_eq!(deserialized.criteria_results.len(), 1);
    }

    #[test]
    fn composition_strategy_serialization() {
        let strategies = vec![
            CompositionStrategy::Sequential,
            CompositionStrategy::Parallel,
            CompositionStrategy::Voting { threshold: 0.75 },
            CompositionStrategy::Fallback,
        ];

        for strategy in strategies {
            let json = serde_json::to_string(&strategy).expect("serialize strategy");
            let deserialized: CompositionStrategy =
                serde_json::from_str(&json).expect("deserialize strategy");
            // Verify the round-trip preserves the variant structure.
            let re_json = serde_json::to_string(&deserialized).expect("re-serialize");
            assert_eq!(json, re_json);
        }
    }

    // -- EvalContext tests --

    #[test]
    fn eval_context_builder() {
        let ctx = EvalContext::new()
            .with_run_id("run-1")
            .with_task_id("task-1")
            .with_attr("key", "value");

        assert_eq!(ctx.run_id.as_deref(), Some("run-1"));
        assert_eq!(ctx.task_id.as_deref(), Some("task-1"));
        assert_eq!(ctx.attrs.get("key").map(String::as_str), Some("value"));
    }

    // -- EvalArtifactRef tests --

    #[test]
    fn eval_artifact_ref_builder() {
        let artifact = EvalArtifactRef::new("/workspace").with_label("my-plan");

        assert_eq!(artifact.path, PathBuf::from("/workspace"));
        assert_eq!(artifact.label.as_deref(), Some("my-plan"));
    }

    // -- Empty profile tests --

    #[test]
    fn empty_profile_passes() {
        let profile = Profile::new("empty");
        let artifact = EvalArtifactRef::new(".");
        let evidence = EvidenceBag::new();
        let ctx = EvalContext::new();

        let verdict = profile.evaluate(&artifact, &evidence, &ctx);
        assert!(verdict.passed);
        assert!((verdict.score - 1.0).abs() < f64::EPSILON);
    }
}

// ---------------------------------------------------------------------------
// Opaque evaluation API (spec-defined types)
//
// This module contains the type-erased evaluation API specified for roko-eval.
// It is additive: all existing types above remain intact. The types here use
// `OpaqueEvidence` — a type-erased container — so that evidence collectors and
// criteria can be composed without knowing each other's concrete evidence types.
//
// Primary types:
//   - `OpaqueEvidence`         — type-erased evidence container
//   - `Criterion` (trait)      — async, opaque-evidence evaluator
//   - `CriterionResult`        — per-criterion outcome (with details + metadata)
//   - `EvidenceCollector`      — async collector producing `OpaqueEvidence`
//   - `TaskEvalContext`        — task-scoped evaluation context
//   - `Profile`                — named collection of criteria
//   - `ProfileResult`          — aggregated profile outcome
//   - `LegacyCriterion<G>`     — generic bridge to `roko_core::Verify` gates
// ---------------------------------------------------------------------------

/// Type-erased evidence container.
///
/// Wraps any `Send + Sync + 'static` value so that evidence collectors and
/// criteria can be composed without knowing each other's concrete types. Use
/// [`OpaqueEvidence::downcast_ref`] to recover the inner value.
pub struct OpaqueEvidence(Box<dyn std::any::Any + Send + Sync>);

impl OpaqueEvidence {
    /// Wrap a value as opaque evidence.
    pub fn new<T: std::any::Any + Send + Sync>(value: T) -> Self {
        Self(Box::new(value))
    }

    /// Attempt to downcast to a concrete reference.
    ///
    /// Returns `None` if the inner type does not match `T`.
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.0.downcast_ref::<T>()
    }
}

impl std::fmt::Debug for OpaqueEvidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("OpaqueEvidence").field(&"<dyn Any>").finish()
    }
}

/// Per-criterion result produced by an opaque [`Criterion`] evaluation.
///
/// Distinct from the evidence-bag-based [`CriterionResult`] above: this variant
/// uses `details: String` and `metadata: HashMap<String, serde_json::Value>`
/// to carry richer structured output to agents and dashboards.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OpaqueCriterionResult {
    /// Whether the criterion passed.
    pub passed: bool,
    /// Numeric quality score in `[0.0, 1.0]`.
    pub score: f64,
    /// Human-readable explanation of the result.
    pub details: String,
    /// Arbitrary structured metadata (e.g., line counts, error codes).
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

impl OpaqueCriterionResult {
    /// Construct a passing result with full score and empty details.
    #[must_use]
    pub fn pass() -> Self {
        Self {
            passed: true,
            score: 1.0,
            details: String::new(),
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Construct a failing result with zero score and empty details.
    #[must_use]
    pub fn fail() -> Self {
        Self {
            passed: false,
            score: 0.0,
            details: String::new(),
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Attach a details message.
    #[must_use]
    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = details.into();
        self
    }

    /// Set the score (clamped to `[0.0, 1.0]`).
    #[must_use]
    pub fn with_score(mut self, score: f64) -> Self {
        self.score = score.clamp(0.0, 1.0);
        self
    }

    /// Insert a metadata entry.
    #[must_use]
    pub fn with_meta(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// Task-scoped evaluation context.
///
/// Passed to [`OpaqueEvidenceCollector::collect`] and used by callers to
/// communicate which plan/task is being evaluated and from which directory.
///
/// This is distinct from the process-level [`EvalContext`] above (which
/// carries `run_id`/`task_id`/`attrs`); `TaskEvalContext` uses the field
/// names specified by the eval-framework spec.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct TaskEvalContext {
    /// Working directory for the evaluation (workspace root or worktree).
    pub workdir: std::path::PathBuf,
    /// Plan identifier (correlates to a `tasks.toml` plan directory).
    pub plan_id: String,
    /// Task identifier within the plan.
    pub task_id: String,
    /// Arbitrary key-value metadata for the evaluation run.
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

impl TaskEvalContext {
    /// Construct a minimal context with workdir, plan_id, and task_id.
    #[must_use]
    pub fn new(
        workdir: impl Into<std::path::PathBuf>,
        plan_id: impl Into<String>,
        task_id: impl Into<String>,
    ) -> Self {
        Self {
            workdir: workdir.into(),
            plan_id: plan_id.into(),
            task_id: task_id.into(),
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Insert a metadata entry.
    #[must_use]
    pub fn with_meta(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// A single evaluation criterion operating on opaque evidence.
///
/// Implementors score one quality dimension (compilation, test coverage,
/// lint cleanliness, …) by inspecting the concrete type hidden inside
/// [`OpaqueEvidence`]. Criteria **must not** spawn subprocesses themselves —
/// use an [`OpaqueEvidenceCollector`] for that.
#[async_trait::async_trait]
pub trait OpaqueCriterion: Send + Sync {
    /// Human-readable name of this criterion.
    fn name(&self) -> &str;

    /// Human-readable description of what this criterion checks.
    fn description(&self) -> &str;

    /// Evaluate the artifact given opaque evidence.
    ///
    /// # Errors
    ///
    /// Returns an error if the evidence cannot be downcast to the expected
    /// concrete type, or if evaluation itself fails.
    async fn evaluate(&self, evidence: &OpaqueEvidence) -> anyhow::Result<OpaqueCriterionResult>;
}

/// Gathers opaque evidence for subsequent criterion evaluation.
///
/// Collectors run external tools, read files, or query APIs and package the
/// result as [`OpaqueEvidence`]. They are intentionally separate from criteria
/// so evidence can be reused across multiple criteria without re-running tools.
#[async_trait::async_trait]
pub trait OpaqueEvidenceCollector: Send + Sync {
    /// Human-readable name of this collector.
    fn name(&self) -> &str;

    /// Collect evidence from the context's working directory.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying tool fails to run or its output
    /// cannot be captured.
    async fn collect(&self, context: &TaskEvalContext) -> anyhow::Result<OpaqueEvidence>;
}

/// A named collection of opaque criteria with a mean-score aggregation policy.
///
/// `OpaqueProfile` runs all criteria against a shared [`OpaqueEvidence`] bag,
/// collects per-criterion results, and produces a [`ProfileResult`].
pub struct OpaqueProfile {
    /// Human-readable name for this profile.
    pub name: String,
    /// Ordered criteria to evaluate.
    pub criteria: Vec<Box<dyn OpaqueCriterion>>,
}

impl std::fmt::Debug for OpaqueProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpaqueProfile")
            .field("name", &self.name)
            .field(
                "criteria",
                &self.criteria.iter().map(|c| c.name()).collect::<Vec<_>>(),
            )
            .finish()
    }
}

impl OpaqueProfile {
    /// Construct an empty profile.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            criteria: Vec::new(),
        }
    }

    /// Add a criterion.
    #[must_use]
    pub fn with_criterion(mut self, criterion: Box<dyn OpaqueCriterion>) -> Self {
        self.criteria.push(criterion);
        self
    }

    /// Evaluate all criteria against the given evidence.
    ///
    /// All criteria run regardless of individual pass/fail status. The overall
    /// profile passes only when every criterion passes. The aggregate score is
    /// the arithmetic mean of all criterion scores.
    ///
    /// Criteria that return errors are treated as failures with a score of
    /// `0.0` and an error message captured in `details`.
    pub async fn evaluate_all(&self, evidence: &OpaqueEvidence) -> ProfileResult {
        let mut results: Vec<(String, OpaqueCriterionResult)> =
            Vec::with_capacity(self.criteria.len());

        for criterion in &self.criteria {
            let criterion_result = match criterion.evaluate(evidence).await {
                Ok(r) => r,
                Err(e) => {
                    OpaqueCriterionResult::fail().with_details(format!("criterion error: {e}"))
                }
            };
            results.push((criterion.name().to_string(), criterion_result));
        }

        let passed = results.iter().all(|(_, r)| r.passed);
        let score = if results.is_empty() {
            1.0
        } else {
            results.iter().map(|(_, r)| r.score).sum::<f64>() / results.len() as f64
        };

        ProfileResult {
            profile_name: self.name.clone(),
            passed,
            score,
            results,
        }
    }
}

/// Aggregated result from running an [`OpaqueProfile`].
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProfileResult {
    /// Name of the profile that was evaluated.
    pub profile_name: String,
    /// Whether every criterion in the profile passed.
    pub passed: bool,
    /// Arithmetic mean of all criterion scores (`0.0` to `1.0`).
    pub score: f64,
    /// Per-criterion results paired with criterion names, in evaluation order.
    pub results: Vec<(String, OpaqueCriterionResult)>,
}

/// Generic bridge that wraps any `G: roko_core::Verify` as an [`OpaqueCriterion`].
///
/// This allows existing gate implementations to be composed in the opaque
/// evaluation pipeline without modification. Evidence is ignored (the wrapped
/// gate spawns its own subprocess). The gate is called with a minimal synthetic
/// [`roko_core::Signal`] carrying the workdir from [`TaskEvalContext`].
pub struct LegacyCriterion<G> {
    /// The wrapped gate.
    gate: G,
    /// Human-readable criterion name.
    name: String,
}

impl<G> LegacyCriterion<G> {
    /// Wrap a gate as a criterion.
    pub fn new(name: impl Into<String>, gate: G) -> Self {
        Self {
            gate,
            name: name.into(),
        }
    }
}

impl<G: roko_core::Verify> std::fmt::Debug for LegacyCriterion<G> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LegacyCriterion")
            .field("name", &self.name)
            .finish()
    }
}

#[async_trait::async_trait]
impl<G: roko_core::Verify + Send + Sync> OpaqueCriterion for LegacyCriterion<G> {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &'static str {
        "Legacy Verify gate bridge"
    }

    async fn evaluate(&self, evidence: &OpaqueEvidence) -> anyhow::Result<OpaqueCriterionResult> {
        // Extract workdir from opaque evidence if the caller stored a
        // `TaskEvalContext` inside it; otherwise fall back to ".".
        let workdir = evidence
            .downcast_ref::<TaskEvalContext>()
            .map(|c| c.workdir.to_string_lossy().into_owned())
            .unwrap_or_else(|| ".".to_string());

        let payload = serde_json::json!({ "workdir": workdir });
        let signal = roko_core::Signal::builder(roko_core::Kind::Task)
            .body(roko_core::Body::from_json(&payload).unwrap_or_else(|_| roko_core::Body::empty()))
            .build();
        let ctx = roko_core::Context::now().with_attr("workdir", workdir);

        let verdict = self.gate.verify(&signal, &ctx).await;

        if verdict.passed {
            Ok(OpaqueCriterionResult::pass())
        } else {
            let details = verdict
                .error_digest
                .as_ref()
                .filter(|d| !d.is_empty())
                .or(verdict.detail.as_ref().filter(|d| !d.is_empty()))
                .unwrap_or(&verdict.reason)
                .clone();
            Ok(OpaqueCriterionResult::fail()
                .with_score(f64::from(verdict.score))
                .with_details(details))
        }
    }
}

// ---------------------------------------------------------------------------
// Tests for opaque API types
// ---------------------------------------------------------------------------

#[cfg(test)]
mod opaque_tests {
    use super::*;

    // -- OpaqueEvidence tests --

    #[test]
    fn opaque_evidence_downcast_succeeds_for_correct_type() {
        let ev = OpaqueEvidence::new(42u32);
        let val = ev.downcast_ref::<u32>();
        assert_eq!(val, Some(&42u32));
    }

    #[test]
    fn opaque_evidence_downcast_fails_for_wrong_type() {
        let ev = OpaqueEvidence::new(42u32);
        let val = ev.downcast_ref::<String>();
        assert!(val.is_none());
    }

    #[test]
    fn opaque_evidence_downcast_with_struct() {
        #[derive(Debug, PartialEq)]
        struct MyEvidence {
            value: i32,
        }

        let ev = OpaqueEvidence::new(MyEvidence { value: 99 });
        let inner = ev.downcast_ref::<MyEvidence>();
        assert_eq!(inner, Some(&MyEvidence { value: 99 }));
    }

    // -- OpaqueCriterionResult tests --

    #[test]
    fn opaque_criterion_result_pass_defaults() {
        let r = OpaqueCriterionResult::pass();
        assert!(r.passed);
        assert!((r.score - 1.0).abs() < f64::EPSILON);
        assert!(r.details.is_empty());
        assert!(r.metadata.is_empty());
    }

    #[test]
    fn opaque_criterion_result_fail_defaults() {
        let r = OpaqueCriterionResult::fail();
        assert!(!r.passed);
        assert!((r.score - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn opaque_criterion_result_builder_chain() {
        let r = OpaqueCriterionResult::fail()
            .with_score(0.75)
            .with_details("partial success")
            .with_meta("lines", serde_json::json!(100));

        assert!(!r.passed);
        assert!((r.score - 0.75).abs() < f64::EPSILON);
        assert_eq!(r.details, "partial success");
        assert_eq!(r.metadata.get("lines"), Some(&serde_json::json!(100)));
    }

    #[test]
    fn opaque_criterion_result_score_clamped() {
        let r = OpaqueCriterionResult::pass().with_score(1.5);
        assert!((r.score - 1.0).abs() < f64::EPSILON);

        let r2 = OpaqueCriterionResult::pass().with_score(-0.5);
        assert!((r2.score - 0.0).abs() < f64::EPSILON);
    }

    // -- TaskEvalContext tests --

    #[test]
    fn task_eval_context_builder() {
        let ctx = TaskEvalContext::new("/workspace", "plan-abc", "task-1")
            .with_meta("env", serde_json::json!("staging"));

        assert_eq!(ctx.workdir.to_str(), Some("/workspace"));
        assert_eq!(ctx.plan_id, "plan-abc");
        assert_eq!(ctx.task_id, "task-1");
        assert_eq!(ctx.metadata.get("env"), Some(&serde_json::json!("staging")));
    }

    // -- OpaqueProfile + ProfileResult tests --

    struct AlwaysPassCriterion;

    #[async_trait::async_trait]
    impl OpaqueCriterion for AlwaysPassCriterion {
        fn name(&self) -> &str {
            "always_pass"
        }
        fn description(&self) -> &str {
            "always passes"
        }
        async fn evaluate(
            &self,
            _evidence: &OpaqueEvidence,
        ) -> anyhow::Result<OpaqueCriterionResult> {
            Ok(OpaqueCriterionResult::pass().with_details("ok"))
        }
    }

    struct AlwaysFailCriterion;

    #[async_trait::async_trait]
    impl OpaqueCriterion for AlwaysFailCriterion {
        fn name(&self) -> &str {
            "always_fail"
        }
        fn description(&self) -> &str {
            "always fails"
        }
        async fn evaluate(
            &self,
            _evidence: &OpaqueEvidence,
        ) -> anyhow::Result<OpaqueCriterionResult> {
            Ok(OpaqueCriterionResult::fail().with_details("always fails"))
        }
    }

    #[tokio::test]
    async fn profile_evaluate_all_passes_when_all_criteria_pass() {
        let profile = OpaqueProfile::new("all-pass")
            .with_criterion(Box::new(AlwaysPassCriterion))
            .with_criterion(Box::new(AlwaysPassCriterion));

        let evidence = OpaqueEvidence::new(());
        let result = profile.evaluate_all(&evidence).await;

        assert!(result.passed);
        assert!((result.score - 1.0).abs() < f64::EPSILON);
        assert_eq!(result.results.len(), 2);
    }

    #[tokio::test]
    async fn profile_evaluate_all_fails_when_any_criterion_fails() {
        let profile = OpaqueProfile::new("mixed")
            .with_criterion(Box::new(AlwaysPassCriterion))
            .with_criterion(Box::new(AlwaysFailCriterion));

        let evidence = OpaqueEvidence::new(());
        let result = profile.evaluate_all(&evidence).await;

        // Overall profile fails because one criterion failed.
        assert!(!result.passed);
        // Score is the mean: (1.0 + 0.0) / 2 = 0.5
        assert!((result.score - 0.5).abs() < f64::EPSILON);
        assert_eq!(result.results.len(), 2);
    }

    #[tokio::test]
    async fn profile_evaluate_all_empty_profile_passes() {
        let profile = OpaqueProfile::new("empty");
        let evidence = OpaqueEvidence::new(());
        let result = profile.evaluate_all(&evidence).await;

        assert!(result.passed);
        assert!((result.score - 1.0).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn profile_evaluate_all_criterion_error_treated_as_failure() {
        struct ErrorCriterion;

        #[async_trait::async_trait]
        impl OpaqueCriterion for ErrorCriterion {
            fn name(&self) -> &str {
                "error_criterion"
            }
            fn description(&self) -> &str {
                "always returns an error"
            }
            async fn evaluate(
                &self,
                _evidence: &OpaqueEvidence,
            ) -> anyhow::Result<OpaqueCriterionResult> {
                Err(anyhow::anyhow!("simulated evaluation error"))
            }
        }

        let profile = OpaqueProfile::new("error-profile").with_criterion(Box::new(ErrorCriterion));

        let evidence = OpaqueEvidence::new(());
        let result = profile.evaluate_all(&evidence).await;

        assert!(!result.passed);
        assert_eq!(result.results.len(), 1);
        let (name, cr) = &result.results[0];
        assert_eq!(name, "error_criterion");
        assert!(cr.details.contains("criterion error"));
    }

    // -- ProfileResult serialization --

    #[test]
    fn profile_result_serializes_round_trip() {
        let result = ProfileResult {
            profile_name: "my-profile".to_string(),
            passed: true,
            score: 0.9,
            results: vec![(
                "compile".to_string(),
                OpaqueCriterionResult::pass().with_details("clean"),
            )],
        };

        let json = serde_json::to_string(&result).expect("serialize");
        let back: ProfileResult = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.profile_name, "my-profile");
        assert!(back.passed);
        assert_eq!(back.results.len(), 1);
    }
}
