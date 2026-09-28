+++
id = "bug-24ef3a"
kind = "bug"
title = "[refactor --effort] `--effort` flag silently ignored by Graph plan runs"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/commands"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/archive/refactoring-audit-2026-09-21/GRAPH-RUNNER-PARITY.md#behavioral-items-not-ported"
discovered_from = "audit:tmp/archive/refactoring-audit-2026-09-21/GRAPH-RUNNER-PARITY.md#behavioral-items-not-ported"
anchors = ["crates/roko-cli/src/commands/plan.rs", "--effort", "default_effort"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Runner-v2 honored a per-plan effort tier override; Graph silently ignores --effort and uses default_effort from config (silent-flag anti-pattern).

Imported without verification from:
- `tmp/archive/refactoring-audit-2026-09-21/GRAPH-RUNNER-PARITY.md#behavioral-items-not-ported`
- `tmp/archive/refactoring-audit-2026-09-21/GRAPH-RUNNER-PARITY.md#5-consider-porting-dropped-behaviors-p3`

How to verify: Check whether `roko plan run --effort` reaches Graph dispatch or at least warns.
