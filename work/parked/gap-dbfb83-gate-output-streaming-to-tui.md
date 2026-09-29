+++
id = "gap-dbfb83"
kind = "gap"
title = "Gate Output Streaming to TUI"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/runner"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/backlog/archive/234-gate-output-streaming-tui.md#234 — Gate Output Streaming to TUI"
discovered_from = "audit:tmp/backlog/archive/234-gate-output-streaming-tui.md#234 — Gate Output Streaming to TUI"
anchors = ["crates/roko-cli/src/runner/event_loop.rs", "crates/roko-cli/src/tui/widgets/", "crates/roko-cli/src/tui/views/dashboard_view.rs", "crates/roko-core/src/dashboard_snapshot.rs", "runner/types.rs:198", "dashboard_snapshot.rs:134", "runner/event_loop.rs", "input.rs:51", "crates/roko-cli/src/tui/", "crates/roko-gate/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
During the 30-120 seconds that gates run, the TUI shows nothing: no progress, no output, no indication of what is compiling or testing.. Gate execution captures full cargo/test stdout/stderr in the `GateCompletion.output` field at `runner/types.rs:210`. But `DashboardEvent::GateResult` (defined at…

Imported without verification from:
- `tmp/backlog/archive/234-gate-output-streaming-tui.md#234 — Gate Output Streaming to TUI`
- `tmp/backlog/archive/385-tui-gate-output-streaming.md#385 — TUI Gate Output True Line-by-Line Streaming`

Some cited files are gone: `crates/roko-cli/src/runner/event_loop.rs`, `runner/event_loop.rs`.

How to verify: Check: During gate execution, the TUI shows a "Gate Output" panel with live rung label and elapsed timer.; After gate completion, the full cargo/test output is visible and scrollable.; Pass/fail lines are color-coded (green/red). [evidence: own status: Verified (2026-09-03) — event forwarding, color widget, auto-scroll; 00-STATUS-SUMMARY 3. Open / P1 -- High (Open): M | 3 |; (newer evidence overrides own…] / Check whether the gap described in tmp/backlog/archive/385-tui-gate-output-streaming.md still exists at the anchored paths. [evidence: 00-INDEX (2026-09-21) listed active: 2026-09-21 Audit Sweep Items (#376-#395)]

Merged 2 mined candidates: m1-066, m1-108.
