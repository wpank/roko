+++
id = "spec-b7303f"
kind = "spec"
title = "Epic: one settled record per attempt"
status = "open"
triage = "unverified"
severity = "p0"
goal = "truth"
size = "L"
subsystem = ["roko-learn/telemetry", "roko-cli/graph_task_dispatch", "roko-cli/runtime_feedback", "roko-cli/graph_execution"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e4"
discovered_from = "tmp/cybernetic-harness/tldr/05-GAPS-AND-PROPOSALS.md (P0 #4); specs/S01-instrumentation.md (Phase 0)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/feedback.rs::GraphTaskDispatcher::emit_feedback", "crates/roko-cli/src/runtime_feedback/mod.rs::FeedbackEvent", "crates/roko-learn/src/routing_log.rs::RoutingDecisionLog", "crates/roko-cli/tests/learning_wiring_census.rs"]
doc = "tmp/cybernetic-harness/workstreams/PLAN.md"
lane = "rust-hot"
links = { depends_on = ["gap-528762", "gap-96f7ed", "bug-c34782", "bug-35379d", "gap-ad0d39", "bug-690dc6", "gap-8cb382", "gap-1f2661", "gap-c7c946", "gap-3c430e", "bug-b72a37", "gap-4468bd", "gap-8f6206", "bug-ccc7c4", "bug-55fd84", "bug-31438d", "bug-62e3f4", "bug-2379dc", "bug-92f655", "bug-220385", "bug-ad5487", "bug-b2dd44", "bug-c65bfe", "gap-751ac9", "bug-0ba3d9", "bug-d5fb74", "bug-aa2044"], blocks = [], related = ["gap-7a8474", "bug-f9ae3e", "spec-e9d7ec"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn loop_census_fixture_settles_one_record_per_attempt' crates/roko-cli/tests/ && cargo test -p roko-cli --test learning_wiring_census"
+++
## Problem

An attempt leaves several partial records that cannot be joined, and none of them is the settled truth:

- attempt ids are unique only within one process, so a resumed run reuses them;
- `emit_feedback(succeeded: bool)` collapses the verdict, so `Unverified` counts as a success;
- after failover the executed model is recorded as if it had been chosen;
- cost records have no source;
- `RunMetricsRecord` mints a second run id;
- no decision record has a writer on the Graph path.

**Outcome.** Every attempt leaves one record: the attempt key, a typed verdict, the model that actually ran, and its
cost with the source of that cost, killed attempts included. The router, the cost reports and every learning loop
read that record.

## Why it matters

This is tldr/05 P0 #4. The whitepaper's cost per verified task depends on it, and so does every S02–S06 loop, since
each must be keyed by attempt and labelled by verdict. W5 rec 4 makes the settled outcome the hook that learning loops
attach to.

## Where

- `crates/roko-learn/src/telemetry/` (new) and `routing_log.rs`.
- `graph_task_dispatch.rs`: `dispatch`, `settle_task_verification`, `emit_feedback` and `next_attempt_id`.
- `runtime_feedback/mod.rs`: `FeedbackEvent` and `FeedbackFacade`.
- `plan_runner.rs`: the facade block and the run metrics.
- `build.rs`, `roko-core/src/config/`, and `commands/learn.rs`.

## Current state

Checked at `41c7ffbd6`:
- **Configuration hash:** `ConfigHash::of` exists, but nothing in production calls it.
- **Tests:** `phase0_wiring.rs` holds smoke tests, not a census.
- **bug-c34782:** `725f21e05` fixed its headline, the pre-gate success flag (W3a). What remains is the `bool` at
  `graph_task_dispatch.rs:4038`, which the `ce3bdcbb8` merge did not change.
- **bug-35379d and gap-ad0d39:** the static checks in their verify commands still fail.
- **bug-690dc6:** the fix merged in `d4be4e872` (`e0673e3e0`). Its verify, though, names
  `dispatch_error_records_task_spend`, which the merge did not add; the merged test is
  `a_timed_out_attempt_settles_the_spend_it_streamed`. Before closing, check the errored path that did not time out,
  then either add that test or correct the verify.
- **In flight:** the learn-a worktree (`feat/learning-completion-loops`, uncommitted) edits `runtime_feedback/mod.rs`,
  `episodes.rs`, `graph_task_dispatch.rs` and `plan_runner.rs`.

## Plan

This is the implementation plan.

1. **Now, in `rust-cold`:**
   - gap-528762, the types and the writer;
   - the library half of gap-8cb382;
   - closing bug-690dc6 once its verify is sorted out.
2. **After the portal branches merge and the dispatch split (gap-c8e1f1) lands:** gap-96f7ed threads the key and
   publishes one settled outcome per attempt.
3. **Then the consumers, one at a time per module:**
   - bug-c34782: the router reads `learning_label`;
   - bug-35379d: the executed model;
   - gap-ad0d39: the cost source.
4. **Last:** gap-1f2661, the census, which is the exit check; and gap-c7c946, the report.

**Out of scope:** S01.P0-4, P0-5, P0-8, P0-9 and P0-10 (rung verdicts, timings, the route decision writer, exposures
and state digests). The census lists them in `EXPECTED_MISSING`, and E17 files them.

## Done when

- [x] gap-528762: Attempt records: AttemptKey, record types and a telemetry writer (S01.P0-0)
- [x] gap-96f7ed: Thread the attempt context through dispatch and settle one outcome per attempt (S01.P0-1)
- [x] bug-c34782: Cascade router learns from the provider call's success flag before gates run (existing item)
- [ ] bug-35379d: Provider failover silently runs a different model and records it as if it had been chosen (existing item)
- [ ] gap-ad0d39: Cost records miss Claude per-model usage and reasoning tokens, and price unknown models as Sonnet (existing item)
- [x] bug-690dc6: Timed-out and errored agent runs are recorded at $0 with no tokens (existing item; fix merged in `d4be4e872`)
- [x] gap-8cb382: Run manifest with a config fingerprint for every plan run (S01.P0-2)
- [x] gap-1f2661: Wiring census: a fixture proves every learning loop reads the settled attempt record (S01.P0-11, S01.P0-12)
- [x] gap-c7c946: roko learn telemetry: check and route-report over the attempt records (S01.P0-13)
- [x] gap-3c430e: Specs S01, S05, S06 and S08 disagree on token classes, the audit hash, a decision-point name and a budget-line name
- [ ] bug-b72a37: OpenAI-compatible providers price cached input tokens twice
- [ ] gap-4468bd: Credit or demote a T0 reflex rule only from the settled attempt record, after verify
- [x] gap-8f6206: Learning consumers read the settled verdict's learning label instead of succeeded (S01.P0-3)
- [x] bug-ccc7c4: roko run now writes a second run directory under .roko/runs beside its own
- [ ] bug-55fd84: Episodes record turns = 1 for dispatches on providers that report no turn count
- [ ] bug-31438d: Roko records the model it dispatched, never the model the provider reports serving
- [ ] bug-62e3f4: Episodes, costs.json and efficiency.jsonl leave out the three helper calls after each failed gate
- [ ] bug-2379dc: When a CLI names no model, the CLI adapters still record the configured slug as the served model
- [ ] bug-92f655: ModelCallService's model_call rows carry no model_reported and no attempt key
- [ ] bug-220385: Calls refused for provider exhaustion during failover leave no record
- [ ] bug-ad5487: Gate rows in verification.rs still record turn 1 when the attempt's turn count is unknown
- [ ] bug-b2dd44: The streaming dispatch path ignores a substituted pinned model, which the batch path fails as model_substituted
- [ ] bug-c65bfe: The tool loop leaves usage_obs.source as Unknown
- [ ] gap-751ac9: Graph attempts keep no durable record of their Claude Code isolation settings; the invocation is only debug-logged
- [ ] bug-0ba3d9: Attempt records leave inv null: with_inv is called only in tests
- [ ] bug-d5fb74: A run resumed under a different build keeps the first invocation's harness and config in its manifest
- [ ] bug-aa2044: Stalled attempts record no cost: the watchdog drops the provider before it reports usage
- [ ] The epic's `[[verify]]` command passes on the merged branch: the census fixture shows, for every attempt, a row
      with the attempt key, verdict, executed model and cost source.

## Notes

- **Existing children keep their goal and severity:** bug-c34782 stays in `learning`; bug-35379d, gap-ad0d39 and
  bug-690dc6 stay in `core`. `PLAN.md` §5 proposes moving them to `truth`.
- **Order:** the three consumer items should follow gap-96f7ed. Their own files do not say so; this plan sets the
  order.
- **Hot files:** `graph_task_dispatch.rs`, `plan_runner.rs` and `runtime_feedback/`. One writer per file.
- **Related:**
  - gap-7a8474 and bug-f9ae3e: efficiency records;
  - spec-e9d7ec: honest verdicts, which this epic builds on.
