# Mori Parity Gap Analysis

> Generated 2026-04-09. Exhaustive comparison of roko vs mori, PRD docs, component specs, and original bardo crates.

## Root Cause

The codex automation script (`tmp/run-parity.sh`) never passed mori reference files, PRD docs, or component specs as context to the agent. The `context_files_for()` function only passes roko's own source files. The MASTER-PLAN sections explicitly reference mori source paths, PRD directories, and component specs — but the script ignores them. Every section was implemented from abstract checklist descriptions rather than from the reference material.

---

## 1. TUI Dashboard (biggest visual gap)

**Mori**: 20,678 lines, 55 files, 7 tabs, 26 widgets, 13 modals, atmosphere/particle effects, ROSEDUST theme, mouse support, 136 keybindings
**Roko**: 8,858 lines, 8 files, 13 "pages" (6 with real widgets, 7 text scaffolds), 2 overlays, no effects, basic terminal colors, 15 keybindings

**Reference**: `/Users/will/dev/uniswap/bardo/apps/mori/src/tui/` (20K LOC)
**Component spec**: `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/roko-tui.md`

### Missing views
- **Git view** (`views/git_view.rs`, 691 lines) — branch tree, commit graph, diff view
- **Processes view** (`views/processes.rs`, 639 lines) — process supervisor monitor
- **Context/Inspect view** (`views/context.rs`, 1,001 lines) — MCP tools, context window
- **Interactive config** (`views/config.rs`, 762 lines) — editable settings with toggles
- **Plans view** (`views/plans.rs`, 1,516 lines) — hierarchical plan tree with wave grouping, expand/collapse, inline filters. Roko has a flat table.
- **Dashboard view** (`views/dashboard.rs`, 434 lines) — master-detail split with 7 sub-tabs (Agents/Output/Diff/Errors/Git/MCP/Processes). Roko has a single-panel layout.

### Missing widgets (18 of 26)
- `agent_output` (1,679 lines) — scrollable live agent transcript with ANSI parsing
- `plan_tree` (1,077 lines) — collapsible wave-grouped tree
- `phase_bar` / `phase_compact` / `phase_timeline` — pipeline phase visualization
- `wave_bar` / `wave_progress` — execution wave display
- `diff_panel` — colored unified diff viewer
- `error_digest` — grouped error display by file
- `command_output` — shell/gate output panel
- `context_gauge` — context window utilization
- `branch_tree` — hierarchical git branches
- `agent_grid` / `agent_pool` / `parallel_pool` — agent layout views
- `sys_metrics` — CPU/memory/disk gauges
- `token_bar` — per-agent token budget
- `braille` — fine-grained animation primitives

### Missing modals (11 of 13)
- `plan_detail` (458 lines) — multi-tab plan inspection
- `task_detail` (628 lines) — per-task metadata, checklist, output, gates
- `confirm` (460 lines) — destructive action confirmation
- `queue_overview` (331 lines) — global plan navigator
- `agent_pool_modal` — agent slot roster
- `approval` — capability-escalation Y/N gate
- `inject` — steering message text input
- `notification` — ephemeral toasts
- `task_picker` — fuzzy-searchable task picker
- `wave_overview` — wave DAG inspector
- `quit` — exit confirmation

### Missing infrastructure
- **ROSEDUST theme** (`theme.rs`, 265 lines) — 20+ RGB color constants, per-role/per-phase accents, gradient system (fire/ocean/ember/amber/sage), `brighten()`, focused/unfocused borders
- **Atmosphere** (`atmosphere.rs`, 284 lines) — particle system (500 cap), heartbeat, breathing, shimmer, spinner
- **Post-processing** (`postfx.rs`, 444 lines) — bloom, vignette, dim overlay, modal glow, ambient orbs, drop shadow, amber color grade
- **NERV visualization** (`nerv_viz.rs`, 355 lines) — progress-driven percolation, activity ripples, data rain
- **Visual effects** (`vfx.rs`, 104 lines) — plasma, noise, FBM, voronoi, ripple generators
- **Color utilities** (`color.rs`, 179 lines) — HSV conversion, screen/additive blend, gradient LUTs
- **Bars** (`bars.rs`, 190 lines) — gradient bar, segmented bar, NERV gauge, semantic bar
- **Hit testing** (`hit_test.rs`, 349 lines) — mouse zone computation
- **Math** (`math.rs`, 140 lines) — Vec2, easing functions, wave combinators
- **Effects config** (`effects_config.rs`, 91 lines) — toggleable effects with degraded mode
- **Mouse support** — click, scroll with terminal coordinates
- **Input modes** — Normal/Inject/Filter/Confirm
- **Focus zones** — Tab cycling between panels

---

## 2. Orchestrator / Executor

**Mori**: 27 executor action variants, task-level DAG scheduling, full merge queue, rich crash recovery (15-field snapshots)
**Roko**: 10 action variants, plan-level only (no task awareness), no merge queue, minimal crash recovery (3-field snapshots)

**Reference**: `/Users/will/dev/uniswap/bardo/apps/mori/src/app/parallel.rs`, `/Users/will/dev/uniswap/bardo/crates/bardo-orchestrator/src/executor.rs`
**Component specs**: `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/orchestrator/parallel-executor.md`, `crash-recovery.md`, `merge-queue.md`, `worktree-manager.md`

### Missing executor actions (17 of 27)
- `CreatePipeline`, `EnsureWorktree`, `SpawnTaskAgent` (with GlobalTaskId), `SpawnTaskAgentBatch`
- `PreSpawnWarmReviewer`, `CancelActiveReviewer`
- `SpawnRefactorer`, `RunIntegrationTests`, `RunPostMergeRegression`
- `ReGatePlan`, `RebasePlanBranch`, `AutoFixErrors` (with structured error pass-through)
- `CleanRetryPlan`, `RegenerateVerifyScript`, `ForceAdvancePlan`
- `PlanTimeout`, `SpawnImplementer` (express mode)

### Missing task-level scheduling
- No `GlobalTaskId` (plan + task_id)
- No `UnifiedTaskDag` (cross-plan task dependency graph)
- No `in_flight_tasks` tracking
- No per-task failure counts or backoff
- No batch spawning (multiple ready tasks to one agent)
- No cross-plan task dependencies

### Missing merge queue
- No `MergeQueue` data structure
- No file-set overlap conflict detection
- No FIFO-with-conflict-skip ordering
- No stall detection or force-advance
- No `RebasePlanBranch` on conflict

### Missing crash recovery fields (12 of 15)
- `completed_tasks`, `in_flight_tasks`, `skipped_tasks`, `task_failure_counts`
- `merge_queue`, `plans_since_refactor`, `plans_since_integration_test`
- `review_feedback`, `verify_error_signatures`, `consecutive_verify_fails`, `verify_regenerated`
- `revision` counter, `.bak` rotation

### Missing agent lifecycle management
- No instance IDs (unique per-agent-per-task-per-iteration)
- No primary agent warmth (reuse context across phases)
- No warm reviewer overlap (pre-spawn during gating)
- No spawn failure tracking/backoff (exponential 2s→30s, 10 failures = Failed)
- No dead-on-arrival detection (<15s exit with no output)
- No zombie reaping (>4 hour agents)
- No spawn generation tracking (prevents stale results)

### Missing worktree management
- No `RecoveryDecision` enum (Healthy/NeedsResync/NeedsRebase/ParseRepair/Quarantine/ManualAttention)
- No disk-space-aware reclamation (`reclaim_to()`, `available_disk_kib()`)
- No worktree index persistence (in-memory only, lost on crash)
- No `DirtyUntrackedOverflow` detection
- No ENOSPC handling with automatic retry
- No auto-cleanup of merged plan worktrees

### Roko advantages (keep these)
- Pure executor (no I/O in state machine) — cleaner than mori
- Enriching and DocRevision phases — mori doesn't have these
- Event-log recovery with hash-chained audit entries
- Per-plan `AuditChain` for tamper-evident phase transitions

---

## 3. Conductor

**Mori**: LLM-backed conductor agent, 10+ action types, graduated escalation, cooldown dedup, rate limiter
**Roko**: Heuristic-only, 3 severity levels, no LLM reasoning, no cooldown

**Reference**: `/Users/will/dev/uniswap/bardo/crates/bardo-conductor/`
**Component spec**: `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/roko-conductor.md`

### Missing conductor features
- **LLM conductor agent** with 17+ parseable directives (NUDGE/RESTART/SKIP-REVIEWS/FORCE-ADVANCE/THROTTLE/RETRY-PLAN/SOFT-RETRY/REBASE-REGATE/PHASE-REJECT/ENRICH)
- **State snapshot builder** — rich Markdown summary fed to LLM conductor
- **Directive parser** — structured response parsing
- **Agent spawn rate limiter** — priority-based spawn queue
- **7 intervention tiers** (Nudge/Suggest/Enforce/Pause/Rollback/Abort/Escalate) — roko has 3
- **Per-plan phase timeout escalation** — restart counter with force-advance after 2
- **Cooldown deduplication** — 120s per watcher:plan:role key
- **User inject/steer handling** — operator message forwarding during execution

### Missing watchers
- `AgentSilence` — no output for 180s triggers nudge
- `TaskStall` — no task progress for 300s
- `TaskContinuation` — warm-agent reuse for next ready task

### Missing conductor actions
- `SkipReviews`, `SpawnValidation`, `GenerateFixPlan`, `InsertGate`, `SkipValidation`
- `AssignAdditionalTasks`, `PingWarmAgent`

### Roko advantages (keep these)
- `StuckPatternWatcher` — 6 stuck-condition heuristics (mori had simpler inline detection)
- `CostOverrunWatcher`, `SpecDriftWatcher` — promoted from health checks to full watchers
- `DiagnosisEngine` — 33+ error patterns with confidence scores
- `HealthMonitor` — 4 composable health checks with `SystemSnapshot`

---

## 4. Gates

**Mori**: Affected-crate scoping, cargo semaphore, shared target/sccache, error digest extraction, pattern sharing, nextest detection
**Roko**: Full workspace runs, no build caching, raw output only

**Reference**: `/Users/will/dev/uniswap/bardo/crates/bardo-gate/`, `/Users/will/dev/uniswap/bardo/apps/mori/src/orchestrator/gates.rs`
**Component specs**: `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/gates/`

### Missing gate types
- **Format gate** — auto-format + always-pass
- **Cargo.toml validation gate** — workspace dependency cross-referencing

### Missing gate infrastructure
- **Affected-crate scoping** — `git diff --name-only` to scope cargo commands to changed crates only
- **Cargo gate semaphore** — 2-permit global throttle on concurrent cargo processes
- **Shared target directory / sccache** — env vars for cross-worktree build caching
- **Cache corruption detection** — detect "can't find crate for" and retry in isolated target
- **Error digest extraction** — parse `error[E...]` blocks, deduplicate, cap at 10
- **Discovered pattern sharing** — persist error patterns to JSON for cross-agent learning
- **Nextest detection** — prefer `cargo nextest` when available
- **Failing test name extraction** — structured test name parsing for targeted retry
- **Test failure snippet extraction** — focused 50-line failure section
- **`is_mostly_passing()` classification** — >90% pass rate classification for force-advance decisions
- **`PipelineVerdict` with rung-tagged steps** — per-rung results (spec requirement)
- **`StepHook` trait** — per-step callbacks
- **`GateRegistry` trait** — rung-to-gate lookup
- **`feedback_for_agent()`** — pipeline-level feedback formatting
- **Panic isolation** — catch panics in inner gates via `tokio::task::spawn`

### Roko advantages (keep these)
- `SymbolGate`, `GeneratedTestGate`, `PropertyTestGate`, `LlmJudgeGate` — new gate types
- `GateRatchet` — monotonic quality ratchet
- `AdaptiveThreshold` — EMA-based adaptive thresholds
- Build-system agnostic (Cargo/Go/Npm/shell)
- 6-rung selector with complexity-based escalation

---

## 5. Learning / Cognitive Layer

**Mori**: 25K+ LOC across golem-grimoire (13,666), golem-dreams (2,844), golem-daimon (8,907)
**Roko**: 15K LOC in roko-learn (online monitoring and optimization only)

**PRD docs**: `/Users/will/dev/nunchi/roko/bardo-backup/prd/07-grimoire/`, `prd/11-daimon/`, `prd/13-dreams/`
**Component specs**: `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/learn/`
**Mori crates**: `/Users/will/dev/uniswap/bardo/crates/golem-grimoire/`, `golem-dreams/`, `golem-daimon/`

### Missing: Knowledge hierarchy (Grimoire)
- **5-type knowledge hierarchy** (Episode/Insight/Heuristic/Warning/CausalLink/StrategyFragment) — roko has flat playbooks/skills
- **Admission gate (A-MAC)** — 5-factor scoring (utility/confidence/novelty/recency/type) with hallucination firewall
- **Decay system** — Ebbinghaus forgetting curves (`R = e^(-t/S)`) with 4 decay classes (Structural/RegimeConditional/Tactical/Ephemeral)
- **Curator cycle** — 50-tick maintenance (validate/prune/compress/cross-reference)
- **Causal graph** — directed relationships between phenomena (1,229 LOC in mori)
- **Hierarchical organization** — multi-level knowledge structure (1,702 LOC in mori)
- **Cross-agent knowledge sharing** — fleet-level knowledge (1,917 LOC in mori)
- **Retrieval scoring** — multi-factor retrieval with relevance/recency/confidence (1,137 LOC in mori)

### Missing: Memetic evolution
- **Fitness computation** — `W(E) = fidelity * fecundity * longevity`
- **Replicator dynamics** — `dx_i/dt = x_i * (W_i - W_bar)`
- **Price equation diagnostics** — decomposing improvement into selection vs transmission
- **Epistemic parasite detection** — high-fitness but negative-quality entries
- **Immune response** — self-prediction error penalties
- **AntiKnowledge type** — permanent negative knowledge with confidence floor

### Missing: Consolidation (Dreams)
- **DreamRunner** — cluster failures, extract rules/skills, prune stale, refresh baselines
- **Mattar-Daw replay** — utility-weighted episode replay (`utility = gain * need`)
- **Counterfactual simulation** — Pearl's SCM for "what if I had done X?"
- **Catastrophic forgetting prevention** — interleaved recent+old replay
- **Dream scheduler** — trigger based on accumulated prediction residuals

### Missing: Affect system (Daimon)
- **PAD vectors** (Pleasure-Arousal-Dominance) with 8 named octant states
- **Appraisal engine** — 8-step pipeline (OCC, Scherer, Chain-of-Emotion models)
- **Somatic markers** — situation-emotion associations (Damasio 1994)
- **Emotional memory** with somatic landscape (k-d tree)
- **Depotentiation** — REM-phase emotional charge reduction (Walker & van der Helm 2009)
- **Learned helplessness detection** — low dominance for 200+ ticks (Seligman 1972)
- **Emotional contagion** — cross-agent affect transfer

### Missing: Self-optimization loops
- **DSPy/MIPROv2** prompt optimization — Python sidecar (Bergstra et al. 2011)
- **Bayesian optimization (TPE)** over continuous parameter spaces
- **ADAS meta-agent** — architecture search (Hu et al. 2024)
- **Scaffold optimizer** — per-section LinUCB bandit for prompt inclusion/exclusion
- **Prediction tracker** — Brier score calibration for all learning-stack predictions
- **Chaos engineering** — fault injection for recovery testing
- **Eval framework** — systematic evaluation harness

### Roko advantages (keep these)
- `CascadeRouter` — three-stage cascade with UCB (not in mori)
- `PromptExperiment` — A/B testing framework
- `AdaptiveThreshold` — EMA gate thresholds
- `RuntimeFeedback` — unified integration point for all subsystems per run
- `HDCClustering` — episode clustering via Binary Spatter Codes

---

## 6. Agent / Compose / MCP

**Reference**: `/Users/will/dev/uniswap/bardo/apps/mori/src/agent/`, `/Users/will/dev/nunchi/roko/bardo-backup/tmp/mori-agents/`
**Component specs**: `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/agents/`, `COMPONENTS/compose/`

### Missing agent backends
- **Cursor ACP** — persistent JSON-RPC over stdio (Agent Communication Protocol)
- **Codex app-server** — persistent `codex app-server` JSON-RPC process
- **Automatic backend routing** — `from_model(slug)` dispatches to correct backend

### Missing agent lifecycle
- **Warm pool** — pre-spawn/promote/evict pattern (saves 5-15s per transition)
- **Per-turn dollar budget** (`--max-budget` $0.40-$3.00 per role per model)
- **Per-role tool permission matrix** — encoded role→tools mapping
- **Fallback model retry** on spawn failure
- **Persistent PID registry** surviving crashes (`.mori/runtime/agent-pids.json`)
- **Descendant collection** before kill (snapshot full process tree first)

### Missing streaming
- **Real-time events** during agent execution (MessageDelta, DiffUpdated, TokenUsage, ToolCall, CommandOutput)
- **Stream batching** (256 bytes for messages, 512 for commands)
- **ApprovalRequested** / approval response flow

### Missing prompt composition
- **Skill injection** from `.claude/skills/` directory
- **Iteration feedback compression** (3-bullet fix directives on retry)
- **Disk-based prompt/context pack caching** (SHA256-keyed)
- **In-memory prefix caching** for shared plan sections
- **Verification isolation** (hide test source from implementer)
- **PromptBuild metadata** (cache_hit, playbook_hits, verify_artifacts_fresh)
- **Per-role behavioral guidance stanzas**
- **Per-role artifact hints** (Strategist→brief.md, Auditor→rubric.md)
- **Prior task output compression** (50K→20 lines)

### Missing environment
- **sccache integration** (RUSTC_WRAPPER, SCCACHE_BASEDIRS)
- **Per-worktree target directory**
- **Isolated runtime homes per agent instance**
- **Gateway routing env vars**

### Roko advantages (keep these)
- **ToolLoop** — standalone multi-turn loop for raw LLM backends (ahead of mori)
- **ToolDispatcher** — full safety pipeline with 11 stages (mori delegates to CLIs)
- **SafetyLayer** — 6 policy families (bash/git/network/path/rate_limit/scrub)
- **MCP client** — standalone JSON-RPC client with dynamic registry + tool dedup

---

## Summary by priority

### P0 — Core functionality gaps
1. Task-level scheduling in executor (blocks efficient plan execution)
2. Merge queue (blocks safe parallel plan merging)
3. Per-turn dollar budget (blocks cost control)
4. Streaming events (blocks real-time TUI)
5. Error digest extraction in gates (blocks targeted agent feedback)
6. Affected-crate scoping in gates (blocks performance at scale)

### P1 — TUI parity
7. Port mori's 7 views + layout from `/Users/will/dev/uniswap/bardo/apps/mori/src/tui/views/`
8. Port ROSEDUST theme from `tui/theme.rs`
9. Port 18 missing widgets from `tui/widgets/`
10. Port 11 missing modals from `tui/modals/`
11. Port atmosphere + post-processing effects

### P2 — Learning / cognitive layer
12. Knowledge hierarchy with decay and admission (Grimoire)
13. Dream consolidation (DreamRunner)
14. Affect system (Daimon PAD model)
15. Self-optimization loops (DSPy, TPE, scaffold optimizer)

### P3 — Agent infrastructure
16. Cursor ACP + Codex app-server backends
17. Warm agent pool
18. Skill injection in prompt composition
19. Custom MCP server with code intelligence
20. Crash recovery enrichment (15-field snapshots)

---

## 7. Prompt Composition (second-pass findings)

**Mori**: 5,914 LOC in `prompts.rs` alone, 24 role-specific prompt builders, in-memory + disk caching, learning pack injection, skill loading, iteration memory, feedback compression, 16 task-metadata scaling multipliers
**Roko**: 10 prompt templates, no caching, no learning pack, no skills, no feedback compression

### Missing prompt functions (14 of 24)
- `implementer_fix_prompt()` — lightweight retry-specific prompt (iteration 2+)
- `combined_reviewer_prompt()` — single agent running architect+auditor perspectives
- `pre_planner_prompt()` — pre-planning pass before strategist
- `batch_refactorer_prompt()` — cross-plan refactoring
- `task_implementer_batch_prompt()` — batch task implementer (multiple tasks → one agent)
- `reviewer_prompt_for_plan()` — post-merge reviewer
- `doc_verifier_prompt()` — documentation verification
- `merge_resolver_prompt()` — merge conflict resolution
- `error_diagnoser_prompt()` — structured error diagnosis
- `dependency_validator_prompt()` — dependency check
- `pattern_extractor_prompt()` — pattern extraction from episodes
- `express_implementer_prompt()` — single-pass express mode
- `auto_fix_prompt()` — express gate failure auto-fix
- `generate_static_brief()` — express mode brief generation

### Missing prompt infrastructure
- **SharedPlanContext** — byte-identical prefix for API cache hits (saves 90% token cost)
- **In-memory prefix cache** — `IMPLEMENTER_PREFIX_CACHE`
- **Disk context-pack cache** — SHA-256 keyed to `.roko/memory/context-packs/`
- **Learning context pack** — assembles episodes+playbook+research+deps+fixtures into prompt section
- **Skill loading** from `.claude/skills/` with role defaults and auto-detection
- **Iteration memory** — JSON files per-plan between retries
- **Feedback compression** — parses structured TOML review, extracts unresolved blocking issues
- **LLM reflection** — spawns haiku on gate failure for structured "what/why/fix" diagnosis
- **Prompt logging** — persists metadata per prompt
- **16 task-metadata multipliers** — dynamic section sizing from task metadata
- **Verification isolation** — hide test source from implementer (prevents reward hacking)

---

## 8. Code Intelligence (second-pass findings)

**Mori**: mori-mcp (3,331 LOC) MCP server with 12 tools + mori-index (5,332 LOC) SQLite-backed index + mori-context (702 LOC) context assembly
**Roko**: roko-index skeleton (graph/HDC/parser/symbol, no DB/search/updates), no MCP server

This is the **single most impactful infrastructure gap**. It determines whether agents understand code structure or are limited to blind grep.

### Missing entirely
- **roko-mcp server binary** — no code intelligence MCP tools for agents
- **SQLite index database** — no persistence, no files/symbols/refs schema
- **Hybrid search** — no keyword + HDC + embedding fused via RRF
- **Incremental updates** — no content-hash change detection
- **Context overlays** — no namespace-scoped transient mutations
- **Context compression** — no token-budget-aware truncation
- **Privacy/redaction** — no policies for masking sensitive code

---

## 9. Live Health Monitoring (second-pass findings)

**Mori**: 12-pattern `MonitorPool` (~535 LOC) with live anomaly detection and steering injection
**Roko**: Nothing equivalent — only post-hoc gate results

This is the **largest architectural gap** in agent supervision. Mori's agents are monitored in real-time and steered when stuck. Roko's agents run blind until they finish, then gates check the output.

### Missing patterns
1. CompileFailRepeat (3+ same errors → nudge)
2. ContextWindowPressure (>80% → wrap up; >95% → restart)
3. StuckPattern (repeating output → restart)
4. TokenBurnRate (>2K tokens/min sustained → warn)
5. PhaseTimeout (>20min → graduated restart → force-advance)
6. TerminalUnresponsive (3+ health failures → restart)
7. TerminalRenderRegression (passed → now fails → alert)
8. GolemLifecycleViolation (3+ lifecycle failures → abort)
9. SpecDriftAccumulation (>25% drift → alert)
10. CoverageDrop (>5% drop → alert)
11. CrossCrateBreak (conflict → alert)
12. SpecWeakeningDetector (assertions removed → block)

---

## 10. Configuration Depth (second-pass findings)

**Mori**: ~80+ config fields with per-role model/effort/budget/context, multi-backend, express mode, knowledge toggles
**Roko**: ~15 top-level fields, single backend, no per-role settings

### Missing config capabilities
- Per-role model selection (`role_models: HashMap<String, String>`)
- Per-role effort levels (`role_effort: HashMap<String, ReasoningEffort>`)
- Per-role context limits (`role_context_k: HashMap<String, u32>`)
- Per-role budget caps (Implementer=$1.50, Strategist=$0.75, Conductor=$0.50, etc.)
- 3-tier task routing (fast_task_model, standard_task_model, complex_task_model)
- Multi-backend support (claude + codex + cursor simultaneously)
- Agent role toggles (architect_enabled, auditor_enabled, scribe_enabled)
- Express mode (single-pass, no reviews, auto-fix)
- Knowledge injection toggles (6 toggles + thresholds)
- Per-plan routing overrides
- Disabled providers list
- Auto-respond to agent questions (3 attempts then stop)
- Time estimator with EMA correction factors

---

## 11. Server / Daemon (second-pass findings)

**Mori**: SSE endpoint with `?since=seq` reconnect, persistent event journal, checkpoint resume, 3-tier auth, remote steering
**Roko**: WebSocket only, in-memory event bus (lost on restart), single API key auth, no steering

### Missing server features
- **Remote steering API** — `POST /queue/steer` maps to `ExecutorAction` (ForceAdvancePlan, CleanRetryPlan)
- **SSE endpoint** — `GET /events?since=N` for simple curl-based monitoring
- **Persistent event journal** — monotonic sequence survives daemon restart
- **Checkpoint resume** — session identity preserved across restart
- **3-tier auth** — Read/Write/Admin scopes per token
- **Directive classification** — `POST /ingest` classifies directives before persisting

---

## 12. Runtime Loop Architecture (third-pass findings)

**Mori**: Fully asynchronous event-driven `tokio::select!` loop (17,999 LOC in parallel.rs). Multiple concurrent arms: agent events, gate results, signals, timers, TUI keys. Non-blocking — agents stream in real time while gates run concurrently.
**Roko**: Synchronous tick loop (orchestrate.rs). Blocks on each `dispatch_action()`. One agent must complete before the next action dispatches.

### Behavioral gaps (28 items)

1. **Asynchronous event loop** — mori's `tokio::select!` with concurrent agent/gate/timer/TUI arms vs roko's sequential tick
2. **Persistent agent pool** — mori re-uses warm agent processes across turns (multi-turn sessions); roko starts fresh subprocess every time
3. **Agent streaming** — mori gets MessageDelta events in real-time (buffered 512 chars/agent); roko gets nothing until subprocess exits
4. **Warm reviewer pre-spawn** — mori pre-spawns reviewer during gating phase (saves 5-15s); roko doesn't
5. **Express mode** — mori has single-pass no-review auto-fix execution; roko doesn't
6. **Dead-on-arrival detection** — mori detects agents that exit <15s with <80 chars output; roko doesn't
7. **Auto-commit worktree changes** — mori inspects worktree and auto-commits after each agent turn; roko doesn't
8. **Approval auto-accept** — mori auto-approves Claude permission requests; roko uses --dangerously-skip-permissions
9. **Agent health ring** — mori tracks 20-sample rolling success/failure rate, warns at >70% failure; roko doesn't
10. **Spawn generation tracking** — mori prevents stale spawn completions from being processed; roko doesn't
11. **Duplicate dispatch suppression** — mori detects and suppresses duplicate gate/review/spawn actions; roko doesn't
12. **Global agent budget** — mori enforces max concurrent agents across all plans; roko only limits within a single plan
13. **ENOSPC disk reclamation** — mori detects disk full and prunes caches; roko doesn't
14. **Canonical task state reconciliation** — mori's `overlay_canonical_task_state()` reconciles snapshot with actual tasks.toml on disk; roko doesn't
15. **Multi-stage review** — mori has quick reviewer → architect → scribe → critic pipeline; roko has single reviewer
16. **Per-event cost delta tracking** — mori tracks cost deltas per TokenUsage event (rejects >$50 suspicious); roko only tracks post-completion
17. **Message processing throttle** — mori caps at 20 messages per tick to prevent CPU saturation; roko has no streaming
18. **Fixture auto-start/shutdown** — mori manages test fixture sidecars (databases, services); roko doesn't
19. **Integration test worktree** — mori spawns integration tests in separate worktrees; roko doesn't
20. **Refactorer agent** — mori spawns periodic cross-plan refactoring agents; roko doesn't
21. **Self-heal from canonical state** — mori accepts no-op exits if tasks.toml already marks tasks done; roko doesn't
22. **Stale/terminal exit handling** — mori ignores exits from plans already Complete/Failed; roko doesn't
23. **Time-to-first-output tracking** — mori records time from spawn to first MessageDelta for efficiency; roko doesn't
24. **Agent question auto-respond** — mori detects "Would you like..." patterns and auto-replies (3 attempts); roko doesn't
25. **Time estimator with EMA** — mori tracks estimated vs actual durations with correction factors; roko doesn't
26. **Concurrent gate execution** — mori runs gates in separate tokio::spawn tasks with 20-min timeout; roko blocks inline
27. **Vacuous implementation detection** — mori detects 0 code written and fails immediately instead of looping; roko doesn't
28. **Signal-handler-driven checkpoints** — mori writes checkpoints on SIGTERM/SIGHUP/SIGINT; roko relies on outer framework

---

## 13. Git Operations + Branch Strategy (third-pass findings)

**Mori**: 3-tier branch hierarchy (main → batch → plan → task), 5 worktree types, sophisticated merge pipeline, recovery system
**Roko**: Single-tier branches (main → plan), 1 worktree type, bare git merge in main repo

### Branch strategy gaps
1. **No batch branch** — mori creates `codex/batch/YYYYMMDD` as integration target; roko merges directly into current branch
2. **No per-task branches** — mori creates `codex/plan/{base}/{task_id}` for parallel task isolation; roko has no task branches
3. **No utility/detached worktrees** — mori has ephemeral worktrees for pre-planning, integration testing, refactoring

### Worktree gaps (10 items)
4. No worktree health diagnosis — mori has 5-state recovery (Healthy/NeedsResync/NeedsRebase/Quarantine/ManualAttention)
5. No recovery snapshots — mori archives diffs, logs, status, creates recovery refs under `refs/mori/recovery/`
6. No file overlay — mori copies Cargo.toml, plans/, prd2/ into worktrees while preserving tracked files
7. No IDE/agent config injection — mori writes .cursor/cli.json, .codex/config.toml, .cargo/config.toml per worktree
8. No shared cargo target redirect — mori sets CARGO_TARGET_DIR via .cargo/config.toml in worktrees
9. No disk space management — mori monitors available disk (30 GiB warn, 15 GiB critical), prunes caches
10. No in-progress operation detection — mori checks for MERGE_HEAD, rebase-merge, CHERRY_PICK_HEAD

### Merge flow gaps (7 items)
11. No worktree-safe merge — mori merges inside worktrees and uses `git update-ref`; roko runs merge in main repo
12. No merge feasibility check — mori does `git merge-tree --write-tree` dry run before real merge
13. No auto-commit before merge — mori auto-commits dirty state with --no-verify before merge
14. No auto-tagging — mori creates `plan/{plan_base}` annotated tags on merge
15. No temp worktree for batch→main merge — mori uses throwaway detached worktree to avoid disturbing user's working dir
16. MergeQueue built but not wired — 627 LOC exists in roko-orchestrator but orchestrate.rs doesn't use it
17. No diff capture — mori generates and stores branch diffs, worktree diffs, branch logs for recovery

### Data directory gaps (10 items)
18. No costs.db — mori tracks every API request in SQLite
19. No context-packs cache — mori has SHA256-keyed cache (7,191 files in production)
20. No prompt-logs — mori logs full prompt text with per-section token breakdown
21. No playbook.toml — mori has learned routing rules (96+ entries) as TOML
22. No efficiency-history.jsonl — mori has rolled-up trend snapshots
23. No dependency/fixture manifests — mori has cross-plan dependency graph + sidecar definitions
24. No runs/recovery/ — mori has timestamped recovery snapshots per plan
25. No per-plan artifact structure — mori stores brief, decomposition, prd-extract, research, rubric, integration per plan
26. Plans symlink vs canonical path resolution — mori has 526 LOC path resolver; roko has a symlink
27. No per-worktree MCP config — mori writes MCP config per worktree

---

## 14. Safety + Tool System (third-pass findings)

### Missing safety mechanisms
1. **Loop guard** — mori blocks at 5th identical tool+args call, warns at >80% tool domination. Roko has zero degenerate-loop detection. ~230 LOC, directly portable.
2. **Audit chain** — mori has SHA-256 hash-chained append-only audit log with 11 event types, verification, and on-chain anchoring. Roko has plain JSONL without integrity checking.
3. **Taint tracking** — mori has 6 taint sources, 4 validation gates, `CleanString` type. Roko treats all strings equivalently (model output = human input = external data).

### Missing tool system features
4. **Per-tool-call cost/latency recording** — mori's `ToolResult` has gas_used, cost_usd, latency_ms, ground_truth_source. Roko's `ToolResult` has content string only.
5. **Tool executor circuit breaker** — mori has sliding-window circuit breaker for tool handlers. Roko has no circuit breaker in the tool dispatch path.
6. **Per-tool-call audit chain** — mori appends every tool call to the hash chain. Roko logs episodes at orchestrator level but not individual tool calls.

### Roko safety advantages (keep these)
- Bash command denylist/allowlist with regex (mori has none — it's not a code agent)
- Git branch protection (force push, hard reset, branch delete on protected branches)
- Network sandboxing (scheme/host/private-IP allowlists)
- Path escape prevention (worktree sandbox, symlink denial)
- Secret scrubbing (9 patterns: API keys, JWTs, PEM keys, env vars)
- Rate limiting per (role, tool)
- Batch dispatch with parallel/serial partitioning
- Cancellation token racing tool handlers
- UTF-8 boundary-safe output truncation

---

## 15. Testing Infrastructure (third-pass findings)

### Missing test types
1. **Zero proptest coverage** — mori has 8 crates with proptest suites (1000+ LOC). Roko has none. Safety modules especially need fuzzing.
2. **No compile-fail tests** — mori uses trybuild for safety type invariants. Roko has none.
3. **No eval framework** — the spec exists (375 LOC) but no types, runners, or suite management are implemented.

### Roko testing advantages (keep these)
- Dedicated integration test crate (tests/) with 3,354 LOC
- End-to-end pipeline test (`coding_agent_full_loop`)
- MockToolDispatcher with FIFO expectations and assertion helpers
- Tool replay harness (JSONL recording and playback)
- Golden file tests for built-in tools
- Per-crate integration tests (8 crates)

---

## 16. CLI Surface (third-pass findings)

### Missing CLI commands/flags from mori
1. `mori enrich routing` — offline AI-powered task classification into routing metadata (complexity_band, reasoning_level, speed_priority, quality_profile, context_weight). **High impact** for model routing.
2. `mori learn --write-playbook` — explicit playbook refresh from episode history
3. `--max-agents N` — global concurrent agent cap
4. `--dry-run` for plan execution — show what would execute without running agents
5. `--express` flag — single-pass mode
6. `--preset` — quality/balanced/cost/speed execution presets
7. `--fallback-model` — model fallback on spawn failure
8. `mori plan draft` — create plan with PRD refs, dependency edges, crate metadata (roko's `plan create` is simpler)
9. `mori setup` — multi-backend provider/model config wizard with complexity-band routing

### Roko CLI advantages (keep these)
- 40+ commands (vs mori's ~15)
- `roko run "PROMPT"` — single-prompt universal loop
- `roko replay` — signal DAG walker
- Full PRD lifecycle (idea → draft → edit → promote → plan)
- Research agent (topic/enhance-prd/enhance-plan/enhance-tasks/analyze)
- `roko neuro` — knowledge store management
- Subscription/event-source CRUD
- Full daemon lifecycle with launchd integration
- HTTP serve + cloud worker

---

## 17. Data Schema Incompatibility (third-pass findings)

### Episode schemas are NOT compatible
Mori's Episode is a vector-store record (UUIDv7, 768-dim embedding, PAD emotional state, market regime, bi-temporal validity, bloodstain flag). Roko's Episode is an operational log (hash-derived ID, gate verdicts, token usage, signal hashes, headline flag). Different storage (LanceDB vs JSONL). Migration requires schema adapter.

### Signal/Event systems are architecturally incompatible
Mori: `GolemEvent` with monotonic sequence, typed `EventPayload` enum, tick-based timing, broadcast channel delivery.
Roko: Universal `Signal` with BLAKE3 content hash, `Kind` enum + opaque `Body`, decay, scoring, lineage DAG, JSONL persistence.
Cannot be directly exchanged.

### Task schemas are partially compatible
Core fields overlap (id, title, status, files, depends_on). But mori has 20+ routing/enrichment fields (category, reasoning_level, speed_priority, quality_profile, context_weight, etc.) that roko lacks, while roko has operational fields (timeout, retries, verify pipeline, tool whitelists) that mori lacks.

---

## 18. Learning Feedback Loop — BROKEN READ-BACK PATH (fourth-pass findings)

**The most operationally significant finding across all passes.**

Roko's `LearningRuntime` correctly records 11 subsystem updates per completed run (episodes, costs, provider health, playbook outcomes, skill extraction, pattern mining, cascade router, experiments). The WRITE path works.

**The READ-BACK path is broken.** In `orchestrate.rs` `build_learned_context()`, the `MatchContext` used for playbook rule matching has:
```rust
MatchContext { files: Vec::new(), tags: Vec::new(), category: None, error_signature: None, role: ... }
```
Empty files and tags means rules with file/tag triggers **never fire**. The learning data goes in but never comes back out in a targeted way.

### What mori does that roko doesn't (feedback that actually reaches agents)
1. **Plan-scoped episode history** — `render_plan_learning_md()` (230 lines) generates markdown with same-plan success/failure episodes, common failure signatures, matched playbook rules with confidence, learned execution hints. Written as `learning.md` into worktree.
2. **File intelligence** — `render_file_intel_md()` computes per-file difficulty profiles (pass rate, avg iterations, common errors, best model per file). Written as `file-intel.md` into worktree.
3. **Gate failure reflection** — `spawn_reflection()` calls Claude Haiku to generate structured diagnosis (what/why/fix/files). Stored in iteration memory, injected into next retry's prompt.
4. **Playbook hints override routing** — matched rules change model/provider/strategy/context-weight for the next task spawn. In roko, the match always has empty context so nothing fires.

### Fix required
- Populate `MatchContext.files` from the task's `files` field and `MatchContext.tags` from task tags
- Implement plan-scoped episode retrieval (filter by plan_id before learning context assembly)
- Implement file difficulty profiling
- Implement LLM reflection on gate failure with iteration memory persistence

---

## 19. Conductor Action Vocabulary — TOO COARSE (fourth-pass findings)

Roko's `ConductorDecision` has 3 variants: `Continue`, `Restart`, `Fail`.
Mori's conductor has 15+ action types including the critical:
- **`SendMessage`** — nudge a running agent with targeted guidance (most effective intervention)
- **`ForceAdvance`** — skip past stuck phases without killing
- **`SkipReviews`** — bypass stuck review loops
- **`AssignAdditionalTasks`** — feed more work to warm agents

**Without `SendMessage`, roko cannot nudge stuck agents.** It can only restart (lose all context) or fail (terminal). Mori's most effective pattern — sending the actual error text back to the agent with "try a different approach" — is structurally impossible in roko.

### Watcher-specific weaknesses
- **CompileFailRepeat**: roko has single threshold (3), mori has graduated 3→nudge/5→restart/7→force-advance with error text injection
- **ContextWindowPressure**: roko has single threshold (80%), mori has 80%→nudge "wrap up" + 95%→hard restart
- **ReviewLoop**: roko produces restart, mori produces SkipReviews (fundamentally different recovery)
- **TestFailureBudget**: roko detects regression (wrong concept), mori accepts partial success at 70% pass rate (right concept for force-advance)
- **Missing watchers**: AgentSilence (180s no output), TaskStall (300s no progress), PhaseTimeout (30min graduated), TaskContinuation (warm agent reuse)

---

## 20. Observability Gaps (fourth-pass findings)

### Missing (blocks TUI and diagnostics)
1. **File-based tracing** — roko writes to stdout only; when TUI owns terminal, logs are lost. Need `tracing_appender` to `.roko/runs/roko.log`.
2. **OS metrics** — no CPU/memory/disk/network collection (mori has full `SysCollector` with process attribution to plans and active `rustc` compilation detection). Roko has `sysinfo` dep but doesn't use it for metrics.
3. **Crash reports** — no structured `CrashReport` with backtrace, app state, recent logs, environment. Mori captures full crash context via global `CRASH_STATE` updated every 2s.
4. **`#[instrument]` coverage** — 1 annotated function in roko vs 18 in mori. Traces lack function context.
5. **Error signatures** — no deduplication of crash/error patterns.

### Roko advantages (keep these)
- **Log scrubbing** — `LogScrubber` with 8+ secret patterns wired into tracing subscriber (mori has none)
- **Health probes** — `ProbeRegistry` with structured JSON responses (mori has none)
- **Error taxonomy** — `RokoError` with 21 variants, `is_transient()`, `retry_policy()`, `log_level()` (mori uses scattered anyhow)
- **Prometheus metrics** — `MetricRegistry` with Counter/Gauge/Histogram and text exposition (mori has none)

---

## 21. Golem Crate Relevance Assessment (fourth-pass findings)

| Crate | LOC | Needed for roko? | Reason |
|---|---|---|---|
| **golem-triage** | 7,969 | **No** | 100% blockchain/DeFi transaction triage. Algorithms (Thompson, Hedge, MIDAS-R) already exist in roko-learn as bandits/cascade-router. |
| **golem-runtime** | 7,836 | **No** | Biological simulation (vitality, sleep pressure, habituation, death clock). bardo-runtime already covers process supervision, event bus, cancellation. |
| **golem-context** | 3,995 | **Partially** | Context window assembly with token budgets and regime-aware policies. Roko's SystemPromptBuilder + PromptComposer covers the core need. The `ContextLearner` (EMA-based weight tuning) is a useful concept not in roko. |
| **golem-heartbeat** | 10,228 | **No** | Tick-based heartbeat loop. Roko uses a different execution model (prompt → agent → result, not tick-based). |
| **golem-mortality** | 9,876 | **No** | Death/lifecycle systems. Intentionally removed from roko per naming decisions. |
| **golem-identity** | 7,146 | **No** | DeFi identity, signatures, wallets. |
| **golem-chain-intelligence** | 9,717 | **No** | Blockchain analysis. |

---

## 22. Prompt Text Content — 20-30% Ported (fifth-pass findings)

Roko's templates capture the structural skeleton (sections, budgets, cache layers) but only ~20-30% of mori's operational guidance text. The role identity strings are 500-800 chars in roko vs 2,000-10,000 chars of inline guidance in mori.

### Missing behavioral instructions (20+ items)
- "DO NOT re-implement from scratch" on fix iterations
- "max 3 attempts, then document and move on" self-validation policy
- Full 8-item "Working Against A Live Repo" reconciliation guidance (roko has 2 of 8)
- "What Reviewers Will Check" pre-emption block (accountability framing)
- 4-step "Before You Finish" checklist (grep for .unwrap(), check exports, verify doc comments)
- Completion report artifacts (completion.md, self-check TOML)
- "DO NOT REVISE for" nit exclusion list (clippy, naming, style, unwrap where ? cleaner)
- "fix_hint is mandatory on every [[issues]] entry" with example
- "An incomplete review that forces a second cycle is a failure of your role"
- "APPROVE if compile and tests pass" default reviewer posture
- "Do NOT perform broad dependency churn" auto-fix constraint
- "Do NOT edit Cargo.lock/Cargo.toml" unless error proves necessity
- "Prefer source fixes over manifest fixes"
- "Do NOT re-run cargo test — use gate results" for reviewers
- "Use MCP tools instead of rg for symbol lookup" with specific tool signatures
- "Start with context/in/{role}-pack.md when present" role pack guidance
- "Check first: If tmp/agent-messages.md exists, read it" steering message consumption
- "Write docs during implementation, not after"
- "Read FULL prd2 source files, not truncated extract"
- 7-section structure requirement for scribe with per-section instructions
- Scribe pre-submission 13-item checklist
- Diagram requirements (diagram-before-prose, color coding, max 15 nodes, minimum 4-8 per plan)
- Auditor 4-step invariant coverage check procedure
- Verify-chain script handling guidance

### Missing role-specific prompt templates (9 of 24)
- `refactorer_prompt()` — fix clippy, do NOT change public API
- `pre_planner_prompt()` — parallel task breakdown with estimation
- `express_implementer_prompt()` — single-pass, self-reviewing
- `batch_refactorer_prompt()` — cross-plan cleanup
- `doc_verifier_prompt()` — docs vs code verification
- `error_diagnoser_prompt()` — classify gate errors, write fix-plan TOML
- `merge_resolver_prompt()` — git conflict resolution with "keep both sides" rule
- `dependency_validator_prompt()` — pre-implementation dependency check
- `pattern_extractor_prompt()` — extract code patterns for playbook

---

## 23. Dispatch Loop Gates — NO-OP STUB (fifth-pass findings)

In `crates/roko-serve/src/dispatch.rs`, `run_template_gates()` (lines 1538-1545) is:
```rust
async fn run_template_gates(...) -> Vec<Verdict> {
    let _ = output;
    Vec::new()  // <-- ALWAYS EMPTY
}
```
This means agent output in the webhook/dispatch path is **never validated by gates**. The plan-execution path (orchestrate.rs) has real gates, but the server dispatch path does not. Episodes are logged with `gate_verdicts: []` regardless of output quality.

---

## 24. Express Mode — Infrastructure Exists but Not Routed (fifth-pass findings)

Roko has: config (`express_mode`, `max_auto_fix_attempts`, `auto_fix_model`), state machine (`AutoFixing` phase), `AgentRole::AutoFixer`, and dispatch handler.

Missing: the orchestrator loop never checks express_mode to skip Strategist/Enrichment or bypass reviews. No `generate_static_brief()` equivalent. No express-specific prompt builder. No "try rebase before failing" recovery.

---

## 25. Confirmed REAL — Not Stubs (fifth-pass findings)

These areas were verified as fully implemented, tested, and wired:
- **roko-plugin SDK** (1,055 LOC, 8 tests) — CronEventSource, FileWatchEventSource, PluginBuilder
- **Subscription system** (~1,018 LOC, 7 tests) — 5-dimension filter matching, concurrency/cooldown/dedup enforcement
- **Event sources** (~440 LOC) — Cron and FileWatch wired into runtime via `start_builtin_event_sources`
- **Agent templates** (~700 LOC, 3 tests) — Full CRUD, validation, `{{key}}` interpolation, experiment variants
- **Cross-plan dependencies** — TaskDef.depends_on_plan wired through executor, tested
- **Daemon subsystem** (1,244 LOC) — Background process, PID tracking, IPC socket, HTTP server, SIGHUP, SIGTERM, launchd
- **Bare mode** — Full parity with mori, no gap

---

## 26. Signal Features — Mostly Dead Code (sixth-pass findings)

The Signal type has rich features (Decay, Score, taint tracking, lineage, session) but at runtime only id, kind, body, tags, lineage, and provenance.author are exercised.

### Dead Signal features
- **Decay**: Only Episode signals use `WISDOM` (24h half-life). Ttl and Ebbinghaus variants never used. `prune()` never called from CLI.
- **Score**: Always `Score::NEUTRAL`. `NoOpScorer` used everywhere. `CompositionScorer` never instantiated. The 4-axis scoring algebra is completely unused.
- **Provenance.trust**: Set to different values (1.0/0.75/0.1/0.5) but never read at runtime.
- **Provenance.tainted**: `TaintTracker` has 15 tests but is never instantiated in CLI.
- **Provenance.session**: `with_session()` never called.
- **`Substrate::prune()`**: Implemented in FileSubstrate, never called at runtime.
- **`compact()`**: Implemented, never called at runtime.
- **GcEngine**: Has scan/collect/dry_run, never called from CLI.

### Signal features genuinely exercised
- **Lineage**: Set by `derive()`, walked by `roko replay`, displayed in TUI. Fully wired.
- **Tags**: Workhorse metadata system. Gate pass/fail, plan_id, rung all use tags.
- **Provenance.author**: Set on every signal, displayed in replay/status.
- **`Signal::derive()`**: Used by every agent and gate for lineage tracking.

---

## 27. Budget Enforcement — Decorative (sixth-pass findings)

Roko defines `BudgetConfig` with `max_plan_usd`, `max_turn_usd`, `prompt_token_budget`, and per-role `TurnBudget`. **None of these are enforced at runtime.**

- `max_turn_usd` is never checked against actual turn cost — no agent killed for overspending
- `max_plan_usd` depends on cost signals that aren't reliably produced
- `TurnBudget` per role is defined but never consulted at dispatch time
- `prompt_token_budget` (10K tokens) is not used during prompt assembly — the actual system uses character-based `PromptBudget` caps
- `CostsDb` and `CostsLog` are built but never instantiated in orchestrate.rs
- No live per-plan cost accumulation — costs can only be computed after-the-fact from efficiency events
- Context prune limit hardcoded at 102,400 tokens regardless of model (too conservative for 1M Opus)

---

## 28. AGENTS.md — Built Parser Never Connected (sixth-pass findings)

- `roko-compose/src/agents_md.rs` has a full `AgentsMd::parse()` parser with section filtering and keyword search. **Never imported or called from roko-cli.**
- The orchestrator reads AGENTS.md as a raw string only for reviewer/scribe roles, skipping the main implementer path entirely.
- No `<!-- role: ... -->` marker filtering (mori's pattern for giving each role only relevant sections).
- No AGENTS.md file exists in the roko repo root.
- The `SystemPromptBuilder.with_conventions()` layer could naturally receive AGENTS.md content, but the wiring doesn't exist.

---

## 29. Dead Task Routing Enums (sixth-pass findings)

`TaskReasoningLevel`, `TaskSpeedPriority`, `TaskQualityProfile`, `TaskContextWeight` are defined in roko-core with full serde support but **never consumed by any routing, prompt, or execution logic**. Only `TaskCategory` and `TaskComplexityBand` are actually used (by the cascade router and bandit learning). The dead enums are the ones needed for mori's `scaled_prompt_cap()` 16-multiplier system.

---

## 30. Dead Core Traits (sixth-pass findings)

Three of the six universal traits (`Scorer`, `Router`, `Composer`) are **never called from the CLI runtime**:
- `Scorer`: Only `NoOpScorer` used. `CompositionScorer`, `SumScorer`, `MulScorer` never instantiated.
- `Router`: `FirstRouter`, `HighestScoreRouter`, `RoundRobinRouter` never called. CascadeRouter is a learn-crate impl, not the core trait.
- `Composer`: `PromptComposer` exists but CLI uses `RoleSystemPromptSpec` directly, bypassing the trait.

The "universal loop" (`loop_tick()` / `TickOutcome`) described in CLAUDE.md as the core architecture is **dead code**. `orchestrate.rs` reimplements each step ad-hoc.

---

## 31. Dead Crates (sixth-pass findings)

Five crates are not reachable from the `roko` CLI binary:
- `roko-index` — code indexing (built, not imported by CLI)
- `roko-lang-rust`, `roko-lang-typescript`, `roko-lang-go` — language support (only used within dead roko-index)
- `roko-golem` — Phase 2+ chain/daimon/dreams (per CLAUDE.md)

---

## 32. Edge Cases — Missing Concurrent Run Protection (sixth-pass findings)

**No file lock or PID-based mutual exclusion** for `roko plan run`. Two simultaneous runs can:
- Interleave JSONL writes to `signals.jsonl` and `episodes.jsonl` (in-process Mutex only, not cross-process)
- Overwrite each other's `executor.json` snapshots
- Create conflicting worktree branches
- Double-dispatch the same task

Other edge case gaps:
- Empty task array silently succeeds as "all tasks done" (should reject)
- No pre-flight disk space check before starting
- No OOM/signal-killed distinction in failure classification
- No startup config validation (disabled provider warnings)
- No stale snapshot warning on resume
- `FileSubstrate::put()` calls `flush()` but not `sync_all()` (data not guaranteed durable on power loss)
- Hardcoded `provider: "anthropic"` at orchestrate.rs:3866 (no multi-backend routing)

---

## 33. Enrichment Extraction Quality — Dramatically Lower (seventh-pass findings)

The 7 non-LLM enrichment steps are structurally present in roko but produce dramatically lower-quality output due to a fundamental design constraint: roko's extraction functions take `String` inputs (no filesystem access), while mori's take `PathBuf` inputs and access the filesystem, episode log, playbook, and sibling plans.

| Step | Mori Quality | Roko Quality | Root Cause |
|---|---|---|---|
| **PRD** | Backtick path extraction, normalization, dedup | Keyword line grep | No path parsing |
| **Brief** | Targeted section extraction (Prerequisites/Imports/Exports) + verification checklist | Generic heading + first paragraph echo | No section targeting |
| **Tasks** | 7 heading filters, backtick file extraction, full metadata | Every `##` becomes a task, no file extraction | No heading filtering, no file path parsing |
| **Research** | Episode history + playbook rules + failure signatures + execution hints | Heading echo + task count + brief truncation | **No filesystem access** → no episodes, no playbook |
| **Dependencies** | TOML parsing, cross-plan scanning, keyword flags, typed DependencySpec | Keyword line grep with flat `note` field | **No filesystem access** → no cross-plan analysis |
| **Fixtures** | Manifest parsing, infrastructure-specific specs with commands/healthchecks | Keyword line grep with flat `note` field | **No upstream manifest parsing** |
| **Integration** | Manifest synthesis, surface detection, suggested test commands | Head-truncated paste of 3 artifacts | **No synthesis** |

The structural root cause: roko's `StepInputs` carries only `String` content (anti-pattern #8: "I/O at boundary only"). Mori's `StepInputs` carries `PathBuf` for repo_root and plan_dir, enabling filesystem access. To reach parity, roko must either (a) do the I/O upstream and pass richer structured data, or (b) relax the pure-function constraint.

---

## 34. Test Health — 8 Crates Cannot Compile for Tests (seventh-pass findings)

**8 of 27 crates fail to compile for tests**, blocking 1,169 of 3,432 test annotations.

Root causes:
1. **Feature gate cascade** (5 crates): `roko-learn` imports `roko_golem::AffectEngine` without enabling the `scaffold` feature. This breaks roko-learn, roko-cli, roko-conductor, roko-serve, roko-neuro.
2. **API rename drift** (1 crate): `roko-agent` test calls renamed `with_safety_policy()` → `with_safety()`.
3. **Crate API changes** (1 crate): `roko-plugin` uses removed `notify` crate types.
4. **Lifetime error** (1 crate): `roko-mcp-slack` has a temporary value dropped.

16 additional test failures in compilable crates due to stale assertions after refactors (compress `...` suffix, PlanPhase serialization format, scrubber redaction format, mcp-github parameter count).

**roko-daimon has ZERO tests** — the only crate with no test coverage at all.

---

## 35. Format Bandit — Built but NOT Wired (seventh-pass findings)

The most sophisticated "built but never connected" system in the codebase:
- **10 tool format families** (OpenAI, Anthropic, Hermes, Gemma4, Mistral, Pythonic, QwenXml, ReAct, JsonMode, Custom)
- **3 bandit implementations** (ProfileBandit, EpsilonGreedy, TrackAndStop)
- **Per-model format profiles** with fallback chains and demotion thresholds
- **Galileo TSQ scoring** with 4-component quality composite
- **Complete trace infrastructure** with format_used field

But at runtime: `select()` is called, the result is logged to tracing, and then **nothing happens with it**. Three disconnection points:
1. `selected_format` never passed to agent dispatch — no `with_format()` method exists
2. `translator_for()` never called from orchestrator — translators hardcoded per backend
3. `format_bandit.feedback()` never called — no learning, no demotion, no adaptation

---

## 36. DAG Execution — Task-Level Waves Built but Not Used (seventh-pass findings)

`UnifiedTaskDag` correctly computes task-level waves with topological ordering, file-overlap inference, and overflow spilling. But the `ParallelExecutor` operates at the **plan level** and does not consume the DAG's wave output. The executor dispatches `SpawnAgent { task: "next" }` and `orchestrate.rs` handles task selection separately.

Missing from mori's DAG:
- **`queue.toml` / `QueueConfig`** — milestone-based execution manifest entirely absent
- **Runtime file-conflict checking** — mori checks at dispatch time; roko bakes into DAG statically
- **`exclusive_files` per-task** — no per-task file exclusivity control
- **`__whole__` synthetic nodes** — plans without tasks not representable in DAG
- **Skipped task propagation** — no `next_runnable_with_skipped()`
- **`independent_groups()`** — no Union-Find for file-conflict batch optimization
- **Completion registry** — no cross-plan type/deviation tracking
- **`max_concurrent_tasks` not enforced** — declared in config but executor doesn't check it
- **`parallel_safe` and `parallel_with`** — parsed from frontmatter but never enforced

---

## 37. Un-Audited Mori Files — 62 of 158 Files (19,600 LOC) Never Referenced (eighth-pass findings)

Previous 7 audit passes covered 96 of 158 mori source files. The remaining 62 files (39%) include significant functionality:

**Highest-impact un-audited files:**
- `app/tui_actions.rs` (2,483 LOC) — TUI action handlers: pause/resume, restart phase/plan, force-advance, message injection, command approval, git reconcile, merge-to-main, task picker, config hot-reload, retroactive verification
- `state/mod.rs` (2,285 LOC) — full application state with 600+ fields
- `app/sequential.rs` (1,465 LOC) — legacy sequential execution mode
- `state/persistence.rs` (1,346 LOC) — state persistence layer
- `orchestrator/plan.rs` (534 LOC) — plan schema and metadata
- `orchestrator/paths.rs` (526 LOC) — canonical path resolution (526 LOC resolver)
- `orchestrator/review.rs` (502 LOC) — structured review parsing and verdict synthesis
- `orchestrator/fixture_lifecycle.rs` (345 LOC) — fixture management
- `orchestrator/refresh.rs` (333 LOC) — plan refresh logic
- `orchestrator/ingest.rs` (304 LOC) — external data ingestion
- `orchestrator/prompt_log.rs` (220 LOC) — prompt logging persistence
- `server/handlers.rs` (461 LOC) — HTTP handlers
- `server/sse.rs` (140 LOC) — SSE endpoint implementation
- `deploy/` (897 LOC across 3 files) — deployment infrastructure

---

## 38. No Interactive Human-in-the-Loop Controls (eighth-pass findings)

Mori's `tui_actions.rs` (2,483 LOC) provides keyboard-driven orchestrator control:
- **Pause/Resume** pipeline execution
- **Restart Phase** (kill agent, re-dispatch current phase with different prompt)
- **Restart Plan** (full state reset: events, context, git tags)
- **Force-Advance** (skip stuck plan, force-commit)
- **Message Injection** into running agent (turn interrupt + restart)
- **Command Approval** (approve/reject tool calls from agents)
- **Git Reconcile** (commit, merge, prune, staging — all from TUI)
- **Merge to Main** (guarded by config flag)
- **Task Picker** (re-ingest failed tasks)
- **Config Hot-Reload** (change model mid-run, kill/respawn affected agents)
- **Retroactive Verification** of completed plans

**Roko has NONE of these.** The text-mode dashboard is read-only. A stuck roko session requires killing the process. There is no pause, restart, force-advance, injection, approval, or config change during execution.

---

## 39. No Structured Error Classification in Auto-Fix (eighth-pass findings)

Mori classifies cargo errors into 5 typed variants (`ImportNotFound`, `TypeMismatch`, `MissingField`, `TraitNotImplemented`, `Other`) by parsing `--message-format=json` output. Simple errors (`ImportNotFound`, `MissingField`) are fixed by `cargo fix --allow-dirty` WITHOUT involving an LLM agent. Only complex errors get routed to an agent.

Roko sends ALL gate failures to the AutoFixer agent as raw text. No JSON diagnostic parsing, no error classification, no `cargo fix` auto-application. This wastes tokens on trivially fixable issues.

Also missing: `InvariantFailureClass` 3-way classification (CodeBug/SpecIssue/MissingTest), "spec is sacred" fix generation, human review queue for spec disputes, issue plan generation for stuck agents.

---

## 40. No Fixture Lifecycle Manager (eighth-pass findings)

Mori's `fixture_lifecycle.rs` (345 LOC) manages external dependencies required by tasks:
- `FixtureSpec` with kind (docker/mock_server/database/evm/process), entrypoint, healthcheck, reusable flag
- `FixtureManager` with start/healthcheck-poll/stop lifecycle
- Language-aware mock generation (wiremock for Rust, responses for Python, msw for TypeScript)
- Reusable fixture sharing across tasks
- TUI config: `auto_start_fixtures`, `enabled_fixture_kinds`, `max_fixture_concurrency`

Roko has no runtime fixture management. The enrichment templates reference fixtures but nothing starts/stops them at execution time.

---

## 41. Pipeline Phase Count — 14 vs 25 (eighth-pass findings)

Mori's pipeline has 25 phases including 11 specialized gate phases that roko lacks entirely:
- TerminalValidation, GolemLifecycleTest, DependencyDenyCheck, IgnoredTestCheck, SpecComplianceCheck, RegressionCheck, CoverageCheck, IntegrationTest, FullLoopTest, QuickFix, ErrorDiagnosis, DependencyValidation, PatternExtraction

Mori's pipeline is event-driven (`handle_event(PipelineEvent) -> Vec<PipelineAction>`) with smart routing: simple compile errors → `cargo fix`, complex → agent; code approved + docs need revision → DocRevision only; review cap hit → force-advance. Roko's state machine is a data-driven transition table without smart routing.

---

## 42. RunState — 100+ Orchestration-Critical Fields Missing (ninth-pass findings)

Mori's `RunState` (2,285 LOC) is a god object with 100+ fields tracking every aspect of execution. The most impactful missing categories in roko:

- **Review pipeline**: `plan_pending_reviews`, `plan_review_stage` (5-stage enum: ReviewerPending→ScribePending→DocRevisionScribePending→AuditorPending→CriticPending), `plan_doc_revisions`, `plan_code_revisions`, `consecutive_revise_count` — enables multi-pass convergent reviews
- **Time estimation**: `TimeEstimator` with EMA correction_factor (alpha=0.3), plan/task/phase estimates+actuals, wave-aware ETA, throughput-based parallel ETA — enables dashboard progress bars and scheduling
- **Per-agent instance**: `ParallelAgentState` (15+ fields per instance), `instance_tool_calls`, `instance_write_calls`, `instance_spawn_generation`, `auto_respond_count` — prevents false completions and stale spawn races
- **Cost accumulators**: `cumulative_cost_usd`, `cost_per_plan`, `cost_per_task` as live running totals (not post-hoc computation from JSONL)

---

## 43. Structured Review Parsing — Completely Missing (ninth-pass findings)

Mori's `review.rs` (502 LOC) provides the critical feedback loop between reviewers and implementers:
- `StructuredReview` type with `ReviewVerdict` (code/docs/overall × Approve/Revise/Skip)
- `ReviewIssue` with 7 categories (Compilation/Test/TypeMismatch/MissingImpl/Docs/Style/SpecDeviation) and 3 severities
- `parse_structured_review()` with JSON+TOML strategy chain
- `is_quick_fixable()` routing: Compilation/Docs/Style → quick-fix; MissingImpl/SpecDeviation → full re-implementation
- `REVIEW_JSON_SCHEMA` for `--json-schema` structured output
- 5-stage review pipeline tracked in RunState

Roko has reviewer prompt templates but **no structured output parsing**. The agent's review output is treated as opaque text. The orchestrator cannot distinguish "approved" from "needs revision", cannot route to quick-fix vs full re-implementation, and cannot track review stages per plan. This is the gap that prevents the implement→review→fix convergence loop.

---

## 44. Iteration Memory — Agents Repeat the Same Mistakes (ninth-pass findings)

Mori's `iteration_memory.rs` (276 LOC) records per-plan JSON files with gate results + reflection diagnosis per retry. On the next retry, `format_reflections_md()` injects smart-compressed history (last 3 iterations in full, older compressed to 180 chars) into the agent's prompt.

Roko has NO equivalent. When a task fails gates and retries, the agent gets no context about what went wrong previously. It can repeat the exact same errors because it doesn't know what it already tried.

---

## 45. Prompt Logging — Cannot Inspect Prompts (ninth-pass findings)

Mori's `prompt_log.rs` (221 LOC) captures every prompt as a standalone JSON file with:
- Full prompt text
- cl100k token count (via tiktoken)
- Per-section breakdown (section name → tokens + chars)
- Context-packing metadata (cache_hit, playbook_hits, research_prepass, artifact_freshness)
- Path: `.mori/memory/prompt-logs/{plan}-{task}-{timestamp}.json`

Roko has no prompt logging. When debugging agent behavior, you cannot see what prompt was sent, how many tokens each section consumed, or whether the context cache hit.

---

## 46. Quick Wins — 5 Fixes Under 50 Lines Each (ninth-pass findings)

| # | Fix | Lines | Impact |
|---|---|---|---|
| 1 | Enable `scaffold` feature for roko-golem in roko-learn/Cargo.toml | **1** | Unblocks 5 crates (1,169 tests) |
| 2 | Populate MatchContext files/tags from TaskDef in orchestrate.rs:4117 | **~8** | Makes playbook rules with file/tag triggers actually fire |
| 3 | Add `sync_all()` after flush in file_substrate.rs:208 | **1** | Guarantees signal durability on power loss |
| 4 | Fix roko-plugin test imports (notify crate API change + lifetime) | **~7** | Unblocks roko-plugin tests |
| 5 | Fix safety_integration.rs API rename (with_safety_policy→with_safety) | **~35** | Unblocks roko-agent tests |

---

## 47. Codex Batch 4 Exhaustive Audit — `roko-batch4-cognitive` branch (2026-04-10)

> 83 commits (4C.03–5F.23) from automated codex run. Workspace compiles clean (only doc warnings).
> Audit performed by Claude Opus post-run. Zero `TODO`, `todo!()`, `unimplemented!()` macros found.

---

### 47.1 Full Commit Inventory (83 commits, 6 series)

**4C series — Deploy / Cloud (12 commits)**

| Commit | Hash | Subject |
|---|---|---|
| 4C.03 | `4197df8` | Wire `roko deploy railway` — calls `railway_api.rs` GraphQL backend |
| 4C.04 | `ff15527` | Wire `roko deploy fly` — generates `fly.toml`, runs `flyctl deploy` |
| 4C.05 | `b123c4b` | Wire `roko deploy docker` — `docker build` + `docker tag` (no push) |
| 4C.06 | `502944d` | Health check endpoint `GET /api/health` returns `{"status":"ok"}` |
| 4C.07 | `c503648` | `--cloud` flag on `roko init` — cloud-optimized config template |
| 4C.08 | `4205994` | `register_github_webhook()` — post-deploy webhook registration via octocrab |
| 4C.09 | `909d6c7` | `[serve.deploy]` config section in roko.toml schema |
| 4C.10 | `30468ee` | Post-deploy hook: iterate `[[serve.deploy.webhooks]]` after successful deploy |
| 4C.11 | `44c7813` | `CloudExecutionConfig` struct for remote execution |
| 4C.12 | `2941be8` | Cloud execution flow: clone → branch → plan-run → commit → push → PR |
| 4C.13 | `93f3a48` | Git helper functions for cloud execution (clone, checkout, commit, push, cleanup) |
| 4C.14 | `2251265` | Persistent storage: `.roko/` on volume mount for cloud deploys |

**5B series — Context Assembly (9 commits)**

| Commit | Hash | Subject |
|---|---|---|
| 5B.01 | `a3924be` | Create `ContextAssembler` struct in `roko-compose/src/context_assembler.rs` |
| 5B.02 | `bb4df18` | Stage 1 — Gather: query `KnowledgeStore` for entries matching task tags/slug |
| 5B.03 | `41a9b56` | Stage 2 — Rank: active inference scoring (from 12a §E2) |
| 5B.04 | `f487b19` | Attention-curve positioning (Liu et al. U-shape, from 12a §E3) |
| 5B.05 | `80f964e` | Affect-modulated retrieval (from 12a §E4): Daimon state biases retrieval |
| 5B.06 | `af7a86e` | Stage 3 — Compress: chunks ranked below 50th percentile are summarized |
| 5B.07 | `9efc407` | Stage 4 — Inject: format assembled context as structured markdown section |
| 5B.08 | `b5e0551` | Stage 5 — Validate: count total tokens in final system prompt |
| 5B.09 | `9c136b0` | Wire into `orchestrate.rs`: replace inline context building with `ContextAssembler` |

**5C series — Daimon / Affect Engine (13 commits)**

| Commit | Hash | Subject |
|---|---|---|
| 5C.02 | `5f1f92b` | Add `roko-daimon` to workspace members |
| 5C.03 | `a08ca64` | Define `AffectState` struct using full PAD model |
| 5C.04 | `683e766` | 8 named affect states from PAD octants (`+P+A+D` = Exuberant, etc.) |
| 5C.05 | `85ad3be` | `AffectEngine` appraisal triggers: `on_task_success`, `on_task_failure`, etc. |
| 5C.06 | `34ad21a` | Affect → behavior modulation table |
| 5C.07 | `b076e96` | Wire affect signatures on episodes: every agent turn tagged with affect state |
| 5C.08 | `6c02846` | Wire affect → `SystemPromptBuilder`: emotional state modifies system prompt |
| 5C.09 | `a9656be` | Wire affect into task prioritization in executor |
| 5C.10 | `aca388e` | Wire motivation decay: tasks in queue >24h lose motivation |
| 5C.11 | `3b4a920` | Wire affect → cascade router: low confidence → prefer cheaper model |
| 5C.12 | `c501feb` | Persist affect state to `.roko/daimon/affect.json` |
| 5C.13 | `ef5c0b1` | Emit affect signals on significant state changes |
| 5C.14 | `1589168` | Wire into dashboard: show affect state per plan/agent |

**5D series — Dreams / Offline Learning (16 commits)**

| Commit | Hash | Subject |
|---|---|---|
| 5D.02 | `19298ef` | Add `roko-dreams` to workspace members |
| 5D.03 | `401280e` | `DreamCycle` struct — the main offline learning process |
| 5D.04 | `cd52d0d` | `DreamCycle::run()`: collect episodes → cluster → extract patterns → generate knowledge |
| 5D.05 | `93721e7` | Auto dream in daemon mode: trigger when no active agents + idle >30min |
| 5D.06 | `99775fb` | Manual dream: `roko dream` CLI command |
| 5D.07 | `4918efc` | `roko dream --report`: show last dream report without running new cycle |
| 5D.08 | `0a5b8cb` | Wire dream-generated knowledge into context assembly |
| 5D.09 | `f8df18d` | Wire dream output into affect engine: failure patterns affect state |
| 5D.10 | `9a75f4b` | G2: Re-evaluate past episodes with current knowledge |
| 5D.11 | `f257a4c` | G3: Mistake identification for failed episodes |
| 5D.12 | `d3edc1a` | G4: Heuristic strengthening/weakening during replay |
| 5D.13 | `bc86272` | G6: Counterfactual simulation via HDC vector permutation |
| 5D.14 | `58a8f1a` | G7: Cross-episode consolidation — meta-patterns across unrelated episodes |
| 5D.15 | `3bd6334` | G8: Novel strategy generation — combine heuristics from different domains |
| 5D.16 | `3b4d0c4` | Regression detection in dream cycle: compare success rates for recurring tasks |
| 5D.17 | `c48459d` | Performance stall detection: if no improvement, emit alert signal |

**5E series — Operating Frequencies (10 commits)**

| Commit | Hash | Subject |
|---|---|---|
| 5E.01 | `16ac033` | Define `OperatingFrequency` enum: Reactive / Deliberative / Extended |
| 5E.02 | `d48bc56` | Frequency selection logic: given Task + AffectState → determine frequency |
| 5E.03 | `3b05e05` | Frequency scheduler: decides which loop to run (from 12a §I4) |
| 5E.04 | `fa07181` | Meta-cognition hook: agent reflects on its own performance (from 12a §I5) |
| 5E.05 | `87f9034` | Frequency → model selection: reactive=no model, deliberative=standard, extended=stronger |
| 5E.06 | `4c01bf3` | Frequency → turn limits: reactive=0, deliberative=standard, extended=more |
| 5E.07 | `a324e50` | Frequency → context budget: reactive=0, deliberative=standard |
| 5E.08 | `ba79df6` | Frequency tagging in task TOML: optional `frequency` field |
| 5E.09 | `2b23cb5` | Frequency metrics: add `frequency` field to `EfficiencyEvent` |
| 5E.10 | `8a4e23c` | Wire into dashboard: show operating frequency per active task |

**5F series — C-Factor / Collective Intelligence (23 commits)**

| Commit | Hash | Subject |
|---|---|---|
| 5F.01 | `a195df7` | Define `CFactor` struct in roko-learn |
| 5F.02 | `83f4a0a` | Implement `compute_cfactor()` |
| 5F.03 | `416efa2` | Persist C-Factor to `.roko/learn/c-factor.jsonl` |
| 5F.04 | `927d85f` | Wire computation: after plan run + on `roko status --cfactor` |
| 5F.05 | `dff6a06` | Wire into cascade router: cfactor > 0.8 → cheaper, < 0.4 → stronger |
| 5F.06 | `92bd7aa` | Wire trend into dashboard: sparkline, trend arrow, breakdown |
| 5F.07 | `3f132fe` | `roko status --cfactor` CLI command |
| 5F.08 | `ad46140` | Regression alert: >20% drop from 7-day average |
| 5F.09 | `3436142` | J1: Information flow rate |
| 5F.10 | `a702c11` | J2: Turn-taking equality (Gini) |
| 5F.11 | `2da6582` | J3: Social sensitivity proxy |
| 5F.12 | `5b4985e` | J4: Knowledge integration rate |
| 5F.13 | `e68c8b0` | J5: Task diversity coverage (mutual information) |
| 5F.14 | `e683e67` | J6: Convergence velocity |
| 5F.15 | `dbdf1f6` | J7: Per-agent c-factor contribution (leave-one-out) |
| 5F.16 | `c26e422` | J8: Per-fleet c-factor |
| 5F.17 | `118b9e7` | J9: C-factor → agent selection routing |
| 5F.18 | `732b9b7` | J10: C-factor metrics endpoint |
| 5F.19 | `8624e37` | `roko-neuro` public API: `NeuroStore` trait |
| 5F.20 | `b2d275a` | `roko-daimon` public API: `DaimonState` |
| 5F.21 | `b036cd1` | `roko-dreams` public API: `DreamRunner` |
| 5F.22 | `86f9907` | `AntiKnowledge` entries |
| 5F.23 | `70f2f1d` | AntiKnowledge confidence reduction (×0.5) |

Missing from branch: 4C.01, 4C.02 (predate branch), 5C.01 (roko-daimon crate creation), 5D.01 (DreamCycle base type).

---

### 47.2 C-Factor Core — Detailed Implementation

**File**: `crates/roko-learn/src/cfactor.rs` (1,356 lines)

#### Structs

**`CFactor`** (lines 15–28): `overall: f64`, `components: CFactorComponents`, `agent_contributions: Vec<AgentCFactorContribution>`, `computed_at: DateTime<Utc>`, `episode_count: usize`. Default: all zeros, empty contributions, `Utc::now()`.

**`CFactorComponents`** (lines 59–88): 11 fields, all `f64`:
`gate_pass_rate`, `cost_efficiency`, `speed`, `information_flow_rate`, `first_try_rate`, `knowledge_growth`, `knowledge_integration_rate`, `task_diversity_coverage`, `convergence_velocity`, `turn_taking_equality`, `social_sensitivity`. Fields marked `#[serde(default)]`: information_flow_rate, knowledge_integration_rate, task_diversity_coverage, convergence_velocity, social_sensitivity.

**`AgentCFactorContribution`** (lines 35–45): `agent_id: String`, `episode_count: usize`, `without_agent_overall: f64`, `contribution_score: f64` (= `overall - without_agent_overall`).

**`AgentDispatchBias`** (lines 48–56): enum `PreferStronger | PreferCheaper | Neutral`. Not serialized.

**`CFactorRegression`** (lines 91–109): `current_snapshot_at`, `window_start`, `window_end`, `sample_count`, `historical_average`, `current`, `drop_fraction`, `threshold`.

**`TaskAggregate`** (lines 188–196, private): `cost_usd`, `duration_ms`, `signal_tokens`, `passed_gate`, `saw_replan`, `first_seen`.

#### `compute_cfactor()` — lines 221–413

```rust
pub fn compute_cfactor(
    episodes: &[Episode], window: Duration,
    social_sensitivity: f64, knowledge_integration_rate: f64, convergence_velocity: f64,
) -> CFactor
```

**Algorithm step-by-step:**

1. **Filter**: episodes where `timestamp >= Utc::now() - window` (default 7 days).
2. **Group by task_key**: `episode.task_id` if non-empty, else `episode.id`. Accumulates `cost_usd`, `duration_ms`, `signal_tokens` (input+output tokens), `passed_gate` (OR across turns — uses `gate_verdicts` if present, else `episode.success`), `saw_replan` (true if `kind="replan"` case-insensitive OR `extra["strategy"]`/`extra["replan_strategy"]`/`extra["attempt_number"]` present), `first_seen` (min timestamp).
3. **Sort** task groups by `first_seen`, then `task_key` as tiebreak.
4. **Baseline**: first `min(task_count, 10)` groups (constant `BASELINE_TASK_COUNT = 10`, line 12).
5. **Compute 11 components**:

| Component | Formula | Notes |
|---|---|---|
| `gate_pass_rate` | `passed_tasks / total_tasks` | |
| `first_try_rate` | `(passed_gate AND !saw_replan) / total_tasks` | |
| `cost_efficiency` | `baseline_avg_cost / avg_cost_per_success` | 0.0 if either is 0 |
| `speed` | `baseline_avg_duration / avg_duration_per_success` | 0.0 if either is 0 |
| `information_flow_rate` | `avg_signal_throughput / baseline_signal_throughput` | throughput = tokens/ms |
| `knowledge_growth` | `total_new_knowledge_entries / episode_count` | reads from episode.extra keys: `new_knowledge_entries`, `knowledge_entries_written`, `knowledge_entries`, `knowledge_written`, `knowledge` (priority order) |
| `task_diversity_coverage` | `MI(template; category) / max(H(template), H(category))` | Normalized mutual information. Uses `episode.agent_template` and `episode.extra["task_category"]` |
| `turn_taking_equality` | `mean(1 - Gini(per_agent_counts))` per plan, clamped [0,1] | Gini formula: `(2·Σ(i·v_i))/(n·Σv_i) - (n+1)/n` |
| `knowledge_integration_rate` | passed in as argument | clamped [0,1] |
| `convergence_velocity` | passed in as argument | clamped [0,1] |
| `social_sensitivity` | passed in as argument | clamped [0,1] |

6. **Weighted composite** (lines 372–383):
```
overall = (
    gate_pass_rate             × 0.23
  + cost_efficiency            × 0.15
  + speed                      × 0.10
  + information_flow_rate      × 0.08
  + first_try_rate             × 0.18
  + knowledge_growth           × 0.08
  + knowledge_integration_rate × 0.07
  + task_diversity_coverage    × 0.11
) × 0.9
  + convergence_velocity       × 0.05
  + turn_taking_equality       × 0.05
  + social_sensitivity         × 0.05
```

**BUG**: Weight sum = `(0.23+0.15+0.10+0.08+0.18+0.08+0.07+0.11)×0.9 + 0.05+0.05+0.05 = 1.00×0.9 + 0.15 = 0.90 + 0.15 = **only 0.96**`, not 1.0. Maximum achievable `overall` is 0.96 when all components = 1.0. Result is `.clamp(0.0, 1.0)`.

7. **Leave-one-out contributions** via `compute_agent_contributions()` (lines 765–819): groups episodes by agent; for each agent, removes its episodes, calls `compute_cfactor_from_filtered()` (identical algo minus window filter and contributions), sets `contribution_score = overall - without_agent_overall`. Sorted descending by score, ascending by agent_id as tiebreak.

#### Other public functions

**`trend_arrow()`** (lines 420–447): Filters history to window, sorts by `computed_at`, compares first vs last `overall`. Returns `"↑"`, `"↓"`, or `"→"`.

**`detect_cfactor_regression()`** (lines 455–507): Filters history to window; `historical_average = mean(all except last)`; `drop_fraction = (avg - current) / avg`; fires if `drop_fraction > threshold` (strict greater-than: exact threshold does NOT fire). **NOT CALLED FROM ANY RUNTIME CODE** — only tested inline.

**`dispatch_bias_for_agent()`** (lines 172–185): Looks up agent's leave-one-out `contribution_score`. Returns `PreferStronger` if score ≤ -0.05, `PreferCheaper` if score ≥ 0.05 AND `overall ≥ 0.65`, else `Neutral`. Thresholds hardcoded.

#### Private helper functions (lines 509–1002)

`ratio()`, `compute_turn_taking_equality()`, `turn_taking_equality_for_counts()`, `compute_task_diversity_coverage()`, `entropy_from_counts()` (Shannon: `-Σ p·log₂(p)`), `gini_coefficient()`, `episode_plan_key()` (fallback: extra["plan_id"] → task_id → id), `episode_agent_key()` (agent_id → agent_template → id), `episode_agent_template()`, `episode_task_category()`, `task_key()`, `episode_duration_ms()`, `episode_signal_tokens()`, `signal_throughput()`, `episode_passed_gate()`, `episode_is_replan()`, `episode_new_knowledge_entries()`, `compute_agent_contributions()`, `compute_cfactor_from_filtered()`, `knowledge_entry_count()`.

#### Test coverage

| Function | Tested | Notes |
|---|---|---|
| `compute_cfactor` | 10 tests | Missing: exact `overall` float; `info_flow_rate > 1.0` behavior; `task_key` fallback to `episode.id` |
| `dispatch_bias_for_agent` | 2 tests | Missing: `Neutral` when score between -0.05 and +0.05; positive score but `overall < 0.65` |
| `trend_arrow` | 1 test (↑ only) | Missing: `"↓"` case, `"→"` case, empty history |
| `detect_cfactor_regression` | 2 tests | Good boundary coverage |
| `compute_agent_contributions` | 1 test (indirect) | Via leave-one-out test |
| `gini_coefficient` | 0 dedicated | Covered transitively via turn-taking tests |
| `entropy_from_counts` | 0 dedicated | Covered transitively via diversity tests |
| `knowledge_entry_count` | 0 dedicated | Array/Number paths covered; Bool/Object/_ NOT exercised |

---

### 47.3 C-Factor Wiring Chain — Full Call Graph

```
orchestrate.rs:2962 (after run_task_plans completes)
  → refresh_cfactor_snapshot()             [runtime_feedback.rs:988]
    → compute_cfactor_snapshot()           [runtime_feedback.rs:1008]
      reads: .roko/learn/episodes.jsonl    (via EpisodeLogger::read_all_lossy)
      reads: .roko/context-attribution.jsonl  (for social_sensitivity)
      reads: .roko/neuro/knowledge.jsonl   (for knowledge_integration_rate + convergence_velocity)
      → social_sensitivity_from_attribution()   [runtime_feedback.rs:1068]
      → knowledge_integration_rate()            [runtime_feedback.rs:1131]
      → convergence_velocity_from_agreement()   [runtime_feedback.rs:1195]
      → compute_cfactor()                       [cfactor.rs:222]
    writes: .roko/learn/c-factor.jsonl     (append one JSON line)

orchestrate.rs:6691 (per-task dispatch)
  → self.learning.latest_cfactor()         [runtime_feedback.rs:507]
    reads: last line of .roko/learn/c-factor.jsonl
    → cascade_router.select_for_frequency(frequency, ctx, cfactor, agent_id)
      → route_with_cfactor()               [cascade_router.rs:371]
        → bias_model_for_cfactor()         [cascade_router.rs:625]
          uses: HIGH_CFACTOR_THRESHOLD=0.8 [cascade_router.rs:96]
          uses: LOW_CFACTOR_THRESHOLD=0.4  [cascade_router.rs:98]
          → cfactor.dispatch_bias_for_agent(agent_id)  [cfactor.rs:173]

orchestrate.rs:2887 (PlanRunner::summary)
  → compute_fleet_cfactor(&self.efficiency_events)  [efficiency.rs:561]
    (separate from episode-based CFactor; uses in-memory AgentEfficiencyEvent data only)

main.rs:2646 (cmd_status --cfactor)
  → refresh_cfactor_snapshot(learn_root)
  → load_cfactor_history(cfactor_path)     [main.rs:1349]
    reads: all lines from .roko/learn/c-factor.jsonl
  → cfactor_trend_arrow(&history, 7d)      [cfactor.rs:420]
  → prints 10 component fields             [main.rs:2808-2820]

dashboard.rs:471-512 (DashboardData init)
  → load_cfactor_history(learn_dir/"c-factor.jsonl")  [dashboard.rs:2144]
  → hot-reloads on file change via FileStamp          [dashboard.rs:636-641]
```

---

### 47.4 NeuroStore / KnowledgeStore — Detailed Implementation

**Trait**: `crates/roko-neuro/src/lib.rs`, lines 143–158.
**Impl**: `crates/roko-neuro/src/knowledge_store.rs`.

#### `KnowledgeEntry` fields (lib.rs:72–113)

| Field | Type | Default | Notes |
|---|---|---|---|
| `id` | `String` | `""` | |
| `kind` | `KnowledgeKind` | `Fact` | Variants: Fact(365d), Insight(30d), Procedure(30d), Heuristic(90d), Playbook(30d), Constraint(30d), AntiKnowledge(30d) |
| `source` | `Option<String>` | `None` | |
| `content` | `String` | `""` | |
| `confidence` | `f64` | `1.0` | Halved by anti-knowledge |
| `confidence_weight` | `f64` | `1.0` | Set to `-confidence` for AntiKnowledge in DreamCycle |
| `refuted_insight_id` | `Option<String>` | `None` | Targets entry whose `id` matches |
| `refutation_evidence` | `Option<String>` | `None` | Human-readable explanation |
| `source_episodes` | `Vec<String>` | `[]` | Episode IDs that contributed; ≥2 triggers confirmation boost |
| `tags` | `Vec<String>` | `[]` | |
| `created_at` | `DateTime<Utc>` | `Utc::now()` | |
| `half_life_days` | `f64` | `30.0` | Overridden by `KnowledgeKind::default_half_life_days()` |
| `hdc_vector` | `Option<Vec<u8>>` | `None` | Feature-gated; never populated by any writer |

#### `query()` scoring (knowledge_store.rs:194–225)

```
score = keyword_score(entry, topic_terms, topic_norm)
      × effective_confidence(entry)
      × recency_factor(entry, now)
    [+ hdc_similarity(entry, topic)]  // only with `hdc` feature
```

- **keyword_score** (lines 556–584): substring match of full topic (+1.0) + per-tag/per-word overlap (+1.0 each) over `content` and `tags`.
- **effective_confidence** (lines 604–614): `confidence.clamp(0.0, 1.0) × boost` where `boost = 1.5` if `source_episodes.len() >= 2`, else `1.0`. Constant: `CONFIRMATION_BOOST = 1.5` (line 24).
- **recency_factor** (lines 586–594): `0.5^(age_days / half_life_days)` — standard exponential decay.
- Sort: score desc → effective_confidence desc → created_at desc.

#### Anti-knowledge path in `ingest()` (knowledge_store.rs:118–182)

1. Check if any incoming entry has `kind == AntiKnowledge` AND non-empty `refuted_insight_id`.
2. If yes: read entire JSONL into memory (line 144), append new entries (line 145), for each anti-knowledge entry find original by `id == refuted_insight_id` and do `original.confidence *= 0.5` (line 160). Atomic rewrite via `.jsonl.tmp` + `fs::rename`.
3. If no anti-knowledge in batch: just append with `append=true`, no read required.

#### `decay()` (lines 284–297)

Re-reads all entries, applies `recency_factor()` as a **multiplier against current confidence** (NOT a recalculation from `created_at`). Each call further degrades. Returns total entries processed (NOT entries changed).

#### `gc()` (lines 304–316)

Filters to `effective_confidence(entry) >= threshold`. Note: uses `effective_confidence` with 1.5x confirmation boost, so entry with raw 0.04 and 2+ source_episodes survives 0.05 threshold (0.04 × 1.5 = 0.06).

#### Storage: `PathBuf` + `Arc<Mutex<()>>` write gate (lines 38–41). **No in-memory cache** — every query re-reads the JSONL file.

#### Wiring in orchestrate.rs

- **Constructed** at lines 1841–1845, 1958–1963, 2073–2078: `KnowledgeStore::init(workdir.join(".roko/neuro/knowledge.jsonl"))`.
- **Queried** at lines 4269–4290 (`build_knowledge_context`): `NeuroStore::query(&self.knowledge_store, task_text, limit)` where limit = 4 (focused), 5 (integrative), or 6 (architectural). Results rendered into `## Durable Knowledge` markdown section. Anti-knowledge entries get `Warning:` prefix via `refutation_warning()` at line 8378.
- **NEVER WRITTEN TO** during plan execution. `ingest()`, `decay()`, `gc()` are never called from orchestrate.rs. Population only happens via `roko dream run` (DreamCycle) or the standalone Distiller.

---

### 47.5 DaimonState / AffectEngine — Detailed Implementation

**File**: `crates/roko-daimon/src/lib.rs`

#### Structs

**`DaimonState`** (lines 232–241): `state: AffectState`, `half_life_hours: f64` (default 4.0), `persistence_path: Option<PathBuf>` (serde-skipped).

**`AffectState`** (lines 51–68): `pad: PadVector { pleasure, arousal, dominance }` (all [-1.0, 1.0]), `confidence: f64` ([0.0, 1.0], default 0.5), `updated_at: DateTime<Utc>`.

#### Decay formula (lines 80–92)

```
factor = 0.5^(elapsed_hours / half_life_hours)    // half_life = 4 hours
pad.pleasure *= factor
pad.arousal *= factor
pad.dominance *= factor
confidence = 0.5 + (confidence - 0.5) × factor    // decays toward 0.5, not 0.0
```

#### Appraisal event handlers (lines 314–387)

| Event | pleasure | arousal | dominance | confidence | Notes |
|---|---|---|---|---|---|
| `GateResult { passed: true, rung }` | `+0.05×rs` | `-0.01×rs` | `+0.03×rs` | `+0.03×rs` | `rs = 1 + min(rung,3)×0.15` |
| `GateResult { passed: false, rung }` | `-0.10×rs` | `+0.04×rs` | `-0.08×rs` | `-0.08×rs` | |
| `TaskOutcome { succeeded: true }` | `+0.10` | `0.00` | `+0.10` | `+0.08` | |
| `TaskOutcome { succeeded: false }` | `-0.20` | `0.00` | `-0.15` | `-0.15` | |
| `Blocked { blocker_count }` | `0.0` | `+n×0.05` | `-n×0.08` | `-0.02×n` | n = clamp(count, 1, 5) |
| `TimePressure { deadline_proximity }` | `0.0` | `+p×0.40` | `0.0` | `0.0` | |
| `QueueWait { wait_hours }` | `0.0` | `+bump` | `0.0` | `0.0` | bump=0 if ≤24h, +0.1/day, saturates at 1.0 after 7 days |
| `DreamFailure { failure_count }` | `0.0` | `0.0` | `0.0` | `-(0.07×n).min(0.35)` | n = clamp(count, 1, 5) |

Every appraise call: decay to now → apply delta → autosave.

#### `modulate()` dispatch logic (lines 393–415)

| Condition | Strategy | turn_limit | model |
|---|---|---|---|
| `confidence < 0.30 OR dominance < -0.25` | `Escalating` | `+10` | promote (haiku→sonnet→opus) |
| `pleasure > 0.35 AND confidence > 0.65` | `Exploratory` | `-5` | demote |
| `pleasure < -0.30 AND arousal > 0.30` | `Conservative` | `-3` | demote |
| `arousal < -0.20` | `Proactive` | `+5` | unchanged |
| else | `Balanced` | unchanged | unchanged |

#### Wiring in orchestrate.rs

- **Constructed**: lines 1831, 1948, 2063 — `DaimonState::load_or_new(workdir/.roko/daimon/affect.json)`.
- **Appraise call**: **ONLY ONE SITE** — line 3110: `self.daimon.appraise(AffectEvent::GateResult { ... })` inside gate evaluation loop per rung.
- **Modulate call**: line 6789: `self.daimon.modulate(&mut dispatch_params)` per agent launch.
- **Persist call**: lines 2479–2484 during `take_snapshot()`.

**GAP**: `AffectEvent::TaskOutcome` is **never called** from orchestrate.rs. Success/failure don't move PAD state; only gate results do. `Blocked`, `TimePressure`, `QueueWait`, `DreamFailure` are also never called.

**NOTE**: `roko-golem::daimon::AffectEngine` (richer per-task-keyed version with octant labels, behavior modulation table, signal emission) is a DIFFERENT struct. It's used only by `roko-serve/src/dispatch.rs` and `roko-learn/src/runtime_feedback.rs`, NOT by the CLI plan-runner.

---

### 47.6 DreamRunner — Detailed Implementation

**File**: `crates/roko-dreams/src/runner.rs`

**`DreamRunner`** (lines 101–104): `workdir: PathBuf`, `config: DreamLoopConfig`.

#### `replay_insights()` (lines 120–124)

Calls `NeuroTierProgression::default().analyze(episodes)` — runs trigram PatternMiner over action streams (`trigger:X`, `agent:X`, `gate:X:pass/fail`, `outcome:success/failure`), finds patterns with `min_support=3`, extracts `InsightRecord` for patterns with `support_count >= 3 AND confidence >= 0.7`, promotes `HeuristicRule` for insights with ≥5 source episodes. **Does NOT write to disk.** Pure in-memory computation.

#### `consolidate_async()` (lines 146–156)

Full `DreamCycle` (in `roko-dreams/src/cycle.rs`): reads all episodes → clusters by `(plan_id, task_type, model, outcome)` → distills clusters into `KnowledgeEntry` records → promotes success clusters to `PlaybookStore` → generates regression/strategy hypothesis entries → writes `DreamCycleReport` to `.roko/dreams/dream-{timestamp_ms}.json` → writes knowledge entries to `KnowledgeStore` via `ingest()`.

**This is the ONLY code path that writes to `KnowledgeStore` during normal operation.**

#### `schedule()` (lines 200–231)

1. If `!auto_dream` → `None`.
2. Read all episodes from `.roko/episodes.jsonl`.
3. Find latest dream report from `.roko/dreams/dream-*.json` (most recent by filename timestamp).
4. `cutoff = report.processed_through OR report.started_at`.
5. Filter episodes newer than cutoff. If count < `min_episodes_for_dream` (default 5) → `None`.
6. `target_fire_at = latest_episode_timestamp + idle_threshold_mins` (default 15).
7. If `target_fire_at <= now` → `Some(Duration::ZERO)`. Else → `Some(remaining)`.

Default config (lines 161–179): `auto_dream: true`, `idle_threshold_mins: 15`, `min_episodes_for_dream: 5`, agent command `"cat"` (no-op), `timeout_ms: 120_000`.

#### Wiring

Constructed in `build_dream_runner()` (main.rs:2952). Used in `cmd_dream()` (main.rs:2868) for `roko dream run/report/schedule`. **NOT referenced in orchestrate.rs at all.** Plan-runner does not auto-trigger dreams.

---

### 47.7 AntiKnowledge Pipeline — Full Path Analysis

#### Step 1: Creation in DreamCycle (cycle.rs:~1457–1505)

`build_regression_entry()`:
- All-failure clusters → `KnowledgeKind::Constraint`
- Mixed success/failure clusters → `KnowledgeKind::AntiKnowledge`
- `refuted_insight_id = format!("insight:{}:{}:{}", cluster.key.plan_id, cluster.key.task_type, cluster.key.model)`
- `confidence_weight = -confidence` (negative weight)

#### Step 2: Ingestion in KnowledgeStore (knowledge_store.rs:129–166)

On `ingest()` with any `AntiKnowledge` entry:
1. Read ALL existing entries
2. Find entry with `id == refuted_insight_id`
3. `original.confidence *= 0.5` (one-shot halving)
4. Atomic rewrite

#### Step 3: Retrieval in orchestrate.rs (lines 4269–4290, 8360–8399)

`build_knowledge_context()` → `render_knowledge_context()` → `refutation_warning()` prepends `"Warning: Previous insight {id} was wrong because {evidence}."` into agent prompt.

#### **CRITICAL GAP: ID format mismatch**

DreamCycle creates `refuted_insight_id = "insight:{plan_id}:{task_type}:{model}"` (synthetic key).
Distiller creates entries with `id = derive_knowledge_id(kind, content, source_episodes, tags)` (content hash).
**These ID formats never match.** The anti-knowledge confidence halving targets non-existent IDs. The mechanism is structurally correct but the IDs are disjoint — no existing entry will ever be found by the `id == refuted_insight_id` lookup.

#### Other gaps

- Distiller does NOT generate AntiKnowledge. Its prompt only requests `fact`, `procedure`, `heuristic`, `constraint`.
- No auto-generation from gate failures in the plan-runner. Only `roko dream run` creates anti-knowledge.
- `ingest()` is never called from orchestrate.rs — only from DreamCycle.

---

### 47.8 Deploy Commands — Detailed Implementation

#### `roko deploy railway` (main.rs:3006–3063)

1. `run_release_build()` → `cargo build --release -p roko-cli` via `std::process::Command`
2. Reads `roko.toml`: `config.deploy.railway_api_token`, `project_id`, `environment_id`, `default_region`
3. `git_remote_slug()` → `git remote get-url origin` → derives `owner/repo`
4. `git_current_branch()` → `git branch --show-current`
5. `collect_railway_env_vars()` → harvests: `GITHUB_TOKEN`, `GH_TOKEN`, `SLACK_TOKEN`, `SLACK_BOT_TOKEN`, `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `ROKO_SERVER_AUTH_TOKEN`
6. `RailwayApiBackend::deploy_roko_app()` — calls Railway GraphQL v2 (`https://backboard.railway.com/graphql/v2`): `projectCreate/Update`, `serviceCreate/Update/Connect`, `serviceInstanceUpdate`, `variableCollectionUpsert`, `volumeCreate`, `serviceInstanceDeployV2`. Has `wait_for_ready()` polling (5s→60s backoff, 15-minute deadline).
7. `register_deployment_github_webhooks()` via octocrab: push, pull_request, issues, issue_comment, pull_request_review, check_run events. Checks for duplicates. Requires `GITHUB_TOKEN` + `webhooks.github.secret`.
**Status: Fully wired.** Hardcoded: `dockerfile_path: "docker/roko.Dockerfile"`, `healthcheck_path: "/api/health"`, `volume_mount_path: "/workspace/.roko"`.

#### `roko deploy fly` (main.rs:2982–2989)

1. `write_fly_toml()` → writes static `FLY_TOML_TEMPLATE` to workdir. Hardcoded: app name `"roko-agent"`, region `"iad"`, internal port 3000, healthcheck `/api/health` every 30s, volume `/data/.roko`.
2. `flyctl deploy --remote-only` via `run_command_status()`.
**Status: Minimal.** No config reading. No token handling (assumes `flyctl` pre-authenticated). No post-deploy webhook registration.

#### `roko deploy docker` (main.rs:2991–3004)

1. Resolves registry: `--registry` flag OR `config.deploy.worker_image` from roko.toml
2. `docker build -t roko .`
3. `docker tag roko:latest {registry}/roko:latest`
**GAP: No `docker push`.** Builds and tags but never pushes. No login step. Returns success anyway.

#### Cloud execution flow (worker/cloud.rs:438)

`run_code_implementer_cloud()`: `git clone --depth 1` → `git checkout -b impl/{plan_slug}` → load `roko.toml` → build `PlanRunner` with `enable_cloud_execution()` → `run_task_plans()` → `git add -A && git commit` → `git push` (rewrites remote URL with token) → `github_create_pr()` via MCP stdio client → `git_cleanup()`.
**Status: Fully wired.** Token scrubbing in error messages via `scrub_token()`.

#### Health check (roko-serve/src/routes/status.rs:47–59)

`GET /api/health` → `{"status": "ok", "version": "<CARGO_PKG_VERSION>", "uptime_secs": n}`. Reads from `AppState.started_at`.

#### `--cloud` flag (main.rs:163–169, config.rs:99–135)

When `cloud=true`: `log_format="json"`, `bind="0.0.0.0"`, `data_dir="/data/.roko"`, appends `[[serve.deploy.webhooks]]` entries for `nunchi/roko` and `nunchi/collaboration`.

#### Unused deploy backends

- `railway_cli.rs` — shells out to `railway` CLI. Available via `create_backend("railway-cli")` but never called from any CLI command.
- `manual.rs` — writes Dockerfile + .env + README to `.roko/deploy-bundles/{name}/`. Available via `create_backend("manual")`.

---

### 47.9 roko-golem Scaffolds — All Dead Code

All six scaffold engines in `roko-golem/src/`, behind `scaffold` feature flag. Zero-sized `#[derive(Default, Clone, Copy)]` structs with `const fn new()`, `const fn summary() -> GolemSubsystemSummary`, and one stub method returning a static marker string.

| Engine | File | Stub method | Return value |
|---|---|---|---|
| `DreamsEngine` | `dreams.rs:8–42` | `const fn replay(self)` | `"roko-golem scaffold: dreams"` |
| `GrimoireEngine` | `grimoire.rs:9–43` | `const fn evolve(self)` | `"roko-golem scaffold: grimoire"` |
| `HypnagogiaEngine` | `hypnagogia.rs:8–42` | `const fn interrupt(self)` | `"roko-golem scaffold: hypnagogia"` |
| `DaimonEngine` | `daimon.rs:707–741` | `const fn evaluate(self)` | `"roko-golem scaffold: daimon"` |
| `MortalityEngine` | `mortality.rs:9–43` | `const fn pulse(self)` | `"roko-golem scaffold: mortality"` |
| `ChainWitnessEngine` | `chain_witness.rs:8–42` | `const fn observe(self)` | `"roko-golem scaffold: chain_witness"` |

`GolemScaffold` aggregator (lib.rs:144–197) holds all six, exposes `summaries() -> [GolemSubsystemSummary; 6]`. Referenced from nowhere in the runtime.

**NOTE**: `roko-golem::daimon::AffectEngine` (same file as `DaimonEngine`, lines 127–499) IS fully implemented with per-task PAD, octant mapping, behavior modulation table, and signal emission. But it's wired to `roko-serve` and `roko-learn`, NOT to the CLI plan-runner.

---

### 47.10 All Gaps — Complete Enumeration

| # | Gap | Severity | Location (consumer) | Location (missing producer) | Detail | Fix |
|---|---|---|---|---|---|---|
| 47a | **J3 social sensitivity always 0.0** | Medium | `runtime_feedback.rs:1068` `social_sensitivity_from_attribution()` | `orchestrate.rs:7182–7200` writes only when tasks have `depends_on` items | Reads `context-attribution.jsonl` with `source_type == "prior_output"` and `referenced == true`. Only populated when tasks have `depends_on` dependencies AND prior output was loaded (lines 6819–6841). Tasks without dependencies → file stays empty → metric = 0.0. Weight: 5% of overall. | Wire attribution records for ALL tasks that reference any upstream context, not just `depends_on` items |
| 47b | **J4 knowledge integration rate always 0.0** | Medium | `runtime_feedback.rs:1131` `knowledge_integration_rate()` | No writer for `KnowledgeConfirmationRecord` with `source_episodes.len() >= 2` | Reads from `.roko/neuro/knowledge.jsonl` looking for entries with 2+ `source_episodes`. `KnowledgeStore::ingest()` is never called from orchestrate.rs. Only DreamCycle writes to it, and only on manual `roko dream run`. Weight: 7% of overall. | Wire `KnowledgeStore::ingest()` in orchestrate.rs after task completion, or auto-trigger dream cycle |
| 47c | **J6 convergence velocity always 0.0** | Medium | `runtime_feedback.rs:1195` `convergence_velocity_from_agreement()` | Same as 47b | Same data dependency as J4 — requires multi-agent `KnowledgeConfirmationRecord` chains. Weight: 5% of overall. | Fixed by 47b |
| 47d | **J10 C-Factor metrics endpoint missing** | Low | N/A | `roko-serve/src/routes/learning.rs` has efficiency/cascade/experiments/adaptive-thresholds but no cfactor | No `/api/metrics/c_factor` or `/api/cfactor` HTTP route. Dashboard reads JSONL directly. | Add route reading `.roko/learn/c-factor.jsonl` last line + history |
| 47e | **`detect_cfactor_regression()` never called** | Medium | `cfactor.rs:455` defined and tested | No call site in orchestrate.rs, main.rs, or dashboard.rs | The regression alert (5F.08 commit) defined the function but never wired it into any runtime path. The "emit `cfactor:regression` signal" from the master plan is not implemented. | Call from `refresh_cfactor_snapshot()` in runtime_feedback.rs, emit signal on regression |
| 47f | **C-Factor stale-by-one-run** | Low | `orchestrate.rs:6691` reads `latest_cfactor()` | `orchestrate.rs:2962` calls `refresh_cfactor_snapshot()` only AFTER run completes | Every task dispatched during run N reads the cfactor from the END of run N-1. The snapshot used for model routing is always one full run behind. | Call `refresh_cfactor_snapshot()` periodically during the run, not just at completion |
| 47g | **`FleetCFactor` and episode-based `CFactor` are disconnected** | Low | `efficiency.rs:561` `compute_fleet_cfactor()` vs `cfactor.rs:222` `compute_cfactor()` | No merge logic | FleetCFactor (from in-memory efficiency events for current session) is printed in the report but NOT persisted to `c-factor.jsonl` and NOT read by `latest_cfactor()`. The cascade router only sees episode-based CFactor. | Decide whether to merge or keep separate; at minimum document the distinction |
| 47h | **c-factor.jsonl grows O(N) per run** | Low | `runtime_feedback.rs:637` `persist_completed_run()` calls `append_cfactor_snapshot()` | Also appended at run end via `refresh_cfactor_snapshot()` | Every episode completion triggers a full recompute + append. A run with 50 tasks produces ~50 intermediate snapshots. Dashboard and `latest_cfactor()` only read last line, so correctness is OK, but file grows unboundedly. | Append only at run boundaries, or compact the JSONL periodically |
| 47i | **Docker deploy missing `docker push`** | Low | `main.rs:2991–3004` | After `docker tag`, no push step | `docker build -t roko .` + `docker tag roko:latest {registry}/roko:latest` — but no `docker push {registry}/roko:latest`. Returns success without pushing. | Add `run_command_status(&workdir, "docker", &["push", &tagged_image])` |
| 47j | **AntiKnowledge ID format mismatch** | High | `cycle.rs:~1463` creates `refuted_insight_id = "insight:{plan_id}:{task_type}:{model}"` | Distiller creates IDs via `derive_knowledge_id(kind, content, source_episodes, tags)` (content hash) | The synthetic key from DreamCycle will never match any existing entry created by the Distiller. The `confidence *= 0.5` lookup in `ingest()` silently finds nothing. Anti-knowledge is structurally correct but ID formats are disjoint. | Standardize ID format: either DreamCycle uses content-hash IDs, or add a secondary index by `(plan_id, task_type, model)` tuple |
| 47k | **AntiKnowledge auto-generation not wired** | Medium | N/A — no auto-generation code path | orchestrate.rs | Only `roko dream run` (manual CLI command) creates anti-knowledge. Plan-runner does not auto-generate from repeated gate failures. | Add post-gate-failure logic in orchestrate.rs to call `KnowledgeStore::ingest()` with `AntiKnowledge` entries, or auto-trigger dream cycle |
| 47l | **`KnowledgeStore::decay()` and `gc()` never scheduled** | Medium | `knowledge_store.rs:284, 304` defined | No caller anywhere in runtime | No periodic maintenance. Knowledge store grows without bound. Stale entries never pruned. | Schedule in PlanRunner::finish() or as a periodic background task |
| 47m | **Daimon only appraises GateResult** | Medium | `orchestrate.rs:3110` — sole `appraise()` call | orchestrate.rs | `TaskOutcome` (success/failure), `Blocked`, `TimePressure`, `QueueWait`, `DreamFailure` events are defined but never fired from the plan-runner. PAD state only moves on gate pass/fail. | Add `appraise(TaskOutcome { succeeded })` after task completion; add `appraise(Blocked { ... })` when task is dependency-blocked |
| 47n | **DreamRunner not auto-triggered** | Medium | `main.rs:2868` — manual `roko dream` only | orchestrate.rs | `DreamRunner::schedule()` computes when next dream should fire, but nobody calls it in the plan-runner loop. Dreams only run via manual CLI. | Wire `schedule()` check in PlanRunner, auto-trigger `consolidate_async()` when idle |
| 47o | **roko-golem scaffolds are dead code** | Info | 6 scaffold engines in `roko-golem/src/` | N/A | All return static marker strings. Real implementations live in roko-dreams, roko-neuro, roko-daimon. Golem versions are never referenced from any runtime path. | Delete scaffolds or redirect to real impls |
| 47p | **Weight sum bug in `compute_cfactor`** | Low | `cfactor.rs:372–383` | N/A | Weights sum to 0.96, not 1.0. Maximum achievable `overall` is 0.96 when all components = 1.0. The 0.9 scaling factor on the first 8 terms combined with 3 additive 0.05 terms creates a 4% shortfall. | Adjust weights to sum to 1.0, or document the 0.96 ceiling as intentional |
| 47q | **Fly deploy fully hardcoded** | Low | `main.rs:2982–2989`, `FLY_TOML_TEMPLATE` | No config reading | App name `"roko-agent"`, region `"iad"`, port 3000 all hardcoded in template string. No `roko.toml` deploy config is read. | Read from `config.deploy.fly.*` fields |

### Root cause patterns

1. **Consumer without producer**: J3, J4, J6, anti-knowledge auto-generation, knowledge store maintenance. Codex builds the computation but doesn't verify that the data it reads is written by anything upstream.

2. **Single call site when multiple are needed**: Daimon only appraises `GateResult` when 7 event types exist. Only one of the two CFactor systems (episode-based) feeds the router; the other (fleet) is display-only.

3. **ID format mismatch**: AntiKnowledge creates synthetic IDs that don't match the content-hash IDs used by the distiller. The mechanism is correct but the key spaces are disjoint.

4. **Manual when should be automatic**: DreamRunner has a `schedule()` function but nobody calls it. Knowledge store has `decay()`/`gc()` but nobody schedules them. `detect_cfactor_regression()` exists but is never called.

5. **Stale data in hot paths**: CFactor used for model routing is always from the previous run. The snapshot is refreshed only at run end, not during.
