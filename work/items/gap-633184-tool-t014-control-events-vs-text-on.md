+++
id = "gap-633184"
kind = "gap"
title = "[tool T014] Control events vs text on bounded paths: zero-silent-drop under backpressure not tested"
status = "open"
triage = "verified"
severity = "p2"
subsystem = ["roko-core/transcript_store"]
created = 2026-09-14
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/archive/tool-audit-2026-09-21/10-FINDINGS-REGISTER.md#register"
discovered_from = "audit:tmp/archive/tool-audit-2026-09-21/10-FINDINGS-REGISTER.md#register"
anchors = ["crates/roko-core/src/transcript_store.rs::is_control_event", "crates/roko-core/src/transcript_store.rs::drop_report"]
links = { depends_on = [], blocks = [], related = ["gap-9c8ac0", "gap-836ae9"], supersedes = [], duplicate_of = "" }
+++
TranscriptStore eviction preserves control events (On main), but release gate 'Control events survive the long-stream/backpressure scenario with zero silent drops' remains unchecked (not yet tested).

Imported without verification from:
- `tmp/archive/tool-audit-2026-09-21/10-FINDINGS-REGISTER.md#register`
- `tmp/archive/tool-audit-2026-09-21/11-IMPLEMENTATION-CHECKLIST.md#release-gate`
- `tmp/archive/tool-audit-2026-09-21/00-INDEX.md#status-note-2026-09-14`

How to verify: Search for a long-stream/backpressure test asserting no control/terminal tool events are dropped; check bounded channels on the provider->TUI path use the documented drop policy.

Verified 2026-09-28: store-level unit tests exist: crates/roko-core/src/transcript_store.rs:775 eviction_drops_text_deltas_before_control_events and :800 control_events_never_evicted, with drop accounting via drop_report (:574). No long-stream/backpressure scenario test on the provider->TUI path asserts zero silent drops of control or terminal events, so the release gate is still unchecked. Severity p2 (missing proof).
