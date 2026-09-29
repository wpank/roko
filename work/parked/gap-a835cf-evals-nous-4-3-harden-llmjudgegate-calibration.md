+++
id = "gap-a835cf"
kind = "gap"
title = "[evals NOUS 4.3] Harden LlmJudgeGate calibration (stratified sampling, per-class P/R, kappa drift)"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-gate/llm_judge"]
created = 2026-09-03
updated = 2026-09-28
source = "tmp/archive/evals-audit/NOUS-EVAL-GAPS.md#4.3 Judge Calibration Protocol from Deep-Dive 04"
discovered_from = "audit:tmp/archive/evals-audit/NOUS-EVAL-GAPS.md#4.3 Judge Calibration Protocol from Deep-Dive 04"
anchors = ["LlmJudgeGate", "calibration module (P3-01)"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
P3-01 added a 20-example golden set; Nous methodology adds disagreement-biased stratified labels, per-class precision/recall, moving-window kappa drift detection, confusion-matrix bias analysis.

Imported without verification from:
- `tmp/archive/evals-audit/NOUS-EVAL-GAPS.md#4.3 Judge Calibration Protocol from Deep-Dive 04`

How to verify: Inspect calibration module for kappa drift / per-class metrics.
