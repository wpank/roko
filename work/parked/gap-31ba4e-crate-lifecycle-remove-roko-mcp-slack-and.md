+++
id = "gap-31ba4e"
kind = "gap"
title = "Crate lifecycle: remove roko-mcp-slack and roko-mcp-scripts; exclude roko-demo from default builds"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["workspace"]
created = 2026-09-15
updated = 2026-09-28
source = "docs/v3/39-ROADMAP.md#8. Crate Lifecycle"
discovered_from = "audit:docs/v3/39-ROADMAP.md#8. Crate Lifecycle"
anchors = ["Cargo.toml [workspace.members]"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Roadmap §8: roko-mcp-slack (1.9K LOC) and roko-mcp-scripts (766 LOC) were never wired and should be removed; roko-demo (5.8K LOC, zero callers, alloy dependency) should be excluded from default builds.

Imported without verification from:
- `docs/v3/39-ROADMAP.md#8. Crate Lifecycle`

How to verify: Check workspace members/default-members.
