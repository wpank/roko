+++
id = "gap-1e8ed6"
kind = "gap"
title = "HTTP Run-Scoped Event and Gate Query Endpoints"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-serve/routes"]
created = 2026-09-07
updated = 2026-09-28
source = "tmp/backlog/215-http-run-scoped-event-query.md#215 — HTTP Run-Scoped Event and Gate Query Endpoints"
discovered_from = "audit:tmp/backlog/215-http-run-scoped-event-query.md#215 — HTTP Run-Scoped Event and Gate Query Endpoints"
anchors = [".roko/events.jsonl", "crates/roko-serve/src/routes/events.rs", "crates/roko-serve/src/routes/runs.rs", "crates/roko-serve/src/routes/shared_runs.rs", "crates/roko-serve/src/openapi.rs", "docs/v2/API-REFERENCE.md", "crates/roko-fs/src/run_index.rs", "crates/roko-cli/src/runner/persist.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
[partial] SOURCE-DONE; LOOPBACK API/CURSOR/SSE CHECKPOINT VERIFIED (2026-08-31, `5f689d66e` + `85c052fc9`). The… — events are only accessible by reading JSONL files directly; no HTTP query capability for run-scoped or filtered event access. Runtime events are written to `.roko/events.jsonl` but…

Imported without verification from:
- `tmp/backlog/215-http-run-scoped-event-query.md#215 — HTTP Run-Scoped Event and Gate Query Endpoints`
- `tmp/backlog/archive/215-http-run-scoped-event-query.md#(archived copy; status: SOURCE-DONE; LOOPBACK API/CURSOR/SSE CHECKPOINT VERIFIED…)`
- `tmp/backlog/_archive/_mori-diffs-gaps.md#Group G-2`
- `tmp/archive/backlog-closure-2026-09-01.md#Items receiving verification updates (215)`

Some cited files are gone: `crates/roko-serve/src/routes/events.rs`.

How to verify: Check: `cargo test -p roko-serve` passes, including new endpoint tests, in the release/full-CI lane.; Run a real plan and cross-check its API records against its direct JSONL/index records.; Manual: query filtered event types/sources and attempt… [evidence: own status: Verified (2026-09-03) — 12 routes, cursor pagination, offline repair; 00-STATUS-SUMMARY 2. Partial / : SOURCE-DONE; loopback API/cursor/SSE checkpoint…]
