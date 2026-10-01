+++
id = "find-b2ae71"
kind = "finding"
title = "[provider F028] ACP implements spec 0.12.2; current spec is 0.13"
status = "superseded"
triage = "verified"
severity = "p1"
subsystem = ["roko-acp"]
created = 2026-09-01
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F028"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F028"
anchors = ["crates/roko-acp/src/types.rs:10"]
links = { depends_on = [], blocks = [], related = ["spec-704c28"], supersedes = [], duplicate_of = "gap-83d081" }

[closed]
at = 2026-09-28
evidence = "duplicate of gap-83d081 (ACP spec upgrade from v0.12 to v0.13); the broader v2 scope is in spec-704c28. Still true at HEAD 91b4745f8: ACP_SPEC_VERSION is 0.12.2 (crates/roko-acp/src/types.rs:10). Whether 0.13 is the current upstream spec was not re-checked (no network)."
+++
The ACP server implements version 0.12.2 of the ACP protocol specification. The current specification is 0.13. The gap introduces potential incompatibilities with ACP clients (Cursor, external agents) that expect 0.13 behaviors.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F028`
- `tmp/archive/provider-audit/04-acp-integration.md`

How to verify: Confirm in crates/roko-acp/src/ whether still true: ACP implements spec 0.12.2; current spec is 0.13

Verified 2026-09-28: superseded as a duplicate; see `[closed].evidence`.
