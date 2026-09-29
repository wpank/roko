+++
id = "bug-822dfa"
kind = "bug"
title = "De-flake plugin file-watch event source test (OS notification timing)"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-plugin/events"]
created = 2026-09-05
updated = 2026-09-28
source = "crates/roko-plugin/src/lib.rs:923"
discovered_from = "audit:crates/roko-plugin/src/lib.rs:923"
anchors = ["crates/roko-plugin/src/lib.rs::file_watch_event_source_emits_create_modify_delete_signals"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
file_watch_event_source_emits_create_modify_delete_signals is ignored as flaky because watcher timing depends on OS notification delivery; the plugin file-watch event source has no reliable test.

Imported without verification from:
- `crates/roko-plugin/src/lib.rs:923`

How to verify: Run with --ignored repeatedly; switch to polling with timeout/retry assertions.
