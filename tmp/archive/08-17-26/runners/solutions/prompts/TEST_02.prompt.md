# TEST_02: Build CliRunner wrapper

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#test-02`](../ISSUE-TRACKER.md#test-02)
- Source: `tmp/solutions/roko/tasks/15-TESTING-VERIFICATION.md` — Task 15.2
- Priority: **P0**
- Effort: 3 hours
- Depends on: `TEST_01` (source 15.1)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: TEST_02 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

The existing `run_roko()` and `run_roko_isolated()` functions in `crates/roko-cli/tests/common/mod.rs` wrap `assert_cmd::Command::cargo_bin("roko")` with workspace isolation. These need to be promoted to the shared harness with a richer API for structured output capture.

## Exact Changes

1. Add `assert_cmd = { workspace = true }` and `predicates = { workspace = true }` to `roko-test-harness/Cargo.toml`.
2. Create `CliRunner` struct with fields: `workdir: PathBuf`, `env_overrides: HashMap<String, String>`, `env_removals: Vec<String>`.
3. Methods:
   - `new(workspace: &TestWorkspace) -> Self` -- binds to workspace, removes `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `XDG_CONFIG_HOME` by default
   - `with_env(key: &str, value: &str) -> Self` -- add environment variable
   - `without_env(key: &str) -> Self` -- remove environment variable
   - `run_init() -> CapturedOutput` -- executes `roko init <workdir>` and returns output
   - `run_cmd(args: &[&str]) -> CapturedOutput` -- executes arbitrary subcommand
   - `assert_success(args: &[&str])` -- runs and asserts exit 0
   - `assert_failure(args: &[&str])` -- runs and asserts non-zero exit
   - `assert_output_contains(args: &[&str], pattern: &str)` -- runs, asserts exit 0, asserts stdout or stderr contains pattern
   - `assert_json_output(args: &[&str]) -> serde_json::Value` -- runs, asserts exit 0, parses stdout as JSON
4. `CapturedOutput` struct: `stdout: String`, `stderr: String`, `exit_code: i32`, `duration: Duration`.
5. All commands set `HOME` to the workspace dir (mimicking `run_roko_isolated()`).
6. All commands set `ROKO_LOG=error` to suppress noise.

## Write Scope

- `crates/roko-test-harness/Cargo.toml`
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

- [ ] `CliRunner::new(ws).assert_output_contains(&["--help"], "Usage")` passes
- [ ] `CliRunner::new(ws).run_init()` succeeds with exit 0
- [ ] Environment variables are isolated between test runs
- [ ] `CapturedOutput.duration` is populated

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: TEST_02 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `CliRunner::new(ws).assert_output_contains(&["--help"], "Usage")` passes
- `CliRunner::new(ws).run_init()` succeeds with exit 0
- Environment variables are isolated between test runs
- `CapturedOutput.duration` is populated
- No files outside the Write Scope are modified.
- Commit message contains `tracker: TEST_02 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
