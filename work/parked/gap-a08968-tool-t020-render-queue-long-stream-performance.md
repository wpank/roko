+++
id = "gap-a08968"
kind = "gap"
title = "[tool T020] Render/queue/long-stream performance lacks an evidence ledger (criterion benches never run)"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-core/benches"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/archive/tool-audit-2026-09-21/10-FINDINGS-REGISTER.md#register"
discovered_from = "audit:tmp/archive/tool-audit-2026-09-21/10-FINDINGS-REGISTER.md#register"
anchors = ["crates/roko-core/benches/transcript_bench.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Only finding still In progress: roko-core/benches/transcript_bench.rs exists on main but has never been executed or recorded; soak tests also unrecorded.

Imported without verification from:
- `tmp/archive/tool-audit-2026-09-21/10-FINDINGS-REGISTER.md#register`
- `tmp/archive/tool-audit-2026-09-21/11-IMPLEMENTATION-CHECKLIST.md#release-gate`
- `tmp/archive/tool-audit-2026-09-21/00-INDEX.md#status-note-2026-09-14`

How to verify: Check the bench compiles and has recorded baseline results anywhere (RUN-LEDGER or CI); check Cargo.toml [[bench]] entry.
