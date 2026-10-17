+++
id = "bug-0a7ec1"
kind = "bug"
title = "On Graph runs every knowledge outcome stays 'partial': dispatch feedback records run_id None in a per-call FeedbackService, so no verdict credits or debits the lessons a prompt carried"
status = "done"
triage = "verified"
severity = "p3"
goal = "learning"
size = "M"
subsystem = ["roko-cli/dispatch_v2", "roko-learn/feedback_service"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
last_verified_rev = "0969fdc2a"
source = "round-v6-dogfood-will#R3-9"
discovered_from = "agent:dogfood-will (round v6 run 3, 2026-10-16)"
anchors = ["crates/roko-cli/src/dispatch_v2.rs::record_agent_dispatch_feedback", "crates/roko-learn/src/feedback_service.rs::FeedbackService::record_outcome_for_run"]
lane = "rust-hot"
parent = "spec-6ac537"
links = { depends_on = [], blocks = [], related = ["find-96f541", "bug-eaa318"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn graph_attempt_verdict_settles_its_knowledge_outcome' crates/roko-cli/ && cargo test -p roko-cli --lib graph_attempt_verdict_settles_its_knowledge_outcome"

[closed]
at = 2026-10-17
at_ts = "2026-10-17T14:16:41Z"
commit = "67716f4d5"
by = "roko-delivery"
executor = "claude-agent"
via = "manual"
size = "M"
claimed_at = "2026-10-17T11:11:35Z"
forced = false
evidence = "67716f4d5 (on the merged stack efc9f0443, main 7a5dd6046): graph_attempt_verdict_settles_its_knowledge_outcome and attempt_knowledge_outcome_carries_its_run_and_moves_scores pass (roko-cli lib 3777, roko-learn lib 1373 passed; the verify now names --lib, where the test lives). A passing verify leaves one success row with the run id and the attempt key and a score of 1; a failing one failure and -1. Scripted showcase at no cost (tmp/hermes-roko-v6/roko-delivery/evidence/trio): p2 and p3 each leave a success row with their run id and attempt key for the stored lesson, and knowledge-scores.json holds 2 for it, where main eb7b98ad0 wrote partial rows with run_id null and no scores (evidence/trio-before). The scripted model fails no attempt that carried the lesson, so the showcase has no failure row."
+++

## Problem

`knowledge-feedback.jsonl` is meant to tie each prompt's knowledge entries to the outcome they contributed to:
`KnowledgeOutcome::Success` or `Failure`, a score of ±1, and per-entry success rates in `compute_knowledge_scores`. On
Graph plan runs it only ever records `partial`.

Round v6 dogfood run 3 (2026-10-16, roko 47b3cdd20, learning on) shows this:
- There are 3 rows, for SVF attempt 7, RENDER attempt 2 and the pulse-width run's T1 attempt 2. All three attempts
  passed their checks.
- Each row says `outcome: "partial"`, `score_delta: 0` and `run_id: null`.
- There is no `success` or `failure` row, and `knowledge-scores.json` was never written.

The mechanism:
- `record_agent_dispatch_feedback` builds a fresh `ModelCallFeedbackRecorder`, and so a new `FeedbackService`, for each
  provider call, and records `ModelCallFeedback { run_id: None, … }`.
- On a successful call, `FeedbackService::record` keeps the provenance in that service's in-memory map and writes
  `Partial`.
- The outcome that should settle it goes through `record_outcome_for_run(run_id)`: a `GateResult` or `WorkflowComplete`
  event. That matches provenance by run id, which is `None` here. The provenance also lives in another, already dropped
  service instance. So nothing is ever settled.
- A failed call writes nothing (`*success` is false), so a lesson in a failing attempt is never debited either.

## Why it matters

Goal `learning`: this is the knowledge loop's own outcome record. As it stands, the file is write-only noise
("partial" for passes), and any reader of per-entry success rates sees 0 successes.

The neuro lifecycle does reinforce entries on a gate pass (`knowledge-lifecycle.jsonl`, `gated_reinforcements`), so the
store's confidence moves; this record does not. Related: find-96f541 (confidence rises on retrieval, not use) and
bug-eaa318 (knowledge ids on efficiency rows, done).

## Where

- `crates/roko-cli/src/dispatch_v2.rs::record_agent_dispatch_feedback`: the per-call recorder, with `run_id: None`.
- `crates/roko-learn/src/feedback_service.rs::FeedbackService::record_outcome_for_run`, with
  `remember_model_call_provenance` and `take_provenance_for_run`: provenance kept in memory, keyed by request id and
  settled by run id.
- `crates/roko-learn/src/feedback_service.rs::KnowledgeOutcome`.
- The Graph attempt's verdict settles in `crates/roko-cli/src/graph_task_dispatch/attempt.rs` (the `AttemptBook`
  settlement). That point knows the attempt key, the knowledge ids it carried (its exposures) and the verdict.

## Current state

No production code reads `knowledge-scores.json` or calls `compute_knowledge_scores` today; only roko-compose and
roko-learn tests do. The loop census (L-know) reads efficiency rows and exposures, not this file. So the damage is
false data, not a wrong decision. It becomes a wrong decision as soon as anything ranks lessons by these scores.

## Plan

Options:
1. Settle on the attempt's verdict. When a Graph attempt settles, write one knowledge outcome row per included
   knowledge id, keyed by `attempt_key` and carrying the run id: `Success` for a pass, `Failure` for an agent-blamed
   failure, nothing for an infra or harness failure. Stop writing `Partial` from the per-call recorder on the Graph
   path.
2. Or drop the per-call knowledge feedback on the Graph path and document that the exposures and verdict records hold
   the pairing.

Add the test `graph_attempt_verdict_settles_its_knowledge_outcome`: a Graph attempt whose prompt carried a stored entry
and whose verify passed leaves one `success` row, with the run id, for that entry. A failed verify leaves `failure`.

## Done when

- The `[[verify]]` passes.
- A learning-on Graph run's `knowledge-feedback.jsonl` holds `success` and `failure` rows with run ids, not `partial`
  rows.

## Notes

- Respect decision 2218: `learning_frozen` writes no knowledge feedback.
- Evidence: `tmp/hermes-roko-v6/dogfood-will/evidence/run3/learn/knowledge-feedback.jsonl`,
  `…/learn/efficiency.jsonl` (its `model_call` rows have `run_id: null`) and `…/neuro/knowledge-lifecycle.jsonl`.
