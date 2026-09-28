+++
id = "gap-700893"
kind = "gap"
title = "[evals deferred] Process reward models end-to-end (labeled trajectory collection)"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-learn"]
created = 2026-09-05
updated = 2026-09-28
source = "tmp/archive/evals-audit/SUMMARY.md#Gaps Deferred to Product Roadmap"
discovered_from = "audit:tmp/archive/evals-audit/SUMMARY.md#Gaps Deferred to Product Roadmap"
anchors = ["PromiseTracker"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
P4-03 added PromiseTracker early termination, but end-to-end PRMs need labeled trajectory data collection infrastructure (deferred).

Imported without verification from:
- `tmp/archive/evals-audit/SUMMARY.md#Gaps Deferred to Product Roadmap`
- `tmp/archive/evals-audit/12-gaps-and-residuals.md#4.4 Process Reward Models at Runtime`

How to verify: Check for trajectory labeling/storage feeding a reward model.
