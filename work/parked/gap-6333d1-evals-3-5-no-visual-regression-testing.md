+++
id = "gap-6333d1"
kind = "gap"
title = "[evals 3.5] No visual regression testing across vision-loop runs"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/vision_loop"]
created = 2026-09-02
updated = 2026-09-28
source = "tmp/archive/evals-audit/12-gaps-and-residuals.md#3.5 No Visual Regression Testing"
discovered_from = "audit:tmp/archive/evals-audit/12-gaps-and-residuals.md#3.5 No Visual Regression Testing"
anchors = ["roko vision-loop", "checkpoint manager screenshots"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Vision loop scores screenshots within a run only; checkpoint manager persists screenshots but nothing compares them to a prior-run baseline.

Imported without verification from:
- `tmp/archive/evals-audit/12-gaps-and-residuals.md#3.5 No Visual Regression Testing`
- `tmp/archive/evals-audit/11-vision-eval.md`

How to verify: Look for baseline screenshot diffing in vision-loop code.
