+++
id = "gap-5a4afd"
kind = "gap"
title = "Fresh end-to-end self-hosting dogfood rerun has not been performed"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["dogfood/self-hosting"]
created = 2026-09-15
updated = 2026-09-28
source = "gaps-md#2026-09-15-refactoring-audit-batch/dogfood-rerun"
anchors = ["crates/roko-cli/src/prd.rs:1161", "dev.sh"]
links = { depends_on = [], blocks = [], related = ["bug-96aff4", "bug-f7943a"], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++

The 2026-08-13 dogfood run hit four blockers: config layering, a stale snapshot, fsmonitor, and an enrichment/scheduler deadlock. Each now has a regression fix, and a pre-execution proof on 2026-09-15 passed (`doctor`, `prd idea`, `plan list`/`plan validate`, `status`, `learn all`). No live rerun of the full workflow (`prd idea` -> `prd draft` -> `research enhance-prd` -> `prd plan` -> `plan run` -> `status`) has been recorded since then.

The build blocker GAPS.md cited, the `SnapshotRebased` match arm, is fixed (`crates/roko-cli/src/runner/output_sink.rs:1485`). However, several of the old fixes targeted Runner-v2 code that has since been deleted, `roko prd` and `roko do` still call the removed runner (bug-96aff4), and `./dev.sh fast` is broken (bug-f7943a). The rerun therefore has to exercise the Graph engine.

Fix: run the complete self-hosting workflow on a clean checkout with the Graph engine, record the evidence bundle, and file an item for each blocker found.
