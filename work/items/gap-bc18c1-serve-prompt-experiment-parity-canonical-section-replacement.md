+++
id = "gap-bc18c1"
kind = "gap"
title = "Serve prompt-experiment parity (canonical section replacement under durable receipts)"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-serve/experiments"]
created = 2026-09-15
updated = 2026-09-28
source = "docs/v3/39-ROADMAP.md#7.6 Serve Experiment Parity"
discovered_from = "audit:docs/v3/39-ROADMAP.md#7.6 Serve Experiment Parity"
anchors = ["roko-serve experiment injection"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
Runner has durable prompt-experiment lifecycle, ACP gained canonical section replacement, but roko-serve still injects ephemeral context; three-phase fix designed in P1-07 but not implemented.

Imported without verification from:
- `docs/v3/39-ROADMAP.md#7.6 Serve Experiment Parity`
- `tmp/refactoring-audit/P1-07-EXPERIMENT-PARITY.md`

How to verify: grep roko-serve for experiment assignment/receipt calls.
