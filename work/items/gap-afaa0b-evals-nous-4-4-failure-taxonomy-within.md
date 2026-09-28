+++
id = "gap-afaa0b"
kind = "gap"
title = "[evals NOUS 4.4] Failure taxonomy within gate rungs"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-gate"]
created = 2026-09-03
updated = 2026-09-28
source = "tmp/archive/evals-audit/NOUS-EVAL-GAPS.md#4.4 Failure Taxonomy Framework from Deep-Dive 06"
discovered_from = "audit:tmp/archive/evals-audit/NOUS-EVAL-GAPS.md#4.4 Failure Taxonomy Framework from Deep-Dive 06"
anchors = ["ErrorPatternStore", "post-gate reflection"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Gate failures are classified only by rung; build an open/axial-coded failure-mode taxonomy (possibly LLM-classified) to target prompt/tool/dispatch improvements.

Imported without verification from:
- `tmp/archive/evals-audit/NOUS-EVAL-GAPS.md#4.4 Failure Taxonomy Framework from Deep-Dive 06`

How to verify: Check error_pattern_store/reflection categorization granularity.
