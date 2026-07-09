# 37 -- Learning & Feedback Subsystem: Dead Code, Write-Only Sinks, Broken Loops

Scope: deep audit of the learning/feedback subsystem across `roko-learn`,
`roko-cli/src/runtime_feedback/`, `roko-cli/src/runner/event_loop.rs`, and their
consumers. Focuses on write-only data paths, dead modules, broken feedback loops,
and poisoned telemetry fields. No changes -- catalog only.

Generated: 2026-05-01

---

## 1. AgentOutcome constructed with empty model/provider (CRITICAL)

`runner_event_to_feedback()` translates runner events into `FeedbackEvent`s for the
feedback facade. When it constructs the `AgentOutcome` for `TaskAttemptCompleted`
events, it fills `model` and `provider` with `String::new()`.

| File | Line | Issue |
|---|---|---|
| `crates/roko-cli/src/runner/event_loop.rs` | 1373-1385 | `model: String::new(), provider: String::new()` in `AgentOutcome` |
| `crates/roko-cli/src/runner/event_loop.rs` | 1367-1372 | Comment acknowledges the problem: "fills model / provider / tokens / cost with empty defaults" |

**Impact chain:**

1. **RoutingObservationSink** calls `router.record_confidence_outcome(&outcome.model, success)`
   with an empty string. `CascadeRouter::record_confidence_outcome` calls
   `model_index_for_slug("")` which returns `None`, so the observation is silently dropped.
   The routing sink records zero observations. The cascade router never progresses past
   Stage 1 (Static) via this path.

2. **EpisodeSink** writes episodes with `backend: ""` and `model: ""`. Any downstream
   consumer (dreams, neuro ingestion) that keys on model/provider gets empty values.

3. **KnowledgeIngestionSink** writes candidates with `model: ""` and `provider: ""`.
   Candidates are persisted but carry no attribution.

The comment says "the routing sink dampens its observation accordingly when the model
slug is empty" -- but inspection of the routing sink shows no dampening logic; the
empty slug simply produces a no-op in the router's `model_index_for_slug` lookup.

**Parallel path works:** The legacy `emit_feedback()` function at line 2489 uses
`state.agent_model` which IS populated from `AgentEvent::AgentStarted` (line 20 in
`agent_events.rs`). The `observe_cascade_router` call at line 2554 correctly guards
on `model.is_empty()` and produces real observations. So the cascade router does
receive data -- but only through the legacy path, not through the new facade sinks.

**Net effect:** The facade-based feedback loop (RoutingObservationSink, EpisodeSink,
KnowledgeIngestionSink) is architecturally wired but operationally inert for routing.
Episodes get written with empty model fields. The system has two parallel feedback
paths (legacy `emit_feedback` + facade `FeedbackFacade`), both active, writing to the
same JSONL files, but only the legacy path carries real model/provider attribution.

---

## 2. KnowledgeIngestionSink writes to wrong filename (CRITICAL)

The `KnowledgeIngestionSink` writes candidates to `knowledge_candidates.jsonl`
(underscore). The neuro admission gate (`roko-neuro/src/admission.rs`) reads from
`knowledge-candidates.jsonl` (hyphen).

| File | Line | Issue |
|---|---|---|
| `crates/roko-cli/src/commands/plan.rs` | 375 | Sink path: `.roko/learn/knowledge_candidates.jsonl` (underscore) |
| `crates/roko-neuro/src/admission.rs` | 26 | `DEFAULT_KNOWLEDGE_CANDIDATES_FILE = "knowledge-candidates.jsonl"` (hyphen) |
| `crates/roko-learn/src/runtime_feedback.rs` | 189 | LearningRuntime also uses `knowledge-seeds.jsonl` (hyphen naming convention) |

The naming mismatch means the neuro admission gate never sees candidates written by
the KnowledgeIngestionSink. The sink writes to a file nothing reads; the admission
gate reads from a file nothing (in this path) writes to. The `LearningRuntime`
separately writes to `knowledge-seeds.jsonl` via the legacy `emit_feedback` path --
that file IS read by `LearningRuntime::read_knowledge_seeds()` and by the snapshot
query system.

**Net effect:** KnowledgeIngestionSink is a pure write-only sink. Its JSONL output
is orphaned.

---

## 3. `.with_ingestor()` never called -- KnowledgeIngestionSink always offline (HIGH)

`KnowledgeIngestionSink` has a builder method `.with_ingestor()` that accepts a
live `KnowledgeIngestor` for in-process ingestion. The wiring in `plan.rs` never
calls it:

| File | Line | Issue |
|---|---|---|
| `crates/roko-cli/src/runtime_feedback/knowledge.rs` | 88 | `pub fn with_ingestor(...)` -- defined, never called outside tests |
| `crates/roko-cli/src/commands/plan.rs` | 396 | `KnowledgeIngestionSink::at(&knowledge_path)` -- no `.with_ingestor()` |

The sink always writes to JSONL and never passes candidates to a live neuro store.
The doc comment says "consumed by an offline reinforcement pass" but no such pass
exists in the codebase.

---

## 4. ConductorObservationSink writes to a file nothing reads (HIGH)

The conductor sink writes gate and retry observations to
`.roko/conductor/observations.jsonl`. No code in `roko-conductor` or `roko-daimon`
reads this file.

| File | Line | Issue |
|---|---|---|
| `crates/roko-cli/src/runtime_feedback/conductor.rs` | 13 | Doc: "the conductor reads JSONL files written by runner/persist.rs" |
| `crates/roko-cli/src/commands/plan.rs` | 379 | Writes to `.roko/conductor/observations.jsonl` |

A grep for `conductor/observations` across the entire crate tree returns only:
- The sink definition
- The plan.rs wiring
- The E2E test

No runtime consumer reads `conductor/observations.jsonl`. The conductor module
(`roko-conductor`) reads from different sources (inline state, not JSONL files).

**Net effect:** Pure write-only sink. Observations are logged but never consumed.

---

## 5. DreamTriggerSink writes triggers no worker drains (HIGH)

The dream sink writes `DreamTrigger` records to `.roko/learn/dream_triggers.jsonl`.
The doc comment says "a separate worker consumes those" but no such worker exists.

| File | Line | Issue |
|---|---|---|
| `crates/roko-cli/src/runtime_feedback/dreams.rs` | 20 | "a separate worker consumes those" |
| `crates/roko-cli/src/commands/plan.rs` | 380 | `DreamTriggerSink::at(&dream_path)` -- no `.with_runner()` |

The `.with_runner()` builder method exists but is never called from production code.
Dream consolidation (`roko-dreams`) has its own trigger mechanism via
`orchestrate.rs` (`DreamRunner::run`), but that is a separate codepath that does not
read the JSONL file produced by this sink.

No code reads `dream_triggers.jsonl` at runtime.

**Net effect:** Pure write-only sink. The dream subsystem runs (when manually
triggered via `roko knowledge dream run`), but the feedback facade's dream trigger
output is orphaned.

---

## 6. ContextualBanditPolicy permanently in Shadow mode (HIGH)

The bandit policy is constructed with `BanditPolicyMode::Shadow` in both plan.rs and
serve_runtime.rs.

| File | Line | Issue |
|---|---|---|
| `crates/roko-cli/src/commands/plan.rs` | 355 | `cfg.mode = BanditPolicyMode::Shadow` |
| `crates/roko-cli/src/serve_runtime.rs` | 542 | Same shadow-mode construction |
| `crates/roko-learn/src/contextual_bandit.rs` | 757-760 | Shadow mode returns `safe_indices[0]` -- always picks first candidate |

In Shadow mode, `select_action()` always returns the first safe action regardless of
learned rewards. The bandit records observations (line 2760 in event_loop.rs calls
`policy.record_reward()`) and persists them to `bandit-decisions.jsonl`, but never
uses them to influence decisions. Shadow mode is the exploration/logging phase before
going live -- but there is no code path to transition to `Candidate` or `Active` mode.

**Net effect:** The bandit observes and logs but never affects model selection.
All bandit telemetry is write-only.

---

## 7. `observe_pipeline()` and `drain_spc_alerts()` never called from runner (MEDIUM)

`AdaptiveThreshold` exposes `observe_pipeline()` for joint cross-rung anomaly
detection and `drain_spc_alerts()` for SPC (Statistical Process Control) alerts.
Neither is called from the runner.

| File | Line | Issue |
|---|---|---|
| `crates/roko-gate/src/adaptive_threshold.rs` | 468 | `pub fn observe_pipeline(...)` |
| `crates/roko-gate/src/adaptive_threshold.rs` | 446 | `pub fn drain_spc_alerts(...)` |
| `crates/roko-cli/src/runner/event_loop.rs` | 2569 | `update_gate_thresholds` calls `thresholds.observe()` (per-rung only) |

The runner calls per-rung `observe(rung, passed)` which updates EMA pass rates --
that works. But the pipeline-level anomaly detection (`observe_pipeline` which checks
cross-rung correlations) is never invoked. SPC alerts accumulate internally but are
never drained or acted upon.

**Net effect:** Adaptive gate thresholds work at the per-rung level but cross-rung
anomaly detection is dead. SPC alerts are never surfaced.

---

## 8. Four orphan `.rs` files not compiled (MEDIUM)

Four files exist in `crates/roko-learn/src/` but are NOT listed as `mod` entries
in `lib.rs`, meaning rustc never compiles them:

| File | Status |
|---|---|
| `crates/roko-learn/src/resonant_patterns.rs` | Not in `lib.rs` -- dead file |
| `crates/roko-learn/src/signal_metabolism.rs` | Not in `lib.rs` -- dead file |
| `crates/roko-learn/src/shapley.rs` | Not in `lib.rs` -- dead file |
| `crates/roko-learn/src/kalman.rs` | Not in `lib.rs` -- dead file |

These files contain well-tested implementations (Lotka-Volterra dynamics, replicator
dynamics, Shapley values, Kalman filters) but are completely inert. They are not
compiled, not tested in CI, and not importable.

---

## 9. Seven learn modules with zero external callers (MEDIUM)

These modules are exported from `roko-learn/src/lib.rs` and compile, but no crate
outside `roko-learn` imports them:

| Module | What it does | External callers |
|---|---|---|
| `adversarial` | HDC adversarial signal detection | 0 |
| `adas` | Autocatalytic optimization (LEARN-08) | 0 |
| `calibration_policy` | Bus-backed predict-publish-correct | 0 |
| `causal` | Causal discovery / DAG extraction | 0 |
| `reinforce_kind` | Typed reinforcement signal categories | 0 |
| `research_pipeline` | Paper -> Claim -> Trial -> Ledger | 0 |
| `regression` | Regression analysis helpers | 0 |

Additionally, these modules have callers but only in narrow contexts:

| Module | External callers | Notes |
|---|---|---|
| `bandit_research` | 0 external callers | Exported but unused |
| `forensic_replay` | Referenced only in `roko-core::forensic` / `roko-gate::forensic` docs, no runtime call | |
| `drift` | 0 external callers | |
| `local_reward` | 0 external callers | |
| `section_outcome` | 0 external callers | |
| `post_gate_reflection` | 0 external callers | |
| `verdict_scorer` | 0 external callers | |

**Net effect:** ~14 learn modules (~4,000+ LOC estimated) compile but are never
reached from any runtime path. They exist as theoretical infrastructure.

---

## 10. Dual feedback paths cause double-writing (MEDIUM)

The runner has two parallel feedback mechanisms:

1. **Legacy path** (`emit_feedback()` at line 2489): directly appends to
   `episodes.jsonl` and `efficiency.jsonl` via `persist::append_jsonl`, then calls
   `observe_cascade_router`, `observe_bandit_policy`, and `update_gate_thresholds`.

2. **Facade path** (`FeedbackFacade` at line 1339): translates `RunnerEvent` to
   `FeedbackEvent` and fans out to 5 sinks (EpisodeSink, RoutingObservationSink,
   KnowledgeIngestionSink, ConductorObservationSink, DreamTriggerSink).

Both fire on the same events. The legacy path writes to `episodes.jsonl`; the
EpisodeSink also writes to `episodes.jsonl`. This produces duplicate episode entries
(one with real model/provider, one with empty strings).

| File | Line | Path |
|---|---|---|
| `crates/roko-cli/src/runner/event_loop.rs` | 2511 | Legacy: `persist::append_jsonl(&paths.episodes_jsonl, &episode)` |
| `crates/roko-cli/src/runner/event_loop.rs` | 1339-1351 | Facade: spawns `facade.on_event()` which includes EpisodeSink |
| `crates/roko-cli/src/runtime_feedback/episodes.rs` | 83 | EpisodeSink: `self.logger.append(&episode)` to same file |

**Net effect:** Episode JSONL contains duplicates. Downstream consumers that count
episodes will overcount. Cost tracking that sums episode costs will double-count.

---

## 11. CascadeRouter does progress -- but only via legacy path (MEDIUM)

The cascade router DOES receive real observations and CAN progress past Stage 1. The
progression is real but comes exclusively from the legacy `observe_cascade_router()`
call (line 2554), which uses `state.agent_model` populated by
`AgentEvent::AgentStarted`. This path calls `router.observe_multi_objective()` which
updates both confidence stats AND the LinUCB bandit.

The stage thresholds are: Static < 50 observations, Confidence 50-200, UCB > 200.

| Stage | Threshold | Data source |
|---|---|---|
| Static -> Confidence | 50 observations | `observe_multi_objective` in legacy path |
| Confidence -> UCB | 200 observations | Same |

The facade's `RoutingObservationSink` calls `record_confidence_outcome("")` which is
a no-op (Finding 1). So the facade path contributes zero observations.

**Net effect:** Router progression works but depends entirely on the legacy path.
If the legacy path is removed during the planned migration to the facade pattern,
the router will permanently stall at Stage 1.

---

## 12. `knowledge-seeds.jsonl` is written and read -- but only in snapshot queries (LOW)

The `LearningRuntime` writes knowledge seeds to `.roko/learn/knowledge-seeds.jsonl`
and exposes `read_knowledge_seeds()`. This IS called from the runtime feedback
snapshot system (`query_runtime_feedback_snapshot`) and the filtered query system.

| File | Line | What |
|---|---|---|
| `crates/roko-learn/src/runtime_feedback.rs` | 1819 | `append_jsonl_record(&self.paths.knowledge_seeds_jsonl, seed)` |
| `crates/roko-learn/src/runtime_feedback.rs` | 1920-1923 | `read_knowledge_seeds()` -- reads the file |
| `crates/roko-learn/src/runtime_feedback.rs` | 3231 | Read in `query_runtime_feedback_snapshot` |
| `crates/roko-learn/src/runtime_feedback.rs` | 3294 | Read in filtered query path |

However, nothing in the runtime (router, dream, conductor, neuro) reads knowledge
seeds to influence future behavior. They are read for dashboard display and snapshot
queries only.

**Net effect:** Not fully write-only (it has readers), but the data does not
close any feedback loop. It is telemetry, not learning input.

---

## 13. DaimonState is loaded and used -- feedback loop works (LOW -- informational)

`DaimonState` is loaded via `DaimonState::load_or_new()` in `orchestrate.rs`
(lines 4317, 4542, 4754) and its strategy space is configured. It is used in the
`dispatch_agent_with` path to modulate dispatch parameters (temperament, operating
frequency, tier selection).

This is one of the few feedback loops that actually closes: affect events from
gate outcomes update the daimon state, which then influences the next dispatch.

---

## 14. AdaptiveThresholds receive real data and influence retries (LOW -- informational)

`GateThresholds` (the runner-side wrapper around `AdaptiveThreshold`) is loaded,
updated per gate verdict (`thresholds.observe(rung, passed)` at line 2570), saved
to disk, and used to influence retry budgets (`gate_thresholds.suggested_max_retries()`
at line 626).

This loop closes correctly for per-rung EMA tracking. Only the pipeline-level
cross-rung detection is dead (Finding 7).

---

## 15. EpisodeLogger output is consumed by multiple systems (LOW -- informational)

Despite the duplicate-writing issue (Finding 10), episodes.jsonl IS consumed by:

- `roko-neuro` context and knowledge lifecycle ingestion
- `roko-dreams` consolidation cycles
- `roko-acp` bridge events
- `roko-serve` routes
- `roko-core` dashboard snapshots
- TUI dashboard

The episode data path works end-to-end. The concern is data quality (empty model
fields from facade path, duplicates from dual writes).

---

## Root Causes

**RC-1: Runner event does not carry dispatch metadata.** The `RunnerEvent::TaskAttemptCompleted`
variant carries only `TaskAttemptRef` and `TaskAttemptOutcome`. It does not carry the
model/provider/tokens/cost that the dispatch layer knew. The facade translator has to
fabricate an `AgentOutcome` with empty fields because the event doesn't give it the
real data. The legacy path avoids this by reading from `RunState` which was populated
by the agent events stream.

**RC-2: Premature facade introduction.** The `FeedbackFacade` was built as a clean
replacement for the ad-hoc legacy feedback helpers. But it was layered on top of the
legacy path rather than replacing it. Now both run, producing duplicates and
inconsistent data quality.

**RC-3: Build-then-wire pattern.** Sinks (conductor, dreams, knowledge) were built
with extensibility points (`.with_runner()`, `.with_ingestor()`) that are never called.
The worker/consumer side of each sink was planned but never implemented.

**RC-4: Naming inconsistency between teams.** `knowledge_candidates.jsonl` vs
`knowledge-candidates.jsonl` is a hyphen/underscore mismatch between the
`runtime_feedback` module and the `roko-neuro::admission` module.

**RC-5: Orphan research code.** Modules like `resonant_patterns`, `signal_metabolism`,
`shapley`, and `kalman` appear to have been written as exploratory research or as part
of spec-driven development (matching a design doc) without a wiring plan.

---

## Fix Directions

**FD-1: Propagate dispatch metadata into RunnerEvent (fixes 1, 10, 11).**
Add `model: String, provider: String, tokens_in: u64, tokens_out: u64, cost_usd: f64,
duration_ms: u64` fields to `RunnerEvent::TaskAttemptCompleted`. The dispatch layer
already has this data; it just needs to pass it through. Once the facade gets real
data, the legacy `emit_feedback()` can be removed, eliminating the dual-write problem.

**FD-2: Fix the filename mismatch (fixes 2).**
Change the `KnowledgeIngestionSink` path in `plan.rs` from `knowledge_candidates.jsonl`
to `knowledge-candidates.jsonl` (matching the neuro admission constant).

**FD-3: Wire consumers for each sink or remove the sink (fixes 3, 4, 5).**
For each write-only sink, either:
- Wire a consumer (e.g., have the conductor read `observations.jsonl`, have the
  dream runner read `dream_triggers.jsonl`), or
- Remove the sink from the facade and document that the subsystem uses a different
  data flow.

**FD-4: Add mode transition for bandit policy (fixes 6).**
Either promote the bandit from Shadow to Candidate after N observations, or remove the
shadow infrastructure and let the cascade router handle all model selection learning.

**FD-5: Wire `observe_pipeline()` into the runner (fixes 7).**
After all gate rungs complete for a task, call `thresholds.observe_pipeline(&pass_rates)`
with the per-rung pass rates, then drain and log SPC alerts.

**FD-6: Delete or re-export orphan files (fixes 8, 9).**
For the 4 non-compiled files: either add `pub mod` entries to `lib.rs` and wire them,
or delete them. For the ~10 compiled-but-uncalled modules: either wire them into the
runtime or feature-gate them behind a `research` feature flag to make the dead-code
boundary explicit.
