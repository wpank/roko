+++
id = "bug-eaa318"
kind = "bug"
title = "Efficiency rows never carry knowledge_ids; every ModelCallFeedback construction hardcodes it empty"
status = "open"
triage = "verified"
severity = "p2"
goal = "learning"
size = "M"
subsystem = ["roko-cli"]
created = 2026-10-05
updated = 2026-10-05
last_verified = 2026-10-05
source = "wave-19 follow-up reports 2026-10-05 (gap-5b8767, work/gap-addf2a)"
discovered_from = "gap-5b8767 (done on work/gap-addf2a; hand-check against a real binary showed 0/10)"
anchors = ["crates/roko-learn/src/model_call_feedback.rs::ModelCallFeedback", "crates/roko-learn/src/loop_audit/census.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn dispatch_with_knowledge_produces_an_efficiency_row_with_knowledge_ids' crates/roko-cli/ && cargo test -p roko-cli dispatch_with_knowledge_produces_an_efficiency_row_with_knowledge_ids"
+++

## Problem

Efficiency rows never carry `knowledge_ids`, so the loop census shows 0 of 10 for that link —
confirmed by `gap-5b8767`'s own hand-check against a real binary run (reported in its
implementing commit, `08501b033`): `roko learn loops --census` gave L-know "4 knowledge
exposures; 4/5 episodes and **0/10 efficiency rows carry knowledge ids**."

The census reads a top-level `knowledge_ids` key directly off each efficiency row:
`logs.efficiency_with_knowledge += usize::from(has_ids(&row["knowledge_ids"]));`
(`crates/roko-learn/src/loop_audit/census.rs:960`). `ModelCallFeedback`
(`crates/roko-learn/src/model_call_feedback.rs:32-41`) has a real `knowledge_ids: Vec<String>`
field that's threaded through to the row it feeds (line 203: `knowledge_ids:
feedback.knowledge_ids`), but every real production call site that constructs a
`ModelCallFeedback` hardcodes it empty:

- `crates/roko-cli/src/dispatch_v2.rs:2096-2100` (the Graph dispatch path): `prompt_section_ids:
  Vec::new(), knowledge_ids: Vec::new(),`
- `crates/roko-cli/src/chat.rs:352-356`: same, `Vec::new()`.
- `crates/roko-cli/src/vision_loop/evaluator.rs:154-...` and
  `crates/roko-serve/src/dispatch.rs:2140-...`: not confirmed to differ.

The data this field needs already exists elsewhere in the same dispatch flow:
`record_knowledge_access` (`crates/roko-cli/src/graph_task_dispatch/decision_log.rs:80-87`)
computes exactly this list from the same prompt — `plan.prompt.diagnostics.items.iter().filter(|item|
item.kind == ExposureItemKind::Knowledge && item.included)` — for a completely different purpose
(access counting), but it's never passed into the `ModelCallFeedback` that would let the census
see it too.

## Why it matters

Goal: learning, same goal as `gap-5b8767`. The census's own "efficiency rows carry knowledge
ids" check exists to confirm knowledge actually reaches the efficiency log, not just episodes —
with every real call site hardcoding an empty list, that check can never pass for a live
workspace, understating L-know's measured link even when knowledge is genuinely included and
genuinely reinforced (as `gap-5b8767`'s own fixture now demonstrates on the episode side).

## Where

- `crates/roko-learn/src/model_call_feedback.rs::ModelCallFeedback` (the field; already
  threaded to the row, just never filled).
- `crates/roko-cli/src/dispatch_v2.rs`, `crates/roko-cli/src/chat.rs`,
  `crates/roko-cli/src/vision_loop/evaluator.rs`, `crates/roko-serve/src/dispatch.rs` (all four
  real construction sites).
- `crates/roko-cli/src/graph_task_dispatch/decision_log.rs::record_knowledge_access` (the
  existing computation of the same ids, for a different purpose).
- `crates/roko-learn/src/loop_audit/census.rs:960` (the reader; read-only reference).

## Current state

Confirmed: the field exists end-to-end in the type system (`ModelCallFeedback` ->
the row written to `efficiency.jsonl`), but every real caller passes `Vec::new()`.

## Plan

1. At each of the four dispatch sites, compute the prompt's included knowledge-item ids (the
   same filter `record_knowledge_access` already applies) and pass them as
   `ModelCallFeedback.knowledge_ids` instead of `Vec::new()`.
2. Regression test: a dispatch whose prompt included a knowledge entry produces an efficiency
   row whose `knowledge_ids` is non-empty, and the loop census's `efficiency_with_knowledge`
   count reflects it.

## Done when

- A dispatch with knowledge in its prompt produces an efficiency row carrying
  `knowledge_ids`, and the census's L-know link is no longer 0 of N by construction.
- The `[[verify]]` command passes.

## Notes

- 2026-10-05 (wave-19 follow-up, gap-5b8767, work/gap-addf2a not yet merged): the 0/10 figure
  comes from `gap-5b8767`'s own implementing commit's hand-check against a real binary run.
  Confirmed independently here by reading `ModelCallFeedback`'s field and all four real
  (non-test) construction sites directly.
