# Diagnosis Engine -- 34 Patterns, 20 Error Categories

> Depth file for [30-CONDUCTOR.md](../../30-CONDUCTOR.md) section 5.
> Source: `crates/roko-conductor/src/diagnosis.rs`

---

## 1. Purpose

The Diagnosis Engine replaces ad-hoc error parsing with structured classification.
Instead of each component grepping for "error[E0308]" in raw output, the engine
accepts raw error text and returns a typed `Diagnosis` with:

- Error category (one of 20 enumerated types)
- Confidence score (0.0 to 1.0)
- Suggested intervention (one of 9 actions)
- Description and context

This structured classification enables consistent handling, appropriate response
routing, learning system integration, and actionable observability.

---

## 2. Error Categories

The `ErrorCategory` enum defines twenty categories covering the full range of errors
encountered in production batch runs:

```rust
pub enum ErrorCategory {
    CompileError,
    TestFailure,
    TypeMismatch,
    BorrowCheckerError,
    LifetimeError,
    ImportError,
    MissingFile,
    PermissionDenied,
    NetworkError,
    TimeoutError,
    OomError,
    DiskFull,
    LlmRateLimit,
    LlmContextOverflow,
    LlmRefusal,
    ProcessCrash,
    LoopDetected,
    ClippyWarning,
    GitConflict,
    DependencyError,
}
```

### 2.1 Category Groupings

**Rust compiler errors** (6): CompileError, TypeMismatch, BorrowCheckerError,
LifetimeError, ImportError, ClippyWarning

**Test and verification** (1): TestFailure

**File system** (3): MissingFile, PermissionDenied, DiskFull

**Infrastructure** (3): NetworkError, TimeoutError, OomError

**LLM provider** (3): LlmRateLimit, LlmContextOverflow, LlmRefusal

**Process** (2): ProcessCrash, LoopDetected

**Version control** (1): GitConflict

**Dependencies** (1): DependencyError

---

## 3. Suggested Interventions

Each diagnosis maps to one of nine intervention actions:

```rust
pub enum SuggestedIntervention {
    RetryWithContext,
    AutoFix,
    RestartAgent,
    AbortPlan,
    BackoffRetry,
    MergeResolution,
    ReduceContext,
    SwitchModel,
    WarnAndContinue,
}
```

### 3.1 Intervention Semantics

| Intervention | When Used | What Happens |
|-------------|-----------|-------------|
| `RetryWithContext` | Transient errors | Retry with additional error context in prompt |
| `AutoFix` | Simple, well-understood errors | Route to cheap Haiku-tier auto-fix agent |
| `RestartAgent` | Agent confused or stuck | Kill and respawn with fresh context |
| `AbortPlan` | Unrecoverable errors | Mark plan as failed immediately |
| `BackoffRetry` | Rate limits, outages | Wait with exponential backoff, then retry |
| `MergeResolution` | Git merge conflicts | Spawn merge resolver agent |
| `ReduceContext` | Context overflow | Compact context and retry with smaller prompt |
| `SwitchModel` | Model limitation | Route to a different model |
| `WarnAndContinue` | Non-blocking issues | Log warning, do not interrupt |

### 3.2 Category-to-Intervention Mapping

| Category | Primary Intervention | Rationale |
|----------|---------------------|-----------|
| CompileError | RetryWithContext | Agent may fix with error details |
| TestFailure | RetryWithContext | Agent may fix with test output |
| TypeMismatch | RetryWithContext | Agent needs expected/found types |
| BorrowCheckerError | RestartAgent | Requires fresh approach |
| LifetimeError | RestartAgent | Structurally difficult |
| ImportError | AutoFix | Missing imports are cheap to fix |
| MissingFile | RetryWithContext | Agent may need to create the file |
| PermissionDenied | AbortPlan | Cannot fix from agent context |
| NetworkError | BackoffRetry | Likely transient |
| TimeoutError | BackoffRetry | Likely transient |
| OomError | AbortPlan | Resource exhaustion, operator action |
| DiskFull | AbortPlan | Resource exhaustion, cleanup needed |
| LlmRateLimit | BackoffRetry | Wait for rate limit window |
| LlmContextOverflow | ReduceContext | Compact and retry |
| LlmRefusal | SwitchModel | Try a different model |
| ProcessCrash | RestartAgent | Agent died; respawn |
| LoopDetected | RestartAgent | Agent is stuck; fresh context |
| ClippyWarning | WarnAndContinue | Non-blocking |
| GitConflict | MergeResolution | Spawn merge resolver |
| DependencyError | RetryWithContext | Agent may fix with dependency info |

---

## 4. Pattern Matching

The engine contains 34 built-in patterns. Each pattern is a substring match with an
associated category, confidence, and suggested intervention:

```rust
struct ErrorPattern {
    substring: &'static str,
    category: ErrorCategory,
    confidence: f64,
    intervention: SuggestedIntervention,
}
```

### 4.1 Representative Patterns

| Pattern Substring | Category | Confidence | Intervention |
|------------------|----------|-----------|-------------|
| `"error[E0308]"` | TypeMismatch | 0.95 | RetryWithContext |
| `"error[E0382]"` | BorrowCheckerError | 0.95 | RestartAgent |
| `"error[E0106]"` | LifetimeError | 0.95 | RestartAgent |
| `"error[E0432]"` | ImportError | 0.95 | AutoFix |
| `"error[E0433]"` | ImportError | 0.95 | AutoFix |
| `"error[E0063]"` | CompileError | 0.90 | AutoFix |
| `"cannot find"` | ImportError | 0.70 | RetryWithContext |
| `"test result: FAILED"` | TestFailure | 0.90 | RetryWithContext |
| `"panicked at"` | TestFailure | 0.85 | RetryWithContext |
| `"Connection refused"` | NetworkError | 0.80 | BackoffRetry |
| `"rate limit"` | LlmRateLimit | 0.90 | BackoffRetry |
| `"context_length_exceeded"` | LlmContextOverflow | 0.95 | ReduceContext |
| `"No space left"` | DiskFull | 0.95 | AbortPlan |
| `"CONFLICT"` | GitConflict | 0.80 | MergeResolution |
| `"clippy::"` | ClippyWarning | 0.90 | WarnAndContinue |

Rust error codes (E0308, E0382) are highly specific -- confidence 0.95. Generic
substrings ("cannot find") are less specific -- confidence 0.70.

### 4.2 Matching Algorithm

```rust
impl DiagnosisEngine {
    pub fn diagnose(&self, error_text: &str) -> Vec<Diagnosis> {
        let lower = error_text.to_lowercase();
        self.patterns.iter()
            .filter(|p| lower.contains(&p.substring.to_lowercase()))
            .map(|p| Diagnosis {
                category: p.category,
                confidence: p.confidence,
                intervention: p.intervention,
                description: format!("Matched pattern: {}", p.substring),
            })
            .collect()
    }
}
```

Multiple patterns can match the same error text. The caller receives all matching
diagnoses and can select the highest-confidence one.

---

## 5. Integration Points

### 5.1 With the Conductor

When a watcher fires, the Conductor passes error context through the Diagnosis
Engine before making its decision. This enriches the intervention signal with
structured classification.

### 5.2 With the Auto-Fix Pipeline

The `AutoFix` intervention routes errors to a lightweight Haiku-tier agent. The cost
difference is significant: an auto-fix costs ~$0.01 (Haiku, small context). A full
re-implementation cycle costs ~$2.00+ (Opus, full context).

### 5.3 With the Learning System

Error categories feed into the efficiency tracking system, enabling per-category
success rate analysis over time.

---

## 6. Production Error Distribution

| Category | Frequency | Auto-Fix Rate |
|----------|----------|--------------|
| ImportError | 35% | 95% |
| CompileError (general) | 20% | 30% |
| TypeMismatch | 15% | 50% |
| TestFailure | 12% | 0% (requires understanding) |
| BorrowCheckerError | 5% | 10% |
| LifetimeError | 4% | 10% |
| LlmRateLimit | 3% | N/A (retry) |
| All others | 6% | varies |

The key insight: over a third of all errors are import errors, and 95% of those can
be auto-fixed for $0.01 each. Without the diagnosis engine, all errors go through
full re-implementation at $2+ each.

---

## 7. Design Decisions

### 7.1 Why Substring Matching Instead of Regex

Simple substring matching (`contains()`), not regular expressions:

1. **Performance**: O(n) per pattern, O(n*m) for all patterns
2. **Readability**: `"error[E0308]"` is immediately clear
3. **Maintainability**: Adding a pattern is adding a string literal
4. **Coverage**: 34 patterns cover ~95% of observed errors

### 7.2 Why 34 Patterns

Derived from production data during batch runs in March-April 2026. The
`has_at_least_20_patterns()` test ensures the pattern set is not accidentally
reduced.

### 7.3 Why 20 Categories

Balances granularity against complexity. Each category maps to a different
intervention strategy -- fewer would lose actionable information, more would create
maintenance burden without proportional benefit.

---

## 8. File Reference

| File | What |
|------|------|
| `crates/roko-conductor/src/diagnosis.rs` | DiagnosisEngine, ErrorCategory, SuggestedIntervention, 34 patterns |
