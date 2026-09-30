+++
id = "bug-5f0604"
kind = "bug"
title = "The portal-programme live-check harness gitignores the artifacts its implementer tasks write, so live checks fail as pre_verify:no_changes"
status = "open"
triage = "unverified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["plans/portal-programme/_harness"]
created = 2026-09-30
updated = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "wk-runstate report on gap-568056 (2026-09-30)"
anchors = ["plans/portal-programme/_harness/lib.sh"]
lane = "tests"
parent = "spec-9230a9"
links = { depends_on = [], blocks = [], related = ["gap-b72761"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q \"printf '.roko/\\\\nout/\" plans/portal-programme/_harness/lib.sh"
+++

## Problem

`plans/portal-programme/_harness/lib.sh` (`make_workspace`, about line 125) writes a workspace `.gitignore` of `.roko/`, `out/` and `hello/`. Its implementer fixtures write `out/*.txt`, and the canned PRD plan writes `hello/main.rs`. Since gap-b72761, the pre-verify screen rejects an attempt whose git-visible diff is empty, so the live checks (live-events, workspace-server, revision, authoring) would fail as `pre_verify:no_changes`. The same pattern broke `scripts/test_run_evidence_graph.py`, which was fixed by un-ignoring `out/`.

## Why it matters

The live checks are the portal programme's end-to-end evidence. They fail for a fixture reason, not a product one. Epic spec-9230a9.

## Where

`plans/portal-programme/_harness/lib.sh::make_workspace`.

## Current state

Not re-run since batch 11.

## Plan

1. Ignore only `.roko/` in the harness workspace.
2. Delete a rerun's stale artifacts where a check reruns fresh.
3. Re-run the live checks against a current binary and record the results.

## Done when

- [ ] The harness keeps its artifacts visible to git, and the live checks pass.
- [ ] The `[[verify]]` command passes.
