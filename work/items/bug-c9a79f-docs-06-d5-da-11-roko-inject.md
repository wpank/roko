+++
id = "bug-c9a79f"
kind = "bug"
title = "DOCS-06 D5 / DA-11: `roko inject` is a stub that never sends signals"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/inject"]
created = 2026-09-15
updated = 2026-09-28
source = "tmp/docs-audit/06-POTENTIAL-BACKLOG.md#D5. `roko inject` stub command"
discovered_from = "audit:tmp/docs-audit/06-POTENTIAL-BACKLOG.md#D5. `roko inject` stub command"
anchors = ["roko inject"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
The command accepts arguments but never injects anything (CLI audit critical finding #1); decision: wire it to actually send signals. 09-03 claims inject now writes a real Signal via FileSubstrate::put; conflicts with 09-15.

Imported without verification from:
- `tmp/docs-audit/06-POTENTIAL-BACKLOG.md#D5. `roko inject` stub command`
- `tmp/dev-audit/11-implementation-status.md#New risks surfaced by the audits that affect dev-audit scope`
- `tmp/archive/dogfood-audit-2026-09-03/06-status-update-2026-09-03.md#Cross-Audit Findings (from 2026-09-01 register)`
- `docs/v3/39-ROADMAP.md#3.4 CLI Surface Fixes`
- `tmp/nous-research/AUDIT-SUMMARY-2026-09-04.md#6. CLI Audit`

A source claims this was fixed; confirm against current code before closing.

How to verify: Run `roko inject <session> <payload>` and check signal log/bus. / Run roko inject <session> <payload> and check the signal log.

Merged 2 mined candidates: m4-049, m5-081.
