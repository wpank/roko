+++
id = "gap-6e970b"
kind = "gap"
title = "Evals audit: BenchmarkRegressionGate stub, EvalGenerator unconsumed, no fuzzing/mutation testing"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-gate/evals"]
created = 2026-09-04
updated = 2026-09-28
source = "tmp/nous-research/AUDIT-SUMMARY-2026-09-04.md#8. Evals Audit (`tmp/evals-audit/`, 22 files)"
discovered_from = "audit:tmp/nous-research/AUDIT-SUMMARY-2026-09-04.md#8. Evals Audit (`tmp/evals-audit/`, 22 files)"
anchors = ["roko-gate BenchmarkRegressionGate", "crates/roko-gate/src/eval_generator.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Gate pipeline 95% (BenchmarkRegressionGate is a stub); EvalGenerator output never consumed; no fuzzing or mutation testing; arena flywheel design-only; ~5% TUI eval visibility.

Imported without verification from:
- `tmp/nous-research/AUDIT-SUMMARY-2026-09-04.md#8. Evals Audit (`tmp/evals-audit/`, 22 files)`
- `tmp/evals-audit/`

How to verify: grep BenchmarkRegressionGate impl; grep EvalGenerator consumers.
