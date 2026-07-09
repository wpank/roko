# TEST_01: Create roko-test-harness crate with TestWorkspace

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#test-01`](../ISSUE-TRACKER.md#test-01)
- Source: `tmp/solutions/roko/tasks/15-TESTING-VERIFICATION.md` — Task 15.1
- Priority: **P0**
- Effort: 4 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: TEST_01 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Test helpers are currently embedded in `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/tests/common/mod.rs` and only available to roko-cli tests. The `TestWorkspace` needs to be shared across roko-gate, roko-learn, roko-runtime, roko-compose, roko-orchestrator, and roko-acp integration tests.

The existing `common/mod.rs` (470 LOC) provides `init_workspace()`, `seed_minimal_rust_project()`, `seed_git_repo()`, `setup_sample_plan_workspace()`, `run_roko()`, `run_roko_isolated()`, `spawn_roko_serve_on_random_port()`, `pick_unused_port()`, and a mock claude script. These patterns should be extracted and generalized.

## Exact Changes

1. Create `crates/roko-test-harness/Cargo.toml` with dependencies:
   - `tempfile = { workspace = true }`
   - `serde = { workspace = true }`
   - `serde_json = { workspace = true }`
   - `roko-core = { path = "../roko-core" }`
   - `tokio = { workspace = true, features = ["full", "test-util"] }`
2. Add `"crates/roko-test-harness"` to workspace members in the root `Cargo.toml`.
3. Create `TestWorkspace` struct wrapping `TempDir` with methods:
   - `new() -> Self` -- creates tempdir, initializes `.roko/` directory structure, writes minimal `roko.toml`
   - `path() -> &Path` -- returns tempdir path
   - `roko_dir() -> PathBuf` -- returns `.roko/` path
   - `write_config(toml: &str)` -- writes `roko.toml`
   - `write_file(relative: &str, content: &str)` -- writes arbitrary file
   - `read_file(relative: &str) -> String` -- reads file content
   - `file_exists(relative: &str) -> bool`
   - `seed_learning_state()` -- creates `.roko/learn/` with empty artifacts (episodes.jsonl, cascade-router.json, efficiency.jsonl, gate-thresholds.json, section-effects.json)
   - `seed_episodes(episodes: &[Episode])` -- writes pre-built episodes to `.roko/episodes.jsonl`
4. `TestWorkspace` implements `Drop` by delegating to `TempDir::drop` (automatic cleanup).
5. Re-export `TestWorkspace` from `lib.rs`.

## Design Guidance

`TestWorkspace` should NOT depend on the `roko` binary (no `assert_cmd`). It creates the filesystem layout that roko expects, but tests that need the binary use `CliRunner` (Task 15.2). This separation lets gate, learn, and runtime tests use `TestWorkspace` without building the full CLI binary.

The `.roko/` directory layout must match what `roko init` creates. Inspect the actual `roko init` output path: `.roko/`, `.roko/learn/`, `.roko/state/`, `.roko/prd/`, `.roko/research/`, `.roko/bench/`, `roko.toml`. The `seed_learning_state()` method should create files that match the schemas expected by `EpisodeLogger`, `CascadeRouter`, `AdaptiveThresholds`, etc.

## Write Scope

- `Cargo.toml`

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

- [ ] `TestWorkspace::new()` creates a valid directory layout
- [ ] `seed_learning_state()` creates all expected files
- [ ] `TestWorkspace` can be used from any crate via `[dev-dependencies]`

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: TEST_01 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `TestWorkspace::new()` creates a valid directory layout
- `seed_learning_state()` creates all expected files
- `TestWorkspace` can be used from any crate via `[dev-dependencies]`
- No files outside the Write Scope are modified.
- Commit message contains `tracker: TEST_01 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
