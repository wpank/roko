+++
id = "gap-431b43"
kind = "gap"
title = "Exclusive workspace lock blocks live-server verification inside plan runs"
status = "open"
triage = "unverified"
severity = "p2"
goal = "core"
subsystem = ["roko-cli/locking"]
created = 2026-09-26
updated = 2026-09-28
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#N-1. The workspace lock makes in-plan live-server verification impossible"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#N-1. The workspace lock makes in-plan live-server verification impossible"
anchors = ["commands/plan.rs:302", "commands/server.rs:10", "acquire_workspace_lock_shared", "crates/roko-cli/src/commands/plan.rs:569", "crates/roko-cli/src/commands/server.rs:167", "crates/roko-cli/src/workspace_lock.rs::acquire_runner_lock"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
plan run and roko serve both take the exclusive workspace lock, so a task can never start serve against its workspace (one task burned 600s x retries); proposal: shared read lock plus narrow write lock, or a read-only serve mode.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#N-1. The workspace lock makes in-plan live-server verification impossible`

How to verify: Start roko serve while a plan runs in the same workdir.

Check on 2026-09-28 was inconclusive: The stated mechanism does not match current code. The anchor commands/plan.rs:301-302 (exclusive acquire_workspace_lock) is the `plan create` arm. `plan run` (PlanCmd::Run, plan.rs:462) takes only acquire_runner_lock, a separate .roko/runtime/roko.runner.lock (plan.rs:566-569, present since 244f564e1). `roko serve`/`roko up` take the exclusive roko.lock (commands/server.rs:10, :167) but never the runner lock. So a CLI plan run should not block serve through the workspace lock. The conflict would still occur for plans executed by a running `roko serve`, which holds the exclusive roko.lock, or through port 6677 contention, and no read-only serve mode exists. To decide, run the item's own check: start `roko serve` in the workdir while `roko plan run` is active and see which lock or port error it hits.
