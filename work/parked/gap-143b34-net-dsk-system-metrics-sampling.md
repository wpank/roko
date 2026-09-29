+++
id = "gap-143b34"
kind = "gap"
title = "NET/DSK System Metrics Sampling"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/tui"]
created = 2026-09-07
updated = 2026-09-28
source = "tmp/backlog/archive/240-net-dsk-system-metrics-sampling.md#240 — NET/DSK System Metrics Sampling"
discovered_from = "audit:tmp/backlog/archive/240-net-dsk-system-metrics-sampling.md#240 — NET/DSK System Metrics Sampling"
anchors = ["crates/roko-cli/src/tui/app.rs", "crates/roko-cli/src/tui/state.rs", "app.rs:3628-3654", "state.rs:1134-1154", "state.rs:1153-1154", "app.rs:3638", "collect_sys_metrics_bg()", "SysSnapshot"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
The `collect_sys_metrics_bg` background thread only samples CPU and MEM; the `SysMetrics` struct has fields for network and disk I/O but they are never populated.. The `collect_sys_metrics_bg` function at `app.rs:3628` runs on a dedicated background thread and samples system metrics every 2…

Imported without verification from:
- `tmp/backlog/archive/240-net-dsk-system-metrics-sampling.md#240 — NET/DSK System Metrics Sampling`

Warning: every file this item cites is gone (`crates/roko-cli/src/tui/app.rs`, `crates/roko-cli/src/tui/state.rs`) — likely obsolete or moved.

How to verify: Check: `SysMetrics.net_down_bytes_sec` shows non-zero values when network traffic is present.; `SysMetrics.disk_read_bytes_sec` shows non-zero values during disk activity.; Rate values are per-second (not cumulative totals). [evidence: 00-STATUS-SUMMARY 3. Open / P2 -- Medium (Open): XS | 4 |]
