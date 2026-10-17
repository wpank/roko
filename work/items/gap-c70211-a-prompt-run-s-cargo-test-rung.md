+++
id = "gap-c70211"
kind = "gap"
title = "A prompt run's cargo test rung gets no base check before its agent runs: the agent is not told which tests already fail, and the base builds cold only after a step fails"
status = "open"
triage = "verified"
severity = "p3"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
source = "round-v6-roko-gates#bug-5089fb-cargo"
discovered_from = "agent:roko-gates (round v6, bug-5089fb, 2026-10-17)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/baseline_verify.rs::prompt_base_checks", "crates/roko-cli/src/graph_task_dispatch/baseline_verify.rs::baseline_run", "crates/roko-cli/src/graph_task_dispatch/diff_snapshot.rs::snapshot_tree"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["bug-5089fb", "gap-161be1"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn prompt_run_on_a_red_cargo_base_checks_the_base_first' crates/roko-cli/ && cargo test -p roko-cli --lib prompt_run_on_a_red_cargo_base_checks_the_base_first"
+++

## Problem

Since bug-5089fb (merged a9f7fdf3f), a prompt run runs its Python test checks on the base commit before its agent
starts. It keeps that run per commit and tells the agent which tests already fail. Cargo suites are left out:
`prompt_base_checks` skips any rung whose runner is not unittest or pytest. Suppose a Rust workspace's `cargo test`
rung is red before the change. Then:

- the agent is not told which tests already fail, and may spend its turns on them;
- the base runs only after an attempt's step fails (the lazy gap-161be1 judgement). It is a cold build in a
  scratch checkout with a target directory of its own;
- a base build that outlasts the step's timeout leaves the judgement undecided, so the failure stands and the retry
  pays again.

Expected: the cargo rung's base run happens before the agent starts, cheaply, and the agent gets the same note as
for Python.

## Why it matters

Goal `golden-path`, for roko's own Rust workspaces and the Rust showcases. A red `cargo test` before the change is
common mid-refactor. Related: bug-5089fb, gap-161be1.

## Where

- `crates/roko-cli/src/graph_task_dispatch/baseline_verify.rs::prompt_base_checks` runs only the checks whose
  `TestRunner::runs_files()` is true.
- `crates/roko-cli/src/graph_task_dispatch/baseline_verify.rs::baseline_run` is the lazy base run of a failed
  step. It reads the kept run (`kept_baseline`) first.
- `crates/roko-cli/src/graph_task_dispatch/diff_snapshot.rs::snapshot_tree` snapshots a working tree, uncommitted
  changes included, as a git tree object.

## Current state

- The base checkout builds into `<checkout>/target`, because a target directory shared between worktrees lets cargo
  reuse the other tree's crates. That build is cold, which is why cargo was left out.
- Runs are kept per base commit, so a later run from the same commit pays nothing. The first one pays a full build.

## Plan

1. Before the agent starts, the working tree is the base. Run the cargo rung (with `--no-fail-fast`) there, in the
   workspace's own warm target directory, instead of in a scratch checkout.
2. Key the kept run by the snapshot tree (`snapshot_tree`) as well as the commit, so uncommitted changes count, and
   have `baseline_run` look the run up by the attempt's diff base.
3. Tell the agent which tests already fail (`BaseCheck::agent_note`), as for Python.
4. Add the test `prompt_run_on_a_red_cargo_base_checks_the_base_first`. It uses a stand-in `cargo` like
   `baseline_verify`'s `FAKE_CARGO` on a git repo whose base fails one test. The kept run must exist before the agent
   runs, the agent's prompt must carry the note, and the run must pass with that failure named as pre-existing.

## Done when

- The `[[verify]]` passes.
- A prompt run on a Rust workspace whose `cargo test` rung is red before the change tells the agent which tests
  already fail, and does not build the base a second time when its step fails.

## Notes

- Never share a target directory between two checkouts: memory `shared_cargo_target_dir_worktrees`.
- The base run must race the run's interrupt, as `run_prompt` does for Python (`tokio::select!` on
  `interrupt.stopped()`).
