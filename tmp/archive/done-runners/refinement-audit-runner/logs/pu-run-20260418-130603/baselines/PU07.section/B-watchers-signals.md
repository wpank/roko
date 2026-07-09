# B — Watcher Ensemble + Cognitive Signals (Docs 01 + 09)

Parity audit of `docs/07-conductor/01-watcher-ensemble.md` (10-watcher
catalog, `Policy` trait, severity ladder, composition / CEP /
streaming-anomaly narrative) and `docs/07-conductor/09-cognitive-signals.md`
(typed interrupt vocabulary: `Pause`, `Resume`, `Reprioritize`,
`InjectContext`, `Escalate`, `Cooldown`, `Explore`, `Shutdown`) against the
conductor implementation in `crates/roko-conductor/` and the
`ConductorDecision` type in `crates/roko-core/`. Both docs carry an
"Implementation: Built" banner; the watcher doc is largely accurate but has
several constant-name drifts and a section-level claim (CEP composition,
streaming anomaly detection) that is design-only. Doc 09 explicitly labels
`CognitiveSignal` as a "planned extension" and that self-report is correct
— nothing under that enum ships.

Generated: 2026-04-16.

---

## B.01 — `Policy` trait location and signature (Doc 01 §"The Policy Trait")

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 01 §"The Policy Trait" (lines 11-29) declares every
watcher implements `pub trait Policy { fn decide(&self, stream: &[Signal], ctx: &Context) -> Vec<Signal>; fn name(&self) -> &str; }`
with `Send + Sync` bounds; `decide()` receives the full signal stream and
returns intervention signals, and the `Context` provides the current tick
position.
**Reality**: `Policy` is defined in `roko-core` (not `roko-conductor`), at
`crates/roko-core/src/traits.rs:166-172` with exact signature
`pub trait Policy: Send + Sync { fn decide(&self, stream: &[Engram], ctx: &Context) -> Vec<Engram>; fn name(&self) -> &str; }`.
The only naming drift vs the doc snippet is `Signal` → `Engram` (the
engram/signal rename is consistent across the whole tree — `Kind` is the
`Engram` discriminator). Every watcher in `crates/roko-conductor/src/watchers/`
imports `roko_core::{Body, Context, Engram, Kind, Policy}` and supplies
both methods. `Context::at(_)` / `Context::now()` at `crates/roko-core/src/context.rs`
provides the tick. Doc 01 §"The Policy Trait" accurately reflects the
production signature modulo the Signal/Engram alias.

---

## B.02 — `Severity` enum with `Info | Warning | Critical` (Doc 01 — implicit "severity ladder")

**Status**: DONE
**Severity**: —
**Doc claim**: The per-watcher sections in Doc 01 assign severities
`Warning` or `Critical`; the §"Watcher Priority and Conflict Resolution"
section (lines 656-796) says "WorstSeverityPolicy takes the maximum
severity across all fired watchers" and implies a three-level ladder
`Info | Warning | Critical`.
**Reality**: `Severity` is declared at `crates/roko-conductor/src/interventions.rs:22-31`
as `#[derive(PartialOrd, Ord)] enum Severity { Info = 0, Warning = 1, Critical = 2 }`,
with `Severity::to_decision()` at `:36-44` mapping `Info → Continue`,
`Warning → Restart`, `Critical → Fail`. Ordering is asserted by unit test
`severity_ordering` at `:150-154` (`Info < Warning < Critical`). The
`WorstSeverityPolicy` picks `outputs.iter().max_by_key(|o| o.severity)` at
`:113`, so the "worst severity wins" doc claim holds literally. No watcher
fires `Info` today — the conductor's `collect_watcher_outputs()` at
`conductor.rs:210-214` tags unknown severity strings as `Info`, but every
shipping watcher emits `severity=warning` or `severity=critical`.

---

## B.03 — `WatcherOutput` structured record (Doc 01 §"Policy" — implicit)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 01 describes watcher findings as carrying watcher name,
severity tag, human-readable description, and optional metric value (see
per-watcher `.tag("watcher", …).tag("severity", …)` examples used
throughout the catalog).
**Reality**: `WatcherOutput { watcher, severity, description, metric }` at
`interventions.rs:50-60` is the canonical structured form. Constructor
`WatcherOutput::new(watcher, severity, description)` at `:65-76`,
chainable `.with_metric(v)` at `:80-83`, `to_decision()` adaptor at
`:86-89`. `outputs_to_signals()` helper at `:124-144` serializes these
into `Kind::Custom("conductor:alert:<watcher>")` engrams while filtering
`Severity::Info` entries. Full serde roundtrip verified by the
`watcher_output_serde_roundtrip` test at `:227-232`. The doc never names
`WatcherOutput` as a type, but the implementation matches the documented
shape exactly.

---

## B.04 — `InterventionPolicy` trait + `WorstSeverityPolicy` (Doc 01 §"Current approach: WorstSeverityPolicy")

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 01 §"Current approach: WorstSeverityPolicy" (lines
663-673) describes `WorstSeverityPolicy` taking the max severity across
fired watchers; the default is "simple, conservative, and effective — it
never under-reacts."
**Reality**: `pub trait InterventionPolicy { fn evaluate(&self, outputs: &[WatcherOutput], ctx: &Context) -> ConductorDecision; fn name(&self) -> &str; }`
at `interventions.rs:99-105`. Default impl `WorstSeverityPolicy` at
`:108-121` implements `evaluate` as
`outputs.iter().max_by_key(|o| o.severity).map_or_else(ConductorDecision::cont, WatcherOutput::to_decision)`.
`Conductor::new()` wires it via `policy: Box::new(WorstSeverityPolicy)` at
`conductor.rs:98`, with `with_policy()` at `:125-128` offering swap-in for
alternate policies. Unit tests `worst_severity_policy_critical_wins`
(`:200-209`), `worst_severity_policy_picks_worst` (`:188-198`), and
`worst_severity_policy_empty_is_continue` (`:181-186`) cover the three
observable branches.

---

## B.05 — Ghost Turn watcher (Doc 01 §1)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 01 §1 (lines 35-74): module `watchers/ghost_turn.rs`,
constant `MAX_GHOST_TURNS = 3`, watcher name `ghost-turn`, scans
`AgentOutput` signals for ghost pattern, 3 consecutive ghost turns →
Warning (restart with fresh context).
**Reality**: File `crates/roko-conductor/src/watchers/ghost_turn.rs`,
`pub const MAX_GHOST_TURNS: usize = 3` at `:11`, `WATCHER_NAME: &str = "ghost-turn"`
at `:14`. The detection logic differs from the doc description in one
important way: the watcher does NOT scan raw `Kind::AgentOutput` signals;
it scans `Kind::Custom("conductor.ghost_turn")` signals emitted by the CLI
(`TURN_SIGNAL_KIND` at `:17`) whose body deserializes into `GhostTurnEvent`
(`:19-32`, 11 fields including `output_meaningful`, `net_new_changes`,
`wasted_cost`). `extract_ghost_turn_event` at `:65-81` filters out events
where `output_meaningful || net_new_changes != 0`, and the decide loop at
`:84-140` walks the stream in reverse, counts consecutive ghost events,
and fires when `consecutive >= self.max_ghost_turns` with tag
`severity=warning` at `:124`. The doc's "AgentOutput" narrative is a
simplification — the real watcher relies on an upstream CLI enrichment
step. Tests `at_threshold_fires`, `below_threshold_no_fire`,
`meaningful_output_breaks_consecutive_chain` at `:219-273` confirm the
3-consecutive-turn behavior.
**Fix sketch**: Update Doc 01 §1 "How it works" paragraph to say the
watcher consumes `conductor.ghost_turn` custom-kind signals produced
upstream by the CLI (with `output_meaningful` and `net_new_changes`
fields), not raw `AgentOutput` engrams.

---

## B.06 — Compile Fail Repeat watcher (Doc 01 §2) — constant name drift

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 01 §2 (lines 78-112): module `watchers/compile_fail_repeat.rs`,
constant `MAX_COMPILE_FAIL_REPEAT = 3`, examines `GateVerdict` signals
for compile-related gate results, extracts error fingerprints from the
gate verdict body, 3 identical compile errors → Warning.
**Reality**: File `crates/roko-conductor/src/watchers/compile_fail_repeat.rs`;
the constant is named `MAX_IDENTICAL_COMPILE_FAILURES: usize = 3` at
`:9`, NOT `MAX_COMPILE_FAIL_REPEAT`. Watcher name is `compile-fail-repeat`
at `:12`. The detection scope also differs: the watcher scans
`Kind::CompileDiagnostic` signals, not `Kind::GateVerdict`
(`compile_fail_repeat.rs:68` — `stream.iter().filter(|s| s.kind == Kind::CompileDiagnostic)`).
`diagnostic_key()` at `:42-61` extracts the fingerprint — `Body::Text` is
trimmed directly, `Body::Json` pulls the `"message"` field or stringifies
the full value. Firing at `:85-97` emits `severity=warning` with
description `"{max_failures} consecutive identical compile failures: {truncated}"`.
Tests `identical_errors_fires`, `different_errors_no_fire`,
`non_consecutive_at_end_no_fire` at `:146-179` pass.
**Fix sketch**: Doc 01 §2 should say `MAX_IDENTICAL_COMPILE_FAILURES`
(not `MAX_COMPILE_FAIL_REPEAT`) and the watcher consumes
`Kind::CompileDiagnostic` engrams rather than `GateVerdict`. The
Diagnosis-Engine tie-in claim at lines 106-112 is narrative, not wired
through this watcher — the watcher emits only a generic warning.

---

## B.07 — Cost Overrun watcher (Doc 01 §3) — constant name drift

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 01 §3 (lines 115-157): module `watchers/cost_overrun.rs`,
constant `DEFAULT_BUDGET_USD = 10.0`, watcher name `cost-overrun`, scans
for `Metric` signals tagged `name=plan_cost`, compares against plan
budget, fires at threshold.
**Reality**: File `crates/roko-conductor/src/watchers/cost_overrun.rs`;
the constant is named `DEFAULT_BUDGET: f64 = 10.0` at `:22`, NOT
`DEFAULT_BUDGET_USD`. Watcher name `cost-overrun` matches at `:10`.
`PLAN_COST_METRIC = "plan_cost"` at `:15` and `PLAN_BUDGET_METRIC = "plan_budget"`
at `:17` are the canonical metric names. `latest_metric()` at `:51-58`
pulls the most-recent `Kind::Metric` engram matching the name. Decide
path at `:60-87`: when `cost > budget`, emits intervention with
`severity=warning` (contrary to Doc 01 line 133 "escalates based on
overage" — there is no escalation logic, severity is always Warning).
Budget falls back to `default_budget` when no `plan_budget` metric
appears. Unit tests `above_budget_fires`, `below_budget_no_fire`,
`uses_most_recent_cost` at `:134-167` confirm behavior.
**Fix sketch**: Doc 01 §3 should say `DEFAULT_BUDGET` (not
`DEFAULT_BUDGET_USD`). Drop the "escalates based on overage" hedge on
line 133 — severity is flat Warning regardless of overage amount.

---

## B.08 — Iteration Loop watcher (Doc 01 §4) — only Critical watcher

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 01 §4 (lines 160-201): module `watchers/iteration_loop.rs`,
constant `MAX_ITERATION_LOOP = 3`, tracks `GateVerdict` signals per plan,
3 consecutive gate failures → Critical (plan failure). This is explicitly
the "only watcher that defaults to Critical severity" (line 180).
**Reality**: File `crates/roko-conductor/src/watchers/iteration_loop.rs`;
the constant is named `MAX_IMPLEMENTER_ATTEMPTS: usize = 3` at `:9`, NOT
`MAX_ITERATION_LOOP`. Watcher name `iteration-loop` at `:12`. The scan
target is `Kind::PlanPhase` (not `Kind::GateVerdict`) with
`plan_event == "GateFailed"` at `:94-96`. Counter resets on `"GatePassed"`,
`"ImplementationDone"`, `"ReviewApproved"`, `"DocRevisionDone"`,
`"MergeSucceeded"`, `"VerifyPassed"` at `:111-117`. Critical severity is
tagged at `:104` (`tag("severity", "critical")`) — confirmed at
`iteration_loop.rs:172` test assertion. The `Conductor` grep confirms
this is indeed the only watcher emitting critical — `Grep '"critical"' crates/roko-conductor/src/watchers/`
returns only `iteration_loop.rs:104`. Tests at `:145-207` pass.
**Fix sketch**: Doc 01 §4 should say `MAX_IMPLEMENTER_ATTEMPTS` (not
`MAX_ITERATION_LOOP`) and the watcher consumes `PlanPhase` engrams (with
`event=GateFailed`), not `GateVerdict` directly.

---

## B.09 — Review Loop watcher (Doc 01 §5)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 01 §5 (lines 204-249): module `watchers/review_loop.rs`,
constant `MAX_REVIEW_CYCLES = 3`, scans `PlanPhase` signals, counts
`ReviewRejected` for the latest plan, resets on `ReviewApproved`,
`DocRevisionDone`, or `MergeSucceeded`, Warning at threshold. Includes a
code snippet at lines 218-232.
**Reality**: File `crates/roko-conductor/src/watchers/review_loop.rs`,
`pub const MAX_REVIEW_CYCLES: usize = 3` at `:10`, `WATCHER_NAME: &str = "review-loop"`
at `:13`. `latest_plan_id()` at `:58-60` finds the most recent plan ID.
The decide loop at `:78-121` walks signals matching that plan, matching
`plan_event()` against `"ReviewRejected"` → increments counter (`:93-109`,
fires at threshold with `severity=warning`), `"ReviewApproved" | "DocRevisionDone" | "MergeSucceeded"` → resets to 0
at `:111-113`. The doc-snippet logic at lines 220-231 matches exactly
(match arms verbatim). Tests `repeated_reviews_fires`,
`approved_review_resets_loop`, `interleaved_reviews_still_count` at
`:169-227` verify all three paths.

---

## B.10 — Spec Drift watcher (Doc 01 §6)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 01 §6 (lines 253-299): module `watchers/spec_drift.rs`,
constant `MAX_SPEC_DRIFT_RATIO = 0.25`, examines `Metric` signals with
`name=spec_drift`, body carries `SpecDriftEvent` with
`write_files`/`changed_files`/`unexpected_files`/`drift_ratio`, fires
Warning above 25%. Dual-format support (structured JSON body or
tag-only). `path_is_allowed()` supports prefix matches.
**Reality**: File `crates/roko-conductor/src/watchers/spec_drift.rs`,
`pub const MAX_SPEC_DRIFT_RATIO: f64 = 0.25` at `:12`, watcher name
`spec-drift` at `:15`. `SpecDriftEvent` struct at `:24-38` has exactly
the fields the doc describes (all with `#[serde(default)]`).
`path_is_allowed()` at `:40-46` matches exact paths and supports both
`/` and `\` directory prefixes. `drift_ratio()` at `:61-72` falls back
to computed ratio when the explicit field is absent — matches the doc
snippet at lines 278-283 byte-for-byte. Decide path at `:101-157` fires
only when `drift > self.max_drift` (strictly greater), with
`severity=warning`. Dual-format: JSON body first, then
`signal.tag(METRIC_VALUE_TAG)` fallback at `:118`. Unit tests at
`:184-262` cover threshold boundary (`at_threshold_no_fire` at 0.25,
`above_threshold_fires` at 0.30), detailed JSON payload, and the
most-recent-wins property.

---

## B.11 — Stuck Pattern watcher (Doc 01 §7) — constant name drift

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 01 §7 (lines 302-322): module `watchers/stuck_pattern.rs`,
constant `MAX_STUCK_REPEATS = 4`, tracks recent agent actions (tool
calls, file edits), computes similarity between consecutive turns, 4+
identical → Warning.
**Reality**: File `crates/roko-conductor/src/watchers/stuck_pattern.rs`;
the constant is named `MAX_IDENTICAL_ACTIONS: usize = 4` at `:10`, NOT
`MAX_STUCK_REPEATS`. Watcher name `stuck-pattern` at `:13`. The detection
target `ACTION_KINDS` is `&[Kind::AgentOutput, Kind::AgentMessage]` at
`:16` — narrower than Doc 01's "tool calls, file edits" claim (the
watcher does not scan `ToolInvocation`). `body_fingerprint()` at
`:51-72` normalizes `Body::Text`, `Body::Json`, `Body::Bytes` into a
comparable string; `Body::Empty` returns `None`. The decide loop at
`:74-122` walks backwards through actions, counts consecutive identical
fingerprints, breaks on first mismatch, fires when
`consecutive >= self.max_actions` with `severity=warning`. Tests
`at_threshold_fires`, `different_action_kinds_both_counted`,
`non_action_signals_skipped_in_chain` at `:186-223` confirm.
**Fix sketch**: Doc 01 §7 should say `MAX_IDENTICAL_ACTIONS` (not
`MAX_STUCK_REPEATS`) and that action kinds are limited to `AgentOutput`
and `AgentMessage` — tool invocations and file edits are not scanned
directly by this watcher.

---

## B.12 — Test Failure Budget watcher (Doc 01 §8)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 01 §8 (lines 326-368): module `watchers/test_failure_budget.rs`,
constant `MIN_FAILURE_INCREASE = 1`, scans `GateVerdict` signals for
structured test counts, records first-seen baseline per plan, fires
Warning when latest failures exceed baseline. Per-plan independent
baselines. Custom threshold via constructor.
**Reality**: File `crates/roko-conductor/src/watchers/test_failure_budget.rs`,
`pub const MIN_FAILURE_INCREASE: u32 = 1` at `:13`, watcher name
`test-failure-budget` at `:16`. JSON shape: body has `test_count: {passed, failed, ignored, total}`,
`plan_id`, `gate`, `failed` fields — pulled from
`crates/roko-core` schema. `baselines.entry(plan_id).or_insert(failed)`
at `:84` captures first-seen, `latest.insert(plan_id, failed)` at `:85`
tracks current. Firing at `:99-111` emits `severity=warning` with tags
`baseline_failures`, `current_failures`, `failure_delta`. The doc's
code snippet (lines 341-347) matches the implementation exactly. Tests
`increased_failure_count_fires`, `multiple_plans_independent`,
`custom_threshold_requires_larger_increase` at `:161-200` verify
per-plan independence and threshold customization.

---

## B.13 — Time Overrun watcher (Doc 01 §9)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 01 §9 (lines 371-408): module `watchers/time_overrun.rs`,
constant `ALERT_THRESHOLD = 0.80`, examines `Custom("conductor.agent_output")`
signals for `TaskTimingEvent { duration_ms, timeout_secs }`, fires
Warning when ratio exceeds 80%. Integer arithmetic
`duration_ms * 5 > timeout_ms * 4` avoids division.
**Reality**: File `crates/roko-conductor/src/watchers/time_overrun.rs`,
`pub const ALERT_THRESHOLD: f64 = 0.80` at `:16`, `TASK_OUTPUT_KIND = "conductor.agent_output"`
at `:13`, watcher name `time-overrun` at `:10`. `TaskTimingEvent` at
`:22-28` has `plan_id, task, duration_ms, timeout_secs`. The integer
arithmetic at `:50-57` is `duration_ms.saturating_mul(5) > timeout_ms.saturating_mul(4)`,
matching the doc snippet at lines 386-391 exactly. Decide path at
`:59-100` finds the most recent timing engram in reverse, computes
threshold, emits intervention with `severity=warning` and tags
`duration_ms`, `timeout_secs`, `threshold=0.8`, `ratio`. Zero-timeout
guard at `:51-53`. Tests `above_threshold_fires` (8_001ms of 10s
timeout), `at_threshold_no_fire` (8_000ms = exactly 80%),
`zero_timeout_no_fire` at `:124-169` confirm.

---

## B.14 — Context Window Pressure watcher (Doc 01 §10)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 01 §10 (lines 411-457): module
`watchers/context_window_pressure.rs`, constant `MAX_CONTEXT_USAGE_RATIO = 0.80`,
examines `TokenUsage` signals. Dual format: `AgentEfficiencyEvent` body
(→ lookup model context window) OR tag-based (`tokens_used`, `tokens_total`,
`model`). Model table: `*opus* → 1,000,000`, `*haiku*|*sonnet* → 200,000`,
unknown → no fire. Fires at 80% → Warning (compaction trigger).
**Reality**: File `crates/roko-conductor/src/watchers/context_window_pressure.rs`,
`pub const MAX_CONTEXT_USAGE_RATIO: f64 = 0.80` at `:10`, watcher name
`context-window-pressure` at `:13`, constants
`OPUS_CONTEXT_WINDOW_TOKENS = 1_000_000` and
`SMALL_CONTEXT_WINDOW_TOKENS = 200_000` at `:22-23`. `extract_usage()`
at `:93-114` prefers `AgentEfficiencyEvent` body deserialization (falls
back to `total_prompt_tokens` against `context_window_tokens(model)`),
then tag-based `tokens_used` + `tokens_total`, then tag-based
`tokens_used` + `model` lookup. `context_window_tokens()` at `:116-125`
checks substrings `"opus"`, `"haiku"`, `"sonnet"` on lowercase slug.
Firing at `:71-86` emits `severity=warning` when `ratio > self.max_ratio`
(strictly greater). `AgentEfficiencyEvent` is imported from
`roko_learn::efficiency::AgentEfficiencyEvent` at `:7` — crosses into the
`roko-learn` crate for the rich event shape. Tests at `:173-232` cover
all three formats and the at-threshold non-fire at exactly 80%.

---

## B.15 — `Conductor::new()` assembles 10 watchers (Doc 01 §"Watcher Catalog")

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 01 lists exactly 10 watchers (§§1-10) and the §"Adding
a New Watcher" box (lines 486-489) instructs "Update the `watcher_count()`
test (currently asserts 10)".
**Reality**: `Conductor::new()` at `crates/roko-conductor/src/conductor.rs:82-102`
boxes exactly 10 policies in the declared order: `GhostTurnWatcher`,
`ReviewLoopWatcher`, `IterationLoopWatcher`, `TestFailureBudgetWatcher`,
`CompileFailRepeatWatcher`, `ContextWindowPressureWatcher`,
`SpecDriftWatcher`, `CostOverrunWatcher`, `TimeOverrunWatcher`,
`StuckPatternWatcher`. Unit test `watcher_count` at `:491-495` asserts
`c.watchers.len() == 10`. `Conductor::with_watchers()` at `:114-122`
supports custom sets. `Conductor::check_all(&stream)` at `:108-111`
dispatches `self.decide(stream, &Context::now())` for the periodic-check
ergonomic mentioned in Doc 01's implicit integration narrative.
`crates/roko-conductor/src/watchers/mod.rs:8-28` re-exports all 10
watcher structs from their modules.

---

## B.16 — `Conductor::evaluate()` decision pipeline (Doc 01 §"The Policy Trait" — implicit)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 01 describes the Conductor running all watchers,
collecting their outputs, and merging via the `InterventionPolicy`
(§"Current approach: WorstSeverityPolicy"). The §"Watcher Independence"
paragraph (lines 461-477) claims watchers have no shared state, no
ordering dependency, no cross-watcher interaction.
**Reality**: `Conductor::evaluate(stream, ctx) -> ConductorDecision` at
`conductor.rs:156-187` is the single-decision path: it (1) checks the
circuit breaker at `:158-166`, returning
`ConductorDecision::fail("circuit-breaker", FailureKind::MaxIterations)`
if tripped; (2) calls `collect_watcher_outputs(&self.watchers, stream, ctx)`
at `:169` which iterates each watcher sequentially, reads `severity` and
`watcher` tags off each emitted engram, and pushes `WatcherOutput`
records (`:201-224`); (3) updates routing bias via `update_routing_bias`
at `:170`; (4) applies `self.policy.evaluate()` at `:173`; (5) records
failures back into the circuit breaker at `:176-184`. A parallel
`impl Policy for Conductor` at `:226-255` provides a `decide()` that
emits signals (not decisions) for substrate writes. Watchers are
invoked in definition order but never share data — the sequential loop
at `:207-222` iterates them independently, each receiving the full
`stream`. Tests `empty_stream_continues`, `ghost_turns_trigger_restart`,
`circuit_breaker_aborts_tripped_plan`, `multiple_watchers_worst_wins`
at `:396-489` validate the decision flow.

---

## B.17 — CEP composite pattern matching (Doc 01 §"Watcher Composition") is design-only

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 01 §"Watcher Composition — Complex Pattern Detection"
(lines 514-652) describes NFA-based pattern matching with
`CompositePattern { stages, within, contiguity }`, `PatternStage`,
`Contiguity { Strict, Relaxed, NonDeterministic }`,
`Quantifier { Exactly, AtLeast, Between }` — inspired by Flink CEP /
Esper EPL. A full code snippet (lines 562-588) shows the proposed types.
Separately, §"Multi-watcher correlation" (lines 604-652) declares
`WatcherFamily` with three families (`resource`, `behavioral`,
`coordination`) and a `WATCHER_FAMILIES: &[WatcherFamily]` constant.
**Reality**: `Grep 'CompositePattern|PatternStage|Contiguity|WatcherFamily|WATCHER_FAMILIES'`
across `crates/` returns **zero matches**. Neither `Contiguity` nor
`Quantifier` exists as a type anywhere in the tree. The
`WATCHER_FAMILIES` constant is grep-positive only inside the doc
itself (`docs/07-conductor/01-watcher-ensemble.md:634`). There is no
CEP-style pattern composition layer. The only form of cross-watcher
integration that exists is the `derive_routing_bias()` helper at
`conductor.rs:272-315` which groups watcher names into a
hard-coded resource set (`cost-overrun | context-window-pressure | time-overrun`,
`:278-280`) and a behavioral/coordination set (`ghost-turn | review-loop | iteration-loop | test-failure-budget | compile-fail-repeat | stuck-pattern | spec-drift`,
`:290-298`) for routing bias only — not for intervention decisions.
The three families described in doc lines 610-625 partially map onto the
routing bias grouping, but `WatcherFamily` as a named struct is
entirely design.
**Fix sketch**: Tag Doc 01 §"Watcher Composition" as "Design — not
implemented" and note that `derive_routing_bias()` in `conductor.rs`
is the only shipping cross-watcher correlation (for model routing, not
for decisions). Alternatively, move the §"Watcher Composition" and
§"Watcher Priority and Conflict Resolution" → "Bayesian fusion",
"Dempster-Shafer", "Weighted voting", "Temporal hysteresis" blocks
into a separate "future work" doc so the `01-watcher-ensemble.md`
banner "Implementation: Built" applies only to what ships.

---

## B.18 — Streaming anomaly detection (Doc 01 §"Streaming Anomaly Detection Integration") is design-only

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 01 §"Streaming Anomaly Detection Integration" (lines
800-972) proposes Online Isolation Forest, CUSUM change-point
detection, and TraceAegis-style behavioral rules, with Rust type
sketches at lines 835-865 (`OnlineIsolationForest`, `IsolationTree`,
`IsolationNode`) and lines 896-924 (`CusumDetector` with `update()`).
References: Liu et al. 2008, ICML 2024, Page 1954, TraceAegis
arXiv 2510.11203.
**Reality**: `Grep 'OnlineIsolationForest|IsolationTree|IsolationNode|CusumDetector|TraceAegis'`
of `crates/` returns **zero matches**. No statistical anomaly
detection layer exists in `roko-conductor` or anywhere else in the
tree. The watcher ensemble is entirely threshold-based on explicit
signal kinds — no Isolation Forest, no EWMA, no CUSUM, no behavioral
rule engine. `crates/roko-conductor/src/stuck_detection.rs` (referenced
by grep for `ShutdownRequest|Shutdown`) implements a separate
stagnation-detection primitive but does not instantiate any of the
streaming-anomaly types doc 01 proposes.
**Fix sketch**: Same as B.17 — label this entire section as
"Design / future work". The doc's "Implementation: Built" banner
should apply only to the 10-watcher catalog and `WorstSeverityPolicy`.

---

## B.19 — `CognitiveSignal` enum (Doc 09 §"Definition") is design-only

**Status**: NOT DONE
**Severity**: MEDIUM
**Doc claim**: Doc 09 §"Definition" (lines 12-31) proposes
`pub enum CognitiveSignal { Pause, Resume, Reprioritize(TaskId), InjectContext(Engram), Escalate, Cooldown, Explore, Shutdown }`.
Doc 09 §"Implementation Status" (lines 247-271) is **honest** about
this: "Cognitive Signals are defined in the refactoring PRD (§XII.2,
09-innovations.md) but not yet implemented as a formal type in the
codebase" (line 247).
**Reality**: `Grep 'CognitiveSignal|cognitive_signal|cognitive\.pause|cognitive\.escalate'`
of `crates/` returns **zero matches**. `Grep 'Reprioritize|InjectContext|Cooldown|Explore'`
returns matches only in unrelated contexts (`Explore` appears in
`roko-cli/src/tui/input.rs` as a UI command, not as a cognitive
signal; `Pause` / `Shutdown` are TUI or shutdown-sequence keywords).
The doc's self-report is correct: the enum is unimplemented. Doc 09's
honest "planned extension" framing + the "Path to implementation"
checklist at lines 265-272 (define `CognitiveSignal` in `roko-core`,
extend `ConductorDecision`, teach orchestrator, wire watchers, add
learning integration) accurately describes work that has not started.
The file still carries the "Implementation: Built" banner at
`docs/07-conductor/09-cognitive-signals.md:8`, which is misleading
given the enum is unimplemented.
**Fix sketch**: Change the "Implementation: Built" banner at
`09-cognitive-signals.md:8` to "Design — not implemented" or
"Planned extension". The page's self-report at lines 247-271 is
correct, but the top-of-file banner contradicts it.

---

## B.20 — `ConductorDecision` 3-state is the shipping replacement for cognitive signals (Doc 09 §"Implementation Status")

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 09 §"Implementation Status" (lines 250-263) declares
the Conductor currently expresses decisions through
`ConductorDecision (Continue / Restart / Fail)`, which covers a subset
of cognitive signal semantics. The mapping table at lines 256-260
says: `Continue → (no signal, healthy)`, `Restart → Escalate or InjectContext + Resume`,
`Fail → Shutdown (for the specific plan)`.
**Reality**: Confirmed. `ConductorDecision` at
`crates/roko-core/src/conductor.rs:22-42` is `#[non_exhaustive] enum ConductorDecision { Continue, Restart { watcher, reason }, Fail { watcher, reason: FailureKind } }`.
The doc comment at `conductor.rs:5-8` explicitly says "simplified from
Mori's `InterventionTier { Nudge, Restart, Abort }`" and "§11.2 of the
parity checklist: **no nudges**". `ConductorDecision::cont/restart/fail`
constructors at `:44-67`, `is_terminal/is_continue/label` helpers at
`:71-89`. Serde roundtrip tests at `:127-149` confirm JSON-stable shape
via `#[serde(rename_all = "snake_case")]`. The Conductor runtime at
`crates/roko-conductor/src/conductor.rs:156-187` emits this enum; the
cognitive-signal vocabulary (Pause / Cooldown / Explore / Reprioritize
/ InjectContext) maps onto Restart + auxiliary tags in today's
implementation — no separate type. The doc's mapping table is an
honest description of the current collapse.

---

## B.21 — `Kind` enum variants consumed by watchers (Doc 09 §"Engram" — implicit)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 09 §"InjectContext(Engram)" (lines 89-110) defines an
Engram as "a unit of persistent context" — a typed piece of information.
Doc 01's per-watcher narrative references `AgentOutput`, `GateVerdict`,
`PlanPhase`, `Metric`, `TokenUsage`, `CompileDiagnostic`, and
`Custom("conductor.agent_output")` as the consumed kinds.
**Reality**: `Kind` enum at `crates/roko-core/src/kind.rs:25-106` has 28
named variants plus `Custom(String)`. Mapping each shipping watcher to
the `Kind` variant it scans: `ghost_turn.rs:62` — `Kind::Custom("conductor.ghost_turn")`;
`review_loop.rs:42,63,92` — `Kind::PlanPhase`; `iteration_loop.rs:41,62,90` —
`Kind::PlanPhase`; `test_failure_budget.rs:62` — `Kind::GateVerdict`;
`compile_fail_repeat.rs:68` — `Kind::CompileDiagnostic`;
`context_window_pressure.rs:55` — `Kind::TokenUsage`;
`spec_drift.rs:107` — `Kind::Metric` (filtered by tag `name=spec_drift`);
`cost_overrun.rs:55` — `Kind::Metric` (filtered by tag `name=plan_cost`
or `name=plan_budget`); `time_overrun.rs:39` — `Kind::Custom("conductor.agent_output")`;
`stuck_pattern.rs:16` — `Kind::AgentOutput` + `Kind::AgentMessage`.
Doc 09's broader "typed interrupts" list (Pause, Resume, etc.) does not
correspond to any of the `Kind` variants — those remain cognitive
signals-as-design. Every shipping watcher reads from a real `Kind`
variant; none emit a `CognitiveSignal`-tagged custom kind.

---

## B.22 — `Kind::Pheromone` / `Kind::Insight` / `Kind::Prediction` are grep-positive but unused by watchers

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 09's signal vocabulary hints at richer typed
interrupts ("pause execution", "reprioritize this task", "inject this
context"). Doc 01 implicitly suggests watchers could consume a broad
range of engram kinds, though it names only a subset.
**Reality**: `Kind` includes several chain-participation variants —
`Insight`, `Pheromone`, `Bounty`, `Transaction`, `Service`, `Prediction`
at `crates/roko-core/src/kind.rs:89-100` — as well as `RouterChoice`,
`RouterFeedback` (`:66-68`), `Episode`, `PlaybookRule`, `Skill`
(`:71-77`), `ExperimentResult`, `ToolInvocation`, `ToolHealthDegraded`
(`:80-87`). `Grep 'Kind::Pheromone|Kind::Insight|Kind::Prediction|Kind::RouterFeedback|Kind::Episode|Kind::ToolInvocation|Kind::ToolHealthDegraded'`
returns **zero hits** in `crates/roko-conductor/src/watchers/`. None of
the 10 watchers observes those kinds. This is not drift per se — the
doc never claims watchers should consume those kinds — but it is worth
recording that the conductor's surface is narrower than the `Kind`
enum it could theoretically watch.
**Fix sketch**: Consider a follow-up watcher that consumes
`Kind::ToolHealthDegraded` for tool-level anomalies — today the
`stuck-pattern` + `compile-fail-repeat` combo covers only output
behavior, not tool-call health.

---

## B.23 — `outputs_to_signals()` bridge + `conductor.decision` engram (Doc 01 §"Adding a New Watcher" — implicit)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 01 §"Adding a New Watcher" (lines 481-492) says "The
Conductor's `evaluate()` method automatically picks up any watcher in
the `watchers` vector. No other code needs to change." Implicit in this
is a stable emission contract.
**Reality**: `outputs_to_signals()` at `interventions.rs:124-144`
converts `&[WatcherOutput]` → `Vec<Engram>` for substrate writes. Each
`WatcherOutput` becomes a `Kind::Custom("conductor:alert:{watcher}")`
engram (`:133`) with serialized body and tags `watcher`, `severity`
(`:138-139`). `Severity::Info` outputs are filtered out at `:128`.
`Conductor::decide()` at `conductor.rs:227-249` additionally emits a
`Kind::Custom("conductor.decision")` engram when the decision is
non-continue, with body = serialized `ConductorDecision` and tag
`decision={continue|restart|fail}` (`:240-244`). So the full emitted
surface per tick is: zero or more `conductor:alert:<watcher>` engrams +
one `conductor.decision` engram on anomaly. Test
`conductor_policy_emits_on_anomaly` at `:439-449` confirms both engram
kinds appear when ghost turns trigger the pipeline.

---

## B.24 — Circuit breaker integration (Doc 01 silent; code-level coupling)

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 01 never mentions a circuit breaker — the §"Watcher
Ensemble" page speaks only of watchers and intervention policies. Doc
09 §"Implementation Status" says "Fail → Shutdown (for the specific
plan)" (line 259) without naming the mechanism.
**Reality**: `CircuitBreaker` is a first-class field on `Conductor` at
`conductor.rs:59`, imported from `crate::circuit_breaker::CircuitBreaker`
at `:9`. `evaluate()` checks `self.circuit_breaker.is_tripped(&plan_id)`
at `:159` **before** any watcher runs — a tripped plan short-circuits
directly to
`ConductorDecision::fail("circuit-breaker", FailureKind::MaxIterations)`
at `:161-164`. After watchers run, failures are recorded back into the
breaker at `:176-184` via `record_failure(plan_id, reason, ctx.now_ms)`.
Test `circuit_breaker_aborts_tripped_plan` at `:417-428` demonstrates
the short-circuit path. `Conductor::with_circuit_breaker()` at
`:132-136` supports injecting a custom breaker.
**Fix sketch**: Add a short §"Circuit Breaker" section to Doc 01 noting
the pre-watcher short-circuit and that repeated failures accumulate
into a breaker-trip fail — currently invisible from the doc but
load-bearing in evaluation.

---

## B.25 — `RoutingBias` emission (Doc 01 silent; code-level addition)

**Status**: DONE (undocumented feature beyond Doc 01's scope)
**Severity**: LOW
**Doc claim**: Doc 01 does not describe any routing bias output from
the Conductor. The watcher narrative stops at "emit intervention
signal" and the conflict-resolution section at "emit single
ConductorDecision".
**Reality**: `RoutingBias { deprioritize, prefer_cheaper, reason }` at
`conductor.rs:27-35` is a per-tick snapshot the Conductor maintains in
`Mutex<RoutingBias>` at `:61`. `update_routing_bias()` at `:258-269`
is called from both `evaluate()` (`:170`) and the `Policy::decide()`
impl (`:230`). `derive_routing_bias()` at `:272-315` groups the 10
watchers into load-pressure (cost / context / time) which flips
`prefer_cheaper`, and behavior/coordination failures which push the
latest model slug into `deprioritize`. `Conductor::routing_bias()`
getter at `:145-148` exposes the snapshot. Tests
`routing_bias_tracks_recent_failures_and_load_pressure` at `:506-538`
and `routing_bias_deprioritizes_recent_model_failures` at `:540-554`
verify. This is an entirely undocumented shipping feature — Doc 01's
"Implementation: Built" claim is technically incomplete (this is more
than what the doc describes, not less).
**Fix sketch**: Add a §"Routing Bias" or "Side-Effects" subsection to
Doc 01 describing the `RoutingBias` snapshot and its consumers
(cascade router).

---

## Section Summary

| Status | Count |
|--------|-------|
| DONE | 16 (B.01 Policy trait, B.02 Severity enum, B.03 WatcherOutput, B.04 WorstSeverityPolicy, B.05 ghost-turn watcher, B.09 review-loop watcher, B.10 spec-drift watcher, B.12 test-failure-budget watcher, B.13 time-overrun watcher, B.14 context-window-pressure watcher, B.15 Conductor 10 watchers, B.16 evaluate() pipeline, B.20 ConductorDecision 3-state, B.21 Kind variants consumed, B.23 outputs_to_signals bridge, B.24 circuit breaker, B.25 RoutingBias) |
| PARTIAL | 5 (B.06 compile-fail-repeat constant drift + Kind mismatch, B.07 cost-overrun constant drift + no escalation, B.08 iteration-loop constant drift + Kind mismatch, B.11 stuck-pattern constant drift + narrower action set, B.22 Kind variants unused) |
| NOT DONE | 3 (B.17 CEP composite patterns, B.18 streaming anomaly detection, B.19 CognitiveSignal enum) |

(Count total is 24; B.25 is documented as DONE-but-undocumented which
lifts the DONE total to 17 effectively — the table above uses the
strict per-item status.)

**HIGH-severity items**: none. **MEDIUM-severity items**: B.19
(`CognitiveSignal` enum is unimplemented despite the
"Implementation: Built" banner on Doc 09).

Doc 01 is largely accurate on the 10-watcher catalog — all ten
watchers ship with the exact severity ladder and signal-stream
semantics the doc describes. Four watchers have constant-name drifts
(B.06 `MAX_COMPILE_FAIL_REPEAT` → `MAX_IDENTICAL_COMPILE_FAILURES`,
B.07 `DEFAULT_BUDGET_USD` → `DEFAULT_BUDGET`, B.08 `MAX_ITERATION_LOOP`
→ `MAX_IMPLEMENTER_ATTEMPTS`, B.11 `MAX_STUCK_REPEATS` →
`MAX_IDENTICAL_ACTIONS`) and two have `Kind` mismatches (B.06 scans
`CompileDiagnostic` not `GateVerdict`; B.08 scans `PlanPhase` not
`GateVerdict`) — all LOW severity and easy doc fixes. The 172-line
§"Watcher Composition" block (CEP, NFA patterns, `WatcherFamily`
struct) and the 172-line §"Streaming Anomaly Detection Integration"
block (Online Isolation Forest, CUSUM, TraceAegis) are design-only
(B.17, B.18). The `WATCHER_FAMILIES` constant is grep-positive only
inside the doc; the `CompositePattern` / `OnlineIsolationForest` /
`CusumDetector` / `BayesianFusionPolicy` types are entirely absent.
Doc 01 should either tag those two sections "Design — not implemented"
or move them to a separate future-work file so the "Implementation:
Built" banner applies only to the 10-watcher catalog.

Doc 09 is internally consistent: the §"Implementation Status" table
honestly declares `CognitiveSignal` as a planned extension and lists
the `ConductorDecision` 3-state replacement currently in service. The
only drift is the top-of-file "Implementation: Built" banner at line
8, which contradicts the honest self-report 239 lines later (B.19);
flipping it to "Design — planned extension" resolves the contradiction.

Two undocumented shipping features strengthen the conductor beyond
Doc 01's scope: the pre-watcher `CircuitBreaker` short-circuit (B.24)
and the `RoutingBias` feedback into the cascade router (B.25). Both
are load-bearing in production and worth a doc pass.

Recommend: (a) fix the four constant-name drifts in Doc 01 §§2, 3, 4,
7; (b) correct the `Kind` mismatches in Doc 01 §2 and §4; (c) re-tag
Doc 01 §"Watcher Composition" and §"Streaming Anomaly Detection
Integration" as "Design — not implemented"; (d) flip Doc 09's banner
from "Built" to "Planned extension"; (e) add `CircuitBreaker` and
`RoutingBias` subsections to Doc 01 so the doc's surface area matches
what `Conductor::evaluate()` actually emits.

## Agent Execution Notes

### B.19 — Truth-In-Advertising, Not Signal Redesign

The main actionable gap here is doc honesty around `CognitiveSignal`.

Default action:

1. keep `ConductorDecision` as the real shipped intervention surface,
2. mark `CognitiveSignal` as planned or deferred,
3. avoid inventing a new typed signal stack inside batch `07`.

### B.17-B.18 — Explicit Frontier Demotion

CEP composition and isolation-forest style watcher extensions are valid future work, but they are poor unattended-batch targets right now.

Acceptance criteria for this section:

- later agents can tell the difference between the real 10-watcher ensemble and the design-only watcher extensions,
- doc 09 no longer contradicts itself,
- current signal semantics are clearer without adding a new signal architecture.
