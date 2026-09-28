+++
id = "bug-331613"
kind = "bug"
title = "[cli-audit inconsistencies] `--json` ignored by ~15+ subcommands (PRD, knowledge, learn, agent status)"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/commands"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/archive/cli-audit-2026-09-21/SUMMARY.md#Inconsistencies"
discovered_from = "audit:tmp/archive/cli-audit-2026-09-21/SUMMARY.md#Inconsistencies"
anchors = ["crates/roko-cli/src/commands/prd.rs", "crates/roko-cli/src/commands/knowledge*", "dispatch_learn", "agent status"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Global --json is silently ignored by ~15+ subcommands: all 9 PRD commands, 6 knowledge commands, learn tune/inspect, agent status. SUMMARY item 14 still OPEN; #309/#311/#303 may have fixed subsets.

Imported without verification from:
- `tmp/archive/cli-audit-2026-09-21/SUMMARY.md#Inconsistencies`
- `tmp/archive/cli-audit-2026-09-21/SUMMARY.md#Recommended Priority Order`

Some cited files are gone: `crates/roko-cli/src/commands/knowledge*`.

How to verify: Run each listed subcommand with --json and check for structured JSON output; grep handlers for cli.json usage.
