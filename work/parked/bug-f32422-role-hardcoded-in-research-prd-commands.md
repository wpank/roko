+++
id = "bug-f32422"
kind = "bug"
title = "`--role` hardcoded in research/PRD commands"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/cli"]
created = 2026-09-15
updated = 2026-10-02
source = "docs/v3/39-ROADMAP.md#3.4 CLI Surface Fixes"
discovered_from = "audit:docs/v3/39-ROADMAP.md#3.4 CLI Surface Fixes"
anchors = ["crates/roko-cli/src/prd.rs", "roko research"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Roadmap §3.4: the --role flag is hardcoded in research and PRD commands instead of using the specified role.

Imported without verification from:
- `docs/v3/39-ROADMAP.md#3.4 CLI Surface Fixes`

How to verify: grep research/prd command handlers for fixed role strings.

2026-10-02 (roko-7d): The PRD commands were removed (merge bfd36512f); only the research half can still apply. Re-check `crates/roko-cli/src/research.rs` before reviving; still parked.
