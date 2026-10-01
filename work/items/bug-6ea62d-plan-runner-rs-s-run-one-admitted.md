+++
id = "bug-6ea62d"
kind = "bug"
title = "plan_runner.rs's \"Run one admitted plan\" doc comment sits above drop_exclusion_for_worktrees instead of run_one_plan"
status = "open"
triage = "unverified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/graph_execution"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-tiers' report on gap-4ec59f)"
anchors = ["crates/roko-cli/src/graph_execution/plan_runner.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-4ec59f"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "python3 -c \"import re,sys;s=open('crates/roko-cli/src/graph_execution/plan_runner.rs').read();i=s.find('Run one admitted plan');sys.exit(0 if 'fn run_one_plan' in s[i:i+600] and 'fn drop_exclusion_for_worktrees' not in s[i:i+600] else 1)\""
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
