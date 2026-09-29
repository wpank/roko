+++
id = "gap-398289"
kind = "gap"
title = "Hot-path performance fixes lack benchmark evidence (backlog #58 final lane)"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["workspace/performance"]
created = 2026-09-28
updated = 2026-09-28
source = "gaps-md#p2--medium/58"
anchors = ["crates/roko-compose", "crates/roko-runtime"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++

Backlog #58 fixed four hot-path problems in source, including synchronous I/O on Tokio threads and per-dispatch loads such as reading the model router from disk on every dispatch. The `roko-cli` library suite passes with the fixes. Its spec names real cold and warm benchmark repetitions (`scripts/run_benchmark_evidence.sh`) as the final verification, and those have not been recorded. #58 is the only item from GAPS.md's backlog tables that is still active in the local backlog.

Fix: run the benchmark evidence script at a fixed SHA and record the results. Then close this item, or file regressions for anything that got slower.
