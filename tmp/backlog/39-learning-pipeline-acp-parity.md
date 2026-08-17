# ACP Learning-Pipeline Parity

**Priority**: P2
**Size**: M (1-2 days)

---

## Problem

Roko has three learning subsystems that shape model selection and accumulate feedback
during CLI plan execution: the `CascadeRouter` (learned model routing), `DaimonState`
(affect-based dispatch modulation), and `ExperimentStore` (canonical section
assignment with outcome recording). When work arrives through the CLI runner-v2 path,
all three subsystems are exercised at dispatch time and updated at completion time.

When the same work arrives through the ACP path — the protocol used by IDE integrations
such as Cursor — the three subsystems are *partially* wired. The result is that
IDE-driven sessions route more naively and contribute incomplete learning signal.

Specifically:

- **`CascadeRouter` model selection** is wired: `cascade_select_model` in
  `bridge_events.rs` is called before dispatch when no explicit model is set and no
  experiment overrides the model. The outcome is fed back through
  `record_cascade_observation` after the turn completes. This part works.

- **`DaimonState` affect modulation** is *partially* wired. `DaimonPolicy` is read
  from disk (canonical path `.roko/daimon/affect.json`, legacy fallback
  `.roko/state/daimon.json`) and passed into the dispatch call. However, the ACP path
  reads a lightweight policy value derived from raw JSON fields, not the full
  `DaimonState` struct. The mapping discards the energy, behavioral vitality, and
  cortical-state fields that the runner uses for modulation. The ACP comment at
  line 713 reads: *"We read-only — the orchestrator is the sole writer of DaimonState"*,
  which correctly avoids a write conflict but also means the ACP path only uses a
  coarse approximation of the affect state.

- **`ExperimentStore` outcome recording** is wired for *content* injection (the
  selected variant text is appended to the prompt), but the recorded outcome is a
  simple binary success/failure based on whether dispatch completed without error. The
  CLI runner uses a richer receipt protocol that ties the experiment to the gate
  verdicts from the completed task. ACP outcomes therefore land in the experiment stats
  but carry weaker signal than CLI outcomes.

- **`CalibrationTracker` feedback into `CascadeRouter`** does not exist in ACP at all.
  The CLI path threads per-turn calibration hints (output token counts, TTFT, latency
  deviation from expectation) back into the router. ACP calls
  `record_cascade_observation` directly without the calibration layer. For short-lived
  ACP turns this is acceptable, but for research-mode sessions that run multiple
  tool-loop iterations it means the router never learns that a model is consistently
  slow or over-generating.

The net effect is that an engineer using the IDE integration gets model routing that
lags behind what the CLI runner would pick, and their work produces weaker experiment
signal. Over time, experiment winners trained mostly on CLI data may not transfer well
to IDE usage patterns.

### What already exists

| Component | Location | Status |
|---|---|---|
| `cascade_select_model` | `crates/roko-acp/src/bridge_events.rs:1004` | EXISTS and called |
| `record_cascade_observation` | `crates/roko-acp/src/bridge_events.rs:2377` | EXISTS and called |
| `DaimonPolicy` construction from JSON | `crates/roko-acp/src/bridge_events.rs:714` | EXISTS (coarse mapping) |
| `assign_acp_experiment` | `crates/roko-acp/src/bridge_events.rs:798` | EXISTS and called |
| `record_acp_experiment_outcome` | `crates/roko-acp/src/bridge_events.rs:889` | EXISTS and called (binary signal only) |
| `ExperimentStore::transaction` | `crates/roko-learn/src/prompt_experiment.rs` | EXISTS |
| `CalibrationTracker` | `crates/roko-learn/src/` | EXISTS (not wired in ACP) |
| `DaimonState` full struct | `crates/roko-core/src/` | EXISTS (ACP uses policy subset) |

### What is missing

1. **Full `DaimonState` mapping in ACP** — The affect-state read at
   `bridge_events.rs:714` constructs a coarse `DaimonPolicy` from two JSON fields
   (`confidence`, `behavioral_state`). The runner loads the full `DaimonState` and
   passes all energy/vitality fields into dispatch. ACP should load the full struct
   (or at minimum pass through the same fields the runner uses for modulation) so that
   high-fatigue or low-energy states suppress expensive model selection the same way
   they do in CLI mode.

2. **`CalibrationTracker` feedback wiring** — After `record_cascade_observation` is
   called, the CLI runner feeds back a `CalibrationHint` (actual output tokens vs
   estimate, actual TTFT vs expectation). ACP omits this. For multi-iteration tool
   loops (research mode, long coding sessions), this calibration signal is how the
   router learns that a model is consistently underestimating cost.

3. **Richer experiment outcome signal** — ACP records `success = dispatch_succeeded`
   (a transport-level bool). The runner ties experiment outcomes to gate verdicts: an
   experiment that caused a compile failure contributes a `false` regardless of whether
   the LLM responded. ACP should record outcomes against task-level quality signal when
   that signal is available (e.g. the pipeline dispatch path already has access to a
   pass/fail result from the workflow engine).

---

## Proposed changes

### Change A: full DaimonState read in `acp_dispatch_prompt`
In `bridge_events.rs`, replace the two-field JSON parse at line 714 with a full
`DaimonState::load_or_new(path)` call and derive the `DaimonPolicy` from its
`to_policy()` method (or equivalent). This is a read-only operation — the ACP path
remains a non-writer.

Estimated: ~30 lines changed. Risk: low.

### Change B: CalibrationTracker feedback after observation
After the `record_cascade_observation` call at line 2377, construct a
`CalibrationHint` from the `stream_result` usage fields (output tokens, TTFT measured
from `dispatch_started.elapsed()`) and feed it to the router via the same API the
runner uses. Gate this behind the same `!is_pipeline_dispatch` check that already
guards the existing observation.

Estimated: ~40 lines. Risk: low.

### Change C: quality-signal experiment outcome in pipeline dispatch
In the `is_pipeline_dispatch` branch, the pipeline result already carries a pass/fail
from `run_with_workflow_engine`. Thread that result into
`record_acp_experiment_outcome` instead of defaulting to `dispatch_succeeded`
(which is always `true` for a successful HTTP round trip).

Estimated: ~20 lines. Risk: low.

---

## Acceptance criteria

1. `grep -n 'DaimonState' crates/roko-acp/src/` returns a real struct load, not just
   `DaimonPolicy::default()` or a two-field JSON parse.
2. `grep -n 'CalibrationTracker\|CalibrationHint' crates/roko-acp/src/bridge_events.rs`
   returns at least one call site in the post-dispatch completion block.
3. An ACP session that triggers a pipeline dispatch records an experiment outcome using
   the pipeline pass/fail result rather than the transport-level `dispatch_succeeded`.
4. `cargo test -p roko-acp` passes with zero failures.
5. `cargo clippy -p roko-acp -- -D warnings` is clean.

---

## References

- `crates/roko-acp/src/bridge_events.rs` — main ACP dispatch path (6,600+ lines)
- `crates/roko-acp/src/session.rs` — ACP session state
- `crates/roko-learn/src/cascade_router.rs` — `CascadeRouter`, `CalibrationHint`
- `crates/roko-core/src/` — `DaimonState`, `DaimonPolicy`
- `crates/roko-cli/src/runner/event_loop.rs` — CLI runner dispatch for reference
- `.roko/GAPS.md` — "ACP/serve still inject context rather than using the runner receipt protocol" (known gap)
