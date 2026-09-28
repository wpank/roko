+++
id = "spec-051035"
kind = "spec"
title = "[evals NOUS 4.6] Adversarial red-team gate rung"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-gate"]
created = 2026-09-03
updated = 2026-09-28
source = "tmp/archive/evals-audit/NOUS-EVAL-GAPS.md#4.6 Red Team as Gate Rung"
discovered_from = "audit:tmp/archive/evals-audit/NOUS-EVAL-GAPS.md#4.6 Red Team as Gate Rung"
anchors = ["gate rung pipeline", "E34 safety"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Add an output-level adversarial gate rung probing for safety violations, tool misuse and instruction non-compliance (attacker model + scoring), complementing E34 process-level safety.

Imported without verification from:
- `tmp/archive/evals-audit/NOUS-EVAL-GAPS.md#4.6 Red Team as Gate Rung`

How to verify: grep gates for red-team/adversarial rung.
