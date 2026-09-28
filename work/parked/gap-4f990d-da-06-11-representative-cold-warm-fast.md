+++
id = "gap-4f990d"
kind = "gap"
title = "DA-06/11: Representative cold/warm FAST benchmark scorecard never executed"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["scripts/dev-benchmark"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/dev-audit/06-benchmark-scorecard.md#Execution status"
discovered_from = "audit:tmp/dev-audit/06-benchmark-scorecard.md#Execution status"
anchors = ["scripts/dev_benchmark.py", "scripts/run_benchmark_evidence.sh", "dev.sh benchmark", "docs/v2/29-FAST-DEVELOPMENT.md"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Benchmark tooling exists (dev_benchmark.py, run_benchmark_evidence.sh, dev.sh benchmark history) but no real 5 cold + 5 warm runs per fixture/lane, manual Claude/Codex samples, p50/p95 or bundle links were produced; cache-strategy choice and escaped-regression baseline depend on it.

Imported without verification from:
- `tmp/dev-audit/06-benchmark-scorecard.md#Execution status`
- `tmp/dev-audit/11-implementation-status.md#Final evidence still required`
- `tmp/dev-audit/00-executive-audit.md#The highest-leverage fixes`
- `tmp/dev-audit/07-rollout.md#Stage 0: reversible experiment`

How to verify: Check .roko/benchmarks/ for real scorecards; first confirm `./dev.sh fast` still runs after the Runner-v2 deletion.
