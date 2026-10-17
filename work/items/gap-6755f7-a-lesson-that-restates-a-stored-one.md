+++
id = "gap-6755f7"
kind = "gap"
title = "A lesson that restates a stored one in other words is admitted as a new entry, so paraphrases of one rule crowd the three knowledge slots"
status = "open"
triage = "verified"
severity = "p3"
goal = "learning"
size = "S"
subsystem = ["roko-neuro/lifecycle"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
source = "round-v6-learning-research#4c.16"
discovered_from = "agent:learning-research (round v6, 2026-10-16)"
anchors = ["crates/roko-neuro/src/lifecycle.rs::RuntimeKnowledgeLifecycle::admit_runtime_candidate", "crates/roko-neuro/src/lifecycle.rs::RUNTIME_DUPLICATE_SIMILARITY", "crates/roko-neuro/src/knowledge_store/query.rs::find_similar_entry"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-7e0556", "bug-dee062"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn paraphrased_lesson_confirms_the_stored_one' crates/roko-neuro/ && cargo test -p roko-neuro paraphrased_lesson_confirms_the_stored_one"
+++

## Problem

A lesson that restates a stored one in other words is admitted as a new entry, not as a confirmation. In round v6's
live run (`tmp/hermes-roko-v6/learning-research/evidence/live/workspace/.roko/neuro/knowledge.jsonl`), three similar
tasks each ended with a verified `Lesson:`, and all three were `light_admitted`:
1. `runtime_adc27866…`: "In textkit slugs, apostrophes and similar word-internal punctuation are removed entirely (not
   converted to separators), so "Don't" becomes "dont" not "don_t"."
2. `runtime_ec96bea6…`: "In textkit, URL slugs use underscores (`_`) as word separators, not hyphens (`-`), and
   apostrophes are removed entirely rather than converted to separators."
3. `runtime_aa36c660…`: "The repository defines textkit URL slugs using lowercase alphanumerics, underscores as
   separators, and completely removes apostrophes."

3 restates 2, and 2 subsumes 1. A prompt shows at most 3 knowledge entries, so in a growing store restatements of one
rule crowd out other lessons. (Here they helped: the third task passed on the cheap rung with 1 and 2 in its prompt.)

## Why it matters

Goal `learning`. Lessons are the loop that visibly makes later runs cheaper. Without deduplication or consolidation, the
store fills with paraphrases. Consolidation is the dreams' job today, and dreams are unsafe (bug-7e0556;
bug-dee062 merges by tags).

## Where

- `crates/roko-neuro/src/lifecycle.rs::RuntimeKnowledgeLifecycle::admit_runtime_candidate`: a candidate whose
  `max_similarity` is over `RUNTIME_DUPLICATE_SIMILARITY` (0.8) confirms the most similar entry
  (`find_similar_entry`); otherwise the light gate admits it.
- `crates/roko-neuro/src/knowledge_store/query.rs::find_similar_entry` and `entry_similarity`: the similarity used.
- `crates/roko-neuro/src/lifecycle.rs::RUNTIME_DUPLICATE_SIMILARITY`.

## Current state

Exact repeats confirm (tested: `second.admission_path == Duplicate`). Paraphrases fall under 0.8 on the current
similarity and are admitted.

## Plan

1. Measure: compute `entry_similarity` for the three lessons above (add them as a fixture).
2. Make restatements confirm, without merging different rules:
   - compare on normalized content terms (lowercased, stopwords and generic words removed, as
     `prompt_builder::topic_terms` does), with a containment test: a candidate whose terms are mostly inside one
     entry's confirms that entry;
   - or keep the newest, more complete statement and mark the older one superseded, with provenance kept.
3. Test `paraphrased_lesson_confirms_the_stored_one`: with lessons 1–3 as fixtures, 3 confirms 2, and 1 and 2 stay
   distinct, or 2 supersedes 1 with a recorded link.

## Done when

The `[[verify]]` command passes, and re-running the round-v6 live lessons through admission leaves two entries, not
three.

## Notes

Don't lower the threshold globally: unrelated rules that share a domain's vocabulary must stay separate (bug-dee062 is
the cautionary tale).
