+++
id = "bug-49c2f0"
kind = "bug"
title = "DOCS-06 A6: `roko init` template emits [github]/[resources] sections the config schema does not recognize"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-core/config"]
created = 2026-09-15
updated = 2026-09-28
source = "tmp/docs-audit/06-POTENTIAL-BACKLOG.md#A6. Config init generates unrecognized sections"
discovered_from = "audit:tmp/docs-audit/06-POTENTIAL-BACKLOG.md#A6. Config init generates unrecognized sections"
anchors = ["roko init", "roko config validate"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Generated roko.toml contains sections the loader does not recognize, so validation may warn/fail (decision: add to schema and fix template).

Imported without verification from:
- `tmp/docs-audit/06-POTENTIAL-BACKLOG.md#A6. Config init generates unrecognized sections`
- `tmp/docs-audit/03-SUPERSEDED-AND-DEAD.md#Config Issues`
- `tmp/dogfood/2026-09-18-session.md#Final Summary`

A source claims this was fixed; confirm against current code before closing.

How to verify: roko init in a temp dir then roko config validate.
