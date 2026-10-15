+++
id = "find-d1a883"
kind = "finding"
title = "Plan-set footprints trust declared files and read verify commands word by word"
status = "open"
triage = "verified"
severity = "p3"
goal = "core"
subsystem = ["roko-cli/graph_execution/plan_set"]
created = 2026-09-29
updated = 2026-10-15
last_verified = 2026-10-01
last_verified_rev = "ebdc0f5d5"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:d1-parallel-plans"
anchors = ["crates/roko-cli/src/graph_execution/plan_set.rs::built_by"]
links = { depends_on = [], blocks = [], related = ["gap-0001a1"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -n 'changed_files' crates/roko-cli/src/graph_execution/plan_set.rs (only if the decision is to compare actual changed files; otherwise close as a decision with a recorded rationale)"
+++

The plan-set scheduler decides which plans may run together from each plan's footprint: the files its tasks declare, plus the areas its verify commands build. `AreaResolver::built_by` splits each command with `split_whitespace` and matches words such as `-p <crate>`. A task that edits files it did not declare, or a verify command that builds more than its words show (a script, or an alias for `cargo test --workspace`), can let two conflicting plans run side by side. Undeclared files and workspace manifests already make a plan run alone (test `undeclared_files_and_workspace_manifests_run_alone`).

Decide whether to also compare the files each task actually changed, and stop co-scheduling on overlap, instead of trusting declarations.

## Notes

2026-10-01 (wk-planrun): blocked on a decision; no code changed. Rechecked at `ebdc0f5d5`: the premise holds. `PlanFootprint::of` trusts `files`/`crates_touched` and `AreaResolver::built_by` reads verify commands word by word: a root-level script or alias reads as "reads files only". Parallel plans only ever share one working tree: `run_graph_plan_body` refuses `--worktree-per-task` with `max_parallel_plans > 1` (`plan_runner.rs`, the bail after `max_parallel_plans` is resolved), and `max_parallel_plans` defaults to 1.
Why comparing actual changed files is not enough: by the time a task's diff shows an undeclared write, a plan beside it may already have built against it, and in one shared tree `git status` cannot say which of several concurrently running tasks made a change. It could only mark the plan exclusive for the rest of the run.
Options: (a) keep declarations as the contract and lint them in `plan validate`; (b) conservative footprints: a verify program `built_by` cannot read (a script path, `make`/`just`/`npm`, an alias) counts as building every package in a Rust workspace; (c) while plans run beside each other, limit write tools to declared files (tool policy); (d) give parallel plans their own worktrees or plan branches, so an overlap becomes a merge conflict that delivery catches (gap-4ec59f, spec-f830c4). Recommendation: (b) now, about 20 lines in `built_by` plus a test next to `undeclared_files_and_workspace_manifests_run_alone`; (d) as the real fix; no runtime diffing.
Next step: Will picks an option. This item's `[[verify]]` is prose, not a command, so it must be rewritten to match the choice (for (b), a named test that a script-calling verify step conflicts with a plan that writes a package).

2026-10-15 (status-quo assessment at 7b6a5af44, finding 41): the verify can never pass as written: `grep -n 'changed_files' crates/roko-cli/src/graph_execution/plan_set.rs (only if …)` ends in an English parenthetical, which is a bash syntax error (rc 2). Will's 2026-10-15 decision 3 chose (b) conservative footprints now and (d) per-plan worktrees later; rewrite the verify for (b) (a guarded named test that a verify command naming an undeclared file widens the footprint) in the change that implements it.

2026-10-15 decision (Will, 2026-10-15; tmp/status-quo/2026-10-15/DECISIONS-2026-10-15.md), decision 3: (b) conservative footprints now (about 20 lines in built_by plus a test), (d) per-plan worktrees later as the real fix. Rewrite this item's verify to (b) when you pick it up.
