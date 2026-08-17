# ACP and MCP Deep Dive: Protocol Architecture, Integration, and Evolution

This document provides a comprehensive technical analysis of Roko's ACP (Agent Client
Protocol) and MCP (Model Context Protocol) implementations, how they interconnect, what
they enable in the aggregation-funnel-execute workflow, and where they need to go next.

---

## 1. ACP Protocol: Current State

### 1.1 What ACP Is

ACP is a JSON-RPC 2.0 protocol over stdio that standardizes communication between code
editors (clients) and AI coding agents (servers). Released by Zed Industries in August 2025,
it has since been adopted by JetBrains, Neovim, and over 25 agents including Claude Code,
Codex CLI, and Gemini CLI.

Roko implements ACP spec version 0.12.2 (protocol version 1) in `crates/roko-acp/`. This
makes Roko usable as a coding agent from any ACP-compatible editor -- the same agent binary
that powers `roko plan run` also speaks the wire protocol that Zed, JetBrains, and Neovim
expect.

### 1.2 Crate Structure

The `roko-acp` crate contains 12 source files organized into clean layers:

| File | Role | LOC |
|------|------|-----|
| `types.rs` | Wire format: JSON-RPC messages, session types, update types, content blocks | ~770 |
| `transport.rs` | Newline-delimited JSON-RPC 2.0 over async stdin/stdout | ~220 |
| `handler.rs` | Main dispatch loop: routes requests to handlers, manages session lifecycle | ~310 |
| `session.rs` | Session state: config, history, modes, persistence, config options | ~1300 |
| `bridge_events.rs` | Cognitive event to session/update streaming: the heart of the protocol | ~2800 |
| `pipeline.rs` | Pure state machine: receives events, emits actions, zero I/O | ~540 |
| `runner.rs` | Side-effect driver: spawns agents, runs gates, creates commits | ~700 |
| `workflow.rs` | Workflow run tracking: timing, cost, token counts, status summary | ~160 |
| `knowledge.rs` | Dispatch-time knowledge retrieval from neuro store + playbook store | ~410 |
| `acp_adapter.rs` | RuntimeEvent to CognitiveEvent bridge for WorkflowEngine integration | ~200 |
| `config.rs` | Server configuration: workdir, profile, log file, roko.toml loading | ~65 |
| `lib.rs` | Module declarations, public re-exports | ~20 |

Total: approximately 7,500 lines of Rust implementing a full ACP agent server.

### 1.3 Wire Format and Message Flow

The transport layer (`transport.rs`) implements bidirectional JSON-RPC 2.0 with support for
outbound requests from the agent to the editor (a critical ACP feature that MCP lacks). The
`StdioTransport<R, W>` is generic over reader/writer for testability.

Key transport capabilities:
- `read_message()` -- reads one newline-delimited JSON-RPC message
- `send_response()` / `send_error()` -- standard JSON-RPC responses
- `send_notification()` -- server-initiated notifications (session/update)
- `send_request()` -- agent-to-editor requests with pending response tracking

The pending request registry (`HashMap<u64, oneshot::Sender<JsonRpcResponse>>`) enables
the agent to make requests to the editor and await responses -- used for filesystem
operations, terminal commands, and permission prompts.

### 1.4 Session Lifecycle

The handler (`handler.rs`) implements the ACP session lifecycle:

```
initialize -> session/new -> session/prompt* -> [session/cancel | session/config/update]*
                    |
                    +-> session/list -> session/load (resume previous session)
```

Method routing in `handle_request()`:

| Method | Handler | What It Does |
|--------|---------|-------------|
| `initialize` | Inline | Returns protocol version, agent capabilities, agent info |
| `session/new` | `SessionManager::create_session` | Creates session with modes, config options, slash commands |
| `session/list` | `SessionManager::list_sessions_with_persisted` | Lists active + on-disk sessions |
| `session/load` | `SessionManager::load_session` | Restores persisted session state |
| `session/prompt` | `handle_session_prompt` | The main work -- dispatches to cognitive loop |
| `session/config/update` | `session.update_config` | Updates model, effort, workflow, etc. |
| `session/set_mode` | `session.set_mode` | Switches between agent/plan/research modes |
| `session/cancel` | Notification handler | Sets CancelToken on the active session |

### 1.5 The Cognitive Loop

When `session/prompt` arrives, the bridge_events module orchestrates:

1. **Guard**: Check session is not already busy (returns `SESSION_BUSY` error code -32001)
2. **Knowledge query**: Parallel query of neuro knowledge store + playbook store
3. **Context resolution**: File context from @-mentions and resource blocks
4. **System prompt assembly**: Mode-specific system prompt + file context + knowledge context
5. **History construction**: Build conversation history from prior turns
6. **Workflow selection**: Determine if a pipeline template applies (express/standard/full/auto)
7. **Dispatch**: Either single-agent path or pipeline path
8. **Episode recording**: Append episode to `.roko/episodes.jsonl` with usage, cost, model info
9. **Cascade router update**: Record observation for bandit-based model routing

The event streaming architecture uses `mpsc::channel(64)` of `CognitiveEvent` variants:

```rust
pub enum CognitiveEvent {
    TokenChunk(String),           // Streamed text
    ThinkingChunk(String),        // Internal reasoning
    ToolCallStart { ... },        // Tool invocation started
    ToolCallComplete { ... },     // Tool finished with results
    PlanUpdate { entries: Vec<PlanEntry> },  // Progress tracking
    Complete { stop_reason, usage },         // Normal completion
    MaxTokens,                    // Budget exhausted
}
```

These are mapped 1:1 to ACP `SessionUpdate` notifications:

| CognitiveEvent | SessionUpdate |
|---------------|---------------|
| `TokenChunk` | `AgentMessageChunk` |
| `ThinkingChunk` | `AgentThoughtChunk` |
| `ToolCallStart` | `ToolCall` (status: pending/in_progress) |
| `ToolCallComplete` | `ToolCallUpdate` (status: completed/failed) |
| `PlanUpdate` | `Plan` |
| `Complete` / `MaxTokens` | Terminates the prompt response |

### 1.6 The Pure Pipeline State Machine

The pipeline (`pipeline.rs`) is the architectural highlight of the ACP implementation.
It is a completely side-effect-free state machine: it receives `PipelineEvent` values and
produces `PipelineAction` values. All I/O (agent spawning, gate execution, git commits)
is performed by the runner module.

State machine phases:

```
Pending -> Strategizing -> Implementing -> Gating -> Reviewing -> Committing -> Complete
                               ^                        |
                               |   AutoFixing <---------+  (gate failure)
                               |       |
                               +-------+  (autofix failed, iterations remain)
```

Three workflow templates control which phases execute:

| Template | Phases | Auto-Selection Criteria |
|----------|--------|------------------------|
| Express | Implement -> Gate -> Commit | Short prompts (<15 words) with fix/typo/rename/update/bump |
| Standard | Implement -> Gate -> Review -> Commit | Default for medium prompts |
| Full | Strategy -> Implement -> Gate -> Review -> Commit | Long prompts (>50 words) or refactor/architecture/system |

The state machine handles all failure paths deterministically:
- Gate failure -> AutoFixer (if iterations remain) -> re-gate
- AutoFix failure -> back to Implementer with combined error context
- Review rejection -> back to Implementer with structured feedback
- Max iterations exceeded -> Halt with reason
- User cancel / timeout / budget exceeded -> Halt from any phase

All transitions are exhaustively pattern-matched in a single `step()` method. Ten unit
tests cover the full transition table.

### 1.7 Session Configuration

Each session exposes 9 configurable options as ACP config dropdowns:

| Option | Type | Values | Effect |
|--------|------|--------|--------|
| Model | Select | All models from roko.toml | Controls which LLM is used |
| Effort | Select | Low / Medium / High / Max | Maps to thinking budget |
| Temperament | Select | Conservative / Balanced / Aggressive / Exploratory | Risk tolerance |
| Routing | Select | Auto / Manual | Cascade router vs. explicit model |
| Clippy | Toggle | On / Off | Whether clippy gate runs |
| Tests | Toggle | On / Off | Whether test gate runs |
| Workflow | Select | None / Express / Standard / Full / Auto | Pipeline template |
| Review | Select | None / Quick / Standard / Thorough | Review depth |
| Retries | Select | 1 / 2 / 3 | Max pipeline iterations |

### 1.8 Knowledge-Informed Dispatch

The knowledge module (`knowledge.rs`) enriches every prompt with context from two stores:

1. **Neuro knowledge store** (`roko-neuro`): Persistent/consolidated/working knowledge entries
   scored by keyword match, confidence, recency, and emotional boost. Top 5 hits returned.

2. **Playbook store** (`roko-learn`): Recorded success/failure playbooks with step sequences.
   Top 3 relevant playbooks returned with success rates.

This context appears in two forms:
- **Knowledge card**: Visible tool-call card in the editor showing what knowledge was found
- **Prompt context**: Injected into the system prompt as structured markdown sections

### 1.9 WorkflowEngine Integration

The `acp_adapter.rs` bridges the WorkflowEngine (from `roko-runtime`) to the ACP session.
It implements the `EventConsumer` trait, receiving `RuntimeEvent` values and mapping them
to `CognitiveEvent` values that stream to the editor.

The runner's `run_with_workflow_engine()` function:
1. Builds `ServiceFactory` with workspace config
2. Creates `WorkflowEngine` with JSONL logger + ACP event consumer
3. Spawns a runtime event bus bridge for async event forwarding
4. Runs the workflow and returns a `WorkflowRunReport` with cost/token data

---

## 2. MCP Integration: Code Intelligence, GitHub, Slack, Scripts

### 2.1 MCP Architecture in Roko

Roko implements MCP as a suite of standalone server binaries sharing a common transport
layer (`roko-mcp-stdio`). Each MCP server runs as a subprocess of the editor or agent,
communicating via the same newline-delimited JSON-RPC 2.0 protocol.

The shared transport (`crates/roko-mcp-stdio/src/lib.rs`) provides:
- `serve_stdio(reader, writer, handler)` -- synchronous JSON-RPC loop
- `JsonRpcRequest` / `JsonRpcError` types
- Standard error codes (parse error, invalid request, method not found, internal error)
- Notification handling (requests without `id` are dispatched but responses discarded)

This design means every MCP server in Roko is a ~100-line `main.rs` that calls
`serve_stdio()` with a handler function.

### 2.2 roko-mcp-code: Code Intelligence

Path: `crates/roko-mcp-code/src/lib.rs` (~1500 lines)

This is Roko's most sophisticated MCP server. It exposes code intelligence queries backed
by `roko-index::WorkspaceIndex` -- a pre-built index of the entire codebase.

**Exposed tools (12):**

| Tool | Description | Arguments |
|------|-------------|-----------|
| `symbol_lookup` | Find symbol definitions by name | `name` |
| `call_graph` | Trace call chains from a function | `function`, `depth` |
| `file_imports` | List imports for a file | `file` |
| `semantic_search` | Semantic code search | `query`, `limit` |
| `search_code` | Multi-strategy code search | `query`, `strategy`, `max_results`, `file_pattern`, `kind_filter` |
| `get_symbol_context` | Full context for a symbol | `symbol_name`, `include_dependencies`, `include_callers`, `expansion_depth` |
| `get_file_ast` | AST outline for a file | `file_path`, `include_bodies` |
| `find_similar_patterns` | HDC-based pattern matching | `reference`, `min_similarity`, `max_results` |
| `find_references` | All usages of a symbol | `symbol_name`, `file_path`, `include_definitions` |
| `find_implementations` | Trait implementors | `trait_name`, `include_methods` |
| `get_callers` | Reverse call graph | `function_name`, `transitive`, `max_depth` |
| `workspace_map` | Crate/module/symbol overview | `depth`, `focus_path` |
| `get_context` | Task-aware context assembly | `task`, `token_budget`, `include_tests` |

**Search strategies:**
- `keyword` -- text matching
- `structural` -- AST-aware search
- `hdc` -- hyperdimensional computing similarity
- `embedding` -- semantic vector similarity
- `hybrid` -- weighted combination of all strategies

The `get_context` tool is particularly notable: given a task description and token budget,
it assembles the most relevant code context by combining symbol lookup, call graph
traversal, and semantic search. This is the tool that agents use to understand what code
they need to modify.

### 2.3 roko-mcp-github: GitHub Integration

Path: `crates/roko-mcp-github/src/main.rs` (~800 lines)

Full GitHub API integration via REST. Includes automatic rate-limit handling with
retry-after headers and exponential backoff.

**Exposed tools (14):**

| Tool | Description |
|------|-------------|
| `github_list_prs` | List pull requests with state/head/base filters |
| `github_get_pr` | Get PR details including diff |
| `github_create_pr` | Create PR with title, body, head, base, draft |
| `github_comment_pr` | Add comment to PR |
| `github_review_pr` | Submit review (approve/request_changes/comment) |
| `github_merge_pr` | Merge PR (merge/squash/rebase) |
| `github_list_issues` | List issues with state/labels/assignee filters |
| `github_create_issue` | Create issue with labels and assignees |
| `github_comment_issue` | Add comment to issue |
| `github_close_issue` | Close issue with reason |
| `github_get_file` | Read file contents from repo (base64 decoded) |
| `github_search_code` | Search code in repository |
| `github_list_commits` | List commits with sha/path/since/until filters |
| `github_create_branch` | Create branch from SHA |
| `github_get_branch` | Get branch info |
| `github_compare_branches` | Compare two branches |
| `github_get_actions_status` | CI status for a ref |
| `github_add_labels` | Add labels to issue/PR |
| `github_create_label` | Create repository label |

Authentication via `GITHUB_TOKEN` environment variable. All responses are sanitized to
return structured JSON with relevant fields extracted from the GitHub API responses.

### 2.4 roko-mcp-slack: Team Communication

Path: `crates/roko-mcp-slack/src/main.rs` (~600 lines)

Slack Web API integration with full CRUD for messages, threads, channels, and users.

**Exposed tools (9):**

| Tool | Description |
|------|-------------|
| `slack.post_message` | Post message to channel (with optional thread_ts) |
| `slack_reply` | Reply to a thread |
| `slack_get_thread` | Get all messages in a thread |
| `slack_react` | Add emoji reaction |
| `slack_list_channels` | List channels with topic, purpose, member count |
| `slack_lookup_user` | Find user by email or name |
| `slack_dm` | Send direct message to user |
| `slack_get_channel_history` | Get channel history with time range |
| `slack_update_message` | Edit an existing message |

Authentication via `SLACK_BOT_TOKEN`. Handles Slack pagination, DM channel opening,
and user list searching.

### 2.5 roko-mcp-scripts: Script Execution

Path: `crates/roko-mcp-scripts/src/main.rs` (~400 lines)

Sandboxed script execution MCP server. Scripts are discovered from configured root
directories, executed with environment variable allowlisting, and subject to configurable
timeouts.

**Exposed tools (2):**

| Tool | Description |
|------|-------------|
| `run_script` | Execute a script with arguments, return stdout/stderr/exit_code |
| `list_scripts` | List available scripts with descriptions |

Security features:
- Environment variable clearing (`env_clear()`) with explicit allowlist
- Configurable timeout (default: 300s) with `tokio::time::timeout`
- Path traversal prevention via component validation
- Script description extraction from `# description:` comments

### 2.6 MCP-ACP Integration Point

The ACP session accepts MCP server configurations at session creation time:

```rust
pub struct SessionNewParams {
    pub session_name: Option<String>,
    pub client_capabilities: Option<ClientCapabilities>,
    pub mcp_servers: Vec<McpServerConfig>,  // <-- MCP servers attached to this session
}
```

MCP servers are specified with transport configuration:

```rust
pub enum McpTransport {
    Http { url: String },
    Stdio { command: String, args: Vec<String> },
}
```

The ACP agent also declares MCP capabilities in its `initialize` response:

```rust
AgentCapabilities {
    mcp_capabilities: McpCapabilities {
        http: true,
        sse: true,
    },
    ...
}
```

In the agent dispatch path, MCP configuration from `roko.toml` is passed through to
the `ServiceFactory` via `ServiceConfig.mcp_config`. The agent-level MCP passthrough
writes `.roko/mcp-config.json` and passes `--mcp-config` to Claude CLI processes.

---

## 3. Workflow Patterns: How ACP/MCP Enable Aggregation-Funnel-Execute

### 3.1 The Three-Phase Pattern

The aggregation-funnel-execute pattern maps directly to how ACP and MCP collaborate:

**Phase 1 -- Aggregation (MCP):**
MCP servers gather context from diverse sources. The `roko-mcp-code` server provides
codebase intelligence (symbols, call graphs, patterns). `roko-mcp-github` provides
repository context (PRs, issues, CI status). `roko-mcp-slack` provides team context
(discussions, decisions). `roko-mcp-scripts` provides workspace tooling.

Each MCP server acts as a context funnel: raw external data is queried, filtered, and
returned as structured JSON that agents can consume.

**Phase 2 -- Funnel (ACP + Knowledge):**
The ACP bridge_events module acts as the funnel. When a `session/prompt` arrives:

1. Knowledge store query surfaces relevant prior knowledge (scored, ranked, truncated)
2. Playbook store query finds relevant success/failure patterns
3. File context resolution reads @-mentioned files
4. System prompt assembly combines mode prompt + knowledge + file context + history
5. Cascade router selects the optimal model based on bandit-learned rewards

This funnel reduces potentially unbounded context to a focused, scored, budget-aware
prompt that fits within the model's context window.

**Phase 3 -- Execute (ACP Pipeline):**
The pure state machine drives execution through structured phases. Agents are spawned
with focused prompts. Gates validate output. Review agents provide feedback. The pipeline
iterates until convergence or budget exhaustion.

### 3.2 Context Flow Diagram

```
                    MCP Layer (aggregation)
                    ========================
 roko-mcp-code -----> symbol context, call graphs, patterns
 roko-mcp-github ---> PRs, issues, CI status, file contents
 roko-mcp-slack ----> discussions, decisions, team context
 roko-mcp-scripts --> workspace tooling, custom checks
                    |
                    v
                    ACP Layer (funnel)
                    ========================
 Knowledge store ---> scored knowledge hits (top 5)
 Playbook store ---> relevant playbooks with success rates (top 3)
 File resolver -----> @-mentioned file contents
 History builder ---> conversation turns (truncated at 10KB/turn)
 Cascade router ----> optimal model selection
                    |
                    v
                    Pipeline Layer (execute)
                    ========================
 Strategist --------> analysis brief (Full template only)
 Implementer -------> code changes
 Gates -------------> compile, test, clippy validation
 Reviewer ----------> code review verdict
 AutoFixer ---------> targeted error fixes
 Committer ---------> git commit with structured message
```

### 3.3 Knowledge-Enriched Dispatch

The dispatch path shows how knowledge shapes every agent invocation:

```
prompt arrives
    |
    +-> query_dispatch_knowledge(workdir, prompt)
    |       |
    |       +-> query_knowledge_hits(workdir, prompt)  [roko-neuro, top 5]
    |       +-> query_playbooks(workdir, prompt)       [roko-learn, top 3]
    |       |
    |       +-> DispatchKnowledge { hits, playbooks }
    |
    +-> emit_knowledge_card(&knowledge, &event_sender)
    |       |
    |       +-> KnowledgeCard { title, body } -> ToolCallStart + ToolCallComplete
    |
    +-> build_provenance(&knowledge.hits, &knowledge.playbooks, prompt, workdir)
    |       |
    |       +-> Provenance metadata for episode recording
    |
    +-> knowledge.context_text()
    |       |
    |       +-> "## Relevant playbooks from past tasks:\n..."
    |       +-> "## Relevant knowledge:\n..."
    |
    +-> append_context(&system_prompt, &knowledge_context)
            |
            +-> Combined system prompt with knowledge injection
```

### 3.4 Cost and Episode Tracking

Every ACP dispatch records a complete episode:

```rust
episode.kind = "acp-dispatch" | "acp-pipeline-{template}";
episode.model = resolved.slug;
episode.backend = resolved.provider_kind.label();
episode.usage = EpUsage {
    wall_ms, input_tokens, output_tokens,
    cache_read_tokens, cache_write_tokens,
    cost_usd, cost_usd_without_cache,
};
episode.success = acp_dispatch_succeeded(...);
episode.failure_reason = Some("cancelled" | "max_tokens" | "refusal" | ...);
```

The cascade router is also updated with bandit observations:

```rust
let reward = compute_acp_reward(success, wall_ms, output_tokens);
// reward = 0.8 (success) + latency bonus (0.05-0.15) + token bonus (0.02-0.05)
router.observe(context_vec, model_idx, reward);
```

This creates a feedback loop: every ACP interaction trains the model router, which
improves future model selection, which improves future interaction quality.

---

## 4. Missing Capabilities: What the Protocols Need

### 4.1 ACP Gaps

**4.1.1 No Agent-to-Agent Communication**

The current pipeline is strictly sequential: one agent runs at a time. The Strategist
finishes before the Implementer starts. The Implementer finishes before Gates run.
There is no mechanism for agents to communicate during execution.

What is needed:
- Parallel agent execution with shared context
- Cross-agent message passing for real-time coordination
- Shared artifact store that multiple agents read/write concurrently

**4.1.2 No Progressive Context Management**

The current context assembly is all-or-nothing: knowledge and file context are gathered
once at prompt time and injected into the system prompt. There is no mechanism for:
- Context refresh mid-conversation as the codebase changes
- Progressive context narrowing as the agent focuses on specific files
- Context budget awareness that prevents context window overflow
- Dynamic context eviction when new information arrives

**4.1.3 No Task Decomposition in the Pipeline**

The pipeline handles single prompts. Complex tasks require manual decomposition into
separate prompts. What is needed:
- Automatic task splitting based on complexity analysis
- Sub-task dependency tracking (DAG execution within a pipeline)
- Cross-task context propagation

**4.1.4 Limited Bidirectional Communication**

ACP supports agent-to-editor requests (`fs/read_text_file`, `terminal/create`, etc.)
but the current Roko implementation primarily uses the single-agent dispatch path with
SSE-style provider calls. The bidirectional capabilities are declared but underutilized.

**4.1.5 No Session Transfer**

Sessions cannot be transferred between editors. A session started in Zed cannot be
continued in JetBrains. The session state is persisted locally but not in a format
that another editor could resume.

### 4.2 MCP Gaps

**4.2.1 No MCP Federation**

Each MCP server is an isolated subprocess. There is no mechanism for MCP servers to
discover or communicate with each other. The `roko-mcp-code` server cannot query
`roko-mcp-github` to understand what PRs touch the symbols it found.

**4.2.2 No Streaming or Async in MCP Servers**

The `roko-mcp-stdio` transport is synchronous. The MCP spec (November 2025) introduced
the Tasks primitive for async operations and Streamable HTTP for remote deployment.
Roko's MCP servers support neither.

**4.2.3 No Context Budget Awareness**

MCP servers return whatever data they find with no awareness of the caller's context
budget. The `get_context` tool in `roko-mcp-code` accepts a `token_budget` parameter,
but the other MCP servers have no such concept. A GitHub PR diff could be 100KB.

**4.2.4 No MCP Authentication**

The MCP spec (November 2025) introduced OAuth 2.1 with Protected Resource Metadata
discovery. Roko's MCP servers use environment variables for authentication
(`GITHUB_TOKEN`, `SLACK_BOT_TOKEN`). There is no OAuth flow, no token refresh, no
multi-tenant support.

**4.2.5 No Sampling Support**

The MCP spec (November 2025) introduced Sampling with Tools (SEP-1577), allowing MCP
servers to initiate LLM calls using the client's model. This enables MCP servers to
run their own agentic loops. Roko's MCP servers are purely tool-based with no sampling
capability.

### 4.3 Cross-Protocol Gaps

**4.3.1 No Unified Context Pipeline**

ACP knowledge injection and MCP tool results flow through different paths. Knowledge
hits go into the system prompt. MCP tool results go into tool_result messages. There
is no unified ranking or budget allocation across both sources.

**4.3.2 No Feedback from MCP to Learning**

MCP tool call outcomes are not recorded in the episode log or fed to the cascade
router. If a `search_code` query returns poor results, there is no feedback mechanism
to improve future queries.

**4.3.3 No A2A Bridge**

Google's Agent-to-Agent Protocol (A2A), now under the Linux Foundation with 150+
organizations, provides agent discovery, task management, and collaboration capabilities.
Roko has no bridge between its internal agent system and the A2A ecosystem.

---

## 5. Novel Extensions

### 5.1 Agent-to-Agent Communication via ACP

The ACP pipeline's pure state machine can be extended to support parallel agent
execution with shared context.

**Proposed architecture:**

```
PipelinePhase::ParallelExecution {
    agents: Vec<AgentSlot>,
    barrier: BarrierCondition,
    shared_context: SharedContextStore,
}
```

New pipeline events:
- `AgentPartialResult { agent_id, artifact }` -- intermediate output
- `AgentContextRequest { agent_id, query }` -- cross-agent context query
- `BarrierReached { agent_id }` -- agent finished its phase

New pipeline actions:
- `SpawnParallelAgents { specs: Vec<AgentSpec> }` -- launch N agents
- `BroadcastContext { context }` -- update shared context for all agents
- `MergeResults { strategy: MergeStrategy }` -- combine parallel outputs

The key insight is that the state machine remains pure. Parallel execution is an
action; the runner handles the async orchestration. The state machine only tracks
which agents have completed and what barrier conditions are met.

**Example: Full pipeline with parallel review:**

```
Strategizing
    |
    v
Implementing
    |
    v
Gating
    |
    v
ParallelExecution [Architect, Auditor, Scribe]  <-- NEW
    |
    v
VerdictMerge  <-- NEW: combine 3 review opinions
    |
    v
Committing
    |
    v
Complete
```

### 5.2 Distributed MCP Federation

MCP servers should be able to discover and query each other. This enables compound
queries like "find all symbols changed in PR #42 and their test coverage."

**Proposed architecture:**

```
MCP Federation Registry
    |
    +-- roko-mcp-code (tools: symbol_lookup, call_graph, ...)
    |       |
    |       +-- can query roko-mcp-github for PR context
    |       +-- can query roko-mcp-slack for decision context
    |
    +-- roko-mcp-github (tools: list_prs, create_pr, ...)
    |       |
    |       +-- can query roko-mcp-code for symbol impact analysis
    |
    +-- roko-mcp-slack (tools: post_message, ...)
    |       |
    |       +-- can query roko-mcp-github for PR status
    |
    +-- roko-mcp-scripts (tools: run_script, ...)
```

Implementation approach: MCP servers register with a local registry (file-based or
socket-based). Each server can make cross-server tool calls via the registry. The
registry handles lifecycle, health checking, and request routing.

This aligns with the MCP 2026 roadmap priorities: Streamable HTTP transport (remote
servers), Tasks primitive (async operations), and enterprise readiness (audit trails,
configuration portability).

### 5.3 Progressive Context Management

Replace the current all-or-nothing context assembly with a progressive system:

**Context lifecycle:**
1. **Initial sweep**: Broad context gathering (knowledge + file + MCP tools)
2. **Scoring**: Rank all context items by relevance to the current task
3. **Budget allocation**: Fit into token budget using priority queue
4. **Mid-turn refresh**: As the agent works, refresh context based on files touched
5. **Eviction**: When new context arrives, evict lowest-scored items
6. **Carry-forward**: Between pipeline phases, carry relevant context forward

**Implementation:**
```rust
pub struct ContextManager {
    budget: TokenBudget,
    items: BTreeMap<ContextScore, ContextItem>,
    sources: Vec<Box<dyn ContextSource>>,
}

pub trait ContextSource {
    async fn query(&self, task: &str, budget: usize) -> Vec<ContextItem>;
    fn priority(&self) -> u8;  // Higher = evicted last
}
```

Context sources would include: knowledge store, playbook store, MCP code intelligence,
MCP GitHub context, file resolver, conversation history, and pipeline phase context
(strategist brief, review feedback, gate errors).

### 5.4 Learning-Informed MCP

MCP tool calls should feed the learning system:

```
MCP tool call -> result
    |
    +-> record ToolCallEpisode {
            tool: "search_code",
            query: "...",
            strategy: "hybrid",
            results_count: 5,
            result_quality: agent_reported_score,
            context_used: true/false,
        }
    |
    +-> update ToolEffectiveness bandit
            |
            +-> future calls prefer strategies that worked
```

This creates a feedback loop where MCP tool usage improves over time. The cascade
router pattern (bandit-based selection) generalizes naturally to tool strategy selection.

### 5.5 A2A Bridge for External Agent Collaboration

Bridge Roko's internal agent system to Google's A2A protocol for cross-organization
agent collaboration:

```
Roko Agent System           A2A Bridge           External Agents
=================           ==========           ===============
PipelineAction::         -> A2A Task            -> External agent
  SpawnImplementer           with Agent Card        executes task
                                                        |
CognitiveEvent::         <- A2A Task Result    <--------+
  ToolCallComplete
```

Roko would publish an Agent Card describing its capabilities:
- Code implementation (multiple languages)
- Code review
- Research and analysis
- Plan generation from PRDs

External agents could delegate coding tasks to Roko via A2A, and Roko could delegate
specialized tasks (security audit, performance profiling, UI design) to external agents.

---

## 6. Implementation Plan

### 6.1 Phase 1: Progressive Context (2-3 weeks)

**Goal**: Replace all-or-nothing context assembly with budget-aware progressive system.

| Task | File | Description |
|------|------|-------------|
| 1.1 | `crates/roko-acp/src/knowledge.rs` | Add `TokenBudget` struct with per-source allocation |
| 1.2 | `crates/roko-acp/src/knowledge.rs` | Implement `ContextManager` with priority queue |
| 1.3 | `crates/roko-acp/src/bridge_events.rs` | Replace inline context assembly with `ContextManager` |
| 1.4 | `crates/roko-mcp-code/src/lib.rs` | Add `token_budget` parameter to all search tools |
| 1.5 | `crates/roko-acp/src/session.rs` | Add context budget to session config options |
| 1.6 | `crates/roko-acp/src/session.rs` | Track per-turn context usage for budget learning |

**Key constraint**: Context budget must account for system prompt + history + knowledge +
file context + tool results. The `ContextManager` allocates remaining budget after
fixed-size items (system prompt, history) are subtracted.

### 6.2 Phase 2: MCP Async and Streaming (2-3 weeks)

**Goal**: Upgrade MCP servers to support Tasks primitive and Streamable HTTP.

| Task | File | Description |
|------|------|-------------|
| 2.1 | `crates/roko-mcp-stdio/src/lib.rs` | Add async `serve_stdio_async()` with tokio |
| 2.2 | `crates/roko-mcp-stdio/src/lib.rs` | Add Tasks primitive support (create, poll, cancel) |
| 2.3 | `crates/roko-mcp-stdio/src/http.rs` | NEW: Streamable HTTP transport for remote deployment |
| 2.4 | `crates/roko-mcp-code/src/lib.rs` | Convert to async with progress reporting for large queries |
| 2.5 | `crates/roko-mcp-github/src/main.rs` | Add async rate-limit-aware request batching |
| 2.6 | All MCP crates | Add MCP spec version 2025-11-25 compliance |

**Key constraint**: Must remain backward-compatible with existing stdio transport.
Streamable HTTP is additive.

### 6.3 Phase 3: Parallel Agent Execution (3-4 weeks)

**Goal**: Extend pipeline state machine for parallel agent phases.

| Task | File | Description |
|------|------|-------------|
| 3.1 | `crates/roko-acp/src/pipeline.rs` | Add `ParallelExecution` phase and barrier logic |
| 3.2 | `crates/roko-acp/src/pipeline.rs` | Add `AgentPartialResult`, `BarrierReached` events |
| 3.3 | `crates/roko-acp/src/runner.rs` | Implement parallel agent spawning with `tokio::JoinSet` |
| 3.4 | `crates/roko-acp/src/runner.rs` | Implement shared context store for cross-agent access |
| 3.5 | `crates/roko-acp/src/workflow.rs` | Track per-agent cost and token usage in WorkflowRun |
| 3.6 | `crates/roko-acp/src/pipeline.rs` | Add `VerdictMerge` phase for combining review opinions |
| 3.7 | `crates/roko-acp/src/types.rs` | Add ACP session update types for parallel progress |

**Key constraint**: The state machine must remain pure. All parallel I/O is in the runner.
The state machine tracks which agents have completed and emits merge actions when all
agents in a parallel phase finish.

### 6.4 Phase 4: MCP Federation (2-3 weeks)

**Goal**: Enable MCP servers to discover and query each other.

| Task | File | Description |
|------|------|-------------|
| 4.1 | `crates/roko-mcp-stdio/src/registry.rs` | NEW: Local MCP server registry |
| 4.2 | `crates/roko-mcp-stdio/src/federation.rs` | NEW: Cross-server tool call routing |
| 4.3 | `crates/roko-mcp-code/src/lib.rs` | Add `github_context` tool that queries roko-mcp-github |
| 4.4 | `crates/roko-mcp-github/src/main.rs` | Add `code_impact` tool that queries roko-mcp-code |
| 4.5 | `crates/roko-acp/src/config.rs` | Add federation config to `AcpConfig` |

**Key constraint**: Federation must be optional. MCP servers must work standalone when
the registry is not available. Cross-server calls must have timeout and circuit-breaker
protection.

### 6.5 Phase 5: Learning-Informed MCP (1-2 weeks)

**Goal**: Record MCP tool call outcomes and feed the learning system.

| Task | File | Description |
|------|------|-------------|
| 5.1 | `crates/roko-learn/src/tool_effectiveness.rs` | NEW: Tool call episode recording |
| 5.2 | `crates/roko-mcp-stdio/src/lib.rs` | Add optional episode callback to `serve_stdio` |
| 5.3 | `crates/roko-acp/src/bridge_events.rs` | Record MCP tool calls in episode log |
| 5.4 | `crates/roko-learn/src/cascade_router.rs` | Extend bandit to tool strategy selection |
| 5.5 | `crates/roko-mcp-code/src/lib.rs` | Use learned strategy preferences in `search_code` |

### 6.6 Phase 6: A2A Bridge (3-4 weeks)

**Goal**: Bridge Roko to Google's A2A protocol for external agent collaboration.

| Task | File | Description |
|------|------|-------------|
| 6.1 | `crates/roko-a2a/` | NEW crate: A2A protocol types and client |
| 6.2 | `crates/roko-a2a/src/agent_card.rs` | Roko Agent Card publication |
| 6.3 | `crates/roko-a2a/src/task_bridge.rs` | PipelineAction <-> A2A Task mapping |
| 6.4 | `crates/roko-serve/src/routes/a2a.rs` | HTTP endpoints for A2A task reception |
| 6.5 | `crates/roko-acp/src/pipeline.rs` | Add `DelegateExternal` action for A2A delegation |
| 6.6 | `crates/roko-a2a/src/discovery.rs` | Agent Card discovery and capability matching |

**Key constraint**: A2A is built on HTTP, SSE, and JSON-RPC -- the same stack as
Roko's existing `roko-serve` crate. The A2A bridge should reuse the existing HTTP
infrastructure rather than building parallel transport.

---

## 7. Protocol Landscape: ACP, MCP, and A2A Positioning

### 7.1 Three Protocols, Three Layers

The emerging agent protocol stack has three distinct layers:

| Layer | Protocol | Purpose | Roko Status |
|-------|----------|---------|-------------|
| Editor-Agent | ACP | Agent-editor integration (session, prompt, update) | Implemented (v0.12.2) |
| Agent-Tool | MCP | Tool and context provision (tools, resources, sampling) | Implemented (4 servers) |
| Agent-Agent | A2A | Cross-agent collaboration (tasks, cards, discovery) | Not implemented |

These layers are complementary, not competing:
- ACP handles the user-facing interaction and workflow orchestration
- MCP provides the tools and context that agents use during execution
- A2A enables agents to collaborate with external agents

### 7.2 Protocol Evolution Timeline

**MCP (Anthropic, November 2024 -> Linux Foundation, December 2025):**
- 2024-11: Open-sourced by Anthropic
- 2025-06: Auth spec update (OAuth 2.1, Resource Indicators)
- 2025-11: Major spec release (Tasks, Streamable HTTP, Sampling with Tools, Elicitation)
- 2025-12: Donated to Agentic AI Foundation (Linux Foundation)
- 2026: Working groups on remote transport, task lifecycle, enterprise readiness

**ACP (Zed Industries, August 2025):**
- 2025-08: Released by Zed
- 2025-10: JetBrains partnership announced
- 2025-12: 15+ agents support ACP
- 2026-01: GitHub Copilot CLI adds ACP support
- 2026-03: 25+ agents, community governance at agentclientprotocol.org

**A2A (Google, April 2025 -> Linux Foundation, June 2025):**
- 2025-04: Announced at Google Cloud Next with 50+ partners
- 2025-06: Donated to Linux Foundation
- 2025-08: IBM's ACP (different from Zed's ACP) merged into A2A
- 2026: v1.0 with Signed Agent Cards, 150+ organizations

### 7.3 Roko's Position

Roko is uniquely positioned at the intersection of all three protocols:

1. **ACP server**: Any editor can use Roko as a coding agent
2. **MCP host**: Roko spawns MCP servers for context (code, GitHub, Slack, scripts)
3. **Potential A2A participant**: Roko can both provide and consume agent services

The aggregation-funnel-execute pattern maps naturally to this stack:
- MCP servers aggregate context (aggregation)
- ACP knowledge injection funnels context (funnel)
- ACP pipeline executes the work (execute)
- A2A could extend execution to external agents (distributed execute)

---

## 8. Appendix: Code References

### 8.1 Key Type Definitions

```
crates/roko-acp/src/types.rs:6      -- ACP_PROTOCOL_VERSION = 1
crates/roko-acp/src/types.rs:9      -- ACP_SPEC_VERSION = "0.12.2"
crates/roko-acp/src/types.rs:33     -- JsonRpcMessage (Request | Response | Notification)
crates/roko-acp/src/types.rs:357    -- ContentBlock (Text | Resource | Diff)
crates/roko-acp/src/types.rs:391    -- SessionUpdate (11 variants)
crates/roko-acp/src/types.rs:469    -- ToolCallKind (Edit | Create | Delete | Terminal | Other)
crates/roko-acp/src/types.rs:484    -- ToolCallStatus (Pending | InProgress | Completed | Failed)
crates/roko-acp/src/types.rs:545    -- PlanEntry { content, priority, status }
crates/roko-acp/src/pipeline.rs:12  -- PipelinePhase (10 variants)
crates/roko-acp/src/pipeline.rs:44  -- PipelineEvent (12 variants)
crates/roko-acp/src/pipeline.rs:75  -- PipelineAction (8 variants)
crates/roko-acp/src/pipeline.rs:97  -- WorkflowTemplate (Express | Standard | Full)
```

### 8.2 Key Functions

```
crates/roko-acp/src/handler.rs:26           -- run_acp_server()
crates/roko-acp/src/bridge_events.rs:679    -- handle_session_prompt()
crates/roko-acp/src/bridge_events.rs:538    -- stream_events_to_editor()
crates/roko-acp/src/bridge_events.rs:263    -- append_acp_episode()
crates/roko-acp/src/runner.rs:410           -- run_with_workflow_engine()
crates/roko-acp/src/knowledge.rs:65         -- query_dispatch_knowledge()
crates/roko-acp/src/acp_adapter.rs:38       -- AcpAdapter::map_event()
crates/roko-acp/src/pipeline.rs:195         -- PipelineState::step()
crates/roko-mcp-stdio/src/lib.rs:110        -- serve_stdio()
crates/roko-mcp-code/src/lib.rs:183         -- run() [code intelligence server]
```

### 8.3 File Paths

```
/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/           -- ACP implementation
/Users/will/dev/nunchi/roko/roko/crates/roko-mcp-code/src/      -- Code intelligence MCP
/Users/will/dev/nunchi/roko/roko/crates/roko-mcp-github/src/    -- GitHub MCP
/Users/will/dev/nunchi/roko/roko/crates/roko-mcp-slack/src/     -- Slack MCP
/Users/will/dev/nunchi/roko/roko/crates/roko-mcp-scripts/src/   -- Scripts MCP
/Users/will/dev/nunchi/roko/roko/crates/roko-mcp-stdio/src/     -- Shared MCP transport
```
