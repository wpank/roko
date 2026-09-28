+++
id = "find-104db9"
kind = "finding"
title = "[provider F140] Layer numbering in doc comments does not match actual emission order"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-compose/system_prompt_builder"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F140"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F140"
anchors = ["crates/roko-compose/src/system_prompt_builder.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
The `SystemPromptBuilder` documents 9 layers with sequential numbers (1–8). The actual emission order is determined by `(CacheLayer, Priority, index)` sort keys, not layer numbers. Layer 5 (tool_instructions) emits with Role tier content, not in position 5.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F140`
- `tmp/archive/provider-audit/15-prompt-composition.md`

How to verify: Confirm in crates/roko-compose/src/system_prompt_builder.rs whether still true: Layer numbering in doc comments does not match actual emission order
