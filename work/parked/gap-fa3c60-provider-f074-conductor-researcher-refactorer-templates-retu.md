+++
id = "gap-fa3c60"
kind = "gap"
title = "[provider F074] Conductor, Researcher, Refactorer templates return zero sections"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-compose/templates"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F074"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F074"
anchors = ["crates/roko-compose/src/templates/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
`ConductorTemplate`, `ResearcherTemplate`, and `RefactorerTemplate` implement `sections()` returning `vec![]`. They provide `role_identity()` only. The template section mechanism is unused for these roles; all contextual prompt content must come from `RoleSystemPromptSpec` callers.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F074`
- `tmp/archive/provider-audit/15-prompt-composition.md`

How to verify: Confirm in crates/roko-compose/src/templates/ whether still true: Conductor, Researcher, Refactorer templates return zero sections
