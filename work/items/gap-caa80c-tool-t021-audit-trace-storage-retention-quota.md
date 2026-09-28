+++
id = "gap-caa80c"
kind = "gap"
title = "[tool T021] Audit/trace storage retention/quota/disk-failure contract untested (disk-full not covered)"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-fs/observability"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/archive/tool-audit-2026-09-21/10-FINDINGS-REGISTER.md#register"
discovered_from = "audit:tmp/archive/tool-audit-2026-09-21/10-FINDINGS-REGISTER.md#register"
anchors = ["crates/roko-fs/src/observability.rs", "crates/roko-fs/src/trace_sink.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
RetentionPolicy type exists in roko-fs observability (On main), but release gate item for disk-full/cancellation/timeout/handler-panic/malformed-args/provider-disconnect coverage is only partial: disk-full and soak tests not recorded.

Imported without verification from:
- `tmp/archive/tool-audit-2026-09-21/10-FINDINGS-REGISTER.md#register`
- `tmp/archive/tool-audit-2026-09-21/11-IMPLEMENTATION-CHECKLIST.md#release-gate`
- `tmp/archive/tool-audit-2026-09-21/00-INDEX.md#status-note-2026-09-14`

How to verify: Check RetentionPolicy is enforced by a production sink (not just a type) and whether any disk-full/ENOSPC test exists for trace/audit writers.
