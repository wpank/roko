+++
id = "bug-46b5c4"
kind = "bug"
title = "[cli-audit agent] `agent create` --skills/--tier/--reputation/--max-concurrent-jobs not persisted"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/agent"]
created = 2026-08-31
updated = 2026-09-28
source = "tmp/archive/cli-audit-2026-09-21/04-agent.md"
discovered_from = "audit:tmp/archive/cli-audit-2026-09-21/04-agent.md"
anchors = ["04-agent.md:40-43", "roko agent create"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
These flags are only forwarded in the serve registration JSON when --serve-url is set; they are not stored in the agent manifest.

Imported without verification from:
- `tmp/archive/cli-audit-2026-09-21/04-agent.md`

How to verify: Run agent create with the flags without --serve-url; inspect written manifest.
