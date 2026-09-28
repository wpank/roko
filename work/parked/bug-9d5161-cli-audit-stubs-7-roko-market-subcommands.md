+++
id = "bug-9d5161"
kind = "bug"
title = "[cli-audit stubs] 7 `roko market` subcommands print \"not yet implemented\" and exit 0"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/market"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/archive/cli-audit-2026-09-21/SUMMARY.md#Stubs & Unimplemented"
discovered_from = "audit:tmp/archive/cli-audit-2026-09-21/SUMMARY.md#Stubs & Unimplemented"
anchors = ["cmd_market (main.rs ~2063 at audit time)"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
market browse/show/install/uninstall/fork/publish/verify all route to a minimal cmd_market stub that prints the subcommand name + "not yet implemented" and returns EXIT_SUCCESS (fake success).

Imported without verification from:
- `tmp/archive/cli-audit-2026-09-21/SUMMARY.md#Stubs & Unimplemented`
- `tmp/archive/cli-audit-2026-09-21/00-main-structure.md`

How to verify: Check if `market` command still exists; run `roko market browse` and check exit code/output.
