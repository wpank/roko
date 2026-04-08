# 10 - Dreams Consolidation: Trigger, Worker, Advice, And Cross-Run Influence

This file is the active implementation handoff for making dreams part of the runner feedback loop.

Scope:

- Automatic dream trigger records after plan completion or idle periods.
- A non-blocking worker that consumes trigger records and runs consolidation.
- Cross-episode clustering and dream-derived routing/prompt advice.
- Proof that a later run can consume dream outputs.

## 1. Current Verdict

Self-grade for the 2026-04-27 deepening pass:

- Initial rating: 9.90 / 10.
- Reasoning: this file is now source-corrected, concise, and implementation-grade. It distinguishes durable trigger emission from actual consolidation and gives concrete batches, generated proof schemas, and archive gates. The score is not higher because no generated proof yet shows trigger drain, consolidation, routing advice consumption, prompt influence, and queryability in one active-runner loop.

Roko has most dream primitives now, but the active runner only proves trigger emission.

- [x] `crates/roko-cli/src/runtime_feedback/dreams.rs` exists and defines `DreamTriggerSink`.
- [x] `DreamTriggerSink` consumes `FeedbackEvent::PlanCompleted` and eligible `FeedbackEvent::IdleTick`.
- [x] `DreamTriggerSink` appends trigger records to `.roko/learn/dream_triggers.jsonl`.
- [x] `DreamTriggerSink::with_runner` can call an attached runner immediately for tests or eager hosts.
- [x] `crates/roko-cli/src/commands/plan.rs` wires `DreamTriggerSink::at(.roko/learn/dream_triggers.jsonl)` into the active runner feedback facade.
- [x] `crates/roko-cli/src/runner/event_loop.rs` translates `RunnerEvent::PlanCompleted` into `FeedbackEvent::PlanCompleted`.
- [x] `crates/roko-dreams/src/runner.rs` defines `DreamRunner`, `DreamLoopConfig`, `DreamTrigger`, and `PlanCompletionTriggerPolicy`.
- [x] `crates/roko-dreams/src/routing_advice.rs` defines `DreamRoutingAdvice`, `RoutingRecommendation`, `PatternSummary`, load/save helpers, and bias conversion.
- [x] `crates/roko-dreams/src/cycle.rs` uses cross-episode consolidation and writes dream outputs.
- [x] `crates/roko-serve/src/dreams.rs` contains a serve/daemon dream loop that can run consolidation when auto-dreaming is enabled.
- [ ] Active `plan run` writes trigger records but does not attach a real `DreamRunner` by default.
- [ ] No active-runner worker is proven to consume `.roko/learn/dream_triggers.jsonl`.
- [ ] Active dispatch/routing is not proven to read `.roko/learn/dream-routing-advice.json`.
- [ ] Active prompt assembly is not proven to inject dream-derived pattern summaries.
- [ ] Dream start/skip/claim/complete/fail projection events are not proven.
- [ ] No generated proof shows first-run dream output influencing a later dispatch.

## 2. What Was Broken Historically

Older versions of this file described three disconnected pieces:

- [x] Dream primitives existed but were manual-only.
- [x] Cross-episode clustering existed but was not part of the dream cycle.
- [x] Dream clusters had no route into model routing or prompts.

Current source corrected part of that:

- [x] Cross-episode consolidation and routing advice exist in `roko-dreams`.
- [x] Serve/daemon paths can run dream loops.
- [x] Active runner feedback can write durable trigger records.

The remaining problem is more precise:

- [ ] Trigger records are not equivalent to completed consolidation.
- [ ] Completed consolidation is not equivalent to future dispatch influence.
- [ ] Future dispatch influence is not equivalent to queryable proof.

## 3. Target Runtime Semantics

Dreams should be a feedback subscriber and worker, not hot-path runner glue.

- [ ] Runner emits `PlanCompleted` and `IdleTick` feedback events.
- [ ] Dream feedback sink records durable trigger requests with run id, plan id, task counts, episode counts, config, and trigger reason.
- [ ] A dream worker claims trigger requests idempotently.
- [ ] The dream worker applies `PlanCompletionTriggerPolicy` or configured policy before expensive consolidation.
- [ ] The dream worker runs `DreamRunner::consolidate_now` outside the runner event loop.
- [ ] Dream outputs are persisted as reports, routing advice, staged knowledge, and prompt pattern summaries.
- [ ] Later dispatch reads dream advice through the same prompt/routing services as other feedback inputs.
- [ ] All lifecycle steps emit durable projection/query events.
- [ ] Dream failures are observable but non-fatal to the plan run.

## 4. Source Anchors

Use these files before implementing:

- [ ] `crates/roko-cli/src/runtime_feedback/dreams.rs`: active trigger sink and trigger record writer.
- [ ] `crates/roko-cli/src/runtime_feedback/mod.rs`: feedback event vocabulary and fan-out.
- [ ] `crates/roko-cli/src/commands/plan.rs`: feedback facade wiring for `plan run`.
- [ ] `crates/roko-cli/src/runner/event_loop.rs`: `RunnerEvent` to `FeedbackEvent` translation.
- [ ] `crates/roko-dreams/src/runner.rs`: `DreamRunner`, config, trigger policy, scheduling.
- [ ] `crates/roko-dreams/src/cycle.rs`: dream cycle, reports, cross-episode consolidation.
- [ ] `crates/roko-dreams/src/routing_advice.rs`: advice schema and routing-bias conversion.
- [ ] `crates/roko-serve/src/dreams.rs`: serve/daemon background dream loop.
- [ ] `crates/roko-cli/src/dispatch/mod.rs`: dispatch facade that should consume route/prompt influence.
- [ ] `crates/roko-cli/src/dispatch/prompt_builder.rs`: prompt diagnostics and context assembly.

## 5. Implementation Batches

### DR-01: Trigger Record Schema And Idempotency

- [ ] Add a schema version to dream trigger records.
- [ ] Add run id, plan id, trigger id, trigger kind, created_at, policy version, and source event id.
- [ ] Include task counts, failed task counts, total cost, recent episode count, and idle ticks when known.
- [ ] Derive stable trigger ids so replay does not duplicate consolidation.
- [ ] Add processed/claimed state using offsets, receipt ids, or a separate ledger.
- [ ] Add tests for plan-completed and idle trigger records.
- [ ] Add proof that `plan run` emits exactly one terminal trigger per completed plan.

### DR-02: Dream Trigger Worker

- [ ] Implement a worker that reads `.roko/learn/dream_triggers.jsonl`.
- [ ] Claim each unprocessed trigger exactly once.
- [ ] Load `DreamLoopConfig` from runtime config.
- [ ] Apply `PlanCompletionTriggerPolicy` or configured gating before consolidation.
- [ ] Run `DreamRunner::consolidate_now` outside the runner event loop.
- [ ] Mark each trigger as `skipped`, `completed`, or `failed` with reason.
- [ ] Make worker errors non-fatal to the plan runner.
- [ ] Store worker evidence in `tmp/mori-diffs/generated/dream-trigger-worker-proof.json`.

### DR-03: Active Runner Integration

- [ ] Ensure every terminal plan completion emits one `FeedbackEvent::PlanCompleted`.
- [ ] Ensure trigger records include real task completed/failed counts, not placeholder zeros.
- [ ] Optionally emit idle ticks from runner or daemon when no work is active.
- [ ] Confirm runner does not block on consolidation unless explicitly configured for synchronous proof mode.
- [ ] Emit projection events for trigger written, trigger skipped, trigger claimed, consolidation started, consolidation completed, and consolidation failed.
- [ ] Store active-runner evidence in `tmp/mori-diffs/generated/dream-active-runner-proof.json`.

### DR-04: Cross-Episode Dream Output

- [ ] Confirm `DreamRunner::consolidate_now` writes a dream report under `.roko/dreams/`.
- [ ] Confirm cross-episode meta-patterns are included when enough episodes exist.
- [ ] Confirm dream routing advice is saved under `.roko/learn/dream-routing-advice.json`.
- [ ] Confirm staged/promoted knowledge outputs are recorded where applicable.
- [ ] Add deterministic fixture episodes that produce at least one routing recommendation.
- [ ] Store output evidence in `tmp/mori-diffs/generated/dream-output-proof.json`.

### DR-05: Routing Advice Consumption

- [ ] Load `.roko/learn/dream-routing-advice.json` in the dispatch routing service, not only legacy `orchestrate.rs`.
- [ ] Convert matching advice to a bounded routing bias.
- [ ] Record advice ids, confidence, supporting episode count, and bias decisions in route diagnostics.
- [ ] Avoid hardcoded model-name tier replacement in the active path; use router/model tier metadata.
- [ ] Prove a deterministic advice fixture changes or explicitly influences model choice.
- [ ] Store routing evidence in `tmp/mori-diffs/generated/dream-routing-advice-proof.json`.

### DR-06: Prompt Pattern Injection

- [ ] Load dream pattern summaries through `PromptAssembler`.
- [ ] Filter summaries by task type, domain, context, and confidence.
- [ ] Apply a strict token budget and redaction rules.
- [ ] Record included and dropped dream pattern ids in prompt diagnostics.
- [ ] Prove a later dispatch includes a dream-derived pattern in prompt diagnostics.
- [ ] Store prompt evidence in `tmp/mori-diffs/generated/dream-prompt-influence-proof.json`.

### DR-07: Query And Observability

- [ ] Expose dream trigger records through CLI or HTTP query.
- [ ] Expose dream worker state through CLI or HTTP query.
- [ ] Expose latest dream report and dream routing advice through CLI or HTTP query.
- [ ] Emit projection events for trigger lifecycle and consolidation lifecycle.
- [ ] Include trigger id, run id, plan id, dream report id, and advice id in logs.
- [ ] Store query evidence in `tmp/mori-diffs/generated/dream-query-proof.json`.

### DR-08: Cross-Run Closed-Loop Proof

- [ ] Create or replay enough episodes for a dream trigger.
- [ ] Run active `plan run` and prove a trigger record is written.
- [ ] Run the dream trigger worker and prove a dream report is written.
- [ ] Prove dream routing advice or pattern summaries are produced.
- [ ] Run a second dispatch and prove prompt or routing diagnostics cite the dream output.
- [ ] Query the evidence through HTTP or CLI.
- [ ] Store evidence in `tmp/mori-diffs/generated/dream-cross-run-proof.json`.

## 6. Generated Proof Contract

An agent implementing this file must create `tmp/mori-diffs/generated/dreams-proof-report.json`:

```json
{
  "schema": "mori-diffs.dreams-proof.v1",
  "generated_at": "ISO-8601 timestamp",
  "git_commit": "HEAD sha",
  "trigger_emission": {
    "proved": false,
    "path": ".roko/learn/dream_triggers.jsonl",
    "trigger_count": 0,
    "real_task_counts": false
  },
  "worker": {
    "claimed_trigger": false,
    "skipped_with_reason": false,
    "completed_consolidation": false,
    "failed_non_fatal": false
  },
  "outputs": {
    "dream_report": false,
    "routing_advice": false,
    "pattern_summaries": false,
    "staged_knowledge": false
  },
  "influence": {
    "route_diagnostics_reference_dream_advice": false,
    "prompt_diagnostics_reference_dream_patterns": false,
    "second_run_consumed_first_run_output": false
  },
  "queries": {
    "http": false,
    "cli": false,
    "projection_events": []
  },
  "evidence_paths": [],
  "remaining_gaps": []
}
```

## 7. No-Context Handoff Checklist

Use this exact order if another agent receives only this file:

- [ ] Run `rg -n "DreamTriggerSink|DreamTrigger|DreamRunner|PlanCompletionTriggerPolicy|dream_triggers|dream-routing-advice|dream_advice_to_routing_bias|PromptAssembler|PlanCompleted|IdleTick" crates`.
- [ ] Verify the source-corrected checked items in section 1.
- [ ] Implement DR-01 before building a worker; replay/idempotency must be explicit first.
- [ ] Implement DR-02 before claiming automatic consolidation.
- [ ] Implement DR-03 before claiming active-runner integration.
- [ ] Implement DR-04 before claiming useful dream output.
- [ ] Implement DR-05 before claiming routing influence.
- [ ] Implement DR-06 before claiming prompt influence.
- [ ] Implement DR-07 before claiming observability parity.
- [ ] Implement DR-08 before claiming Mori-like closed-loop learning.
- [ ] Generate `tmp/mori-diffs/generated/dreams-proof-report.json`.
- [ ] Update [README.md](README.md), [21-FEATURE-PARITY-MATRIX.md](21-FEATURE-PARITY-MATRIX.md), [29-CURRENT-RUNTIME-GAP-LEDGER.md](29-CURRENT-RUNTIME-GAP-LEDGER.md), and [38-COGNITIVE-FEEDBACK-LOOP-AUDIT.md](38-COGNITIVE-FEEDBACK-LOOP-AUDIT.md).

## 8. Acceptance Criteria

- [ ] Manual `roko knowledge dream run` still works.
- [ ] A real runner completion writes a dream trigger with real counts.
- [ ] A worker or attached runner consumes that trigger and runs consolidation.
- [ ] Dream output writes a report and routing advice or pattern summaries.
- [ ] A later dispatch consumes dream output through prompt or routing diagnostics.
- [ ] Dream lifecycle is queryable through HTTP or CLI.
- [ ] Dream failures are non-fatal and visible in projections.

## 9. Archive Gate

Do not archive this file until:

- [ ] `tmp/mori-diffs/generated/dreams-proof-report.json` exists.
- [ ] Trigger emission is proved from a real active-runner plan.
- [ ] Trigger consumption is proved.
- [ ] Consolidation output is proved.
- [ ] Future dispatch influence is proved.
- [ ] Query/projection evidence is proved.
- [ ] Remaining gaps are either complete or moved to the canonical active ledger.
