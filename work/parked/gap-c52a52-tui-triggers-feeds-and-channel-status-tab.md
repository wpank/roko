+++
id = "gap-c52a52"
kind = "gap"
title = "TUI Triggers, Feeds, and Channel Status Tab"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/tui"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/backlog/archive/424-tui-triggers-feeds-tab.md#424 — TUI Triggers, Feeds, and Channel Status Tab"
discovered_from = "audit:tmp/backlog/archive/424-tui-triggers-feeds-tab.md#424 — TUI Triggers, Feeds, and Channel Status Tab"
anchors = ["crates/roko-cli/src/tui/", "crates/roko-cli/src/tui/views/mod.rs", "crates/roko-cli/src/tui/views/config_view.rs", "crates/roko-cli/src/tui/dashboard.rs", "crates/roko-cli/src/tui/state.rs", "roko-serve/src/routes/triggers.rs", "roko-serve/src/routes/feeds.rs", "roko-core/src/trigger.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
The TUI (`roko dashboard`) has 11 tabs (F1-F11) but none surface trigger bindings, feed status, or channel health. The F6 Config tab currently has three sub-views: `ConfigEditor`, `ProviderHealth`, `ModelComparison` (defined in `crates/roko-cli/src/tui/views/mod.rs`…

Imported without verification from:
- `tmp/backlog/archive/424-tui-triggers-feeds-tab.md#424 — TUI Triggers, Feeds, and Channel Status Tab`

Some cited files are gone: `crates/roko-cli/src/tui/state.rs`.

How to verify: Check: F6 tab shows 6 sub-views selectable via number keys 1-6; Trigger sub-view lists bindings with kind badges, armed status, sparkline; `f` key fires selected trigger; result shown in status bar [evidence: 00-INDEX (2026-09-21) listed active: 2026-09-21 Channel/Webhook/Reactive Audit (#409-#4]
