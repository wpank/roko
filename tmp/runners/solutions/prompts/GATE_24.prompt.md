# GATE_24: Remove dead gate dispatch code

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#gate-24`](../ISSUE-TRACKER.md#gate-24)
- Source: `tmp/solutions/roko/tasks/04-GATE-PIPELINE.md` — Task 4.24
- Priority: **P1**
- Effort: 2 hours
- Depends on: `GATE_05` (source 4.5), `GATE_06` (source 4.6)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: GATE_24 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

After Tasks 4.5 and 4.6, the old inline gate dispatch code in ACP runner and `roko run` is replaced by GateService calls. The old code should be removed to prevent drift.

## Exact Changes

1. Remove the old `run_gates()` function body in `crates/roko-acp/src/runner.rs` (the old ~100 line implementation that directly constructs CompileGate/TestGate/ClippyGate).
2. Remove the `#[cfg(feature = "legacy-orchestrate")] async fn run_gate()` in `crates/roko-cli/src/run.rs` if the legacy feature flag is no longer needed.
3. Remove unused imports of `CompileGate`, `TestGate`, `ClippyGate` from migrated files.
4. Run `cargo clippy --workspace --no-deps -- -D warnings` to catch dead code.

## Write Scope

- `crates/roko-acp/src/runner.rs`
- `crates/roko-cli/src/run.rs`

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

- [ ] No direct CompileGate/TestGate/ClippyGate construction outside of roko-gate crate
- [ ] `grep -rn 'CompileGate::new\|TestGate::new\|ClippyGate::new' crates/ --include='*.rs' | grep -v roko-gate | grep -v test` returns empty

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: GATE_24 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- No direct CompileGate/TestGate/ClippyGate construction outside of roko-gate crate
- `grep -rn 'CompileGate::new\|TestGate::new\|ClippyGate::new' crates/ --include='*.rs' | grep -v roko-gate | grep -v test` returns empty
- No files outside the Write Scope are modified.
- Commit message contains `tracker: GATE_24 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
