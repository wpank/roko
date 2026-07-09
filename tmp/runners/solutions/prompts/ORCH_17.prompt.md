# ORCH_17: Anti-Pattern Check Registry

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#orch-17`](../ISSUE-TRACKER.md#orch-17)
- Source: `tmp/solutions/roko/tasks/02-ORCHESTRATION.md` — Task 2.17
- Priority: **P1**
- Effort: 4 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ORCH_17 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

The mega-parity runner uses fast grep-based anti-pattern checks (AP-1 through AP-10) that catch common LLM code generation mistakes in milliseconds. These are not integrated into WorkflowEngine or any gate. The checks are:

- AP-1: Stub gates that return pass (silent-pass)
- AP-2: `block_on` in async code
- AP-3: Duplicate trait definitions vs foundation.rs
- AP-5: Raw `Command::new("claude")` shell-outs
- AP-6: Inline prompt strings (`format!("You are a...")`)
- AP-7: std::sync::Mutex held across .await
- AP-8: Empty function bodies
- AP-9: unimplemented!/unreachable! left behind
- AP-10: Hardcoded localhost/port in non-test code

## Exact Changes

1. Create `crates/roko-gate/src/anti_pattern.rs` with:
   ```rust
   pub struct AntiPatternCheck {
       pub id: String,
       pub name: String,
       pub pattern: regex::Regex,
       pub file_glob: String,
       pub exclude_paths: Vec<String>,  // e.g., "tests/", "*_test.rs"
       pub severity: Severity,
       pub message: String,
   }
   pub enum Severity { Error, Warning }

   pub struct AntiPatternRegistry {
       checks: Vec<AntiPatternCheck>,
   }
   impl AntiPatternRegistry {
       pub fn default_checks() -> Self { /* AP-1 through AP-10 */ }
       pub fn run(&self, workdir: &Path, exempt: &[String]) -> Vec<AntiPatternViolation>;
   }
   ```
2. Register all 10 AP checks with their regex patterns.
3. `run()` walks .rs files in workdir (skipping test files for applicable checks), applies regex, collects violations.
4. Add `pub mod anti_pattern;` to `crates/roko-gate/src/lib.rs`.
5. Re-export `AntiPatternRegistry` and `AntiPatternViolation`.

## Design Guidance

Anti-pattern checks must be fast (< 100ms for the full workspace). Use `walkdir` + `memmap2` or simple `std::fs::read_to_string` + `Regex::is_match`. Do not use tree-sitter or AST parsing -- these are grep-level checks. The false positive rate from the mega-parity runner was ~2-3% (mostly AP-10 for localhost in legitimate config code).

## Write Scope

- `crates/roko-gate/src/anti_pattern.rs`
- `crates/roko-gate/src/lib.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/02-ORCHESTRATION.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] All 10 AP checks registered with correct regex patterns
- [ ] `run()` completes in < 100ms on a typical crate (< 10K LOC)
- [ ] AP-7 (mutex across await) detects the pattern in synthetic test code
- [ ] Exempt list excludes specific AP IDs for a task
- [ ] Test files are excluded from AP-8 (empty functions are common in test stubs)

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ORCH_17 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- All 10 AP checks registered with correct regex patterns
- `run()` completes in < 100ms on a typical crate (< 10K LOC)
- AP-7 (mutex across await) detects the pattern in synthetic test code
- Exempt list excludes specific AP IDs for a task
- Test files are excluded from AP-8 (empty functions are common in test stubs)
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ORCH_17 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
