# Agent System Comparison: Mori vs Roko

Deep comparison of agent architecture between the original Mori orchestrator
(`/Users/will/dev/uniswap/bardo/apps/mori/src/agent/`) and its successor Roko
(`/Users/will/dev/nunchi/roko/roko/crates/roko-agent/`,
`/Users/will/dev/nunchi/roko/roko/crates/roko-compose/`,
`/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch/`).

---

## 1. Role Definition

### Mori: Hardcoded Enum with Static Config

Mori defines 28 roles in a flat enum at
`apps/mori/src/agent/roles.rs`:

```rust
pub enum AgentRole {
    Conductor, Strategist, Implementer, Architect, Auditor,
    Scribe, Critic, Refactorer, PrePlanner, DocVerifier,
    IntegrationTester, MergeResolver, TerminalValidator,
    GolemLifecycleTester, SpecDriftDetector, RegressionDetector,
    PerformanceSentinel, CoverageTracker, PlanLifecycleManager,
    CrossSystemTester, ErrorDiagnoser, Researcher,
    DependencyValidator, PatternExtractor, SnapshotComparator,
    AutoFixer, QuickReviewer, FullLoopValidator,
}
```

Each role carries three pieces of static configuration:

1. **Backend mapping** (`backend()`): hardcoded match arm deciding Claude vs
   Codex vs Cursor per role. 10 roles default to Claude, 18 to Codex, 0 to
   Cursor.
2. **Label/short strings**: `label()` for logs, `short()` for TUI columns.
3. **Index number**: `index()` for stable ordering.

No model tier, budget, or tool permissions are encoded in the enum -- those are
derived inline at spawn time through separate functions.

### Roko: Same Enum + Layered Policy System

Roko defines the same 28 roles in `roko-core/src/agent.rs`:

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

But each role also carries:

1. **Default `ModelTier`**: which capability class to route to (configurable).
2. **Default `TurnBudget`**: dollar ceiling per turn.
3. **Default `ToolPermissions`**: Read/Write/Exec scope.
4. **`RoleProfile` + `PromptPolicy`**: loaded from a manifest for the 6 core
   roles (Strategist, Implementer, Architect, Auditor, QuickReviewer, Scribe).
   These are versioned, config-overridable documents.
5. **`AgentContract`**: tool allowlists that intersect with role capabilities;
   denials win over allows.

The enum is the same shape, but it indexes into a richer policy layer.

---

## 2. Backend / Provider Architecture

### Mori: 3 Backends (Claude, Codex, Cursor)

Mori has three `AgentBackend` variants:

```rust
pub enum AgentBackend { Codex, Cursor, Claude }
```

Backend inference is slug-based (`from_model()`): `claude-*` goes to Claude,
cursor-prefixed slugs go to Cursor, everything else goes to Codex. Each backend
is a concrete connection type:

| Backend | Connection Type | Protocol |
|---------|----------------|----------|
| Claude | `ClaudeConnection` | `claude` CLI subprocess, stream-json |
| Codex | `AppServerConnection` | `codex` app-server, JSON-RPC |
| Cursor | `CursorAcpConnection` | ACP JSON-RPC over stdio |

All three are variants of a single `AgentConnection` enum:
```rust
pub enum AgentConnection { Codex(AppServerConnection), Cursor(...), Claude(...) }
```

### Roko: 11 Provider Kinds + Provider Adapters

Roko extends backends to a two-level system:

**Level 1: ProviderKind (11 protocol families)**:
```rust
pub enum ProviderKind {
    AnthropicApi, ClaudeCli, OpenAiCompat, CursorAcp,
    PerplexityApi, GeminiApi, GeminiCli, CerebrasApi,
    CursorCli, Hermes, OpenClaw,
}
```

**Level 2: AgentBackend (10 CLI dispatch categories)**:
```rust
pub enum AgentBackend {
    Claude, Codex, Cursor, Ollama, OpenAi,
    Perplexity, Cerebras, Hermes, OpenClaw, GeminiCli,
}
```

Each `ProviderKind` has a concrete `ProviderAdapter` implementation:
`AnthropicApiAdapter`, `ClaudeCliAdapter`, `OpenAiCompatAdapter`,
`CursorAcpAdapter`, `CursorCliAdapter`, `GeminiCliAdapter`,
`GeminiAdapter`, `CerebrasAdapter`, `HermesProviderAdapter`,
`OpenClawProviderAdapter`, plus `PerplexityAdapter`.

The critical difference: **Mori hardcodes slug-to-backend inference; Roko
resolves from config first** (`resolve_model()` reads `[models.*]` and
`[providers.*]` from `roko.toml`), falling back to the slug heuristic only
when config is absent. This makes adding new providers a config change, not a
code change.

---

## 3. Agent Lifecycle

### Mori: Process-per-Turn with Persistent Session

Mori's `ClaudeConnection.turn_start()` at `connection.rs:2444-2620` is the
reference spawn. Each "turn" spawns a fresh `claude` subprocess:

```
1. Kill any still-running child from previous turn
2. validate_pre_spawn() -- check working dir, state
3. Build Command("claude") with flags:
   --bare / --print / --verbose / --output-format stream-json
   --model <slug> / --effort <level>
   --append-system-prompt <role_prompt>
   --fallback-model claude-haiku-4-5
   --settings <hooks_json>
   --tools <allowlist>  (role-specific)
   --permission-mode plan | --dangerously-skip-permissions
   --mcp-config <path>  (if available)
   --resume <session_id>  (if continuing)
4. Set env: CARGO_INCREMENTAL=0, CARGO_BUILD_JOBS=2,
   RUSTC_WRAPPER=sccache, ANTHROPIC_BASE_URL/KEY
5. Spawn child, register PID, write prompt to stdin, close stdin
6. Stream stdout (line-buffered JSON), collect stderr
7. On completion: emit TurnCompleted event with thread_id
```

The lifecycle is: **Spawn CLI -> pipe prompt -> stream output -> kill process**.
Session continuity comes from `--resume <session_id>`, not process persistence.

**Codex lifecycle** is similar but uses JSON-RPC: initialize handshake, then
`turn_start` with a thread_id for session persistence.

**Cursor lifecycle** uses ACP: `session/new` creates a session, then
`prompt/start` sends messages. Sessions are lighter weight than full process
spawns.

### Roko: Trait-Based Agent + Tool Loop

Roko's agent lifecycle is more layered:

```
1. SharedAgentFactory (created once per plan run):
   - ProviderSemaphores (concurrency limits)
   - McpRuntime (MCP tool discovery, done once)
   - ProviderDispatchResolver (model -> provider resolution)
   - ProviderRateLimiter
   - ProviderHealthRegistry

2. Per-task dispatch (event_loop.rs -> dispatch/mod.rs):
   a. ModelRouter selects ModelSpec from CascadeRouter + overrides
   b. PromptAssembler builds structured prompt via SystemPromptBuilder
   c. AgentDispatcherV2 resolves the provider adapter
   d. Provider adapter creates a Box<dyn Agent>
   e. Agent.run() or ToolLoop drives the conversation

3. Agent trait (roko-agent/src/agent.rs):
   - run(input: Signal) -> AgentResult
   - Returns output Signal + trace Vec + Usage + success flag
```

The key architectural difference: Mori couples spawn logic directly to each
backend's connection type. Roko decouples through the `Agent` trait and the
`ProviderAdapter` factory, allowing new backends to be added without touching
dispatch logic.

Additionally, Roko has a `ToolLoop` (`roko-agent/src/tool_loop/mod.rs`) that
drives the `prompt -> LLM -> tool_calls -> dispatch -> results -> LLM` cycle
for backends where Roko controls the conversation (Ollama, OpenAI-compat).
Claude CLI drives its own internal loop.

---

## 4. Pool Management

### Mori: Two Pool Types

**`AgentPool`** (single-agent per role):
```rust
pub struct AgentPool {
    connections: HashMap<AgentRole, AgentConnection>,
    working_dir: PathBuf,
    event_tx: mpsc::UnboundedSender<AgentEvent>,
    fast_mode: bool,
    fallback_model: Option<String>,
    bare_mode: bool,
}
```

Operations: `spawn(role, effort, model)`, `turn_start(role, message, model)`,
`turn_interrupt(role)`, `respond_approval(role, id, approved)`,
`set_thread_id(role, id)`, `kill(role)`, `kill_all()`.

**`MultiAgentPool`** (multiple instances per role):
```rust
pub struct MultiAgentPool {
    connections: HashMap<AgentInstanceId, AgentConnection>,
    warm_pool: HashMap<(AgentRole, String), AgentConnection>,
    // ... same config fields
}
```

Adds: `spawn_instance(id, working_dir, effort, model)`,
`pre_spawn_warm(id, wd, effort, model)`, `promote_warm(role, instance)`,
`evict_warm(role, instance)`, `kill_plan_agents(plan)`, `kill_role(role)`.

The warm pool is a Mori innovation: pre-spawn the next agent while the current
one is still working, then promote it to active when needed. Saves 5-15s per
phase transition.

### Roko: Simulated Multi-Pool + Warm Pool

Roko has equivalent structures:

**`AgentPool`** (`roko-agent/src/pool.rs`): sequential task execution with
`AgentInstanceId`, `InstanceStatus` (Warm/Pending/Active/Done/Failed/
Cancelled).

**`WarmPool`** (`roko-cli/src/dispatch/warm_pool.rs`): pre-constructed prompt
cache entries and model specs for anticipated next-task dispatches.

**`SharedAgentFactory`** (`roko-cli/src/dispatch/factory.rs`): the factory
itself plays the role of Mori's pool by caching expensive setup (semaphores,
MCP runtime, provider resolver, rate limiter, health registry) across all
dispatches in a run.

---

## 5. System Prompt Construction

### Mori: Single Function, Flat String

Mori constructs its system prompt in `claude_system_prompt()` at
`connection.rs:427`:

```rust
fn claude_system_prompt(role: AgentRole) -> String {
    let role_specific = mori_role_guidance(role);        // 3-5 sentences
    let tool_specific = mori_tool_usage_guidance();      // 1 sentence
    let artifact_specific = mori_role_artifact_hint(role); // 2-3 sentences
    format!(
        "You are running inside Mori... <~400 chars of context>
         {} {} {} Start from {} and widen only when that pack leaves a concrete ambiguity.",
        role_specific, tool_specific, artifact_specific,
        preferred_context_entry(role)  // "context/in/{role}-pack.md"
    )
}
```

The entire prompt is one flat string, ~500-800 tokens. Role differentiation
comes from four match-arm functions:

- `mori_role_guidance(role)` -- 3 variants (implementer, strategist, reviewer)
- `mori_role_artifact_hint(role)` -- 5 variants (strategist, reviewer, tester,
  scribe, default)
- `mori_tool_usage_guidance()` -- one global string
- `preferred_context_entry(role)` -- role label template

There is no layered assembly, no caching tier separation, no scoring, no
budget-awareness.

### Roko: 9-Layer Composable Builder

Roko uses `SystemPromptBuilder` (`roko-compose/src/system_prompt_builder.rs`)
with 9 distinct layers:

| Layer | Content | Cache Tier |
|-------|---------|------------|
| 1 | Role identity | System (stable) |
| 2 | Conventions | System (semi-stable) |
| 3 | Domain context | Session |
| 3c | Active signals (pheromone) | Session |
| 4 | Task context | Task (volatile) |
| 4b | Gate feedback | Dynamic |
| 5 | Tool instructions | System (stable) |
| 6 | Relevant techniques (playbooks/skills) | Task |
| 7 | Anti-patterns | Task |
| 8 | Affect guidance | Dynamic |

Each layer is a `PromptSection` with priority, placement, cache alignment,
and a tag. The builder feeds into `PromptComposer` which:

- Sorts by `SectionPriority`
- Applies a token budget (per-role, per-complexity)
- Scores sections using `GoalDirectedHeuristicScorer` against the task goal
- Inserts cache alignment markers between tiers
- Emits a `CompositionManifest` for audit

Each role has a dedicated template struct implementing `RolePromptTemplate`:
- `ImplementerTemplate` -- typed `ImplementerInput` with plan, brief, tasks,
  workspace map, preflight, registry snapshot, reviews, enhancements
- `StrategistTemplate` -- typed `StrategistInput` with plan, workspace map,
  cross-plan context, PRD extract, iteration, decomposition
- `ReviewerTemplate` -- parametric over `Reviewer::{Architect, Auditor, Combined}`
- `ConductorTemplate`, `ScribeTemplate`, `ResearcherTemplate`,
  `RefactorerTemplate`, `QuickReviewerTemplate`, `QuickFixTemplate`,
  `IntegrationTemplate`, `TaskImplTemplate`

The 6 core roles additionally load `BuiltinRolePolicy` from a versioned
`RolePolicyManifest` with explicit `RoleProfile` + `PromptPolicy`.

This is the largest architectural divergence: Mori has ~50 lines of prompt
code per role; Roko has ~200-400 lines of typed, budget-aware, cache-aligned,
learning-enriched prompt assembly per role.

---

## 6. Tool Access Control

### Mori: Per-Role CLI Flag

Mori sets tool allowlists directly in the `claude` CLI invocation at
`connection.rs:2485-2536`:

```rust
match self.role {
    Conductor => {
        cmd.arg("--tools").arg("Read,Glob,Grep,WebFetch,WebSearch");
        cmd.arg("--permission-mode").arg("plan");
    }
    Scribe => {
        cmd.arg("--tools").arg("Read,Glob,Grep,Write,Edit,WebFetch,WebSearch");
        cmd.arg("--dangerously-skip-permissions");
    }
    QuickReviewer | Auditor | Critic => {
        cmd.arg("--tools").arg("Read,Glob,Grep,Bash,WebFetch,WebSearch");
        cmd.arg("--dangerously-skip-permissions");
        cmd.arg("--json-schema").arg(review_json_schema());
    }
    Implementer | AutoFixer if effort == "low" | "medium" => {
        cmd.arg("--tools").arg("Read,Glob,Grep,Edit,Write,Bash");
    }
    _ => {
        cmd.arg("--tools").arg("Read,Glob,Grep,Edit,Write,Bash,WebFetch,WebSearch");
    }
}
```

Plus a `--settings` JSON for git hooks (blocking `git checkout`, `git switch`,
`git branch -m`, `git push`).

This is effective but has limitations:
- Only Claude CLI tools are controlled (Codex/Cursor have their own tool
  systems)
- No intersection with task-level policy
- No runtime revocation

### Roko: Multi-Layer Safety Stack

Roko's safety enforcement lives in `roko-agent/src/safety/` with 21
submodules:

```
allowlist, authz, bash, capabilities, contract, data_llm, git,
hallucination, hooks, network, path, provenance, rate_limit,
recursive, result_filter, risk, sandbox, scrub, spending,
taint_propagation, temporal, witness
```

The dispatch pipeline (per tool call):
1. **Validate identity and args** -- reject malformed/injected calls
2. **Authorize** through `AgentContract` role/task allowlist intersection
3. **Run safety hooks** -- `CorrigibilityHook`, `TaintLevelHook` from
   durable immune controls
4. **Check path policy** -- worktree-relative canonicalization, escape
   prevention
5. **Check bash policy** -- command allowlist/denylist
6. **Check network policy** -- outbound destination allowlist
7. **Execute handler** under timeout + cancellation
8. **Screen result** through the immune Graph
9. **Finalize** with sanitized terminal audit

Key additions over Mori:
- `AgentContract`: typed allowlists per role+task, denial wins
- `SafetyLayer`: configurable policies from `RokoConfig`
- `ToolDispatcher`: batch limits, per-call and per-batch byte caps
- `TaintPropagation`: tracks taint levels through tool chains
- `CorrigibilityHook`: 5-head corrigibility ordering enforcement
- `RateLimiter`: per-tool, per-role rate limits
- `SpendingPolicy`: budget enforcement at the tool level

---

## 7. The Conductor Pattern

### Mori: LLM-Optional Watchdog

Mori's conductor (`apps/mori/src/conductor/`) has three components:

**Watchers** (`conductor/watchers.rs`): 12 heuristic detectors:
- Ghost turn detector (no output + fast turn)
- Review loop detector (3+ consecutive REVISE verdicts)
- Iteration loop detector (iteration >= 6 without convergence)
- Test failure budget (pass rate meets threshold -> force advance)
- Silence timeout, compile fail threshold, task stall timeout,
  context pressure ratio, phase timeout

**Actions** (`conductor/actions.rs`): typed interventions:
```rust
pub enum ConductorAction {
    SendMessage { role, message },
    RestartAgent { role },
    ForceAdvance,
    SkipReviews,
    SpawnValidation { type, plan },
    GenerateFixPlan { title, desc, crates },
    InsertGate { plan, type },
    SkipValidation { type, plan },
    AssignAdditionalTasks { instance_id, tasks },
    PingWarmAgent { instance_id },
}
```

**LLM advisor** (`conductor/llm.rs`): optional LLM call for when heuristics
don't have a clear action.

The conductor is a read-only observer that emits `Intervention` values; the
event loop decides whether to execute them. It never writes code or modifies
the repo.

### Roko: Distributed Conductor Logic

Roko doesn't have a separate conductor module. Instead, the conductor's
functionality is distributed:

- **Watcher logic** is in `roko-conductor` crate with 12 watchers and
  circuit breaker
- **Conductor role** exists as `AgentRole::Conductor` with a
  `ConductorTemplate` that defines a read-only, decision-making persona
- **Runner event loop** (`roko-cli/src/runner/event_loop.rs`) embeds the
  actual intervention execution: gate failure classification, replan triggers,
  retry logic, timeout enforcement
- **ProcessSupervisor** in `roko-runtime` handles lifecycle management
  (the process-level concerns Mori's conductor tracked)

The runner itself acts as the conductor, with the `ConductorTemplate` serving
as a prompt for situations where an LLM needs to make a routing decision.

---

## 8. Token Tracking

### Mori: Per-Agent Event Streaming

Mori tracks tokens via `AgentEvent::TokenUsage`:
```rust
TokenUsage {
    role: AgentRole,
    instance: Option<String>,
    input_tokens: u64,
    output_tokens: u64,
    context_window: Option<u64>,
    cost_usd: Option<f64>,
}
```

Token events are emitted during stream parsing. The TUI shows `0/200K` per
agent (used/window). Budget enforcement is per-role, set by
`claude_budget_usd()`:

| Role | Base USD |
|------|----------|
| Implementer | $1.50 |
| Strategist/Researcher | $0.75 |
| AutoFixer | $0.75 |
| Conductor | $0.50 |
| Auditor/QuickReviewer | $0.50 |
| Scribe/Critic | $0.40 |
| Others | $0.50 |

With model multipliers: opus = 2x, haiku = 0.6x (min $0.35).

### Roko: Multi-Layer Budget + Learning

Roko's token tracking is substantially richer:

1. **`Usage`** struct in `roko-agent/src/usage.rs` -- raw counters
2. **`UsageObservation`** -- typed observation with provenance
3. **`TurnBudget`** per role in `roko-core/src/agent.rs` -- dollar ceiling
4. **`BudgetTracker`** in lifecycle -- running cost accumulation
5. **`CascadeRouter`** -- learned model routing that considers cost/quality
6. **Efficiency events** -- per-turn records in `.roko/learn/efficiency.jsonl`
7. **Provider health registry** -- cost tracking feeds learned routing

The budget system is configurable via `roko.toml` and is per-plan + per-task,
not just per-role.

---

## 9. MCP Integration

### Mori: Config-File Discovery

Mori discovers MCP tools by searching for config files:
```
$MORI_MCP_CONFIG (env var)
  or .mori/mcp-config.local.json
  or .mori/mcp-config.json
  or .codex/config.toml
```

The config is passed to `claude --mcp-config <path> --strict-mcp-config`.
Some roles skip MCP (AutoFixer, Conductor) to avoid latency and ghost agents.

The MCP server (`mori-mcp`) provides code search tools:
`search_code`, `get_symbol_context`, `find_references`, `find_similar_patterns`,
`workspace_map`, `get_callers`.

### Roko: Runtime MCP + Plugin Layer

Roko has two MCP paths:

1. **CLI passthrough**: `--mcp-config` forwarding (like Mori) for Claude CLI
   dispatches
2. **Runtime MCP** (`roko-agent/src/mcp/`): `McpRuntime` discovers and
   initializes HTTP MCP clients at factory creation time. These persist for
   the duration of the run.

Additionally, `roko-plugin` provides a plugin layer that includes MCP
integration: `Claude/Codex MCP`, `Cursor/Hermes ACP`, and `native
authenticated Gemini CLI MCP` connections.

`roko-std` defines 35 tools by default (16 executable local + 19 GitHub MCP),
with 52 including typed optional-chain placeholders. Tool definitions are
registered through `ToolRegistry` and resolved through `HandlerResolver` at
dispatch time.

`roko-mcp-code` provides a code-intelligence MCP server equivalent to
Mori's `mori-mcp`.

---

## 10. Context Management

### Mori: Context Packs

Mori's context strategy is file-based "packs":
- `context/in/{role}-pack.md` -- pre-assembled per-role context
- `context/in/execution-pack.md` -- fallback shared context
- Role artifacts: `brief.md`, `research.md`, `decomposition.md`,
  `review-tasks.toml`, etc.

The system prompt tells agents to start from their role pack and widen only
on ambiguity. This is effective but static -- the packs are generated
before the run and don't adapt.

### Roko: Dynamic Context Assembly

Roko assembles context dynamically per task through multiple systems:

1. **`ContextAssembler`** (`roko-compose/src/context_assembler.rs`) --
   collects context from multiple sources
2. **`AttentionBidder`** (`roko-compose/src/attention.rs`) -- 3 variants
   (Neuro, Task, Research) that bid for context window space
3. **`ContextMesh`** (`roko-compose/src/context_mesh.rs`) -- cross-agent
   context sharing
4. **Pheromone/Stigmergic signals** -- active environmental context
5. **`ForagingStrategy`** (`roko-compose/src/foraging.rs`) -- information
   foraging heuristics
6. **Knowledge store** (`roko-neuro`) -- durable knowledge with tier
   progression (Transient -> Working)
7. **Playbook injection** (`roko-learn/playbook`) -- top matching
   when/then patterns from prior successful completions

Context is budget-aware: the `GoalDirectedHeuristicScorer` scores each
section against the current task goal, and `PromptComposer` drops
low-priority sections when approaching the context window limit.

---

## 11. Event System

### Mori: Simple Agent Events

```rust
pub enum AgentEvent {
    MessageDelta { role, instance, content },
    TurnCompleted { role, instance, thread_id },
    DiffUpdated { role, instance, diff },
    ApprovalRequested { role, instance, command, approval_id },
    TokenUsage { role, instance, input_tokens, output_tokens, context_window, cost_usd },
    ToolCall { role, instance, name },
    CommandOutput { role, instance, content },
    Error { role, instance, error },
    Exited { role, instance, exit_code },
}
```

9 event variants, all carrying role + instance. Consumed by the TUI and
conductor.

### Roko: Rich Runtime Event Bus

Roko has multiple event layers:

1. **`AgentRuntimeEvent`** -- per-agent events (similar to Mori)
2. **`RuntimeEvent`** -- workspace-level events
3. **`ObservableEvent`** -- 39 production telemetry variants
4. **`DashboardEvent`** -- TUI-targeted events via `StateHub`
5. **`AffectEvent`** -- somatic/daimon signals for affect modulation
6. **`ToolTraceEvent`** -- per-tool-call trace events

All of these feed into the telemetry Lens system (E33) with bounded delivery,
breaker controls, and restart-durable history.

---

## 12. How Mori's Role System Maps to Roko's Generic Dispatch

This is the key question for migration and preset design. Mori's specialized
roles are implemented as a set of *configuration decisions* at spawn time.
Here is how each decision maps to Roko's dispatch system:

### Configuration Dimensions per Role

| Dimension | Mori Location | Roko Equivalent |
|-----------|---------------|-----------------|
| System prompt text | `claude_system_prompt()` + `mori_role_guidance()` | `RolePromptTemplate::role_identity()` + `SystemPromptBuilder` 9 layers |
| Tool allowlist | `--tools` arg in `turn_start()` | `AgentContract` + `ToolPermissions` + `SafetyLayer` |
| Output format | `--json-schema` for reviewers | `PromptPolicy.output_format` in role manifest |
| Permission mode | `--permission-mode plan` for Conductor | `SafetyLayer` config + `sandbox::SandboxPolicy` |
| Budget | `claude_budget_usd()` | `TurnBudget` on `AgentRole` + `budget_remaining` in `DispatchContext` |
| Model selection | `model.unwrap_or("claude-opus-4-6")` | `ModelRouter` + `CascadeRouter` + `model_hint` override |
| Effort level | `--effort` flag | `ReasoningEffort` on `ModelSpec` |
| MCP tools | Role-based skip list | `McpRuntime` + per-role plugin config |
| Context artifacts | `preferred_context_entry()` | `PromptAssembler` + `AttentionBidder` |
| Hooks/constraints | `agent_hooks_settings()` | `SafetyLayer` hooks + `CorrigibilityHook` |

### Preset Layer Design

Mori's role system can be cleanly expressed as a **preset/configuration layer**
on top of Roko's generic dispatch. Each Mori role maps to a tuple:

```rust
struct RolePreset {
    // Identity
    role: AgentRole,
    template: Box<dyn RolePromptTemplate>,

    // Model
    default_model_tier: ModelTier,
    default_effort: ReasoningEffort,
    budget_usd: f64,

    // Tools
    tool_permissions: ToolPermissions,     // Read/Write/Exec
    tool_allowlist: Vec<String>,           // specific tool names
    agent_contract: AgentContract,         // role+task policy
    output_format: Option<OutputFormat>,   // JSON schema for reviewers

    // Safety
    sandbox_level: SandboxLevel,           // plan/skip-permissions/...
    git_hooks: Vec<HookSpec>,              // blocked git operations

    // Context
    context_strategy: ContextStrategy,     // which bidders to activate
    skip_mcp: bool,                        // skip MCP for latency-sensitive roles
    preferred_artifacts: Vec<String>,      // context pack priority
}
```

### Concrete Mappings

**Implementer** (Mori's most active role):
```
template: ImplementerTemplate
tier: ModelTier::Primary (opus/sonnet)
effort: Medium (low tasks) / High (complex tasks)
budget: $1.50 (2x for opus)
tools: Read, Glob, Grep, Edit, Write, Bash [+WebFetch,WebSearch at high effort]
sandbox: skip-permissions
hooks: block git checkout/switch/branch-m/push
context: task-focused, plan + brief + tasks + workspace_map
mcp: yes (code search)
```

**Conductor** (read-only orchestrator):
```
template: ConductorTemplate
tier: ModelTier::Primary
effort: Medium
budget: $0.50
tools: Read, Glob, Grep, WebFetch, WebSearch (NO write/edit/bash)
sandbox: plan mode (read-only)
hooks: all git operations blocked
context: state-focused, execution snapshot + gate results
mcp: no (latency concern)
```

**Reviewer** (Architect/Auditor/QuickReviewer):
```
template: ReviewerTemplate(variant)
tier: ModelTier::Primary
effort: High (thorough analysis)
budget: $0.50
tools: Read, Glob, Grep, Bash, WebFetch, WebSearch (NO write/edit)
sandbox: skip-permissions (read + bash for verification)
output_format: review_json_schema (structured verdict)
hooks: all git operations blocked
context: diff-focused, plan + brief + files_changed + prd2
mcp: yes
```

**AutoFixer** (lightweight post-gate repair):
```
template: QuickFixTemplate
tier: ModelTier::Fast (haiku/small model)
effort: Low
budget: $0.75
tools: Read, Glob, Grep, Edit, Write, Bash
sandbox: skip-permissions
hooks: block git operations
context: minimal, gate failure output only
mcp: no (latency)
```

### What Roko Already Has vs What's Missing

**Already built in Roko:**
- All 28 `AgentRole` variants with labels and defaults
- 11 `RolePromptTemplate` implementations (not all 28, but covering primary
  workflow)
- `AgentContract` for tool policy enforcement
- `SystemPromptBuilder` with 9 layers
- `CascadeRouter` for learned model selection
- `SafetyLayer` with 21 policy families
- `PromptAssembler` + `AttentionBidder` for dynamic context
- `TurnBudget` + `BudgetTracker` for cost enforcement
- `ProviderHealthRegistry` for routing based on success/failure
- Per-task learning and feedback loops (playbooks, efficiency, episodes)

**Missing / partial for full Mori parity:**
1. **Warm pool pre-spawning**: Mori's `pre_spawn_warm` / `promote_warm` /
   `evict_warm` lifecycle. Roko has `WarmPool` but it caches prompt data,
   not live processes.
2. **Process-level orphan reaping**: Mori's `reap_orphaned_children()` and
   `cleanup_orphaned_agents()` with persistent PID registry. Roko's
   `ProcessSupervisor` handles lifecycle but doesn't have the
   crash-recovery PID file persistence.
3. **Structured JSON output for reviewers**: Mori passes `--json-schema` to
   Claude CLI for reviewer roles. Roko has `PromptPolicy.output_format` in
   the manifest but this needs to be wired through to the CLI dispatch path
   for all provider adapters.
4. **Role-specific MCP skip logic**: Mori skips MCP for AutoFixer and
   Conductor to avoid latency. Roko's factory creates MCP once, but
   per-role filtering of whether to inject MCP tools into the dispatch
   is partial.
5. **Gateway API key rotation**: Mori forwards `ANTHROPIC_BASE_URL` and
   `BARDO_GATEWAY_API_KEY` env vars. Roko has key rotation in the
   inference gateway (E26) but the env-var forwarding for CLI subprocess
   dispatches may need alignment.
6. **sccache/cargo job limits per agent**: Mori caps `CARGO_BUILD_JOBS=2`
   and sets `RUSTC_WRAPPER=sccache` per agent to prevent CPU exhaustion
   with 20+ agents. Roko's `ProcessSupervisor` manages lifecycle but
   doesn't enforce cargo-specific resource limits.

---

## 13. Architectural Verdict

Mori's agent system is a **direct-wired, process-oriented, single-project
system**. Every spawn decision is hardcoded for the Bardo workspace. It is
effective, battle-tested, and fast to reason about -- but it can't serve
arbitrary workspaces, arbitrary model providers, or evolve its behavior
from experience.

Roko's agent system is a **provider-neutral, config-driven, learning-capable
framework**. It separates the what (role identity, policy, tools) from the
how (provider adapter, transport protocol, process lifecycle) through clean
trait boundaries. It already exceeds Mori's capabilities in prompt quality,
tool safety, model routing, and telemetry -- while maintaining backward
compatibility through the shared `AgentRole` enum.

The migration path is complete at the architectural level: every Mori role
can be expressed as a configuration of Roko's existing dispatch pipeline.
The remaining gaps are operational (warm process pools, orphan reaping,
cargo resource limits) rather than architectural.
