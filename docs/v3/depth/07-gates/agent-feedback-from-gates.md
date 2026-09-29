# Agent Feedback from Gates

> Depth file for [07-GATES.md](../../07-GATES.md) section 10.
> Source: `crates/roko-gate/src/feedback.rs`

---

## 1. Overview

Raw gate output -- compiler stderr, test logs, linter JSON -- is verbose
and full of noise. Progress bars, download messages, Cargo metadata all
waste agent context tokens. The feedback module parses raw output into
structured `GateFeedback` containing only actionable items classified by
severity.

This is the bridge between the gate layer (where gates run) and the
agent's context window.

---

## 2. The GateFeedback Type

```rust
pub struct GateFeedback {
    pub rung: u8,                  // Which rung produced this feedback
    pub passed: bool,              // Whether the gate passed
    pub errors: Vec<String>,       // Error-level items (must fix)
    pub warnings: Vec<String>,     // Warning-level items (should fix)
    pub suggestions: Vec<String>,  // Informational/help items
}
```

Three severity buckets, ordered from most to least critical:

- **errors:** Compilation errors, test failures, panics. The agent *must*
  fix these.
- **warnings:** Unused variables, deprecated usage, style issues. The
  agent *should* fix these.
- **suggestions:** Compiler help messages, notes, file location pointers.
  These *inform* the agent about what to do.

### Helper Methods

```rust
impl GateFeedback {
    pub fn item_count(&self) -> usize;    // Total across all categories
    pub fn is_empty(&self) -> bool;        // True if no actionable items
    pub fn items(&self) -> Vec<FeedbackItem>; // All items, errors first
}
```

`items()` returns all feedback ordered: errors first, then warnings, then
suggestions. Most critical information appears first in the agent's context.

---

## 3. The Classification Pipeline

### 3.1 Per-Line Classification

```rust
fn classify_line(line: &str) -> Option<(Severity, &str)>
```

Each line of raw output is classified via a priority chain:

1. **Empty/whitespace** -> `None`
2. **Noise patterns** -> `None`
3. **Error patterns** -> `Some(Severity::Error, line)`
4. **Warning patterns** -> `Some(Severity::Warning, line)`
5. **Suggestion patterns** -> `Some(Severity::Info, line)`
6. **Anything else** -> `None` (dropped as noise)

### 3.2 Noise Detection

Lines that are pure noise -- no actionable information:

| Pattern | Example |
|---------|---------|
| Cargo progress | `Downloading`, `Downloaded`, `Compiling`, `Checking`, `Finished`, `Running`, `Fresh`, `Packaging` |
| npm deprecation | `npm WARN deprecated stable@0.1.0` |
| Progress bars | Lines containing Unicode bar characters |

### 3.3 Error Detection

| Pattern | What It Catches |
|---------|-----------------|
| `error` (starts with) | Rust `error:` messages |
| `Error:` (starts with) | Generic error format |
| `ERROR:` (starts with) | Uppercase error format |
| `FAILED` (starts with) | Test failure markers |
| `FAIL ` (starts with) | Go test failure |
| Contains `error[E` | Rustc error codes (`error[E0425]`) |
| Contains `panicked at` | Panic messages |
| `thread '...' panicked` | Thread panic with test name |

### 3.4 Warning Detection

| Pattern | What It Catches |
|---------|-----------------|
| `warning` (starts with) | Rust warnings |
| `Warning:` (starts with) | Generic warning format |
| `WARNING:` (starts with) | Uppercase warning format |
| `warn[` (starts with) | Clippy warning codes |

### 3.5 Suggestion Detection

| Pattern | What It Catches |
|---------|-----------------|
| `help:` (starts with) | Compiler help messages |
| Contains `= help:` | Inline help annotations |
| `note:` (starts with) | Compiler notes |
| Contains `= note:` | Inline note annotations |
| `suggestion:` (starts with) | Explicit suggestions |
| `hint:` (starts with) | Hint messages |
| `-->` (starts with/contains) | Source location pointers |

---

## 4. The Public API

```rust
pub fn feedback_for_agent(gate_output: &str, rung: u8) -> GateFeedback
```

Takes raw gate output (stdout + stderr concatenated) and a rung number,
returns structured feedback.

### Algorithm

```
for each line in gate_output:
    classify_line(line)
    match severity:
        Error   -> push to errors, set has_errors = true
        Warning -> push to warnings
        Info    -> push to suggestions
        None    -> skip (noise)

passed = !has_errors
return GateFeedback { rung, passed, errors, warnings, suggestions }
```

### Pass Detection

The feedback's `passed` field is based purely on whether any error-level
items were found. Warnings without errors means `passed = true`, aligning
with the convention that warnings do not block compilation or test
execution.

---

## 5. Token Economy

The feedback module is a critical part of Roko's token economy:

### 5.1 A Typical cargo check Failure

| Category | Lines |
|----------|-------|
| Raw output | ~2,000 lines |
| Noise (Downloading, Compiling, etc.) | ~1,500 lines |
| Errors | ~10 lines |
| Warnings | ~20 lines |
| Suggestions | ~15 lines |
| **Filtered feedback** | **~45 lines** |

That is a **97.75% reduction.** At ~4 tokens per line, the feedback saves
~7,800 tokens per gate failure.

### 5.2 Compound Savings

Over a 5-attempt retry loop with 3 gate failures, savings total ~23,400
tokens -- a significant fraction of the agent's context window. For plans
with 50+ tasks, this scales to hundreds of thousands of tokens saved.

---

## 6. How the Orchestrator Uses Feedback

After a gate failure, the orchestrator generates feedback and injects it
into the agent's retry prompt:

```rust
let verdict = pipeline.verify(signal, ctx).await;

if !verdict.passed {
    let feedback = feedback_for_agent(
        verdict.detail.as_deref().unwrap_or(""),
        current_rung,
    );

    let retry_context = format!(
        "Your previous attempt failed at rung {}.\n\
         Errors ({}):\n{}\n\
         Warnings ({}):\n{}\n\
         Suggestions ({}):\n{}",
        feedback.rung,
        feedback.errors.len(), feedback.errors.join("\n"),
        feedback.warnings.len(), feedback.warnings.join("\n"),
        feedback.suggestions.len(), feedback.suggestions.join("\n"),
    );
}
```

This pattern:

1. Extracts only actionable information from thousands of lines
2. Categorizes by severity so the agent knows what to prioritize
3. Preserves source location pointers (the `-->` lines) for targeting

---

## 7. Severity Ordering

```rust
pub enum Severity {
    Info,       // 0
    Warning,    // 1
    Error,      // 2
}
```

`Severity` derives `PartialOrd` and `Ord`, with `Info < Warning < Error`.
This enables:

- Sorting feedback items by severity
- Filtering by minimum severity
- Threshold-based policies

---

## 8. The Gate-to-Scaffold Feedback Loop

The feedback module is one half of a closed loop:

```
Agent receives prompt (with sections)
    |
Agent produces code
    |
Gate verifies code -> Verdict
    |
feedback_for_agent() -> GateFeedback
    |
Two consumers:
    1. Agent retry prompt (immediate)
    2. Section effectiveness tracking (learning)
       -> Which prompt sections correlated with gate success?
       -> Adjust section priorities for future prompts
```

The immediate feedback operates at machine speed. The section effectiveness
learning operates at consolidation speed -- 50+ observations before
statistical claims.

---

## 9. Serde Support

Both `GateFeedback` and `FeedbackItem` derive `Serialize` and `Deserialize`:

- Persisting feedback to the episode log
- Transmitting feedback as JSON to agents expecting structured input
- Aggregating feedback across executions for learning

---

## 10. Limitations and Future Work

### 10.1 Language-Specific Heuristics

The classifier is biased toward Rust/Cargo output. Patterns for npm, Go,
and other build systems are minimal. Future: per-language classifiers.

### 10.2 Multi-Line Error Messages

Rustc error messages span multiple lines. The current classifier treats
each line independently, losing visual structure. Future: group consecutive
lines belonging to the same diagnostic.

### 10.3 Structured Error Formats

Some tools emit structured output (JSON, SARIF). The current classifier
works line-by-line. Future: detect and parse structured formats.

---

## 11. Testing

| Test | What It Verifies |
|------|------------------|
| `feedback_empty_output_passes` | Empty input -> passed, no items |
| `feedback_extracts_errors` | Error lines extracted correctly |
| `feedback_extracts_warnings` | Warning lines extracted correctly |
| `feedback_extracts_suggestions` | Help/note lines extracted correctly |
| `feedback_filters_noise` | Cargo progress lines dropped |
| `feedback_mixed_output` | Mixed output classified correctly |
| `feedback_item_count` | Total count across categories |
| `feedback_items_ordering` | Errors first, then warnings, then suggestions |
| `feedback_rung_preserved` | Rung number roundtrips correctly |
| `feedback_test_failure_detected` | FAILED/panicked lines are errors |
| `feedback_severity_ordering` | Info < Warning < Error |
| `feedback_npm_deprecation_is_noise` | npm-specific noise detected |
| `feedback_progress_bars_are_noise` | Unicode progress bars detected |
| `feedback_serde_roundtrip` | JSON serialization preserves all fields |

---

## Verification

```bash
cargo test -p roko-gate -- feedback
```
