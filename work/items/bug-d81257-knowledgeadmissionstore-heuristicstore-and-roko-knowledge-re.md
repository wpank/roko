+++
id = "bug-d81257"
kind = "bug"
title = "KnowledgeAdmissionStore, HeuristicStore and roko knowledge restore bypass the new shared knowledge write gate"
status = "open"
triage = "verified"
severity = "p2"
goal = "learning"
size = "M"
subsystem = ["roko-cli/commands"]
created = 2026-10-05
updated = 2026-10-05
last_verified = 2026-10-05
source = "wave-20 follow-up reports 2026-10-05 (bug-c4f0ed, work/bug-c4f0ed)"
discovered_from = "bug-c4f0ed (open; its fix covers KnowledgeStore only, not these three)"
anchors = ["crates/roko-neuro/src/admission.rs::KnowledgeAdmissionStore", "crates/roko-cli/src/commands/knowledge.rs::publish_staged_neuro_files"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn restore_waits_for_a_concurrent_knowledge_store_writer' crates/roko-cli/ && cargo test -p roko-cli restore_waits_for_a_concurrent_knowledge_store_writer"
+++

## Problem

`bug-c4f0ed` (on `work/bug-c4f0ed`, not yet merged) gave `KnowledgeStore` one shared, per-path
write gate (a process-wide map keyed by the canonical `knowledge.jsonl` path) plus a
cross-process `.lock`-file lock (`lock_writes`, `roko_fs::log_rotation::lock_jsonl`) on every
write path. Two siblings that write their own JSONL files next to it were not touched, and a
third command bypasses locking entirely:

- **`KnowledgeAdmissionStore`** (`crates/roko-neuro/src/admission.rs:605-627`) and
  **`HeuristicStore`** (`crates/roko-neuro/src/tier_progression.rs:362-382`) each still declare
  their own `write_gate: Arc<Mutex<()>>`, constructed fresh (`Arc::new(Mutex::new(()))`) on
  every instance — the exact per-instance pattern `bug-c4f0ed` just fixed for `KnowledgeStore`
  itself, confirmed unchanged for these two. Two instances of either store pointed at the same
  `candidates.jsonl`/`decisions.jsonl` (admission) or
  `heuristics.jsonl`/`observations.jsonl`/`demotions.jsonl` (heuristic) files can still lose an
  update to each other, and neither goes through the new cross-process `.lock` file either.
- **`roko knowledge restore`** (`crates/roko-cli/src/commands/knowledge.rs::publish_staged_neuro_files`,
  lines 1805-1853) renames the live `knowledge.jsonl` out of the way and renames the staged,
  restored one into its place with four bare `std::fs::rename` calls — no reference anywhere in
  the function to `write_gate`, `lock_jsonl`, or any `KnowledgeStore` at all. A concurrent
  writer (in-process via the new shared gate, or another process via the new `.lock` file) has
  no way to know a restore is swapping the file out from under it.

## Why it matters

Goal: learning, same goal as `bug-c4f0ed`. The fix just landed specifically to close a
lost-update race for `knowledge.jsonl`; `KnowledgeAdmissionStore` and `HeuristicStore` are the
same architectural pattern on sibling files, with the identical race still open, and
`roko knowledge restore` can step on any of these (including the now-fixed `knowledge.jsonl`
path) by swapping the file wholesale with no coordination at all — a conceptually worse
failure mode than a lost field update, since it can discard an entire in-flight write or hand a
concurrent reader a half-renamed file.

## Where

- `crates/roko-neuro/src/admission.rs::KnowledgeAdmissionStore` (its own `write_gate`).
- `crates/roko-neuro/src/tier_progression.rs::HeuristicStore` (its own `write_gate`).
- `crates/roko-cli/src/commands/knowledge.rs::publish_staged_neuro_files`,
  `restore_neuro_store` (the unguarded rename sequence).
- `crates/roko-neuro/src/knowledge_store/mod.rs` (the shared-gate-map pattern to extend to the
  other two stores, and the lock `restore` should take).

## Current state

Confirmed on `work/bug-c4f0ed` (not yet merged): both sibling stores still build a fresh,
unshared `Arc::new(Mutex::new(()))`; `publish_staged_neuro_files` has zero lock references.

## Plan

1. Extend `bug-c4f0ed`'s shared-gate-map mechanism to cover `KnowledgeAdmissionStore`'s and
   `HeuristicStore`'s own files, the same way it now covers `knowledge.jsonl`.
2. Have `roko knowledge restore` take the knowledge store's write gate and cross-process lock
   (and the admission/heuristic stores' once they're covered) before renaming any live file out
   from under a running process.
3. Regression tests: concurrent access-count/reinforcement writers against the admission and
   heuristic stores don't lose an update, the same way `bug-c4f0ed`'s test proves for
   `knowledge.jsonl`; a restore running concurrently with a live writer doesn't corrupt either
   side.

## Done when

- `KnowledgeAdmissionStore` and `HeuristicStore` share one write gate per file, cross-process
  included.
- `roko knowledge restore` takes the store's lock before swapping the live file.
- The `[[verify]]` command passes.

## Notes

- 2026-10-05 (wave-20 follow-up, bug-c4f0ed, work/bug-c4f0ed not yet merged): confirmed
  directly by reading both sibling stores' constructors and `publish_staged_neuro_files` in
  full.

## Progress

- bug-d81257: implemented at b3b328004. The per-path gate map from bug-c4f0ed now serves KnowledgeAdmissionStore (keyed by its candidates log) and HeuristicStore (keyed by heuristics.jsonl). Each takes its file's shared gate and then the `.lock` sibling's lock among processes through `WriteGuard::hold`, and guards its sibling logs under that one gate. This also serializes HeuristicStore's fixed `heuristics.jsonl.tmp` rewrite. The lock order stays admission or heuristic first, then knowledge. `KnowledgeStore::lock_writes` is now public, and `restore_neuro_store` holds it from its check of the live files until the restored ones are published. Restore touches only knowledge.jsonl and its confirmations, so it takes only that store's lock. The verify's grep passes; cargo verification is deferred to the batch gate (`restore_waits_for_a_concurrent_knowledge_store_writer`, `admission_stores_of_one_workspace_keep_each_decision_with_its_candidate`, `heuristic_stores_of_one_file_do_not_lose_an_upsert`).
