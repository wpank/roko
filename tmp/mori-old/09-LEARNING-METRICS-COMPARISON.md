# 09 - Learning, Metrics, and Observability: Mori vs Roko

## Executive summary

Mori's learning system was a 5,800 LOC monolith inside the orchestrator, tightly
coupled to the TUI, with flat-file persistence and no formal separation between
concerns. Roko's replacement is a 123,700 LOC dedicated crate (`roko-learn`) with
65 modules, 1,056 test assertions, and clear architectural layering. What Mori
demonstrated as a single production dashboard (F7:inspect) with seven panels, Roko
decomposes into formally defined subsystems -- many of which now exceed Mori's
production state in depth and correctness, though TUI visualization and the
end-to-end dogfood feedback loop remain residual.

---

## 1. Episode logging

### Mori

- **Schema**: 40+ fields per episode in a flat `Episode` struct.
  - Key fields: `plan_id`, `task_id`, `model`, `provider`, `gate_passed`,
    `cost_usd`, `duration_secs`, `iterations`, `error_signature`, `reflection`,
    `context_strategy`, `routing_band`, `routing_source`, `prompt_bytes`,
    `inline_context_bytes`, `context_pack_bytes`, `prompt_cache_hit`,
    `playbook_hits`, `research_prepass`, `fixture_keys_used`,
    `sidecars_started`, `file_intel_entries`, `warning_entries`,
    `wave_context_entries`, `error_pattern_hits`, `fixture_predictions`.
- **Storage**: Append-only JSONL at `.mori/memory/episodes.jsonl`.
  Non-blocking writes, `warn` on failure.
- **Read**: Synchronous, full-file scan -- loads all episodes into memory via
  `read_episodes(repo_root)`.
- **Production volume**: ~6,600 episodes accumulated.
- **Source**: `apps/mori/src/orchestrator/memory.rs` (487 lines for the Episode
  struct and JSONL I/O).

### Roko

- **Schema**: Richer `Episode` struct with 50+ fields including:
  - `kind` (agent_turn, gate, replan -- multi-event taxonomy);
  - `gate_verdicts: Vec<EpisodeGateVerdict>` (per-gate pass/fail with hashed signatures);
  - `usage: Usage` (input/output/cache_read/cache_write tokens, cost with and
    without cache, wall latency);
  - `hdc_fingerprint` (HDC vector fingerprint per episode for similarity clustering);
  - `extra: HashMap<String, Value>` (bounded to 16KB, forward-compatible);
  - `emotional_tag` (optional affect marker from the Daimon subsystem).
- **Storage**: Append-only JSONL via `EpisodeLogger` with `parking_lot::Mutex` for
  concurrent writers. Async append (`tokio::Mutex`) with `fsync` by default.
  Size-bounded `extra` field prevents unbounded growth.
- **Read**: Tolerant reader -- malformed lines produce `LoggerError::Parse` with
  line numbers; callers choose to skip or halt.
- **Template suggestion**: HDC similarity matching across recent episodes provides
  "similar past episode" retrieval (within 30 days, cosine > 0.7).
- **Production files**: `.roko/learn/episodes.jsonl.v2-legacy` (201KB),
  `.roko/episodes.jsonl` (root-level canonical log).
- **Source**: `crates/roko-learn/src/episode_logger.rs` (2,279 lines).

### Delta

| Dimension | Mori | Roko |
|-----------|------|------|
| Schema fields | ~40 | ~50+ |
| Gate detail | Single `gate_passed` bool | Per-gate verdict array with signatures |
| Cache accounting | `prompt_cache_hit` bool | Separate read/write/cold-cost fields |
| Similarity search | None | HDC fingerprint + cosine matching |
| Write safety | Fire-and-forget sync | `parking_lot` + `fsync` + bounded extras |
| Error tolerance | Silent skip of bad lines | Structured `Parse { line, source }` error |
| LOC | ~300 | 2,279 |

---

## 2. Playbook rules

### Mori

- **Format**: TOML at `.mori/memory/playbook.toml`.
- **Rule schema**: `PlaybookRule` with `id`, `trigger_files`, `trigger_tags`,
  `context`, `confidence`, `validated_count`, `preferred_model`,
  `preferred_provider`, `context_strategy`, `context_weight`,
  `reasoning_level`, `speed_priority`, `quality_profile`,
  `research_before_edit`, `fixture_keys`, `sidecar_requirements`.
- **Confidence dynamics**: boost +0.05 per validation, penalty -0.10 per
  contradiction. Prune below 0.2 (retained if >= 5 validations).
- **Learning cycle**: `run_learning_cycle` loads all episodes, clusters them
  by error signature / file prefix, matches rules to recent 50 episodes,
  updates confidence.
- **Production volume**: 98 rules total (learned + manual).
- **Trigger matching**: File-glob + tag intersection (OR semantics).
- **Source**: `apps/mori/src/orchestrator/pattern_learning.rs` (487 lines) +
  playbook types in `memory.rs`.

### Roko

- **Format**: JSON files in `.roko/learn/playbooks/`, one file per playbook.
  14 playbooks on disk: 4 manual ("compile-check-loop", "grep-before-write",
  "minimal-edit", "wire-not-build", "test-first") + 9 dream-generated
  playbooks from the Dream consolidation system.
- **Rule schema** (`playbook_rules.rs`): `Rule` with `rule_id`, `title`,
  `body`, `triggers` (5-way: file_globs, tags, categories, error_signatures,
  roles), `confidence`, `validations`, `contradictions`, `last_applied`,
  `created_at`, `source_episodes`, `balance`, `demurrage_rate`,
  `last_decay_at_ms`.
- **Confidence dynamics**: Same +0.05/−0.10 bounds, but also includes:
  - **Gesellian demurrage**: rules decay attention balance over time (hourly
    rate) and must be actively validated to replenish. Depleted rules are
    deprioritized in retrieval.
  - **Reflection admission**: post-gate reflections can generate playbook
    rule candidates that go through an admission pipeline.
- **Playbook sequences** (`playbook.rs`): Full `Playbook` with ordered
  `PlaybookStep` structs (index, description, action_kind,
  expected_signals). `PlaybookStore` persists and merges similar playbooks
  (80% step overlap triggers merge).
- **Extraction**: `extract_playbook_from_episode` mines tool-call sequences
  into new playbook candidates.
- **Trigger matching**: Five-way OR (file globs via `globset`, tags,
  categories, error signatures, roles) vs Mori's two-way.
- **Source**: `crates/roko-learn/src/playbook.rs` (1,891 lines) +
  `playbook_rules.rs` (2,589 lines) = 4,480 lines.

### Delta

| Dimension | Mori | Roko |
|-----------|------|------|
| Trigger dimensions | 2 (files, tags) | 5 (files, tags, categories, errors, roles) |
| Attention decay | None | Gesellian demurrage with hourly rate |
| Step sequences | Rule bodies only | Ordered PlaybookStep with action_kind + expected_signals |
| Playbook merging | None | Automatic merge at 80% step overlap |
| Dream-generated rules | None | Dream consolidation produces playbook candidates |
| Production rules | 98 | 14 (lower count, but structured sequences) |
| LOC | ~487 | 4,480 |

---

## 3. Model routing

### Mori

- **Architecture**: `ProviderHealthTracker` (in-memory, thread-safe via atomics)
  + `ProviderMetrics` computed from episode history.
- **Routing strategy**: Highest pass rate among healthy providers with >= 5
  episodes. Preferred provider honored if healthy.
- **Health tracking**: Consecutive failure counter. Unhealthy after 3 failures.
  Recovery after 120s cooldown.
- **Routing display**: F7:inspect shows routing coverage % (92%), routed/total
  tasks (1.6k/1.7k), rich route hints (model, provider, research).
- **No bandit**: Static preference, no exploration/exploitation tradeoff.
- **Source**: `apps/mori/src/orchestrator/provider_routing.rs` (298 lines).

### Roko

- **Architecture**: `CascadeRouter` -- three-stage cascade:
  1. **Static** (< 50 observations): hardcoded role-to-model table.
  2. **Confidence** (50-200): empirical pass rates + confidence intervals.
  3. **UCB1** (> 200): Full `LinUCB` contextual bandit.
- **Context vector**: 18-dimensional feature space encoding task category
  (8-dim one-hot), complexity band, iteration count, agent role (4-dim hash),
  crate familiarity, prior failure flag, bias term, cache affinity.
- **Alpha decay**: Exploration parameter decays from 1.0 to 0.05 over 200
  observations (`0.05 + 0.95 * exp(-obs/60)`).
- **Fallback chains**: Each primary model carries an ordered fallback chain
  and a context-overflow fallback for when the prompt exceeds the window.
- **Health tracking**: `ProviderHealthRegistry` with three-state circuit
  breaker (Closed/Open/HalfOpen), classified error types (RateLimit,
  AuthFailure, Timeout, ServerError, ContentPolicy, ContextOverflow),
  rolling failure window (last 20), variable cooldown per error class.
- **Pareto frontier**: Periodic recomputation of cost-quality Pareto frontier
  across models to down-weight dominated arms during UCB scoring.
- **Stage transitions**: Tracked and persisted with timestamps in
  `stage_tracking`.
- **Shadow evaluation**: Optional free-tier Gemini runner for shadow quality
  comparisons.
- **Behavioral modulation**: DaimonPolicy (affect engine), conductor load,
  temperament, cache affinity, and knowledge hints all feed into routing.
- **Persistence**: `cascade-router.json` (70KB on disk), crash-safe with
  WAL support.
- **Audit log**: `routing_log.rs` appends every decision with candidate
  scores and disqualification reasons. Outcome fields populated post-task.
- **Source**: `crates/roko-learn/src/cascade_router.rs` (4,055 lines) +
  `model_router.rs` (2,404 lines) + `cascade/` submodules (types, helpers,
  persistence, tests) + `provider_health.rs` (2,182 lines) = ~12,000+ lines.

### Delta

| Dimension | Mori | Roko |
|-----------|------|------|
| Routing algorithm | Highest pass rate | Three-stage cascade (Static/Confidence/LinUCB) |
| Context features | None | 18-dimensional feature vector |
| Exploration | None | UCB exploration with alpha decay |
| Fallback chains | None | Ordered per-model fallback + context overflow |
| Circuit breaker | Binary (healthy/unhealthy) | Three-state with typed error classes |
| Pareto pruning | None | Periodic cost-quality frontier computation |
| Audit trail | None | Full routing decision log with candidate scores |
| Shadow evaluation | None | Optional free-tier Gemini shadow runs |
| Affect modulation | None | DaimonPolicy, temperament, operating frequency |
| LOC | 298 | ~12,000+ |

---

## 4. Efficiency and cost tracking

### Mori

- **EfficiencyBucket**: Aggregated by provider, model, strategy, routing band,
  routing source, fixture key, sidecar. Fields: episodes, passed, avg duration,
  avg cost, avg iterations, avg retries, avg tokens, avg prompt bytes, avg
  inline context bytes, avg context pack bytes, avg playbook hits.
- **EfficiencySnapshot**: Generated periodically from episodes. Written to
  `.mori/memory/efficiency.json`. History appended to
  `.mori/memory/efficiency-history.jsonl`.
- **Refresh cadence**: Triggered when 2+ new episodes accumulate since last
  refresh.
- **TUI display**: F7 shows per-model, per-provider, per-strategy best
  summaries (e.g. "claude 89% pass, 120s avg, 1.2 retry, $0.340/run").
- **Prompt stats**: Average prompt bytes, inline context bytes, context pack
  bytes shown in the inspect panel.
- **Source**: ~200 lines in `memory.rs`.

### Roko

- **AgentEfficiencyEvent**: Rich per-turn event (80+ fields) including:
  - Per-section token attribution (`prompt_sections: Vec<PromptSectionMeta>`
    with name, tokens, priority, was_truncated, was_dropped);
  - Tool utilization (`tool_calls: Vec<ToolCallMeta>` with tool_name,
    duration_ms, result_tokens, succeeded, advanced_task, was_redundant,
    error_category);
  - Timing (wall_time, time_to_first_token, warm_start flag);
  - Cost with and without cache discount;
  - Reasoning token accounting;
  - Attempt-level correlation IDs.
- **PromptEfficiencyScore + Grade**: A-D letter grading for prompt assembly
  efficiency, with per-section contribution analysis.
- **RoleCostProfile**: Aggregate cost profile per agent role.
- **Efficiency summaries JSONL**: `.roko/learn/efficiency-summaries.jsonl` (25KB).
- **Efficiency events JSONL**: `.roko/learn/efficiency.jsonl` (61KB).
- **Cost tracking**: Separate `costs.jsonl` (8KB) with `CostRecord` and
  `CostsDb` in-memory cost database; `cost_projection.rs` for pre-dispatch
  budget estimation; `cost_table.rs` for per-model cost rates.
- **Budget guardrails**: `budget.rs` -- enforced USD budget limits per plan
  and per session.
- **Source**: `crates/roko-learn/src/efficiency.rs` (1,646 lines) +
  `costs_log.rs` (434 lines) + `costs_db.rs` + `cost_table.rs` +
  `cost_projection.rs` + `budget.rs`.

### Delta

| Dimension | Mori | Roko |
|-----------|------|------|
| Per-section attribution | None | Full PromptSectionMeta with truncation/drop tracking |
| Tool call analysis | None | Per-call metadata with redundancy detection |
| Prompt grading | None | A-D letter grade with section scores |
| Cache regret | None | cost_usd_without_cache for regret accounting |
| Budget enforcement | None | Per-plan and per-session USD limits |
| Cost projection | None | Pre-dispatch budget estimation |
| Time-to-first-token | None | Tracked per event |
| LOC | ~200 | ~3,500+ |

---

## 5. Prompt logging and section analysis

### Mori

- **PromptLogEntry**: UUID, timestamp, plan, task, role, context strategy,
  total tokens (cl100k via tiktoken), per-section breakdown (`LogSection`
  with name, tokens, chars), full prompt text, context pack bytes, inline
  context bytes, cache hit, playbook hits, research prepass used, verify
  artifacts fresh.
- **Tokenizer**: Lazy-init `tiktoken_rs::CoreBPE` (cl100k), fallback `len/4`.
- **Section splitting**: By `## ` heading lines.
- **Storage**: One JSON file per invocation in `.mori/memory/prompt-logs/`.
- **Source**: `apps/mori/src/orchestrator/prompt_log.rs` (220 lines).

### Roko

- **PromptSectionMeta**: More structured -- name, tokens, priority (compose-assigned
  0-255), was_truncated, was_dropped.
- **Section-level outcome tracking**: `section_outcome.rs` tracks per-section
  success/failure correlation to feed adaptive prompt assembly policy.
- **Section effect analysis**: `section_effect.rs` measures which prompt
  sections correlate with task success.
- **No separate prompt log files**: Section metadata is embedded in
  `AgentEfficiencyEvent` rather than standalone files.
- **Source**: `section_outcome.rs` + `section_effect.rs` + prompt section
  fields in `efficiency.rs` -- ~1,000+ lines.

### Delta

| Dimension | Mori | Roko |
|-----------|------|------|
| Tokenizer | tiktoken cl100k | Token counts from provider response |
| Full prompt text stored | Yes (one file per invocation) | No -- section metadata only |
| Section priority | None | 0-255 compose-assigned priority |
| Truncation tracking | None | was_truncated, was_dropped per section |
| Section-outcome correlation | None | section_outcome + section_effect analysis |

---

## 6. Pattern discovery and reflection

### Mori

- **Episode clustering**: `cluster_episodes()` groups by error signature
  (failures) or directory prefix (successes). Minimum 3 episodes per cluster.
  Reports success rate, common files/tags, best model/provider, avg cost.
- **Reflection**: `spawn_reflection()` calls Claude Haiku (~$0.01/call) to
  generate structured reflections on gate failures (What failed / Why / What
  to try / Files to focus on). Deduplicates by error line. Stored in
  `IterationMemory` per plan. Non-blocking via `tokio::spawn`.
- **Source**: `pattern_learning.rs` (487 lines) + `reflection.rs` (227 lines).

### Roko

- **Pattern mining**: `PatternMiner` with trigram extraction across episode
  action sequences. Configurable support threshold and confidence floor.
  HDC-based clustering via k-medoids. `EpisodeView` trait decouples mining
  from the concrete Episode type.
- **Hindsight relabeling**: `HindsightRelabeler` scans a 30-day window for:
  - **Regression**: later gate failure invalidates earlier success;
  - **Successful reuse**: later success reused approach from a failed episode;
  - **Heuristic falsified**: a rule sourced from an episode was contradicted.
  Produces immutable `EpisodeAdjustment` corrections (append-only).
- **Post-gate reflection**: `post_gate_reflection.rs` generates structured
  reflections with admission status for playbook rule candidate generation.
- **Error enrichment**: `error_enrichment.rs` preprocesses noisy gate failures
  into retry-ready diagnoses.
- **Error pattern store**: `error_pattern_store.rs` persists discovered error
  patterns for cross-episode matching.
- **Anomaly detection**: `AnomalyDetector` with three detectors:
  - Prompt loop (same hash 5x in 20 prompts);
  - Cost spike (z-score > 3.0 vs EWMA baseline);
  - Quality drift (recent vs earlier window comparison).
- **Source**: `pattern_discovery.rs` (977 lines) + `hindsight.rs` (210 lines) +
  `post_gate_reflection.rs` + `error_enrichment.rs` + `error_pattern_store.rs` +
  `anomaly.rs` (816 lines) -- ~3,000+ lines.

### Delta

| Dimension | Mori | Roko |
|-----------|------|------|
| Clustering approach | Error signature / file prefix | Trigram mining + HDC k-medoids |
| Hindsight correction | None | 30-day relabeling with three adjustment kinds |
| Anomaly detection | None | Prompt loop, cost spike, quality drift |
| Error enrichment | None | Gate failure preprocessing pipeline |
| LLM-generated reflection | Yes (Haiku calls) | Structured post-gate reflection module |
| Pattern abstraction | None | EpisodeView trait for composable mining |

---

## 7. Telemetry and observability infrastructure

### Mori

- **F7:inspect tab**: One TUI view (`tui/views/context.rs`, 1,001 lines)
  showing three columns: Servers/Roots, AST Index, and Tools/Learning.
- **LearningSnapshot**: ~40 aggregated fields computed by
  `compute_learning_snapshot()` (scanning all episodes, playbooks, iteration
  memories, task TOMLs, registries, efficiency snapshots).
- **Displayed metrics**:
  - Episodes: 6.6k total, ok/fail split
  - Playbook: 98 rules (learned vs manual)
  - Routing: 92% coverage (1.6k/1.7k tasks)
  - Rich route hints: model, provider, research, playbook counts
  - Prompt stats: avg prompt, inline, pack bytes
  - Artifact counts: research, integration, dependency, fixture manifests
  - Registries: 1.1k deps, 612 fixtures, 2.9k sidecars
  - History: per-plan reflection entries
  - Best-of summaries: per-model, per-provider, per-strategy with
    pass rate / duration / retry / token / cost / run count
  - Knowledge utilization: file-intel, warnings, wave-ctx, err-pat
- **No formal telemetry layer**: Metrics are computed on-demand by scanning
  disk files. No event-bus architecture or circuit breaker.
- **Source**: `memory.rs` (3,107 lines) + `tui/views/context.rs` (1,001 lines).

### Roko

- **Telemetry Lens system (E33 -- 9/9 tasks complete)**:
  - `Lens` trait in `roko-core/src/obs/lens.rs`: read-only telemetry adapter
    projecting live state into `LensSnapshot` structs.
  - Three lens specializations: Collector, Transform, Export.
  - Six scope levels: Component, Graph, Agent, Space, Lens (chain), Global.
  - 11 built-in lens implementations: `CollectorLens`, `TokenUsageLens`,
    `LatencyLens`, `CostLens`, `AnomalyLens`, `CollectiveIntelligenceLens`,
    `EfficiencyLens`, `QualityLens`, `TrendLens`, `UsageLens`, health lens.
  - `LensExecutor` in `roko-runtime` routes events to named Lens implementations.
  - **Circuit breaker**: Per-lens overhead budget (default 1% of observed
    operation time). Three stages: normal --> sampled (50%) --> disabled.
    Configurable sample/disable thresholds.
  - **Backpressure**: Bounded delivery queue (1,024 capacity) with
    drop-oldest overflow policy.
  - **39 production event variants**: All production telemetry flows through
    typed `ObservableEvent` variants.
  - **StateHub integration**: Lens snapshots feed `DashboardEvent`
    -> `watch::Sender` -> TUI/SSE/WebSocket.
  - **Restart-durable history**: Projection history survives process restarts.
  - **Configurable retention**: 7-day time-based retention with resolution queries.
- **Periodic telemetry sampling**: `PeriodicObserver` in `roko-serve` samples
  shared metrics every 30s, writes rotation-bounded JSONL.
- **REST/SSE routes**: ~317 HTTP routes in `roko-serve` expose telemetry,
  learning state, and lens snapshots via REST + SSE.
- **Source**: `roko-core/src/obs/lens.rs` + `roko-core/src/lens_circuit_breaker.rs` +
  `roko-core/src/lens_registry.rs` + `roko-runtime/src/lens_executor.rs` -- the
  observability infrastructure alone exceeds Mori's entire learning LOC.

### Delta

| Dimension | Mori | Roko |
|-----------|------|------|
| Architecture | On-demand file scan | Event-driven Lens trait + LensExecutor |
| Lens implementations | None | 11 built-in, extensible |
| Circuit breaker | None | Per-lens overhead budget with sampling/disable |
| Event variants | None | 39 typed ObservableEvent kinds |
| Delivery model | Synchronous | Bounded async queues with backpressure |
| External exposure | TUI only | REST, SSE, WebSocket via roko-serve |
| History durability | Per-run only | Restart-durable with configurable retention |
| Scope hierarchy | Flat | 6-level (Component/Graph/Agent/Space/Lens/Global) |

---

## 8. A/B experiments

### Mori

- **None**: No A/B testing framework for models or prompts. Routing was static
  preference with pass-rate fallback.

### Roko

- **Model experiments** (`model_experiment.rs`, 781 lines): `ModelExperiment`
  with multiple `ModelVariant` entries, per-variant stats (trials, successes,
  cost, tokens, duration, pass rate, cost per success), minimum effect size,
  auto-conclusion.
- **Prompt experiments** (`prompt_experiment.rs`, 2,326 lines): `PromptVariant`
  with section-level A/B testing. Bandit-driven variant selection with
  under-sampling exploration. `ExperimentStore` persists experiments. Auto-
  promotion of concluded winners. Chi-squared p-value for statistical
  significance.
- **Runner integration**: Runner attempts durably assign and replace canonical
  prompt sections, bind exact prelaunch prompts, and idempotently settle
  from terminal facts.
- **Persistence**: `.roko/learn/experiments.json`, `.roko/learn/experiment-winners.json`.

---

## 9. C-Factor and collective intelligence

### Mori

- **None**: No collective intelligence scoring.

### Roko

- **CFactor** (`cfactor.rs`, 2,189 lines): Composite 0.0-1.0 score with
  component breakdown. Features:
  - Per-agent leave-one-out contribution scores (positive = raises collective
    quality, negative = drags it down);
  - Pathology detection in the episode stream;
  - Multi-window trend analysis;
  - Governance recommendations (AdjustModel, AdjustGate, IncreaseDiversity,
    ReduceParallelism);
  - `AgentDispatchBias`: PreferStronger / PreferCheaper / Neutral routing
    directives derived from C-Factor contributions.

---

## 10. Advanced learning features unique to Roko

These subsystems have no Mori equivalent:

| Subsystem | Module | Description |
|-----------|--------|-------------|
| Contextual bandit | `contextual_bandit.rs` | Model-selection feedback with reward recording |
| Active inference | `active_inference.rs` | Belief state updating for tier routing |
| Bayesian confidence | `bayesian_confidence.rs` | Beta-Binomial conjugate model (AS-07) |
| Calibration policy | `calibration_policy.rs` | Bus-backed predict-publish-correct loop |
| Curriculum ordering | `curriculum.rs` | Task scheduling based on learning state |
| Verdict scorer | `verdict_scorer.rs` (1,549 lines) | Gate-verdict re-entry scoring |
| HDC clustering | `hdc_clustering.rs` | k-medoids clustering on HDC vectors |
| HDC fingerprint | `hdc_fingerprint.rs` | Per-episode HDC vector computation |
| WAL | `wal.rs` (302 lines) | Write-ahead log for crash-safe state |
| Pareto frontier | `pareto.rs` | Cost-quality Pareto frontier computation |
| Quality judge | `quality_judge.rs` | Automated quality assessment |
| Regression detector | `regression.rs` | Regression detection across episodes |
| Bandits (EWC) | `bandits.rs` | Elastic Weight Consolidation regularizer |
| Oracles | `oracles/` (coding, research, chain, witness, selector) | Domain-specific evaluation |
| Latency tracker | `latency.rs` | Rolling EMA + percentile tracking |
| Heuristics | `heuristics.rs` | Worldview and research provenance shells |
| Routing extras | `routing_extras.rs` | Lookahead and calibration around cascade |
| Skill library | `skill_library.rs` (2,756 lines) | Structured skill registry |
| Task metric | `task_metric.rs` (698 lines) | Per-task metric aggregation |
| Model call feedback | `model_call_feedback.rs` | Direct model-call outcome recording |
| Provider-model outcome | `provider_model_outcome.rs` | Provider/model pass-rate telemetry |
| Event subscriber | `event_subscriber.rs` | Runtime event fan-in to learning subsystems |
| Feedback service | `feedback_service.rs` (1,074 lines) | Unified workflow telemetry sink |
| Conductor | `conductor.rs` | Learned intervention policy for retries/aborts |
| Context pack cache | `context_pack_cache.rs` | Cached composed prompts by task fingerprint |

---

## 11. Fixture and dependency tracking

### Mori

- **Fixture registry**: TOML at `.mori/memory/fixtures.toml`. Scanned from
  per-plan `fixture-manifest.toml` files. 612 fixtures, 1.1k dependency
  entries, 2.9k sidecar starts.
- **Live fixture display**: F7 shows active fixtures with PID, key, uptime.
- **Dependency registry**: TOML at `.mori/memory/dependencies.toml`.
- **Support artifact freshness**: Checks mtime of research.md,
  dependency-manifest.toml, etc. against plan.md / tasks.toml.
- **Source**: ~200 lines in `memory.rs` for registries.

### Roko

- No direct equivalent of Mori's fixture/dependency registry system. The
  equivalent functionality is distributed across:
  - Plugin manifests and dependency graphs in `roko-plugin`;
  - Process supervision in `roko-runtime/ProcessSupervisor`;
  - Tool policy in `AgentContract`.
- The fixture lifecycle is less centralized than Mori's. This is an area where
  Mori's approach (structured registry with live TUI display) was more
  immediately practical for the dogfood workflow.

---

## 12. TUI/dashboard visibility

### Mori

- **F7:inspect** is a dense, production-tested single view with three columns:
  1. **Servers/Roots**: MCP config paths, backend status, worktree routing;
  2. **AST Index**: file/symbol/reference counts, resolution %, density;
  3. **Tools/Learning**: episodes, playbook, routing, hints, prompt stats,
     registries, knowledge utilization, history, per-strategy summaries,
     tool calls, top tools.
- **Inline gauges**: Semantic color bars for routing coverage and resolution %.
- **Live route display**: Shows the routing metadata for the currently-selected
  task (model, provider, research, playbook hits).
- **Live efficiency metrics**: Per-agent-instance metrics displayed inline.
- **Token burn sparklines**: Relocated from dashboard to inspect tab.

### Roko

- **F7 in roko TUI**: The dashboard exists (`crates/roko-cli/src/tui/`, F1-F10
  tabs), but the learning/metrics visualization is less mature than Mori's
  production F7 panel.
- **CLI inspection**: `roko learn all/router/experiments/efficiency/episodes`
  subcommands provide textual inspection of all learning state.
- **roko-serve routes**: HTTP REST + SSE routes expose the same data
  programmatically -- potentially richer than TUI alone, but less immediately
  visible during development.
- **Named surfaces (E37)**: Typed projections (Workbench, Inbox, Canvas,
  Minimap, Autonomy) with StateHub-backed routes, but full TUI rendering
  remains a product residual.

### Delta

Mori's F7:inspect was a production-hardened TUI panel that showed everything in
one glance. Roko has the data and more -- it just hasn't fully converged on
an equivalent single-pane-of-glass for the interactive developer experience.
The data is accessible via CLI subcommands and HTTP routes, but the ratatui
rendering of the full learning state is still catching up.

---

## 13. Scale comparison

| Metric | Mori | Roko |
|--------|------|------|
| Learning-related LOC | ~5,800 (orchestrator modules) | ~123,700 (roko-learn crate) |
| Test assertions | ~266 (orchestrator-wide) | ~1,056 (roko-learn only) |
| Modules/files | 6 files in orchestrator/ | 65 modules in roko-learn/src/ |
| Persisted files | ~8 (.mori/memory/) | ~50 (.roko/learn/) |
| Cascade router state | None | 70KB JSON with LinUCB matrices |
| Episode schema fields | ~40 | ~50+ |
| Routing context features | 0 | 18 |
| Lens implementations | 0 | 11 |
| Event types | 0 | 39 |
| A/B experiment framework | None | Model + prompt experiments |

---

## 14. What Mori had that Roko should preserve

1. **Production volume proof**: 6,600 episodes, 98 playbook rules, 92%
   routing coverage, 1.1k dependencies, 612 fixtures. These are real numbers
   from production use, not test fixtures. Roko has all the machinery but the
   comparable production run has not yet completed (dogfood re-verification
   is pending).

2. **Single-pane visibility**: F7:inspect showed everything at a glance.
   Roko's learning data is fragmented across CLI subcommands, HTTP routes,
   and partially-rendered TUI tabs.

3. **Fixture lifecycle visibility**: Live fixture display (PID, key, uptime)
   directly in the TUI was immediately useful for debugging service-dependent
   tasks.

4. **Prompt text logging**: Mori stored full prompt text per invocation.
   Roko tracks section metadata but not the full text, which makes offline
   analysis harder.

5. **Support artifact freshness checking**: Mori's mtime-based freshness
   check for research.md, integration.md, etc. was a simple but effective
   way to detect stale plan context.

---

## 15. Key files

### Mori

| File | Path | LOC |
|------|------|-----|
| Episode + playbook + efficiency types | `/Users/will/dev/uniswap/bardo/apps/mori/src/orchestrator/memory.rs` | 3,107 |
| Pattern learning + clustering | `/Users/will/dev/uniswap/bardo/apps/mori/src/orchestrator/pattern_learning.rs` | 487 |
| Provider routing + health | `/Users/will/dev/uniswap/bardo/apps/mori/src/orchestrator/provider_routing.rs` | 298 |
| Prompt logging | `/Users/will/dev/uniswap/bardo/apps/mori/src/orchestrator/prompt_log.rs` | 220 |
| Gate failure reflection | `/Users/will/dev/uniswap/bardo/apps/mori/src/orchestrator/reflection.rs` | 227 |
| F7:inspect TUI view | `/Users/will/dev/uniswap/bardo/apps/mori/src/tui/views/context.rs` | 1,001 |

### Roko

| File | Path | LOC |
|------|------|-----|
| Cascade router | `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/cascade_router.rs` | 4,055 |
| LinUCB model router | `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/model_router.rs` | 2,404 |
| Episode logger | `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/episode_logger.rs` | 2,279 |
| Playbook rules | `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/playbook_rules.rs` | 2,589 |
| Playbook sequences | `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/playbook.rs` | 1,891 |
| Efficiency events | `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/efficiency.rs` | 1,646 |
| Provider health | `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/provider_health.rs` | 2,182 |
| C-Factor | `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/cfactor.rs` | 2,189 |
| Prompt experiments | `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/prompt_experiment.rs` | 2,326 |
| Pattern discovery | `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/pattern_discovery.rs` | 977 |
| Anomaly detection | `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/anomaly.rs` | 816 |
| Lens trait (roko-core) | `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/obs/lens.rs` | -- |
| Lens executor (roko-runtime) | `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/lens_executor.rs` | -- |
| Learn data directory | `/Users/will/dev/nunchi/roko/roko/.roko/learn/` | ~50 files |
