# Backlog Spec: Express Mode (Skip Strategist for Trivial Fixes)

**Status**: Backlog
**Priority**: P2 — optimization for common trivial-fix scenarios
**Size**: M (2-3 days)
**Origin**: `tmp/architecture-archive/20-orchestrator-gaps.md`, Gap 1 (lines 30-58)
**Related types**: `crates/roko-gate/src/review_verdict.rs`
**Wire target**: `crates/roko-cli/src/runner/event_loop.rs`

---

## Problem Statement

Every gate failure — regardless of how trivial the underlying issue is — goes through the
full strategist → implementer pipeline. A task whose only failing issues are formatting
violations, unused imports, or doc-comment style follows the same expensive path as a task
with a genuine architectural problem or failing test suite.

The strategist phase exists to handle non-trivial design decisions. For mechanical fixes
(style, lint, formatting, unused bindings, doc corrections), the strategist adds no value:
it consumes one full LLM call ($1–3 and 30–60s of wall-clock time) to produce a plan
that the implementer could have generated directly from the raw gate feedback.

At P2 priority this is the most accessible performance win in the runner because trivial
failures are the most common gate failure mode in practice (style/lint/format failures
from `cargo fmt`, `cargo clippy`, or doc-comment checks).

---

## Existing Types (do NOT rebuild)

All required types already exist in `crates/roko-gate/src/review_verdict.rs`. The gap is
entirely in wiring and in adding a small predicate that maps the existing
`IssueCategory` enum values to the quick-fixable classification.

| Type | File | Status |
|------|------|--------|
| `ReviewDecision` | `review_verdict.rs:13` | EXISTS — `Approve | Revise | Skip` |
| `ReviewIssue` | `review_verdict.rs:50` | EXISTS — has `category: IssueCategory`, `blocking: bool` |
| `IssueCategory` | `review_verdict.rs:23` | EXISTS — 10 variants |
| `ReviewVerdict` | `review_verdict.rs:74` | EXISTS — `decision`, `issues`, `blocking_count` |
| `ParsedReviewVerdict` | `review_verdict.rs:137` | EXISTS — result of `parse_structured_review_verdict()` |
| `ReviewParseSource` | `review_verdict.rs:123` | EXISTS — tracks JSON / JsonCodeBlock / TomlCodeBlock / FailClosed |
| `parse_structured_review_verdict()` | `review_verdict.rs:184` | EXISTS — full fallback chain implemented |

The `IssueCategory` enum currently has these variants:

```rust
pub enum IssueCategory {
    CompileError,       // <- quick-fixable
    TestFailure,        // NOT quick-fixable
    LintViolation,      // <- quick-fixable
    IncompleteImpl,     // NOT quick-fixable
    SecurityIssue,      // NOT quick-fixable
    PerformanceRegression, // NOT quick-fixable
    FormatViolation,    // <- quick-fixable
    SymbolMissing,      // NOT quick-fixable
    IntegrationFailure, // NOT quick-fixable
    NeedsHumanReview,   // NOT quick-fixable
}
```

Note: the gap document's original spec listed `Docs`, `Style`, and `Unused` as separate
categories. In the actual codebase these map to `FormatViolation` (docs/style) and
`LintViolation` (unused imports/variables via clippy). The quick-fixable predicate should
use the actual variant names above.

---

## Proposed Solution

After the gate pipeline runs and the reviewer produces output, parse the output into a
`ReviewVerdict`. If every issue in that verdict is quick-fixable, skip the strategist
agent entirely and dispatch the implementer directly with the gate feedback as context.

This is purely a phase-transition optimization. It does not change what the implementer
receives — it only removes the strategist detour.

### Quick-fixable categories

An issue is quick-fixable when an implementer agent can resolve it mechanically without
strategic planning:

| `IssueCategory` | Quick-fixable | Rationale |
|-----------------|---------------|-----------|
| `CompileError` | Yes | Import additions, syntax fixes, missing semicolons are deterministic |
| `LintViolation` | Yes | Clippy suggestions are mechanical; unused imports/variables have known fixes |
| `FormatViolation` | Yes | `cargo fmt` / doc-comment style are mechanical |
| `TestFailure` | **No** | Tests can reveal deeper semantic bugs requiring design decisions |
| `TypeMismatch` | **No** | May require API surface changes that affect callers |
| `IncompleteImpl` | **No** | Stub implementations need architectural guidance |
| `SecurityIssue` | **No** | Security flaws need careful review, not expedited fixes |
| `PerformanceRegression` | **No** | Regressions may require profiling and design trade-offs |
| `SymbolMissing` | **No** | Missing public symbols may indicate a wider API contract gap |
| `IntegrationFailure` | **No** | Cross-crate failures can have far-reaching root causes |
| `NeedsHumanReview` | **No** | Escalated by LLM judge for a reason |

The invariant: if `all_issues_quick_fixable()` returns `true`, every single issue in the
verdict can be resolved by applying a known mechanical transformation. No issue belongs
to a "needs design" category.

### What needs to be built

Two pieces:

**1. `IssueCategory::is_quick_fixable()` predicate** — add to `review_verdict.rs`:

```rust
impl IssueCategory {
    /// True when an implementer agent can fix this category without
    /// strategic planning.  Quick-fixable categories are mechanical:
    /// compiler-suggested changes, lint suppressions, and format corrections.
    pub fn is_quick_fixable(&self) -> bool {
        matches!(
            self,
            IssueCategory::CompileError
                | IssueCategory::LintViolation
                | IssueCategory::FormatViolation
        )
    }
}
```

**2. `ReviewVerdict::all_issues_quick_fixable()` method** — add to `review_verdict.rs`:

```rust
impl ReviewVerdict {
    /// True when every issue in this verdict can be resolved without
    /// strategist involvement.  Returns `false` on an empty issue list
    /// (no issues means Approve, not express mode).
    pub fn all_issues_quick_fixable(&self) -> bool {
        !self.issues.is_empty()
            && self.issues.iter().all(|i| i.category.is_quick_fixable())
    }
}
```

**3. Express-mode phase transition in `event_loop.rs`** — after a gate failure produces a
`ReviewVerdict` with `decision == ReviewDecision::Revise`, inspect the verdict before
dispatching the strategist:

```rust
// In the gate-failure handling path, before strategist dispatch:
let verdict = ReviewVerdict::from_verdicts(&gate_results);
if verdict.all_issues_quick_fixable() {
    tracing::info!(
        task_id = %task_id,
        issues = verdict.issues.len(),
        "express mode: skipping strategist, all issues quick-fixable"
    );
    // Dispatch implementer directly with verdict feedback injected into prompt.
    dispatch_express_implementer(&verdict, &task, &run_config, ...).await?;
} else {
    // Normal path: strategist → implementer.
    dispatch_strategist(...).await?;
}
```

The express implementer dispatch follows the same code path as a normal implementer
dispatch, with one addition: the structured verdict's issues are serialized as a
`## Gate Feedback` section prepended to the task prompt.

---

## Parsing Fallback Chain

The existing `parse_structured_review_verdict()` function in `review_verdict.rs:184`
already implements the full fallback chain required. It is not necessary to write a new
parser. The chain is:

1. Attempt to parse the entire reviewer output as a raw JSON object.
2. Extract and parse a fenced ` ```json ` code block.
3. Extract and parse a fenced ` ```toml ` code block.
4. Fail closed: return `ParsedReviewVerdict` with `source: FailClosed`,
   `status: NeedsHuman`, `required_next_action: Human`.

The fail-closed path means express mode is never accidentally entered on a parse failure:
`NeedsHumanReview` is not a quick-fixable category, so `all_issues_quick_fixable()`
returns `false` and the runner falls through to the normal strategist path.

When the runner drives the review from gate verdicts directly (not from an agent reviewer's
text output), use `ReviewVerdict::from_verdicts()` which is already implemented and maps
rung indices to `IssueCategory` values without any parsing step.

---

## Implementation Plan

### Day 1 — Predicate and method

- Add `IssueCategory::is_quick_fixable()` to `review_verdict.rs`.
- Add `ReviewVerdict::all_issues_quick_fixable()` to `review_verdict.rs`.
- Write unit tests (see Acceptance Criteria below).
- Confirm that `ReviewVerdict::from_verdicts()` produces the correct `IssueCategory`
  values for the three quick-fixable variants.

### Day 2 — Runner wiring

- Locate the gate-failure branch in `event_loop.rs` where the strategist is currently
  dispatched unconditionally.
- Insert the express-mode check between the gate failure and the strategist dispatch.
- Wire the structured verdict into the implementer prompt as a `## Gate Feedback` section.
- Add a `tracing::info!` span (`express_mode = true/false`) for observability.
- Confirm no regressions by running the existing runner integration tests.

### Day 3 (buffer) — Integration test and tuning

- Write an integration test that supplies a mock gate output containing only
  `LintViolation` and `FormatViolation` issues and asserts the strategist agent is
  never dispatched.
- Write a complementary test with a mixed set (one `TestFailure` + one `FormatViolation`)
  and assert the strategist IS dispatched.
- Check timing: confirm the fast path does not add latency vs. the strategist path
  (any overhead must be sub-millisecond — only a couple of method calls and a log line).

---

## Acceptance Criteria

1. `IssueCategory::is_quick_fixable()` returns `true` for `CompileError`,
   `LintViolation`, and `FormatViolation`; returns `false` for all other variants
   including `TestFailure`, `IncompleteImpl`, `SecurityIssue`, `PerformanceRegression`,
   `SymbolMissing`, `IntegrationFailure`, and `NeedsHumanReview`.

2. `ReviewVerdict::all_issues_quick_fixable()` returns `true` only when the `issues`
   list is non-empty and every issue satisfies `is_quick_fixable()`; returns `false` for
   an empty issue list, for a list with any non-quick-fixable issue, and for any
   `FailClosed` parse result.

3. In `event_loop.rs`, a gate failure where all issues are quick-fixable dispatches the
   implementer directly without calling the strategist agent; verified by confirming the
   strategist spawn never occurs in the integration test.

4. In `event_loop.rs`, a gate failure with at least one non-quick-fixable issue
   dispatches the strategist as before; express mode does not affect the normal path.

5. A `FailClosed` parse result (unstructured reviewer text) never triggers express mode;
   the runner falls through to the strategist path safely.

6. The express-mode phase transition emits a `tracing::info!` span with structured fields
   `task_id`, `issues` (count), and `express_mode = true` so the decision is visible in
   runner logs and the TUI dashboard without additional tooling.

---

## Out of Scope

- **Auto-fix via `cargo fix`** — Gap 2 in the same source document. Overlapping concern
  but a separate feature. Express mode skips the strategist; auto-fix would skip the
  implementer agent entirely. Do not conflate them.
- **Warm agent pre-spawning** — Gap 6. Could be combined with express mode to further
  reduce latency, but is independently tracked.
- **Reflection loop** — Gap 4. Express mode does not interact with post-gate reflection.
- **Changing what the implementer receives** — Express mode only changes the phase
  transition logic. The implementer prompt construction, gate pipeline, and persistence
  layer are unchanged.

---

## References

- `tmp/architecture-archive/20-orchestrator-gaps.md` lines 30–58 (Gap 1: Structured
  review verdict system)
- `tmp/architecture-archive/20-orchestrator-gaps.md` lines 463–499 (Spec clarifications:
  parsing fallback chain, `is_quick_fixable()` categories)
- `crates/roko-gate/src/review_verdict.rs` — all existing types and `parse_structured_review_verdict()`
- `crates/roko-cli/src/runner/event_loop.rs` — phase transition wire target
- `crates/roko-gate/src/compile_errors.rs` — `ErrorCategory` for cross-reference with
  gate-level categories vs. review-level `IssueCategory`
