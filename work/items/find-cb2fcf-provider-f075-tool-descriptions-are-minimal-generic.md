+++
id = "find-cb2fcf"
kind = "finding"
title = "[provider F075] Tool descriptions are minimal generic text, not coordinated with provider schemas"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-compose/templates"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F075"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F075"
anchors = ["crates/roko-compose/src/templates/common.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
The tool instructions prompt section lists tool names in a comma-separated text block with a generic "use MCP tools" message. Structured tool definitions sent to providers (JSON schemas) are assembled independently. The text-level and schema-level tool descriptions may diverge.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F075`
- `tmp/archive/provider-audit/15-prompt-composition.md`

How to verify: Confirm in crates/roko-compose/src/templates/common.rs whether still true: Tool descriptions are minimal generic text, not coordinated with provider schemas
