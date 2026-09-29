+++
id = "spec-69ab3f"
kind = "spec"
title = "[evals NOUS 4.1] lm-evaluation-harness as an optional external gate rung"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-gate"]
created = 2026-09-03
updated = 2026-09-28
source = "tmp/archive/evals-audit/NOUS-EVAL-GAPS.md#4.1 lm-evaluation-harness as an External Gate"
discovered_from = "audit:tmp/archive/evals-audit/NOUS-EVAL-GAPS.md#4.1 lm-evaluation-harness as an External Gate"
anchors = ["cascade router admission"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Add an optional gate rung running lm-eval-harness tasks against a model before admitting it into the cascade router, catching model-level regressions task gates miss (subprocess interop).

Imported without verification from:
- `tmp/archive/evals-audit/NOUS-EVAL-GAPS.md#4.1 lm-evaluation-harness as an External Gate`

How to verify: grep for lm-eval / lm_eval integration.
