+++
id = "bug-521f08"
kind = "bug"
title = "DF-0925 P0-3: Episodes record the dispatch outcome, not the gate outcome"
status = "done"
triage = "verified"
severity = "p0"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-09-25
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#P0-3. Episodes record the dispatch result, not the gate result"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#P0-3. Episodes record the dispatch result, not the gate result"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs::emit_feedback", "crates/roko-cli/src/graph_task_dispatch.rs::settle_task_verification", ".roko/episodes.jsonl"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'Settled after the gate' crates/roko-cli/src/graph_task_dispatch.rs"

[closed]
at = 2026-09-28
commit = "725f21e05"
evidence = "Committed in 725f21e05 by the portal session. Re-checked 2026-09-28: the cited files are clean at HEAD and the named symbols and tests exist there (not rebuilt or retested here). crates/roko-cli/src/graph_task_dispatch.rs: emit_feedback now takes the verified outcome and runs after settle_task_verification on both the buffered and streaming dispatch paths ('Settled after the gate so episodes, routing, playbooks, affect, and experiments learn from the verified outcome'); failed provider calls settle with success=false. File was visibly mid-edit by a concurrent session."
+++
emit_feedback runs ~100 lines before the verify block, so episodes say success=true even when verify failed; routing, playbooks, reward and cascade learning train on false positives.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#P0-3. Episodes record the dispatch result, not the gate result`

How to verify: Fail a verify step and inspect the episode success flag.

Verified 2026-09-28: closed as done; see [closed].evidence.
