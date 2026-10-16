+++
id = "bug-f12b2d"
kind = "bug"
title = "plan run --force-resume's help says it re-queues drifted tasks, but on the Graph path it archives the checkpoint and runs every task"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/commands"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
source = "paper-review-2026-10-16 (gap-aed6f1)"
discovered_from = "gap-aed6f1"
anchors = ["crates/roko-cli/src/commands/plan.rs::PlanCmd", "crates/roko-cli/src/graph_checkpoint.rs::prepare_graph_checkpoint_for_run"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-aed6f1"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -rqi 're-queue drifted' crates/roko-cli/src/commands/plan.rs crates/roko-cli/src/resolved_overrides.rs crates/roko-execution/src/overrides.rs && grep -B3 'force_resume: bool,' crates/roko-cli/src/commands/plan.rs | grep -qi 'archiv'"
+++

## Problem

`roko plan run --help` describes `--force-resume` as "Re-queue drifted tasks instead of aborting when resuming from a
snapshot" (`crates/roko-cli/src/commands/plan.rs:200`). On the Graph engine, the only plan executor, it does
something else: when a checkpoint cannot resume (the plan was edited, so "the converted plan graph has changed", or its
activity log or cost ledger is missing or unreadable), `--force-resume` archives the checkpoint and the run starts
again with every task (`prepare_graph_checkpoint_for_run` and `ResumePreview::unusable` in
`crates/roko-cli/src/graph_checkpoint.rs`; tests `preview_refuses_an_edited_plan_unless_forced` and
`force_resume_archives_drifted_state_and_starts_new_run`). The Runner-v2 strict resume that re-queued drifted tasks
(`crates/roko-cli/src/runner/resume.rs`, `prepare_resume`) is called only by tests. The same stale wording is in
`crates/roko-cli/src/resolved_overrides.rs:288` and `crates/roko-execution/src/overrides.rs:125`.

## Why it matters

An operator who edits a plan and resumes with `--force-resume` expects only the changed tasks to run again; every task
runs, and the completed work is re-done at its cost. The error message itself is right ("use --fresh to archive it or
--force-resume to start a new run"), so the help and the error disagree.

## Where

- `crates/roko-cli/src/commands/plan.rs::PlanCmd`: the `force_resume` flag's doc comment (the `--help` text).
- `crates/roko-cli/src/resolved_overrides.rs` and `crates/roko-execution/src/overrides.rs`: the field docs.
- `crates/roko-cli/src/graph_checkpoint.rs`: the behaviour to describe.

## Current state

Behaviour is as the Graph checkpoint code and its tests say; only the three doc comments are stale.

## Plan

1. Rewrite the flag's doc, for example: "When the checkpoint cannot resume (the plan was edited, or its logs are
   missing), archive it and run every task again instead of stopping."
2. Fix the two field docs to match.
3. If re-queueing only the drifted tasks is wanted on the Graph path, file that as a separate gap; this item is the doc.

## Done when

The `[[verify]]` command passes: the stale phrase is gone and the flag's doc names the archive.

## Notes

Found by the research-paper review (gap-aed6f1), whose §7.4 had repeated the re-queue claim; the paper now says an
edited plan is refused and a forced resume runs every task again.
