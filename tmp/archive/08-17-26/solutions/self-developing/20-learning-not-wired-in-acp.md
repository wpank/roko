# 20: Self-Learning / Cybernetic Features Not Wired in ACP

## Problem Statement

The user asked: "How do I enable the self-learning cybernetic things in ACP/Zed? Like the
cascade router? Or things that learn as I keep doing things?"

This document audits every learning subsystem in the codebase, explains what each one does,
identifies exactly why each is CLI-only, and provides a concrete design and Rust code sketches
for wiring each into the ACP path.

---

## 1. Complete Inventory of All Learning Subsystems

### 1.1 Core Learning Crate (`crates/roko-learn/src/`)

| Module | File | What It Does | State Storage |
|--------|------|-------------|--------------|
| `cascade_router` | `cascade_router.rs` | Three-stage LinUCB bandit for model selection | `.roko/learn/cascade-router.json` |
| `episode_logger` | `episode_logger.rs` | Append-only JSONL of every agent turn | `.roko/episodes.jsonl` |
| `efficiency` | `efficiency.rs` | Per-turn cost/quality events (20+ fields) | `.roko/learn/efficiency.jsonl` |
| `prompt_experiment` | `prompt_experiment.rs` | UCB1 A/B testing for prompt variants | `.roko/learn/experiments.json` |
| `model_experiment` | `model_experiment.rs` | UCB1 A/B testing for model variants | `.roko/learn/model-experiments.json` |
| `error_pattern_store` | `error_pattern_store.rs` | Records failure patterns from gates | `.roko/learn/discovered-patterns.json` |
| `conductor` | `conductor.rs` | Thompson/LinUCB bandit for retry interventions | In-memory, no persistence |
| `playbook` | `playbook.rs` | Reusable success patterns extracted from episodes | `.roko/learn/playbooks/` |
| `skill_library` | `skill_library.rs` | Structured skills agents invoke | `.roko/learn/skills.json` |
| `cfactor` | `cfactor.rs` | Catalyst Factor: ratio of productive to wasted tokens | `.roko/learn/c-factor.jsonl` |
| `runtime_feedback` | `runtime_feedback.rs` | `LearningRuntime`: single integration point coordinating all subsystems | All paths above |
| `context_pack_cache` | `context_pack_cache.rs` | Cached composed prompts keyed by task fingerprint | `.roko/learn/context-cache/` |
| `section_effect` | `section_effect.rs` | Prompt section effectiveness scoring | `.roko/learn/section-effects.json` |
| `provider_health` | `provider_health.rs` | Per-provider circuit breaker for LLM routing | In-memory + `AppState` |
| `latency` | `latency.rs` | Rolling latency EMAs and p-tiles per provider/model | In-memory |
| `anomaly` | `anomaly.rs` | Runaway loop, cost spike, quality degradation detection | In-memory |
| `bandits` | `bandits.rs` | Thompson sampling and UCB1 primitives | Depended on by cascade_router |
| `hdc_fingerprint` | `hdc_fingerprint.rs` | HDC vector fingerprints for episode deduplication | Stored in `Episode.hdc_fingerprint` |
| `pattern_discovery` | `pattern_discovery.rs` | Mining recurring causal patterns from episodes | Triggered by `record_completed_run` |
| `post_gate_reflection` | `post_gate_reflection.rs` | Structured reflection records for gate outcomes | `.roko/learn/post-gate-reflections.json` |
| `contextual_bandit` | `contextual_bandit.rs` | Generic contextual bandit policy | Used by cascade_router |
| `wal` | `wal.rs` | Write-ahead log for crash-safe learning state | `.roko/learn/wal.jsonl` |
| `routing_log` | `routing_log.rs` | Append-only routing decision audit log | `.roko/learn/routing-log.jsonl` |
| `playbook_rules` | `playbook_rules.rs` | Rule confidence tracking over playbook outcomes | `.roko/learn/playbook-rules.toml` |
| `budget` | `budget.rs` | Budget tracking and enforcement guardrails | In-memory |
| `regression` | `regression.rs` | Cross-run regression detection on task metrics | In-memory |
| `curriculum` | `curriculum.rs` | Task difficulty model for scheduling | In-memory |
| `event_subscriber` | `event_subscriber.rs` | Event bus fan-out to all learning subsystems | Runtime fan-out |
| `heuristics` | `heuristics.rs` | Worldview and research-provenance shells | In-memory |
| `pareto` | `pareto.rs` | Cost-quality Pareto frontier computation | Cached in `CascadeRouter` |

### 1.2 Dream Crate (`crates/roko-dreams/src/`)

| Module | File | What It Does |
|--------|------|-------------|
| `cycle` | `cycle.rs` | Main dream consolidation cycle: cluster episodes, extract patterns |
| `hypnagogia` | `hypnagogia.rs` | Pre-sleep processing: thalamic gating, executive loosening |
| `imagination` | `imagination.rs` | Counterfactual generation and hypothesis synthesis |
| `replay` | `replay.rs` | Experience replay for consolidating episode memory |
| `rehearsal` | `rehearsal.rs` | Threat scenario rehearsal |
| `routing_advice` | `routing_advice.rs` | Dream-derived routing bias → `.roko/dreams/routing-advice.json` |
| `staging` | `staging.rs` | Staging buffer for dream→neuro promotion candidates |
| `runner` | `runner.rs` | Public facade for triggering consolidation (`DreamRunner`) |
| `phase2/` | `phase2/` | Advanced dream phases: divergence, evolution, hauntology, synthesis |

### 1.3 Neuro Crate (`crates/roko-neuro/src/`)

| Module | File | What It Does | State Storage |
|--------|------|-------------|--------------|
| `knowledge_store` | `knowledge_store.rs` | Append-only JSONL store for durable knowledge entries | `.roko/neuro/knowledge.jsonl` |
| `distiller` | `distiller.rs` | Batch-distills episodes → `KnowledgeEntry` candidates via LLM | Called by `spawn_episode_distillation` |
| `tier_progression` | `tier_progression.rs` | D1 (episodes→insights) → D2 (insights→heuristics) → D3 (heuristics→PLAYBOOK.md) | `.roko/neuro/` |
| `context` | `context.rs` | `ContextAssembler`: queries knowledge for dispatch context | Read-only |
| `admission` | `admission.rs` | `KnowledgeAdmissionStore`: gates what knowledge enters the store | `.roko/neuro/admissions.jsonl` |
| `lifecycle` | `lifecycle.rs` | Decay, GC, archival for knowledge entries | Applied in-place on `knowledge.jsonl` |
| `episode_completion` | `episode_completion.rs` | `spawn_episode_distillation()`: called on every episode completion | Async, fire-and-forget |
| `temporal` | `temporal.rs` | Temporal decay and half-life calculations | Used by lifecycle |
| `hdc` | `hdc.rs` | HDC encoding for knowledge entries | Used at ingest time |

---

## 2. Subsystem Deep Dives

### 2.1 Cascade Router

**File:** `crates/roko-learn/src/cascade_router.rs`

**What it does:** Three-stage bandit for model selection that automatically transitions as observation count grows:

| Stage | Observations | Algorithm |
|-------|-------------|-----------|
| Static (cold) | < 50 | Hardcoded role→model lookup table |
| Confidence (warm) | 50–200 | Empirical pass rates + Wilson 95% confidence intervals |
| UCB (hot) | > 200 | `LinUCBRouter` — 17-dimensional contextual bandit |

**Feature vector (17 dims):**
- Dims 0–3: task tier one-hot (mechanical / focused / integrative / architectural)
- Dims 4–7: complexity band one-hot
- Dim 8: crate familiarity score (0.0–1.0)
- Dim 9: has-prior-failure flag
- Dim 10: conductor load
- Dim 11: active agents
- Dim 12: queue depth
- Dim 13: max queue wait hours
- Dim 14: iteration number
- Dim 15–16: daimon affect scalars

**Arm selection:** Returns a `CascadeModel` with: primary model, ordered fallback chain, context-overflow fallback, latency SLA. The router also adjusts selection based on: Pareto dominance filtering, provider health circuit breaker, C-Factor pressure signal, daimon temperament shift.

**Reward signal:** `compute_acp_reward()` in ACP, `compute_routing_reward_v2()` in CLI. Both produce 0.0–1.0 with bonuses for fast time-to-first-token and low output-token counts (penalizes verbose/confused models).

**Persistence:** `CascadeRouter::save()` / `CascadeRouter::load_or_new()` → `.roko/learn/cascade-router.json`

**ACP status:** Observations ARE written (`record_cascade_observation()` at `bridge_events.rs:508`). Selection is NOT used — ACP uses the model the user picks from the dropdown.

### 2.2 Episode Logger

**File:** `crates/roko-learn/src/episode_logger.rs`

**What it does:** Append-only JSONL record of every agent invocation. One `Episode` per turn with 30+ fields: model, backend, success/failure, token counts, cost, gate verdicts, HDC fingerprint, extra metadata.

**ACP status:** WIRED. `append_acp_episode()` at `bridge_events.rs:296` writes an episode after every non-slash-command dispatch.

### 2.3 Efficiency Events

**File:** `crates/roko-learn/src/efficiency.rs`

**What it does:** `AgentEfficiencyEvent` — 20+ fields covering identity, token accounting, cost accounting, prompt section attribution, tool utilization, and timing. Emitted once per agent turn. Downstream consumers: bandits, dashboards, regression detector.

**ACP status:** NOT WIRED. ACP never emits `AgentEfficiencyEvent`. The efficiency JSONL (`efficiency.jsonl`) is never written from the ACP path. The `AppState` in `roko-serve` holds a `CascadeRouter` and serves `/api/learn/efficiency` by reading the file, but nothing in ACP writes it.

**CLI location:** `orchestrate.rs emit_efficiency_event()` called at lines 11597 (success path), ~11700 (failure path).

### 2.4 Episode Distillation

**File:** `crates/roko-neuro/src/episode_completion.rs`, `distiller.rs`

**What it does:** After each completed episode, `spawn_episode_distillation()` is called. It batches recently completed episodes and asks a fast model (Haiku by default) to extract reusable `KnowledgeEntry` candidates: insights, heuristics, warnings, causal links, strategy fragments. These are then promoted via the `KnowledgeAdmissionStore` into `.roko/neuro/knowledge.jsonl`.

**ACP status:** NOT WIRED. In the CLI, `install_episode_distillation_hook()` in `learning_helpers.rs:417` registers the hook on `LearningRuntime`. This hook is never installed in ACP. Episodes accumulate in `episodes.jsonl` but are never distilled automatically.

**CLI trigger:** `learning_helpers.rs:417` → `LearningRuntime.set_episode_completion_hook()` → called from `runtime_feedback.rs:2344` on every `record_completed_run()`.

### 2.5 Dream Consolidation

**File:** `crates/roko-dreams/src/runner.rs`, `cycle.rs`

**What it does:** Offline consolidation of episode memory. Three phases:
1. **Hypnagogia**: executive loosening + thalamic gating — selects which episodes enter the dream
2. **NREM (replay)**: clusters episodes by causal patterns, detects regressions
3. **REM (imagination)**: generates counterfactual hypotheses and strategy mutations
4. **Integration**: promotes `StagingBuffer` entries into `KnowledgeStore`, updates dream routing advice

**Output:** `DreamCycleReport` with: processed episodes, clusters, knowledge entries written, playbooks created, regressions detected, strategy hypotheses.

**ACP status:** PARTIALLY WIRED. The `/dream` slash command in `bridge_events.rs:2847` triggers `DreamRunner::consolidate_now()`. The `roko-serve` daemon runs `start_dream_loop()` in `dreams.rs` as a background loop. However, auto-dream on episode accumulation is NOT wired from within the ACP dispatch path itself. The CLI's `maybe_auto_dream()` (called at plan completion) does not exist in ACP.

**CLI trigger:** `orchestrate.rs:8108` `maybe_auto_dream()` called after each plan completes if `config.dreams.auto_dream` is true and enough new episodes have accumulated since the last dream report.

### 2.6 Model Experiments (A/B Testing)

**File:** `crates/roko-learn/src/model_experiment.rs`

**What it does:** Runs A/B tests across model variants. Each `ModelExperiment` has variants with UCB1 arm selection. On each dispatch, a variant is selected; on outcome, `ModelVariantStats` are updated. When `min_trials_per_variant` is met and effect size exceeds `min_effect_size`, a winner is declared. Winners are promoted into the static role table.

**ACP status:** NOT WIRED. `ModelExperimentStore` is initialized in the CLI via `model_experiments_path()` at `orchestrate.rs:280`, loaded at plan start, and queried during dispatch. Nothing in ACP reads or writes it.

### 2.7 Prompt Experiments (Section A/B Testing)

**File:** `crates/roko-learn/src/prompt_experiment.rs`

**What it does:** `ExperimentStore` manages `PromptExperiment` records. Each experiment tests variants of a prompt section (e.g. system prompt paragraph). Variant selection uses UCB1. On gate outcome, `VariantStats` are updated. Concluded winners are promoted to `.roko/learn/static-overrides.json` which the CLI then injects as static section text.

**ACP status:** NOT WIRED. `apply_concluded_experiment_overrides()` in `learning_helpers.rs` is called from `orchestrate.rs:4515` but never from ACP.

### 2.8 Error Pattern Store

**File:** `crates/roko-learn/src/error_pattern_store.rs`

**What it does:** Records `GateFailureObservation` records from gate failures. Builds a structured corpus of: gate name, failure category, error text, task context, and whether the following retry succeeded. Used to surface "this kind of error has occurred N times in similar contexts" to agents.

**ACP status:** NOT WIRED. Only written from `orchestrate.rs` gate failure handlers. ACP uses raw text similarity matching on last failure output instead.

**CLI location:** `orchestrate.rs` — called after each gate failure via `self.error_pattern_store`.

### 2.9 C-Factor Metrics

**File:** `crates/roko-learn/src/cfactor.rs`

**What it does:** `CFactor` measures catalyst impact — the ratio of "productive forward progress" to "total token spend". Components: `knowledge_integration_rate`, `convergence_velocity`, `decision_quality`, `autonomy_ratio`. Computed per-turn and stored as a fleet-level aggregate. High C-Factor → models are choosing good paths and not wasting tokens on exploration.

**ACP status:** NOT WIRED. `FleetCFactor` is computed in `orchestrate.rs` and written to `.roko/learn/c-factor.jsonl`. ACP never computes it.

### 2.10 Conductor / Retry Bandit

**File:** `crates/roko-learn/src/conductor.rs`

**What it does:** `ConductorBandit` is a Thompson/LinUCB bandit that decides whether a failing task should: Continue, InjectHint (ErrorDigest/SkillSuggestion/SimplifyApproach), SwitchModel, Restart, or Abort. It learns over time which interventions work for which error patterns.

**ACP status:** NOT WIRED. ACP retries are fixed-count with no learned intervention selection. The conductor only runs in the CLI orchestrator.

### 2.11 Daimon / Affect Engine

**File:** `crates/roko-daimon/`

**What it does:** `DaimonState` maintains an "emotional model" of the system: curiosity, frustration, fatigue, arousal. These affect routing decisions (e.g. high frustration → upgrade to a stronger model) and are updated by task outcomes, dream results, time pressure events. The `StrategyCoordinates` from daimon inform the cascade router's temperament shifts.

**ACP status:** NOT WIRED. `DaimonPolicy` in `acp_routing_context()` at `bridge_events.rs:457` is always `DaimonPolicy::default()`. The `DaimonState` object is never loaded or updated in ACP.

### 2.12 Tier Progression (D1/D2/D3)

**File:** `crates/roko-neuro/src/tier_progression.rs`

**What it does:** Three compression stages:
- **D1**: raw episodes → `InsightRecord` (recurring causal patterns, confidence scored)
- **D2**: insights with ≥5 supporting episodes → heuristics (falsifiable, half-life 45 days)
- **D3**: heuristics → `PLAYBOOK.md` (human-readable distillation)

**ACP status:** NOT WIRED. `TierProgression` is only instantiated in the CLI's knowledge management commands. Never called from ACP.

---

## 3. Why Each Feature Is CLI-Only: Architectural Root Causes

### Root Cause 1: `LearningRuntime` is never instantiated in ACP

The CLI creates a `LearningRuntime` per plan run (`orchestrate.rs:4504`) which holds all in-memory learning state: cascade router reference, episode logger, efficiency log, provider health, costs DB, skill library, playbook store, experiment store, error patterns, anomaly detector, conductor bandit. This runtime is the single integration point.

**ACP never creates a `LearningRuntime`.** ACP writes episodes and cascade observations directly to disk, but uses none of the higher-level subsystems that `LearningRuntime` coordinates.

### Root Cause 2: ACP is stateless across turns

Each ACP prompt dispatch in `handle_session_prompt()` is stateless — it creates a new `AcpSession`, loads config, runs the prompt, and exits. There is no persistent in-memory state between turns within a session, let alone across sessions. `LearningRuntime` is designed for a long-running plan process.

This is the primary architectural gap. The solution is either:
- (a) Load/save learning state from disk on every dispatch (cheap subset of subsystems), or
- (b) Maintain a persistent background service (the `roko serve` daemon approach), or
- (c) Lazy-load individual subsystems from the per-subsystem files on demand

### Root Cause 3: Dream consolidation needs an agent call

`DreamRunner::consolidate_now()` spawns an LLM call to synthesize patterns. ACP can't do this mid-turn (it would be a nested dispatch). The correct approach is background triggering — either via `roko serve` daemon or a post-turn async task.

### Root Cause 4: Efficiency events require prompt section attribution

`AgentEfficiencyEvent` captures `prompt_sections: Vec<PromptSectionMeta>` — per-section token counts from the composed prompt. ACP's `run_with_workflow_engine()` pathway doesn't track prompt section composition at the granularity the efficiency event requires.

### Root Cause 5: Daimon requires persistent emotional state

`DaimonState` needs to accumulate across many turns to be meaningful. ACP's stateless dispatch model can't maintain this. It requires the daemon/service approach.

---

## 4. ACP vs CLI: What IS Working vs What Is Missing

### Already Working in ACP (on every non-slash-command dispatch)

| Feature | Location | Persistence |
|---------|----------|-------------|
| Episode logging | `bridge_events.rs:296` → `append_acp_episode()` | `.roko/episodes.jsonl` |
| Cascade router observation | `bridge_events.rs:508` → `record_cascade_observation()` | `.roko/learn/cascade-router.json` |
| Knowledge injection | `knowledge.rs` → `query_dispatch_knowledge()` | Read from `.roko/neuro/knowledge.jsonl` |
| Playbook injection | Same as above, `Playbook` queried from `.roko/learn/playbooks/` | Read-only |
| Dream routing advice | `bridge_events.rs:2437` → `load_dream_routing_advice()` | Read from `.roko/dreams/routing-advice.json` |
| Provenance context | `bridge_events.rs:2402` → `build_provenance()` | Reads episodes + dream advice |
| `/dream` slash command | `bridge_events.rs:2847` | Writes `.roko/dreams/`, `.roko/neuro/` |
| `/knowledge` slash commands | `bridge_events.rs:2850+` | Reads/writes `.roko/neuro/` |

### Not Working in ACP (no writes, no effect on future dispatches)

| Feature | Missing Since |
|---------|--------------|
| Efficiency events | Never written in ACP |
| Episode distillation | No hook installed in ACP |
| Auto-dream on accumulation | No threshold check in ACP |
| C-Factor computation | Never computed in ACP |
| Daimon affect model | Always `DaimonPolicy::default()` |
| Model experiments | Never read or written |
| Prompt experiments | Winner overrides never applied |
| Error pattern store | Never written |
| Conductor bandit | ACP retries are fixed-count |
| Tier progression (D1/D2/D3) | Never triggered |
| Section effectiveness | Never updated |
| Curriculum difficulty model | Never updated |
| Lookahead router | Never applied |

---

## 5. The Cascade Router in Detail

The router already learns from ACP. What it doesn't yet do is SELECT the model. Filling the
selection gap closes the main cybernetic loop for ACP.

### Current ACP cascade loop (observe-only):

```
User picks model in Zed dropdown
     ↓
bridge_events.rs resolves slug
     ↓
Dispatch runs with user-chosen model
     ↓
record_cascade_observation() called
     ↓ (spawn_blocking → load_or_new → observe → save)
CascadeRouter.observe(context_vec, model_idx, reward)
     ↓
cascade-router.json updated on disk
```

### What needs to happen (full observe + select loop):

```
bridge_events.rs builds acp_routing_context()
     ↓
CascadeRouter::load_or_new() from disk (cached in AppState if roko serve is running)
     ↓
router.select_for_context(ctx, cfactor=None, agent_id=None)
     ↓  (Stage 1: static table / Stage 2: confidence / Stage 3: LinUCB)
Returns CascadeModel { primary: "claude-sonnet-4-5", fallback: [...] }
     ↓
ACP uses router-selected model (not dropdown)
     ↓
Dispatch runs
     ↓
record_cascade_observation() with actual outcome
```

### Bandit algorithm (Stage 3: LinUCB):

From `cascade/types.rs` and `model_router.rs`:
- **Arms**: one per model slug (e.g. claude-haiku-4-5, claude-sonnet-4-5, claude-opus-4-6)
- **Context vector**: 17 dimensions (task tier, complexity, familiarity, failures, load, etc.)
- **Reward**: `compute_acp_reward()` → `0.8 + latency_bonus + token_bonus`, clamped to 1.0
- **Update**: ridge regression `theta += learning_rate * (reward - prediction) * context`
- **UCB**: `predicted_reward + alpha * sqrt(context' * A_inv * context)` where alpha controls exploration/exploitation tradeoff

---

## 6. Priority-Ordered Implementation Plan

### Priority 1: Auto-dream after N episodes (1 day)

**Impact:** High. This is the main path from raw ACP usage to durable knowledge. Without it, ACP users accumulate episodes that never become reusable context.

**Trigger point:** After `append_acp_episode()` returns in `bridge_events.rs:1388`.

**Rust sketch:**

```rust
// In bridge_events.rs, after append_acp_episode() call at line ~1388:
if let Some(config) = roko_config_for_logging.learning.as_ref() {
    if config.dream_on_completion {
        maybe_trigger_acp_dream(
            workdir_for_logging.clone(),
            config.min_episodes_for_dream,
        );
    }
}

fn maybe_trigger_acp_dream(workdir: PathBuf, min_episodes: usize) {
    // Non-blocking: spawn as background task.
    tokio::spawn(async move {
        let episodes_path = workdir.join(".roko").join("episodes.jsonl");
        let episodes = match EpisodeLogger::read_all_lossy(&episodes_path).await {
            Ok(eps) => eps,
            Err(_) => return,
        };
        let dreams_dir = workdir.join(".roko").join("dreams");
        let last_report = roko_dreams::runner::load_latest_dream_report(&dreams_dir)
            .ok().flatten();
        let cutoff = last_report.and_then(|r| r.processed_through.or(Some(r.started_at)));
        let new_count = episodes.iter()
            .filter(|ep| cutoff.is_none_or(|ts| ep.timestamp > ts))
            .count();
        if new_count < min_episodes {
            return;
        }
        tokio::task::spawn_blocking(move || {
            let config = roko_dreams::DreamLoopConfig {
                auto_dream: true,
                idle_threshold_mins: 0,
                min_episodes_for_dream: min_episodes,
                agent: roko_dreams::DreamAgentConfig {
                    command: "claude".to_string(),
                    args: vec![],
                    model: None,
                    bare_mode: false,
                    effort: "low".to_string(),
                    fallback_model: None,
                    timeout_ms: 120_000,
                    env: vec![],
                },
            };
            let mut runner = roko_dreams::DreamRunner::new(workdir.clone(), config);
            if let Ok(report) = runner.consolidate_now() {
                tracing::info!(
                    processed = report.processed_episodes,
                    knowledge = report.knowledge_entries_written,
                    "ACP: auto-dream consolidation complete"
                );
            }
        });
    });
}
```

### Priority 2: Episode distillation hook (2 hours)

**Impact:** High. Without distillation, episodes never become knowledge entries. The hook already exists (`spawn_episode_distillation()`); it just needs to be called after `append_acp_episode()`.

**Rust sketch:**

```rust
// In bridge_events.rs, after EpisodeLogger::append() call at line ~422:
// (already inside append_acp_episode())

// After logger.append(&episode).await:
roko_neuro::spawn_episode_distillation(
    workdir.to_path_buf(),
    episode.clone(),
    None, // uses default ModelCallService from workdir config
);
```

**Note:** `spawn_episode_distillation()` is already defined in `crates/roko-neuro/src/episode_completion.rs`. It is already `pub`. This is literally a 3-line change.

### Priority 3: Cascade router selection (1 day)

**Impact:** Very high. Closes the cybernetic loop for model routing. Currently the router learns but is never consulted.

**Design:** ACP should check the router BEFORE falling back to user dropdown. When the router is in Stage 1 (< 50 observations), it returns the static table which respects user configuration. So this is safe even for cold-start users.

**Rust sketch (in bridge_events.rs, before the dispatch):**

```rust
// Add a function to optionally resolve the model from the cascade router:
fn cascade_selected_model(
    roko_config: &RokoConfig,
    workdir: &Path,
    model_key: &str,
    mode: &str,
    effort: &str,
    prompt: &str,
) -> Option<ResolvedModel> {
    let router_path = workdir.join(".roko").join("learn").join("cascade-router.json");
    if !router_path.exists() {
        return None; // cold start: use user setting
    }
    let model_slugs = cascade_router_model_slugs(roko_config, model_key);
    if model_slugs.is_empty() {
        return None;
    }
    let router = CascadeRouter::load_or_new(&router_path, model_slugs);
    let ctx = acp_routing_context(mode, prompt, effort);
    let selection = router.select(&ctx, None, None)?;
    let resolved = resolve_model(roko_config, &selection.primary.slug);
    if resolved.slug.is_empty() { None } else { Some(resolved) }
}
```

### Priority 4: Efficiency events (4 hours)

**Impact:** Medium. Enables the `/api/learn/efficiency` dashboard to show real ACP data, feeds the section effectiveness bandit, and enables C-Factor computation.

**Rust sketch (in bridge_events.rs, after dispatch completes):**

```rust
// After record_cascade_observation() at line ~1406:
if let Ok(sr) = &stream_result {
    let efficiency_path = workdir_for_logging.join(".roko").join("learn").join("efficiency.jsonl");
    let evt = AgentEfficiencyEvent {
        agent_id: session.session_id.clone(),
        role: session.config_state.agent_mode.clone(),
        backend: resolved_for_logging.provider_kind.label().to_string(),
        model: resolved_for_logging.slug.clone(),
        plan_id: "acp".to_string(),
        task_id: prompt_text_for_logging.chars().take(40).collect(),
        attempt_id: uuid::Uuid::new_v4().to_string(),
        input_tokens: sr.usage.as_ref().map(|u| u.input_tokens).unwrap_or(0),
        output_tokens: sr.usage.as_ref().map(|u| u.output_tokens).unwrap_or(0),
        cost_usd: cost_override.unwrap_or(0.0),
        wall_time_ms: dispatch_started.elapsed().as_millis() as u64,
        duration_ms: dispatch_started.elapsed().as_millis() as u64,
        gate_passed: dispatch_succeeded,
        outcome: if dispatch_succeeded { "success".to_string() } else { "failure".to_string() },
        timestamp: chrono::Utc::now().to_rfc3339(),
        // ... other fields default to 0/empty
        ..AgentEfficiencyEvent::default()
    };
    if let Some(parent) = efficiency_path.parent() {
        let _ = tokio::fs::create_dir_all(parent).await;
    }
    // Append JSONL
    if let Ok(line) = serde_json::to_string(&evt) {
        use tokio::io::AsyncWriteExt;
        if let Ok(mut f) = tokio::fs::OpenOptions::new().create(true).append(true)
            .open(&efficiency_path).await {
            let _ = f.write_all(format!("{line}\n").as_bytes()).await;
        }
    }
}
```

### Priority 5: Prompt experiment winners (1 hour)

**Impact:** Low-medium. Allows concluded A/B test winners to affect ACP prompts. The machinery already exists; it just needs to be called.

**Rust sketch (in bridge_events.rs, during config load):**

```rust
// In build_system_prompt_with_knowledge() or wherever system prompt is assembled:
// Read static-overrides.json and apply any override for the active workflow section.
let overrides_path = workdir.join(".roko").join("learn").join("static-overrides.json");
if let Ok(overrides_json) = std::fs::read_to_string(&overrides_path) {
    if let Ok(overrides) = serde_json::from_str::<HashMap<String, String>>(&overrides_json) {
        // Apply override for "system_prompt" section if present
        if let Some(override_text) = overrides.get("system_prompt") {
            // inject into system prompt construction
        }
    }
}
```

---

## 7. The Full Cybernetic Loop in ACP

Once all five priorities are wired, the loop is:

```
User types prompt in Zed
     │
     ▼
bridge_events.rs: load_roko_config()
     │
     ▼  [New: cascade router selection]
CascadeRouter::load_or_new() → router.select(ctx)
Returns: claude-haiku for simple, claude-sonnet for complex
     │
     ▼
query_dispatch_knowledge() → inject knowledge + playbooks into system prompt
     │                        (already wired, uses .roko/neuro/knowledge.jsonl)
     ▼
[New: apply experiment winners] → override system prompt sections if A/B test concluded
     │
     ▼
dispatch to selected model (workflow engine or tool loop)
     │
     ▼
Stream response back to Zed
     │
     ▼
append_acp_episode() → .roko/episodes.jsonl
     │
     ├──► [New] spawn_episode_distillation() → async: episode → KnowledgeEntry candidates
     │                                          → .roko/neuro/knowledge.jsonl
     │
     ├──► record_cascade_observation() → update CascadeRouter arm (already wired)
     │                                   → .roko/learn/cascade-router.json
     │
     ├──► [New] emit AgentEfficiencyEvent → .roko/learn/efficiency.jsonl
     │
     └──► [New] maybe_trigger_acp_dream() → async, if N new episodes accumulated:
                 DreamRunner::consolidate_now()
                 → cluster episodes → extract patterns → routing advice
                 → .roko/dreams/routing-advice.json
                 → promote staging → .roko/neuro/knowledge.jsonl
```

On the NEXT prompt in the same or a future session:
- `query_dispatch_knowledge()` finds the newly distilled knowledge entries
- `load_dream_routing_advice()` reads the updated routing patterns
- `CascadeRouter::select()` returns a better-calibrated model choice
- `apply_concluded_experiment_overrides()` uses winning prompt variants

**This is the cybernetic loop**: every prompt makes future prompts better.

---

## 8. What "Learning" Looks Like to the User

Once wired, the user experiences:

**Day 1 (cold start, < 50 dispatches):**
- Cascade router uses static table from `roko.toml` model configuration
- Episodes accumulate in `.roko/episodes.jsonl`
- No visible learning yet

**After 10–20 dispatches:**
- Distillation fires: patterns like "writing tests with tokio runtime requires async runtime setup" become knowledge entries
- Next prompt touching tests gets that knowledge injected automatically
- User sees: responses that understand the codebase better

**After 50 dispatches (confidence stage):**
- Cascade router starts preferring models with higher pass rates for specific task types
- Haiku auto-selected for simple file reads, Sonnet for implementation tasks
- User sees: faster/cheaper responses for simpler prompts, stronger models only when needed

**After 200 dispatches (UCB stage):**
- Full LinUCB routing: router considers task complexity, prior failures, queue depth
- Dream consolidation has run 3–5 times by now: `.roko/dreams/routing-advice.json` has patterns
- User sees: "When Bash commands fail in CI context, the Sonnet family succeeds 80% more"
- Future dispatches that match the pattern auto-route to Sonnet

**Ongoing:**
- Prompt experiments: if a system prompt variant is enabled, the winning variant is injected
- User sees: system gradually converges on prompts that pass the most gate checks
- `roko learn cascade-router` / `/learn-router` shows routing weights shifting over time

**Visible signals (available today):**
```
roko learn cascade-router        # Stage, observations, model weights
roko learn efficiency            # Cost per task, token efficiency trends
roko learn episodes              # Recent episode log
roko knowledge stats             # Knowledge store entry counts by tier
```

---

## 9. Files to Modify

| File | Changes Needed |
|------|---------------|
| `crates/roko-acp/src/bridge_events.rs` | Add: distillation hook after `append_acp_episode()`, efficiency event emission, auto-dream trigger, cascade router selection, experiment winner application |
| `crates/roko-acp/src/session.rs` | Add: `[learning]` config key reading to enable dream/distillation thresholds |
| `crates/roko-acp/src/runner.rs` | Add: gate-failure error pattern recording from pipeline runs |

The most impactful change (distillation hook) is also the smallest change: 3 lines added after the existing `logger.append()` call in `append_acp_episode()`.

---

## 10. Notes on the ACP Architecture Gap

The root architectural gap is that ACP's stateless per-turn dispatch cannot maintain the long-running `LearningRuntime` state that the CLI uses. Two paths forward:

**Path A (short-term, practical):** Wire individual subsystems file-by-file. Load from disk, update, save back. This is what `record_cascade_observation()` already does. Apply the same pattern to efficiency events and distillation.

**Path B (long-term, correct):** The `roko serve` daemon is already running a persistent `CascadeRouter` in `AppState`. The ACP path (when running under `roko serve`) should read/write through the shared `AppState` rather than loading from disk on every turn. This eliminates the load-lock-save per-turn penalty and allows the in-memory cascade router to serve selection requests without disk I/O.

Path A is the pragmatic path for the next 2–4 days. Path B is the architectural target once `roko serve` is the standard ACP host.
