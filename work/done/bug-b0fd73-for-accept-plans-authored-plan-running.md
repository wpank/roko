+++
id = "bug-b0fd73"
kind = "bug"
title = "For accept plans, authored_plan_running reports that tasks.toml no longer matches on every run"
status = "done"
triage = "verified"
severity = "p3"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/graph_checkpoint"]
created = 2026-09-29
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "2ae9d2a7f"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-accept's report on gap-d14a43, branch work/gap-d14a43 at 37b6d7c95)"
anchors = ["crates/roko-cli/src/graph_checkpoint.rs"]
lane = "rust-cold"
parent = "spec-e57870"
links = { depends_on = ["gap-d14a43"], blocks = [], related = ["gap-1b5636", "gap-ba4d01"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn an_accept_plan_matches_its_own_tasks_toml' crates/roko-cli/src/ && cargo test -p roko-cli --lib an_accept_plan_matches_its_own_tasks_toml"

[closed]
at = 2026-09-30
by = "coordinator (session 7622b882)"
evidence = "Merged in 23f666920. authored_plan_running sets generated '# roko accept:' steps aside and records each pinned sha256 on its [task.accept] entry, so accept plans match their tasks.toml. Batch 14 gate: first run on 4e030ab47 (check, clippy clean; tests pass: roko-cli 3171, roko-agent 2263, roko-core 1952, roko-learn 1203, roko-serve 986, roko-graph 472, roko-fs 259, roko-neuro 239), then re-gated on 8ce3bb131 (same code as MAIN 2ae9d2a7f) after the coordinator's rustfmt commits and serve-sec's bug-633b68 root fix: check, nightly fmt, clippy -p roko-cli -p roko-serve -p roko-core -p roko-agent -p roko-learn --keep-going -D warnings clean; roko-cli lib 3172 passed (one sibling-settle race flake passes alone, bug-779ae7); --test secret_canary 11 passed; --test secrets_and_git_guard_canary 1 passed, 1 ignored (bug-0d9ac4). Verify: its test passes in that run and its static checks pass on MAIN."
+++

## Problem

On `work/gap-d14a43` (37b6d7c95), `graph_checkpoint.rs::authored_plan_running` (:448 on the branch) compares the tasks a run converted with the current tasks.toml. On a mismatch it logs "tasks.toml no longer matches the tasks this run converted" (:440).

For a plan that uses `[task.accept]`, the converted tasks carry the pinned steps and their hashes in `task_def_json`, and the tasks.toml text does not. So the comparison fails on every run, and the warning fires for nothing (reported by wk-accept).

## Why it matters

Specs a cheap model can execute (epic spec-e57870): a warning that always fires hides the case it exists for, a tasks.toml edited while its plan runs. Resume and the checkpoint's authored-plan record rely on the same comparison.

## Where

`authored_plan_running` and `authored_plan` in `graph_checkpoint.rs`, and the fingerprint they compare.

## Current state

This exists only on the unmerged branch, which gap-d14a43 merges.

## Plan

1. Before comparing, strip the pinned accept steps (identified by their `# roko accept:` header) from the converted side. Alternatively, compile the tasks.toml side the same way before comparing.
2. Add `an_accept_plan_matches_its_own_tasks_toml`: an accept plan's run matches its own unchanged tasks.toml, and an edited tasks.toml still mismatches.

## Done when

- [ ] Running an unchanged accept plan logs no mismatch, and editing its tasks.toml still does.
- [ ] The `[[verify]]` command passes.

## Notes

- Merge gap-d14a43 first.
- Implemented on `work/bug-b0fd73` at `21c8df33f`; cargo verification deferred to the batch check.
- 2026-09-30: premise re-checked at `4d79f0016`. `authored_plan_running` compared the converted tasks, which
  have the generated `# roko accept:` steps in front of their own verify steps, with the file's tasks, which do
  not. The comparison now sets those steps aside. Plan step 1 alone would also have dropped the pinned hashes
  from the identity: the legacy fingerprint covered them only because every accept plan fell back to it. So each
  pinned sha256 is now recorded on its authored `[task.accept]` entry. A re-pinned test with other content still
  changes the identity, and the store's place on disk does not. Existing accept-plan checkpoints recorded the
  legacy fingerprint and resume through the legacy match.
