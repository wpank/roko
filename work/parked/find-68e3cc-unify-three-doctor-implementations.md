+++
id = "find-68e3cc"
kind = "finding"
title = "Unify three doctor implementations"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/doctor"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#6.2 Unify three doctor implementations"
discovered_from = "audit:tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#6.2 Unify three doctor implementations"
anchors = ["doctor.rs", "config_cmd.rs", "chat_inline.rs", "runner/preflight.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Four separate diagnostic implementations with no shared code and divergent checks. Extract a shared diagnostic service.

Imported without verification from:
- `tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#6.2 Unify three doctor implementations`

How to verify: Source: CLI audit report 11; engine audit report 05. Check `doctor.rs`, `config_cmd.rs`, `chat_inline.rs`, `runner/preflight.rs` for: Four separate diagnostic implementations with no shared code and divergent checks. Extract a shared diagnostic service.
