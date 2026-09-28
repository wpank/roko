+++
id = "gap-b5fea2"
kind = "gap"
title = "Execute Benchmark Data Collection Runs"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/runner"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/backlog/archive/390-benchmark-data-collection.md#390 — Execute Benchmark Data Collection Runs"
discovered_from = "audit:tmp/backlog/archive/390-benchmark-data-collection.md#390 — Execute Benchmark Data Collection Runs"
anchors = ["scripts/dev_benchmark.py", "dev.sh", ".config/nextest.toml", "plans/archive/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
zero benchmark data despite complete tooling. Dev-audit found complete benchmark automation tooling (dev_benchmark.py at 2,689 lines, nextest profiles, fixture plans) but zero actual benchmark data collected. The `.roko/benchmarks/` directory doesn't exist. No cold/warm samples have been gathered.

Imported without verification from:
- `tmp/backlog/archive/390-benchmark-data-collection.md#390 — Execute Benchmark Data Collection Runs`

How to verify: Check whether the gap described in tmp/backlog/archive/390-benchmark-data-collection.md still exists at the anchored paths. [evidence: 00-INDEX (2026-09-21) listed active: 2026-09-21 Audit Sweep Items (#376-#395)]
