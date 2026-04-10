# Roko Multi-Agent Architecture

## Agent Roles (28, same as mori)

Defined in `crates/roko-core/src/agent.rs`:

```rust
pub enum AgentRole {
    Conductor, Strategist, Implementer, Architect, Researcher,
    Auditor, QuickReviewer, Scribe, Critic, AutoFixer,
    Refactorer, PrePlanner, DocVerifier, IntegrationTester,
    MergeResolver, TerminalValidator, GolemLifecycleTester,
    SpecDriftDetector, RegressionDetector, PerformanceSentinel,
    CoverageTracker, PlanLifecycleManager, CrossSystemTester,
    ErrorDiagnoser, DependencyValidator, PatternExtractor,
    SnapshotComparator, FullLoopValidator,
}
```

## Role Manifests (Formal Definitions)

Six core roles have formal TOML manifests in `crates/roko-core/src/builtin_roles/core_roles.toml`:

**Strategist**, **Implementer**, **Architect**, **Auditor**, **Quick-Reviewer**, **Scribe**

Each manifest specifies:
- `objectives` and `responsibilities`
- `context_policy` with `budget_tokens` and `bidders` lists
- `tools.capabilities` (e.g., implementer: `["filesystem.read", "filesystem.write", "process.exec"]`; architect: `["filesystem.read"]`)
- `output_schema` (required format)
- `gate_expectations` (which gates must pass)
- `safety.bounds` (constraints like "Do not edit source files while acting as strategist")
- `prompt_policy` with ordered sections

## Provider Kinds

```rust
pub enum ProviderKind {
    AnthropicApi,    // Anthropic Messages API over HTTP
    ClaudeCli,       // `claude` CLI subprocess
    OpenAiCompat,    // OpenAI-compat HTTP APIs (GPT, GLM, Moonshot, Gemini, Ollama)
    CursorAcp,       // Cursor Agent Client Protocol
    PerplexityApi,   // Perplexity Sonar HTTP API
    GeminiApi,       // Google Gemini API
}
```

vs mori's 3 backends (Claude, Codex, Cursor), roko has 6 provider kinds.

## Agent Trait

```rust
pub trait Agent: Send + Sync {
    async fn run(&self, input: &Engram, ctx: &Context) -> AgentResult;
    fn name(&self) -> &str;
    fn backend_id(&self) -> &'static str { "unknown" }
    fn supports_streaming(&self) -> bool { false }
}
```

## Agent Factory

`create_agent_for_model()` in `crates/roko-agent/src/provider/mod.rs`:
1. Check for `ROKO_DISPATCHER=mock-*` env var
2. Resolve model via `resolve_model(config, model_key)` -- looks up `[models.*]` in config
3. Load provider config
4. Fall back to known-protocol CLI synthesis (e.g., `claude` -> `ClaudeCli`)
5. Fall back to `ExecAgent` (raw subprocess) if nothing matches
6. Call `adapter_for_kind(provider_config.kind)` to get provider adapter
7. Wrap with safety layer and temperament

## Spawn Spec

```rust
pub struct SpawnAgentSpec {
    pub model: String,
    pub command: Option<String>,
    pub system_prompt: Option<String>,
    pub tools: Option<String>,
    pub mcp_config: Option<PathBuf>,
    pub working_dir: Option<PathBuf>,
    pub env: Vec<(String, String)>,
    pub extra_args: Vec<String>,
    pub effort: Option<String>,
    pub bare_mode: bool,
    pub dangerously_skip_permissions: bool,
    pub name: String,
    pub role: Option<String>,
}
```

## Plan Execution System (3 Layers)

### Layer 1: Plan Discovery

`crates/roko-orchestrator/src/plan_discovery.rs` -- discovers plans from directory.

Frontmatter:
```yaml
---
plan: "09-chain-layer"
depends_on: [01-a, 02-b]
parallel_with: [03-c]
crates_touched: ["crates/roko-core/"]
estimated_tasks: 5
estimated_parallel_width: 3
estimated_minutes: 60
priority: 10
parallel_safe: true
---
```

### Layer 2: Task DAG

`crates/roko-orchestrator/src/dag.rs` -- `UnifiedTaskDag`

Edges from:
1. Intra-plan `depends_on`
2. Cross-plan `depends_on` (e.g., `"09-foo:t3"`)
3. Plan-level dependencies
4. File-overlap inference (opt-in via `DagConfig::infer_file_overlap`)

Tasks layered into **waves** via BFS. Wave 0 = no deps, Wave 1 = depends on wave 0 only, etc.

### Layer 3: Parallel Executor

`crates/roko-orchestrator/src/executor/mod.rs` -- pure state machine

```rust
pub struct ParallelExecutor {
    config: ExecutorConfig,
    plans: HashMap<String, PlanState>,
    queue: Vec<String>,
    plan_deps: HashMap<String, Vec<String>>,
    speculative_executions: HashMap<String, SpeculativeExecution>,
    audit_chain: Option<AuditChain>,
}
```

Defaults:
- `max_concurrent_plans`: 4
- `max_concurrent_tasks`: 8
- `task_timeout_secs`: 600
- `speculative_threshold_multiplier`: 2.0

## Plan Phases (Per-Plan State Machine)

```
Queued -> Enriching (Strategist) -> Implementing -> Gating
  -> AutoFixing (on failure) -> Gating (retry)
  -> Verifying -> Reviewing -> DocRevision -> Merging -> Complete
```

With failure paths:
- Gate fail -> AutoFix (up to 5 iterations) -> Gating retry
- Gate fail exhausted -> Failed
- Review reject -> back to Implementing
- Review approve -> DocRevision -> Merge -> Complete

## The orchestrate.rs Main Loop

`crates/roko-cli/src/orchestrate.rs` (800KB+):

### Initialization
- Load config, FileSubstrate, EpisodeLogger, ProcessSupervisor, CancelToken
- Discover plans, parse tasks.toml
- Build UnifiedTaskDag
- Create ParallelExecutor
- Restore from snapshot if `--resume`

### Tick Loop
```
loop {
    actions = executor.tick()
    for action in actions {
        match action {
            DispatchPlan { plan_id }      => start_plan(plan_id)
            SpawnAgent { plan_id, task }  => dispatch_agent_with(...)
            RunGate { plan_id, rung }     => run_gate_pipeline(...)
            RunVerify { plan_id }         => run_verify_steps(...)
            MergeBranch { plan_id }       => git_merge(...)
            StartSpeculativeExec ..       => spawn_backup_agent(...)
        }
    }
    executor.apply_event(plan_id, event)
    autosave_snapshot_if_needed()
}
```

### dispatch_agent_with() (core dispatch function)

1. **Budget check** -- task/plan budget available
2. **Load task** from tasks.toml
3. **Resolve domain** -- determines git ops needed
4. **Build prompt** -- from TaskDef.build_prompt() (surgical) or generic fallback
5. **Model routing** -- CascadeRouter with adaptive selection based on:
   - Task complexity
   - Budget pressure
   - Conductor load
   - Efficiency history
   - Routing log calibration
6. **Build system prompt** (9 layers):
   - Layer 1: Role identity from template
   - Layer 2: Conventions from config
   - Layer 3: Domain context from neuro store / code intelligence
   - Layer 3c: Active signals / pheromones
   - Layer 4: Task context (prior outputs, file contents, deps)
   - Layer 4b: Gate feedback (if retry)
   - Layer 5: Tool instructions
   - Layer 6: Learned playbooks and skills
   - Layer 7: Anti-patterns from error pattern store
   - Layer 8: Affect guidance from daimon state
7. **Create agent** via spawn_agent_with_layer()
8. **Invoke** -- agent.run(input, ctx)
9. **Record episode** -- .roko/episodes.jsonl
10. **Publish efficiency event** -- cost, tokens, duration
11. **Update daimon state** -- affect engine observes outcome

## Inter-Agent Communication

### Signal System (Engrams)
Core data unit: `Engram` -- typed, content-hashed signal in `.roko/signals.jsonl`.
Each has Kind, Body, Provenance, lineage (parent hashes), tags.

### Episode Logging
Every agent turn -> Episode record in `.roko/episodes.jsonl`:
```rust
pub struct Episode {
    pub kind: String,              // "agent_turn", "gate", "replan"
    pub agent_id: String,
    pub task_id: String,
    pub model: String,
    pub backend: String,
    pub input_signal_hash: String,
    pub output_signal_hash: String,
    // usage metrics, gate verdicts, HDC fingerprint...
}
```

### Pheromone System (Stigmergic)
`crates/roko-orchestrator/src/coordination.rs` -- agents leave pheromone traces:
- Kinds: `threat`, `opportunity`, `wisdom`, `alpha`, `pattern`, `anomaly`, `consensus`
- Time-decaying signals
- Influence downstream agents via system prompt builder (layer 3c)

### Task Output Persistence
`save_task_output()` / `load_prior_task_outputs()` -- pass results between sequential tasks in same plan.

### Event Bus
`roko-runtime` RokoEvent: `PlanRevision`, `GateVerdictSummary`, etc.
TUI, HTTP server, and orchestrator all subscribe.

## Gate System

### 7 Rungs
```rust
pub enum Rung {
    Compile = 0,       // CompileGate
    Lint = 1,          // ClippyGate
    Test = 2,          // TestGate
    Symbol = 3,        // SymbolGate
    GeneratedTest = 4, // GeneratedTestGate + VerifyChainGate
    PropertyTest = 5,  // PropertyTestGate + FactCheckGate
    Integration = 6,   // LlmJudgeGate + IntegrationGate
}
```

Rungs 0-3 = deterministic. Rungs 4-6 = oracle-backed (LLM or external).

### Adaptive Thresholds
EMA-based, tracked in `.roko/learn/gate-thresholds.json`. Tighten as system gains confidence.

### On Failure
1. AutoFix (up to 5 iterations)
2. Replan (if `learning.replan_on_gate_failure` = true)
3. Terminal failure

## Safety Layer

Each agent wrapped in `SafetyLayer`:
- Tool permissions scoped by role
- Bounds from role manifests enforced
- Temperament-based behavior tuning

## Crash Recovery

State snapshots to `.roko/state/executor.json`. Resume with `--resume`.
ProcessSupervisor tracks PIDs for orphan cleanup.
