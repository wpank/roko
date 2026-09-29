+++
id = "bug-194c8f"
kind = "bug"
title = "`--json` flag silently ignored on ~15+ commands"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/cli"]
created = 2026-09-15
updated = 2026-09-28
source = "docs/v3/39-ROADMAP.md#3.4 CLI Surface Fixes"
discovered_from = "audit:docs/v3/39-ROADMAP.md#3.4 CLI Surface Fixes"
anchors = ["crates/roko-cli/src/commands/"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Roadmap §3.4: --json is accepted but silently ignored on ~15+ commands; implement JSON output or remove the flag.

Imported without verification from:
- `docs/v3/39-ROADMAP.md#3.4 CLI Surface Fixes`

How to verify: Run each subcommand with --json and check output is valid JSON.
