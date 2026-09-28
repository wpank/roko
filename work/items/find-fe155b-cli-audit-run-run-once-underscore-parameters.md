+++
id = "find-fe155b"
kind = "finding"
title = "[cli-audit run] `run_once()` underscore parameters never receive non-None values"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/run"]
created = 2026-08-31
updated = 2026-09-28
source = "tmp/archive/cli-audit-2026-09-21/01-run.md"
discovered_from = "audit:tmp/archive/cli-audit-2026-09-21/01-run.md"
anchors = ["crates/roko-cli/src/run.rs run_once", "backlog #302"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
run_once() is called by run_inline.rs, worker/handler.rs, commands/job.rs, demo_cmd.rs but two underscore params are never provided; dead parameters. #302 residual run.rs pruning claimed done.

Imported without verification from:
- `tmp/archive/cli-audit-2026-09-21/01-run.md`

A source claims this was fixed; confirm against current code before closing.

How to verify: Inspect run_once signature for unused underscore params.
