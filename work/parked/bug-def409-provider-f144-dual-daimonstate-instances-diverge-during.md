+++
id = "bug-def409"
kind = "bug"
title = "[provider F144] Dual DaimonState instances diverge during a run"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/runner"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F144"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F144"
anchors = ["crates/roko-cli/src/runner/event_loop.rs", "crates/roko-learn/src/runtime_feedback.rs", "DaimonState"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Runner-v2 loads one `DaimonState` instance updated in real-time. `RuntimeFeedbackEngine` loads a separate instance from the same file. During a run, they diverge because the learn instance is not notified of runner updates.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F144`
- `tmp/archive/provider-audit/16-daimon-affect.md`

Warning: every file this item cites is gone (`crates/roko-cli/src/runner/event_loop.rs`, `crates/roko-learn/src/runtime_feedback.rs`) — likely obsolete or moved.

How to verify: Confirm in crates/roko-cli/src/runner/event_loop.rs, crates/roko-learn/src/runtime_feedback.rs whether still true: Dual `DaimonState` instances diverge during a run Runner-v2 event_loop.rs (the audited location) was deleted 2026-09-06; check whether the Graph path (graph_task_dispatch.rs / graph_execution/) has the same behavior.
