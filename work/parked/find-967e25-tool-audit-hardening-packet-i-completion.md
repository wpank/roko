+++
id = "find-967e25"
kind = "finding"
title = "Tool Audit Hardening (Packet I Completion)"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-core"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/backlog/archive/384-tool-audit-hardening-packet-i.md#384 — Tool Audit Hardening (Packet I Completion)"
discovered_from = "audit:tmp/backlog/archive/384-tool-audit-hardening-packet-i.md#384 — Tool Audit Hardening (Packet I Completion)"
anchors = ["crates/roko-core/", "crates/roko-agent/", "crates/roko-cli/src/transcript/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
acceptance evidence missing for tool lifecycle. The tool-audit found 36 findings across 9 implementation packets (A-H merged to main). Packet I (hardening) remains incomplete: criterion benchmarks exist but haven't been executed, TUI terminal-size snapshots not generated, soak/disk-full tests not…

Imported without verification from:
- `tmp/backlog/archive/384-tool-audit-hardening-packet-i.md#384 — Tool Audit Hardening (Packet I Completion)`

How to verify: Check whether the gap described in tmp/backlog/archive/384-tool-audit-hardening-packet-i.md still exists at the anchored paths. [evidence: 00-INDEX (2026-09-21) listed active: 2026-09-21 Audit Sweep Items (#376-#395)]
