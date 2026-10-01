+++
id = "bug-6ea62d"
kind = "bug"
title = "plan_runner.rs's \"Run one admitted plan\" doc comment sits above drop_exclusion_for_worktrees instead of run_one_plan"
status = "done"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/graph_execution"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-tiers' report on gap-4ec59f)"
anchors = ["crates/roko-cli/src/graph_execution/plan_runner.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-4ec59f"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "python3 -c \"import re,sys;s=open('crates/roko-cli/src/graph_execution/plan_runner.rs').read();i=s.find('Run one admitted plan');sys.exit(0 if 'fn run_one_plan' in s[i:i+600] and 'fn drop_exclusion_for_worktrees' not in s[i:i+600] else 1)\""

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:18Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T16:12:56Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

## Problem

The doc comment for run_one_plan was left above another function.

## Why it matters

Misleading docs on the plan runner's main entry.

## Plan

Move the comment to run_one_plan and give drop_exclusion_for_worktrees its own.

## Done when

- [ ] The comment documents the right function
- [ ] The `[[verify]]` command passes.

## Notes

2026-10-01 (wk-planrun): implemented on work/gap-dd4826; cargo verification deferred to the batch check.
Comment-only change in `graph_execution/plan_runner.rs`: the "Run one admitted plan to a terminal checkpoint" paragraph now sits on `run_one_plan`, and `drop_exclusion_for_worktrees` keeps only its own `--worktree-per-task` paragraph. The `[[verify]]` script passes.
