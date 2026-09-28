+++
id = "bug-b14145"
kind = "bug"
title = "Hermes parser keeps only the first call of a tool_calls wrapper block"
status = "open"
triage = "verified"
severity = "p2"
goal = "hermes"
subsystem = ["roko-agent/translate"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-agent/src/translate/hermes.rs::tool_call_from_value"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'cargo test -p roko-agent --lib translate::hermes'
+++

`tool_call_from_value` (`translate/hermes.rs:217-227`) unpacks a `{"tool_calls": [...]}` body but returns only the first element; the rest are dropped without a warning, although the module doc (`:36`) says such arrays are unpacked.
Fix: return every entry in order with sequential ids; extend `parse_handles_tool_calls_array_wrapper` (`:479`) to two entries.
