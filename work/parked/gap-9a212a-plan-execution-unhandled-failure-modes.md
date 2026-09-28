+++
id = "gap-9a212a"
kind = "gap"
title = "Plan Execution Unhandled Failure Modes"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli"]
created = 2026-09-07
updated = 2026-09-28
source = "tmp/backlog/archive/103-plan-execution-resilience.md#103 — Plan Execution Unhandled Failure Modes"
discovered_from = "audit:tmp/backlog/archive/103-plan-execution-resilience.md#103 — Plan Execution Unhandled Failure Modes"
anchors = ["crates/roko-cli/", "crates/roko-agent/", "crates/roko-cli/src/runner/event_loop.rs", "crates/roko-agent/src/http.rs", "crates/roko-agent/src/provider/anthropic_api.rs:122", "crates/roko-agent/src/openai_compat_backend.rs:52", "crates/roko-cli/src/runner/types.rs", "crates/roko-cli/src/runner/snapshot_writer.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
reliability; four gaps cause silent data loss or missed recovery opportunities during long-running plan execution. The plan execution loop in roko (`roko plan run`) handles many failure modes well: it checks disk space before starting, prevents concurrent runs via file locking, classifies agent…

Imported without verification from:
- `tmp/backlog/archive/103-plan-execution-resilience.md#103 — Plan Execution Unhandled Failure Modes`

Some cited files are gone: `crates/roko-cli/src/runner/event_loop.rs`, `crates/roko-cli/src/runner/snapshot_writer.rs`.

How to verify: Check: Disk space is checked before each gate rung evaluation, not only at startup. A critically; When an agent response includes a `Retry-After` header (429/529), the runner waits at; Worktree creation is retried up to 3 times with exponential… [evidence: 00-STATUS-SUMMARY 3. Open / P2 -- Medium (Open): M | 3 |]
