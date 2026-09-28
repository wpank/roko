+++
id = "gap-8b2867"
kind = "gap"
title = "Engine audit residue: duplicate code paths (4 doctor, 6 chat, 11 model-resolution fns) and ~30 flag inconsistencies"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli"]
created = 2026-09-04
updated = 2026-09-28
source = "tmp/nous-research/AUDIT-SUMMARY-2026-09-04.md#5. Engine Audit (`tmp/engine-audit/`, 15 files)"
discovered_from = "audit:tmp/nous-research/AUDIT-SUMMARY-2026-09-04.md#5. Engine Audit (`tmp/engine-audit/`, 15 files)"
anchors = ["crates/roko-cli/src/doctor.rs", "crates/roko-cli/src/chat.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Beyond Graph parity: 7+ duplicate code paths (4 doctor implementations, 6 chat paths, 11 model resolution functions) and ~30 flag inconsistencies/dead flags in the CLI.

Imported without verification from:
- `tmp/nous-research/AUDIT-SUMMARY-2026-09-04.md#5. Engine Audit (`tmp/engine-audit/`, 15 files)`
- `tmp/engine-audit/`

How to verify: Count model-resolution helpers and chat entry points on main.
