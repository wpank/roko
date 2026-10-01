+++
id = "bug-5f0604"
kind = "bug"
title = "The portal-programme live-check harness gitignores the artifacts its implementer tasks write, so live checks fail as pre_verify:no_changes"
status = "done"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["plans/portal-programme/_harness"]
created = 2026-09-30
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "47c003a50"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "wk-runstate report on gap-568056 (2026-09-30)"
anchors = ["plans/portal-programme/_harness/lib.sh"]
lane = "tests"
parent = "spec-9230a9"
links = { depends_on = [], blocks = [], related = ["gap-b72761"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -qF '\\nout/' plans/portal-programme/_harness/lib.sh"

[closed]
at = 2026-09-30
commit = "47c003a50"
by = "commit trailer"
evidence = "make_workspace now gitignores .roko/ and the checks' own root records (serve.log, *.sse, *.json, *.out, *.headers, portal-fixture/), never out/ or hello/, and workspace-server-check clears live-a's and live-nested's artifacts before its three fresh reruns. The item's verify passes. Against roko 7902e44a3, live-events 12/12, live-output 15/15, revision 15/15 and workspace-server 58/58 pass. Without the deletions, the reruns fail as pre_verify:no_changes."
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

## Notes

- Premise re-checked against roko 7902e44a3 (built after batch 13): with the unmodified harness, live-events (12/12) and
  workspace-server (58/58) passed. `out/` was ignored, but the checks' own records at the workspace root (`serve.log`,
  the `events*.sse` captures, the `status.json` that `wait_idle` rewrites every second) change during every attempt. The
  screen counted them as the task's diff, so it passed for the wrong reason, and a no-op agent would have passed too.
- The fix ignores those records as well as `.roko/`, so the screen sees only what a task writes. `clear_artifacts`
  (lib.sh) deletes an earlier run's artifacts before a fresh rerun. Only workspace-server-check reruns plans fresh: the
  plain execute of live-a, run-all (live-a a third time) and the nested execute of live-nested.
- Results with the fix (ROKO_BIN=roko-batch-target/debug/roko, 7902e44a3): live-events 12/12, live-output 15/15,
  revision 15/15 and workspace-server 58/58 pass. Negative control, the new `.gitignore` without `clear_artifacts`:
  workspace-server fails 3 checks, all from `pre_verify:no_changes` on live-a T01 (twice) and live-nested T01.
- Not caused by this item; the unmodified harness fails the same checks:
  - authoring-check fails "tasks carry verify commands and model_hint" (35/36). The `POST /api/plans` scaffold no
    longer writes `model_hint`.
  - access-check fails its two `/demo` checks (22/24). roko serve finds the demo app at the compile-time path
    `demo/demo-app/dist`, which the batch build tree lacks.
