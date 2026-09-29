+++
id = "gap-2e69b2"
kind = "gap"
title = "Efficiency attempt_id repeats across runs, and its field doc still describes the old shared id"
status = "open"
triage = "verified"
severity = "p3"
goal = "learning"
subsystem = ["roko-cli/graph-dispatch", "roko-learn/efficiency"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:t2-attempt-id"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs::next_attempt_id", "crates/roko-learn/src/efficiency.rs::AgentEfficiencyEvent"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'Shared between the dispatch cost event' crates/roko-learn/src/efficiency.rs && cargo test -p roko-cli --lib attempt_id_is_unique_across_runs"
+++

`next_attempt_id` builds `<task key>/a<n>` from a per-process counter, with no run id. Another run of the same plan, or a resumed run, starts again at `a0`, so efficiency records from different runs share attempt ids and joins over `.roko/learn/efficiency.jsonl` can mix attempts. The `attempt_id` field doc in roko-learn (efficiency.rs:106-108) still says the dispatch cost event and gate-failure events share one id; since t2 they share a prefix.

Fix: include the run id (or a random suffix) in the attempt id and update the doc.
