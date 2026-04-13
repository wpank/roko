# PRD-03 — Criterion Library: Built-in Criteria, Authoring Format, and Profiles

**Status**: Draft
**Author**: Will (architect) + Claude (synthesis)
**Date**: 2026-04-25 (revised 2026-04-29)
**Crate**: `roko-eval-metrics` (new, Layer 2), with select items in `roko-eval-judge` and `roko-eval-browser`
**Prerequisites**: PRD-00 (System Overview), PRD-01 (Core Abstractions), PRD-02 (Evidence Collectors)
**Implementation path**: `crates/roko-eval-metrics/src/`

---

## 0. Scope

This document defines every built-in criterion that ships with roko's unified evaluation
framework. Each criterion is specified with:

- **What it measures** and why it matters
- **Required evidence kinds** (`EvidenceKind` from PRD-01)
- **Severity classification** (hard vs soft) with rationale
- **Formula** (for computational metrics)
- **Threshold** (default, configurable)
- **Research citation** (full references in PRD-09)
- **Rust type stub** (struct definition + `Criterion` impl sketch)
- **Migration notes** (which existing gate it replaces, what changes)
- **Integration with existing gate pipeline** (how the criterion maps to `GateService` rungs)

Criteria are organized into ten domains:

1. **Code** (compile, lint, test, security, format, diff, symbol)
2. **AST** (structural completeness, complexity, dead code) -- Novel
3. **Semantic Diff** (change classification, substance detection) -- Novel
4. **Runtime** (coverage, performance regression) -- Novel
5. **Visual** (15 computational metrics)
6. **Accessibility** (structural integrity, full WCAG, APCA, reduced motion)
7. **Performance** (Core Web Vitals, benchmark regression)
8. **Judge** (single judge, judge panel)
9. **Visual Regression** (odiff + dssim)
10. **Behavioral** (journey completion, console clean, network clean)

Section 11 covers the criterion authoring format (user-authored criteria in TOML).
Section 12 defines the built-in profiles that compose criteria into named strategies.

---

## 1. Code Criteria

Code criteria migrate from `roko-gate`'s existing gate implementations. They form the
foundation of every evaluation profile.

### 1.1 CompileCriterion

**Migrates**: `CompileGate` (`crates/roko-gate/src/compile.rs`)
**GateService rung**: 0 (compile)
**What**: Verifies that the project compiles via its build system.
**Why**: Compilation is the cheapest gate with the highest signal.

**Required Evidence**: `EvidenceKind::ProcessOutput`, `EvidenceKind::ProcessStatus`
**Severity**: Hard
**Score**: Binary -- 0.0 (fail) or 1.0 (pass)
**Threshold**: 1.0
**Kind**: `CriterionKind::Deterministic`

```rust
// File: crates/roko-eval-metrics/src/compile.rs

/// Verifies that the project compiles via its build system.
///
/// Migrates `CompileGate` from `crates/roko-gate/src/compile.rs`.
/// The key difference: this criterion consumes evidence from the bag
/// rather than spawning its own subprocess. The `ProcessCollector`
/// runs the build command; this criterion interprets the result.
///
/// Leverages the structured compile error parsing from
/// `crates/roko-gate/src/compile_errors.rs`:
/// - `parse_cargo_json` for JSON-formatted Cargo output
/// - `parse_plain_stderr` for plain-text stderr
/// - `classify_error_code` for error categorization (type, lifetime, borrow, etc.)
/// - `structured_gate_failure` for `GateFailureClassification`
pub struct CompileCriterion {
    pub build_system: BuildSystem,
    pub max_error_findings: usize,
    /// Whether to use JSON diagnostics (Cargo --message-format=json).
    pub json_diagnostics: bool,
}

impl CompileCriterion {
    pub fn cargo() -> Self {
        Self {
            build_system: BuildSystem::Cargo,
            max_error_findings: 5,
            json_diagnostics: true,
        }
    }
}

#[async_trait]
impl Criterion for CompileCriterion {
    fn name(&self) -> &str { "compile" }
    fn criterion_kind(&self) -> CriterionKind { CriterionKind::Deterministic }
    fn is_hard(&self) -> bool { true }
    fn required_evidence(&self) -> &[EvidenceKind] {
        &[EvidenceKind::ProcessOutput, EvidenceKind::ProcessStatus]
    }
    fn default_threshold(&self) -> f64 { 1.0 }

    async fn evaluate(
        &self,
        _artifact: &ArtifactRef,
        evidence: &EvidenceBag,
        _ctx: &Context,
    ) -> Result<CriterionResult, EvalError> {
        let status = evidence.get_one(EvidenceKind::ProcessStatus)
            .ok_or_else(|| EvalError::EvidenceUnavailable {
                criterion: "compile".into(),
                kind: EvidenceKind::ProcessStatus,
            })?;
        let output = evidence.get_one(EvidenceKind::ProcessOutput);

        let exit_code = status.extract_exit_code()?;
        let stderr = output
            .and_then(|e| e.extract_text().ok())
            .unwrap_or_default();

        if exit_code == 0 {
            Ok(CriterionResult {
                criterion: "compile".into(),
                kind: CriterionKind::Deterministic,
                score: 1.0,
                passed: true,
                findings: vec![],
                ..Default::default()
            })
        } else {
            // Use the existing structured error parsing from roko-gate
            let findings = if self.json_diagnostics {
                // Parse JSON diagnostics from cargo
                let errors = roko_gate::parse_cargo_json(&stderr);
                errors.iter()
                    .take(self.max_error_findings)
                    .map(|e| Finding {
                        criterion: "compile".into(),
                        severity: Severity::Hard,
                        summary: e.message.clone(),
                        source_location: Some(format!("{}:{}:{}",
                            e.file.as_deref().unwrap_or("?"),
                            e.line.unwrap_or(0),
                            e.column.unwrap_or(0))),
                        rule_id: e.code.clone(),
                        source_tool: Some("rustc".into()),
                        fix_hint: e.suggestion.clone(),
                        ..Default::default()
                    })
                    .collect()
            } else {
                // Fall back to plain-text parsing
                let errors = roko_gate::parse_plain_stderr(&stderr);
                errors.iter()
                    .take(self.max_error_findings)
                    .map(|e| Finding {
                        criterion: "compile".into(),
                        severity: Severity::Hard,
                        summary: e.message.clone(),
                        source_location: e.location.clone(),
                        rule_id: e.code.clone(),
                        source_tool: Some(self.build_system.program().into()),
                        ..Default::default()
                    })
                    .collect()
            };

            Ok(CriterionResult {
                criterion: "compile".into(),
                kind: CriterionKind::Deterministic,
                score: 0.0,
                passed: false,
                findings,
                ..Default::default()
            })
        }
    }
}
```

**Integration with GateService**: When `BridgeGateService` receives gate name
`"compile"` or `"compile:cargo"` (rung 0), it routes through `CompileCriterion`
instead of `CompileGate`. The `ProcessCollector::for_compile()` provides the
evidence. `GateService::rung_for_name("compile")` returns `Some(0)`, which maps
to the `CompileCriterion` in the new registry.

---

### 1.2 LintCriterion

**Migrates**: `ClippyGate` (`crates/roko-gate/src/clippy_gate.rs`)
**GateService rung**: 1 (clippy)

```rust
// File: crates/roko-eval-metrics/src/lint.rs

pub struct LintCriterion {
    pub build_system: BuildSystem,
    pub strict: bool,
    pub error_weight: f64,
    pub warning_weight: f64,
    pub max_findings: usize,
}

impl LintCriterion {
    pub fn cargo_strict() -> Self {
        Self { build_system: BuildSystem::Cargo, strict: true,
               error_weight: 0.2, warning_weight: 0.05, max_findings: 10 }
    }
    pub fn cargo_graduated() -> Self {
        Self { strict: false, ..Self::cargo_strict() }
    }
}

#[async_trait]
impl Criterion for LintCriterion {
    fn name(&self) -> &str { "lint" }
    fn criterion_kind(&self) -> CriterionKind { CriterionKind::Deterministic }
    fn is_hard(&self) -> bool { true }
    fn required_evidence(&self) -> &[EvidenceKind] {
        &[EvidenceKind::ProcessOutput, EvidenceKind::ProcessStatus]
    }

    async fn evaluate(
        &self,
        _artifact: &ArtifactRef,
        evidence: &EvidenceBag,
        _ctx: &Context,
    ) -> Result<CriterionResult, EvalError> {
        let status = evidence.get_one(EvidenceKind::ProcessStatus)
            .ok_or_else(|| EvalError::EvidenceUnavailable {
                criterion: "lint".into(), kind: EvidenceKind::ProcessStatus,
            })?;
        let output = evidence.get_one(EvidenceKind::ProcessOutput);
        let stderr = output.and_then(|e| e.extract_text().ok()).unwrap_or_default();

        let diagnostics = parse_lint_diagnostics(&stderr, self.build_system);
        let error_count = diagnostics.iter().filter(|d| d.level == "error").count();
        let warning_count = diagnostics.iter().filter(|d| d.level == "warning").count();

        let score = if self.strict {
            if status.extract_exit_code()? == 0 { 1.0 } else { 0.0 }
        } else {
            (1.0 - (error_count as f64 * self.error_weight
                + warning_count as f64 * self.warning_weight))
                .clamp(0.0, 1.0)
        };

        let findings: Vec<Finding> = diagnostics.iter()
            .take(self.max_findings)
            .map(|d| Finding {
                criterion: "lint".into(),
                severity: if d.level == "error" { Severity::Hard } else { Severity::Soft },
                summary: format!("[{}] {}", d.rule, d.message),
                source_location: d.location.clone(),
                rule_id: Some(d.rule.clone()),
                fix_hint: d.suggestion.clone(),
                source_tool: Some(self.build_system.program().into()),
                ..Default::default()
            })
            .collect();

        Ok(CriterionResult {
            criterion: "lint".into(),
            kind: CriterionKind::Deterministic,
            score,
            passed: score >= 1.0 || (!self.strict && score >= 0.8),
            findings,
            ..Default::default()
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintDiagnostic {
    pub rule: String,
    pub level: String,
    pub message: String,
    pub location: Option<String>,
    pub suggestion: Option<String>,
}

fn parse_lint_diagnostics(stderr: &str, build_system: BuildSystem) -> Vec<LintDiagnostic> {
    // Mirrors ClippyGate::summarize_lint_issues with structured extraction.
    // For Cargo: matches `warning[rule]: message` / `error[rule]: message`
    // with `--> file:line:col` on the following line.
    todo!()
}
```

---

### 1.3 TestCriterion

**Migrates**: `TestGate` (`crates/roko-gate/src/test_gate.rs`)
**GateService rung**: 2 (test)

```rust
// File: crates/roko-eval-metrics/src/test.rs

pub struct TestCriterion {
    pub build_system: BuildSystem,
    pub selector: TestSelector,
    pub min_pass_rate: f64,
    pub max_failure_findings: usize,
}

impl TestCriterion {
    pub fn cargo() -> Self {
        Self { build_system: BuildSystem::Cargo, selector: TestSelector::All,
               min_pass_rate: 1.0, max_failure_findings: 10 }
    }
}

#[async_trait]
impl Criterion for TestCriterion {
    fn name(&self) -> &str { "test" }
    fn criterion_kind(&self) -> CriterionKind { CriterionKind::Deterministic }
    fn is_hard(&self) -> bool { true }
    fn required_evidence(&self) -> &[EvidenceKind] {
        &[EvidenceKind::ProcessOutput, EvidenceKind::ProcessStatus]
    }

    async fn evaluate(
        &self,
        _artifact: &ArtifactRef,
        evidence: &EvidenceBag,
        _ctx: &Context,
    ) -> Result<CriterionResult, EvalError> {
        let output = evidence.get_one(EvidenceKind::ProcessOutput)
            .ok_or_else(|| EvalError::EvidenceUnavailable {
                criterion: "test".into(), kind: EvidenceKind::ProcessOutput,
            })?;
        let combined = output.extract_text()?;

        // Reuse existing parsing from roko-gate
        let counts = roko_gate::parse_test_counts(&combined);
        let (score, passed) = match counts {
            Some(tc) => {
                let rate = if tc.total_run() == 0 { 0.0 }
                           else { tc.passed as f64 / tc.total_run() as f64 };
                (rate, rate >= self.min_pass_rate)
            }
            None => (0.0, false),
        };

        let findings = extract_test_failure_findings(
            &combined, self.build_system, self.max_failure_findings
        );

        Ok(CriterionResult {
            criterion: "test".into(),
            kind: CriterionKind::Deterministic,
            score,
            passed,
            findings,
            metadata: counts.map(|tc| serde_json::to_value(tc).unwrap_or_default()),
            ..Default::default()
        })
    }
}

fn extract_test_failure_findings(
    output: &str,
    build_system: BuildSystem,
    max: usize,
) -> Vec<Finding> {
    // Extract failing test names and their output.
    // For Cargo: parse `---- test_name stdout ----` blocks.
    // For Go: parse `--- FAIL: TestName` blocks.
    let mut findings = Vec::new();
    // ... extraction logic matching TestGate behavior ...
    findings.truncate(max);
    findings
}
```

---

### 1.4 SecurityCriterion, FormatCriterion, DiffCriterion, SymbolCriterion

These follow the same pattern as above. Each migrates from its corresponding gate:

| Criterion | Migrates | Rung | Key Difference |
|---|---|---|---|
| `SecurityCriterion` | `SecurityScanGate` | N/A (standalone) | Emits info-level finding when audit tool missing |
| `FormatCriterion` | `FormatCheckGate` | 4 (fmt) | Extracts unformatted file paths as findings |
| `DiffCriterion` | `DiffGate` | 3 (diff) | Optionally consumes `SemanticDiff` evidence for richer analysis |
| `SymbolCriterion` | `SymbolGate` | 3 (symbol) | Optionally consumes `Ast` evidence instead of regex scanning |

---

## 2. AST Criteria -- Novel

These criteria consume `EvidenceKind::Ast` evidence from the `AstCollector` (PRD-02 Section 5).
They provide structural analysis that goes beyond what text-level gates can detect.

### 2.1 StructuralCompletenessCriterion

**What**: Verifies that the agent produced all expected structural elements: functions, types,
trait implementations, modules. Uses AST evidence instead of the regex-based approach in the
current `SymbolGate`.

**Required Evidence**: `EvidenceKind::Ast`
**Optional Evidence**: `EvidenceKind::SemanticDiff` (to focus on newly added items)
**Severity**: Hard
**Score**: `met_expectations / total_expectations`
**Threshold**: 1.0
**Kind**: `CriterionKind::Deterministic`

```rust
// File: crates/roko-eval-metrics/src/structural_completeness.rs

/// AST-based structural completeness check.
///
/// Upgrades `SymbolGate` from regex scanning to tree-sitter AST analysis.
/// Advantages over the regex approach:
/// - Handles nested items (methods inside impl blocks inside modules)
/// - Correctly parses generics and where clauses
/// - Detects macro-generated items
/// - Understands `pub(crate)` and `pub(super)` visibility
/// - Can verify trait implementations, not just type definitions
pub struct StructuralCompletenessCriterion {
    /// Expected items. Each expectation declares what should exist.
    pub expectations: Vec<StructuralExpectation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuralExpectation {
    /// Item kind: function, struct, enum, trait, impl, module.
    pub kind: String,
    /// Item name (can be a glob pattern, e.g., "new_*").
    pub name: String,
    /// Expected module path (e.g., "crate::eval::criterion").
    pub path: Option<String>,
    /// Expected visibility: public, pub_crate, private.
    pub visibility: Option<String>,
    /// For impl blocks: which trait is expected.
    pub impl_trait: Option<String>,
    /// Whether the item must have a non-empty body (not just `todo!()`).
    pub substantive_body: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectationResult {
    pub expectation: StructuralExpectation,
    pub met: bool,
    pub matched_item: Option<AstItem>,
    pub mismatch_reason: Option<String>,
}

#[async_trait]
impl Criterion for StructuralCompletenessCriterion {
    fn name(&self) -> &str { "structural_completeness" }
    fn criterion_kind(&self) -> CriterionKind { CriterionKind::Deterministic }
    fn is_hard(&self) -> bool { true }
    fn required_evidence(&self) -> &[EvidenceKind] { &[EvidenceKind::Ast] }

    async fn evaluate(
        &self,
        _artifact: &ArtifactRef,
        evidence: &EvidenceBag,
        _ctx: &Context,
    ) -> Result<CriterionResult, EvalError> {
        let ast_evidence = evidence.get_one(EvidenceKind::Ast)
            .ok_or_else(|| EvalError::EvidenceUnavailable {
                criterion: "structural_completeness".into(),
                kind: EvidenceKind::Ast,
            })?;

        let file_asts: Vec<FileAst> = ast_evidence.extract_json()?;

        // Flatten all items from all files
        let all_items: Vec<&AstItem> = file_asts.iter()
            .flat_map(|f| collect_all_items(&f.items))
            .collect();

        let mut results = Vec::new();
        for expectation in &self.expectations {
            let result = check_expectation(expectation, &all_items);
            results.push(result);
        }

        let met = results.iter().filter(|r| r.met).count();
        let total = results.len();
        let score = if total == 0 { 1.0 } else { met as f64 / total as f64 };

        let findings: Vec<Finding> = results.iter()
            .filter(|r| !r.met)
            .map(|r| Finding {
                criterion: "structural_completeness".into(),
                severity: Severity::Hard,
                summary: format!(
                    "missing {} `{}` at path `{}`",
                    r.expectation.kind,
                    r.expectation.name,
                    r.expectation.path.as_deref().unwrap_or("*"),
                ),
                detail: r.mismatch_reason.clone(),
                ast_path: r.expectation.path.clone(),
                fix_hint: Some(format!(
                    "add `{} {} {}` to the expected module",
                    r.expectation.visibility.as_deref().unwrap_or("pub"),
                    r.expectation.kind,
                    r.expectation.name,
                )),
                ..Default::default()
            })
            .collect();

        Ok(CriterionResult {
            criterion: "structural_completeness".into(),
            kind: CriterionKind::Deterministic,
            score,
            passed: score >= 1.0,
            findings,
            ..Default::default()
        })
    }
}

fn collect_all_items<'a>(items: &'a [AstItem]) -> Vec<&'a AstItem> {
    let mut result = Vec::new();
    for item in items {
        result.push(item);
        result.extend(collect_all_items(&item.children));
    }
    result
}

fn check_expectation(exp: &StructuralExpectation, items: &[&AstItem]) -> ExpectationResult {
    // Match by kind, name pattern, path, visibility.
    // If substantive_body is required, check that the item's body is
    // not just `todo!()`, `unimplemented!()`, or empty.
    todo!()
}
```

### 2.2 ComplexityCriterion

**What**: Checks that no function exceeds a cyclomatic or cognitive complexity threshold.
**Required Evidence**: `EvidenceKind::Ast`
**Severity**: Soft
**Score**: `functions_within_threshold / total_functions`
**Threshold**: 0.9 (90% of functions below threshold)
**Kind**: `CriterionKind::Computed`

```rust
// File: crates/roko-eval-metrics/src/complexity.rs

pub struct ComplexityCriterion {
    /// Maximum cyclomatic complexity per function.
    pub max_cyclomatic: u32,
    /// Maximum cognitive complexity per function.
    pub max_cognitive: u32,
    /// Maximum function body lines.
    pub max_body_lines: u32,
    /// Minimum fraction of functions within thresholds.
    pub threshold: f64,
}

impl Default for ComplexityCriterion {
    fn default() -> Self {
        Self {
            max_cyclomatic: 15,
            max_cognitive: 20,
            max_body_lines: 100,
            threshold: 0.9,
        }
    }
}

#[async_trait]
impl Criterion for ComplexityCriterion {
    fn name(&self) -> &str { "complexity" }
    fn criterion_kind(&self) -> CriterionKind { CriterionKind::Computed }
    fn is_hard(&self) -> bool { false }
    fn required_evidence(&self) -> &[EvidenceKind] { &[EvidenceKind::Ast] }

    async fn evaluate(
        &self,
        _artifact: &ArtifactRef,
        evidence: &EvidenceBag,
        _ctx: &Context,
    ) -> Result<CriterionResult, EvalError> {
        let file_asts: Vec<FileAst> = evidence.get_one(EvidenceKind::Ast)
            .ok_or_else(|| EvalError::EvidenceUnavailable {
                criterion: "complexity".into(), kind: EvidenceKind::Ast,
            })?
            .extract_json()?;

        let all_functions: Vec<&FunctionComplexity> = file_asts.iter()
            .flat_map(|f| f.complexity.iter())
            .collect();

        let within_threshold = all_functions.iter()
            .filter(|f| f.cyclomatic <= self.max_cyclomatic
                     && f.cognitive <= self.max_cognitive
                     && f.body_lines <= self.max_body_lines)
            .count();

        let total = all_functions.len();
        let score = if total == 0 { 1.0 } else { within_threshold as f64 / total as f64 };

        let findings: Vec<Finding> = all_functions.iter()
            .filter(|f| f.cyclomatic > self.max_cyclomatic
                     || f.cognitive > self.max_cognitive
                     || f.body_lines > self.max_body_lines)
            .map(|f| {
                let reasons = vec![
                    (f.cyclomatic > self.max_cyclomatic)
                        .then(|| format!("cyclomatic={} (max {})", f.cyclomatic, self.max_cyclomatic)),
                    (f.cognitive > self.max_cognitive)
                        .then(|| format!("cognitive={} (max {})", f.cognitive, self.max_cognitive)),
                    (f.body_lines > self.max_body_lines)
                        .then(|| format!("lines={} (max {})", f.body_lines, self.max_body_lines)),
                ]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(", ");

                Finding {
                    criterion: "complexity".into(),
                    severity: Severity::Soft,
                    summary: format!("`{}`: {reasons}", f.name),
                    ast_path: Some(f.path.clone()),
                    fix_hint: Some("consider extracting helper functions to reduce complexity".into()),
                    ..Default::default()
                }
            })
            .collect();

        Ok(CriterionResult {
            criterion: "complexity".into(),
            kind: CriterionKind::Computed,
            score,
            passed: score >= self.threshold,
            findings,
            ..Default::default()
        })
    }
}
```

---

## 3. Semantic Diff Criteria -- Novel

These criteria consume `EvidenceKind::SemanticDiff` evidence to make judgments about
the substance and quality of code changes.

### 3.1 SubstanceCriterion

**What**: Scores the substantive content of a diff by classifying changes at the AST level.
Catches the "did nothing" failure mode that `DiffGate` currently detects via forbidden token
matching, but with much higher precision.

**Required Evidence**: `EvidenceKind::SemanticDiff`
**Optional Evidence**: `EvidenceKind::Diff` (fallback to text-level analysis)
**Severity**: Soft (default), configurable to Hard
**Score**: Weighted significance of changes
**Threshold**: 0.3 (at least 30% of changes are structurally significant)
**Kind**: `CriterionKind::Computed`

```rust
// File: crates/roko-eval-metrics/src/substance.rs

/// Scores the substantive content of code changes.
///
/// Upgrades `DiffGate`'s forbidden-token detection with AST-level
/// change classification. A diff that adds 200 lines of `todo!()` stubs
/// has significance ~0.0. A diff that adds 50 lines of real function
/// implementations has significance ~0.9.
pub struct SubstanceCriterion {
    /// Minimum average significance to pass.
    pub threshold: f64,
    /// Change kinds that count as non-substantive.
    pub non_substantive_kinds: Vec<SemanticChangeKind>,
}

impl Default for SubstanceCriterion {
    fn default() -> Self {
        Self {
            threshold: 0.3,
            non_substantive_kinds: vec![
                SemanticChangeKind::FormattingOnly,
                SemanticChangeKind::DocumentationChanged,
                SemanticChangeKind::ImportChanged,
            ],
        }
    }
}

#[async_trait]
impl Criterion for SubstanceCriterion {
    fn name(&self) -> &str { "substance" }
    fn criterion_kind(&self) -> CriterionKind { CriterionKind::Computed }
    fn is_hard(&self) -> bool { false }
    fn required_evidence(&self) -> &[EvidenceKind] { &[EvidenceKind::SemanticDiff] }
    fn optional_evidence(&self) -> &[EvidenceKind] { &[EvidenceKind::Diff] }

    async fn evaluate(
        &self,
        _artifact: &ArtifactRef,
        evidence: &EvidenceBag,
        _ctx: &Context,
    ) -> Result<CriterionResult, EvalError> {
        let semantic_diff: Vec<SemanticChange> = evidence
            .get_one(EvidenceKind::SemanticDiff)
            .ok_or_else(|| EvalError::EvidenceUnavailable {
                criterion: "substance".into(),
                kind: EvidenceKind::SemanticDiff,
            })?
            .extract_json()?;

        if semantic_diff.is_empty() {
            return Ok(CriterionResult {
                criterion: "substance".into(),
                score: 0.0,
                passed: false,
                findings: vec![Finding {
                    criterion: "substance".into(),
                    severity: Severity::Soft,
                    summary: "no semantic changes detected".into(),
                    ..Default::default()
                }],
                ..Default::default()
            });
        }

        // Compute weighted significance
        let total_significance: f64 = semantic_diff.iter()
            .map(|c| c.significance)
            .sum();
        let avg_significance = total_significance / semantic_diff.len() as f64;

        let substantive_count = semantic_diff.iter()
            .filter(|c| !self.non_substantive_kinds.contains(&c.kind))
            .count();

        let score = avg_significance.clamp(0.0, 1.0);

        let findings: Vec<Finding> = if substantive_count == 0 {
            vec![Finding {
                criterion: "substance".into(),
                severity: Severity::Soft,
                summary: format!(
                    "all {} changes are non-substantive (formatting, docs, imports)",
                    semantic_diff.len()
                ),
                fix_hint: Some("add functional changes (new functions, logic, types)".into()),
                ..Default::default()
            }]
        } else {
            // Report the most significant changes
            semantic_diff.iter()
                .filter(|c| c.significance >= 0.5)
                .take(5)
                .map(|c| Finding {
                    criterion: "substance".into(),
                    severity: Severity::Info,
                    summary: format!("{:?}: {} (sig={:.2})", c.kind, c.description, c.significance),
                    ast_path: c.ast_path.clone(),
                    ..Default::default()
                })
                .collect()
        };

        Ok(CriterionResult {
            criterion: "substance".into(),
            kind: CriterionKind::Computed,
            score,
            passed: score >= self.threshold,
            findings,
            ..Default::default()
        })
    }
}
```

---

## 4. Runtime Criteria -- Novel

### 4.1 CoverageCriterion

**What**: Checks that test coverage meets a minimum threshold.
**Required Evidence**: `EvidenceKind::RuntimeTrace`
**Severity**: Soft
**Score**: `covered_lines / total_lines`
**Threshold**: 0.7 (70% line coverage)

```rust
// File: crates/roko-eval-metrics/src/coverage.rs

pub struct CoverageCriterion {
    pub min_line_coverage: f64,
    pub min_branch_coverage: Option<f64>,
    /// Only consider coverage for files changed in the current diff.
    pub diff_only: bool,
}

impl Default for CoverageCriterion {
    fn default() -> Self {
        Self { min_line_coverage: 0.7, min_branch_coverage: None, diff_only: true }
    }
}

#[async_trait]
impl Criterion for CoverageCriterion {
    fn name(&self) -> &str { "coverage" }
    fn criterion_kind(&self) -> CriterionKind { CriterionKind::Computed }
    fn is_hard(&self) -> bool { false }
    fn required_evidence(&self) -> &[EvidenceKind] { &[EvidenceKind::RuntimeTrace] }
    fn optional_evidence(&self) -> &[EvidenceKind] { &[EvidenceKind::Diff] }

    async fn evaluate(
        &self,
        _artifact: &ArtifactRef,
        evidence: &EvidenceBag,
        _ctx: &Context,
    ) -> Result<CriterionResult, EvalError> {
        let trace: RuntimeTraceData = evidence
            .get_one(EvidenceKind::RuntimeTrace)
            .ok_or_else(|| EvalError::EvidenceUnavailable {
                criterion: "coverage".into(), kind: EvidenceKind::RuntimeTrace,
            })?
            .extract_json()?;

        let coverage = trace.coverage.ok_or_else(|| EvalError::Evaluation {
            criterion: "coverage".into(),
            message: "no coverage data in runtime trace".into(),
        })?;

        let score = coverage.line_coverage;
        let passed = score >= self.min_line_coverage;

        let mut findings = Vec::new();
        if !passed {
            // Report files with lowest coverage
            let mut files = coverage.files.clone();
            files.sort_by(|a, b| a.line_coverage.partial_cmp(&b.line_coverage).unwrap());
            for file in files.iter().take(5) {
                if file.line_coverage < self.min_line_coverage {
                    findings.push(Finding {
                        criterion: "coverage".into(),
                        severity: Severity::Soft,
                        summary: format!(
                            "{}: {:.0}% line coverage (need {:.0}%)",
                            file.path,
                            file.line_coverage * 100.0,
                            self.min_line_coverage * 100.0,
                        ),
                        source_location: Some(file.path.clone()),
                        fix_hint: Some(format!(
                            "add tests covering lines: {:?}",
                            &file.uncovered_lines[..file.uncovered_lines.len().min(10)]
                        )),
                        ..Default::default()
                    });
                }
            }
        }

        Ok(CriterionResult {
            criterion: "coverage".into(),
            kind: CriterionKind::Computed,
            score,
            passed,
            findings,
            metadata: Some(serde_json::to_value(&coverage).unwrap_or_default()),
            ..Default::default()
        })
    }
}
```

---

## 5. Visual Criteria (15 Computational Metrics)

These criteria migrate from the metrics engine specified in the original `PRD-02-Browser-Runner-and-Metrics.md`. They run on evidence produced by `BrowserCollector`.

### Summary Table

| # | Criterion | Evidence | Kind | Severity | Threshold |
|---|---|---|---|---|---|
| 5.1 | TokenCoverageCriterion | ComputedStyles, DesignTokens | Deterministic | Hard | >= 0.9 |
| 5.2 | WcagContrastCriterion | ComputedStyles, Dom | Deterministic | Hard | 1.0 |
| 5.3 | ApcaContrastCriterion | ComputedStyles, Dom | Deterministic | Hard | >= 0.95 |
| 5.4 | GridAdherenceCriterion | LayoutMetrics | Deterministic | Hard | >= 0.95 |
| 5.5 | AlignmentScoreCriterion | LayoutMetrics | Computed | Hard | >= 0.7 |
| 5.6 | VisualDensityCriterion | LayoutMetrics | Computed | Soft | [0.3, 0.7] |
| 5.7 | VisualBalanceCriterion | LayoutMetrics | Computed | Soft | >= 0.6 |
| 5.8 | TypeScaleCriterion | ComputedStyles | Deterministic | Soft | >= 0.9 |
| 5.9 | ConsoleCleanCriterion | ConsoleLog | Deterministic | Hard | 0 errors |
| 5.10 | CoreWebVitalsCriterion | PerformanceTrace | Computed | Soft | LCP<2.5s, CLS<0.1 |
| 5.11 | ColorfulnessCriterion | Screenshot | Heuristic | Soft | [15, 80] |
| 5.12 | SaliencyCriterion | SaliencyMap | Heuristic | Soft | >= 0.3 |
| 5.13 | ReducedMotionCriterion | ComputedStyles, Dom | Deterministic | Hard | 1.0 |
| 5.14 | OdiffRegressionCriterion | RegressionDiff | Computed | Hard | < 5% changed pixels |
| 5.15 | AimFeatureCongestionCriterion | Screenshot | Heuristic | Soft | < 0.7 |

Each follows the same pattern as the code criteria. The key structural difference is
that visual criteria consume browser evidence rather than process evidence.

---

## 6. Integration with Existing Gate Pipeline

### 6.1 Rung-to-Criterion Mapping

The existing 7-rung gate pipeline (defined in `crates/roko-gate/src/rung_dispatch.rs`)
maps to criteria as follows:

| Rung | Index | Current Gate | New Criterion | Evidence Collector |
|---|---|---|---|---|
| Compile | 0 | `CompileGate` | `CompileCriterion` | `ProcessCollector::for_compile()` |
| Lint | 1 | `ClippyGate` | `LintCriterion` | `ProcessCollector::for_lint()` |
| Test | 2 | `TestGate` | `TestCriterion` | `ProcessCollector::for_test()` |
| Symbol | 3 | `SymbolGate` | `StructuralCompletenessCriterion` | `AstCollector` |
| GeneratedTest | 4 | `GeneratedTestGate` + `VerifyChainGate` | `GeneratedTestCriterion` | `ProcessCollector` + `RuntimeTraceCollector` |
| PropertyTest | 5 | `PropertyTestGate` + `FactCheckGate` | `PropertyTestCriterion` | `ProcessCollector` |
| Integration | 6 | `LlmJudgeGate` + `IntegrationGate` | `JudgePanelCriterion` | `DiffCollector` + `ProcessCollector` |

### 6.2 GateService Extension Points

The `GateService::gate_for_name()` method (at `crates/roko-gate/src/gate_service.rs` line 66)
currently uses a hardcoded match. The `BridgeGateService` intercepts this by checking
its `migrated` set first:

```rust
// Pseudocode for the migration path in bridge.rs
fn resolve_gate(&self, name: &str, build_system: BuildSystem) -> GateResolution {
    if self.migrated.contains(name) {
        // Route through new EvalService
        GateResolution::Criterion(self.criterion_registry.get(name))
    } else {
        // Fall back to old GateService
        GateResolution::Legacy(self.legacy.gate_for_name(name, build_system))
    }
}
```

### 6.3 AdaptiveThresholds Integration

The existing `AdaptiveThresholds` (at `crates/roko-gate/src/adaptive_threshold.rs`) tracks
per-rung pass rates. In the new system, this extends to per-criterion stats:

```rust
// File: crates/roko-eval/src/stats.rs

/// Per-criterion statistics (extends AdaptiveThresholds to criterion granularity).
///
/// Instead of tracking stats per-rung (7 entries), tracks per-criterion
/// (unbounded). The EMA, CUSUM, and SPC algorithms from AdaptiveThresholds
/// are reused unchanged.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriterionStats {
    /// EMA pass rate for this criterion.
    pub ema_pass_rate: f64,
    pub total_observations: u64,
    pub consecutive_passes: u32,
    pub cusum_high: f64,
    pub cusum_low: f64,
    pub cusum_shift_detected: bool,
    /// Score history (last N scores for trend detection).
    pub score_history: Vec<f64>,
    /// Average evaluation duration (ms).
    pub avg_duration_ms: f64,
    /// Average evaluation cost (USD).
    pub avg_cost_usd: f64,
}

impl CriterionStats {
    pub fn should_skip(&self, skip_streak_threshold: u32) -> bool {
        self.consecutive_passes >= skip_streak_threshold
    }

    pub fn observe(&mut self, passed: bool, score: f64, duration_ms: u64, cost_usd: f64) {
        // EMA update (same algorithm as RungStats)
        let alpha = 0.1;
        let value = if passed { 1.0 } else { 0.0 };
        self.ema_pass_rate = alpha * value + (1.0 - alpha) * self.ema_pass_rate;
        self.total_observations += 1;

        if passed {
            self.consecutive_passes += 1;
        } else {
            self.consecutive_passes = 0;
        }

        // CUSUM update
        let k = 0.25;
        let h = 4.0;
        self.cusum_high = (self.cusum_high + value - 0.5 - k).max(0.0);
        self.cusum_low = (self.cusum_low - value + 0.5 - k).max(0.0);
        self.cusum_shift_detected = self.cusum_high > h || self.cusum_low > h;

        // Duration/cost tracking
        let n = self.total_observations as f64;
        self.avg_duration_ms = ((n - 1.0) * self.avg_duration_ms + duration_ms as f64) / n;
        self.avg_cost_usd = ((n - 1.0) * self.avg_cost_usd + cost_usd) / n;

        // Score history (keep last 50)
        self.score_history.push(score);
        if self.score_history.len() > 50 {
            self.score_history.remove(0);
        }
    }
}
```

---

## 7. Criterion Authoring Format

Users can author custom criteria in TOML. Custom criteria delegate to either a shell
command (deterministic) or an LLM judge (stochastic).

```toml
# File: .roko/criteria/custom-no-unwrap.toml

[criterion]
name = "no_unwrap"
description = "Reject .unwrap() calls in production code (not tests)"
kind = "deterministic"
severity = "hard"

[criterion.evidence]
required = ["diff"]

[criterion.check]
# Shell command that exits 0 for pass, nonzero for fail.
# Evidence is available via environment variables.
type = "shell"
command = """
grep -rn '\.unwrap()' --include='*.rs' \
  --exclude-dir=tests --exclude='*_test.rs' \
  ${EVAL_ARTIFACT_PATH}/src/ \
  && exit 1 || exit 0
"""

[criterion.finding]
summary_template = "Found .unwrap() calls in production code"
fix_hint = "Replace .unwrap() with .expect(\"reason\") or proper error handling"
```

```toml
# File: .roko/criteria/code-review-judge.toml

[criterion]
name = "code_review"
description = "LLM-based code review quality check"
kind = "judge_panel"
severity = "soft"

[criterion.evidence]
required = ["diff"]
optional = ["ast", "semantic_diff"]

[criterion.judge]
type = "panel"
models = ["claude-sonnet-4-20250514", "gpt-4o"]
rubric = """
Evaluate this diff for:
1. Correctness: Does the code do what the task requires?
2. Completeness: Are all edge cases handled?
3. Style: Does it follow the project's conventions?
4. Safety: Are there obvious bugs or security issues?

Score each dimension 0-10. Overall score is the minimum.
"""
```

---

## 8. Built-in Profiles

### 8.1 Rust Strict Profile

```toml
# Replaces the default GateConfig { enabled_gates: ["compile", "clippy", "test", ...] }

[profile]
id = "rust-strict"
name = "Rust Strict"
tags = ["rust", "ci"]

[profile.strategy]
kind = "sequential"

[[profile.criteria]]
ref_ = "compile"

[[profile.criteria]]
ref_ = "lint"

[[profile.criteria]]
ref_ = "test"

[[profile.criteria]]
ref_ = "format"

[[profile.criteria]]
ref_ = "diff"
hard = false

[[profile.criteria]]
ref_ = "substance"
hard = false
threshold = 0.3
```

### 8.2 Full Stack Web Profile

```toml
[profile]
id = "fullstack-web"
name = "Full Stack Web"
tags = ["web", "visual", "a11y"]

[profile.strategy]
kind = "sequential"

# Code gates first (cheap, deterministic)
[[profile.criteria]]
ref_ = "compile"
[[profile.criteria]]
ref_ = "lint"
[[profile.criteria]]
ref_ = "test"

# AST analysis (cheap, structural)
[[profile.criteria]]
ref_ = "structural_completeness"
[[profile.criteria]]
ref_ = "complexity"
hard = false

# Visual gates (requires browser, expensive)
[[profile.criteria]]
ref_ = "wcag_contrast"
[[profile.criteria]]
ref_ = "apca_contrast"
[[profile.criteria]]
ref_ = "grid_adherence"
[[profile.criteria]]
ref_ = "console_clean"

# Judge (most expensive, runs last)
[[profile.criteria]]
ref_ = "code_review"
hard = false
```

---

## 9. Implementation Checklist

| # | File | What |
|---|---|---|
| 1 | `crates/roko-eval-metrics/Cargo.toml` | New crate |
| 2 | `crates/roko-eval-metrics/src/lib.rs` | Crate root, re-exports |
| 3 | `crates/roko-eval-metrics/src/compile.rs` | CompileCriterion |
| 4 | `crates/roko-eval-metrics/src/lint.rs` | LintCriterion |
| 5 | `crates/roko-eval-metrics/src/test.rs` | TestCriterion |
| 6 | `crates/roko-eval-metrics/src/format.rs` | FormatCriterion |
| 7 | `crates/roko-eval-metrics/src/security.rs` | SecurityCriterion |
| 8 | `crates/roko-eval-metrics/src/diff.rs` | DiffCriterion |
| 9 | `crates/roko-eval-metrics/src/symbol.rs` | SymbolCriterion (legacy regex mode) |
| 10 | `crates/roko-eval-metrics/src/structural_completeness.rs` | StructuralCompletenessCriterion (AST mode) |
| 11 | `crates/roko-eval-metrics/src/complexity.rs` | ComplexityCriterion |
| 12 | `crates/roko-eval-metrics/src/substance.rs` | SubstanceCriterion |
| 13 | `crates/roko-eval-metrics/src/coverage.rs` | CoverageCriterion |
| 14 | `crates/roko-eval-metrics/src/ast_analysis.rs` | AstCollector (from PRD-02) |
| 15 | `crates/roko-eval-metrics/src/semantic_diff.rs` | SemanticDiff (from PRD-02) |
| 16 | `crates/roko-eval-metrics/src/runtime_trace.rs` | RuntimeTraceCollector (from PRD-02) |
| 17 | `crates/roko-eval/src/stats.rs` | CriterionStats |
| 18 | `crates/roko-gate/src/bridge.rs` | BridgeGateService |
