# STAB_65: Add anti-pattern checks as pre-gate step

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-65`](../ISSUE-TRACKER.md#stab-65)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.65
- Priority: **P2**
- Effort: 4 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_65 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Grep-based anti-pattern checks (AP-1 through AP-10) catch common LLM mistakes in milliseconds.
Not integrated into any gate.

## Exact Changes

1. Create `AntiPatternGate` in roko-gate.
2. Patterns: stub pass, `block_on` in async, duplicate traits, raw `Command::new("claude")`,
   inline prompts, `Mutex` across `.await`, empty function bodies, `unimplemented!/unreachable!`.
3. Run as rung -1 (before compile) -- millisecond cost.
4. Return structured feedback per pattern found.

## Write Scope

- `crates/roko-gate/src/`
- `crates/roko-runtime/src/effect_driver.rs`

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

- [ ] `unimplemented!()` in non-test code triggers anti-pattern check
- [ ] Feedback is structured (pattern name, file, line)

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_65 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `unimplemented!()` in non-test code triggers anti-pattern check
- Feedback is structured (pattern name, file, line)
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_65 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
