+++
id = "bug-f8655e"
kind = "bug"
title = "The episode distiller writes knowledge entries straight to the store, skipping the run's guarded batch: a plan write's distillation added three entries to Undertow's store at 16:44Z ('The task category in the observed episode is scaffolding.' and two like it) that no guard row, check or delivery stood behind"
status = "open"
triage = "verified"
severity = "p2"
goal = "learning"
size = "M"
subsystem = ["roko-neuro/knowledge"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
source = "round-v7 roko-learnguard (Undertow .roko/neuro/knowledge.jsonl, kn_9e5937f98f5b1e1c, kn_678683b3f7c9e46a, kn_e855df32713af5d9), the lead's ask"
discovered_from = "agent:roko-learnguard (round v7)"
anchors = ["crates/roko-neuro/src/episode_completion.rs::distill_episode", "crates/roko-cli/src/learning_helpers.rs::install_capture_distillation", "crates/roko-serve/src/dispatch.rs", "crates/roko-acp/src/bridge_events/cost.rs"]
links = { depends_on = [], blocks = [], related = ["bug-2ad4fa", "gap-f1dd0c", "gap-1220c5"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_distilled_entry_goes_through_the_guard' crates/roko-neuro/src && cargo test -p roko-neuro a_distilled_entry_goes_through_the_guard"
+++

## Problem

The episode distiller writes knowledge entries straight to the store. They skip the knowledge guard, the guarded
batch of a run, and any check on whether the episode delivered.

- **The write path:** `episode_completion.rs::distill_episode` asks a model to distil one episode, then calls
  `KnowledgeStore::for_workdir(&workdir).add(entry)` on each result.
  - The store it opens has no commit batch, so each entry lands committed at once.
  - No `.roko/learn/commits/knowledge/commits.jsonl` row records it.
  - No anchor, invariant or delivery check sees it.
- **Three paths start it**, each detached from the work that logged the episode:
  - the CLI's capture episodes, which a plan write is one of: `learning_helpers::install_capture_distillation`, and
    `agent_exec::persist_capture_episode_distilled_by` from `plan_generate/pipeline.rs::generate_plan`;
  - the serve's template dispatch (`roko-serve/src/dispatch.rs`, `spawn_episode_distillation` after the episode is
    logged);
  - ACP (`roko-acp/src/bridge_events/cost.rs`).
- **The evidence:** Undertow's `.roko/neuro/knowledge.jsonl` gained three entries at 2026-10-26T16:44:21Z, from the
  plan-writing episode `ep_9f2f838fc5c90dfe`:
  - `kn_9e5937f98f5b1e1c`: "For planning/scaffolding tasks, produce a complete implementation plan rather th…";
  - `kn_678683b3f7c9e46a`: "The observed episode is a successful plan-generation turn for task plan:generate…";
  - `kn_e855df32713af5d9`: "The task category in the observed episode is scaffolding."

  Each has source `distiller`, kind insight, confidence 0.75, tier working, the one tag `insight`, and no
  `commit_batch`. Nothing in the knowledge commit log names them.
- **bug-2ad4fa doesn't reach them.** It made a run's lessons wait for the run's delivery and gave the guard a
  `delivered` precondition. This path never had a batch, so its entries are trusted at once, whatever the episode
  was.

## Why it matters

- **Same integrity class as bug-2ad4fa.** Entries that no delivery or guard check stood behind are retrieved like
  verified lessons: tier working, confidence 0.75.
- **They are noise.** These three describe the episode, not the repository. gap-f1dd0c found round v7's 17
  plan-writing distillations cost about $0.23 and yielded one empty entry before these.
- **Nobody can see them.** `roko learn commits` never lists them. `roko learn rollback knowledge` can't remove
  them, since no batch holds them. Only `roko knowledge retire` can, one at a time.
- Goal: learning.

## Where

- `crates/roko-neuro/src/episode_completion.rs::distill_episode`: the unguarded `store.add`.
- `crates/roko-cli/src/learning_helpers.rs::install_capture_distillation`: the capture paths' hook.
- `crates/roko-serve/src/dispatch.rs`: the template dispatch's `spawn_episode_distillation`.
- `crates/roko-acp/src/bridge_events/cost.rs`: ACP's.
- `crates/roko-neuro/src/knowledge_store/commit.rs::propose_batch`: the guard a distillation should go through.

## Plan

1. **Through the guard.** A distillation writes its entries into a batch of its own, such as `distill:<episode id>`,
   and proposes it at once with `propose_batch` and the knowledge checks. Then each distillation has a commits row,
   shows in `roko learn commits`, and can be rolled back.
   - When the episode belongs to a guarded plan run, its entries join that run's batch instead. They commit only if
     the run delivers.
2. **Evidence first.** Entries distilled from an episode that verified nothing, such as a plan-writing turn or a
   capture with no gate, enter at Transient with lower confidence, or not at all. Decide this together with
   gap-f1dd0c, which skips plan-writing distillation that yields nothing.
3. **An admission floor for self-description.** Content that only describes the episode ("The observed episode is
   …", "The task category … is …") is no knowledge.
4. **Tests:**
   - `a_distilled_entry_goes_through_the_guard`: one commits.jsonl row; `roko learn commits` lists it; a rollback
     removes it;
   - `a_plan_write_distills_no_entry_without_evidence`.

## Done when

- The `[[verify]]` passes.
- A plan write on a copy of Undertow's store adds no entry outside a guarded batch.

## Notes

- Found by roko-learnguard while proving bug-2ad4fa on a copy of Undertow's store (2026-10-26). Filed at the lead's
  ask, with the lead's title.
- Related: bug-2ad4fa (the run-end guard), gap-f1dd0c (plan-writing distillation is low value), gap-1220c5
  (confirmations count what every lesson of a run shares).
