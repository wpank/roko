+++
id = "gap-52df4b"
kind = "gap"
title = "Provider-role routing (Codex/PRD)"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-agent"]
created = 2026-09-01
updated = 2026-10-02
source = "tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#6.6 Provider-role routing (Codex/PRD)"
discovered_from = "audit:tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#6.6 Provider-role routing (Codex/PRD)"
anchors = []
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Provider roles are not explicit enough: Codex works for implementation but is poor for PRD/research. Needs role-aware provider selection.

Imported without verification from:
- `tmp/archive/MASTER-ACTION-PLAN-2026-09-23.md#6.6 Provider-role routing (Codex/PRD)`

How to verify: Source: Dogfood audit, P2 finding. Check the described code path for: Provider roles are not explicit enough: Codex works for implementation but is poor for PRD/research. Needs role-aware provider selection.

2026-10-02 (roko-7d): The PRD commands were removed (merge bfd36512f); only the Codex and research routing can still apply. Still parked.
