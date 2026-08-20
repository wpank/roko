# 05 — Express Mode

**Priority**: P2 — removes a full strategist LLM call (~$1-3, 30-60s) for the most common gate failure patterns (format/lint/compile)
**Size**: M (2-3 days)
**Crates**: `crates/roko-gate/` (predicate methods), `crates/roko-cli/` (runner wiring)
**Depends on**: None

---

## Background

Roko's plan runner executes agent tasks in a pipeline: an implementer agent writes code, gate rungs validate it (compile, lint, test, etc.), and when gates fail the runner typically dispatches a "strategist" agent to analyze the failures and produce a revision plan before re-dispatching the implementer. The strategist phase exists to handle complex, non-trivial failures that require design judgment.

However, the most common gate failures in practice are mechanical: formatting violations caught by `cargo fmt`, clippy lint violations, or simple compile errors like missing semicolons or unused imports. For these failures, the strategist adds no value — it consumes one full LLM call (typically $1-3 and 30-60 seconds of wall-clock time) to produce a plan that the implementer could have derived directly from the raw gate error output.

Express mode is a phase-transition optimization: when every issue in a gate failure verdict is "quick-fixable" (belongs to a mechanical category), the runner skips the strategist dispatch entirely and routes directly to the implementer with structured gate feedback pre-injected into the prompt. This is purely a routing change — the implementer receives the same gate output, and the gate pipeline itself is unchanged.

## Current State

1. All required types already exist in `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/review_verdict.rs`. No new structs are needed.

2. `IssueCategory` enum is defined at line 25 with 10 variants: `CompileError`, `TestFailure`, `LintViolation`, `IncompleteImpl`, `SecurityIssue`, `PerformanceRegression`, `FormatViolation`, `SymbolMissing`, `IntegrationFailure`, `NeedsHumanReview`. It has no `is_quick_fixable()` method.

3. `ReviewVerdict` struct is defined at line 74 with fields `decision: ReviewDecision`, `issues: Vec<ReviewIssue>`, `blocking_count: usize`, `advisory_count: usize`, `rung_results: Vec<RungResult>`. It has no `all_issues_quick_fixable()` method.

4. `ReviewVerdict::from_verdicts()` is implemented at line 354. It maps gate rungs to `IssueCategory` values: rung 0 → `CompileError`, rung 1 → `LintViolation`, rungs 2/4/5 → `TestFailure`, rung 3 → `SymbolMissing`, rung 6+ → `NeedsHumanReview`. Note that `FormatViolation` is not currently emitted by `from_verdicts()` — it would only appear from a parsed agent reviewer output. The express-mode predicate must work correctly regardless.

5. `parse_structured_review_verdict()` is implemented at line 184 with the full fallback chain: raw JSON → fenced `json` block → fenced `toml` block → fail-closed (`NeedsHumanReview`). The fail-closed path ensures that unstructured reviewer text never accidentally triggers express mode.

6. The runner event loop in `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs` handles gate failures at lines 4394-4673. When a gate fails and `RetryDecision::should_retry()` is true, the runner calls `begin_gate_retry_rollover()` (line 4497) and `build_gate_retry_context()` (line 4467). There is no express-mode shortcut — the retry always uses the same path regardless of failure category.

7. The runner does NOT currently dispatch a separate strategist agent after gate failures. The existing retry path dispatches the implementer directly with `build_gate_retry_context()` output injected into the prompt. This means express mode in this codebase specifically means: using structured `ReviewVerdict` issue classification to select a richer, targeted retry prompt format vs. the current generic one, rather than skipping a strategist agent.

8. `ReviewIssue` struct at line 50 has a `category: IssueCategory` field and a `blocking: bool` field. Issues are constructed inside `from_verdicts()`.

## Implementation Plan

### Step 1: Add `IssueCategory::is_quick_fixable()` to `review_verdict.rs`

Add to `crates/roko-gate/src/review_verdict.rs` after the existing `IssueCategory` enum definition (after line 46):

```rust
impl IssueCategory {
    /// True when an implementer agent can fix this category mechanically,
    /// without requiring strategic planning or design decisions.
    /// Quick-fixable categories have deterministic fixes: compiler-suggested
    /// changes, clippy suggestions, and formatting corrections.
    #[must_use]
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

### Step 2: Add `ReviewVerdict::all_issues_quick_fixable()` to `review_verdict.rs`

Add to the existing `impl ReviewVerdict` block (after line 434):

```rust
impl ReviewVerdict {
    /// True when every issue in this verdict can be resolved without
    /// strategist involvement. Returns `false` on an empty issue list
    /// (no issues means Approve, not express mode). Returns `false`
    /// if any single issue is not quick-fixable.
    #[must_use]
    pub fn all_issues_quick_fixable(&self) -> bool {
        !self.issues.is_empty()
            && self.issues.iter().all(|i| i.category.is_quick_fixable())
    }
}
```

### Step 3: Use `ReviewVerdict` in the gate retry path in `event_loop.rs`

The gate failure handling path lives between lines 4394 and 4673 of `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs`. The relevant section is where `build_gate_retry_context()` is called at line 4467 to construct the retry prompt injected into the implementer's next attempt.

The change: construct a `ReviewVerdict` from the gate verdicts already present in `completion.verdicts` and, when all issues are quick-fixable, build a more structured retry prompt that explicitly lists the mechanical fixes needed. When issues are not all quick-fixable, fall back to the existing `build_gate_retry_context()` output unchanged.

In the gate-failure retry path (near line 4465-4475), replace the replan context construction:

```rust
// Before change:
replan_context: build_gate_retry_context(
    &completion.output,
    &state.agent_output,
    next_attempt,
),

// After change:
replan_context: {
    use roko_gate::review_verdict::ReviewVerdict;
    let rung_pairs: Vec<_> = completion
        .verdicts
        .iter()
        .map(|v| (v.rung, v.gate_name.as_str(), &v.verdict))
        .collect();
    let review = ReviewVerdict::from_verdicts(&rung_pairs);
    if review.all_issues_quick_fixable() {
        tracing::info!(
            task_id = %completion.task_id,
            issues = review.issues.len(),
            express_mode = true,
            "express mode: all gate issues are quick-fixable, using structured retry prompt"
        );
        build_express_retry_context(&review, &completion.output, next_attempt)
    } else {
        build_gate_retry_context(
            &completion.output,
            &state.agent_output,
            next_attempt,
        )
    }
},
```

Note: `completion.verdicts` contains `GateVerdictSummary` items defined in `crates/roko-cli/src/runner/types.rs`. Check what fields are available on `GateVerdictSummary` and `roko_core::Verdict` to confirm the mapping. The `ReviewVerdict::from_verdicts()` function signature is `fn from_verdicts(verdicts: &[(u8, &str, &roko_core::Verdict)]) -> Self`.

### Step 4: Add `build_express_retry_context()` in `event_loop.rs`

Add a new function near `build_gate_retry_context()` (around line 15323):

```rust
/// Build a structured retry prompt for express-mode gate failures.
///
/// Express mode applies when all issues are quick-fixable (compile errors,
/// lint violations, or format violations). The prompt explicitly lists each
/// issue with its gate, file location, and suggestion rather than relying on
/// the raw gate output excerpt.
fn build_express_retry_context(
    verdict: &roko_gate::review_verdict::ReviewVerdict,
    gate_output: &str,
    attempt_num: u32,
) -> String {
    let issue_lines = verdict
        .issues
        .iter()
        .map(|issue| {
            let location = match (&issue.file, &issue.line) {
                (Some(f), Some(l)) => format!(" ({f}:{l})"),
                (Some(f), None) => format!(" ({f})"),
                _ => String::new(),
            };
            let suggestion = issue
                .suggestion
                .as_deref()
                .map(|s| format!("\n   Fix: {s}"))
                .unwrap_or_default();
            format!("- [{:?}]{location}: {}{suggestion}", issue.category, issue.message)
        })
        .collect::<Vec<_>>()
        .join("\n");

    let gate_excerpt = if gate_output.len() > 2000 {
        &gate_output[..2000]
    } else {
        gate_output
    };

    format!(
        "## IMPORTANT: Your previous attempt failed (attempt {attempt_num})\n\n\
         All issues are mechanical and can be fixed without redesign:\n\n\
         {issue_lines}\n\n\
         ### Full gate output\n```\n{gate_excerpt}\n```\n\n\
         Apply the fixes above. Do not rewrite unrelated code."
    )
}
```

### Step 5: Add unit tests

Add to the `#[cfg(test)]` block in `crates/roko-gate/src/review_verdict.rs` (after the existing tests around line 436):

```rust
#[test]
fn quick_fixable_categories_are_correct() {
    assert!(IssueCategory::CompileError.is_quick_fixable());
    assert!(IssueCategory::LintViolation.is_quick_fixable());
    assert!(IssueCategory::FormatViolation.is_quick_fixable());

    assert!(!IssueCategory::TestFailure.is_quick_fixable());
    assert!(!IssueCategory::IncompleteImpl.is_quick_fixable());
    assert!(!IssueCategory::SecurityIssue.is_quick_fixable());
    assert!(!IssueCategory::PerformanceRegression.is_quick_fixable());
    assert!(!IssueCategory::SymbolMissing.is_quick_fixable());
    assert!(!IssueCategory::IntegrationFailure.is_quick_fixable());
    assert!(!IssueCategory::NeedsHumanReview.is_quick_fixable());
}

#[test]
fn all_issues_quick_fixable_requires_nonempty_issues() {
    // Empty verdict (all passed) should NOT be quick-fixable.
    let compile = Verdict::pass("compile");
    let verdicts = vec![(0u8, "compile", &compile)];
    let review = ReviewVerdict::from_verdicts(&verdicts);
    assert!(!review.all_issues_quick_fixable(), "no issues → not quick-fixable");
}

#[test]
fn all_issues_quick_fixable_true_when_only_lint_and_compile() {
    let compile = Verdict::fail("compile", "error[E0433]: unresolved import");
    let lint = Verdict::fail("clippy", "warning: unused variable");
    let verdicts = vec![(0u8, "compile", &compile), (1u8, "clippy", &lint)];
    let review = ReviewVerdict::from_verdicts(&verdicts);
    // rung 0 → CompileError (quick-fixable), rung 1 → LintViolation (quick-fixable)
    assert!(review.all_issues_quick_fixable());
}

#[test]
fn all_issues_quick_fixable_false_when_test_fails() {
    let compile = Verdict::fail("compile", "error[E0433]: unresolved import");
    let test = Verdict::fail("test", "assertion failed");
    let verdicts = vec![(0u8, "compile", &compile), (2u8, "test", &test)];
    let review = ReviewVerdict::from_verdicts(&verdicts);
    // rung 2 → TestFailure (NOT quick-fixable)
    assert!(!review.all_issues_quick_fixable());
}

#[test]
fn fail_closed_verdict_is_not_quick_fixable() {
    // A FailClosed parse result produces NeedsHumanReview, which is not quick-fixable.
    // Simulate: build a verdict where rung >= 6 → NeedsHumanReview.
    let judge = Verdict::fail("llm_judge", "needs manual review");
    let verdicts = vec![(6u8, "llm_judge", &judge)];
    let review = ReviewVerdict::from_verdicts(&verdicts);
    assert!(!review.all_issues_quick_fixable());
}
```

## Acceptance Criteria

1. `IssueCategory::is_quick_fixable()` returns `true` for `CompileError`, `LintViolation`, and `FormatViolation`; returns `false` for all other variants: `TestFailure`, `IncompleteImpl`, `SecurityIssue`, `PerformanceRegression`, `SymbolMissing`, `IntegrationFailure`, `NeedsHumanReview`.

2. `ReviewVerdict::all_issues_quick_fixable()` returns `false` for an empty issue list; returns `true` only when every issue's category satisfies `is_quick_fixable()`; returns `false` if any single issue is not quick-fixable.

3. `build_express_retry_context()` produces a prompt beginning with `"## IMPORTANT: Your previous attempt failed"` that lists each issue with category, location, message, and suggestion; ends with a directive not to rewrite unrelated code.

4. When a gate fails with only `CompileError` and/or `LintViolation` issues, the runner emits a `tracing::info!` span with fields `express_mode = true` and `issues` (count), and uses `build_express_retry_context()` for the retry prompt.

5. When a gate fails with any `TestFailure`, `IncompleteImpl`, or other non-quick-fixable issue, the runner uses `build_gate_retry_context()` unchanged (express mode does not apply).

6. `cargo test -p roko-gate` passes with the new unit tests added to `review_verdict.rs`.

7. `cargo test --workspace` passes with no regressions.

## Verification Checklist

- [ ] `cargo test -p roko-gate -- quick_fixable` runs and passes all five new tests
- [ ] `cargo clippy -p roko-gate -- -D warnings` passes clean
- [ ] `cargo clippy -p roko-cli -- -D warnings` passes clean after wiring in event_loop.rs
- [ ] `cargo +nightly fmt --all` produces no diff
- [ ] Run `cargo run -p roko-cli -- plan run plans/ --engine runner-v2` against a plan with known lint failures; confirm `express_mode = true` appears in runner log output (stdout with `RUST_LOG=roko_cli=info`)
- [ ] Run against a plan with test failures; confirm `express_mode = true` does NOT appear in logs
- [ ] Check that retry prompt in the `express_mode = true` case begins with "All issues are mechanical" rather than "Your previous attempt failed" followed by an error analysis section

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/review_verdict.rs` | Add `IssueCategory::is_quick_fixable()` impl block after line 46; add `ReviewVerdict::all_issues_quick_fixable()` to existing `impl ReviewVerdict` after line 434; add 5 new unit tests in `#[cfg(test)]` block |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs` | Add `build_express_retry_context()` function near `build_gate_retry_context()` (~line 15323); modify the `replan_context` construction in the gate-failure retry path (~line 4467) to use express mode when `all_issues_quick_fixable()` is true |
