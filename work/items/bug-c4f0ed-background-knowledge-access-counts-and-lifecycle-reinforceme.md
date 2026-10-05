+++
id = "bug-c4f0ed"
kind = "bug"
title = "Background knowledge access counts and lifecycle reinforcement take separate per-instance write gates"
status = "open"
triage = "verified"
severity = "p2"
goal = "learning"
size = "M"
subsystem = ["roko-cli/graph-task-dispatch"]
created = 2026-10-05
updated = 2026-10-05
last_verified = 2026-10-05
source = "wave-19 follow-up reports 2026-10-05 (gap-5b8767, work/gap-addf2a)"
discovered_from = "gap-5b8767 (done on work/gap-addf2a; reported while seeding the 4131 fixture's knowledge store)"
anchors = ["crates/roko-neuro/src/knowledge_store/mod.rs::KnowledgeStore", "crates/roko-cli/src/runtime_feedback/verified_knowledge.rs::VerifiedKnowledgeSink"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn concurrent_access_count_and_reinforcement_writes_do_not_lose_an_update' crates/ && cargo test -p roko-cli concurrent_access_count_and_reinforcement_writes_do_not_lose_an_update"
+++

## Problem

Knowledge access counts (a background `KnowledgeStore`) and lifecycle reinforcement take
separate per-instance write gates on `knowledge.jsonl`, so a lost update is possible when both
write around the same time. `KnowledgeStore`'s write lock is created fresh on every
construction, not shared by path: `write_gate: Arc::new(Mutex::new(()))`
(`crates/roko-neuro/src/knowledge_store/mod.rs:68,90`) — two `KnowledgeStore` values pointed at
the exact same `knowledge.jsonl` file have two unrelated mutexes, so neither one's lock
excludes the other's writer.

Two real call sites build independent instances this way, both routinely active during a run:

- **Access counting, off the reactor.** `record_knowledge_access`
  (`crates/roko-cli/src/graph_task_dispatch/decision_log.rs:80-98`) builds its own
  `roko_neuro::KnowledgeStore::for_workdir(&self.workdir)` and spawns `store.count_access(&ids)`
  on a background task (`crate::background_writes::spawn`) after every dispatch whose prompt
  included knowledge — "off the reactor... access counts are the store's evidence that a
  prompt used an entry" (the function's own doc comment).
- **Lifecycle reinforcement.** `VerifiedKnowledgeSink::for_workdir`
  (`crates/roko-cli/src/runtime_feedback/verified_knowledge.rs:147-150`) wraps a
  `RuntimeKnowledgeLifecycle::for_workdir(workdir)` — a separate lifecycle object with its own
  path to the same file — and the sink additionally keeps its own `serial: Arc<Mutex<()>>`
  (line 143, "One ingestion at a time: each rewrites the knowledge store several times, and
  parallel tasks can finish together") — a *third*, still separate, lock layer, scoped to one
  sink instance, not the file.

So a background access-count write (from one `KnowledgeStore` instance's `write_gate`) and a
lifecycle reinforcement write (from a completely different instance's `write_gate`, itself
distinct from its sink's own `serial` lock) can genuinely interleave at the file level: each
read-modify-write cycle reads the file, modifies its own entry's fields in memory, and writes
the whole file back — if the two race, one write's changes get silently discarded by the
other's overwrite.

## Why it matters

Goal: learning, same goal as `gap-5b8767`/`gap-6c8965` (knowledge/section wiring accuracy). A
lost update here silently drops either an access-count increment (weakening the spaced-retrieval
signal `count_access`'s own doc comment describes) or a reinforcement outcome (a confirmation or
contradiction count that should have landed), with no error, no log line pointing at the race,
and no way to tell after the fact that it happened — exactly the kind of silent data-integrity
gap the loop census work this wave is trying to surface, not create a new instance of.

## Where

- `crates/roko-neuro/src/knowledge_store/mod.rs::KnowledgeStore` (`write_gate`, created fresh
  per instance rather than shared by path).
- `crates/roko-cli/src/graph_task_dispatch/decision_log.rs::record_knowledge_access` (one
  writer, off the reactor).
- `crates/roko-cli/src/runtime_feedback/verified_knowledge.rs::VerifiedKnowledgeSink` (the
  other writer, with its own, third, unrelated lock).

## Current state

Confirmed: `write_gate` is per-instance by construction, and the two cited call sites each
build their own `KnowledgeStore`/lifecycle instance independently, with no shared coordination
across them.

## Plan

1. Decide the right scope for a shared write lock: a process-wide registry keyed by the
   resolved `knowledge.jsonl` path (similar in spirit to `PROCESS_BATCHES` in the same module,
   though that serves a different purpose), or route every writer through one long-lived
   `KnowledgeStore`/sink instance per workspace instead of constructing fresh ones.
2. Thread that shared gate through both `record_knowledge_access`'s background task and
   `VerifiedKnowledgeSink`'s lifecycle writes.
3. Regression test: two concurrent writers (simulating the background access-count path and the
   lifecycle reinforcement path) against the same `knowledge.jsonl` both land, instead of one
   silently overwriting the other.

## Done when

- A background access-count write and a lifecycle reinforcement write to the same
  `knowledge.jsonl` cannot lose either update.
- The `[[verify]]` command passes.

## Notes

- 2026-10-05 (wave-19 follow-up, gap-5b8767, work/gap-addf2a not yet merged): the premise was
  reported by the worker implementing `gap-5b8767`'s own fixture fix; confirmed independently
  here by reading `write_gate`'s construction and both call sites directly.
