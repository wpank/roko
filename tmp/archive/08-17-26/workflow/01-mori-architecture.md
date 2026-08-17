# Mori Multi-Agent Architecture

## Agent Roles (28)

Defined in `apps/mori/src/agent/roles.rs`:

```rust
pub enum AgentRole {
    // --- Core pipeline roles ---
    Conductor,           // Meta-orchestrator, watches all agents, can intervene
    Strategist,          // Analyzes plan, produces brief + task checklist
    Implementer,         // Writes code
    Architect,           // Reviews code quality
    Auditor,             // Reviews spec compliance
    Scribe,              // Writes reference documentation
    Critic,              // Reviews documentation quality

    // --- Support roles ---
    AutoFixer,           // Lightweight fix after gate failure
    QuickReviewer,       // Single-pass reviewer (replaces Architect+Auditor+Scribe for Standard plans)
    Refactorer,          // Batch cross-plan cleanup
    Researcher,          // Deep research with citations
    PrePlanner,          // Ahead-of-execution analysis

    // --- Specialist roles ---
    DocVerifier,         // Verifies docs post-merge
    IntegrationTester,   // Cross-crate integration tests
    MergeResolver,       // Resolves merge conflicts
    TerminalValidator,   // TUI rendering validation
    GolemLifecycleTester,// Golem lifecycle validation
    SpecDriftDetector,   // Spec drift detection
    RegressionDetector,  // Regression detection
    PerformanceSentinel, // Performance monitoring
    CoverageTracker,     // Test coverage tracking
    PlanLifecycleManager,// Plan lifecycle management
    CrossSystemTester,   // Cross-system testing
    ErrorDiagnoser,      // Error diagnosis
    DependencyValidator, // Dependency validation
    PatternExtractor,    // Code pattern extraction
    SnapshotComparator,  // Snapshot comparison
    FullLoopValidator,   // End-to-end pipeline validation
}
```

## Backend Assignment (Hardcoded)

Each role has a default backend. Core pipeline roles use Claude CLI; most specialist roles use Codex:

```rust
pub fn backend(self) -> AgentBackend {
    match self {
        // Claude CLI (agent-mode roles)
        Self::Conductor | Self::Implementer | Self::Strategist
        | Self::Auditor | Self::Scribe | Self::Critic
        | Self::Researcher | Self::AutoFixer | Self::QuickReviewer
        | Self::FullLoopValidator => AgentBackend::Claude,

        // Codex (specialist/testing roles)
        Self::Architect | Self::Refactorer | Self::PrePlanner
        | Self::DocVerifier | Self::IntegrationTester
        // ... all other testing/specialized roles
        => AgentBackend::Codex,
    }
}
```

Three backends: `AgentBackend::Claude`, `AgentBackend::Codex`, `AgentBackend::Cursor`.

## The Pipeline State Machine

Defined in `apps/mori/src/orchestrator/pipeline.rs`. Each plan runs through:

```
Preflight -> [Strategist] -> Implementer -> CompileGate -> DependencyDenyCheck
  -> TestGate -> IgnoredTestCheck -> SpecComplianceCheck -> [FullLoopTest]
  -> Reviewing -> Verdict -> [DocRevision] -> Committing -> Complete
```

Phases in brackets are conditional (based on complexity classification).

### Failure Loops

- **CompileGate fail** (simple rustc suggestions) -> `AutoFix` -> CompileGate again
- **CompileGate fail** (complex errors) -> back to `Implementer`
- **TestGate fail** (warnings only) -> `AutoFix`; otherwise -> `Implementer`
- **Review verdict: REVISE** (code issues) -> back to `Implementer`
- **Review verdict: REVISE** (docs only) -> `DocRevision` only
- **Review verdict: REVISE** (quick-fixable) -> `QuickFix` -> CompileGate
- Max 3 gate failures before hard halt
- Max configurable iterations (typically 2-3) before force-commit

### Complexity Classification

Defined in `apps/mori/src/orchestrator/complexity.rs`. Determines which phases run:

| Complexity | Strategist | Reviews | Mode | Critic | Max Iter |
|---|---|---|---|---|---|
| **Trivial** (1 crate, 1 task, <=10 min) | No | No | - | No | 1 |
| **Simple** (<=2 crates, <=3 tasks, <=20 min) | No | No | - | No | 2 |
| **Standard** (default) | No | Yes | QuickReviewer | No | 2 |
| **Complex** (>=4 crates, >=8 tasks, >=60 min) | No | Yes | Full (Arch+Aud+Scribe) | Yes | 2 |

Risk escalation: touching core crates or having 3+ deps bumps Trivial/Simple to Standard.

## Parallel Execution (DAG-Based)

Defined in `apps/mori/src/app/parallel.rs` and `apps/mori/src/orchestrator/executor.rs`.

### Plan-Level DAG

```rust
pub struct PlanDag {
    deps: HashMap<String, HashSet<String>>,        // plan -> plans it depends on
    reverse_deps: HashMap<String, HashSet<String>>, // plan -> plans that depend on it
    all_plans: Vec<String>,
    estimates: HashMap<String, u32>,
}
```

Built from:
1. Frontmatter `depends_on` fields
2. Cross-plan task references (e.g., `"09:T3"`)
3. Sequential fallback (sort order)

### Task-Level DAG

```rust
pub struct UnifiedTaskDag {
    nodes: Vec<GlobalTaskId>,
    deps: HashMap<GlobalTaskId, HashSet<GlobalTaskId>>,
    reverse_deps: HashMap<GlobalTaskId, HashSet<GlobalTaskId>>,
    file_sets: HashMap<GlobalTaskId, HashSet<String>>,
    exclusive_files: HashMap<GlobalTaskId, bool>,
    estimates: HashMap<GlobalTaskId, u32>,
    plan_for_task: HashMap<GlobalTaskId, String>,
}
```

`GlobalTaskId` = `"plan_base:task_id"` (e.g., `"06-terminal-navigation:T1"`).

### Parallel Executor

```rust
pub struct ParallelExecutor {
    dag: UnifiedTaskDag,
    completed_tasks: HashSet<GlobalTaskId>,
    in_flight_tasks: HashMap<GlobalTaskId, String>,
    max_concurrent_agents: usize,
    max_parallel_plans: usize,        // default 3, max 4
    plan_states: HashMap<String, PlanState>,
    merge_queue: Vec<String>,         // dependency-ordered
    merge_in_progress: bool,          // only one merge at a time
}
```

Emits `ExecutorAction` commands:
```rust
pub enum ExecutorAction {
    CreatePipeline { plan },
    EnsureWorktree { plan },
    SpawnTaskAgent { task_id, instance_id },
    SpawnTaskAgentBatch { plan, tasks, instance_id },
    RunPlanGates { plan },
    PreSpawnWarmReviewer { plan },
    RunPlanReviews { plan },
    MergePlanToBatch { plan },
    SpawnRefactorer { batch_branch },
}
```

### Worktree Isolation

Each plan gets its own **git worktree**. Agents run in isolated copies of the repo. This allows:
- Multiple plans running concurrently without conflicts
- Worktree-scoped agent instances
- Dependency-ordered merge into batch branch

### MultiAgentPool

Parallel mode uses `MultiAgentPool` (not `AgentPool`):

```rust
pub struct MultiAgentPool {
    connections: HashMap<AgentInstanceId, AgentConnection>,
    warm_pool: HashMap<(AgentRole, String), AgentConnection>,
}

pub struct AgentInstanceId {
    pub role: AgentRole,
    pub instance: String,  // e.g., "06-terminal-navigation"
}
```

Supports:
- **Warm spawning**: pre-start a reviewer while implementer still running
- **Instance scoping**: kill all agents for a plan without affecting others
- **Budget control**: global cap on total active agents

## Agent Dispatch (Spawn Lifecycle)

Three backend types:

### Claude (ClaudeConnection)
```
spawn `claude` CLI with:
  --output-format stream-json
  --append-system-prompt <role_prompt>
  --max-turns <N>
  --allowedTools <tool_list>
  --dangerouslySkipPermissions
  --model <model_slug>
  --max-cost <budget_usd>
```
Safety hooks block `git checkout/switch/push`.

### Codex (AppServerConnection)
Connects to Codex app-server via JSON-RPC over stdio. Protocol: `initialize` + `turn_start`.
Each agent gets isolated runtime home at `.mori/runtime/codex-home/<role>/<instance>`.

### Cursor (CursorAcpConnection)
Connects to Cursor Composer via ACP protocol: `initialize` + `session/new` + `prompt`.

### Process Isolation

Each agent runs in its own process group (`setpgid(0,0)`). Kill sends signal to the process group, then individually to all descendants. PIDs persisted to `.mori/runtime/agent-pids.json` for crash recovery.

## Inter-Agent Communication

Agents communicate through **files on disk**, not direct message passing:

### Context Injection
The `ContextInjector` writes role-filtered context files into each worktree's `context/in/`:
- `execution-pack.md`, `implementer-pack.md`, `architect-pack.md`
- `brief.md`, `tasks.toml`, `prd2-extract.md`, `decomposition.md`
- `learning.md`, `research.md`, `playbook.md`, `reflections.md`

### Review -> Implementer Feedback
Review outputs written by reviewers are parsed by the orchestrator for verdicts, then fed back as "prior reviews" to subsequent Implementer iterations.

### AGENTS.md (Global)
Master document injected into every agent session. Role-filtered using `<!-- role: role1,role2 -->` markers:
- Sections tagged "all" always included
- Other sections filtered by agent's role label
- Reduces ~6K tokens to ~1.5-3K per agent

### Event Bus
```rust
pub enum AgentEvent {
    MessageDelta { role, instance, content },
    TurnCompleted { role, instance, thread_id },
    DiffUpdated { role, instance, diff },
    ApprovalRequested { role, instance, command, approval_id },
    TokenUsage { role, instance, input_tokens, output_tokens, cost_usd },
    ToolCall { role, instance, name },
    CommandOutput { role, instance, content },
    Error { role, instance, error },
    Exited { role, instance, exit_code },
}
```

### Conductor (Meta-Agent)

Monitors agent behavior and intervenes:
```rust
pub enum ConductorAction {
    SendMessage { role, message },     // Nudge/steer an agent
    RestartAgent { role },             // Kill and restart
    ForceAdvance,                      // Skip reviews, commit what's there
    SkipReviews,                       // Skip review phase
    AssignAdditionalTasks { instance_id, task_descriptions },
    PingWarmAgent { instance_id },     // Keepalive
}
```

Configurable thresholds:
```rust
pub struct ConductorConfig {
    pub silence_timeout: Duration,           // 180s
    pub compile_fail_threshold: u32,         // 3
    pub task_stall_timeout: Duration,        // 300s
    pub phase_timeout: Duration,             // 1800s (30 min)
    pub agent_soft_limit: usize,             // 8
    pub test_pass_budget_ratio: f64,         // 0.7
}
```

## Merge Queue

Plans merge to batch in **dependency order**. Only one merge at a time. After every N plans, a `Refactorer` agent runs batch cleanup.

## Crash Recovery

State snapshots:
```rust
pub struct ExecutorSnapshot {
    pub completed_tasks: Vec<String>,
    pub in_flight_tasks: HashMap<String, String>,
    pub completed_plans: Vec<String>,
    pub plan_phases: HashMap<String, PlanPhase>,
    pub plan_iterations: HashMap<String, u32>,
    pub merge_queue: Vec<String>,
    pub review_feedback: HashMap<String, Vec<String>>,
    pub task_failure_counts: HashMap<String, u32>,
    pub skipped_tasks: Vec<String>,
}
```

On startup, orphaned agents from previous runs are killed via PID registry at `.mori/runtime/agent-pids.json`.

## Budget System

Per-role dollar budgets:
```rust
pub fn claude_budget_usd(role: AgentRole, model_slug: &str) -> String {
    let base = match role {
        AgentRole::Implementer => 1.50,
        AgentRole::Strategist | AgentRole::Researcher => 0.75,
        AgentRole::Conductor => 0.50,
        AgentRole::Auditor | AgentRole::QuickReviewer => 0.50,
        AgentRole::Scribe | AgentRole::Critic => 0.40,
        AgentRole::AutoFixer => 0.75,
        _ => 0.50,
    };
    // Opus models get 2x budget, Haiku models get 0.6x
}
```

Spawn priority by role:
```rust
pub fn role_priority(role: AgentRole) -> u32 {
    match role {
        AgentRole::Implementer => 0,       // highest
        AgentRole::Strategist => 1,
        AgentRole::Architect | AgentRole::Auditor => 2,
        AgentRole::Scribe | AgentRole::Critic => 3,
        AgentRole::PrePlanner => 4,
        AgentRole::Refactorer => 5,
    }
}
```

## Provider Health Tracking

```rust
pub struct ProviderHealthTracker {
    states: RwLock<HashMap<String, ProviderHealthState>>,
    failure_threshold: u32,    // 3 consecutive failures = unhealthy
    recovery_ms: u64,         // 2 minute cooldown before retry
}
```
