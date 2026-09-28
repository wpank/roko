+++
id = "bug-dacaeb"
kind = "bug"
title = "DA-bak-17/18: Corrupt JSONL segments silently degrade startup (no integrity check/quarantine)"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-fs/jsonl"]
created = 2026-09-02
updated = 2026-09-28
source = "tmp/archive/dev-audit-2026-09-21/dev-audit-backup/17-state-corruption.md#2. Corrupt Events JSONL"
discovered_from = "audit:tmp/archive/dev-audit-2026-09-21/dev-audit-backup/17-state-corruption.md#2. Corrupt Events JSONL"
anchors = [".roko/events.*.jsonl", ".roko/learn/efficiency.jsonl", "prompt-experiment reconciliation"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
An unparseable events.20260821T*.jsonl line permanently degraded prompt-experiment reconciliation; efficiency.jsonl held model_call records that broke the TUI tailer each startup. Proposed: validate JSONL at startup and quarantine corrupt segments.

Imported without verification from:
- `tmp/archive/dev-audit-2026-09-21/dev-audit-backup/17-state-corruption.md#2. Corrupt Events JSONL`
- `tmp/archive/dev-audit-2026-09-21/dev-audit-backup/18-learning-health.md#Data Contamination`
- `tmp/dev-audit/09-additional-live-run-findings.md#Global event storage is already degrading`

Some cited files are gone: `.roko/events.*.jsonl`.

How to verify: Feed a malformed-line fixture to event archive/reconciliation and the efficiency tailer; check quarantine.
