+++
id = "gap-a4e0aa"
kind = "gap"
title = "[evals 4.3] EvoSkills/MAP-Elites quality-diversity archive not connected to arena outcomes"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-learn"]
created = 2026-09-02
updated = 2026-09-28
source = "tmp/archive/evals-audit/12-gaps-and-residuals.md#4.3 EvoSkills/MAP-Elites Quality-Diversity Archive"
discovered_from = "audit:tmp/archive/evals-audit/12-gaps-and-residuals.md#4.3 EvoSkills/MAP-Elites Quality-Diversity Archive"
anchors = ["MAP-Elites archive in roko-learn"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Scaffolding exists in roko-learn but the archive is not fed by arena outcomes, so the system cannot accumulate behaviorally diverse high-performing strategies.

Imported without verification from:
- `tmp/archive/evals-audit/12-gaps-and-residuals.md#4.3 EvoSkills/MAP-Elites Quality-Diversity Archive`
- `tmp/archive/evals-audit/SUMMARY.md#Recommended Priority Order`

How to verify: grep -ri 'map_elites\|MapElites\|evoskill' crates/ for non-test callers.
