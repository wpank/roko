+++
id = "find-b9d033"
kind = "finding"
title = "[status-quo P3-C] Runner-v2 `--engine legacy` residue across 20+ files, no retirement date"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli"]
created = 2026-09-15
updated = 2026-09-28
source = "tmp/archive/status-quo-audit-2026-09-21/09-completion-tracking.md#P3 items: 8/12 done (+1 P3-L, +1 P3-B)"
discovered_from = "audit:tmp/archive/status-quo-audit-2026-09-21/09-completion-tracking.md#P3 items: 8/12 done (+1 P3-L, +1 P3-B)"
anchors = ["--engine legacy", "PlanEngine", "resolve_engine_flag"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Tracker (2026-09-15) says Runner-v2 retained as --engine legacy with references in 20+ files and no retirement date, while other audits say Runner-v2 was deleted; residual references/flags need a final disposition.

Imported without verification from:
- `tmp/archive/status-quo-audit-2026-09-21/09-completion-tracking.md#P3 items: 8/12 done (+1 P3-L, +1 P3-B)`

How to verify: grep -rni 'runner.v2\|runner_v2\|engine legacy' crates/ docs/ CLAUDE.md
