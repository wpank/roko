+++
id = "gap-1220c5"
kind = "gap"
title = "Knowledge confirmations count what every lesson of a run shares: entries_are_similar needs 1 shared tag and 2 shared words, and plan:, lesson and gate:passed make that near-universal, so phase 5's gamed lesson 'confirmed' 31 committed entries; a run's passes also confirm committed entries whether or not the run delivers"
status = "done"
triage = "verified"
severity = "p2"
goal = "learning"
size = "M"
subsystem = ["roko-neuro/knowledge", "roko-neuro/lifecycle"]
created = 2026-10-26
updated = 2026-10-27
last_verified = 2026-10-27
last_verified_rev = "ea7661e29"
source = "round-v7 roko-learnguard while fixing bug-2ad4fa (Undertow run 1f677933's knowledge-confirmations.jsonl), the lead's ask"
discovered_from = "agent:roko-learnguard (round v7)"
anchors = ["crates/roko-neuro/src/knowledge_store/scoring.rs::entries_are_similar", "crates/roko-neuro/src/knowledge_store/scoring.rs::detect_confirmations", "crates/roko-neuro/src/knowledge_store/crud.rs::ingest", "crates/roko-neuro/src/lifecycle.rs::promote_runtime_entries"]
links = { depends_on = [], blocks = [], related = ["bug-2ad4fa", "bug-3b3bb6", "gap-6755f7"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn run_generic_tags_confirm_nothing' crates/roko-neuro/src && cargo test -p roko-neuro run_generic_tags_confirm_nothing"

[[verify]]
command = "grep -rqw 'fn an_undelivered_run_confirms_nothing' crates/roko-neuro/src && cargo test -p roko-neuro an_undelivered_run_confirms_nothing"

[closed]
at = 2026-10-27
at_ts = "2026-10-27T00:27:52Z"
commit = "ea7661e29"
executor = "claude-agent"
via = "manual"
size = "M"
claimed_at = "2026-10-26T21:18:07Z"
forced = false
evidence = "ea7661e29. crates/roko-neuro: entries_are_similar (scoring.rs) counts only topical tags (a file or a package; lesson, runtime, kind names, plan:/gate:/task:/model: left out) and needs a content-word Jaccard of 0.17 with stopwords out (MIN_CONTENT_JACCARD, in place of MIN_KEYWORD_OVERLAP's count of 2); promotion thresholds unchanged, and no recorded decision names these constants. Held confirmations (knowledge_store/pending.rs): a guarded run's confirmations of committed entries (surfaced passes from the lifecycle, repeats from ingest) wait in .roko/neuro/knowledge-pending-confirmations.jsonl beside its batch; KnowledgeBatch::commit applies them once each (deduplicated, never to a retired entry) with the repeats' records, discard drops them, propose_batch settles a lesson-less batch by delivery, orphaned_batches counts confirmation-only batches; an unguarded run confirms at once; reinforcement stays immediate. Replay on a read-only copy of Undertow's store (tmp/hermes-roko-v7/roko-learnguard/bin/confirm-report.py, logs/confirm-report-5-2.log): the old rule's mirror reproduces 854/854 records; 3 survive, all real repeats (phase 1's envelope double precision, phase 4.1/4.2's budget PNGs, phase 5/5.1's master_gain trim); the gamed lesson's 31, run 1f677933's 230 and phase 5.2's 38 go; closest dropped 0.152 (two different tools/spec.sh facts). Red first: run_generic_tags_confirm_nothing, an_undelivered_run_confirms_nothing, a_resumed_run_applies_its_pending_confirmations_once and the updated a_run_does_not_confirm_its_own_provisional_lessons failed on the old code; green all pass; lib suites roko-learn 1408/0, roko-neuro 254/0, roko-cli 4049/0; clippy -D warnings and nightly fmt clean (logs/1220c5-chain.log, logs/1220c5-clippy.log)."
+++

## Problem

Knowledge confirmations count what every lesson of a run shares, so the counts that drive tier promotion carry
little evidence.

- **The rule is near-universal.** An ingested entry confirms an existing one when they share 1 tag and 2 content
  words (`entries_are_similar`: `MIN_TAG_OVERLAP = 1`, `MIN_KEYWORD_OVERLAP = 2`). Every lesson a run stores carries
  the same run-generic tags: `plan:<id>`, `lesson`, `runtime`, `gate:passed`, `gate:<label>`, its kind and its
  packages. Two words such as "the" are filtered out by the tokenizer, but "showcase", "render" or "preset" are
  enough.
- **One gamed lesson "confirmed" 31 entries.** In Undertow run 1f677933, T4#3's lesson (bug-2ad4fa, bug-3b3bb6) was
  admitted at 15:57:28Z. At the same instant `knowledge-confirmations.jsonl` gained 31 records naming it as the
  confirming entry. The entries it "confirmed" were committed lessons of phases 0 to 4. Each confirmation bumped a
  `confirmation_count` and added a `distinct_contexts` entry.
- **A run's passes confirm committed entries whether or not the run delivers.** bug-2ad4fa stopped a run from
  confirming its own uncommitted entries and rolled back an undelivered run's batch. Its confirmations of committed
  entries are still applied at once, at each pass, and are kept when the run then fails its plan check or is
  cancelled:
  - `crud.rs::ingest` through `detect_confirmations`;
  - `lifecycle.rs::promote_runtime_entries`, for the entries the pass's prompt surfaced.

## Why it matters

- Confirmation counts and distinct contexts drive tier promotion: Transient to Working at 2, Working to Consolidated
  at 3 contexts. Promotion raises retrieval weight and half-life.
- A count that a shared `plan:` tag can raise measures co-occurrence in a run, not independent evidence.
- An undelivered run's confirmations are the same weak evidence that bug-2ad4fa keeps out of the store as new
  lessons.
- Goal: learning.

## Where

- `crates/roko-neuro/src/knowledge_store/scoring.rs::entries_are_similar`: the tag and word overlap.
- `crates/roko-neuro/src/knowledge_store/scoring.rs::detect_confirmations`: which pairs confirm. Since bug-2ad4fa it
  skips existing entries that are uncommitted or retired.
- `crates/roko-neuro/src/knowledge_store/crud.rs::ingest`: applies the confirmations and the tier promotions.
- `crates/roko-neuro/src/lifecycle.rs::promote_runtime_entries`: a pass confirms the committed entries its prompt
  surfaced.
- `crates/roko-neuro/src/knowledge_store/commit.rs::propose_batch`: where a run's batch commits or rolls back.

## Plan

1. **Match on topic, not on the run.**
   - Leave run-generic tags out of `entries_are_similar`'s tag overlap: `plan:*`, `gate:*`, `lesson`, `runtime`, kind
     names, and maybe package tags.
   - Require a real content overlap: a Jaccard floor on content words, not a count of 2.
2. **Confirm only on delivery.**
   - A guarded run records the confirmations its passes and lessons would give committed entries as pending, beside
     its batch.
   - `propose_batch` applies them when the batch commits and drops them when it rolls back.
   - An unguarded run (`off`) keeps applying them at once.
3. **Tests:**
   - `run_generic_tags_confirm_nothing`: two lessons sharing only `plan:`, `lesson` and `gate:passed` and two words
     don't confirm each other;
   - `an_undelivered_run_confirms_nothing`: a cancelled run's pass leaves the committed entries it surfaced and
     resembled unconfirmed.

### Design and measurement (roko-learnguard, 2026-10-26)

**What counts as a confirmation.** An ingested lesson confirms an existing entry when two things hold:
- they share a topical tag: a file or a package, which leaves out what every lesson of a run carries (`lesson`,
  `runtime`, the kind names, and `plan:*`, `gate:*`, `task:*`, `model:*`);
- their content words overlap by a Jaccard similarity of at least 0.17. Words of two characters or fewer, and
  prompt assembly's query stopwords, are left out.

`MIN_TAG_OVERLAP` stays at 1, over topical tags now. `MIN_KEYWORD_OVERLAP`, a count of 2 that "the" and "so" met,
becomes `MIN_CONTENT_JACCARD`. The promotion thresholds (2 confirmations, 3 contexts) don't change.

**No recorded decision covers these constants or the rule.** I searched `work/` (items, done, closed, parked, the
`dec-*` and `q-*` records), `tmp/cybernetic-harness/DECISIONS.md` and
`tmp/status-quo/2026-10-15/DECISIONS-2026-10-15.md`. The only rules are the design docs' promotion thresholds,
which stand.

**Measured on a read-only copy of Undertow's store** (2026-10-26 21:49: 55 entries, 796 confirmation records), with
`tmp/hermes-roko-v7/roko-learnguard/bin/confirm-report.py`, log `logs/confirm-report.log`:
- **The replay is faithful:** its mirror of the old rule reproduces all 796 records.
- **The new rule keeps 3 of the 796.**
- **The gamed lesson:** 31 rows name `runtime_3c3b9654100195f1`, and 0 survive. Run 1f677933's lessons confirmed 230
  pairs, and 0 survive.
- **Phases 0 to 4:** of their 414 rows, 2 survive, and both are real repeats:
  - phase 1's envelope lesson, said twice in other words ("`double` precision, not `float`" by T3#1 and T3#2);
  - phase 4.1's and 4.2's "the budget script rewrites the tracked 1x review PNGs directly".
- **The third survivor** is phase 5.1's "preset fixes need a paired master_gain trim" confirming phase 5's lesson on
  master_gain trimming.
- **The closest pair dropped** is two different facts about `tools/spec.sh`, at 0.152. The real repeats measured
  0.194 to 0.217.
- **The 43 rows of plan-writing distiller insights** have no topical tags, and none survive (see bug-f8655e).
- **One survivor wouldn't count now.** The envelope pair is two tries of one task in one run. Under bug-2ad4fa a
  run doesn't confirm its own uncommitted lessons, so that confirmation would no longer happen either.
- **Again after phase 5.2** (run 13260e60, a copy taken 2026-10-27 01:24: 58 entries, 854 records, log
  `logs/confirm-report-5-2.log`):
  - its new lesson, "The phase 5 demo script is deterministic …", made 38 confirmations under the old rule, and 0
    survive;
  - the 20 new distiller rows are dropped too;
  - the same 3 real repeats stand.
  - Its run's guard rows v16 and v17 already carry bug-2ad4fa's `delivered` check: "the run delivered plan
    p5-2-the-presets-as-named".

**roko's own store** (205 entries, 9,696 resolvable records) is mostly templated dream and hypothesis entries with
near-identical text. They still match: 2,756 of 9,312 pairs at 0.17. That is a separate problem, the templates
themselves (bug-7e0556), not this rule.

**Confirming only on delivery.**
- **The ledger:** a guarded run's confirmations of committed entries wait in
  `.roko/neuro/knowledge-pending-confirmations.jsonl`, one row per confirmation, tagged with the run's batch. A row
  is either `surfaced` (a pass's context and episode) or `repeat` (the confirming lesson and its episodes).
- **Commit:** `KnowledgeBatch::commit` applies them. Each pass and each repeat counts once, however often it was
  held, and an entry retired since gets nothing. The repeats' records are written then.
- **Rollback:** `discard` drops them.
- **A batch with no lesson:** `propose_batch` settles its held confirmations by delivery alone.
  `orphaned_batches` counts a batch that holds only confirmations.
- **No batch (`off`):** confirmations apply at once, as before.
- **Reinforcement stays immediate.** The balance bumps (retrieved, gated, cited, quoted) are no confirmation.

**A resume is a new batch.** The serve gives a resume a new uuid, and the CLI a new id. Its start rolls the dead
run's batch back, held confirmations included. `a_resumed_run_applies_its_pending_confirmations_once` covers it, a
pass that reaches the sink twice included.

## Done when

- The `[[verify]]` commands pass.
- A copy of Undertow's store replaying T4#3's lesson records no confirmation of the phase 0 to 4 lessons.

## Notes

- Evidence: Undertow `.roko/neuro/knowledge-confirmations.jsonl`: 31 records at 2026-10-26T15:57:28.045Z naming
  `runtime_3c3b9654100195f1`.
- `tmp/hermes-roko-v7/findings/roko-learnguard.md` has the analysis.
- Related: bug-2ad4fa (the delivered check, no same-run confirmation of provisional entries, retire), bug-3b3bb6
  (the gamed try).
