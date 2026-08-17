# GATE_08: Add Verdict::skip constructor and convert stub_verdict

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#gate-08`](../ISSUE-TRACKER.md#gate-08)
- Source: `tmp/solutions/roko/tasks/04-GATE-PIPELINE.md` — Task 4.8
- Priority: **P0**
- Effort: 2 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: GATE_08 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`Verdict` at `crates/roko-core/src/verdict.rs:51` has `pass()` and `fail()` constructors but no `skip()`. The `stub_verdict()` function at `crates/roko-gate/src/rung_dispatch.rs:132` uses `Verdict::pass()` for stubs, meaning missing inputs produce passing verdicts. This is the AP-1 anti-pattern: false confidence from silent passes.

`GateVerdict` (roko-core) already has `skipped: bool` and `skip_reason: Option<String>` fields, but `Verdict` (roko-core) does not have corresponding fields. The `to_gate_verdict()` function in gate_service.rs hardcodes `skipped: false`.

## Exact Changes

1. Add a `skip()` constructor to `Verdict` in `crates/roko-core/src/verdict.rs`:
   ```rust
   /// A skipped verdict -- the gate did not run but this is not a failure.
   /// Skipped verdicts have `passed: true` (they don't block the pipeline)
   /// but carry a distinct marker so they are not counted as real passes.
   #[must_use]
   pub fn skip(gate: impl Into<String>, reason: impl Into<String>) -> Self {
       Self {
           passed: true,       // Does not block pipeline
           reason: reason.into(),
           gate: gate.into(),
           score: 0.0,         // No quality signal from a skip
           detail: None,
           test_count: None,
           error_digest: None,
           duration_ms: 0,
       }
   }

   /// Whether this verdict represents a skip (gate did not execute).
   #[must_use]
   pub fn is_skip(&self) -> bool {
       self.score == 0.0 && self.passed && self.reason.starts_with("stub gate;")
   }
   ```
2. Actually -- better approach: add a `skipped: bool` field to `Verdict` directly:
   ```rust
   pub struct Verdict {
       pub passed: bool,
       /// Whether the gate was skipped rather than executed.
       #[serde(default)]
       pub skipped: bool,
       // ... existing fields ...
   }
   ```
   Update `pass()` and `fail()` to set `skipped: false`. Add `skip()` that sets `skipped: true, passed: true`.
3. Change `stub_verdict()` in `crates/roko-gate/src/rung_dispatch.rs:132`:
   ```rust
   fn stub_verdict(gate: &str, detail: impl Into<String>) -> Verdict {
       let message = format!("stub gate; {}", detail.into());
       Verdict::skip(gate, message)
   }
   ```
4. Update `to_gate_verdict()` in `crates/roko-gate/src/gate_service.rs:369` to propagate the skipped flag:
   ```rust
   fn to_gate_verdict(gate_name: String, verdict: Verdict) -> GateVerdict {
       GateVerdict {
           gate_name,
           passed: verdict.passed,
           skipped: verdict.skipped,
           skip_reason: if verdict.skipped { Some(verdict.reason.clone()) } else { None },
           output: /* ... existing logic ... */,
           duration_ms: verdict.duration_ms,
       }
   }
   ```
5. Update `GateReport::all_passed()` at `crates/roko-core/src/foundation.rs:308` -- it already checks `v.passed && !v.skipped`, so skipped verdicts will correctly not count as "passed". Verify this logic.
6. Update `AdaptiveThresholds::observe()` callers -- skipped verdicts should NOT be observed (no real data). The gate_service.rs loop at line 353 already checks `!was_skipped`, so this is already correct.

## Design Guidance

Stubs should be `passed: true, skipped: true` so they don't block the pipeline (backward-compatible) but are clearly distinguishable from real passes. This means `all_passed()` returns false when only stubs ran (since it checks `!skipped`), which is the desired behavior: a pipeline of only stubs is not a real pass.

## Write Scope

- `crates/roko-core/src/verdict.rs`
- `crates/roko-gate/src/rung_dispatch.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/04-GATE-PIPELINE.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `Verdict::skip("gate", "reason").skipped == true`
- [ ] `Verdict::skip("gate", "reason").passed == true` (does not block pipeline)
- [ ] `stub_verdict()` returns a skipped verdict, not a passing verdict
- [ ] `GateReport::all_passed()` returns false when all verdicts are skipped
- [ ] `AdaptiveThresholds::observe()` is NOT called for skipped verdicts

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: GATE_08 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `Verdict::skip("gate", "reason").skipped == true`
- `Verdict::skip("gate", "reason").passed == true` (does not block pipeline)
- `stub_verdict()` returns a skipped verdict, not a passing verdict
- `GateReport::all_passed()` returns false when all verdicts are skipped
- `AdaptiveThresholds::observe()` is NOT called for skipped verdicts
- No files outside the Write Scope are modified.
- Commit message contains `tracker: GATE_08 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
