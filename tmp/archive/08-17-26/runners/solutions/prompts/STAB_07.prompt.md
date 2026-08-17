# STAB_07: Fix stub gate verdicts giving false PASS

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-07`](../ISSUE-TRACKER.md#stab-07)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.07
- Priority: **P0**
- Effort: 2 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_07 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`stub_verdict()` at line 132 of `rung_dispatch.rs`:
```rust
fn stub_verdict(gate: &str, detail: impl Into<String>) -> Verdict {
    let message = format!("stub gate; {}", detail.into());
    let mut verdict = Verdict::pass(gate.to_string());
    // ... sets detail
}
```
This returns a PASSING verdict when a gate cannot run (no oracle, no manifest, etc.).
Called from 8 locations (lines 146, 149, 173, 186, 201, 204, 220, 223, 237).

The `Verdict` struct in `verdict.rs` has fields: `passed`, `reason`, `gate`, `score`,
`detail`, `test_count`, `error_digest`, `duration_ms`. No `skipped` field exists.

## Exact Changes

1. Add a `skipped` field to `Verdict` in `verdict.rs`:
   ```rust
   pub struct Verdict {
       pub passed: bool,
       /// Whether the gate was skipped (not executed, not failed).
       #[serde(default)]
       pub skipped: bool,
       pub reason: String,
       // ... existing fields
   }
   ```
2. Add a `skip` constructor:
   ```rust
   pub fn skip(gate: impl Into<String>, reason: impl Into<String>) -> Self {
       Self {
           passed: false,
           skipped: true,
           reason: reason.into(),
           gate: gate.into(),
           score: 0.0,
           detail: None,
           test_count: None,
           error_digest: None,
           duration_ms: 0,
       }
   }
   ```
3. Update `stub_verdict()` in `rung_dispatch.rs` to use `Verdict::skip()`:
   ```rust
   fn stub_verdict(gate: &str, detail: impl Into<String>) -> Verdict {
       let message = format!("stub gate; {}", detail.into());
       Verdict::skip(gate, &message).with_detail(message)
   }
   ```
4. Ensure `Default` for `skipped` is `false` (`#[serde(default)]`) for backward compat.
5. Update any callers that check `verdict.passed` to also consider `verdict.skipped`:
   - Callers checking "did gate pass?" should check `verdict.passed && !verdict.skipped`
     (or just `verdict.passed` since skip sets `passed = false`).
   - TUI display should show "SKIP" instead of "PASS" or "FAIL" for skipped verdicts.
6. Update episode recording to distinguish pass/fail/skip.

## Design Guidance

The `skipped` flag allows three states: passed, failed, skipped. This is cleaner than
overloading `passed` because downstream consumers (TUI, episodes, learning) need to
distinguish "gate ran and passed" from "gate was not executed."

## Write Scope

- `crates/roko-core/src/verdict.rs`
- `crates/roko-gate/src/rung_dispatch.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `stub_verdict()` returns `Verdict { passed: false, skipped: true, ... }`
- [ ] Existing `Verdict::pass()` returns `Verdict { passed: true, skipped: false, ... }`
- [ ] `Verdict::fail()` returns `Verdict { passed: false, skipped: false, ... }`
- [ ] TUI shows "SKIP" for stub verdicts, not "PASS"
- [ ] Serialization/deserialization handles `skipped` field with default=false

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_07 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `stub_verdict()` returns `Verdict { passed: false, skipped: true, ... }`
- Existing `Verdict::pass()` returns `Verdict { passed: true, skipped: false, ... }`
- `Verdict::fail()` returns `Verdict { passed: false, skipped: false, ... }`
- TUI shows "SKIP" for stub verdicts, not "PASS"
- Serialization/deserialization handles `skipped` field with default=false
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_07 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
