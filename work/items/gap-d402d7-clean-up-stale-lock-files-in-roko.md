+++
id = "gap-d402d7"
kind = "gap"
title = "Clean Up Stale Lock Files in .roko/learn/"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli"]
created = 2026-09-21
updated = 2026-09-28
source = "tmp/backlog/archive/394-stale-lock-file-cleanup.md#394 — Clean Up Stale Lock Files in .roko/learn/"
discovered_from = "audit:tmp/backlog/archive/394-stale-lock-file-cleanup.md#394 — Clean Up Stale Lock Files in .roko/learn/"
anchors = ["crates/roko-cli/src/", "crates/roko-learn/src/", "roko doctor"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
5 orphaned lock files. Dev-audit found 5 stale .lock files in `.roko/learn/` that were left by crashed processes. No automatic cleanup exists.

Imported without verification from:
- `tmp/backlog/archive/394-stale-lock-file-cleanup.md#394 — Clean Up Stale Lock Files in .roko/learn/`

How to verify: Check whether the gap described in tmp/backlog/archive/394-stale-lock-file-cleanup.md still exists at the anchored paths. [evidence: 00-INDEX (2026-09-21) listed active: 2026-09-21 Audit Sweep Items (#376-#395)]
