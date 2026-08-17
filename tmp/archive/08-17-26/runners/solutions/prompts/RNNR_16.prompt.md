# RNNR_16: Implement AntiPatternChecker with configurable rules

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#rnnr-16`](../ISSUE-TRACKER.md#rnnr-16)
- Source: `tmp/solutions/roko/tasks/14-RUNNER-PATTERNS.md` — Task 14.16
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: RNNR_16 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: A fast, regex-based checker that scans agent output for known LLM
code-generation anti-patterns. Runs in milliseconds, no compilation needed.

## Exact Changes

1. Define `AntiPatternRule`:
   ```rust
   pub struct AntiPatternRule {
       pub id: String,              // e.g. "AP-1"
       pub name: String,
       pub pattern: Regex,
       pub description: String,
       pub severity: Severity,      // Error, Warning
       pub file_glob: Option<String>,
       pub exemptions: Vec<String>,
   }
   ```
2. Implement the 10 checks from the mega-parity runner:
   - AP-1: Stub gates returning pass (`Ok(GateVerdict::pass` without real check)
   - AP-2: `block_on` in async code
   - AP-3: Duplicate trait definitions vs foundation types
   - AP-5: Raw `Command::new("claude")` shell-outs
   - AP-6: Inline prompt strings (`format!("You are a"`)
   - AP-7: `std::sync::Mutex` held across `.await`
   - AP-8: Empty function bodies (`{ }` or `{ todo!() }`)
   - AP-9: `unimplemented!` / `unreachable!` left behind
   - AP-10: Hardcoded localhost/port in non-test code
3. Add `AntiPatternChecker::check(files: &[PathBuf]) -> Vec<AntiPatternViolation>`
4. Support per-task exemptions via `ap_exemptions: ["AP-10"]`

## Write Scope

_None — this is a documentation/verification-only batch._

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/14-RUNNER-PATTERNS.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] All 10 checks execute in < 100ms for a typical task diff
- [ ] Each violation includes: rule ID, file, line number, matched text
- [ ] Exemptions work per-task
- [ ] False positive rate tracked via violation metadata

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: RNNR_16 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- All 10 checks execute in < 100ms for a typical task diff
- Each violation includes: rule ID, file, line number, matched text
- Exemptions work per-task
- False positive rate tracked via violation metadata
- No files outside the Write Scope are modified.
- Commit message contains `tracker: RNNR_16 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
