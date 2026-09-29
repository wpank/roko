+++
id = "bug-cbdb38"
kind = "bug"
title = "roko-cli failed to compile: CliProviderConfig has no field require_confirmation"
status = "parked"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli"]
created = 2026-09-04
updated = 2026-09-28
source = "tmp/nous-research/ROKO-STATE-2026-09-04.md#Compilation Status: BROKEN"
discovered_from = "audit:tmp/nous-research/ROKO-STATE-2026-09-04.md#Compilation Status: BROKEN"
anchors = ["CliProviderConfig"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
Snapshot 2026-09-04: roko-cli lib failed with 2x E0560 (CliProviderConfig has no field require_confirmation) on feat/provider-audit-implementation. Likely fixed since; confirm and add a regression check.

Imported without verification from:
- `tmp/nous-research/ROKO-STATE-2026-09-04.md#Compilation Status: BROKEN`

How to verify: cargo check -p roko-cli on main; grep require_confirmation.
