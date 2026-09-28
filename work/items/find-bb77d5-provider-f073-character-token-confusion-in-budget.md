+++
id = "find-bb77d5"
kind = "finding"
title = "[provider F073] Character/token confusion in budget system"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-compose/system_prompt_builder"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F073"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F073"
anchors = ["crates/roko-compose/src/system_prompt_builder.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
`PromptBudget` fields are documented and used as character counts. `PromptSection.hard_cap` is documented as "token ceiling" but is used as characters in some paths. `estimate_tokens` uses `bytes/4`. The budget system mixes characters and tokens inconsistently.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F073`
- `tmp/archive/provider-audit/15-prompt-composition.md`

How to verify: Confirm in crates/roko-compose/src/system_prompt_builder.rs whether still true: Character/token confusion in budget system
