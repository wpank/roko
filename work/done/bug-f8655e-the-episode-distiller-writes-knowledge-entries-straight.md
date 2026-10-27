+++
id = "bug-f8655e"
kind = "bug"
title = "The episode distiller writes knowledge entries straight to the store, skipping the run's guarded batch: a plan write's distillation added three entries to Undertow's store at 16:44Z ('The task category in the observed episode is scaffolding.' and two like it) that no guard row, check or delivery stood behind"
status = "done"
triage = "verified"
severity = "p2"
goal = "learning"
size = "M"
subsystem = ["roko-neuro/knowledge"]
created = 2026-10-26
updated = 2026-10-27
last_verified = 2026-10-27
last_verified_rev = "074f31340"
source = "round-v7 roko-learnguard (Undertow .roko/neuro/knowledge.jsonl, kn_9e5937f98f5b1e1c, kn_678683b3f7c9e46a, kn_e855df32713af5d9), the lead's ask"
discovered_from = "agent:roko-learnguard (round v7)"
anchors = ["crates/roko-neuro/src/episode_completion.rs::distill_episode", "crates/roko-cli/src/learning_helpers.rs::install_capture_distillation", "crates/roko-serve/src/dispatch.rs", "crates/roko-acp/src/bridge_events/cost.rs"]
links = { depends_on = [], blocks = [], related = ["bug-2ad4fa", "gap-f1dd0c", "gap-1220c5"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_distilled_entry_goes_through_the_guard' crates/roko-neuro/src && cargo test -p roko-neuro a_distilled_entry_goes_through_the_guard"

[closed]
at = 2026-10-27
at_ts = "2026-10-27T03:23:32Z"
commit = "074f31340"
by = "commit trailer"
executor = "claude-agent"
via = "manual"
size = "M"
claimed_at = "2026-10-27T02:34:57Z"
forced = false
evidence = "074f31340. crates/roko-neuro: episode_completion.rs::distill_episode, where the CLI's captures, the serve's webhook dispatches and ACP all end, distils an episode only when it succeeded and passed at least one gate, failing none; [learning] distill_ungated = true distils every episode as before, still through the guard (documented in docs/v3/depth/21-config/01-schema-sections.md); a frozen workspace distils nothing. Under observe or enforce a distillation ingests into its own batch distill:<episode> and proposes it at once (the anchors, the invariants and the episode's delivery; actor distiller, reason knowledge:distill:<episode>), so roko learn commits lists it and a rollback removes it; a failed episode is not distilled under a guard; off writes straight to the store. lock_knowledge_guard (the .lock sibling of the guard's commits.jsonl) is held by propose_batch and by roko learn rollback knowledge. The distiller drops a candidate that describes its episode or corpus (9 phrases): 6 of Undertow's 12 distiller entries; the other 6, generic planning advice, are stopped by evidence first. Accepted: the orphan race (a run starting between a distillation's write and its proposal rolls that batch back). Not built: joining a plan run's batch (no plan run distils). Filed: gap-f54be7 (p3, the guard's 5-version depth, now shared with distillations). Superseded: gap-f1dd0c. Red first on db7d63f67 (logs/f8655e-chain.log): a_distilled_entry_goes_through_the_guard (no guard row), a_plan_write_distills_no_entry_without_evidence (1 model call, want 0), distill_ungated_distils_a_gate_less_episode_through_the_guard (a failed episode distilled), off_writes_straight_to_the_store_and_frozen_distils_nothing (a frozen workspace distilled), a_candidate_that_describes_its_episode_is_dropped (all 13 kept) and concurrent_proposals_take_one_version_each (7 of 8 concurrent proposals failed); green: all pass; lib suites roko-acp 237/0, roko-cli 4067/0, roko-core 2049/0, roko-neuro 261/0, roko-serve 1087/0; provider_failover, knowledge_workflows and property_tests pass; clippy -D warnings and nightly fmt clean. Done when 2 (a plan write on a copy of Undertow's store adds no entry outside a guarded batch) is shown on the capture path by a_plan_write_distills_no_entry_without_evidence, with no model call; no live plan write was run."
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

### Design (roko-learnguard, with the lead's answers, 2026-10-27)

**One choke point.** All three triggers end in `episode_completion.rs::distill_episode`: the CLI's captures, the
serve's webhook dispatches and ACP. The change is all there.

**Evidence first.** An episode is distilled only when it succeeded and passed at least one gate, failing none.
- Plan writes, research, direct agent calls and ACP turns run no gate, so by default they make no distillation
  call and add no entry.
- `[learning] distill_ungated = true` restores the old behaviour, still through the guard. It is documented in
  `docs/v3/depth/21-config/01-schema-sections.md`.
- A frozen workspace distils nothing.
- Undertow's store held 12 distiller entries, all from plan-writing turns, and none was about the repository. That
  is why the knob defaults to off.

**Through the guard.** Under `guarded_commit` = `observe` or `enforce`, a distillation ingests into its own batch,
`distill:<episode>`, and proposes it at once with `propose_batch`.
- The checks are the anchors and the invariants.
- The `delivered` precondition comes from the episode: "episode <id> passed its 2 gates".
- The row's actor is `distiller`, and its reason is `knowledge:distill:<episode>`.
- `roko learn commits` lists it, and `roko learn rollback knowledge` removes it.
- `off` writes the entries straight to the store, as before.
- Under a guard, a failed episode is not distilled at all: its batch could never commit.

**One lock for the knowledge guard.** `lock_knowledge_guard` is the `.lock` sibling of the guard's commits.jsonl.
- `propose_batch` holds it from opening the guard until the batch is settled, which covers run ends, orphan recovery
  and distillations. `roko learn rollback knowledge` holds it too.
- On the old code, 7 of 8 concurrent proposals failed, writing over each other
  (`concurrent_proposals_take_one_version_each`).

**The self-description floor.** The distiller drops a candidate that talks about its episode, or about the "Episode
corpus" its prompt shows, rather than the repository. The phrases are "observed episode", "this episode", "the
episode indicates", "episode corpus", "this corpus", "the corpus", "task category", "task_category" and
"plan:generate:".
- Of Undertow's 12, it drops the 6 that talk about their episode or corpus.
- The other 6 are generic planning advice, which evidence first keeps out.
- A lesson about the repository's own episode logger stays.

**Accepted: the orphan race.** A plan run's start rolls back every batch but its own as an orphan. A distillation's
batch exists only between its write and its proposal, a few milliseconds in one blocking task. A run that starts in
that window rolls the batch back, and the row says orphan. It loses one distillation.

**Not built: joining a plan run's batch** (plan item 1b). No plan run distils: Graph runs install no
episode-completion hook, and their own lessons already wait in the run's batch through the knowledge lifecycle.

**Filed separately: the guard's depth.** The guard keeps 5 versions, and a committed distillation is one of them.
Five gated webhook dispatches in a serve push a plan run's version out of `roko learn rollback`'s reach: gap-f54be7
(p3). It isn't built here.

**gap-f1dd0c** is closed as superseded: plan writes no longer distil without evidence.

## Done when

- The `[[verify]]` passes.
- A plan write on a copy of Undertow's store adds no entry outside a guarded batch.

## Notes

- Found by roko-learnguard while proving bug-2ad4fa on a copy of Undertow's store (2026-10-26). Filed at the lead's
  ask, with the lead's title.
- Related: bug-2ad4fa (the run-end guard), gap-f1dd0c (plan-writing distillation is low value), gap-1220c5
  (confirmations count what every lesson of a run shares).
