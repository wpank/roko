+++
id = "bug-ba0c40"
kind = "bug"
title = "roko commits and delivers an attempt's undeclared build output: run 1's T7 ran cargo from crates/alpha under a relative CARGO_TARGET_DIR, and 233 build files (7.4 MB) under crates/alpha/target/ landed on the delivered branch; acceptance stages everything not ignored and the scope check only logs"
status = "done"
triage = "verified"
severity = "p1"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/worktree"]
created = 2026-10-19
updated = 2026-10-20
last_verified = 2026-10-20
last_verified_rev = "953af63d7"
source = "gap-f30b8e golden-path rerun, live run 1 (2026-10-19)"
discovered_from = "gap-f30b8e"
anchors = ["crates/roko-cli/src/orchestrator/worktree/acceptance.rs::commit_attempt", "crates/roko-cli/src/graph_task_dispatch/red_flags.rs", "crates/roko-cli/tests/golden_path_acceptance.rs::plan_command"]
links = { depends_on = [], blocks = [], related = ["gap-f30b8e", "find-5bcba7"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn an_attempts_undeclared_build_output_is_not_committed' crates/roko-cli/src && cargo test -p roko-cli --lib an_attempts_undeclared_build_output_is_not_committed"

[closed]
at = 2026-10-20
at_ts = "2026-10-20T17:08:06Z"
commit = "953af63d7"
by = "commit trailer"
executor = "claude-agent"
via = "work-batch"
size = "M"
claimed_at = "2026-10-20T14:02:51Z"
forced = false
evidence = "953af63d7 keeps an attempt's new build output that its task's files do not name out of its commit. crates/roko-cli/src/orchestrator/worktree/build_output.rs (new): a path the attempt adds is build output when it lies in a directory that holds no file of its base and is named target, node_modules, __pycache__ or .pytest_cache or marked by its tool (a CACHEDIR.TAG with the tag's signature, CMake's CMakeCache.txt), or is a .pyc file; a files entry keeps it only by reaching into the build directory, except under an unmarked target, which may be a source module. acceptance.rs commit_attempt stages as before, lists what the index adds to HEAD, takes the left-out paths out of the index (git rm --cached) and names them in the commit message and a warning; AcceptedWorktree::left_out records them and release_accepted removes them, so the delivered checkout still goes; attempt_review_diff takes the task's files. AttemptAcceptance and WorkspaceAcceptRequest carry the task's files (dispatcher, provider adapter, PlanGateCell); roko-gate exposes BUILD_OUTPUT_DIRS, BUILD_OUTPUT_EXTENSIONS and covers. golden_path_acceptance: plan_command removes CARGO_TARGET_DIR instead of setting the relative target, and assert_merges_green checks that every path the delivered branch changes is in some task's files. Checked through heavy.sh on the committed code: the verify an_attempts_undeclared_build_output_is_not_committed passes (before the fix, the old git add --all staging puts crates/a/target/* in the commit, replayed on the test's layout in a scratch repo), as do build_output_an_attempt_staged_is_left_out_too and build_output.rs's 2 unit tests; 141 lib tests under orchestrator::worktree, attempt_workspace, graph_execution::workspaces and plan_handback, and 28 under red_flags, graph_execution::batch and find-5bcba7's a_failed_plan_check_goes_back_to_the_task_whose_file_it_names and a_plan_check_is_not_handed_back_to_no_purpose; roko-gate attempt_diff (10) and roko-graph plan_gate and workspace (52); the 7 CI golden-path canaries (18 tests) and golden_path_acceptance (2 passed, live ignored; golden_path_fixture_plan_merges_green ran in full with node, output uncaptured); nightly rustfmt and cargo clippy -p roko-gate -p roko-graph -p roko-cli --no-deps -- -D warnings clean. Filed bug-346498 and bug-aee1cb (6d12064ee)."
+++

## Problem

roko commits whatever an attempt leaves untracked and not gitignored, inside or outside its task's `files`, and
delivers it. In the 2026-10-19 golden-path live run 1 (`graph-golden-path-31d6a6fb-…`, `4535b3914`, gap-f30b8e),
T7's commit `50e22de` added 235 files: `crates/alpha/src/lib.rs`, its accept test, and 233 cargo build files
(7.4 MB, three test executables, a `.rustc_info.json` naming the operator's `~/.rustup`) under
`crates/alpha/target/`. They went onto the plan branch, through the whole-plan check and into the delivered batch
branch, which the operator's `main` then fast-forwarded to. No check failed: the twelve checks of
`golden_path_live` hold, and the pre-verify scope check only logs paths outside a task's `files` unless
`[gates] diff_scope = "enforce"`.

How it happened:

- `run_tests` was refused in every call of the run (bug-1f8db7), so T7's agent ran
  `cd crates/alpha && cargo test --quiet` through `bash` (`tool_audit.jsonl`).
- The run's environment had `CARGO_TARGET_DIR=target`, relative, as `golden_path_live` sets it
  (`crates/roko-cli/tests/golden_path_acceptance.rs::plan_command`) and the live harness copied. Run from
  `crates/alpha`, cargo built into `crates/alpha/target/`. The seed's `.gitignore` covers only the root
  `/target`.
- Acceptance stages `git add --all -- .` minus roko's isolation dirs
  (`crates/roko-cli/src/orchestrator/worktree/acceptance.rs::commit_attempt`).

## Why it matters

A delivered branch with build output in it is one nobody would merge (both reviews of run 1 say so), and it can
carry machine paths. Any agent that builds or runs a tool from a subdirectory, or writes caches, logs or editor
files the repository does not ignore, gets them committed. The scope check sees these paths and lets them through.

## Where

- `crates/roko-cli/src/orchestrator/worktree/acceptance.rs::commit_attempt` (`add --all`).
- `crates/roko-cli/src/graph_task_dispatch/red_flags.rs` (the advisory `outside_scope` finding).
- `crates/roko-cli/tests/golden_path_acceptance.rs::plan_command` (the relative `CARGO_TARGET_DIR`).

## Plan

1. Do not commit an attempt's untracked files that are outside its task's `files` and look like build or cache
   output (a `target/`, `__pycache__/`, `node_modules/` directory or a file cargo marks `CACHEDIR.TAG`), or,
   stricter, any untracked path outside `files` unless the task declares it; say what was left out.
2. `golden_path_live` and the scripted test: drop the relative `CARGO_TARGET_DIR` (each checkout then builds
   in its workspace root's `target/`), and add a check that every path the delivered branch changes is in
   some task's `files`.
3. A test: an attempt that leaves `crates/a/target/…` beside its declared change commits only the declared
   change.

## Done when

- [ ] The `[[verify]]` test passes, and fails before the fix.

## Notes

- Found by the reviews of gap-f30b8e's 2026-10-19 rerun (run 1); evidence in the main checkout (gitignored)
  `tmp/status-quo/2026-10-16-live/evidence/gp3/golden-path/run1/` (`merged.stat`, `review.md`, run record).
- The worker's live harness now leaves `CARGO_TARGET_DIR` unset for roko.
