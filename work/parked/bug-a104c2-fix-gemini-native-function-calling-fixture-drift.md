+++
id = "bug-a104c2"
kind = "bug"
title = "Fix Gemini native function-calling fixture drift"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-agent/gemini"]
created = 2026-09-21
updated = 2026-09-28
source = "crates/roko-agent/tests/gemini_integration.rs:246"
discovered_from = "audit:crates/roko-agent/tests/gemini_integration.rs:246"
anchors = ["crates/roko-agent/tests/gemini_integration.rs::gemini_native_generate_content_with_function_calling"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
gemini_native_generate_content_with_function_calling is ignored as 'gemini function calling fixture drift — pre-existing'; native Gemini function-calling parsing lacks a passing regression test.

Imported without verification from:
- `crates/roko-agent/tests/gemini_integration.rs:246`

How to verify: Run with --ignored; compare Gemini response fixture against current generateContent function-call schema.
