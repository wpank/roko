+++
id = "find-f97e28"
kind = "finding"
title = "[provider F160] Cache hit rate in TUI is arithmetic mean of per-task rates, not weighted aggregate"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/tui"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F160"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F160"
anchors = ["crates/roko-cli/src/tui/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
The `LearningTab` cache hit rate displays the average of per-task `cache_hit_rate` values. A task with 100 tokens and 0% cache hit has the same weight as a task with 100,000 tokens. The displayed rate is misleading for heterogeneous workloads.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F160`
- `tmp/archive/provider-audit/19-caching.md`

How to verify: Confirm in crates/roko-cli/src/tui/ whether still true: Cache hit rate in TUI is arithmetic mean of per-task rates, not weighted aggregate
