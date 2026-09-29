+++
id = "bug-000aa6"
kind = "bug"
title = "[provider F136] sanitize_audit_label truncates without warning"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-agent/tool_loop"]
created = 2026-09-01
updated = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F136"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F136"
anchors = ["crates/roko-agent/src/tool_loop/scrub.rs", "sanitize_audit_label"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
`sanitize_audit_label` truncates labels to a fixed maximum length without logging that truncation occurred. Audit logs may contain truncated labels that silently lose information.

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F136`
- `tmp/archive/provider-audit/14-tool-loop.md`

Warning: every file this item cites is gone (`crates/roko-agent/src/tool_loop/scrub.rs`) — likely obsolete or moved.

How to verify: Confirm in crates/roko-agent/src/tool_loop/scrub.rs whether still true: `sanitize_audit_label` truncates without warning
