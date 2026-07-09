# Layer 6+: Architecture implementation

**Goal**: The full set of architecture doc implementations, organized by phase so agents can execute them in dependency order.

**Depends on**: Layer 5 (self-hosting loop operational)

---

## Overview

21 architecture docs. 13 phases (A-M) from the roadmap. 41 DeFi batches. Plus a parallel track of 10 orchestrator gap tasks (P0, runs alongside Phases A-C). Grouped by dependency order. Each phase lists the architecture doc reference, number of tasks, estimated effort, dependencies, and key crates affected.

## Audit addendum -- 2026-04-25

This plan was originally reconciled on 2026-04-24. A fresh repo audit on
2026-04-25 found several components that now exist or are more complete than
the plan text implies. The implementation rule for every phase is:

**extend existing modules before creating new ones.** If a task below says
"create `x.rs`" but an equivalent module exists, keep the existing module and
add the missing behavior, tests, route wiring, or runtime integration there.

### Current-code corrections

| Area | Current code reality | Planning consequence |
|------|----------------------|----------------------|
| Extension system | `roko-core/src/extension.rs` already defines `ExtensionLayer`, decision enums, `ExtensionMeta`, and a default-hook `Extension` trait | Do not redefine the trait from scratch. Remaining work is async/context parity, timeout enforcement, loader dependency resolution, built-in extensions, and orchestrator/runtime hook invocation. |
| Heartbeat/gating | `roko-runtime/src/heartbeat.rs`, `heartbeat_attention.rs`, `heartbeat_probes.rs`, `theta_consumer.rs`, and `delta_consumer.rs` already contain `CorticalState`, `HeartbeatPolicy`, `FrequencyScheduler`, prediction-error computation, `gate_tier`, 16-probe infrastructure, VCG attention, theta, and delta consumers | Do not create duplicate adaptive-clock or gating primitives. Remaining work is a canonical 9-step `TickPipeline` facade, concurrent gamma/theta/delta task wiring, integration with agent execution, persistent snapshots, reflex promotion/demotion, and replacing delta stubs with real `roko-dreams` calls. |
| Auth | `roko-serve/src/jwks.rs` performs Privy JWKS verification using ES256 keys, and `routes/middleware.rs` calls the cache | Treat real JWKS verification as mostly implemented. Remaining work is route-level scope coverage tests, device flow, and any missing issuer/audience/stale-cache edge tests. |
| Agent tokens | `routes/agents.rs` exposes `GET/POST /api/agents/{id}/token`, and `state.rs` can rotate token hashes | Do not rebuild token issue/status. Remaining work is revoke, refresh grace window, expiry configurability, and auth matrix tests. |
| Gateway | `routes/gateway.rs` already exposes completion, batch submit/status, models, and stats, with CascadeRouter selection and cost estimation | Keep gateway in `roko-serve`. Remaining work is the production request pipeline: exact cache, semantic cache, loop guard, output budget, tool pruning, convergence, fallback, accurate per-model stats, and non-placeholder cache hit metrics. |
| Connectors/feeds | `roko-core/src/connector.rs`, `roko-core/src/feed.rs`, `routes/connectors.rs`, and `routes/feeds.rs` already provide registries and CRUD | Do not recreate CRUD. Remaining work is async connector/feed traits, adapters, feed data storage, pagination, WS rooms, health probes, paid feed authorization, and dashboard subscription wiring. |
| Dreams | `roko-runtime/src/delta_consumer.rs` has trigger and reporting scaffolding, but NREM/REM/integration methods are explicit stubs | Keep the delta consumer, replace stub phase bodies with `roko-dreams` + `roko-learn` + `roko-neuro` integration. |
| Route surface | `roko-serve/src/routes/mod.rs` currently mounts status, jobs, heartbeats, plans, PRDs, research, subscriptions, templates, agents, learning, config, deployments, diagnosis, integrations, projections, neuro, dream, gateway, chain, connectors, feeds, auth, secrets, vision_loop, team, providers/models/routing, SSE, and WS | New route files should be added only for surfaces absent today: groups, arenas, evals, bounties, meta, registries, extensions, gates, recipes, and parity if not already covered by `routes/status.rs` `/parity`. |
| Older docs corpus | `docs/` has 422 markdown files across 22 topical directories, ~208K total doc lines with many "built but not wired" notes | Plan 06 alone is insufficient for full parity. Implement Plan 07 and Plan 08 after or alongside this plan. |

### Execution rule for every Plan 06 batch

Every batch below must be executed as a context-free implementation packet. A
Codex agent must not mark a checkbox complete until it has:

1. Read the architecture doc(s), existing code files, and any docs-parity rows
   that mention the same feature.
2. Searched the repo for existing implementations using `rg` and extended the
   canonical module instead of adding a duplicate.
3. Added or updated typed data models, runtime wiring, persistence, CLI/API
   surfaces, events, dashboard/TUI projections, and tests where the source docs
   require end-to-end behavior.
4. Added a task-specific verification command and, after Plan 07 exists, a
   `.roko/parity/docs-ledger.json` row for every source doc requirement it
   satisfies.
5. Removed placeholder/stub behavior from the production path, or recorded an
   explicit `deferred` ledger row with owner, dependency, reason, and future
   gate.

If a batch lists only a high-level target, use Plan 07's execution packets to
expand it into concrete target files, route contracts, tests, and acceptance
gates before editing code. If Plan 08 has a gate for the feature, that gate is
the final definition of done.

**Source docs**:
- `/Users/will/dev/nunchi/roko/roko/tmp/architecture/00-INDEX.md`
- `/Users/will/dev/nunchi/roko/roko/tmp/architecture/18-roadmap.md`
- `/Users/will/dev/nunchi/roko/roko/tmp/architecture/20-orchestrator-gaps.md`
- `/Users/will/dev/nunchi/roko/roko/tmp/defi/gap/00-INDEX.md`
- `/Users/will/dev/nunchi/roko/roko/tmp/defi/gap/11-CHECKLIST-IMPLEMENTATION.md`

---

## Parallel Track: Orchestrator Gaps (P0)

**Architecture doc**: [20-orchestrator-gaps.md](../architecture/20-orchestrator-gaps.md)

**Estimated tasks**: 10 (gaps 1 wiring, 2 wiring, 3, 4, 5, 6, 8, 9, 10, 11)
**Estimated effort**: 2-3 weeks (2 agents in parallel)
**Dependencies**: None (runs alongside Phases A-C)
**Key crates**: `roko-gate`, `roko-learn`, `roko-compose`, `roko-runtime`, `roko-cli`

These are P0 because they improve the quality of self-hosted plan execution — the core loop that drives everything else.

### What already exists (do not rebuild)

| Item | Location | Status |
|------|----------|--------|
| `ReviewDecision` enum (Approve, Revise, Skip) | `roko-gate/src/review_verdict.rs` | **EXISTS** |
| `ReviewIssue` struct (10 categories) | `roko-gate/src/review_verdict.rs` | **EXISTS** |
| `ReviewVerdict` struct | `roko-gate/src/review_verdict.rs` | **EXISTS** |
| `CompileError` struct (10 categories) | `roko-gate/src/compile_errors.rs` | **EXISTS** |
| `ErrorCategory` enum + `classify_error_code()` | `roko-gate/src/compile_errors.rs` | **EXISTS** |
| All 10 conductor watchers | `roko-conductor/src/watchers/` | **EXISTS** |
| CascadeRouter (LinUCB bandit) | `roko-learn/src/cascade_router.rs` | **EXISTS** |
| KnowledgeStore (query, ingest) | `roko-neuro/src/knowledge_store.rs` | **EXISTS** |
| Playbook rules store | `roko-learn/src/playbook_rules.rs` | **EXISTS** |

### Batch OG.1: Review + compile error wiring (Gaps 1-2)

Types exist. **Wiring into orchestrate.rs** is missing.

- [ ] **OG.1.1** Wire ReviewVerdict parsing + express mode into orchestrate.rs
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs`
  - Current: After review phase, agent output is treated as raw text. Structured parsing not called.
  - Change: After review agent returns, call `parse_review(output)` (algorithm from 20-orchestrator-gaps.md spec clarifications). If `all_issues_quick_fixable()`, skip strategist and dispatch directly to implementer (express mode).
  - Parsing chain: JSON → JSON code block → TOML code block → fallback (Revise with raw text)
  - Express mode saves one agent spawn (~5-15s + inference cost) for trivial fixes.
  - Acceptance:
    - [ ] Agent returns `{"verdict":"Approve","issues":[],"summary":"LGTM"}` → skips to next task
    - [ ] Agent returns verdict with only Style/Docs issues → express mode (skip strategist)
    - [ ] Agent returns verdict with TestFailure → full strategist cycle
    - [ ] Malformed output → fallback to Revise with raw text (never crashes)
    - [ ] Express mode visible in episode log (`express_mode: true`)
  - Size: M (1-2 days)

- [ ] **OG.1.2** Wire auto-fix path before agent retry
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs`
  - Current: On compile failure, raw `cargo check` output passed to agent. No automatic fix attempt.
  - Change: Before spawning retry agent, run:
    1. `cargo fix --allow-dirty` on the worktree
    2. `cargo +nightly fmt --all`
    3. Re-run `cargo check --message-format=json`
    4. If all errors resolved → skip agent retry entirely (gate now passes)
    5. If errors remain → pass **classified** errors (via `classify_error_code()`) to agent instead of raw output
  - Acceptance:
    - [ ] Simple import error → `cargo fix` resolves it, no agent needed
    - [ ] Complex error → agent receives classified errors (ImportNotFound, TypeMismatch, etc.)
    - [ ] `cargo fix` failure (non-zero exit) → skip auto-fix, fall through to agent
    - [ ] Cost savings logged when auto-fix succeeds (saved_inference_cost field)
  - Size: M (1-2 days)

### Batch OG.2: Error pattern sharing + reflection (Gaps 3-4)

- [ ] **OG.2.1** Implement error pattern discovery and cross-agent sharing
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/error_patterns.rs` (new file) + wire into orchestrate.rs
  - Design:
    ```rust
    pub struct DiscoveredPattern {
        pub plan: String,
        pub digest: String,           // normalized error signature
        pub timestamp: DateTime<Utc>,
        pub resolved: bool,
    }

    pub fn extract_error_digest(output: &str) -> String {
        // 1. Parse error[E...] blocks from cargo output
        // 2. Deduplicate by (error_code, file_path) — same E0425 in same file = 1 pattern
        // 3. Cap at 10 unique patterns, 200 chars each
        // 4. Join into compact digest string
    }

    pub fn is_mostly_passing(results: &[TestResult]) -> bool {
        let total = results.len();
        let passed = results.iter().filter(|r| r.passed).count();
        total > 20 && passed as f64 / total as f64 > 0.90 && passed < total
    }
    ```
  - Storage: `.roko/learn/discovered-patterns.json`
  - Integration:
    - After gate failure: `extract_error_digest()` → `append_discovered_pattern()`
    - Before agent dispatch: `read_discovered_patterns()` (last 5 unresolved, 200 chars each) → inject into system prompt
    - When `is_mostly_passing()`: use targeted fix instead of full replan
  - Acceptance:
    - [ ] Error digest extracted from real cargo output
    - [ ] Patterns persisted to discovered-patterns.json
    - [ ] Parallel agents see each other's patterns (shared file)
    - [ ] `is_mostly_passing()`: true for 95% pass with 1 failure, false for 50% pass
    - [ ] Pattern injection visible in agent system prompt
  - Size: M (2 days)

- [ ] **OG.2.2** Implement post-gate reflection loop
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs` (new function)
  - Design:
    ```rust
    async fn generate_reflection(
        gate_output: &str,
        files_changed: &[String],
        iteration: u32,
        previous_reflections: &[String],
    ) -> Result<String> {
        // Skip if error_digest matches a previous reflection's pattern
        let digest = extract_error_digest(gate_output);
        if previous_reflections.iter().any(|r| r.contains(&digest)) {
            return Ok(previous_reflections.last().unwrap().clone());
        }

        // Use cheapest model, max 500 output tokens
        let response = dispatch_agent(AgentConfig {
            model: "claude-haiku-4-5-20251001",
            max_tokens: 500,
            prompt: format!(
                "Analyze this gate failure. What went wrong? What should the next attempt do differently?\n\
                Gate output: {}\nFiles changed: {:?}\nAttempt #{}",
                &digest, files_changed, iteration
            ),
        }).await?;

        Ok(response.text)
    }
    ```
  - Storage: Add `reflection: Option<String>` field to Episode struct (in `roko-learn/src/episode_logger.rs`)
  - Injection: On retry, prepend to agent's system prompt: `"Lessons from previous attempt: {reflection}"`
  - Cost guard: max_tokens=500 on cheapest model. At Haiku pricing, this is ~$0.0001 per reflection.
  - Acceptance:
    - [ ] Gate failure triggers reflection generation
    - [ ] Reflection stored in episode's `reflection` field
    - [ ] Same error pattern → reuses previous reflection (deduplication)
    - [ ] Retry agent sees "Lessons from previous attempt" in system prompt
    - [ ] Reflection uses Haiku model with max_tokens=500
    - [ ] Episode log shows reflection text
  - Size: M (2 days)

### Batch OG.3: Context scoping + warm spawning (Gaps 5-6)

- [ ] **OG.3.1** Implement context injection scoping with KnowledgeConfig
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/context_scoping.rs` (new file) + wire into orchestrate.rs
  - Design:
    ```rust
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct KnowledgeConfig {
        pub file_intel_enabled: bool,         // default: true
        pub file_intel_max_entries: usize,    // default: 10
        pub warnings_enabled: bool,           // default: true
        pub warning_max_entries: usize,       // default: 5
        pub error_patterns_enabled: bool,     // default: true
        pub error_pattern_min_cluster: usize, // default: 3
        pub wave_context_enabled: bool,       // default: true
        pub dynamic_budget_enabled: bool,     // default: false
    }
    ```
  - Role filtering:
    | Role | File intel | Warnings | Error patterns |
    |------|-----------|----------|----------------|
    | Implementer | 10 (full: path + functions + recent changes) | 5 | 5 |
    | Reviewer | 3 (summary: path + one-line description) | 3 | 3 |
    | Strategist | 0 (plan-level only) | 0 | 0 |
  - `collect_plan_playbook_scope(plan, tasks) -> PlaybookScope`: Extract file globs + tags from task definitions. Only match playbook rules whose `trigger_files` overlap with plan's files.
  - Configurable via roko.toml:
    ```toml
    [knowledge]
    file_intel_max_entries = 10
    warnings_max_entries = 5
    error_pattern_min_cluster = 3
    wave_context_enabled = true
    ```
  - Acceptance:
    - [ ] `KnowledgeConfig` loads from roko.toml with defaults
    - [ ] Implementer gets 10 file intel entries in prompt
    - [ ] Reviewer gets 3 summary-only entries
    - [ ] Strategist gets no file-level context
    - [ ] Config toggles suppress sections (e.g., `warnings_enabled = false` → no warnings)
    - [ ] Playbook rules scoped to plan's files only (not global)
  - Size: M (2 days)

- [ ] **OG.3.2** Implement warm agent spawning pool
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/warm_pool.rs` (new file) + wire into orchestrate.rs
  - Design:
    ```rust
    pub struct WarmPool {
        pool: HashMap<String, WarmAgent>,  // role → warm agent
    }

    pub struct WarmAgent {
        pub role: String,
        pub process: Child,
        pub spawned_at: Instant,
        pub ready: bool,           // true when agent has initialized
    }

    impl WarmPool {
        /// Pre-spawn agent for the next phase. Called during gate execution.
        pub async fn pre_spawn(&mut self, role: &str, config: &AgentConfig) -> Result<()>;

        /// Swap warm agent to active. Returns connection immediately (<100ms).
        pub fn promote(&mut self, role: &str) -> Result<AgentConnection>;

        /// Kill warm agent on gate failure. Frees resources.
        pub fn evict(&mut self, role: &str) -> Result<()>;

        /// Kill all warm agents. Called on plan completion or error.
        pub fn drain(&mut self);
    }
    ```
  - Integration in orchestrate.rs:
    ```
    // During gate execution (compile/test)
    warm_pool.pre_spawn("reviewer", reviewer_config).await?;

    // On gate pass
    let reviewer = warm_pool.promote("reviewer")?;  // <100ms vs 5-15s cold

    // On gate fail
    warm_pool.evict("reviewer")?;  // clean up, no leaked processes
    ```
  - Safety: `drain()` called on plan completion, error, or signal (SIGTERM). No orphan processes.
  - Eviction error handling: If process is already dead (e.g., crashed during warm-up), `evict()` is a no-op (infallible).
  - Acceptance:
    - [ ] Warm agent spawns during gate execution (background)
    - [ ] `promote()` returns connection in <100ms
    - [ ] Cold spawn baseline: 5-15s (for comparison)
    - [ ] `evict()` kills process, no zombie
    - [ ] `drain()` kills all warm agents
    - [ ] No leaked processes on any error path
    - [ ] Timing logged: `warm_spawn_ms`, `promote_ms`, `cold_spawn_ms` (for learning)
  - Size: M (2 days)

### Batch OG.4: Learning loop integration (Gaps 8-11)

- [ ] **OG.4.1** Wire neuro store into cascade router
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/cascade_router.rs`
  - Current: Router selects models based on LinUCB bandit (pass/fail history). Does NOT consult knowledge store.
  - Change: At `decide()` time:
    1. Query `knowledge_store.query(task_description, limit=3)` for relevant prior knowledge
    2. If knowledge entries mention specific model preferences → bias +0.1
    3. If knowledge entries describe failure patterns with a model → bias -0.1
    4. Extend LinUCB context vector with 2 new features: `knowledge_match_score`, `knowledge_model_bias`
    5. Clamp final score to [0.05, 1.0] (0.05 floor for exploration)
  - Config: `cascade_router.consult_knowledge: bool` (default true)
  - Performance constraint: knowledge query must complete in <10ms (store is local JSONL)
  - Acceptance:
    - [ ] Router queries knowledge store at decide time
    - [ ] Model with positive knowledge mention → higher score
    - [ ] Model with failure pattern mention → lower score
    - [ ] LinUCB feature vector extended (verify via debug log)
    - [ ] Config toggle: `consult_knowledge = false` → no query
    - [ ] Query latency <10ms (test with 1000-entry store)
  - Size: M (2 days)

- [ ] **OG.4.2** Implement episode clustering for model recommendations
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/pattern_discovery.rs` (extend existing)
  - Design:
    ```rust
    pub struct EpisodeCluster {
        pub key: String,                // error_signature or file_pattern
        pub count: usize,
        pub maturity: ClusterMaturity,
        pub success_rate: f64,
        pub common_files: Vec<String>,
        pub best_model: Option<String>,     // None for immature clusters
        pub best_provider: Option<String>,
        pub avg_cost_usd: f64,
    }

    pub enum ClusterMaturity {
        Immature,   // < 3 episodes — no recommendation
        Mature,     // >= 3 episodes — produces model recommendation
    }

    pub fn cluster_episodes(episodes: &[Episode]) -> Vec<EpisodeCluster> {
        // Group by error_signature (failures) or file_pattern (successes)
        // For each group >= 3: compute best model (highest success_rate)
        // Groups < 3: store as Immature (no recommendation yet)
    }
    ```
  - Cadence: Run every 10 new episodes (check counter in orchestrate.rs)
  - Integration: Feed mature cluster recommendations into cascade_router as soft bias
  - Acceptance:
    - [ ] 5 episodes with same error + model A succeeding → recommends model A
    - [ ] Cluster with 2 episodes → Immature, no recommendation
    - [ ] Clustering runs every 10 episodes
    - [ ] Recommendations used as soft bias in cascade_router
    - [ ] Clusters persisted to `.roko/learn/episode-clusters.json`
  - Size: M (2 days)

- [ ] **OG.4.3** Wire provider pass-rate into model scoring
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/cascade_router.rs` (extend)
  - Design:
    ```rust
    pub struct ProviderMetrics {
        pub provider: String,
        pub pass_rate: f64,       // gate passes / total attempts
        pub avg_cost: f64,
        pub avg_duration_ms: u64,
        pub sample_count: usize,  // min 5 before metrics affect scoring
    }

    fn compute_provider_metrics(episodes: &[Episode]) -> HashMap<String, ProviderMetrics> {
        // Group by provider, compute pass_rate, avg_cost, avg_duration
        // Only include providers with 5+ episodes
    }
    ```
  - In cascade_router stages 2-3: `model_score *= provider_pass_rate`
  - Use existing `ProviderHealthTracker` if available; fall back to episode-derived metrics
  - Acceptance:
    - [ ] Provider with 0.9 pass_rate boosts its models vs provider with 0.6
    - [ ] Provider with <5 episodes → no effect (insufficient data)
    - [ ] Multiplier visible in router decision log
  - Size: S (1 day)

- [ ] **OG.4.4** Implement reflection-derived playbook rules
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/playbook_rules.rs` (extend existing)
  - Design: After reflection stored in episode (OG.2.2), extract actionable patterns:
    - Reflection mentions specific files → create rule with `trigger_files` glob
    - Reflection mentions error type → create rule with `trigger_tags`
    - Context = reflection's key insight text
  - Confidence lifecycle:
    - New rules: 0.5 (neutral)
    - On gate pass: +0.05
    - On gate fail: -0.10
    - Below 0.2: auto-remove (unless `source: "manual"`)
  - Cadence: Run after every 3 new reflections
  - Persistence: `.roko/learn/playbook-rules.json` with `source: "reflection"` tag
  - Acceptance:
    - [ ] Reflection mentioning `src/auth.rs` → rule with `trigger_files: ["**/auth.rs"]`
    - [ ] Gate pass → confidence 0.5 → 0.55
    - [ ] Gate fail → confidence 0.55 → 0.45
    - [ ] Confidence drops to 0.15 → auto-removed
    - [ ] Manually created rules (source: "manual") preserved even below 0.2
    - [ ] Rules used in context injection (from OG.3.1)
  - Size: M (2 days)

---

## Phase A: Core agent runtime

**Architecture docs**: [02-agent-runtime.md](../architecture/02-agent-runtime.md), [03-extensions.md](../architecture/03-extensions.md)

**Estimated tasks**: 14 (reduced from 18 — several items already exist)
**Estimated effort**: 2-3 weeks (2 agents in parallel)
**Dependencies**: None (foundation layer)
**Key crates**: `roko-conductor`, `roko-runtime`, `roko-agent`, `roko-core`

### What already exists (do not rebuild)

| Item | Location | Status |
|------|----------|--------|
| `AgentMode` enum (Ephemeral, Persistent, Reactive) | `roko-core/src/config/schema.rs` | **EXISTS** — serializable, used in AgentConfig |
| `DomainProfile` enum + `.label()` methods | `roko-core/src/domain_profile.rs` | **EXISTS** — Coding, Research, Chain, DataMl, Ops, Writing |
| `ExtensionLayer` enum (8 layers) + `ExtensionMeta` | `roko-core/src/extension.rs` | **EXISTS** — layer ordering, action/tool/recovery decisions |
| `ConnectorKind` + `ConnectorRegistry` + `ConnectorHealth` | `roko-core/src/connector.rs` | **EXISTS** |
| `FeedKind` + `FeedAccess` + `FeedRegistry` | `roko-core/src/feed.rs` | **EXISTS** |
| 10 conductor watchers | `roko-conductor/src/watchers/` | **EXISTS** — all 10 from the orchestrator gaps spec |
| State machine + intervention policies | `roko-conductor/src/` (state_machine, conductor, interventions, circuit_breaker) | **EXISTS** |
| Self-healing + pattern detection | `roko-conductor/src/self_healing.rs`, `pattern_detector.rs` | **EXISTS** |
| Lifecycle state machine | `roko-runtime/src/lifecycle.rs` | **EXISTS** — AgentLifecycleState, LifecycleHooks |
| Heartbeat emission | `roko-runtime/src/heartbeat.rs` | **EXISTS** |
| Event bus | `roko-runtime/src/event_bus.rs` | **EXISTS** — typed broadcast with replay |

### Batch A.1: Agent mode lifecycle wiring

**Read**: `02-agent-runtime.md` (three modes section, lines 96-127)

The `AgentMode` enum exists but mode-specific **lifecycle behavior** is not wired.

- [ ] **A.1.1** Wire ephemeral auto-stop into process supervisor
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/process.rs`
  - Current: ProcessSupervisor tracks agents but doesn't inspect mode on task completion
  - Change: After `dispatch_agent_with()` returns success for an Ephemeral agent, call `supervisor.stop(agent_id)` and clean up
  - Termination trigger: The agent's task completes (gate pipeline finishes). NOT "first response" — the full task-gate-persist cycle.
  - Timeout: If ephemeral agent runs >30 minutes without completing, log warning and stop (configurable via `agent.ephemeral_timeout_secs` in roko.toml, default 1800)
  - Acceptance:
    - [ ] Start ephemeral agent via `roko agent start --name x --mode ephemeral`
    - [ ] Give it a task via `POST /api/agents/{id}/task`
    - [ ] Agent stops after task completion (process exits, no zombie)
    - [ ] Timeout fires after 30min of no completion
    - [ ] `roko agent list` shows status `stopped` after auto-stop
  - Size: S (1 day)

- [ ] **A.1.2** Wire reactive mode with trigger registry
  - Target: new file `/Users/will/dev/nunchi/roko/roko/crates/roko-runtime/src/reactive.rs`
  - Existing: `roko-runtime/src/lifecycle.rs` has `LifecycleHooks` and state transitions
  - Design:
    ```rust
    pub struct ReactiveScheduler {
        triggers: Vec<TriggerConfig>,
        waker: mpsc::Sender<TriggerEvent>,
    }

    pub enum TriggerConfig {
        Webhook { path: String },
        Cron { schedule: String },        // standard cron syntax
        ChainEvent { contract: String, event: String },
        Message { room: String },
    }

    pub struct TriggerEvent {
        pub trigger_type: String,
        pub payload: serde_json::Value,
        pub received_at: Instant,
    }
    ```
  - Cron implementation: Use `cron` crate (already in workspace deps). Register schedules at agent startup. Each schedule fires a `TriggerEvent` into the agent's waker channel.
  - Webhook implementation: Register paths in roko-serve router dynamically. Incoming POST to `/hooks/{path}` sends `TriggerEvent` to the matching agent.
  - Sleep behavior: When no trigger is pending, the agent's process suspends (no tick loop, zero CPU). The `ReactiveScheduler` holds the waker channel — the agent's `run()` loop blocks on `waker.recv()`.
  - Acceptance:
    - [ ] Configure reactive agent with `triggers = [{ type = "schedule", cron = "*/5 * * * *" }]` in roko.toml
    - [ ] Agent wakes every 5 minutes, runs pipeline, then sleeps
    - [ ] `roko agent status --name x` shows `sleeping` between triggers
    - [ ] Webhook trigger: `POST /hooks/github-pr` wakes agent within 100ms
    - [ ] Zero CPU usage while sleeping (verify with `ps`)
  - Size: L (3-4 days)

### Batch A.2: Tick pipeline and adaptive clock

**Read**: `02-agent-runtime.md` (9-step pipeline, T0/T1/T2 gating, adaptive clock algorithm)

The conductor has watchers and interventions but no **tick pipeline** or **adaptive clock**. These are distinct from the existing task-dispatch model (which is plan-based, not tick-based). The tick pipeline enables persistent/reactive agents to operate autonomously.

- [ ] **A.2.1** Implement `TickPipeline` struct with 9 steps
  - Source: `bardo/crates/golem-heartbeat/src/pipeline.rs` (3,019 LOC reference)
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-conductor/src/tick_pipeline.rs` (new file)
  - Design:
    ```rust
    pub struct TickPipeline {
        steps: [Box<dyn PipelineStep>; 9],
        reflex_store: ReflexStore,
    }

    #[async_trait]
    pub trait PipelineStep: Send + Sync {
        fn name(&self) -> &str;
        async fn execute(
            &self,
            cortical: &mut CorticalState,
            extensions: &[Box<dyn Extension>],
            inference: &InferenceHandle,
        ) -> Result<StepOutcome>;
    }

    pub enum StepOutcome {
        Continue,
        SkipTo(usize),   // T0 reflex: skip to step 7 (Execute)
        Stop,             // Agent should terminate
    }
    ```
  - Steps: Observe, Retrieve, Analyze, Gate, Simulate, Validate, Execute, Verify, Reflect
  - Extension hooks fire between steps according to layer ordering (see 03-extensions.md)
  - Acceptance:
    - [ ] `TickPipeline::execute_tick()` runs all 9 steps in sequence
    - [ ] Extensions fire at correct interleave points (L1 Perception before Observe, L3 Cognition around Gate, etc.)
    - [ ] `StepOutcome::SkipTo` correctly jumps to the target step
    - [ ] Unit test: mock pipeline with 9 no-op steps, verify execution order via log
    - [ ] Unit test: T0 reflex path skips steps 5-6
  - Size: L (3-4 days)

- [ ] **A.2.2** Implement T0/T1/T2 gating decision function
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-conductor/src/tick_pipeline.rs` (inside the Gate step)
  - Algorithm (from 02-agent-runtime.md):
    ```
    fn decide_tier(pe: f64, budget_remaining: u64, cortical_urgency: f64) -> Tier {
        if budget_remaining == 0 { return Tier::Sleepwalk; }
        if pe < 0.15 && cortical_urgency < 0.30 { return Tier::T0; }
        if pe > 0.40 || cortical_urgency > 0.70 { return Tier::T2; }
        Tier::T1
    }
    ```
  - Edge case — PE oscillation at boundary: If PE oscillates around 0.15 (e.g., 0.14, 0.16, 0.14), the gating function applies **no hysteresis** — it evaluates fresh each tick. Hysteresis is only on the clock regime, not on the tier decision. This is intentional: tier decisions are per-tick and cheap to switch.
  - Acceptance:
    - [ ] `decide_tier(0.10, 1000, 0.1)` returns T0
    - [ ] `decide_tier(0.25, 1000, 0.5)` returns T1
    - [ ] `decide_tier(0.50, 1000, 0.8)` returns T2
    - [ ] `decide_tier(0.50, 0, 0.8)` returns Sleepwalk
    - [ ] Integration: T0 tier causes pipeline to skip steps 5-6 and use reflex store
  - Size: S (half day)

- [ ] **A.2.3** Implement T0 reflex store
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-conductor/src/reflex_store.rs` (new file)
  - Storage: `.roko/learn/reflexes.jsonl` (one condition-action pair per line)
  - Design:
    ```rust
    pub struct ReflexStore {
        rules: Vec<ReflexRule>,
        path: PathBuf,
    }

    pub struct ReflexRule {
        pub condition: ReflexCondition,
        pub action: ReflexAction,
        pub confidence: f64,
        pub source_episode: String,
        pub success_count: u32,
        pub total_count: u32,
        pub promoted_at: DateTime<Utc>,
    }
    ```
  - Matching: Linear scan of rules. First match wins. Conditions are structural: tool name + args glob + context tag.
  - Promotion: A T2 decision becomes a T0 reflex when the same observation→action pair succeeds 3+ times with zero gate failures. Confidence = success_count / total_count.
  - Demotion: On gate failure, confidence is halved. Below 0.50, delete the rule and log deletion.
  - Mixed success/failure: Confidence recalculates as running ratio. Example: 5 successes, then 1 failure → confidence = 5/6 ≈ 0.83 (stays). Then 2 more failures → confidence = 5/8 = 0.625 (stays). Then a gate failure halves it → 0.3125 → deleted.
  - Acceptance:
    - [ ] Reflex rule created after 3 identical T2 successes
    - [ ] T0 path matches rule and executes action without LLM call
    - [ ] Gate failure halves confidence
    - [ ] Rule deleted when confidence < 0.50
    - [ ] `.roko/learn/reflexes.jsonl` persists across restarts
    - [ ] Max 200 rules (evict lowest confidence when full)
  - Size: M (2 days)

- [ ] **A.2.4** Implement `AdaptiveClock` with regime detection
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-conductor/src/adaptive_clock.rs` (new file)
  - Design:
    ```rust
    pub struct AdaptiveClock {
        base_interval: Duration,         // default 500ms, configurable
        regime: Regime,
        hysteresis_counter: u32,         // ticks at candidate regime
        candidate_regime: Option<Regime>,
        last_gamma: Instant,
        gamma_count_since_theta: u32,
        episodes_since_delta: u32,
        last_activity: Instant,
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum Regime { Calm, Normal, Volatile, Crisis }
    ```
  - Regime transitions (from 02-agent-runtime.md state diagram):
    - Calm → Normal: PE > 0.40 for 3 consecutive ticks
    - Normal → Volatile: PE > 0.60 for 3 consecutive ticks
    - Volatile → Crisis: error_rate > 0.50 for 3 consecutive ticks
    - Crisis → Calm: error_rate < 0.10 for 3 consecutive ticks
    - Volatile → Normal: PE < 0.30 for 3 consecutive ticks
    - Normal → Calm: PE < 0.10 for 3 consecutive ticks
  - Hysteresis edge case: If PE oscillates at boundary (0.10, 0.20, 0.10), the counter resets on non-qualifying tick. Counter increments only on consecutive qualifying ticks.
  - Interval calculations:
    - Gamma: base * regime_factor (Calm=4.0, Normal=1.0, Volatile=0.5, Crisis=0.25)
    - Theta: N * gamma (Calm N=8, Normal N=5, Volatile N=3, Crisis N=2)
    - Delta: triggered by idle_timeout (60s no activity) OR episode_threshold (20 episodes)
  - Acceptance:
    - [ ] `clock.tick()` returns correct interval for each regime
    - [ ] Regime changes only after 3 consecutive qualifying ticks
    - [ ] Oscillating PE does not cause regime change (counter resets)
    - [ ] Delta tick fires after 60s idle
    - [ ] Delta tick fires after 20 episodes
    - [ ] `base_interval` configurable via `agent.clock_base_ms` in roko.toml
  - Size: M (2 days)

- [ ] **A.2.5** Implement cortical state persistence
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-conductor/src/cortical.rs` (new file)
  - Design:
    ```rust
    pub struct CorticalState {
        pub agent_id: String,
        pub snapshot_at: DateTime<Utc>,
        pub working_memory: Vec<MemoryItem>,   // last N context items
        pub goals: Vec<Goal>,                   // current task goals
        pub beliefs: HashMap<String, f64>,      // key-value belief scores
        pub attention: AttentionFocus,
        pub regime: Regime,
        pub prediction_error_ema: f64,
        pub episode_count: u64,
    }

    pub struct AttentionFocus {
        pub focus: String,       // current task description
        pub salience: f64,       // 0.0-1.0
    }
    ```
  - Persistence: Serialize to `.roko/agents/{id}/cortical.json` on every theta tick (not gamma — too frequent)
  - Restart behavior:
    - Snapshot exists AND < 1 hour old → load and resume
    - Snapshot exists AND >= 1 hour old → discard, start fresh (stale beliefs/goals drift from environment)
    - No snapshot → start fresh (first run)
  - Staleness threshold: 1 hour is hardcoded for now. Rationale: after 1 hour, external state (code changes, PR updates, chain state) has likely changed enough that stale beliefs hurt more than help. Can be made configurable later if needed.
  - Acceptance:
    - [ ] Cortical state serializes to JSON correctly
    - [ ] Theta tick writes snapshot to `.roko/agents/{id}/cortical.json`
    - [ ] Agent restart loads snapshot if < 1 hour old
    - [ ] Agent restart ignores snapshot if >= 1 hour old
    - [ ] Missing snapshot starts fresh without error
    - [ ] Working memory capped at 50 items (LRU eviction)
  - Size: M (1-2 days)

### Batch A.3: Extension trait and hook wiring

**Read**: `03-extensions.md` (full trait, 22 hooks, loading, dependency resolution)

The `ExtensionLayer` enum and `ExtensionMeta` struct exist in `roko-core/src/extension.rs`. What's missing is the **actual `Extension` trait** with async hooks, the **loader**, and the **hook invocation** in the dispatch pipeline.

- [ ] **A.3.1** Define full `Extension` trait with 22 async hooks
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/extension.rs` (extend existing file)
  - Current: File has `ExtensionLayer` enum and `ExtensionMeta` but no trait with async methods
  - Add: The full `#[async_trait] pub trait Extension: Send + Sync` from 03-extensions.md (lines 10-68)
  - Decision enum variants (resolving the gap in the spec):
    ```rust
    pub enum FilterDecision { Pass, Drop, Transform(AgentMessage) }
    pub enum ActionDecision { Proceed, Block { reason: String }, Modify(Action) }
    pub enum ToolDecision { Allow, Block { reason: String }, Substitute(ToolCall) }
    pub enum RecoveryAction { Propagate, Retry, Ignore, Escalate(String) }
    pub enum BudgetAction { Sleepwalk, Stop, RequestMore(u64) }
    pub enum Adjustment { SetGoal(Goal), UpdateBelief(String, f64), ShiftAttention(String) }
    ```
  - Hook timeout: 5 seconds per hook invocation (hardcoded, matching 03-extensions.md line 158). If a hook exceeds 5s, log warning and continue. Not configurable per-hook (keep it simple; revisit if needed).
  - AgentContext (resolving the gap): Extensions receive `&AgentContext` which contains:
    ```rust
    pub struct AgentContext {
        pub agent_id: String,
        pub profile: DomainProfile,
        pub mode: AgentMode,
        pub regime: Regime,
        pub budget_remaining: u64,      // microdollars
        pub episode_count: u64,
        pub config: Arc<AgentConfig>,
    }
    ```
  - Acceptance:
    - [ ] Trait compiles with all 22 hooks
    - [ ] All hooks have default no-op implementations (agent works with zero extensions)
    - [ ] Decision enums are exhaustive and serializable
    - [ ] `FilterDecision::Drop` causes message to be silently discarded
    - [ ] `ActionDecision::Block` halts the action but not the agent
    - [ ] `ToolDecision::Substitute` replaces the tool call with the provided alternative
  - Size: M (1-2 days)

- [ ] **A.3.2** Implement extension loader with dependency resolution
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/extensions/mod.rs` (new directory + module)
  - Design:
    ```rust
    pub struct ExtensionLoader {
        builtin_registry: HashMap<String, fn() -> Box<dyn Extension>>,
        local_path: PathBuf,         // .roko/extensions/
    }

    impl ExtensionLoader {
        pub fn load_chain(config: &[ExtensionRef]) -> Result<Vec<Box<dyn Extension>>> {
            // 1. Resolve each name: builtin → local → registry
            // 2. Read manifest.toml for dependencies
            // 3. Topological sort within each layer
            // 4. Return ordered chain
        }
    }
    ```
  - Three sources (in priority order): built-in (compiled in) → local (`.roko/extensions/{name}/`) → registry (future, not implemented now — return error)
  - Dependency resolution: Topological sort within each layer. Cross-layer deps not supported (layer ordering handles it). Cyclic dependency = startup error.
  - `optional` flag: `optional = true` means log warning and continue on load failure. `optional = false` (default) means abort agent startup.
  - Acceptance:
    - [ ] Built-in extensions load by name (e.g., "git", "compiler")
    - [ ] `extensions = [{ name = "git" }, { name = "custom", optional = true }]` in roko.toml works
    - [ ] Missing optional extension logs warning, agent starts
    - [ ] Missing required extension aborts with clear error message
    - [ ] Dependency cycle detected and reported as error
    - [ ] Extensions sorted by layer then by dependency then by config order
  - Size: M (2 days)

- [ ] **A.3.3** Implement 3 built-in extensions (coding, research, chain)
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/extensions/builtins/` (new directory)
  - Built-in extensions are thin wrappers that implement a few hooks each:
    - `CodingExt`: `pre_action` (validate file paths exist), `post_action` (run formatter), `on_error` (classify compile errors)
    - `ResearchExt`: `on_retrieve` (query web search), `post_inference` (extract citations), `on_store` (validate sources)
    - `ChainExt`: `on_observe` (poll chain state), `pre_action` (safety check tx values), `on_cost_update` (track gas costs)
  - These are NOT comprehensive implementations — they're proof-of-concept extensions that demonstrate the hook system works end-to-end.
  - Acceptance:
    - [ ] `roko agent create --domain coding` gets CodingExt in its chain
    - [ ] CodingExt's `pre_action` fires before file operations
    - [ ] Each extension implements 3-5 hooks, rest use defaults
    - [ ] Extensions load via `ExtensionLoader` by name
  - Size: M (2-3 days)

- [ ] **A.3.4** Wire extension hooks into orchestrate.rs dispatch pipeline
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs` (modify `dispatch_agent_with()`)
  - Current: `dispatch_agent_with()` dispatches to agent backend but doesn't invoke extension hooks
  - Change: Wrap the dispatch call with extension hook invocations:
    ```
    for ext in extensions (sorted by layer):
        ext.pre_inference(&mut request)?
    let response = dispatch_to_backend(request).await?
    for ext in extensions:
        ext.post_inference(&mut response)?
    for ext in extensions:
        ext.pre_action(&mut action)?    // if action generated
    execute_action(action)
    for ext in extensions:
        ext.post_action(&action, &result)?
    ```
  - Fault isolation: If any hook returns Err, log it and continue. Only `pre_action` returning `Block` stops the action.
  - Acceptance:
    - [ ] `pre_inference` hook can modify the inference request (e.g., add system prompt)
    - [ ] `post_inference` hook can modify the response (e.g., strip PII)
    - [ ] `pre_action` Block prevents action execution
    - [ ] Hook errors are logged but don't crash the agent
    - [ ] Hook timeout (5s) enforced
    - [ ] Integration test: mock extension that modifies request → verify modification visible in response
  - Size: M (2 days)

- [ ] **A.3.5** Implement profile loading from `~/.roko/profiles/*.toml`
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/domain_profile.rs` (extend existing)
  - Current: `DomainProfile` is an enum with 6 variants. Architecture spec says it should be a newtype over String.
  - Change: Keep existing enum for built-in profiles BUT add `DomainProfile::Custom(String)` variant. Custom profiles load from `~/.roko/profiles/{name}.toml`.
  - Profile TOML format:
    ```toml
    [profile]
    name = "defi-trader"
    description = "DeFi trading agent"
    extensions = ["chain-reader", "tx-builder", "risk-engine"]
    tools = ["eth_call", "send_tx", "subscribe_events"]
    default_mode = "persistent"
    [profile.budget]
    daily_limit_usd = 50.0
    ```
  - Acceptance:
    - [ ] `roko agent create --domain defi-trader` loads `~/.roko/profiles/defi-trader.toml`
    - [ ] Built-in profiles (coding, research, chain) still work without TOML files
    - [ ] Missing profile TOML for custom domain → error with helpful message
    - [ ] Profile extensions merged with agent-level extensions (profile first, then agent overrides)
  - Size: S (1 day)

---

## Phase B: Connectivity and relay

**Architecture docs**: [04-connectivity.md](../architecture/04-connectivity.md), [05-feeds.md](../architecture/05-feeds.md)

**Estimated tasks**: 8 (reduced — feed types and routes already exist)
**Estimated effort**: 2-3 weeks
**Dependencies**: Phase A (agents must publish heartbeats via tick pipeline)
**Key crates**: `roko-serve`, `roko-runtime`, `roko-agent`

### What already exists (do not rebuild)

| Item | Location | Status |
|------|----------|--------|
| WebSocket routes | `roko-serve/src/routes/ws.rs` | **EXISTS** — basic WS streaming |
| SSE routes | `roko-serve/src/routes/sse.rs` | **EXISTS** — server-sent events |
| Event bus | `roko-runtime/src/event_bus.rs` | **EXISTS** — typed broadcast with replay |
| Feed routes | `roko-serve/src/routes/feeds.rs` | **EXISTS** — feed CRUD endpoints |
| Feed types | `roko-core/src/feed.rs` | **EXISTS** — FeedKind, FeedAccess, FeedRegistry |
| Heartbeat routes | `roko-serve/src/routes/heartbeats.rs` | **EXISTS** |
| Subscription routes | `roko-serve/src/routes/subscriptions.rs` | **EXISTS** |

### Batch B.1: Relay protocol enhancements

The basic WS/SSE infrastructure exists. What's missing is the **structured envelope**, **room-based filtering**, **sequence-based replay**, and **backpressure coalescing**.

- [ ] **B.1.1** Add structured message envelope to WS
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/ws.rs` (extend existing)
  - Design:
    ```rust
    pub struct WsEnvelope {
        pub seq: u64,                    // monotonic per-connection sequence
        pub ts: u64,                     // unix millis
        pub room: String,               // e.g., "agent:coder-1", "plan:abc", "global"
        pub event_type: String,          // e.g., "heartbeat", "trace", "presence_join"
        pub payload: serde_json::Value,
    }
    ```
  - Room naming convention (from 04-connectivity.md):
    | Room pattern | Events | Example |
    |-------------|--------|---------|
    | `agent:{id}` | heartbeat, trace, tool_call, status | `agent:coder-1` |
    | `plan:{id}` | task_start, task_complete, gate_result | `plan:abc-123` |
    | `global` | presence_join, presence_leave, cost_update | `global` |
    | `feed:{id}` | data_point, feed_status | `feed:eth-price` |
  - Current WS sends raw events. Change to wrap all events in `WsEnvelope` with auto-incrementing `seq`.
  - Acceptance:
    - [ ] All WS messages have `seq`, `ts`, `room`, `event_type`, `payload`
    - [ ] `seq` monotonically increases per connection
    - [ ] Client can parse envelope and filter by room client-side
    - [ ] Existing WS consumers don't break (backward-compat: if client sends `{"subscribe": "agent:coder-1"}`, only matching events are sent)
  - Size: M (1-2 days)

- [ ] **B.1.2** Implement room-based subscription with server-side filtering
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/ws.rs` (extend existing)
  - Design:
    ```rust
    // Client sends:
    { "action": "subscribe", "rooms": ["agent:coder-1", "plan:abc"] }
    { "action": "unsubscribe", "rooms": ["plan:abc"] }

    // Server tracks per-connection subscription set
    pub struct WsSession {
        subscriptions: HashSet<String>,
        last_seq: u64,
    }
    ```
  - Server-side filtering: Only forward events whose `room` matches a subscription. No subscriptions = receive nothing (opt-in model).
  - Wildcard: `agent:*` subscribes to all agent rooms. Useful for dashboard overview page.
  - Acceptance:
    - [ ] Client subscribes to `agent:coder-1` → only receives that agent's events
    - [ ] Unsubscribe removes room from filter
    - [ ] `agent:*` wildcard works
    - [ ] No subscriptions → no events (silent connection)
    - [ ] Multiple simultaneous WS connections each have independent subscriptions
  - Size: M (1-2 days)

- [ ] **B.1.3** Add sequence-based reconnection replay
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/ws.rs` (extend existing)
  - Design: Server maintains a ring buffer of recent events per room (last 1000 events). On reconnect, client sends `{ "action": "replay", "last_seq": 42 }`. Server replays events with seq > 42 from the buffer.
  - Ring buffer: `VecDeque<WsEnvelope>` per room, capped at 1000 entries. Oldest events evicted on overflow.
  - Edge case: If `last_seq` is older than the ring buffer's oldest entry, send a `{ "type": "replay_gap", "missed_from": 42, "available_from": 500 }` message so the client knows to do a full resync.
  - Deduplication: Client is responsible for deduplication (check `seq` against last processed). Server may send duplicates near the boundary.
  - Acceptance:
    - [ ] Client disconnects, reconnects with `last_seq=42`, receives events 43+
    - [ ] Events older than ring buffer capacity trigger `replay_gap` message
    - [ ] Ring buffer per room, 1000 entries each
    - [ ] Replay works across different rooms (replays each subscribed room independently)
  - Size: M (2 days)

- [ ] **B.1.4** Add event coalescing and backpressure
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/ws.rs` (extend existing)
  - Design:
    ```rust
    pub struct EventCoalescer {
        flush_interval: Duration,    // 100ms
        buffer: Vec<WsEnvelope>,
        last_flush: Instant,
    }
    ```
  - Coalescing logic: Buffer events for up to 100ms. On flush, send a single batched message containing all buffered events. If buffer reaches 50 events before the timer, flush immediately.
  - Heartbeat coalescing: For `heartbeat` events, only send the **latest** per agent (discard intermediate heartbeats within the flush window). This is the answer to the spec gap: 3 heartbeats in a window → client sees only the most recent.
  - Backpressure: If the WS send buffer is full (channel capacity 256), drop the oldest batch and increment a `dropped_batches` counter. Never block the event producer.
  - Acceptance:
    - [ ] Burst of 100 events in 50ms → delivered as 2 batches (first at 50 events, second at timer)
    - [ ] 3 heartbeats in 100ms → client sees only the latest heartbeat
    - [ ] WS send buffer full → oldest batch dropped, counter incremented
    - [ ] Stats: `batches_sent`, `events_coalesced`, `dropped_batches`
  - Size: M (2 days)

### Batch B.2: Feed enhancements

Feed CRUD routes and types exist. What's missing is **pagination**, **FeedPublisher extension**, and **feed subscription via WS rooms**.

- [ ] **B.2.1** Add cursor-based pagination to feed data endpoint
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/feeds.rs` (extend existing)
  - API: `GET /api/feeds/{id}/data?cursor={timestamp_ms}&limit=100`
  - Cursor is a timestamp (unix ms). Returns data points after the cursor, ordered by time ascending. Response includes `next_cursor` for the next page.
  - Default limit: 100, max limit: 1000
  - Acceptance:
    - [ ] First page: `GET /api/feeds/eth-price/data?limit=50` returns 50 items + `next_cursor`
    - [ ] Next page: `GET /api/feeds/eth-price/data?cursor={next_cursor}&limit=50` returns next 50
    - [ ] Empty page returns `{ "data": [], "next_cursor": null }`
    - [ ] Limit > 1000 clamped to 1000
  - Size: S (half day)

- [ ] **B.2.2** Wire feed events to WS room system
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/feeds.rs` (extend existing)
  - When a feed publishes a data point, broadcast to room `feed:{feed_id}`. Dashboard subscribes to `feed:eth-price` via WS to get real-time updates.
  - Acceptance:
    - [ ] Feed publishes data point → WS subscribers to `feed:{id}` receive it
    - [ ] No subscribers → data still stored, no WS broadcast (no-op)
    - [ ] Coalescing applies to feed events (latest value per flush window for high-frequency feeds)
  - Size: S (half day)

- [ ] **B.2.3** Implement `FeedPublisherExt` as built-in extension
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/extensions/builtins/feed_publisher.rs` (new file)
  - Design: Extension that registers feeds on `on_init` and publishes data points on `post_action` (when action result contains feed-relevant data).
  - Hooks implemented:
    - `on_init`: Read agent config feeds section, register each feed via `POST /api/feeds`
    - `on_shutdown`: Deregister feeds
    - `post_action`: If action result contains data matching a feed's schema, publish to feed
  - Acceptance:
    - [ ] Agent with `feeds = [{ id = "eth-price", kind = "raw", rate_hz = 1.0 }]` in config auto-registers feed on startup
    - [ ] Feed deregistered on agent shutdown
    - [ ] Data published on matching action results
  - Size: M (1-2 days)

- [ ] **B.2.4** Implement relay topology documentation and health endpoint
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/status.rs` (extend existing)
  - Design decision (resolving spec gap): The relay is **roko-serve itself** for now. There is no separate relay service. roko-serve's WS/SSE endpoints ARE the relay. Dashboard connects to `ws://localhost:6677/api/ws`. In production, the same service runs on Railway.
  - Future: If relay needs to be separated (for scaling or NAT traversal), extract WS handling into a standalone binary. But don't do this prematurely.
  - Health endpoint: `GET /api/relay/health` returns `{ "ws_connections": N, "rooms": [...], "event_rate_per_sec": X }`.
  - Acceptance:
    - [ ] `GET /api/relay/health` returns WS connection count and room list
    - [ ] Dashboard can determine relay availability from this endpoint
  - Size: S (half day)

---

## Phase C: Inference gateway pipeline

**Architecture doc**: [07-gateway.md](../architecture/07-gateway.md)

**Estimated tasks**: 9 (reduced from 12 — routes and basic dispatch exist)
**Estimated effort**: 2-3 weeks (sequential foundation, parallel middle)
**Dependencies**: None (standalone, can run alongside Phase A)
**Key crates**: `roko-serve`, `roko-learn`, `roko-agent`

**Read**: `/Users/will/dev/nunchi/roko/roko/tmp/architecture/07-gateway.md` (full pipeline, 12 subsystems)

### What already exists (do not rebuild)

| Item | Location | Status |
|------|----------|--------|
| Gateway HTTP routes (32 functions) | `roko-serve/src/routes/gateway.rs` | **EXISTS** — `POST /api/inference/complete`, `GET /api/gateway/stats`, batch submit/status |
| CascadeRouter (model selection) | `roko-learn/src/cascade_router.rs` | **EXISTS** — LinUCB bandit + confidence stages |
| 8 LLM provider backends | `roko-agent/src/dispatcher/` | **EXISTS** — Claude CLI, Claude API, Codex, Cursor, OpenAI-compat, Ollama, Gemini, Perplexity |
| Cost tracking per-request | `roko-learn/src/costs_db.rs` | **EXISTS** — per-agent, per-model cost recording |

### Architecture decision: No separate `roko-gateway` crate

The roadmap spec assumed a standalone `roko-gateway` crate. After audit, the gateway logic belongs **inside `roko-serve`** because:
1. The gateway is a routing + dispatch layer, not a library with exportable abstractions
2. It uses `CascadeRouter` from `roko-learn` for model selection (already wired)
3. Provider backends already live in `roko-agent` (8 backends, well-tested)
4. Creating a new crate would duplicate the provider abstraction

The gateway pipeline stages below are implemented as **middleware/interceptors** in the existing `routes/gateway.rs` dispatch path.

### Batch C.1: Request pipeline stages

These stages run **before** the provider call. Each is a standalone module in `roko-serve/src/gateway/`.

- [ ] **C.1.1** Implement hash cache (L1 exact match)
  - Source: `bardo/apps/bardo-gateway/src/cache.rs` (reference)
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/gateway/hash_cache.rs` (new file in new `gateway/` module)
  - Dependencies: `moka` (async LRU cache), `blake3` (hashing)
  - Design:
    ```rust
    pub struct HashCache {
        cache: moka::future::Cache<[u8; 32], CachedResponse>,
    }

    pub struct CachedResponse {
        pub body: Bytes,
        pub cost_usd: f64,
        pub model: String,
        pub cached_at: Instant,
    }
    ```
  - Normalization (applied before hashing):
    - Strip UUIDs (`[0-9a-f]{8}-[0-9a-f]{4}-...-[0-9a-f]{12}`)
    - Strip ISO timestamps, `cch=` hashes, `CWD:` lines, `Date:` headers
    - Replace git status blocks with `[GIT_STATUS]` placeholder
    - Sort JSON keys alphabetically, sort tool definitions by name
  - Regime-aware TTL: Normal=3600s, Calm=7200s, Volatile=900s, Crisis=300s
  - Who determines regime? The current agent's `AdaptiveClock` regime, passed as metadata on the `InferenceRequest`. If no regime metadata, default to Normal (3600s).
  - Exclusions: Never cache responses with `tool_use` stop reason, <3 output tokens, or error responses
  - Capacity: 10,000 entries (configurable via `gateway.cache_max_entries`)
  - Acceptance:
    - [ ] Identical request returns cached response on second call
    - [ ] Normalized request (only timestamps differ) produces cache hit
    - [ ] tool_use responses not cached
    - [ ] Cache respects TTL based on regime
    - [ ] Eviction works when capacity reached (LRU)
    - [ ] Cache hit counter incremented (visible in `/api/gateway/stats`)
  - Size: M (2 days)

- [ ] **C.1.2** Implement semantic cache (L2 similarity match)
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/gateway/semantic_cache.rs` (new file)
  - Dependencies: `dashmap` (lock-free concurrent map)
  - Design:
    ```rust
    pub struct SemanticCache {
        entries: DashMap<u64, SimHashEntry>,  // fingerprint → entry
        max_entries: usize,                   // default 5,000
    }
    ```
  - SimHash algorithm (from 07-gateway.md):
    1. Tokenize on whitespace + punctuation
    2. Hash each token with a fast 64-bit hash (use `ahash`)
    3. For each of 64 bit positions: token hash bit 1 → increment counter, bit 0 → decrement
    4. Final fingerprint: 1 for each positive counter, 0 for negative
  - Match threshold: Hamming distance <= 3 bits. Configurable via `gateway.simhash_threshold` (default 3).
  - False positive rate: At threshold=3, ~1 in 40,000 random pairs match (acceptable for cache — worst case is serving a slightly wrong cached response, which the agent can recover from).
  - TTL: 7,200s fixed (not regime-aware — semantic matches are fuzzier)
  - Namespace isolation: Prefix cache key with workspace ID. Default namespace for single-user.
  - Same exclusions as L1.
  - Acceptance:
    - [ ] Two requests differing by a few words produce L2 cache hit
    - [ ] Requests differing significantly (Hamming > 3) miss
    - [ ] Namespace isolation prevents cross-tenant hits
    - [ ] Max entries enforced (evict oldest on overflow)
    - [ ] Stats: `l2_hits`, `l2_misses` counters
  - Size: M (2 days)

- [ ] **C.1.3** Implement loop detection (per-session)
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/gateway/loop_guard.rs` (new file)
  - Design:
    ```rust
    pub struct SessionLoopState {
        recent_calls: VecDeque<(String, [u8; 32])>,  // (tool_name, blake3(args))
        consecutive_identical: u32,
        tokens_since_progress: u64,
    }
    ```
  - Ring buffer: 16 entries, does not grow
  - Three detection rules (from 07-gateway.md):
    | Pattern | Trigger | Injected message |
    |---------|---------|-----------------|
    | Retry | Same tool + same args hash 5+ times consecutively | "You have called the same tool with the same arguments 5 times. Try a different approach." |
    | Oscillation | A→B→A→B pattern repeats 3+ full cycles (6 calls) | "You are oscillating between two actions. Break the loop by choosing a third option or stopping." |
    | Drift | 15,000+ output tokens without new tool_result | "You have generated 15K+ tokens without making progress. Either take a concrete action or stop." |
  - Injection: Prepend to system prompt on next request. Clears after one injection (one-shot).
  - Acceptance:
    - [ ] 5 identical tool calls → retry guidance injected
    - [ ] A-B-A-B-A-B pattern → oscillation guidance injected
    - [ ] 15K tokens with no tool_result → drift guidance injected
    - [ ] Guidance appears once, then clears
    - [ ] Counters: `loops_detected`, `loop_retry`, `loop_oscillation`, `loop_drift`
  - Size: M (1-2 days)

- [ ] **C.1.4** Implement output budgeting (per-model EMA cap)
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/gateway/output_budget.rs` (new file)
  - Design:
    ```rust
    pub struct ModelOutputStats {
        pub ema: f64,
        pub ema_sq: f64,       // for variance computation
        pub max_seen: u64,
        pub count: u64,
    }
    ```
  - Algorithm (from 07-gateway.md):
    - Alpha: 0.05 (5% weight to new observations)
    - Minimum 20 samples before p95 is trusted (before that, use model default cap)
    - p95 estimate: `ema + 2 * sqrt(ema_sq - ema²)`
    - Cap: `p95 * 1.5`, floor 1,024 tokens
  - Floor rationale: 1,024 is the minimum useful output size. Models consistently outputting <1024 tokens are likely doing short responses — capping lower would truncate them. The floor protects against cold-start underestimation.
  - Behavior:
    - No max_tokens set → auto-set to computed cap
    - Unreasonably high max_tokens (> 2x cap) → reduce to cap
    - Explicit max_tokens below cap → leave it alone
  - Acceptance:
    - [ ] After 20 requests, auto-cap is within 2x of actual p95
    - [ ] Floor of 1024 enforced even when EMA is low
    - [ ] Explicit user max_tokens below cap not overridden
    - [ ] Stats: `output_budgets_applied`, `output_tokens_bounded`
  - Size: S (1 day)

- [ ] **C.1.5** Implement tool pruning (per-session usage tracking)
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/gateway/tool_pruning.rs` (new file)
  - Never-prune list: `Bash`, `Read`, `Write`, `Edit`, `Glob`, `Grep`, `WebSearch`, `WebFetch`, `TaskCreate`, `TaskUpdate`, `TaskList`, `Agent`, `SendMessage`
  - Two-tier pruning (from 07-gateway.md):
    | Tier | Trigger | Logic |
    |------|---------|-------|
    | Session | 50+ requests in session | Remove tools never used in this session |
    | Global | <50 session requests, 50+ global | Remove tools never used globally |
  - Acceptance:
    - [ ] After 50 requests, unused tools removed from request
    - [ ] Core tools never removed
    - [ ] Stats: `tools_pruned`, `tool_tokens_saved`
  - Size: S (1 day)

- [ ] **C.1.6** Implement convergence detection and thinking cap
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/gateway/convergence.rs` (new file)
  - Convergence: SimHash of last 8 responses. Hamming distance ≤ 2 between consecutive = "similar". 3+ consecutive similar → inject guidance.
  - Thinking cap defaults: Opus=32K, Sonnet=16K, Haiku=4K. Only activates when thinking is enabled but budget_tokens absent. Never overrides explicit budgets.
  - Acceptance:
    - [ ] 3 similar responses → convergence guidance injected
    - [ ] Dissimilar response resets counter
    - [ ] Thinking budget auto-set when thinking enabled but no budget
    - [ ] Explicit thinking budget never overridden
  - Size: S (1 day)

### Batch C.2: Pipeline assembly and integration

- [ ] **C.2.1** Assemble pipeline stages into gateway middleware
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/gateway/mod.rs` (new module)
  - Design:
    ```rust
    pub struct GatewayPipeline {
        hash_cache: HashCache,
        semantic_cache: SemanticCache,
        loop_guard: DashMap<String, SessionLoopState>,  // keyed by session_id
        output_budget: DashMap<String, ModelOutputStats>, // keyed by model
        tool_pruner: ToolPruner,
        convergence: DashMap<String, ConvergenceState>,
    }

    impl GatewayPipeline {
        pub async fn process(&self, req: InferenceRequest) -> Result<InferenceResponse> {
            // 1. Loop detection → 2. Cache lookup (L1→L2) → 3. Tool pruning
            // → 4. Output budget → 5. Thinking cap → 6. Convergence check
            // → 7. Provider call → 8. Cache store → 9. Cost tracking
        }
    }
    ```
  - Wire into existing `routes/gateway.rs` handlers. The existing route functions call `pipeline.process()` instead of dispatching directly.
  - Acceptance:
    - [ ] Full pipeline runs end-to-end for a single inference request
    - [ ] Cache hit skips provider call (stages 3-6 skipped, returns cached)
    - [ ] Each stage's counters visible in `/api/gateway/stats`
    - [ ] Pipeline stages are individually disabled via config flags
    - [ ] Integration test: send request → verify all 9 stages ran (check counters)
  - Size: L (3 days)

- [ ] **C.2.2** Implement `InferenceHandle` for in-process agents
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/gateway/handle.rs` (new file)
  - Design (from 07-gateway.md):
    ```rust
    #[derive(Clone)]
    pub struct InferenceHandle {
        sender: mpsc::Sender<InferenceEnvelope>,
        agent_id: AgentId,
        budget: Arc<AtomicU64>,  // microdollars
    }
    ```
  - Channel-based: Agent sends request through channel, gateway processes it, response comes back via oneshot.
  - Budget enforcement: Before processing, check `budget.load()`. If insufficient, return error. After processing, subtract cost.
  - Acceptance:
    - [ ] In-process agent can call `handle.infer(request)` and get response
    - [ ] Agent never sees API keys
    - [ ] Budget decremented after each call
    - [ ] Budget exhaustion returns clear error
    - [ ] `handle.remaining_budget()` returns correct value
  - Size: M (1-2 days)

- [ ] **C.2.3** Implement provider fallback chain
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/gateway/fallback.rs` (new file)
  - Design: CascadeRouter returns `RouteDecision { preferred, fallback_1, fallback_2 }`. Gateway tries each in order. Fallback triggers on: 429, 503, timeout (30s).
  - Default hierarchies (when router has insufficient data):
    - Anthropic: Opus → Sonnet → Haiku
    - OpenAI: GPT-4o → GPT-4o-mini
    - Cross-provider: Sonnet → GPT-4o → Haiku
  - Fallback metadata: Response includes `"fallback": true` and `"original_model"` when fallback served. Router records event to adjust weights.
  - Acceptance:
    - [ ] Primary model 429 → fallback_1 tried automatically
    - [ ] All three fail → return 503 to agent
    - [ ] Fallback response includes metadata markers
    - [ ] Router weight update on fallback event
    - [ ] Timeout of 30s per provider attempt
  - Size: M (1-2 days)

---

## Phase D: Auth and config

**Architecture docs**: [08-auth.md](../architecture/08-auth.md), [16-config.md](../architecture/16-config.md)

**Estimated tasks**: 5 (reduced — scope enforcement and team routes already exist)
**Estimated effort**: 1-2 weeks
**Dependencies**: Phase C (gateway holds API keys)
**Key crates**: `roko-serve`, `roko-core`, `roko-cli`

### What already exists (do not rebuild)

| Item | Location | Status |
|------|----------|--------|
| API key CRUD (create/list/revoke) | `roko-serve/src/routes/auth.rs` (212 LOC) | **EXISTS** — SHA-256 hashing, plaintext once |
| Auth middleware (4-source chain) | `roko-serve/src/routes/middleware.rs` (1063 LOC) | **EXISTS** — X-Api-Key, Bearer/API key, Bearer/agent token, Bearer/Privy JWT |
| Scope enforcement (`require_scope()`) | `roko-serve/src/routes/middleware.rs` | **EXISTS** — admin > agent:write/plan:write > read hierarchy |
| `ServeAuthConfig` + `ApiKeyEntry` | `roko-core/src/config/schema.rs` (lines 3626-3671) | **EXISTS** — name, key_hash, scope, created_at, expires_at |
| CLI auth resolver (4-source chain) | `roko-cli/src/auth.rs` (249 LOC) | **EXISTS** — flag, env, config, credentials.json |
| Credential storage | `roko-cli/src/credentials.rs` (196 LOC) | **EXISTS** — `~/.roko/credentials.json`, 0600 perms, atomic write |
| Agent token validation | `roko-serve/src/routes/middleware.rs` | **EXISTS** — `try_agent_token()` validates SHA-256 hash + expiry |
| Privy JWT structural validation | `roko-serve/src/routes/middleware.rs` | **EXISTS** — `is_structurally_valid_jwt()`, calls `jwks_cache.validate()` |
| Team management routes | `roko-serve/src/routes/team.rs` (446 LOC) | **EXISTS** — /team/me, /team/members, /team/invite, role hierarchy |
| Secrets module | `roko-core/src/secrets/` | **EXISTS** — profile-aware secret storage |

### Remaining work

- [ ] **D.1** Implement real Privy JWKS signature verification
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/middleware.rs` (modify `try_privy_jwt()`)
  - Current: `jwks_cache.validate()` exists but does **structural validation only** — it verifies the JWT has 3 base64url segments and extracts claims, but does NOT verify the cryptographic signature against Privy's public keys.
  - Change: Fetch JWKS from `https://auth.privy.io/.well-known/jwks.json`, cache keys, verify RS256 signature.
  - Design:
    ```rust
    pub struct JwksCache {
        keys: Arc<RwLock<HashMap<String, DecodingKey>>>,  // kid → key
        last_fetch: AtomicInstant,
        privy_app_id: String,
    }

    impl JwksCache {
        pub async fn validate(&self, token: &str) -> Result<JwtClaims> {
            // 1. Decode header to get `kid`
            // 2. Look up key in cache
            // 3. If key not found or cache > 1 hour: refetch JWKS
            // 4. Verify RS256 signature using `jsonwebtoken` crate
            // 5. Validate: exp, aud (matches privy_app_id), iss
            // 6. Return claims (sub, email, wallet)
        }
    }
    ```
  - JWKS caching strategy:
    - Normal: refetch every 1 hour
    - On unknown `kid`: immediate refetch (key rotation)
    - On fetch failure: use stale cache with warning (stale-while-revalidate)
    - Stale cache > 24 hours: log error, still accept (eventual consistency)
  - Dependencies: `jsonwebtoken` crate (already in ecosystem), `reqwest` for JWKS fetch
  - Acceptance:
    - [ ] Valid Privy JWT with correct signature → authenticated, claims extracted
    - [ ] Expired JWT → 401 with `"error": "token_expired"`
    - [ ] JWT with unknown `kid` → JWKS refetched, retry validation
    - [ ] JWKS endpoint down → stale cache used, warning logged
    - [ ] JWT for wrong `aud` (different Privy app) → rejected
    - [ ] No `PRIVY_APP_SECRET` required — public key verification only
  - Size: M (2 days)

- [ ] **D.2** Implement agent token create/refresh/revoke endpoints
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/auth.rs` (extend existing)
  - Current: Agent token validation exists (`try_agent_token()` in middleware.rs checks `token_hash` and `token_expires_at`). But there are no endpoints to **create** or **revoke** tokens.
  - API:
    ```
    POST   /api/agents/{id}/token     — Issue new bearer token (scope: agent:write required)
      Request: { "ttl_days": 30 }
      Response: { "token": "roko-agt-...", "expires_at": "2026-05-24T..." }
      Token returned in plaintext ONCE. Server stores SHA-256 hash.

    DELETE /api/agents/{id}/token     — Revoke agent token
      Response: 204 No Content
      Token immediately invalid.

    POST   /api/agents/{id}/token/refresh  — Rotate token (old valid for 5min grace)
      Response: { "token": "roko-agt-...", "expires_at": "..." }
      Old token valid for 5 minutes after rotation (prevents race conditions).
    ```
  - Token format: `roko-agt-{base64url(32 random bytes)}` (48 chars total)
  - Storage: `token_hash` and `token_expires_at` fields on `DiscoveredAgent` (already present)
  - Grace period: On refresh, old `token_hash` stored in `previous_token_hash` field with `grace_expires_at = now + 5min`. Both tokens validate during grace period.
  - Acceptance:
    - [ ] `POST /api/agents/coder-1/token` returns plaintext token
    - [ ] Agent authenticates with the token (passes `try_agent_token()`)
    - [ ] Token expires after `ttl_days` (default 30)
    - [ ] `DELETE` immediately invalidates token
    - [ ] Refresh: old token works for 5 minutes, new token works immediately
    - [ ] Only `agent:write` or `admin` scope can create tokens
  - Size: M (1-2 days)

- [ ] **D.3** Implement device flow for headless CLI login
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/credentials.rs` (extend existing)
  - Current: `roko login` exists but uses interactive browser flow. Headless servers (SSH, containers) can't open a browser.
  - Design (RFC 8628 Device Authorization Grant):
    ```
    1. CLI calls POST /api/auth/device/start
       Response: { "device_code": "...", "user_code": "ABCD-1234",
                   "verification_uri": "https://app.nunchi.dev/device",
                   "interval": 5, "expires_in": 900 }

    2. CLI displays:
       "Visit https://app.nunchi.dev/device and enter code: ABCD-1234"

    3. CLI polls POST /api/auth/device/poll { "device_code": "..." }
       Until: { "access_token": "...", "token_type": "bearer" }
       Or: 15 minutes timeout

    4. On success: store credential in ~/.roko/credentials.json
    ```
  - Server-side: roko-serve holds pending device codes in memory (DashMap, 15min TTL). When user authenticates on the web and enters the code, the pending entry is marked as authorized.
  - Acceptance:
    - [ ] `roko login --device` displays verification URL and user code
    - [ ] User completes auth on another device via browser
    - [ ] CLI detects completion and stores credential
    - [ ] Timeout after 15 minutes with clear message
    - [ ] Works via SSH (no browser required on CLI machine)
  - Size: L (2-3 days)

- [ ] **D.4** Wire secrets rotation for provider keys
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/secrets/` (extend existing)
  - Current: Secrets module exists with profile-aware storage. Missing: hot rotation without restart.
  - Design:
    ```rust
    // roko config secrets rotate anthropic-api-key
    // 1. Prompt for new key value
    // 2. Store new key in secrets store
    // 3. Signal roko-serve to reload (if running) via POST /api/config/reload
    // 4. Old key still valid until next config reload cycle
    ```
  - Integration with gateway: If `roko-gateway` (Phase C) uses `KeyRing`, rotation appends the new key to the ring. Old key stays in ring until manually removed.
  - Acceptance:
    - [ ] `roko config secrets rotate <key>` updates the secret without restart
    - [ ] Running roko-serve picks up new key within 30 seconds (config hot-reload)
    - [ ] Old key continues working during transition
    - [ ] Rotation logged for audit trail
  - Size: S (1 day)

- [ ] **D.5** Add per-route scope matrix documentation + test
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/middleware.rs` (verify existing)
  - Current: `require_scope()` middleware exists with hierarchy. Verify all routes have correct scope requirements.
  - Scope matrix:
    | Scope | Allowed methods | Allowed routes |
    |-------|----------------|----------------|
    | `read` | GET | Any |
    | `agent:write` | POST/PUT/DELETE | `/api/agents/*`, `/api/agents/*/token` |
    | `plan:write` | POST/PUT/DELETE | `/api/plans/*`, `/api/prd/*` |
    | `admin` | * | * |
  - Acceptance:
    - [ ] API key with `scope: "read"` → GET /api/agents succeeds, POST /api/agents fails with 403
    - [ ] API key with `scope: "agent:write"` → POST /api/agents succeeds, POST /api/plans fails
    - [ ] Integration test covering all scope × route combinations
  - Size: S (half day)

---

## Phase E: Knowledge and pheromones

**Architecture doc**: [09-knowledge.md](../architecture/09-knowledge.md)

**Estimated tasks**: 8 (reduced — knowledge store, decay, HDC vectors, anti-knowledge all exist)
**Estimated effort**: 2-3 weeks
**Dependencies**: Phase A (extension hooks for knowledge publishing)
**Key crates**: `roko-neuro`, `roko-chain`, `roko-primitives`, `roko-dreams`, `roko-serve`

### What already exists (do not rebuild)

| Item | Location | Status |
|------|----------|--------|
| `KnowledgeStore` (append-only JSONL) | `roko-neuro/src/knowledge_store.rs` (1500+ LOC) | **EXISTS** — ingest, query, decay, gc, resurrection |
| `KnowledgeEntry` struct | `roko-neuro/src/lib.rs` | **EXISTS** — id, content, kind, tier, confidence, source_episodes |
| `KnowledgeTier` enum (Transient→Persistent) | `roko-neuro/src/lib.rs` | **EXISTS** — 0.1x to 5.0x decay multipliers |
| `KnowledgeKind` enum (6 variants) | `roko-neuro/src/lib.rs` | **EXISTS** — Insight, Heuristic, Warning, CausalLink, StrategyFragment, AntiKnowledge |
| Anti-knowledge conflict detection | `roko-neuro/src/knowledge_store.rs` | **EXISTS** — HDC similarity thresholds: 0.5 warn, 0.7 discount, 0.9 reject |
| Tier progression (Transient→Confirmed→Durable) | `roko-neuro/src/tier_progression.rs` | **EXISTS** |
| Temporal reasoning (Allen interval algebra) | `roko-neuro/src/temporal.rs` | **EXISTS** — 13 interval relations |
| `HdcVector` (10,240-bit, 160×u64) | `roko-primitives/src/hdc.rs` | **EXISTS** — bind (XOR), bundle (majority vote), Hamming similarity |
| HDC encoder integration | `roko-neuro/src/hdc.rs` | **EXISTS** — feature-gated |
| Distiller (insight consolidation) | `roko-neuro/src/distiller.rs` | **EXISTS** |
| Episode→knowledge extraction | `roko-neuro/src/episode_completion.rs` | **EXISTS** |
| Knowledge query endpoint | `roko-serve/src/routes/neuro.rs` (73 LOC) | **EXISTS** — `POST /api/neuro/query` |
| Dream cycle runner | `roko-dreams/src/` | **EXISTS** — hypnagogia, imagination, cycle (no runtime trigger) |
| Coordination primitives (Pheromone type) | `roko-orchestrator/src/coordination.rs` | **EXISTS** — 7 built-in kinds |

### Batch E.1: Knowledge lifecycle routes

The knowledge store is fully wired internally. What's missing is the **HTTP surface** for publish/validate/challenge and the **A-MAC admission gate**.

- [ ] **E.1.1** Extend knowledge routes with full CRUD + lifecycle
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/neuro.rs` (extend existing)
  - Current: Only `POST /api/neuro/query` exists
  - Add:
    ```
    GET    /api/knowledge                      — List entries (paginated, filtered by kind/tier/min_confidence)
      Query: ?kind=Heuristic&tier=Confirmed&min_confidence=0.5&limit=50&cursor={id}

    GET    /api/knowledge/{id}                 — Get single entry with full metadata

    POST   /api/knowledge/publish              — Publish new entry
      Request: { content, kind, tags: [], confidence: 0.5 }
      Calls KnowledgeStore::ingest() internally
      Returns: { id, tier: "Transient", confidence }

    POST   /api/knowledge/{id}/validate        — Validate existing entry
      Request: { evidence: "supporting observation..." }
      Effect: confidence += 0.05 * (1.0 - confidence), validated_count++, decay clock reset

    POST   /api/knowledge/{id}/challenge       — Challenge entry
      Request: { reason: "contradicts observation X" }
      Effect: challenged_count++. 3+ challenges → confidence halved, flagged

    GET    /api/knowledge/stats                — Store statistics (entry count by tier/kind, decay rate, gc stats)
    ```
  - Acceptance:
    - [ ] Publish entry → appears in store with Transient tier
    - [ ] Validate → confidence increases, decay clock resets
    - [ ] 3 challenges → confidence halved
    - [ ] Stats endpoint returns correct counts
    - [ ] Pagination works (cursor-based, 50 entries default)
  - Size: M (2 days)

- [ ] **E.1.2** Implement A-MAC 5-factor admission gate
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-neuro/src/admission.rs` (new file)
  - Wire into: `KnowledgeStore::ingest()` — call A-MAC before storing
  - Design:
    ```rust
    pub struct AmacGate {
        store: Arc<KnowledgeStore>,
    }

    pub enum AmacResult {
        Admit,
        Reject { reason: String, factor: String },
    }

    impl AmacGate {
        pub fn evaluate(&self, entry: &KnowledgeEntry) -> AmacResult {
            // 1. Similarity: cosine sim > 0.95 to any existing → Reject("duplicate")
            // 2. Novelty: cosine sim < 0.3 to all existing → flag as novel (bonus confidence)
            // 3. Contradiction: topic sim > 0.7 AND assertion sim < -0.3 → Reject("contradicts {id}")
            //    Fallback: skip if assertion vectors unavailable (pre-HDC entries)
            // 4. Relevance: at least 1 tag keyword overlaps with agent's domain tags → pass
            //    No domain tags on entry → pass (permissive for untagged entries)
            // 5. Confidence: source episode gate pass rate >= 0.5 → pass
            //    No source episode → pass (manually created entries exempt)
        }
    }
    ```
  - Logging: All rejections logged to `.roko/learn/admission-rejections.jsonl` with timestamp, reason, entry content hash
  - Acceptance:
    - [ ] Near-duplicate (sim > 0.95) → rejected with "duplicate" reason
    - [ ] Contradictory entry (topic similar, assertion opposite) → rejected with "contradicts {id}"
    - [ ] Novel entry (sim < 0.3 to all) → admitted with confidence bonus (+0.05)
    - [ ] Low-credibility source (gate pass rate < 0.5) → rejected
    - [ ] Entries without HDC vectors → skip contradiction check, pass through
    - [ ] Rejections logged to admission-rejections.jsonl
    - [ ] Unit test: duplicate → reject, novel → admit, contradiction → reject
  - Size: M (2-3 days)

### Batch E.2: Pheromone system

Pheromone types exist in `roko-orchestrator/src/coordination.rs` (7 built-in kinds: threat, opportunity, wisdom, alpha, pattern, anomaly, consensus). What's missing is the **pheromone field store** and **REST routes**.

- [ ] **E.2.1** Implement pheromone field store
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-neuro/src/pheromone.rs` (new file)
  - Design:
    ```rust
    pub struct PheromoneField {
        deposits: Vec<PheromoneDeposit>,
        path: PathBuf,  // .roko/neuro/pheromones.jsonl
    }

    pub struct PheromoneDeposit {
        pub id: String,
        pub ptype: String,                // from coordination.rs kinds
        pub intensity: f64,               // 0.0..=1.0
        pub location_hash: [u8; 32],      // blake3 hash of context (task, file, topic)
        pub depositor: String,            // agent_id
        pub metadata: serde_json::Value,
        pub deposited_at: DateTime<Utc>,
        pub half_life_secs: u64,          // default 3600 (1 hour)
    }
    ```
  - Decay formula: `current_intensity = initial * e^(-0.693 * elapsed_secs / half_life_secs)` (exponential decay)
  - Query: `sense(location_hash, radius, min_intensity) -> Vec<PheromoneDeposit>` — returns active deposits near a location, sorted by intensity
  - Reinforcement: `reinforce(id)` — resets decay clock, boosts intensity by 10% (capped at 1.0)
  - GC: Remove deposits with intensity < 0.01 on every query (lazy cleanup)
  - Acceptance:
    - [ ] Deposit pheromone → persists to JSONL
    - [ ] Query by location → returns deposits with current (decayed) intensity
    - [ ] After 1 hour (default half-life), intensity is ~50% of initial
    - [ ] Reinforcement resets decay clock
    - [ ] GC removes deposits below 0.01 intensity
  - Size: M (2 days)

- [ ] **E.2.2** Add pheromone REST routes
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/neuro.rs` (extend existing)
  - API:
    ```
    POST   /api/pheromones/deposit             — Deposit signal
      Request: { ptype, intensity, location_hash, metadata, half_life_secs }

    GET    /api/pheromones                     — List active signals (decayed intensities)
      Query: ?location={hash}&min_intensity=0.1&limit=20

    POST   /api/pheromones/{id}/reinforce      — Reinforce existing signal

    GET    /api/pheromones/summary             — Per-type aggregate intensity
    ```
  - Wire to WS room: deposits broadcast to `pheromone:*` room for real-time dashboard
  - Acceptance:
    - [ ] Deposit via REST → visible in query
    - [ ] Decayed intensity shown (not original)
    - [ ] Summary aggregates correctly across types
    - [ ] WS subscribers receive deposit events
  - Size: S (1 day)

### Batch E.3: Dream consolidation trigger

Dream cycle logic exists in `roko-dreams/` but has **no runtime trigger** (no cron, no event-driven activation).

- [ ] **E.3.1** Wire dream consolidation triggers
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs` (add trigger check)
  - Triggers (from 09-knowledge.md):
    - `idle_timeout`: 5 minutes of no agent activity → trigger delta-tick dream cycle
    - `episode_threshold`: 50 episodes since last dream → trigger dream cycle
    - Manual: `roko knowledge dream run` (already exists as CLI command)
  - Design: In the orchestrator's main loop, after each task completion, check:
    ```rust
    if episodes_since_dream >= 50 || idle_duration > Duration::from_secs(300) {
        run_dream_cycle().await?;
        episodes_since_dream = 0;
    }
    ```
  - Dream phases (from existing `roko-dreams/`):
    1. NREM replay: cluster high-surprise episodes → Transient insights
    2. REM imagination: counterfactual generation, evaluate against gates
    3. Integration: D1 (episode→insight), D2 (insight→heuristic with 5+ confirmations), D3 (playbook)
  - Acceptance:
    - [ ] After 50 episodes, dream cycle runs automatically
    - [ ] After 5 minutes idle, dream cycle runs
    - [ ] Dream cycle produces new KnowledgeEntry items in store
    - [ ] `roko knowledge dream report` shows latest cycle results
    - [ ] Dream doesn't run during active plan execution (only between tasks or on idle)
  - Size: M (1-2 days)

### Batch E.4: On-chain knowledge (Phase 2+, deferred)

On-chain knowledge via InsightStore contract is Phase 2+ work. The off-chain knowledge system is complete and sufficient for self-hosting. Document the interface here for future reference but do not implement now.

- [ ] **E.4.1** Define `OnChainEntry` struct and InsightStore Solidity interface (types only)
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/insight_store.rs` (new file, types only)
  - Design:
    ```rust
    /// On-chain knowledge entry. Off-chain content, on-chain commitment.
    pub struct OnChainEntry {
        pub entry_id: u256,
        pub kind: u8,                    // KnowledgeKind as u8
        pub content_hash: [u8; 32],      // SHA-256 of off-chain content
        pub confidence: u16,             // fixed-point 0..65535
        pub tier: u8,
        pub author: Address,
        pub created_at: u64,             // block timestamp
        pub validated_count: u32,
        pub challenged_count: u32,
        pub hdc_fingerprint: Vec<u8>,    // 1,280 bytes (PP-HDC)
        pub frozen: bool,
    }
    ```
  - Acceptance:
    - [ ] Types compile and serialize
    - [ ] Solidity interface documented as `sol!` macro (not deployed)
  - Size: S (half day, types only)

---

## Phase F: Groups and coordination

**Architecture doc**: [10-groups.md](../architecture/10-groups.md)

**Estimated tasks**: 6 (reduced — coordination primitives and team routes exist)
**Estimated effort**: 1-2 weeks
**Dependencies**: Phase A (agent runtime), Phase B (relay for group rooms)
**Key crates**: `roko-core`, `roko-serve`, `roko-orchestrator`

### What already exists (do not rebuild)

| Item | Location | Status |
|------|----------|--------|
| Team management routes | `roko-serve/src/routes/team.rs` (446 LOC) | **EXISTS** — /team/me, /team/members, /team/invite, role hierarchy |
| Pheromone type + 7 built-in kinds | `roko-orchestrator/src/coordination.rs` | **EXISTS** — threat, opportunity, wisdom, alpha, pattern, anomaly, consensus |
| `Collective`, `SubnetId`, `CohortStrategy` | `roko-orchestrator/src/coordination.rs` | **EXISTS** — agent group management |
| Process group management (Unix) | `roko-agent/src/process/group.rs` (150 LOC) | **EXISTS** — setpgid, kill_process_group |
| Agent registry (on-chain) | `roko-chain/src/agent_registry.rs` | **EXISTS** — ERC-721 soulbound passports |
| Reputation registry (on-chain) | `roko-chain/src/reputation_registry.rs` | **EXISTS** — 7-domain EMA scoring |

### Key distinction: Groups vs Teams

**Teams** (existing) are **human membership** — users with roles (owner, admin, member, viewer) who manage agents. Storage: `.roko/team/members.json`.

**Groups** (new) are **agent-to-agent coordination** — agents form groups to collaborate on tasks, share pheromones, and coordinate work. Groups may span multiple users' agents (cross-user groups require invitation approval).

These are complementary concepts. Team routes stay as-is. Group routes are new.

- [ ] **F.1** Define `Group` type with agent membership and coordination mode
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/group.rs` (new file)
  - Design:
    ```rust
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Group {
        pub id: String,                      // UUID
        pub name: String,                    // unique per owner
        pub description: String,
        pub owner: String,                   // user ID who created
        pub members: Vec<GroupMember>,
        pub coordination: CoordinationMode,
        pub config: GroupConfig,
        pub relay_room: String,              // "group:{id}"
        pub created_at: DateTime<Utc>,
    }

    pub struct GroupMember {
        pub agent_id: String,
        pub agent_owner: String,             // user who owns this agent (may differ from group owner)
        pub role: GroupRole,
        pub joined_at: DateTime<Utc>,
    }

    pub enum GroupRole { Leader, Member, Observer }

    pub enum CoordinationMode {
        Stigmergic,          // pheromone-based, no direct messaging
        Pipeline,            // DAG execution via cluster (task ordering)
        Broadcast,           // direct messages to group room
        LeaderFollower,      // leader assigns tasks
    }

    pub struct GroupConfig {
        pub max_members: Option<usize>,      // default: None (unlimited)
        pub auto_accept_same_owner: bool,    // default: true
        pub knowledge_policy: KnowledgePolicy,
        pub pheromone_decay_rate: f64,       // default: 0.02 (~1h half-life)
    }

    pub enum KnowledgePolicy {
        Open,           // any member can publish
        WriteLeader,    // only leader publishes, members read
        Curated,        // leader approves before publish
    }
    ```
  - Storage: `.roko/groups/{id}.json` (one file per group)
  - Acceptance:
    - [ ] Group type serializes/deserializes correctly
    - [ ] All fields have sensible defaults
    - [ ] GroupRole ordering: Leader > Member > Observer
  - Size: S (1 day)

- [ ] **F.2** Implement group CRUD + membership routes
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/groups.rs` (new file)
  - API:
    ```
    POST   /api/groups                           — Create group (owner = authenticated user)
      Request: { name, description, coordination: "stigmergic", config: {} }
      Response: { id, name, relay_room: "group:{id}", ... }

    GET    /api/groups                           — List groups (owned + joined)
    GET    /api/groups/{id}                      — Group detail with member list
    PATCH  /api/groups/{id}                      — Update group config (owner only)
    DELETE /api/groups/{id}                      — Delete group (owner only)

    POST   /api/groups/{id}/invite               — Invite agent to group
      Request: { agent_id, role: "member" }
      Same-owner agent: joins instantly (auto_accept_same_owner = true)
      Cross-user agent: creates pending invitation, notifies agent owner

    GET    /api/groups/{id}/invitations          — Pending invitations
    POST   /api/invitations/{inv_id}/accept      — Agent owner accepts invitation
    POST   /api/invitations/{inv_id}/reject      — Agent owner rejects invitation

    GET    /api/groups/{id}/members              — List members with roles
    PATCH  /api/groups/{id}/members/{agent_id}   — Update member role (owner/leader only)
    DELETE /api/groups/{id}/members/{agent_id}   — Remove member (owner or agent's own owner)
    ```
  - Cross-user invitation flow:
    1. Group owner invites agent `coder-1` (owned by user B)
    2. Invitation stored in `.roko/groups/{id}/invitations.json`
    3. Notification published to user B's notification room
    4. User B calls `POST /api/invitations/{inv_id}/accept`
    5. Agent added to group, subscribed to group relay room
    6. Invitation expires after 24 hours if not accepted
  - Acceptance:
    - [ ] Create group → stored in `.roko/groups/{id}.json`
    - [ ] Same-owner agent invitation → instant join, no invitation record
    - [ ] Cross-user invitation → pending until accepted
    - [ ] Invitation expires after 24h
    - [ ] Owner can remove any member; agent owner can remove their own agent
    - [ ] Delete group → all members removed, relay room cleaned
  - Size: L (3 days)

- [ ] **F.3** Wire group relay rooms to WS subscription system
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/groups.rs` (extend)
  - Relay room hierarchy:
    ```
    group:{id}                  — lifecycle events + broadcast messages
    group:{id}:knowledge        — knowledge publish/validate/challenge
    group:{id}:pheromones       — pheromone deposits/decay
    group:{id}:coordination     — task assignment/completion (leader-follower mode)
    ```
  - When agent joins group: auto-subscribe to all group rooms
  - When agent leaves: unsubscribe from all group rooms
  - Event types:
    ```
    group.member_joined, group.member_left, group.message,
    group.knowledge_published, group.pheromone_deposited,
    group.task_assigned, group.task_completed
    ```
  - Acceptance:
    - [ ] Agent joins group → receives events from group rooms
    - [ ] Agent leaves group → stops receiving events
    - [ ] Group message broadcast → all members receive via WS
    - [ ] Events include correct room and event_type in WsEnvelope
  - Size: M (1-2 days)

- [ ] **F.4** Implement group context injection into agent system prompt
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs` (extend `dispatch_agent_with()`)
  - Design: When dispatching an agent that belongs to a group, inject group context into the system prompt:
    ```
    ## Group context: {group_name}
    Coordination mode: {mode}
    Members: {member_list with roles}
    Recent signals: {top 5 pheromone deposits by intensity}
    Recent knowledge: {top 3 knowledge entries by recency}
    ```
  - Context budget: Group context capped at 500 tokens. If multiple groups, cap at 1000 tokens total (prioritize by group activity).
  - Acceptance:
    - [ ] Agent in group sees group context in system prompt
    - [ ] Context respects token budget (500 per group, 1000 total)
    - [ ] Agent in no groups → no group section in prompt
    - [ ] Recent pheromones sorted by decayed intensity
  - Size: S (1 day)

- [ ] **F.5** Implement leader-follower coordination mode
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/groups.rs` (extend)
  - API:
    ```
    POST   /api/groups/{id}/tasks               — Leader assigns task to member
      Request: { agent_id, description, priority }
      Publishes to group:{id}:coordination room

    GET    /api/groups/{id}/tasks               — List pending/active tasks
    POST   /api/groups/{id}/tasks/{task_id}/complete  — Member reports completion
    ```
  - Design: Leader agent posts task assignments. Member agents pick them up in their tick cycle (via group room subscription). On completion, member posts result to coordination room.
  - Assignment strategies (configurable in GroupConfig):
    - `RoundRobin`: rotate through members
    - `CapabilityMatch`: match task tags to agent profile (e.g., coding task → coding agent)
    - `LoadBalanced`: assign to least-busy member (fewest active tasks)
  - Acceptance:
    - [ ] Leader assigns task → member receives via WS
    - [ ] Member completes → leader notified
    - [ ] Round-robin distributes evenly across members
    - [ ] Capability match routes coding tasks to coding agents
  - Size: M (2 days)

- [ ] **F.6** Implement group-scoped pheromone field
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-neuro/src/pheromone.rs` (extend from E.2.1)
  - Design: Pheromone deposits can be **global** (visible to all agents) or **group-scoped** (visible only to group members).
  - Add `group_id: Option<String>` to `PheromoneDeposit`. If set, only agents in that group can query it.
  - API: `POST /api/groups/{id}/pheromones/deposit` creates group-scoped deposit
  - Acceptance:
    - [ ] Group-scoped deposit not visible to non-members
    - [ ] Group members see group-scoped deposits in their sense() queries
    - [ ] Global deposits visible to everyone regardless of group
  - Size: S (1 day)

---

## Phase G: Arenas, evals, and bounties

**Architecture doc**: [11-arenas.md](../architecture/11-arenas.md)

**Estimated tasks**: 7 (reduced — on-chain marketplace and bounty reading exist)
**Estimated effort**: 2-3 weeks
**Dependencies**: Phase A (agents to compete), Phase E (reputation feeds from arenas)
**Key crates**: `roko-serve`, `roko-learn`, `roko-gate`, `roko-chain`

### What already exists (do not rebuild)

| Item | Location | Status |
|------|----------|--------|
| `MarketplaceJob` state machine (7 states) | `roko-chain/src/marketplace.rs` | **EXISTS** — Posted→Assigned→InProgress→Submitted→Settled/Disputed/Expired |
| 3 hiring models (RandomVRF, BlindAuction, DirectHire) | `roko-chain/src/marketplace.rs` | **EXISTS** |
| Escrow + 4-level dispute resolution | `roko-chain/src/marketplace.rs` | **EXISTS** |
| On-chain bounty reader | `roko-serve/src/routes/chain.rs` | **EXISTS** — `GET /api/chain/bounties` reads first 100 jobs |
| On-chain agent count reader | `roko-serve/src/routes/chain.rs` | **EXISTS** — `GET /api/chain/agents` |
| `EvalGenerator` (eval generation from task specs) | `roko-gate/src/eval_generator.rs` | **EXISTS** — ExampleBased, PropertyBased, MutationBased strategies |
| `ValidationRegistry` (work proofs + attestation) | `roko-chain/src/validation_registry.rs` (274 LOC) | **EXISTS** |
| Job marketplace routes | `roko-serve/src/routes/jobs.rs` | **EXISTS** — full job lifecycle CRUD |

### Key design constraints (from 11-arenas.md)

1. **No self-grading**: Evals never use LLM output to judge LLM output. Ground truth from oracles, test suites, humans, or chain state.
2. **Declarative scoring**: Every arena declares scoring at registration; participants know rules beforehand.
3. **Escrow before execution**: Bounties lock funds before agents start.
4. **VCG for matching**: Welfare-maximizing allocation + truthful bidding.

- [ ] **G.1** Implement arena registry with lifecycle state machine
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/arenas.rs` (new file)
  - Design:
    ```rust
    pub enum ArenaState { Draft, Active, Paused, Concluded }

    pub struct Arena {
        pub id: String,
        pub name: String,
        pub description: String,
        pub state: ArenaState,
        pub task_source: TaskSource,         // Static | Procedural | Adversarial
        pub scoring: ScoringFunction,        // Binary | Continuous | Composite
        pub aggregation: AggregationRule,    // BestOf | AverageLastN | EWMA | Median
        pub creator: String,
        pub prize_pool_usdc: u64,
        pub max_attempts_per_agent: u64,
        pub deadline: Option<DateTime<Utc>>,
        pub created_at: DateTime<Utc>,
    }
    ```
  - API:
    ```
    POST   /api/arenas                            Create arena (state=Draft)
    GET    /api/arenas                            List (filter by state, category; paginated)
    GET    /api/arenas/{id}                       Arena detail
    POST   /api/arenas/{id}/start                 Draft → Active
    POST   /api/arenas/{id}/pause                 Active → Paused
    POST   /api/arenas/{id}/conclude              Active → Concluded (triggers settlement)
    POST   /api/arenas/{id}/attempts              Submit attempt (202 Accepted, async evaluation)
    GET    /api/arenas/{id}/attempts              List attempts (paginated)
    GET    /api/arenas/{id}/attempts/{attempt_id} Attempt detail with gate verdicts
    ```
  - State transitions: Draft→Active→Paused→Active→Concluded (Paused is toggleable)
  - Storage: `.roko/arenas/{id}.json` + `.roko/arenas/{id}/attempts/` directory
  - Acceptance:
    - [ ] Create arena in Draft state
    - [ ] Start transitions to Active (rejects if no task source configured)
    - [ ] Submit attempt: agent runs against task, gate pipeline evaluates, score recorded
    - [ ] Conclude: no new attempts accepted, leaderboard finalized
    - [ ] Invalid transition (e.g., Draft→Concluded) returns 400
  - Size: L (3 days)

- [ ] **G.2** Implement leaderboard computation
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/arenas.rs` (extend)
  - API: `GET /api/arenas/{id}/leaderboard?limit=50&aggregation=best_of`
  - Design: Leaderboard is a **derived view** recomputed from attempt records using the arena's configured aggregation rule:
    - `BestOf`: highest score per agent
    - `AverageLastN { n }`: average of last N attempts
    - `EWMA { alpha }`: exponentially weighted moving average
    - `Median`: median of all attempts
  - Caching: Recompute on each new attempt completion. Cache in memory (DashMap). Invalidate on new attempt.
  - Response:
    ```json
    {
      "entries": [
        { "rank": 1, "agent_id": "...", "score": 0.95, "attempts": 12, "best_score": 0.98, "avg_score": 0.92 },
        ...
      ],
      "total_agents": 45,
      "total_attempts": 312
    }
    ```
  - Acceptance:
    - [ ] Leaderboard ranks agents by aggregated score
    - [ ] BestOf: highest single attempt score wins
    - [ ] EWMA: recent attempts weighted more heavily
    - [ ] New attempt → leaderboard updates within 1s
  - Size: M (1-2 days)

- [ ] **G.3** Implement eval registry with ground-truth measurement
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/evals.rs` (new file)
  - Existing: `EvalGenerator` in roko-gate generates eval templates. This task adds the **registry** and **execution** layer.
  - Design:
    ```rust
    pub struct Eval {
        pub id: String,
        pub name: String,
        pub domain: String,
        pub scoring: ScoringFunction,
        pub ground_truth: GroundTruthSource,
        pub creator: String,
        pub created_at: DateTime<Utc>,
    }

    pub enum GroundTruthSource {
        TestSuite { suite_path: String, runtime: String, timeout_secs: u64 },
        BenchmarkDataset { dataset_path: String, comparison: ComparisonMethod },
        Oracle { endpoint: String, response_schema: String },
    }
    ```
  - API:
    ```
    POST   /api/evals                             Register eval
    GET    /api/evals                             List evals (filter by domain)
    GET    /api/evals/{id}                        Eval detail
    POST   /api/evals/{id}/run                    Run agent through eval (returns 202, async)
    GET    /api/evals/{id}/runs                   List eval runs
    GET    /api/evals/{id}/runs/{run_id}          Run result detail
    ```
  - Acceptance:
    - [ ] Register eval with TestSuite ground truth
    - [ ] Run agent through eval → score computed from ground truth comparison
    - [ ] Results include per-task scores and aggregate
    - [ ] No LLM self-grading — scoring is always external
  - Size: M (2 days)

- [ ] **G.4** Implement bounty market HTTP routes
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/bounties.rs` (new file)
  - Current: `routes/jobs.rs` has a full job lifecycle CRUD. `roko-chain/src/marketplace.rs` has on-chain escrow logic. `routes/chain.rs` reads on-chain bounty state.
  - This task adds a **bounty-specific layer** that wraps jobs with escrow, bidding, and evaluation:
  - API:
    ```
    POST   /api/bounties                          Post bounty (creates escrow if chain configured)
    GET    /api/bounties                          List bounties (filter by domain, state, min_value)
    GET    /api/bounties/{id}                     Bounty detail
    POST   /api/bounties/{id}/bids                Submit bid (agent passport + price + capability proof)
    GET    /api/bounties/{id}/bids                List bids (poster only)
    POST   /api/bounties/{id}/match               Trigger VCG matching (select winner from bids)
    POST   /api/bounties/{id}/submit              Agent submits result
    POST   /api/bounties/{id}/evaluate            Run eval, score result
    POST   /api/bounties/{id}/settle              Release escrow to winner
    POST   /api/bounties/{id}/dispute             Open dispute
    POST   /api/bounties/{id}/cancel              Cancel (poster only, pre-assignment)
    ```
  - State machine: Open → Claimed → InProgress → Submitted → Evaluated → Completed | Disputed → Settled
  - Works **off-chain by default** (no escrow). Chain integration optional via `bounty.chain_escrow: true` in config.
  - Acceptance:
    - [ ] Post bounty → listed in GET /api/bounties
    - [ ] Agent bids → bid recorded with capability proof
    - [ ] VCG match → selects welfare-maximizing winner
    - [ ] Submit + evaluate → score computed from linked eval
    - [ ] Settle → payment released (or recorded as completed if off-chain)
    - [ ] Cancel pre-assignment returns 200; cancel post-assignment returns 400
  - Size: L (3 days)

- [ ] **G.5** Wire arena/bounty results to reputation
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/arenas.rs` (extend)
  - Design: On attempt completion or bounty settlement, create a `WorkProof` record for `ValidationRegistry`:
    - Arena completion: `delta = (score - 0.5) * arena_weight` (score > 0.5 = positive, < 0.5 = negative)
    - Bounty completion: `delta = +bounty_reward_tier` (success) or `-0.5 * reward_tier` (failure)
  - This feeds the reputation system (Phase J) with verified work records.
  - Acceptance:
    - [ ] Arena attempt with score 0.8 → positive reputation delta
    - [ ] Arena attempt with score 0.3 → negative reputation delta
    - [ ] Bounty success → positive delta proportional to bounty value
    - [ ] WorkProof recorded with gate scores and attestation
  - Size: S (1 day)

- [ ] **G.6** Wire arena events to WS relay
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/arenas.rs` (extend)
  - Room: `arena:{id}` for attempt submissions, score updates, leaderboard changes
  - Events: `arena.attempt_submitted`, `arena.attempt_completed`, `arena.leaderboard_updated`, `arena.concluded`
  - Acceptance:
    - [ ] Dashboard subscribes to `arena:abc` → receives attempt events in real-time
    - [ ] Leaderboard update event includes new ranking
  - Size: S (half day)

- [ ] **G.7** Implement dispute escalation (off-chain only)
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/bounties.rs` (extend)
  - Design: Simple 2-level dispute resolution (on-chain 4-level deferred to Phase J):
    1. **Bond**: Disputer stakes a claim. Counter-party has 48h to respond.
    2. **Peer review**: 3 random agents with Silver+ reputation vote on outcome. Majority wins.
  - Acceptance:
    - [ ] Dispute opens → bounty state changes to Disputed
    - [ ] Counter-party responds → both sides recorded
    - [ ] Peer review: 3 agents vote → majority determines outcome
    - [ ] Timeout (48h no response) → disputer wins by default
  - Size: M (2 days)

---

## Phase H: DeFi infrastructure

**Architecture doc**: [12-defi.md](../architecture/12-defi.md)
**DeFi gap analysis**: All 41 batches from `/Users/will/dev/nunchi/roko/roko/tmp/defi/gap/`

**Estimated tasks**: 41 batches, ~185 work items
**Estimated effort**: 8-12 weeks (4 agents in parallel)
**Dependencies**: Phase A (agent runtime), Phase C (inference gateway for model routing)
**Key crates**: `roko-chain`, `roko-learn`, `roko-agent`, `roko-conductor`, `roko-daimon`, `roko-dreams`, `roko-neuro`, `roko-primitives`, `roko-std`

The DeFi batches are organized into 6 tiers by the gap analysis. See `/Users/will/dev/nunchi/roko/roko/tmp/defi/gap/11-CHECKLIST-IMPLEMENTATION.md` for the complete topological sort. Summary:

### Tier 0 -- No dependencies (start immediately)

| Batch | Title | Effort | Crate |
|-------|-------|--------|-------|
| 0.1 | Mirage-rs integration toolkit | M | roko-chain |
| 1.1 | `get_logs` on `AlloyChainClient` | S | roko-chain |
| 3.1 | Classical indicator expansion (6 indicators) | S | roko-learn |
| 5.1 | Archetype registry and manifest loader | L | roko-agent |

### Tier 1 -- Depends on Tier 0

| Batch | Title | Effort | Depends on |
|-------|-------|--------|------------|
| 1.2 | WebSocket subscription + event bus | L | 1.1 |
| 1.4 | Protocol state cache | M | 1.1 |
| 1.5 | Wallet registry | M | 1.1 |
| 3.2 | DeFi-native indicators | L | 1.1 |
| 3.3 | Microstructure indicators | L | 1.1 |
| 3.4 | On-chain signals and sentiment | M | 1.1 |
| 3.5 | Volatility and regime detection | M | 3.1 |
| 6.3 | Regime detection and adaptive threshold | L | 3.1 |
| 10.1 | Market state HDC encoding | M | 3.1 |

### Tier 2 -- Depends on Tier 1

| Batch | Title | Effort | Depends on |
|-------|-------|--------|------------|
| 1.3 | Triage pipeline enrichment | M | 1.2 |
| 1.6 | Heartbeat chain lag suppression | S | 1.2 |
| 2.1 | VenueAdapter trait + mock | M | 1.2 |
| 3.6 | HDC composite indicators | L | 3.1-3.5 |
| 4.2 | MEV protection pipeline | L | 1.2 |
| 4.4 | DeFi circuit breakers | M | 3.1 |
| 6.1 | Wire heartbeat to DeFi consumers | M | 1.2 |

### Tier 3 -- Depends on Tier 2

| Batch | Title | Effort | Depends on |
|-------|-------|--------|------------|
| 2.2 | DeFi tool handlers | L | 2.1 |
| 2.4 | Analysis tool definitions | M | 2.1 |
| 4.1 | DeFi risk limits + position tracking | L | 2.1 |
| 5.2 | First five DeFi archetypes | L | 5.1, 2.1 |
| 6.4 | DeFi conductor watchers | L | 6.1, 3.1 |
| 7.1 | TradingReflect -- FIFO P&L attribution | L | 2.1 |

### Tier 4 -- Depends on Tier 3

| Batch | Title | Effort | Depends on |
|-------|-------|--------|------------|
| 2.3 | Wire handlers into HandlerRegistry | S | 2.2 |
| 2.5 | Wallet tool handlers | S | 1.5, 2.2 |
| 4.3 | Custody controls + tx lifecycle | L | 4.1 |
| 5.3 | Delegation DAG + tool profiles | L | 5.1, 5.2 |
| 6.2 | 9-step decision pipeline | XL | 6.1, 5.1 |
| 7.2 | Indicator accuracy tracking | M | 3.1, 7.1 |
| 7.3 | Regime detection + strategy learning | L | 3.1, 7.1 |
| 7.4 | Trading playbooks | M | 7.1 |
| 7.5 | Risk-adjusted reward | M | 7.1 |
| 8.1 | PAD mapping from P&L | M | 7.1 |
| 9.1 | Chain triggers + counterfactual replay | L | 7.1, 1.2 |
| 10.2 | Knowledge routing + regime classification | M | 10.1, 7.1 |

### Tier 5 -- Final integration

| Batch | Title | Effort | Depends on |
|-------|-------|--------|------------|
| 8.2 | Affect-to-position-sizing + tilt | M | 8.1, 4.1 |
| 8.3 | Somatic-TA HDC binding + strategy space | L | 8.1, 3.6 |
| 9.2 | Strategy discovery + dream journal | L | 9.1, 3.1 |

**Critical path**: 1.1 -> 1.2 -> 2.1 -> 7.1 -> 9.1 -> 9.2 (20-30 days)

Each batch is fully specified in its gap analysis doc with code skeletons, file paths, and verification commands. Agents should read the specific gap doc (01-10) for implementation details.

---

## Phase I: Meta layer

**Architecture doc**: [13-meta.md](../architecture/13-meta.md)

**Estimated tasks**: 6 (reduced — role morphing and eval generation exist)
**Estimated effort**: 2-3 weeks
**Dependencies**: Phase A (agent runtime + extensions), Phase E (knowledge for lineage), Phase G (arenas for eval)
**Key crates**: `roko-agent`, `roko-core`, `roko-serve`

### What already exists (do not rebuild)

| Item | Location | Status |
|------|----------|--------|
| Role morphing (transition matrix, MorphableAgent) | `roko-agent/src/metamorphosis.rs` | **EXISTS** — role switching during execution |
| Eval generation from task specs | `roko-gate/src/eval_generator.rs` | **EXISTS** — ExampleBased, PropertyBased, MutationBased strategies |
| 23 safety modules | `roko-agent/src/safety/` | **EXISTS** — allowlist, authz, bash, capabilities, etc. (no recursive.rs) |

### Core constraints (from 13-meta.md)

1. **Meta-agents are agents**: Same AgentRuntime, specialized tools (agent_create, agent_configure, etc.)
2. **Depth bounded**: Default max 3. Children inherit `depth - 1`.
3. **Caveats monotonically narrow**: Children cannot exceed parent permissions.
4. **Lineage is permanent**: Recorded on-chain via `parentPassport` field.

- [ ] **I.1** Implement `AgentCreatorExt` extension
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/extensions/builtins/agent_creator.rs` (new file)
  - Design:
    ```rust
    pub struct AgentCreatorExt {
        pub max_depth: u32,              // default 3
        pub current_depth: u32,
        pub max_creations_per_hour: u32, // default 10
        pub creations_this_hour: u32,
        pub min_child_quality: f64,      // min eval score to register child (default 0.5)
    }
    ```
  - Tools provided to agent: `agent_create`, `agent_configure`, `agent_start`, `agent_stop`, `agent_fork`, `agent_list_children`
  - Extension hooks:
    - `pre_action`: Intercept agent creation tool calls, enforce depth limit and rate limit
    - `post_action`: Record lineage edge, update creation counter
    - `on_cost_update`: Track child agent costs against parent budget
  - Caveat inheritance: `compute_child_caveats(parent_caveats, additional_restrictions)` — union of parent caveats + any new restrictions. Never removes a parent caveat.
  - Acceptance:
    - [ ] Agent with extension can call `agent_create` tool to spawn child
    - [ ] Depth 3 agent creates child at depth 2; depth 1 cannot create children
    - [ ] Rate limit: 11th creation in 1 hour → blocked with error
    - [ ] Child inherits parent caveats + any additional restrictions
    - [ ] Lineage edge recorded on creation
  - Size: L (3 days)

- [ ] **I.2** Implement generator framework with schema validation
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/generator.rs` (new file)
  - Existing: `EvalGenerator` generates eval templates. This task generalizes to arbitrary output types.
  - Design:
    ```rust
    pub enum GeneratorOutputType {
        Agent, Arena, Gate, Eval, Extension, DomainProfile,
    }

    pub struct GeneratorConfig {
        pub agent: AgentConfig,
        pub output_type: GeneratorOutputType,
        pub output_schema: Option<serde_json::Value>,  // JSON Schema for validation
        pub auto_register: bool,                        // register output automatically
        pub min_quality: f64,                           // eval threshold before registration
    }

    pub struct GeneratorOutput {
        pub output_type: GeneratorOutputType,
        pub payload: serde_json::Value,
        pub metadata: GenerationMetadata,
    }

    pub fn validate_generator_output(output: &GeneratorOutput) -> Result<(), ValidationError> {
        // 1. Check payload against output_schema (JSON Schema validation)
        // 2. Type-specific checks (e.g., Agent config has required fields)
        // 3. Return Ok or Err with specific field-level errors
    }
    ```
  - Acceptance:
    - [ ] Generator agent produces agent config → validates against schema
    - [ ] Invalid output → rejected with field-level errors
    - [ ] `auto_register = true` + quality >= min_quality → output registered automatically
    - [ ] `auto_register = true` + quality < min_quality → output rejected with score
    - [ ] Generation metadata recorded (model, cost, duration)
  - Size: M (2 days)

- [ ] **I.3** Implement lineage tracking service
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/lineage.rs` (new file)
  - Design:
    ```rust
    pub struct LineageEdge {
        pub parent_id: ObjectId,
        pub child_id: ObjectId,
        pub relationship: LineageRelationship,  // Generated | Forked | Evolved
        pub recorded_at: DateTime<Utc>,
    }

    pub struct ObjectId {
        pub object_type: ObjectType,  // Agent | MetaAgent | Generator | Arena | Eval | Gate
        pub id: String,
    }

    pub struct LineageService {
        store_path: PathBuf,  // .roko/lineage/edges.jsonl
    }

    impl LineageService {
        pub fn record(&self, edge: LineageEdge) -> Result<()>;
        pub fn ancestors(&self, id: &ObjectId) -> Result<Vec<LineageEdge>>;
        pub fn children(&self, id: &ObjectId) -> Result<Vec<LineageEdge>>;
        pub fn graph(&self, root: &ObjectId, max_depth: u32) -> Result<LineageGraph>;
    }
    ```
  - Storage: `.roko/lineage/edges.jsonl` (append-only, one edge per line)
  - Graph query: BFS/DFS up to `max_depth` (default 5). Returns nodes + edges for dashboard visualization.
  - Acceptance:
    - [ ] Record edge on agent creation → persists to JSONL
    - [ ] `ancestors()` returns parent chain
    - [ ] `children()` returns direct children
    - [ ] `graph()` returns full tree up to depth
    - [ ] Works for all object types (Agent, Arena, Eval, etc.)
  - Size: M (1-2 days)

- [ ] **I.4** Implement recursive safety monitoring
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/safety/recursive.rs` (new file)
  - Design:
    ```rust
    pub struct RecursiveSafetyMonitor {
        pub global_max_rate_per_hour: u32,   // default 50
        pub min_quality_slope: f64,          // minimum quality trend slope (default -0.05)
        pub quality_window: usize,           // number of generations to check (default 5)
    }

    pub enum SafetyAnomaly {
        RateLimitViolation { meta_agent_id: String, rate: u32, limit: u32 },
        QualityDegradation { meta_agent_id: String, quality_trend: Vec<f64>, slope: f64 },
        CircularDependency { agents: Vec<String> },
        GlobalRateExceeded { current_rate: u32, limit: u32 },
    }

    pub enum SafetyAction {
        Log,                              // informational
        Pause { agent_id: String },       // stop creating but keep running
        Terminate { agent_id: String },   // kill agent
    }
    ```
  - Monitoring: Runs on every agent creation event. Checks:
    1. Per-meta-agent rate (from AgentCreatorExt limits)
    2. Global rate across all meta-agents
    3. Quality trend: if last 5 generations show declining quality (negative slope), pause
    4. Circular dependency: if agent A creates B creates A, terminate
  - Acceptance:
    - [ ] 51st creation globally in 1 hour → `GlobalRateExceeded` anomaly
    - [ ] 5 generations with declining quality → `QualityDegradation`, agent paused
    - [ ] Circular dependency detected → `CircularDependency`, agent terminated
    - [ ] All anomalies logged to `.roko/learn/safety-anomalies.jsonl`
  - Size: M (2 days)

- [ ] **I.5** Add meta-agent and lineage routes to roko-serve
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/meta.rs` (new file)
  - API:
    ```
    GET    /api/meta/agents                       List meta-agents with children summary
    GET    /api/meta/agents/{id}                  Meta-agent detail (config, children count, quality)
    GET    /api/meta/agents/{id}/children         List agents created by this meta-agent

    GET    /api/meta/generators                   List generators (filter by output_type)
    POST   /api/meta/generate                     Trigger generation (async, returns 202)

    GET    /api/meta/lineage/{type}/{id}/ancestors    Ancestor chain
    GET    /api/meta/lineage/{type}/{id}/descendants  Descendant tree
    GET    /api/meta/lineage/graph?root={type}:{id}&depth=3  Full graph for visualization

    GET    /api/meta/safety/anomalies             Recent safety anomalies
    ```
  - Acceptance:
    - [ ] List meta-agents shows children count and average quality score
    - [ ] Lineage graph returns nodes + edges serializable for dashboard visualization
    - [ ] Safety anomalies paginated with severity filter
  - Size: M (2 days)

- [ ] **I.6** Implement config optimizer extension (optional, stretch goal)
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/extensions/builtins/config_optimizer.rs` (new file)
  - Design: Extension that tunes agent configuration parameters (temperature, model, tool set) based on eval results. Strategies: GridSearch, RandomSearch, Bayesian, Bandit.
  - This is a stretch goal — meta-agents already enable manual optimization. This automates it.
  - Size: L (3 days, stretch)

---

## Phase J: On-chain registries

**Architecture doc**: [14-registries.md](../architecture/14-registries.md)

**Estimated tasks**: 4 (reduced — agent registry, reputation registry, validation registry all exist)
**Estimated effort**: 1-2 weeks
**Dependencies**: Phase E (knowledge registry), Phase G (arena/eval registries)
**Key crates**: `roko-chain`, `roko-serve`

### What already exists (do not rebuild)

| Item | Location | Status |
|------|----------|--------|
| Agent registry (ERC-721 soulbound passports) | `roko-chain/src/agent_registry.rs` (278 LOC) | **EXISTS** — mint, get_passport, update_tier, capabilities bitmask, tier staking |
| Reputation registry (7-domain EMA scoring) | `roko-chain/src/reputation_registry.rs` (668 LOC) | **EXISTS** — submit_feedback, get_score, slash, ban, recovery tracker |
| Validation registry (work proofs + attestation) | `roko-chain/src/validation_registry.rs` (274 LOC) | **EXISTS** — WorkProof storage, GateScore, attestation |
| KORAI token (lazy demurrage) | `roko-chain/src/korai_token.rs` | **EXISTS** |
| Collusion detection (assignment graph clique) | `roko-chain/src/collusion.rs` | **EXISTS** |
| Futures market (agent performance) | `roko-chain/src/futures_market.rs` | **EXISTS** |
| Chain proxy routes | `roko-serve/src/routes/chain.rs` (165 LOC) | **EXISTS** — GET /api/chain/agents, /bounties, /status |
| TraceRank (PageRank-style reputation propagation) | `roko-chain/src/trace_rank.rs` | **EXISTS** |

### Reputation tier thresholds (from 14-registries.md)

| Tier | Score range | Unlocks |
|------|------------|---------|
| Gray | < 10 | Basic participation |
| Copper | 10-49 | Create arenas, publish knowledge |
| Silver | 50-199 | Create evals, participate in clearing |
| Gold | 200-999 | Meta-agent creation, validate knowledge |
| Amber | >= 1000 | All capabilities, featured status, priority |

- [ ] **J.1** Implement knowledge registry on-chain client
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/knowledge_registry.rs` (new file)
  - Design:
    ```rust
    pub struct KnowledgeRegistryClient {
        contract: Address,
        provider: Arc<dyn Provider>,
        signer: Option<Arc<dyn Signer>>,
    }

    impl KnowledgeRegistryClient {
        pub async fn publish(&self, entry: KnowledgePublication) -> Result<[u8; 32]>;
        pub async fn validate(&self, entry_id: [u8; 32], evidence_hash: [u8; 32]) -> Result<()>;
        pub async fn challenge(&self, entry_id: [u8; 32], reason: &str) -> Result<[u8; 32]>;
        pub async fn get_entry(&self, entry_id: [u8; 32]) -> Result<OnChainKnowledgeEntry>;
        pub async fn query(&self, tag: &str, limit: u64) -> Result<Vec<[u8; 32]>>;
    }
    ```
  - Knowledge lifecycle: Active → Challenged → Validated | Retracted → Stale (90 days no validation)
  - Off-chain content, on-chain commitment (content_hash + hdc_fingerprint on chain)
  - Acceptance:
    - [ ] Publish knowledge entry → on-chain event emitted
    - [ ] Validate → validation_count incremented, publisher earns +0.2 reputation
    - [ ] Challenge → state changes to Challenged, resolution window opens
    - [ ] Query by tag returns matching entry IDs
  - Size: M (2 days)

- [ ] **J.2** Implement event indexer
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/indexer.rs` (new file)
  - Design:
    ```rust
    pub struct EventIndexer {
        provider: Arc<dyn Provider>,
        contracts: Vec<(Address, String)>,  // (address, name)
        store_path: PathBuf,                // .roko/chain/events.jsonl
        last_block: AtomicU64,
    }

    pub struct IndexedEvent {
        pub sequence: u64,
        pub contract: String,
        pub event_type: String,
        pub block_number: u64,
        pub tx_hash: String,
        pub timestamp: u64,
        pub data: serde_json::Value,
    }
    ```
  - Contracts to index: AgentPassport, ReputationRegistry, KnowledgeRegistry, ArenaRegistry, BountyMarket
  - Polling: Every 2 seconds for new blocks (mirage-rs block time). Stores events to JSONL. Read-only — never writes to chain.
  - Rebuild: Can be rebuilt from genesis by deleting events.jsonl and reindexing.
  - Acceptance:
    - [ ] Indexer polls new blocks and stores events
    - [ ] Passport mint → indexed with all fields
    - [ ] Reputation update → indexed with delta and domain
    - [ ] Knowledge publication → indexed with content_hash and tags
    - [ ] Rebuild from scratch produces same event set
  - Size: M (2 days)

- [ ] **J.3** Add registry query routes to roko-serve
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/registries.rs` (new file)
  - API:
    ```
    GET    /api/registries/passports                     List passports (paginated, filter by tier/capability)
    GET    /api/registries/passports/{id}                Passport detail
    GET    /api/registries/passports/{id}/history        Full event history for passport

    GET    /api/registries/reputation/{passport_id}      Reputation across all 7 domains
    GET    /api/registries/reputation/top?domain=coding&limit=20  Top agents by domain

    GET    /api/registries/knowledge                     Query indexed knowledge entries
    GET    /api/registries/knowledge/{id}/history        Event history for entry

    GET    /api/registries/events                        Query all indexed events (filter by contract, type, block range)
    GET    /api/registries/events/stream                 SSE stream of new events

    GET    /api/registries/stats                         Indexer health (latest block, lag, event count)
    ```
  - Reads from: event indexer (J.2) + direct chain queries via existing clients
  - Acceptance:
    - [ ] Passport list paginated with tier filter
    - [ ] Reputation shows 7-domain breakdown with EMA scores
    - [ ] Top agents by domain returns ranked list
    - [ ] Events stream via SSE for real-time dashboard
    - [ ] Stats shows indexer lag (current block - last indexed block)
  - Size: M (2 days)

- [ ] **J.4** Wire passport creation into agent lifecycle
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/agents.rs` (extend existing)
  - Current: `POST /api/agents` creates agent. No on-chain passport minted.
  - Change: If chain is configured (`[chain].agent_registry` in roko.toml), mint passport on agent creation. Store `passport_id` in agent config. Optional — works without chain.
  - Acceptance:
    - [ ] Create agent with chain configured → passport minted, `passport_id` stored
    - [ ] Create agent without chain → no passport, agent works normally
    - [ ] Passport `parentPassport` field set when created by a meta-agent
  - Size: S (1 day)

---

## Phase K: Visual composition

**Architecture doc**: [19-visual-composition.md](../architecture/19-visual-composition.md)

**Estimated tasks**: 5 (reduced — template CRUD and job marketplace exist)
**Estimated effort**: 1-2 weeks
**Dependencies**: Layer 4 (plan execution UI), Phase C (gateway for chat model calls)
**Key crates**: `roko-serve`, `roko-core`

### What already exists (do not rebuild)

| Item | Location | Status |
|------|----------|--------|
| Template CRUD (create/list/get/delete/deploy) | `roko-serve/src/routes/templates.rs` (201 LOC) | **EXISTS** — AgentTemplate, rendering, Railway deploy |
| Template registry impl | `roko-serve/src/templates.rs` | **EXISTS** |
| Job marketplace (full lifecycle CRUD) | `roko-serve/src/routes/jobs.rs` | **EXISTS** — 7-state machine, stats, matching |
| Plan mutation protocol | Layer 4 (04-plan-execution.md tasks 4.6, 4.7) | **EXISTS** — plan chat, mutation endpoints |

### Design philosophy (from 19-visual-composition.md)

1. **Composition over configuration**: No freeform textareas; every field maps to typed primitive
2. **Progressive disclosure**: Simple views first (template picker), drill into control
3. **Draft/deploy separation**: Authoring is free; deploying costs tokens/gas
4. **Conversation-as-editor**: User message → LLM generates structured mutations → canvas animates

- [ ] **K.1** Add template forking and versioning
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/templates.rs` (extend existing)
  - Current: Templates have CRUD but no forking or version history.
  - Add:
    ```
    POST   /api/templates/{name}/fork              Fork template (creates new template with forked_from reference)
    GET    /api/templates/{name}/versions           List version history
    POST   /api/templates/{name}/versions           Create new version (snapshot current state)
    ```
  - Forking: Creates a copy with `forked_from: original_name` metadata. Fork can diverge independently.
  - Versioning: Each `POST /versions` creates an immutable snapshot. `GET /versions` returns list ordered by creation time.
  - Acceptance:
    - [ ] Fork template → new template with `forked_from` field
    - [ ] List templates with `?forked_from=original` filter
    - [ ] Create version → snapshot stored immutably
    - [ ] List versions → ordered by time, includes diff summary
  - Size: M (1-2 days)

- [ ] **K.2** Implement extension compilation service
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/extensions.rs` (new file)
  - API:
    ```
    POST   /api/extensions/compile
      Request: { "source": "// Rust source code...", "name": "my-extension" }
      Response (success): { "status": "ok", "artifact_path": ".roko/extensions/my-extension/lib.dylib" }
      Response (failure): { "status": "error", "errors": [{ "line": 12, "column": 5, "message": "..." }] }
    ```
  - Design: Create temp directory, write source, run `cargo build --release --target-dir ...`, parse errors with line numbers, copy artifact to `.roko/extensions/{name}/`.
  - Security: Compilation runs in a sandboxed temp directory. Source is validated for suspicious patterns (no `std::process::Command`, no `std::fs::remove_dir_all`, etc.) before compilation.
  - Acceptance:
    - [ ] Valid Rust extension source → compiled artifact at expected path
    - [ ] Syntax error → error response with line/column
    - [ ] Dangerous patterns in source → rejected pre-compilation
    - [ ] Compilation timeout (60s) → error response
  - Size: L (3 days)

- [ ] **K.3** Implement gate test runner
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/gates.rs` (new file)
  - API:
    ```
    POST   /api/gates/{id}/test
      Request: { "input": { ... }, "expected_outcome": "pass" | "fail" }
      Response: { "passed": true, "score": 0.95, "duration_ms": 120, "detail": "..." }

    POST   /api/gates/{id}/test/batch
      Request: { "fixtures": [{ "input": {...}, "expected": "pass" }, ...] }
      Response: { "results": [...], "pass_rate": 0.9, "avg_duration_ms": 100 }
    ```
  - Design: Instantiate the gate from its config, run against provided input, compare result to expected outcome.
  - Supports all gate types: shell command, Rust function (compiled), chain simulation, risk check.
  - Acceptance:
    - [ ] Test gate with known-pass input → `passed: true`
    - [ ] Test gate with known-fail input → `passed: false` with failure detail
    - [ ] Batch test → aggregate pass_rate and individual results
    - [ ] Gate timeout (configurable, default 30s) → error
  - Size: M (2 days)

- [ ] **K.4** Implement three-tier validation for agents and gates
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/agents.rs` + `routes/gates.rs` (extend)
  - API: `POST /api/agents/{id}/validate`, `POST /api/gates/{id}/validate`
  - Response:
    ```json
    {
      "errors": [{ "field": "model", "message": "Model not found: claude-opus-99" }],
      "warnings": [{ "field": "budget", "message": "No budget set; agent will run until manually stopped" }],
      "suggestions": [{ "field": "extensions", "message": "Consider adding 'compiler' extension for coding tasks" }]
    }
    ```
  - Validation rules:
    - **Errors** (blocking): missing required fields, invalid model name, circular dependencies
    - **Warnings** (advisory): no budget, no gate pipeline, unused tool permissions
    - **Suggestions** (helpful): profile-based recommendations, common patterns
  - Acceptance:
    - [ ] Agent with invalid model → error in `errors` array
    - [ ] Agent with no budget → warning
    - [ ] Coding agent without compiler extension → suggestion
    - [ ] Validation returns all three tiers in single response
  - Size: M (2 days)

- [ ] **K.5** Implement unified deploy endpoint
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/` (extend agents.rs, templates.rs)
  - Current: Templates have deploy (Railway + in-process). Agents have start/stop. Missing: unified deploy that validates → estimates cost → deploys.
  - API:
    ```
    POST   /api/agents/{id}/deploy
      Request: { "target": "local" | "railway" | "fly", "dry_run": false }
      Response (dry_run=true): { "estimated_cost": { "daily_usd": 2.50, "inference_usd": 15.0 }, "warnings": [...] }
      Response (dry_run=false): { "deployment_id": "...", "status": "deploying", "url": "..." }
    ```
  - `dry_run=true`: Validate config + estimate cost without deploying. Useful for dashboard preview.
  - Acceptance:
    - [ ] Dry run returns cost estimate without side effects
    - [ ] Local deploy spawns agent process
    - [ ] Railway deploy creates service (uses existing Railway deploy code)
    - [ ] Invalid config → deploy rejected with validation errors
  - Size: M (1-2 days)

---

## Phase L: Dashboard architecture

**Architecture doc**: [15-dashboard.md](../architecture/15-dashboard.md)

**Estimated tasks**: 6
**Estimated effort**: 2-3 weeks
**Dependencies**: Phase B (relay for subscriptions), Phase C (gateway for cost stats)
**Key crates**: Frontend only (nunchi-dashboard: React 19 + Vite 8 + TanStack Query 5 + Zustand 5)

### What already exists (do not rebuild)

| Item | Location | Status |
|------|----------|--------|
| Backend: ~85 HTTP routes | `roko-serve/src/routes/` (32 modules) | **EXISTS** — all data endpoints |
| Backend: WebSocket relay | `roko-serve/src/routes/ws.rs` | **EXISTS** — basic WS (enhanced in Phase B) |
| Backend: SSE streaming | `roko-serve/src/routes/sse.rs` | **EXISTS** |
| Backend: Event bus | `roko-runtime/src/event_bus.rs` | **EXISTS** |
| TUI: ratatui dashboard | `roko-cli/src/tui/` | **EXISTS** — F1-F7 tabs, fully wired |

### Tech stack (from 15-dashboard.md)

| Layer | Library |
|-------|---------|
| Framework | React 19 |
| Build | Vite 8 |
| Data fetching | TanStack Query 5 |
| State | Zustand 5 |
| Blockchain | ethers.js 6 |
| Charts | Recharts |
| Auth | Privy 3 |

### Performance targets

| Metric | Target |
|--------|--------|
| FCP | < 1.2s |
| LCP | < 2.0s |
| CLS | < 0.05 |
| WS event-to-render p95 | < 100ms |
| Canvas sustained | >= 60fps |
| Initial JS bundle | < 250KB gzipped |

### Three-tier deployment model

| Tier | Components | Required |
|------|-----------|----------|
| Tier 1 — Backbone | Mirage chain, relay, indexer | Yes |
| Tier 2 — Workspace | roko-serve (HTTP + WS) | No |
| Tier 3 — Remote agents | Per-agent sidecars on Fly/Railway | No |

Dashboard must work with **Tier 1 only** (relay + chain). Tier 2 (roko-serve) adds workspace features. Tier 3 adds remote agent management.

- [ ] **L.1** Implement `SubscriptionManager` (WS multiplexing)
  - Target: `src/lib/subscriptions.ts` (in nunchi-dashboard)
  - Design:
    ```typescript
    class SubscriptionManager {
      private ws: WebSocket;
      private subscriptions: Map<string, Set<(event: WsEnvelope) => void>>;

      subscribe(rooms: string[], handler: (event: WsEnvelope) => void): () => void;
      // Returns unsubscribe function

      // Single WS connection shared across all pages
      // On page mount: subscribe to page-specific rooms
      // On page unmount: unsubscribe (returned cleanup function)
      // Reconnect with last_seq on disconnect (Phase B.1.3)
    }

    // React hook:
    function useSubscription(rooms: string[], handler: (event: WsEnvelope) => void) {
      const manager = useContext(SubscriptionContext);
      useEffect(() => manager.subscribe(rooms, handler), [rooms]);
    }
    ```
  - Acceptance:
    - [ ] Single WS connection shared across all pages
    - [ ] Page mounts → subscribes to rooms; unmounts → unsubscribes
    - [ ] Reconnect on disconnect with sequence-based replay
    - [ ] No duplicate events after reconnect
  - Size: M (2 days)

- [ ] **L.2** Implement `EventAggregator` (100ms batching)
  - Target: `src/lib/aggregator.ts` (in nunchi-dashboard)
  - Design: Buffer incoming WS events. Flush every 100ms or when buffer reaches 50 events (whichever first). Deliver as single batch to consumers. Ring buffer of 200 events for replay.
  - Heartbeat coalescing: Keep only latest heartbeat per agent per flush window.
  - Acceptance:
    - [ ] 100 events in 50ms → delivered as 2 batches (50 + 50)
    - [ ] 3 heartbeats for same agent in window → consumer sees only latest
    - [ ] Ring buffer enables replay of last 200 events on new subscriber
  - Size: M (1-2 days)

- [ ] **L.3** Implement `RenderScheduler` (DOM + canvas coordination)
  - Target: `src/lib/render-scheduler.ts` (in nunchi-dashboard)
  - Design: Two render loops:
    - **DOM loop**: Coalesces state updates into `requestAnimationFrame` callbacks. Multiple state changes between frames produce one DOM update.
    - **Canvas loop**: Independent 60fps loop for WebGL/canvas visualizations (agent pulse, network graph). Uses `requestAnimationFrame` with frame budget tracking.
  - Priority: Canvas renders first (visual smoothness), DOM updates second (data freshness).
  - Acceptance:
    - [ ] Canvas maintains 60fps during heavy WS event bursts
    - [ ] DOM updates coalesced (10 state changes → 1 render)
    - [ ] Frame budget tracked: if frame > 16ms, log warning
  - Size: M (2 days)

- [ ] **L.4** Implement adaptive information density
  - Target: Dashboard layout components (in nunchi-dashboard)
  - Three regimes (from 15-dashboard.md):
    | Regime | Trigger | Display |
    |--------|---------|---------|
    | Cruise (calm) | All agents calm, PE < 0.15 avg | Minimal: green dots, aggregated metrics, collapsed cards |
    | Volatile | 1+ agents in T2, active gate failures | Affected agents expand, healthy stay collapsed, anomaly highlights |
    | Crisis | Multiple gate failures, PE > 0.40 | Full traces visible, remediation inline, all agents expand |
  - Regime determined from aggregate agent heartbeat data. Transitions animated (200ms).
  - Acceptance:
    - [ ] All agents calm → cards collapsed, minimal display
    - [ ] One agent in T2 → that agent's card expands, others stay collapsed
    - [ ] Multiple failures → all cards expand, full trace visible
    - [ ] Regime transitions animate smoothly (no flash)
  - Size: M (2 days)

- [ ] **L.5** Wire page-to-data-source mapping
  - Target: Per-page subscription hooks (in nunchi-dashboard)
  - Page-to-room mapping (from 15-dashboard.md):
    | Page | WS rooms | REST fallback |
    |------|----------|---------------|
    | Pulse / Command | `system`, `agent:*:heartbeat` | `GET /api/agents` |
    | Pulse / Console | `agent:*` | — |
    | Fleet / Agent list | `system` | `GET /api/agents` |
    | Fleet / Agent detail | `agent:{id}`, `agent:{id}:heartbeat`, `agent:{id}:trace` | — |
    | Forge / Plans | `plan:*` | `GET /api/plans` |
    | Forge / Execution | `plan:*`, `agent:*` | `GET /api/plans/{id}` |
    | Knowledge / Store | `knowledge:*` | `GET /api/knowledge` |
    | Arena / Leaderboard | `arena:{id}` | `GET /api/arenas/{id}` |
    | Treasury / Costs | `gateway:stats` | `GET /api/gateway/stats` |
  - Each page uses `useSubscription(rooms, handler)` from L.1.
  - REST fallback: Initial data load via TanStack Query, then WS for real-time updates.
  - Acceptance:
    - [ ] Each page subscribes to correct rooms on mount
    - [ ] Page unmount → rooms unsubscribed
    - [ ] REST data loads on first render, WS updates incrementally
    - [ ] Page works without WS (REST fallback only, polling every 5s)
  - Size: M (2 days)

- [ ] **L.6** Implement epistemic aesthetics system
  - Target: Dashboard visualization components (in nunchi-dashboard)
  - Visual encodings (from 15-dashboard.md):
    | Visual property | Data source | Encoding |
    |----------------|-------------|----------|
    | Glow intensity | Gate pass rate | Brighter = higher confidence |
    | Fade / decay | Knowledge staleness | Faded = needs re-validation |
    | Velocity streaks | Agent output tokens/sec | Faster = higher throughput |
    | Heartbeat pulse | Agent tick cadence | Visible rhythm matches agent clock |
    | Saturation | Gate rung depth | Deeper validation = richer color |
  - Implementation: CSS custom properties driven by Zustand store. Canvas layer for pulse/streak animations.
  - Acceptance:
    - [ ] Agent with 95% gate pass rate → bright glow
    - [ ] Agent with 50% pass rate → dim glow
    - [ ] Stale knowledge entry → visually faded
    - [ ] Active agent → visible heartbeat pulse matching tick frequency
  - Size: L (3 days)

---

## Phase M: Universal primitive APIs (Connector, Feed, Recipe)

**Architecture docs**: [03-extensions.md](../architecture/03-extensions.md) (Connector section), [05-feeds.md](../architecture/05-feeds.md) (Feed + Recipe sections), [19-visual-composition.md](../architecture/19-visual-composition.md) (authoring surfaces)
**Dashboard PRD**: `23-universal-primitives.md`

**Estimated tasks**: 8 (reduced from 12 — connector + feed types and routes already exist)
**Estimated effort**: 2-3 weeks (2 agents in parallel)
**Dependencies**: Phase A (agent runtime for connector loading), Phase B (relay for feed subscriptions)
**Key crates**: `roko-core`, `roko-serve`, `roko-agent`, `roko-learn`

### What already exists (do not rebuild)

| Item | Location | Status |
|------|----------|--------|
| `ConnectorKind` enum (6 types) | `roko-core/src/connector.rs` | **EXISTS** — ChainRpc, Exchange, McpServer, Database, Webhook, Api |
| `ConnectorRegistry` + `ConnectorHealth` | `roko-core/src/connector.rs` | **EXISTS** |
| Connector CRUD routes | `roko-serve/src/routes/connectors.rs` | **EXISTS** |
| `FeedKind` enum (Raw, Derived, Composite, Meta) | `roko-core/src/feed.rs` | **EXISTS** |
| `FeedAccess` + `FeedRegistry` | `roko-core/src/feed.rs` | **EXISTS** |
| Feed CRUD routes | `roko-serve/src/routes/feeds.rs` | **EXISTS** |

### Batch M.1: Connector trait and wiring

- [ ] **M.1.1** Add async `Connector` trait with 5 methods to existing `connector.rs`
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/connector.rs` (extend existing)
  - Current: File has `ConnectorKind`, `ConnectorRegistry`, `ConnectorHealth` but no async trait with `connect/query/execute/health/disconnect`
  - Add the `#[async_trait] pub trait Connector` from 03-extensions.md (lines 200-224)
  - Acceptance: Trait compiles, default implementations return `unimplemented!()`, existing types preserved

- [ ] **M.1.2** Implement `McpConnector` adapter wrapping existing MCP configs
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/connectors/mcp.rs` (new file)
  - Acceptance: MCP servers in `roko.toml` auto-register as Connectors, queryable via existing routes

- [ ] **M.1.3** Wire connector loading into agent startup in orchestrate.rs
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs`
  - Acceptance: Agent config referencing connectors loads them at startup, health check on connect

### Batch M.2: Feed trait and connector-backed feeds

- [ ] **M.2.1** Add async `Feed` trait to existing `feed.rs`
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/feed.rs` (extend existing)
  - Current: File has `FeedKind`, `FeedAccess`, `FeedRegistry` but no async trait
  - Add `#[async_trait] pub trait Feed` with `subscribe/unsubscribe/poll/status/configure`
  - Acceptance: Trait compiles alongside existing types

- [ ] **M.2.2** Implement `ConnectorFeed` adapter sourcing from a Connector
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-conductor/src/feeds/connector_feed.rs` (new file)
  - Acceptance: Feed backed by a Connector polls at configurable interval, publishes to event bus

### Batch M.3: Recipe universal API

- [ ] **M.3.1** Define `Recipe` trait in `roko-core` composing `Scorer` instances
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/recipe.rs` (new file)
  - Acceptance: Trait supports `create / execute / chain / status / configure`, compiles with Scorer inputs

- [ ] **M.3.2** Wrap `TradingReflect` and `IndicatorTracker` as built-in Recipe templates
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/recipes/` (new directory)
  - Acceptance: Existing scoring pipelines accessible as named Recipe instances

- [ ] **M.3.3** Add recipe CRUD and backtest routes to roko-serve
  - Target: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/recipes.rs` -- **NEW ROUTE FILE**
  - API: `POST /api/recipes`, `GET /api/recipes`, `GET /api/recipes/{id}`, `POST /api/recipes/{id}/backtest`
  - Acceptance: Create recipe, run backtest against historical data, get output distribution

---

## Phase dependency graph

```
Phase A (Agent Runtime) ──────────────────────────────────────┐
                                                               │
Phase B (Relay) ────── depends on A ──────────────────────────│
                                                               │
Phase C (Gateway) ────── standalone (parallel with A) ────────│
                                                               │
Phase D (Auth) ────── depends on C ───────────────────────────│
                                                               │
Phase E (Knowledge) ────── depends on A ──────────────────────│
                                                               │
Phase F (Groups) ────── depends on A, B ──────────────────────│
                                                               │
Phase G (Arenas) ────── depends on A, E ──────────────────────│
                                                               │
Phase H (DeFi) ────── depends on A, C ────────────────────────│
                                                               │
Phase I (Meta) ────── depends on A, E, G ─────────────────────│
                                                               │
Phase J (Registries) ────── depends on E, G ──────────────────│
                                                               │
Phase K (Visual Composition) ────── depends on L4, C ─────────│
                                                               │
Phase L (Dashboard Arch) ────── depends on B, C ───────────────│
                                                               │
Phase M (Universal APIs) ────── depends on A, B ───────────────┘
```

### Parallel execution plan (4 agents)

| Agent | Phase sequence | Estimated weeks |
|-------|---------------|-----------------|
| Agent 1 (Core) | A.1 -> A.2 -> A.3 -> F -> I | 6-8 |
| Agent 2 (Infra) | C.1 -> C.2 -> C.3 -> C.4 -> D -> J | 5-7 |
| Agent 3 (Data) | B -> E -> G -> K | 6-8 |
| Agent 4 (DeFi) | H (Tier 0-5) | 8-12 |

Dashboard work (Phase L) runs on frontend agent(s) alongside backend phases.

---

### Parallel execution plan (updated with Phase M)

Phase M can start once Phase A completes (for connector loading) and Phase B is in progress (for feed subscriptions). Recipe work (M.3) can run in parallel with the connector and feed batches.

---

## New roko-serve route files summary

> Updated 2026-04-24. Several route files from the original list already exist.

**Already exist** (extend, don't recreate):

| File | Phase | Status |
|------|-------|--------|
| `routes/feeds.rs` | B | **EXISTS** — needs pagination + WS room wiring |
| `routes/gateway.rs` | C | **EXISTS** — needs pipeline stages (cache, loop guard, etc.) |
| `routes/connectors.rs` | M | **EXISTS** — needs Connector trait wiring |
| `routes/team.rs` | F | **EXISTS** — basis for group routes |

**Must be created**:

| File | Phase | Routes |
|------|-------|--------|
| `routes/arenas.rs` | G | `POST /api/arenas`, `GET /api/arenas`, `GET /api/arenas/{id}`, `POST /api/arenas/{id}/start`, `POST /api/arenas/{id}/conclude`, `GET /api/arenas/{id}/leaderboard` |
| `routes/evals.rs` | G | `POST /api/evals`, `GET /api/evals`, `POST /api/evals/{id}/run` |
| `routes/bounties.rs` | G | `POST /api/bounties`, `GET /api/bounties`, `POST /api/bounties/{id}/bid`, `POST /api/bounties/{id}/accept`, `POST /api/bounties/{id}/complete` |
| `routes/meta.rs` | I | `GET /api/meta/lineage/{agent_id}`, `GET /api/meta/generators`, `POST /api/meta/generate` |
| `routes/registries.rs` | J | `GET /api/registries/passports`, `GET /api/registries/reputation/{agent_id}`, `GET /api/registries/events` |
| `routes/extensions.rs` | K | `POST /api/extensions/compile` |
| `routes/gates.rs` | K | `POST /api/gates/{id}/test` |
| `routes/recipes.rs` | M | `POST /api/recipes`, `GET /api/recipes`, `GET /api/recipes/{id}`, `POST /api/recipes/{id}/backtest` |

**New non-route modules** (Phase A + C):

| Module | Phase | Purpose |
|--------|-------|---------|
| `roko-conductor/src/tick_pipeline.rs` | A | 9-step heartbeat pipeline |
| `roko-conductor/src/reflex_store.rs` | A | T0 reflex condition-action pairs |
| `roko-conductor/src/adaptive_clock.rs` | A | Regime-based tick frequency |
| `roko-conductor/src/cortical.rs` | A | Working memory, goals, beliefs persistence |
| `roko-runtime/src/reactive.rs` | A | Webhook/cron trigger scheduler |
| `roko-agent/src/extensions/mod.rs` | A | Extension loader + dependency resolution |
| `roko-agent/src/extensions/builtins/` | A | 3 built-in extensions (coding, research, chain) |
| `roko-serve/src/gateway/` | C | Pipeline stages module (cache, loop guard, etc.) |

---

## Effort summary (updated 2026-04-24)

> All 14 sections now fully fleshed out with reconciliation tables, Rust struct definitions, API contracts, acceptance criteria, and edge case resolutions.

| Phase | Tasks | Effort | Agents (parallel) | Change vs original |
|-------|-------|--------|-------------------|--------------------|
| **OG -- Orchestrator gaps** | **10** | **2-3 weeks** | **1-2** | **NEW — P0, parallel with A-C** |
| A -- Agent runtime | 14 | 2-3 weeks | 1-2 | Reduced from 18 (types + watchers exist) |
| B -- Relay | 8 | 2-3 weeks | 1 | Reduced (WS/feed routes exist) |
| C -- Gateway pipeline | 9 | 2-3 weeks | 1-2 | Reduced from 12 (no separate crate) |
| D -- Auth | 5 | 1-2 weeks | 1 | Reduced from 8 (middleware + team exist) |
| E -- Knowledge | 8 | 2-3 weeks | 1 | Reduced from 12 (store + HDC exist) |
| F -- Groups | 6 | 1-2 weeks | 1 | Reduced from 8 (coordination exists) |
| G -- Arenas | 7 | 2-3 weeks | 1 | Reduced from 12 (marketplace + jobs exist) |
| H -- DeFi | 41 batches | 8-12 weeks | 2-4 | Unchanged (detailed in gap docs) |
| I -- Meta | 6 | 2-3 weeks | 1 | Reduced from 8 (metamorphosis + eval gen exist) |
| J -- Registries | 4 | 1-2 weeks | 1 | Reduced from 10 (agent/rep/validation registries exist) |
| K -- Visual composition | 5 | 1-2 weeks | 1 | Reduced from 10 (template CRUD + jobs exist) |
| L -- Dashboard architecture | 6 | 2-3 weeks | 1 (frontend) | Refined (backend exists, frontend-focused) |
| M -- Universal APIs | 8 | 2-3 weeks | 1-2 | Reduced from 12 (connector/feed types exist) |
| **Total** | **~137** | **~30-42 weeks** | **4 agents = ~8-10 weeks** |

### Updated parallel execution plan (4 agents)

| Agent | Phase sequence | Estimated weeks |
|-------|---------------|-----------------|
| Agent 1 (Core) | A → F → I → K | 5-7 |
| Agent 2 (Infra) | C → D → J | 4-6 |
| Agent 3 (Data) | OG.1-2 → B → E → G | 6-8 |
| Agent 4 (Learning) | OG.3-4 → H (Tier 0-5) | 8-12 |

Dashboard (Phase L) and Universal APIs (Phase M) run on frontend/utility agents alongside backend phases.

### Implementation readiness by phase

| Phase | Readiness | Notes |
|-------|-----------|-------|
| OG | **Ready now** | No dependencies, improves core loop quality |
| A | **Ready now** | Foundation layer, no dependencies |
| B | After A | Agents must publish heartbeats |
| C | **Ready now** | Standalone, parallel with A |
| D | After C | Gateway holds API keys |
| E | After A | Extension hooks for knowledge publishing |
| F | After A + B | Agent runtime + relay rooms |
| G | After A + E | Agents to compete, reputation feeds |
| H | After A + C | Agent runtime + inference gateway |
| I | After A + E + G | Full dependency chain |
| J | After E + G | Knowledge + arena registries |
| K | After C | Gateway for chat model calls |
| L | After B + C | Frontend, backend must be ready |
| M | After A + B | Agent runtime + relay |
