+++
id = "gap-2c51b9"
kind = "gap"
title = "[evals NOUS 4.5] Use proper eval statistics (Wilson CIs, balanced accuracy, MCC, small-n t-dist)"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-learn/experiments"]
created = 2026-09-03
updated = 2026-09-28
source = "tmp/archive/evals-audit/NOUS-EVAL-GAPS.md#4.5 Eval Statistics from Deep-Dive 07"
discovered_from = "audit:tmp/archive/evals-audit/NOUS-EVAL-GAPS.md#4.5 Eval Statistics from Deep-Dive 07"
anchors = ["scripts/check-metrics.sh", "roko learn experiments (P3-03)"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Headline metrics (check-metrics.sh) and experiments CLI use raw pass rates with fixed thresholds; adopt Wilson CIs, balanced accuracy, MCC and t-distribution handling for n<30.

Imported without verification from:
- `tmp/archive/evals-audit/NOUS-EVAL-GAPS.md#4.5 Eval Statistics from Deep-Dive 07`

How to verify: Check metrics script/experiments report for CI computations.
