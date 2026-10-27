+++
id = "gap-1220c5"
kind = "gap"
title = "Knowledge confirmations count what every lesson of a run shares: entries_are_similar needs 1 shared tag and 2 shared words, and plan:, lesson and gate:passed make that near-universal, so phase 5's gamed lesson 'confirmed' 31 committed entries; a run's passes also confirm committed entries whether or not the run delivers"
status = "open"
triage = "verified"
severity = "p2"
goal = "learning"
size = "M"
subsystem = ["roko-neuro/knowledge", "roko-neuro/lifecycle"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
source = "round-v7 roko-learnguard while fixing bug-2ad4fa (Undertow run 1f677933's knowledge-confirmations.jsonl), the lead's ask"
discovered_from = "agent:roko-learnguard (round v7)"
anchors = ["crates/roko-neuro/src/knowledge_store/scoring.rs::entries_are_similar", "crates/roko-neuro/src/knowledge_store/scoring.rs::detect_confirmations", "crates/roko-neuro/src/knowledge_store/crud.rs::ingest", "crates/roko-neuro/src/lifecycle.rs::promote_runtime_entries"]
links = { depends_on = [], blocks = [], related = ["bug-2ad4fa", "bug-3b3bb6", "gap-6755f7"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn run_generic_tags_confirm_nothing' crates/roko-neuro/src && cargo test -p roko-neuro run_generic_tags_confirm_nothing"

[[verify]]
command = "grep -rqw 'fn an_undelivered_run_confirms_nothing' crates/roko-neuro/src && cargo test -p roko-neuro an_undelivered_run_confirms_nothing"
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

## Done when

- The `[[verify]]` commands pass.
- A copy of Undertow's store replaying T4#3's lesson records no confirmation of the phase 0 to 4 lessons.

## Notes

- Evidence: Undertow `.roko/neuro/knowledge-confirmations.jsonl`: 31 records at 2026-10-26T15:57:28.045Z naming
  `runtime_3c3b9654100195f1`.
- `tmp/hermes-roko-v7/findings/roko-learnguard.md` has the analysis.
- Related: bug-2ad4fa (the delivered check, no same-run confirmation of provisional entries, retire), bug-3b3bb6
  (the gamed try).
