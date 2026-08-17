# M119 — Diagnosis Route Cell and error categories

## Objective
Extend the existing `DiagnosisEngine` with a two-stage pipeline: a `PatternMatchScorer` that classifies errors by category with confidence, and a `DiagnosisRouter` that maps scored errors to interventions via confidence-tiered routing. The existing `ErrorCategory` (20 variants), `SuggestedIntervention` (9 variants), and `ErrorPattern` types already exist — this batch adds the scoring/routing abstraction on top.

## Scope
- Crates: `roko-conductor`
- Files:
  - `crates/roko-conductor/src/diagnosis.rs` (extend — already has `DiagnosisEngine`, `ErrorCategory`, `SuggestedIntervention`, `ErrorPattern`, `DiagnosisResult`)
  - `crates/roko-conductor/src/lib.rs` (add re-exports for new types)
- Depth doc: `tmp/unified-depth/07-agent-runtime/16-diagnosis-and-stuck-detection.md`

## Existing types reference

The diagnosis module already exists with these types (`crates/roko-conductor/src/diagnosis.rs`):

```rust
// 20-variant error classification (already exists)
pub enum ErrorCategory {
    CompileError, TestFailure, ClippyWarning, GitConflict, DependencyError,
    TypeMismatch, BorrowCheckerError, LifetimeError, ImportError,
    MissingFile, PermissionDenied, NetworkError, TimeoutError, OomError, DiskFull,
    LlmRateLimit, LlmContextOverflow, LlmRefusal, ProcessCrash, LoopDetected,
}

// 9-variant intervention suggestions (already exists)
pub enum SuggestedIntervention {
    RetryWithContext, AutoFix, RestartAgent, AbortPlan,
    BackoffRetry, MergeResolution, ReduceContext, SwitchModel, WarnAndContinue,
}

// Pattern entry (already exists)
pub struct ErrorPattern {
    pub name: &'static str,
    pub needle: &'static str,
    pub category: ErrorCategory,
    pub suggested_action: SuggestedIntervention,
    pub case_insensitive: bool,
}

// Diagnosis result (already exists)
pub struct DiagnosisResult {
    pub pattern_name: String,
    pub category: ErrorCategory,
    pub confidence: f64,
    pub suggested_intervention: SuggestedIntervention,
    pub matched_excerpt: String,
}

// Engine (already exists) — holds Vec<ErrorPattern>, has fn diagnose(&self, text: &str) -> Vec<DiagnosisResult>
pub struct DiagnosisEngine { patterns: Vec<ErrorPattern> }
```

Do NOT create new `error_kinds.rs` or `intervention_kinds.rs` files — the types already exist in `diagnosis.rs`.

## Steps
1. Discover the full existing API:
   ```bash
   grep -rn 'pub fn\|pub struct\|pub enum\|pub type' crates/roko-conductor/src/diagnosis.rs | head -30
   # Check the existing pattern table size
   grep -c 'ErrorPattern {' crates/roko-conductor/src/diagnosis.rs
   # Check what's re-exported
   grep -rn 'diagnosis::' crates/roko-conductor/src/lib.rs
   ```

2. Add a `CategoryScore` struct to `diagnosis.rs`:
   ```rust
   #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
   pub struct CategoryScore {
       pub category: ErrorCategory,
       pub confidence: f64,
       pub matched_patterns: Vec<String>,
   }
   ```

3. Add a `PatternMatchScorer` struct that wraps the existing pattern table:
   ```rust
   pub struct PatternMatchScorer {
       patterns: Vec<ErrorPattern>,
   }
   impl PatternMatchScorer {
       /// Score raw error text against all patterns, grouping by category.
       pub fn score(&self, raw_text: &str) -> Vec<CategoryScore> {
           // Group matched patterns by ErrorCategory
           // Confidence = max(base_confidence of each matching pattern in category)
           // base_confidence: exact match = 1.0, substring = 0.8, case-insensitive = 0.7
       }
   }
   ```

4. Add a `DiagnosisRouter` struct with confidence-tiered routing:
   ```rust
   pub struct DiagnosisRouter;
   impl DiagnosisRouter {
       /// Route the highest-confidence category score to an intervention.
       pub fn route(&self, scores: &[CategoryScore]) -> RoutedDiagnosis {
           // confidence > 0.9: route directly to the category's default intervention
           // 0.6-0.9: route with needs_review = true
           // < 0.6: fallback to RetryWithContext
       }
   }

   #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
   pub struct RoutedDiagnosis {
       pub category: ErrorCategory,
       pub intervention: SuggestedIntervention,
       pub confidence: f64,
       pub needs_review: bool,
   }
   ```

5. Add a category-to-default-intervention mapping function:
   ```rust
   impl ErrorCategory {
       pub fn default_intervention(&self) -> SuggestedIntervention {
           match self {
               Self::CompileError | Self::TypeMismatch | Self::BorrowCheckerError
               | Self::LifetimeError | Self::ImportError => SuggestedIntervention::RetryWithContext,
               Self::ClippyWarning => SuggestedIntervention::AutoFix,
               Self::TestFailure => SuggestedIntervention::RetryWithContext,
               Self::GitConflict => SuggestedIntervention::MergeResolution,
               Self::DependencyError => SuggestedIntervention::RetryWithContext,
               Self::MissingFile | Self::PermissionDenied | Self::DiskFull => SuggestedIntervention::AbortPlan,
               Self::NetworkError | Self::TimeoutError | Self::LlmRateLimit => SuggestedIntervention::BackoffRetry,
               Self::LlmContextOverflow => SuggestedIntervention::ReduceContext,
               Self::LlmRefusal => SuggestedIntervention::SwitchModel,
               Self::OomError | Self::ProcessCrash => SuggestedIntervention::RestartAgent,
               Self::LoopDetected => SuggestedIntervention::RestartAgent,
           }
       }
   }
   ```

6. Wire the scorer and router together in `DiagnosisEngine`, preserving the existing `diagnose()` API:
   ```rust
   impl DiagnosisEngine {
       /// Two-stage scored diagnosis: score -> route.
       pub fn diagnose_scored(&self, text: &str) -> RoutedDiagnosis { ... }
   }
   ```

7. Add re-exports to `lib.rs`: `CategoryScore`, `PatternMatchScorer`, `DiagnosisRouter`, `RoutedDiagnosis`.

8. Add tests:
   - `PatternMatchScorer` groups patterns by category correctly
   - High-confidence routes directly (needs_review = false)
   - Low-confidence falls back to RetryWithContext
   - Unknown text scores below 0.6
   - `default_intervention()` returns expected values for each category

## Verification
```bash
cargo check -p roko-conductor
cargo clippy -p roko-conductor --no-deps -- -D warnings
cargo test -p roko-conductor -- diagnosis
cargo test -p roko-conductor -- category_score
cargo test -p roko-conductor -- routed
```

## What NOT to do
- Do NOT create separate `error_kinds.rs` or `intervention_kinds.rs` — the types already exist in `diagnosis.rs`
- Do NOT remove or rename existing `ErrorCategory` / `SuggestedIntervention` variants
- Do NOT remove existing `DiagnosisEngine::diagnose()` — add `diagnose_scored()` alongside
- Do NOT add LLM-based scoring — just substring pattern matching
- Do NOT wire into orchestrate.rs — that is integration work
