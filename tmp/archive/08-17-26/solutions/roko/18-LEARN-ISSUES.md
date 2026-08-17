# Learning & Feedback: Issues

## Critical Issues

### I-01: `roko chat` Records Zero Learning Signals

**Severity:** Critical
**Components:** FeedbackService, EpisodeLogger, CascadeRouter
**Files:**
- `crates/roko-cli/src/chat_session.rs`
- `crates/roko-learn/src/feedback_service.rs`

**Problem:** The `roko chat` interactive REPL makes model calls but records no episodes,
no routing observations, no cost tracking, and no learning signals of any kind. Every
chat session is a lost learning opportunity. Users may spend significant time in chat
mode during development, and that experience data is discarded entirely.

**Impact:** Chat is likely the most-used entry point for casual interaction. All of that
usage data -- what models work for which questions, what latency is acceptable, what
cost is incurred -- is invisible to the learning subsystem.

**Fix:**
1. Instantiate `FeedbackService::from_roko_dir_with_episodes()` in chat session setup
2. Emit `FeedbackEvent::ModelCall` after each model response
3. Emit `FeedbackEvent::WorkflowComplete` when the chat session ends
4. Optionally: attach CascadeRouter to FeedbackService for routing observations

**Effort:** Small (FeedbackService already handles all the fan-out)

---

### I-02: ACP Records Only Adaptive Gate Thresholds

**Severity:** Critical
**Components:** FeedbackService, EpisodeLogger
**Files:**
- `crates/roko-acp/src/runner.rs`
- `crates/roko-acp/src/pipeline.rs`

**Problem:** The Agent Control Plane (ACP), used for editor integration (VS Code, etc.),
records only adaptive gate thresholds for rungs 0/1/2. No episodes, no routing, no cost
tracking. ACP is the primary path for inline code assistance, meaning a large volume of
model interactions produce zero learning signal.

**Impact:** Editor-integrated usage is high-frequency and likely represents the majority
of daily model calls for active users. The learning subsystem has no visibility into
this usage pattern.

**Fix:**
1. Create FeedbackService in ACP pipeline initialization
2. Emit ModelCall events from ACP model dispatch
3. Emit GateResult events from ACP gate pipeline (currently only writes thresholds)
4. Consider: should ACP model calls feed the same CascadeRouter as CLI?

**Effort:** Small-Medium (need to thread FeedbackService through ACP pipeline)

---

### I-03: Full Learning Loop Only in Dead Code (orchestrate.rs)

**Severity:** Critical
**Components:** All 10 learning components
**Files:**
- `crates/roko-cli/src/orchestrate.rs` (21K+ lines)

**Problem:** The complete closed learning loop -- CascadeRouter selection, budget checks,
experiment variant assignment, playbook query, 9-layer prompt composition, efficiency
event recording, conductor intervention, distillation trigger, replan on gate failure --
is only reachable from `orchestrate.rs`, which is dead code (no live CLI command invokes it).

**Impact:** The most sophisticated part of the system is unreachable. The system cannot
improve itself because the full feedback loop never runs.

**Fix:** This is the fundamental architectural issue. Two paths:
1. Revive orchestrate.rs as the canonical execution path (high effort, high risk)
2. Port the critical feedback wiring from orchestrate.rs into the live paths:
   `roko run`, `roko chat`, ACP (medium effort, lower risk)

Option 2 is preferred. The FeedbackService was designed specifically for this -- it
centralizes the fan-out so each entry point only needs to emit 3 event types.

**Effort:** Large (the core engineering project)

---

## High-Severity Issues

### I-04: Simplified Routing Context in `roko run`

**Severity:** High
**Components:** CascadeRouter, LearningRuntime
**Files:**
- `crates/roko-cli/src/run.rs`
- `crates/roko-learn/src/runtime_feedback.rs`

**Problem:** When `roko run` updates the CascadeRouter via `LearningRuntime::record_completed_run()`,
it constructs a simplified routing context from the episode rather than the full 18-feature
`RoutingContext`. Key missing features:
- conductor_load (always 0.0)
- active_agents (always 0)
- ready_queue_depth (always 0)
- max_queue_wait_hours (always 0.0)
- daimon_policy (always default)
- thinking_level (always None)
- temperament (always None)
- plan_context_tokens (always None)
- tier_thresholds (always None)

**Impact:** The bandit learns from impoverished context vectors, limiting its ability to
distinguish situations where different models are optimal. The learned policy may not
generalize well to orchestrated plan execution where these features matter.

**Fix:**
1. Thread `RoutingContext` through to `CompletedRunInput`
2. Populate conductor_load from runtime state
3. Populate daimon_policy from loaded affect state
4. Populate plan_context_tokens from actual prompt assembly

**Effort:** Medium

---

### I-05: No Budget Enforcement in Any Live Path

**Severity:** High
**Components:** BudgetGuardrail
**Files:**
- `crates/roko-learn/src/budget.rs`
- `crates/roko-cli/src/run.rs`
- `crates/roko-cli/src/dispatch_direct.rs`

**Problem:** `BudgetGuardrail` implements 3-scope budget limits (per-task, per-session,
per-day) with 5 graduated actions (Ok, Warn, RouteToCheaper, BlockNewSessions, Block).
It is never instantiated or checked in any live path. A runaway agent can spend unlimited
money without any guardrail.

**Impact:** Cost exposure risk. In production or heavy development, there is no automatic
cost protection. Users must manually monitor spend.

**Fix:**
1. Load budget config from `roko.toml` (fields exist: `budget.max_task_usd`, etc.)
2. Instantiate BudgetGuardrail at session start
3. Check before each model dispatch
4. Route to cheaper model or block when thresholds crossed
5. Integrate with FeedbackService cost tracking for cumulative accounting

**Effort:** Medium

---

### I-06: No Conductor Intervention in Live Paths

**Severity:** High
**Components:** ConductorBandit
**Files:**
- `crates/roko-learn/src/conductor.rs`
- `crates/roko-cli/src/run.rs`

**Problem:** The conductor bandit (7 actions, 19-dim context, blended Thompson+linear scoring)
sits above the retry loop and decides whether a failing task should continue, receive a hint,
escalate, restart, or abort. It is never invoked from any live path. All retry decisions are
hardcoded.

**Impact:** Retry behavior cannot improve. The system cannot learn that "when compile fails
after 3 attempts, switching models works better than continuing." Users pay for wasted
retries that learned policy would have avoided.

**Fix:**
1. Load ConductorBandit state from `.roko/learn/conductor.json`
2. Call `bandit.select_action()` before each retry in `roko run`
3. Feed reward after retry outcome
4. Save state after each observation
5. Surface conductor decisions in episode records

**Effort:** Medium

---

### I-07: Dual Episode Writes in `roko run`

**Severity:** Medium
**Components:** EpisodeLogger
**Files:**
- `crates/roko-cli/src/run.rs`

**Problem:** `roko run` writes episodes twice: once via a direct `append_episode_log()` call,
and again via `LearningRuntime::record_completed_run()` which internally appends to its own
episode log. This produces duplicate records in different files (`.roko/episodes.jsonl` at
root vs. `.roko/learn/episodes.jsonl`).

**Impact:** Disk waste, confusion about which log is canonical, potential double-counting
in analytics that read both files.

**Fix:**
1. Remove the direct `append_episode_log()` call
2. Let LearningRuntime be the single writer
3. Ensure all readers use `LearningPaths.episodes_jsonl`
4. Consider: symlink or redirect for backward compatibility

**Effort:** Small

---

### I-08: Dream Cycle Has No Runtime Trigger

**Severity:** Medium
**Components:** DreamCycle, DreamRunner
**Files:**
- `crates/roko-dreams/src/runner.rs`
- `crates/roko-dreams/src/cycle.rs`

**Problem:** The dream cycle supports cron, plan-completion, heartbeat, and bus-pulse
triggers, but no trigger is instantiated at runtime. `roko knowledge dream run` executes
a one-shot cycle, but there is no automatic background consolidation.

**Impact:** Knowledge consolidation, playbook extraction, routing advice generation, and
tier progression only happen when the user explicitly runs the dream command. Offline
learning is manual, not automatic.

**Fix:**
1. Start a dream loop in `roko serve` background tasks
2. Configure trigger policy via `roko.toml` (cron, plan-completion, or both)
3. DreamRunner already supports all trigger modes -- just needs instantiation
4. Report dream cycle status in `roko status` output

**Effort:** Small-Medium

---

### I-09: Section Effectiveness Not Read by PromptAssemblyService

**Severity:** Medium
**Components:** SectionEffectivenessRegistry, PromptAssemblyService
**Files:**
- `crates/roko-learn/src/section_effect.rs`
- `crates/roko-compose/src/prompt_assembly_service.rs`

**Problem:** Section effectiveness tracking works: it records inclusion/exclusion pass rates,
computes lift, and generates budget weights. FeedbackService updates it on every gate result.
But PromptAssemblyService does not read the weights during prompt assembly. The data is
collected but never acted upon.

**Impact:** Prompt sections that actively hurt pass rates continue to consume token budget.
Sections that improve pass rates are not preferentially expanded.

**Fix:**
1. `PromptAssemblyService` already has `section_weights` field
2. Wire FeedbackService.section_effectiveness() output to PromptAssemblyService
3. Apply weights during section budget allocation
4. Log when a section is deprioritized due to negative effectiveness

**Effort:** Small

---

### I-10: Knowledge Store Not Consulted for Model Selection

**Severity:** Medium
**Components:** KnowledgeStore, CascadeRouter
**Files:**
- `crates/roko-neuro/src/knowledge_store.rs`
- `crates/roko-learn/src/cascade_router.rs`

**Problem:** The knowledge store contains task-specific insights, warnings, and causal links
that could inform model routing decisions. CascadeRouter does not query the store before
selecting a model. Dream routing advice is generated but not loaded at dispatch time.

**Impact:** The system cannot learn "this type of task works better with Opus" from durable
knowledge. Only the bandit's per-observation memory is used, which forgets context that
knowledge entries preserve across sessions.

**Fix:**
1. Load `DreamRoutingAdvice` at CascadeRouter initialization
2. Apply `dream_advice_to_routing_bias()` (already implemented in `routing_advice.rs`)
3. Query knowledge store for task-specific model hints during routing
4. Feed routing outcome back to knowledge store via FeedbackService

**Effort:** Medium

---

## Lower-Severity Issues

### I-11: Experiment Winner Application Not Automated

**Severity:** Low
**Components:** ExperimentStore
**Files:**
- `crates/roko-learn/src/prompt_experiment.rs`

**Problem:** When an experiment concludes and identifies a winner, the result is written
to `experiment-winners.json` for operator review. The winning variant is not automatically
applied as the new default. An operator must manually update the configuration.

**Impact:** Experiment conclusions pile up without being acted upon. The benefit of running
experiments is delayed until human intervention.

**Fix:**
1. Load experiment-winners.json at startup
2. Auto-apply winning variants to prompt assembly configuration
3. Log when a variant is auto-applied
4. Add `auto_apply_winners` config flag (default true)

---

### I-12: No Cross-Session Cost Aggregation

**Severity:** Low
**Components:** BudgetGuardrail, CostsDb
**Files:**
- `crates/roko-learn/src/budget.rs`
- `crates/roko-learn/src/costs_db.rs`

**Problem:** Cost tracking per session exists (CostsDb appends records), but there is no
cross-session aggregation for dashboard display or daily budget enforcement. The `per_day`
budget scope in BudgetGuardrail has no way to know what was spent in previous sessions today.

**Fix:**
1. CostsDb.aggregate_since(today_start) -> daily total
2. Initialize BudgetGuardrail.day_spent from aggregate
3. Expose daily/weekly/monthly aggregates via `roko learn efficiency`

---

### I-13: Regression Detection Has No Alerting Path

**Severity:** Low
**Components:** regression.rs
**Files:**
- `crates/roko-learn/src/regression.rs`

**Problem:** `detect_regressions()` produces `RegressionReport` with alerts (pass rate
drop > 15%, cost increase > 20%, etc.), but there is no alerting path. The report is
returned from `record_completed_run()` in `LearningUpdate.regression_report` but not
surfaced to the user or acted upon.

**Fix:**
1. Log regression alerts at WARN level
2. Surface in `roko status` output
3. Feed severe regressions to conductor (trigger model switch or abort)
4. Optional: webhook or notification integration

---

### I-14: Anomaly Detector Not Wired to Live Paths

**Severity:** Low
**Components:** AnomalyDetector
**Files:**
- `crates/roko-learn/src/anomaly.rs`

**Problem:** The anomaly detector (prompt loops, cost spikes, quality degradation) is
session-local and lightweight. It is not instantiated in any live path.

**Fix:**
1. Create AnomalyDetector at session start in `roko run`
2. Check prompt hash before each dispatch
3. Check cost after each response
4. On anomaly: log warning, optionally trigger conductor abort

---

### I-15: HDC Fingerprinting Requires Feature Flag

**Severity:** Low
**Components:** KnowledgeStore, HdcVector
**Files:**
- `crates/roko-neuro/src/knowledge_store.rs`
- `crates/roko-neuro/src/hdc.rs`

**Problem:** Anti-knowledge gating and HDC-based similarity scoring in KnowledgeStore
require the `hdc` feature flag on `roko-neuro`. Without it, several quality-control
mechanisms are inactive (anti-knowledge warn/discount/reject thresholds, similarity
queries for deduplication).

**Impact:** Default builds may miss anti-knowledge protections.

**Fix:** Evaluate whether `hdc` should be a default feature for `roko-neuro`.

---

### I-16: LearningRuntime Update Frequency Defaults May Be Too Aggressive

**Severity:** Low
**Components:** LearningRuntime
**Files:**
- `crates/roko-learn/src/runtime_feedback.rs`

**Problem:** Default update frequencies:
- `router_every_n_episodes: 1` (every episode)
- `experiments_every_n: 1` (every episode)
- `skill_mining_every_n: 10`
- `pattern_discovery_every_n: 20`
- `distiller_every_n: 50`

Router and experiment updates on every episode may cause I/O overhead for high-throughput
workloads. The frequency-gating exists precisely for this reason but the defaults may need
tuning.

**Fix:** Profile I/O overhead at scale. Consider increasing `router_every_n_episodes` to 5
if flush latency is measurable.

---

### I-17: StagingBuffer Promotion Requires Dream Cycle

**Severity:** Low
**Components:** StagingBuffer
**Files:**
- `crates/roko-dreams/src/staging.rs`

**Problem:** Knowledge candidates in the StagingBuffer progress from Raw -> Replayed ->
Validated, but promotion to the durable store only happens during a dream cycle. Without
a running dream cycle, the staging buffer grows without bound.

**Fix:** Add a lightweight promotion check in LearningRuntime that promotes Validated
entries without requiring a full dream cycle.

---

### I-18: Pattern Discovery Trigrams Are Limited in Scope

**Severity:** Low
**Components:** PatternMiner
**Files:**
- `crates/roko-learn/src/pattern_discovery.rs`

**Problem:** Pattern mining uses fixed trigrams (3-action sequences). This misses:
- Longer patterns (4+ action sequences)
- Skip-grams (actions separated by intervening steps)
- Temporal patterns (time-sensitive sequences)

**Impact:** Some valuable patterns are invisible to the miner.

**Fix:** Consider extending to variable-length n-grams or using HDC-based sequence
encoding for more flexible pattern representation. Low priority -- trigrams capture
the most common patterns.

---

### I-19: Provider Health Circuit Breaker Not Connected to CascadeRouter

**Severity:** Medium
**Components:** ProviderHealthTracker, CascadeRouter
**Files:**
- `crates/roko-learn/src/provider_health.rs`
- `crates/roko-learn/src/cascade_router.rs`

**Problem:** ProviderHealthTracker implements circuit breaker logic (tracking success/failure
rates per provider, opening circuits when failure rate exceeds threshold). CascadeRouter has
a `ProviderHealthRegistry` parameter in some helper functions but it is not consistently
wired in live paths.

**Impact:** When a provider goes down, the router may continue selecting models from that
provider, leading to cascading failures.

**Fix:**
1. Load ProviderHealthRegistry at router initialization
2. Filter out models whose provider circuit is open before UCB scoring
3. Feed provider health state from FeedbackService model call success/failure

---

### I-20: Cost Table Not Updated from Live Observations

**Severity:** Low
**Components:** CostTable
**Files:**
- `crates/roko-learn/src/cost_table.rs`

**Problem:** `CostTable` provides per-model pricing for cost estimation. It uses hardcoded
default prices. Actual observed costs (from provider billing) are not used to update the
table, so cost estimates may drift from reality as pricing changes.

**Fix:** Record observed cost-per-token from actual model responses and update CostTable
entries when observed prices differ from defaults.
