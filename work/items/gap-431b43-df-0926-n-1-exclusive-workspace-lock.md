+++
id = "gap-431b43"
kind = "gap"
title = "DF-0926 N-1: Exclusive workspace lock blocks live-server verification inside plan runs"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/locking"]
created = 2026-09-26
updated = 2026-09-28
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#N-1. The workspace lock makes in-plan live-server verification impossible"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#N-1. The workspace lock makes in-plan live-server verification impossible"
anchors = ["commands/plan.rs:302", "commands/server.rs:10", "acquire_workspace_lock_shared"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
plan run and roko serve both take the exclusive workspace lock, so a task can never start serve against its workspace (one task burned 600s x retries); proposal: shared read lock plus narrow write lock, or a read-only serve mode.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#N-1. The workspace lock makes in-plan live-server verification impossible`

How to verify: Start roko serve while a plan runs in the same workdir.
