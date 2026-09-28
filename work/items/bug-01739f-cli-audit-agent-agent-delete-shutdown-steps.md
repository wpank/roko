+++
id = "bug-01739f"
kind = "bug"
title = "[cli-audit agent] `agent delete` shutdown steps 1/2/4/5 are log-only stubs; step 6 wrong signal path"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/agent"]
created = 2026-09-14
updated = 2026-09-28
source = "tmp/archive/cli-audit-2026-09-21/SUMMARY.md#Stubs & Unimplemented"
discovered_from = "audit:tmp/archive/cli-audit-2026-09-21/SUMMARY.md#Stubs & Unimplemented"
anchors = ["roko agent delete", "backlog #304"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
agent delete's 8-step shutdown: stop-processing, flush-pending, deregister-mesh, release-resources are log-only; step 6 references wrong signal file path; no running-process stop. Checklist marks #304 lifecycle correctness done.

Imported without verification from:
- `tmp/archive/cli-audit-2026-09-21/SUMMARY.md#Stubs & Unimplemented`
- `tmp/archive/cli-audit-2026-09-21/04-agent.md`

A source claims this was fixed; confirm against current code before closing.

How to verify: Read agent delete implementation; confirm each step performs real work and signal path is engrams.jsonl.
