+++
id = "gap-c7ef8c"
kind = "gap"
title = "[evals 3.4] Arena 7-stage flywheel stages 3–7 unimplemented"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-learn/arena"]
created = 2026-09-05
updated = 2026-09-28
source = "tmp/archive/evals-audit/12-gaps-and-residuals.md#3.4 Arena Flywheel Not Implemented"
discovered_from = "audit:tmp/archive/evals-audit/12-gaps-and-residuals.md#3.4 Arena Flywheel Not Implemented"
anchors = ["arena_flywheel.rs TraceCollector AutoGrader"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
P4-01 delivered TraceCollector + AutoGrader (stages 1-2) and wrote a spec for stages 3-7 (preference-mine, failure-cluster, curriculum-gen, pattern-extract, preference-bootstrap). Arena results still do not improve agent behavior.

Imported without verification from:
- `tmp/archive/evals-audit/12-gaps-and-residuals.md#3.4 Arena Flywheel Not Implemented`
- `tmp/archive/evals-audit/arena-flywheel-spec.md`
- `tmp/archive/evals-audit/IMPLEMENTATION-CHECKLIST.md#P4-01 — Design and implement arena 7-stage flywheel (Stages 1-2)`

How to verify: Check arena_flywheel.rs for stages beyond AutoGrader and any runtime caller.
