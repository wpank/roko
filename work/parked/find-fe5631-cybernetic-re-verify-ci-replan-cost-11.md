+++
id = "find-fe5631"
kind = "finding"
title = "[cybernetic re-verify: CI/replan/cost] 11 CI/replan/conductor/cost closures wired into deleted Runner-v2"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/graph_execution"]
created = 2026-09-06
updated = 2026-09-28
source = "tmp/archive/cybernetic-audit/30-master-checklist.md#P0 -- Close Broken Feedback Loops"
discovered_from = "audit:tmp/archive/cybernetic-audit/30-master-checklist.md#P0 -- Close Broken Feedback Loops"
anchors = ["anomaly.rs", "record_intervention_outcome", "build_gate_failure_plan_revision", "merge conflict history", "pricing table"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Checked done 2026-09-06 in runner/: P0-08 anomaly cost spikes, P0-17/P1-40 CI outcome to learning, P0-18 conductor intervention outcome, P1-26 PR review text to replan, P1-46 merge conflict history, P1-48 orphan temp/lock cleanup, P3-01/P3-02 replan, P3-33 pricing refresh, P4-01 verbal reflection.

Imported without verification from:
- `tmp/archive/cybernetic-audit/30-master-checklist.md#P0 -- Close Broken Feedback Loops`
- `tmp/archive/cybernetic-audit/30-master-checklist.md#P3 -- Improve Existing Mechanisms`
- `tmp/archive/cybernetic-audit/30-master-checklist.md#P4 -- Implement New Capabilities`

A source claims this was fixed; confirm against current code before closing.

How to verify: Check each hook exists in Graph execution path (graph_execution/, roko-graph cells), not only in removed runner/ files.
