# 20 - Learning and Telemetry Redesign

Scope: `crates/roko-agent/src/usage.rs`, `crates/roko-agent/src/translate/openai.rs`, `crates/roko-runtime/src/effect_driver.rs`, `crates/roko-cli/src/runtime_feedback/routing.rs`, `crates/roko-learn/src/cascade_router.rs`, `crates/roko-learn/src/runtime_feedback.rs`, `crates/roko-learn/src/efficiency.rs`

The runner batches added a lot of feedback plumbing, but the data model still collapses important distinctions before learning can use them. This pass focuses on unknown-vs-zero, actual-vs-requested model, and confidence-stat updates that are presented as routing learning.

## Findings

### HIGH: unknown usage is still converted to zero at the adapter boundary

`usage.rs:11-28` introduces `UsageObservation` with optional token/cost fields so missing provider data can remain unknown. But `impl From<UsageObservation> for Usage` at `usage.rs:57-72` maps every `None` to `0`.

The older flat type still dominates the runtime contract: `TokenUsage` has concrete `u64/f64` fields in `foundation.rs:150-157`, feedback records concrete `input_tokens`, `output_tokens`, and `cost_usd` at `foundation.rs:215-224`, and `effect_driver.rs:209-214` comments that a future improvement would use `None` when zero means unknown.

Expected design: optional usage must stay optional through provider parsing, model response, feedback, runtime reports, and learning ingestion. Convert to display defaults only at UI boundaries.

### HIGH: provider parsers continue to default missing provider fields to zero

`translate/openai.rs:267-285` returns `Usage::default()` when no usage block exists and uses `unwrap_or(0)` for missing prompt/completion/cache fields. Similar zero-default patterns exist in other adapters and runtime feedback conversion.

This corrupts efficiency data: a provider that did not report usage looks identical to a provider that truly used zero tokens or cost.

Expected design: provider parsers should produce `UsageObservation { source: ProviderReported, fields: Option<_> }`. A normalization layer can later estimate missing values with `source: Estimated`, but it should not overwrite provider absence.

### HIGH: routing feedback updates confidence stats, not contextual routing

`runtime_feedback/routing.rs:71-79` says the runner does not compute full `RoutingContext`, so it records through `record_outcome`. `cascade_router.rs:1037-1054` confirms `record_outcome` only updates per-model confidence stats. The test at `cascade/tests.rs:1304-1311` explicitly asserts this does not update the LinUCB observation counter.

That means some "learning feedback" paths are not training the contextual router at all. They create the appearance of closed-loop learning while leaving the decision policy mostly unchanged.

Expected design: feedback ingestion should require either a structured routing context or a typed `ContextUnavailable` reason. Dashboard/reporting should distinguish confidence-only updates from bandit observations.

### MEDIUM: force-backend override learning uses default context

`cascade_router.rs:134-142` implements `ForceBackendOverrideRecorder` by calling `record_override_outcome` with `RoutingContext::default()`. This does update LinUCB, but with a context vector that does not represent the task.

Expected design: forced backend outcomes should either be excluded from contextual learning or recorded with the real dispatch context and a dampening/provenance flag. A default feature vector teaches the router the wrong relationship.

### MEDIUM: runtime feedback records zeros for absent metadata

`runtime_feedback.rs:884-930` maps missing plan id, iteration, reasoning tokens, TTFT, tool counts, and prompt token totals to empty strings or zero values. `runtime_feedback.rs:1019-1023` does the same for gate attempt/duration fields. `efficiency.rs:220-235` shows the default efficiency event also uses concrete zeros for cost, tokens, tools, and time.

Some defaults are reasonable display values, but they are not valid learning observations. They make missing instrumentation look like extremely cheap/fast successful behavior.

Expected design: telemetry records should use optional fields plus provenance. Derived feature builders can decide which observations are eligible for learning and which are dashboard-only.

## Redesign Direction

1. Promote `UsageObservation` or an equivalent optional usage type into `ModelCallResponse`, feedback, runtime events, and learning records.
2. Keep requested model/provider separate from attempted/final model/provider everywhere.
3. Split learning updates into `ContextualObservation`, `ConfidenceObservation`, and `DashboardOnlyObservation`.
4. Reject or quarantine contextual router updates without a real `RoutingContext`.
5. Add tests that fail if unknown usage, unknown duration, or missing model metadata is serialized as zero in learning records.
