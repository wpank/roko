+++
id = "gap-6ff99a"
kind = "gap"
title = "A crashed run's uncommitted knowledge batch is never recovered at the next run's start"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "M"
subsystem = ["roko-cli/graph_execution"]
created = 2026-10-04
updated = 2026-10-04
last_verified = 2026-10-04
source = "wave-13 follow-up reports 2026-10-04 (PK71 gap-099513, tasks 8137/8138)"
discovered_from = "gap-099513 (closed; 8137/8138 implemented, recovery path never built)"
anchors = ["crates/roko-neuro/src/knowledge_store/commit.rs::propose_batch", "crates/roko-cli/src/graph_execution/learning_commit.rs::propose_knowledge"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn orphaned_knowledge_batch_is_recovered_at_next_run_start' crates/ && cargo test -p roko-neuro orphaned_knowledge_batch_is_recovered_at_next_run_start"
+++

## Problem

A crashed run's uncommitted knowledge batch is never recovered. `KnowledgeEntry::commit_batch`
(`crates/roko-neuro/src/lib.rs:513`) tags every entry a run proposes with its batch id; other
runs' retrieval skips any entry whose `commit_batch` is set (`KnowledgeBatch::holds`,
`crates/roko-neuro/src/knowledge_store/commit.rs:147-149`). The only two things that ever clear
or remove that tag are `KnowledgeBatch::commit` (clears it on every held entry) and
`KnowledgeBatch::discard` (deletes every held entry) — both called exclusively from
`propose_batch` (`commit.rs:312-338`), which itself has exactly one production call site:
`propose_knowledge` in `crates/roko-cli/src/graph_execution/learning_commit.rs:150`, the
run-end guarded-commit hook (8138). If the process dies before that run-end hook fires —
killed, panicked, OOM, power loss — the batch's entries keep their `commit_batch` tag forever.
Nothing anywhere in the codebase scans for or recovers a stale/orphaned `commit_batch` at the
next run's start, or anywhere else: a grep for `recover|crash|stale|orphan|startup|on_start`
across `crates/roko-neuro/src/knowledge_store/commit.rs` returns nothing, and `propose_batch`
and `KnowledgeBatch::new` have no other call sites outside this one file and its own unit tests.

## Why it matters

Goal: cybernetic, roko-neuro knowledge store / guarded commit (backlog 8137/8138). A crashed
run's learning doesn't just fail to land — its entries sit permanently invisible to every other
process's retrieval (`KnowledgeBatch::holds` filters them out of `entries()`, and the store's
general read path excludes anything still batch-tagged), silently shrinking the effective
knowledge store after every crash with no symptom pointing at the cause, and no command to list
or clear them.

## Where

- `crates/roko-neuro/src/knowledge_store/commit.rs::propose_batch` (the only committer) and
  `::KnowledgeBatch` (`commit`, `discard`, `holds`).
- `crates/roko-neuro/src/lib.rs:513` (`KnowledgeEntry::commit_batch` field and its doc comment).
- `crates/roko-cli/src/graph_execution/learning_commit.rs::propose_knowledge` (the one caller,
  at run end only).

## Current state

A batch is proposed (committed or rolled back) exactly once, at a successful run-end hook
invocation. There is no startup sweep, no listing of in-store `commit_batch` values, and no CLI
command that enumerates or clears orphaned batches. A run that never reaches
`propose_knowledge` (crash, kill, panic before run-end) leaves its batch's entries permanently
tagged and permanently hidden.

## Plan

1. At the start of a run (or in `roko doctor`/`roko knowledge gc`), list the distinct
   `commit_batch` values present in the store and compare against currently-active run ids
   (e.g. `.roko/state/graph/<plan>/` or the run registry) to identify orphans: batches with no
   live owner.
2. Recover each orphan: either commit it (optimistic — the work probably succeeded) or discard
   it (conservative — treat an unproposed batch as unverified), gated by the same `GuardMode`
   used for the live guarded-commit path. Record the decision on the StateHub/`commits.jsonl`
   the same way a normal proposal does.
3. Add a regression test that creates a batch, tags entries with a run id that is not the
   current run, and checks that a recovery pass (whichever entry point #1/#2 lands on) commits
   or discards it rather than leaving it stuck.

## Done when

- A batch left tagged by a run that never reached `propose_knowledge` is committed or discarded
  by the next run's start (or an explicit recovery command), not left permanently hidden.
- The `[[verify]]` command passes.

## Notes

- 2026-10-04 (wave-13 follow-up, PK71 8137/8138): confirmed at main HEAD `b7ad508ce`. Verified by
  reading `commit.rs` in full (`commit`, `discard`, `propose_batch`, and `holds`) and grepping
  `propose_batch`/`KnowledgeBatch::new`/`release_batch` across `crates/` for other call sites —
  there are none outside `learning_commit.rs:150` and `commit.rs`'s own tests.
