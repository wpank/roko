+++
id = "gap-f1feaf"
kind = "gap"
title = "Deprecated compatibility shims awaiting removal (#342 blocked)"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["workspace/deprecations"]
created = 2026-09-01
updated = 2026-09-28
source = "gaps-md#from-cli-audit-tmpcli-auditsummarymd/deprecated-items"
anchors = ["crates/roko-cli/src/runner/mod.rs", "crates/roko-cli/src/tui/dashboard.rs", "crates/roko-core/src/agent.rs"]
links = { depends_on = [], blocks = [], related = ["bug-96aff4"], supersedes = [], duplicate_of = "" }
+++

The audit found 8 `#[deprecated]` items with zero callers. There are now 11 `#[deprecated]` attributes, including:
- `crates/roko-cli/src/runner/mod.rs`, the Runner-v2 stub, which still has callers (bug-96aff4);
- `crates/roko-cli/src/tui/dashboard.rs`;
- `crates/roko-gate/src/chaos.rs`;
- `crates/roko-conductor/src/stuck_detection.rs`;
- `crates/roko-core/src/agent.rs`.
Backlog #342 (retire deprecated shims) is archived as blocked on #43, #276 and #283.

Fix: remove each deprecated item once its callers have migrated. Keep the Runner-v2 stub until its callers move to the Graph engine.
