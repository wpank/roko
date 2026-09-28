+++
id = "find-6a5b62"
kind = "finding"
title = "DA-bak-17/18: Run-ledger and learning-artifact hygiene (run_id-less rows, zero plan-op cost, stale summaries)"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-learn"]
created = 2026-09-02
updated = 2026-09-28
source = "tmp/archive/dev-audit-2026-09-21/dev-audit-backup/17-state-corruption.md#6. Run Ledger Noise"
discovered_from = "audit:tmp/archive/dev-audit-2026-09-21/dev-audit-backup/17-state-corruption.md#6. Run Ledger Noise"
anchors = [".roko/state/run-ledger.jsonl", ".roko/learn/costs.jsonl", ".roko/learn/efficiency-summaries.jsonl"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
run-ledger.jsonl had 41,672 task_started rows without run_id; costs.jsonl recorded $0.00 for plan operations (plan:regenerate spend not captured); efficiency-summaries.jsonl stale since Aug 22; zero-byte .tmp/.lock remnants in .roko/learn.

Imported without verification from:
- `tmp/archive/dev-audit-2026-09-21/dev-audit-backup/17-state-corruption.md#6. Run Ledger Noise`
- `tmp/archive/dev-audit-2026-09-21/dev-audit-backup/18-learning-health.md#Costs — All Zero`
- `tmp/archive/dev-audit-2026-09-21/dev-audit-backup/18-learning-health.md#File Inventory`

How to verify: Check plan generation records real cost and ledger writers always set run_id.
