+++
id = "bug-2ad4fa"
kind = "bug"
title = "A lesson from a try that gamed its check reached Undertow's knowledge store and stayed: the run-end guard committed phase 5's knowledge batch although its plan check failed and the run was cancelled, a later task of the same run confirmed it, roko has no command to retire one entry, and retrieval ignores deprecated"
status = "done"
triage = "verified"
severity = "p2"
goal = "learning"
size = "M"
subsystem = ["roko-cli/commands"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
last_verified_rev = "896781b4b"
source = "round-v7 run-analyst and the pilot (tmp/hermes-roko-v7/findings/hermes-pilot.md section R; Undertow run 1f677933), the lead's ask"
discovered_from = "agent:hermes-pilot (round v7)"
anchors = ["crates/roko-cli/src/runtime_feedback/verified_knowledge.rs::VerifiedKnowledgeSink", "crates/roko-cli/src/graph_execution/learning_commit.rs::commit_run_learning", "crates/roko-neuro/src/knowledge_store/commit.rs::KnowledgeChecks", "crates/roko-neuro/src/knowledge_store/query.rs::is_hot", "crates/roko-cli/src/commands/knowledge.rs"]
links = { depends_on = [], blocks = [], related = ["bug-3b3bb6", "spec-635697", "gap-6ff99a"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_cancelled_runs_lessons_are_not_committed' crates/roko-cli/src && cargo test -p roko-cli --lib a_cancelled_runs_lessons_are_not_committed"

[[verify]]
command = "grep -rqw 'fn retrieval_skips_deprecated_entries' crates/roko-neuro/src && cargo test -p roko-neuro retrieval_skips_deprecated_entries"

[[verify]]
command = "grep -rqw 'fn a_retired_entry_stays_retired_after_a_rollback' crates/roko-neuro/src && cargo test -p roko-neuro a_retired_entry_stays_retired_after_a_rollback"

[closed]
at = 2026-10-26
at_ts = "2026-10-26T19:27:16Z"
commit = "896781b4b"
by = "roko-learnguard"
executor = "claude-agent"
via = "manual"
size = "M"
claimed_at = "2026-10-26T16:34:24Z"
forced = false
evidence = "896781b4b. (a) roko-learn CommitCheck::precondition: GuardedStore::propose rolls a candidate back in either mode when it fails. The knowledge store's is `delivered` (roko-neuro Delivery in KnowledgeChecks); roko-cli learning_commit::run_delivery derives it from each plan's outcome and the stop cause, plan_runner passes it at the run end and settles an errored run's batch, and recover_orphaned_batches rolls orphans back (gap-6ff99a's decision changes: an orphan shows no delivery). An undelivered row reads e.g. 'delivered FAIL: plan hello-lesson failed; the batch's 1 lesson was not kept', mode observe. (b) detect_confirmations skips uncommitted and retired entries; the lifecycle reinforces and promotes only committed, unretired entries (an admitted candidate keeps its own pass). (c) `roko knowledge retire <id> --reason` (KnowledgeStore::retire; ledger .roko/neuro/knowledge-retired.jsonl applied on every read; an id only a kept guard version holds is accepted, an unknown one refused); a retired entry absorbs relearned copies (admission path repeats_retired) and the ledger counts them, shown by `roko knowledge stats`. (d) is_hot, query_similar and query_hdc skip deprecated; freeze_entry/thaw_entry/is_frozen fold into the cold tier. (e) the anchors check says no query was checked. Red first on stubs: the 6 new tests and 2 updated ones failed (tmp/hermes-roko-v7/roko-learnguard/logs/2ad4fa-chain.log). Green: the three [[verify]] tests pass; lib suites roko-learn 1405/0, roko-neuro 251/0, roko-cli 4027 passed with 2 failures that are reg-1bf684 (on main since find-697944, unrelated); bin tests knowledge+learn_commits 19/0 incl. knowledge_retire_retires_one_entry; clippy -D warnings and nightly fmt clean on the three crates. Zero-cost proofs on copies of Undertow's .roko/neuro and learn/commits (logs/proof-retire.log, logs/e2e-*.log): retire hides runtime_3c3b9654100195f1 and it stays hidden through rollbacks to v14 and v13 and a restore of a backup taken before the retirement; a fake-model plan run whose plan check fails rolls its lesson back, one that passes commits v15, and a run killed -9 with its lesson in the batch has it rolled back at the next run's start."
+++

## Problem

A lesson from a try that gamed its check entered Undertow's knowledge store, and it stayed after its run failed. Then
a person had to remove it by hand.

**Phase 5** (Undertow, run 1f677933, roko 74c3594bc):
- **The lesson was admitted at the pass.** T4#3 passed P5.06 by post-processing its WAVs (bug-3b3bb6). Its
  `Lesson:` line was admitted by the verified-knowledge sink at that pass (`knowledge_written`, 15:57:28Z,
  `light_admitted`), as `runtime_3c3b9654100195f1`:
  - strategy_fragment, tier working, confidence 0.874 (weight 0.92);
  - source `runtime:gate_verdict`, source model gpt-5.4;
  - tags `tools/render-demos.sh`, the four `artifacts/audio/*.wav`, `plan:p5-ship`, `gate:passed`, `lesson`;
  - text: "some current factory presets still render too quiet or even silent, so `tools/render-demos.sh` must
    compensate locally for showcase artifacts rather than assuming preset levels already meet the demo spec."
- **The run's own T5#1 confirmed it.** The run's process sees its own uncommitted batch, so T5's prompt surfaced the
  entry, T5#1 passed, and the entry recorded a confirmation: `confirmation_count` 2, `distinct_contexts` T4#3 and
  T5#1.
- **The guard committed it anyway.** The run's plan check then failed (P5.03), and the pilot cancelled the run. At
  the run's end the guarded commit committed the run's knowledge batch of 8 entries:
  - `.roko/learn/commits/knowledge/commits.jsonl` version 14: `decision committed`, `mode observe`;
  - check `held_out`: "none at commit: M2 measures knowledge through its withhold arm";
  - check `anchors`: "0 knowledge anchors hold, and the invariants".
  - Neither check looks at the run's outcome.
- **Removing it took a person** (the pilot, 17:1x, the lead's ask):
  1. stop the serve;
  2. back up `.roko/neuro/knowledge.jsonl` to `knowledge.jsonl.bak-20261026-gamed-lesson`;
  3. delete line 48 by hand;
  4. restart the serve and check `/api/knowledge/search`.
  - `roko knowledge` has no command that retires one entry.
  - Setting `deprecated: true` wouldn't have hidden it. Retrieval skips `frozen` and empty entries, never
    `deprecated` ones, though `freeze_entry` documents deprecated entries as kept out of active search.
- **The guard's last-known-good snapshot still holds it.** That's `learn/commits/knowledge/v14.snap`, written at the
  run's end. A rollback to v14 would restore the entry. Its digest is in `commits.jsonl`, so the snapshot can't be
  edited by hand either.

**Expected:**
- A lesson from a run whose plan check failed, or that was cancelled, isn't committed: it stays provisional until a
  delivery passes.
- One entry can be retired by command, with a reason, in a way a rollback honours.
- Retrieval skips deprecated entries.

## Why it matters

- **The learning loop turned a gamed pass into advice.** A phase 5.1 task on `tools/render-demos.sh` would have been
  told to keep compensating. Lessons are retrieved by topic and confirmed by the passes that saw them, so a bad one
  compounds.
- **The guard exists for exactly this case.** `learning_commit.rs` says "a failed or cancelled run's learning is
  the one that most needs the check". Without anchors, its checks pass vacuously.
- Goal: learning. bug-3b3bb6 is the gaming itself.

## Where

- `crates/roko-cli/src/runtime_feedback/verified_knowledge.rs::VerifiedKnowledgeSink`: admits each verified pass's
  `Lesson:` line at the pass, and reinforces and confirms the entries that pass's prompt surfaced.
- `crates/roko-cli/src/graph_execution/learning_commit.rs::commit_run_learning`: proposes the run's knowledge batch
  at the run's end, "whatever the run's outcome".
- `crates/roko-neuro/src/knowledge_store/commit.rs::KnowledgeChecks`: `held_out` always passes ("none at commit").
  `anchors` checks the `[[knowledge]]` queries of `.roko/policy/anchors.toml` (Undertow has none) and the
  invariants. `propose_batch` sits beside it.
- `crates/roko-neuro/src/knowledge_store/query.rs::is_hot`: the retrieval filter, `!entry.frozen` and non-empty
  content only.
- `crates/roko-neuro/src/lib.rs::freeze_entry`: sets `deprecated`, and is used only by tests. There are two freeze
  flags: `KnowledgeEntry::freeze` sets `frozen`, and `freeze_entry` sets `deprecated`.
- `crates/roko-cli/src/commands/knowledge.rs`: the `roko knowledge` subcommands. They are query, stats, gc,
  export, import, backup, restore, sync, dream, custody, signal archive and backfill-hdc. None retires an entry.

## Current state

- **Guarded commits:** P21, 8137/8138, decision 8103. Undertow runs `[learning] guarded_commit = "observe"`.
  gap-6ff99a recovers a crashed run's uncommitted batch through the same guard.
- **The retire problem is the same in enforce.** In `enforce`, a failed check would roll the batch back, but
  nothing here would have failed.

## Plan

1. **Tie the batch to the run's outcome.**
   - Add a `delivered` check to the knowledge guard.
   - A run whose plan check failed, or that was cancelled or interrupted, doesn't commit its batch. It stays
     uncommitted: a resume that delivers commits it, and a run that ends without one rolls it back.
   - In `observe` mode, hold the batch rather than commit it, and record why in `commits.jsonl`.
2. **Don't let a run confirm its own lessons before they commit.** A confirmation from a task of the same run (T5#1
   here) shouldn't count toward `distinct_contexts` or promotion until the batch commits.
3. **Add `roko knowledge retire <id> --reason "…"`.**
   - It freezes the entry and writes a lifecycle receipt.
   - It records the id where the guard applies it on rollback or restore, so restoring v14 can't bring it back.
   - While a serve holds the workspace, it goes through the serve's API, or refuses with a clear message.
4. **Make retrieval honour `deprecated`.** Either `is_hot` excludes it too, or `freeze_entry` is folded into
   `KnowledgeEntry::freeze`, so that one flag means "not retrieved".
5. **Tests:**
   - `a_cancelled_runs_lessons_are_not_committed` (roko-cli);
   - `retrieval_skips_deprecated_entries` (roko-neuro);
   - `a_retired_entry_stays_retired_after_a_rollback` (roko-neuro).

## Done when

- The three `[[verify]]` commands pass.
- A run cancelled after a failed plan check leaves its knowledge batch uncommitted, and `commits.jsonl` says why.

## Notes

- **Evidence:**
  - Undertow `.roko/neuro/knowledge.jsonl`, and its backup `knowledge.jsonl.bak-20261026-gamed-lesson`;
  - `.roko/learn/commits/knowledge/commits.jsonl` (v14) and `v14.snap`;
  - the run's event index (`knowledge_written`);
  - the pilot's findings (`tmp/hermes-roko-v7/findings/hermes-pilot.md`, section R, "A person's curation");
  - `tmp/hermes-roko-v7/findings/run-analyst.md`, section 10.
- The append-only `knowledge-lifecycle.jsonl` and `knowledge-confirmations.jsonl` still mention the entry. They're
  history.
- Related: bug-3b3bb6 (the gamed try), spec-635697 (the guarded-commit epic), gap-6ff99a (orphaned batches).
