+++
id = "gap-5e818f"
kind = "gap"
title = "Per-task waste (sync ExperimentStore RMW, double dream advice, unread generated-tests/)"
status = "open"
triage = "verified"
severity = "p3"
goal = "core"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-09-25
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#P2-6. Minor per-task waste"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#P2-6. Minor per-task waste"
anchors = ["graph_task_dispatch.rs:1833", "graph_task_dispatch.rs:1568 EvalGenerator", "generated-tests/", "crates/roko-cli/src/graph_task_dispatch.rs:1676", "crates/roko-cli/src/graph_task_dispatch.rs:3065"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
ExperimentStore does a full synchronous JSON read-modify-write per task on the reactor, dream routing advice loads twice per task, and EvalGenerator writes generated-tests/ files nothing reads (they litter the tree).

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#P2-6. Minor per-task waste`

How to verify: Check git status for generated-tests/ files after a run.

Partly fixed (checked 2026-09-28 against 3d0ee4d02): Fixed (committed at HEAD, commit not pinned): dream advice is loaded once per dispatch and shared with dream_routing_bias (graph_task_dispatch.rs:203-207, :2826). EvalGenerator is opt-in via gates.write_eval_artifacts and writes under .roko/generated-tests/ instead of the tree (:3065-3074), though when enabled nothing in plan run reads the files. Retrieval-arm assignment moved to spawn_blocking (:2779-2782, :2850). Remaining: ExperimentStore::settle_attempt is still called inline in the async dispatch path (:1676). ExperimentStore::load_or_new runs synchronously inside plain tokio::spawn tasks (:2523 in the spawn at :2473, :2649 in the spawn at :2630), not in spawn_blocking, so per-task synchronous JSON store I/O still runs on reactor threads.
