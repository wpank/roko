# 132 — `orchestrate.rs` Freeze and Retirement

**Priority**: P1 — Two parallel runtimes is the root cause of most architectural gaps; a freeze banner and call-site census is the prerequisite for safe retirement and eliminates the risk of new code being added to the legacy path.
**Size**: M (2-3 days)
**Crates**: `crates/roko-cli/src/` (orchestrate.rs and its consumers)
**Depends on**: #131 (PRD/cloud-worker migration must be done so orchestrate.rs has no active callers)
**Sources**: `tmp/backlog/_mori-diffs-gaps.md` §A-2 (suggested 112)

---

## Background

`orchestrate.rs` is the original roko execution engine — the predecessor of runner-v2. It is still exported from `lib.rs` and contains unique behavior that has not been fully ported: a replan ledger, knowledge helpers, and a bandit integration format. It is also used as a reference for some helper functions that are duplicated between orchestrate.rs and runner-v2.

Having two production runtimes is the root cause of many architectural gaps documented in the mori-diffs audit:
- Safety checks added to runner-v2 may not exist in orchestrate.rs.
- Learning hooks wired into runner-v2 may not exist in orchestrate.rs.
- Gate thresholds adaptive in runner-v2 are not adaptive in orchestrate.rs.
- Any caller that uses orchestrate.rs bypasses all these improvements.

The retirement plan is a four-phase process rather than a single deletion: (1) freeze (no new code), (2) audit call sites and unique behavior, (3) port unique behavior, (4) quarantine/delete.

## Current State

- `orchestrate.rs` — present in `crates/roko-cli/src/`; exported from `lib.rs`.
- Unique behavior relative to runner-v2 (needs audit): replan ledger, knowledge helpers, bandit format.
- Call sites: unknown (require grep); after #131 the PRD and cloud-worker call sites are gone.
- No "frozen" banner or CI guard exists.

## Implementation Plan

1. **Add freeze banner to `orchestrate.rs`**: At the top of the file, add:
   ```rust
   // FROZEN — 2026-08-19 — Do not add new code to this file.
   // This is the legacy orchestrator. All new execution goes through runner-v2 (runner/event_loop.rs).
   // Retirement tracking: tmp/backlog/132-orchestrate-rs-freeze-retirement.md
   // #[deprecated] attribute added to all public items below.
   ```
   Mark all exported functions with `#[deprecated(note = "Use runner-v2 event_loop.rs")]`.

2. **Call-site census**: Run `grep -rn 'orchestrate::\|use.*orchestrate' crates/ --include='*.rs'` and list every active call site (excluding the file itself). After #131, there should be zero external call sites.

3. **Audit unique behavior**: Read `orchestrate.rs` and identify:
   - The replan ledger: is it present in runner-v2? If not, create a tracking note.
   - Knowledge helpers: are they duplicated in `runner/knowledge.rs` or equivalent?
   - Bandit format: is `LearningRuntime::bandit_format()` or equivalent in runner-v2?

4. **Port unique behavior**: For any behavior in orchestrate.rs that is absent from runner-v2, port it. Each port is a small separate commit to keep the blame history clean.

5. **Quarantine**: Once all unique behavior is ported and zero call sites remain, rename the file to `orchestrate_retired.rs` and add `#[allow(dead_code)]` to suppress warnings. Do not delete immediately — preserve for reference for 30 days.

6. **CI guard**: Add a test that fails if `orchestrate.rs` is imported from any non-deprecated path:
   ```bash
   # CI: ensure orchestrate.rs has no live call sites
   count=$(grep -rn 'orchestrate::' crates/ --include='*.rs' | grep -v 'orchestrate.rs' | grep -v 'target/' | wc -l)
   if [ "$count" -gt 0 ]; then echo "ERROR: live orchestrate.rs call sites detected"; exit 1; fi
   ```

## Acceptance Criteria

1. `orchestrate.rs` has a freeze banner and all public items are `#[deprecated]`.
2. Zero call sites outside of `orchestrate.rs` itself (verified by CI grep).
3. Replan ledger, knowledge helpers, and bandit format are either ported to runner-v2 or documented as intentionally excluded.
4. `cargo check` produces deprecation warnings for any code importing `orchestrate` items.
5. CI guard fails on any new import of `orchestrate::` items.

## Verification Checklist

- [ ] `grep -rn 'orchestrate::' crates/ | grep -v 'orchestrate.rs' | grep -v target/` returns 0 lines.
- [ ] `cargo check -p roko-cli 2>&1 | grep 'deprecated.*orchestrate'` shows warnings for existing uses.
- [ ] Read `orchestrate.rs` and confirm freeze banner is present at the top.
- [ ] Run CI guard script and verify it passes with zero call sites.
- [ ] Add a deliberate import in a test file; verify CI guard catches it.

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/src/orchestrate.rs` | Add freeze banner; `#[deprecated]` on all public items |
| `crates/roko-cli/src/lib.rs` | Remove public re-export of `orchestrate` module |
| `tests/` or `.github/workflows/` | Add CI guard script for `orchestrate::` usage |
| `crates/roko-cli/src/runner/` | Port any unique orchestrate.rs behavior |
