# TEST_03: Build gate test scaffold

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#test-03`](../ISSUE-TRACKER.md#test-03)
- Source: `tmp/solutions/roko/tasks/15-TESTING-VERIFICATION.md` — Task 15.3
- Priority: **P0**
- Effort: 3 hours
- Depends on: `TEST_01` (source 15.1)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: TEST_03 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Gate tests need a minimal Cargo project to run `CompileGate`, `ClippyGate`, `TestGate`, and friends. The existing `scaffold_cargo_project()` in `crates/roko-gate/tests/gate_truth.rs` creates a minimal project but only for compile tests. The `seed_minimal_rust_project()` in `crates/roko-cli/tests/common/mod.rs` creates another variant. Both should be replaced by a shared scaffold with mutation methods.

## Exact Changes

1. Create `GateTestProject` struct wrapping a `TempDir` with a valid Cargo project.
2. `GateTestProject::new()` creates:
   - `Cargo.toml` with `[package]` and `edition = "2021"` and `[lib]`
   - `src/lib.rs` with `pub fn answer() -> u32 { 42 }` and a passing `#[test]`
3. Mutation methods:
   - `break_compile()` -- inserts `let x: i32 = "not a number";` into `src/lib.rs`
   - `break_clippy()` -- inserts `let _ = vec![1,2,3].len() > 0;` (triggers `clippy::len_zero`)
   - `break_test()` -- changes test assertion to `assert_eq!(answer(), 999)`
   - `add_unused_import()` -- adds `use std::collections::HashMap;` (clippy warning)
   - `add_borrow_error()` -- inserts code with borrow checker violation
   - `add_type_mismatch()` -- inserts `let _: String = 42u32;`
   - `restore()` -- resets `src/lib.rs` to the original passing state
4. Query methods:
   - `path() -> &Path` -- root of the Cargo project
   - `lib_path() -> PathBuf` -- path to `src/lib.rs`
5. Helper: `run_gate(project: &GateTestProject, gates: &[&str]) -> GateReport` -- runs `GateService::run_gates()` with the project as workdir.

## Design Guidance

The gate scaffold must produce deterministic Cargo projects. Use `edition = "2021"` (not `edition = "2024"` which the existing `seed_minimal_rust_project` uses and which is only available on newer rustc). The test method in `src/lib.rs` should be `#[test] fn it_works() { assert_eq!(answer(), 42); }`.

## Write Scope

- `crates/roko-test-harness/src/lib.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/15-TESTING-VERIFICATION.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `GateTestProject::new()` passes compile + clippy + test gates
- [ ] `break_compile()` causes `CompileGate` to fail
- [ ] `break_clippy()` causes `ClippyGate` to fail
- [ ] `break_test()` causes `TestGate` to fail
- [ ] `restore()` returns project to passing state

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: TEST_03 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `GateTestProject::new()` passes compile + clippy + test gates
- `break_compile()` causes `CompileGate` to fail
- `break_clippy()` causes `ClippyGate` to fail
- `break_test()` causes `TestGate` to fail
- `restore()` returns project to passing state
- No files outside the Write Scope are modified.
- Commit message contains `tracker: TEST_03 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
