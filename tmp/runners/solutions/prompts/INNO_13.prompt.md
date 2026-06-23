# INNO_13: Create GateEvolver for failure-pattern-driven gate generation

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-13`](../ISSUE-TRACKER.md#inno-13)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.13
- Priority: **P1**
- Effort: 12 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_13 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`ErrorPatternStore` at `crates/roko-learn/src/error_pattern_store.rs`
accumulates patterns with error_hash, category, message, frequency, last_seen.
These are never fed back into gate construction.

Research: Darwin Godel Machine (Sakana, May 2025) -- self-improving agent that
also reward-hacked by removing monitoring tokens. Gates must evolve BUT remain
immutable from the agent's perspective (see Task 11.41).

## Exact Changes

1. Create `crates/roko-gate/src/gate_evolver.rs`.
2. Define `GateEvolver` struct holding a reference path to `ErrorPatternStore`
   data (`.roko/learn/error-patterns.json`).
3. Define `GeneratedGate`: `name: String`, `shell_command: String`,
   `target_pattern: String`, `created_from: String`, `effectiveness: f64`,
   `retired: bool`.
4. Implement `evolve_gates(&self, patterns: &[ErrorPattern]) -> Vec<GeneratedGate>`:
   - For each pattern with count >= 3, generate a ShellGate:
     - "unused import" -> `grep -rn "^use.*unused" {files}`
     - "missing semicolon" -> targeted syntax check
     - "type mismatch" -> focused `cargo check` on changed files only
5. Implement `should_retire(&self, gate: &GeneratedGate) -> bool`:
   - Retire if 3+ consecutive false positives.
6. Persist generated gates to `.roko/learn/gate-evolution.json`.
7. Add `pub mod gate_evolver;` to `crates/roko-gate/src/lib.rs`.

## Write Scope

- `crates/roko-gate/src/lib.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/11-INNOVATIONS.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] After 5 runs with recurring "unused import" failures, a targeted grep-based gate exists in `gate-evolution.json`
- [ ] Generated gate runs in < 100ms vs clippy's 3-8 seconds
- [ ] A gate with 3+ consecutive false positives is marked `retired: true`

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_13 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- After 5 runs with recurring "unused import" failures, a targeted grep-based gate exists in `gate-evolution.json`
- Generated gate runs in < 100ms vs clippy's 3-8 seconds
- A gate with 3+ consecutive false positives is marked `retired: true`
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_13 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
