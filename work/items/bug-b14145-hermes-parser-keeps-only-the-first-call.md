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
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "c58c7c2ba"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-agent/src/translate/hermes.rs::tool_call_from_value"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'fn parse_handles_tool_calls_array_wrapper_with_two_entries' crates/roko-agent/src/translate/hermes.rs && cargo test -p roko-agent --lib translate::hermes::tests::parse_handles_tool_calls_array_wrapper_with_two_entries"
+++

`tool_call_from_value` (`translate/hermes.rs:217-227`) unpacks a `{"tool_calls": [...]}` body but returns only the first element; the rest are dropped without a warning, although the module doc (`:36`) says such arrays are unpacked.
Fix: return every entry in order with sequential ids; extend `parse_handles_tool_calls_array_wrapper` (`:479`) to two entries.

## Notes

- 2026-10-01 (wk-tiers): implemented on work/bug-7567eb; cargo verification deferred to the batch check.
  `tool_call_from_value` and `parse_tool_call_body` return every call, so a `{"tool_calls": [...]}` wrapper gives
  one call per entry, in order. An entry without a `"name"` is skipped. Ids continue from the calls already parsed,
  across blocks too. Test: `parse_handles_tool_calls_array_wrapper_with_two_entries`.
