+++
id = "find-1e7c98"
kind = "finding"
title = "S7: Hardcoded Values Scattered Across the Codebase"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["workspace"]
created = 2026-04-28
updated = 2026-09-28
source = "tmp/archive/08-15-26/binary-issues/MASTER-INDEX.md#S7. Hardcoded Values Scattered Across the Codebase"
discovered_from = "audit:tmp/archive/08-15-26/binary-issues/MASTER-INDEX.md#S7. Hardcoded Values Scattered Across the Codebase"
anchors = ["dispatch_direct.rs:291", "dispatch_direct.rs:374", "orchestrate.rs:4982-12783", "dispatch_direct.rs:300", "dispatch_direct.rs:302", "dispatch_direct.rs:295,379", "chat_inline.rs:3516", "cost_table.rs:99-122"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Systemic audit finding (2026-04-28) with 6 open checklist fixes: S7.1-3 Consolidate model defaults into one const; S7.4-5 Move API URLs and versions to provider co; S7.6 Make max_tokens configurable (per-provider ; S7.7 Configurable baseline model for savings cal; S7.8 Load pricing from config…

Imported without verification from:
- `tmp/archive/08-15-26/binary-issues/MASTER-INDEX.md#S7. Hardcoded Values Scattered Across the Codebase`

How to verify: Check each open sub-item (S7.1-3, S7.4-5, S7.6, S7.7, S7.8, S7.9). grep for hardcoded model ids, API URLs/versions, max_tokens, pricing and web-search constants in roko-agent/roko-cli.
