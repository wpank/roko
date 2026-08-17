# ACP & MCP Protocol Stack: Implementation Plan

Improvements to the ACP session lifecycle, multi-turn context management,
parallel agent execution, MCP federation, learning-informed tool selection,
A2A bridge, workflow template completion, agent-to-agent communication,
permission bridge, and tracker integrations.

Each task includes file paths, steps, and acceptance criteria that can be
verified mechanically. Tasks are grouped by phase and ordered by dependency.

---

## Phase 1: ACP Session Improvements & Multi-Turn Context (7 tasks)

**Objective**: Fix the ACP session's single-prompt limitation and build
progressive context management so agents receive focused, budget-aware
context that evolves across turns.

**Duration estimate**: 4-5 days

### Task 1.1: Add TokenBudget to DispatchKnowledge

**File**: `crates/roko-acp/src/knowledge.rs`
**What**: Replace the unbounded knowledge/playbook query results with
budget-aware retrieval that respects the model's context window.

**Steps**:
1. Add `TokenBudget` struct with fields: `total: usize`,
   `system_prompt: usize`, `history: usize`, `knowledge: usize`,
   `file_context: usize`, `tool_results: usize`
2. Add `fn estimate_tokens(text: &str) -> usize` using the 4-chars-per-token
   heuristic (same as `roko-compose`)
3. Modify `query_dispatch_knowledge()` to accept `budget: usize` parameter
4. In `render_context_body()`, truncate knowledge hits when cumulative
   tokens exceed `budget`, keeping highest-scored items first
5. In `render_playbook_context()`, truncate step lists when they exceed
   per-playbook allocation (`budget / playbooks.len()`)

**Acceptance criteria**:
- Unit test: `query_dispatch_knowledge` with `budget = 500` returns fewer
  items than `budget = 5000` when store has 20+ entries
- Unit test: `render_context_body` output token estimate is <= budget
- Existing tests pass unchanged (backward compatible default budget)

### Task 1.2: Build ContextManager with Priority Queue

**File**: `crates/roko-acp/src/context_manager.rs` (new file)
**What**: Unified context assembly that ranks all context sources by
relevance and fits them into the model's context window.

**Steps**:
1. Define `ContextSource` enum: `Knowledge`, `Playbook`, `FileContext`,
   `ConversationHistory`, `PipelineContext` (strategy brief, gate errors,
   review feedback), `McpToolResult`
2. Define `ContextItem { source: ContextSource, score: f64, text: String,
   tokens: usize, evictable: bool }`
3. Implement `ContextManager` with:
   - `budget: usize` (total token budget for context)
   - `items: BTreeMap<OrderedFloat<f64>, ContextItem>` (sorted by score)
   - `fn add(&mut self, item: ContextItem)` -- inserts, evicts lowest-scored
     evictable item if budget exceeded
   - `fn render(&self) -> String` -- concatenates items in score order
   - `fn remaining_budget(&self) -> usize`
4. Add `pub mod context_manager;` to `crates/roko-acp/src/lib.rs`

**Acceptance criteria**:
- Unit test: adding items past budget evicts lowest-scored evictable item
- Unit test: non-evictable items (system prompt) are never evicted
- Unit test: `render()` output token estimate equals sum of item tokens

### Task 1.3: Wire ContextManager into ACP Bridge Events

**File**: `crates/roko-acp/src/bridge_events.rs`
**What**: Replace the inline context assembly in `handle_session_prompt()`
with ContextManager-based assembly.

**Steps**:
1. Import `ContextManager` and `ContextItem`
2. In the prompt handling path (after knowledge query), construct a
   `ContextManager` with budget from session config or model's
   `max_context_tokens` (default 100_000)
3. Add knowledge hits as `ContextItem { source: Knowledge, evictable: true }`
4. Add playbook context as `ContextItem { source: Playbook, evictable: true }`
5. Add file context from @-mentions as `ContextItem { source: FileContext,
   evictable: false }` (user-requested context is never evicted)
6. Add conversation history as `ContextItem { source: ConversationHistory,
   evictable: true }` with recency-based scoring
7. Call `manager.render()` to produce the final context string
8. Replace the existing `append_context()` chain with the manager output

**Acceptance criteria**:
- Run ACP session with a prompt that @-mentions 5 files
- Verify file context appears in agent prompt
- Verify knowledge context is truncated when file context consumes most
  of the budget
- Existing ACP unit tests pass

### Task 1.4: Add Context Budget Session Config Option

**File**: `crates/roko-acp/src/session.rs`
**What**: Expose context budget as the 10th session config dropdown so
users can control how much context the agent receives.

**Steps**:
1. Add config option #10 to `build_config_options()`:
   `id: "context_budget"`, `name: "Context Budget"`, `option_type: Select`,
   `options: ["auto", "small (32k)", "medium (64k)", "large (128k)", "max"]`,
   `default: "auto"`
2. In `update_config()`, map the string value to a token count:
   `"auto"` -> model's max_tokens / 2, `"small"` -> 32000,
   `"medium"` -> 64000, `"large"` -> 128000, `"max"` -> model's max_tokens
3. Store the resolved budget in `AcpSession.context_budget: usize`
4. Pass to ContextManager construction in bridge_events

**Acceptance criteria**:
- ACP `session/new` response includes 10 config options (was 9)
- Setting context budget to "small" reduces knowledge items in prompt
- Setting to "max" includes all available context

### Task 1.5: Multi-Turn Context Carry-Forward

**File**: `crates/roko-acp/src/session.rs`
**What**: Track which files the agent touched in prior turns and
automatically include them as context in subsequent turns.

**Steps**:
1. Add `touched_files: Vec<String>` to `AcpSession`
2. After each prompt completes, extract file paths from tool call updates
   (ToolCallKind::Edit, Create, Delete) and append to `touched_files`
3. Deduplicate the list, cap at 20 files
4. In the next prompt's context assembly, add touched files as
   `ContextItem { source: FileContext, score: 0.7, evictable: true }`
5. Score touched files by recency: most recently touched = highest score

**Acceptance criteria**:
- ACP session: first prompt edits `src/lib.rs`, second prompt receives
  `src/lib.rs` content in context without @-mention
- Files from 3+ turns ago are evicted when budget is tight
- `touched_files` does not grow beyond 20 entries

### Task 1.6: Session State Persistence for Resume

**File**: `crates/roko-acp/src/session.rs`
**What**: Persist full session state (history, config, touched files,
workflow run) to disk so sessions can survive process restarts.

**Steps**:
1. Add `fn persist(&self, path: &Path) -> Result<()>` to `AcpSession`
   that serializes to JSON (history, config options, touched_files,
   workflow run if active, mode)
2. Add `fn load(path: &Path) -> Result<AcpSession>` that deserializes
3. In `handle_session_prompt()`, call `session.persist()` after each
   prompt completes
4. In `load_session()` handler, call `AcpSession::load()` when the
   session ID matches a persisted file
5. Store persisted sessions at `.roko/sessions/{session_id}.json`

**Acceptance criteria**:
- Start ACP session, send 3 prompts, restart process
- `session/load` with the old session ID restores history
- Config options are preserved across restart
- Touched files list is preserved

### Task 1.7: Per-Turn Context Usage Tracking

**File**: `crates/roko-acp/src/bridge_events.rs`
**What**: Record per-turn context usage statistics so the learning system
can optimize future context budgets.

**Steps**:
1. Define `ContextUsageRecord { turn_id: String, total_budget: usize,
   knowledge_tokens: usize, playbook_tokens: usize, file_tokens: usize,
   history_tokens: usize, items_evicted: usize, success: bool }`
2. After each prompt completes, construct the record from ContextManager
   statistics
3. Append to `.roko/learn/context-usage.jsonl` via JSONL append
4. In the cascade router observation, include context_budget as a
   feature in the routing context vector

**Acceptance criteria**:
- Run 5 ACP prompts
- `.roko/learn/context-usage.jsonl` has 5 entries
- Each entry has non-zero `total_budget` and accurate source breakdowns

---

## Phase 2: Parallel Agent Execution in ACP Pipeline (6 tasks)

**Objective**: Extend the pure state machine with parallel agent phases
for Full template's multi-reviewer pattern and future wave-level gating.

**Duration estimate**: 4-5 days

### Task 2.1: Add ParallelExecution Phase to Pipeline

**File**: `crates/roko-acp/src/pipeline.rs`
**What**: Extend the state machine with a parallel execution phase that
tracks multiple concurrent agents.

**Steps**:
1. Add `ParallelExecution` variant to `PipelinePhase`:
   ```rust
   ParallelExecution {
       agent_ids: Vec<String>,
       completed: Vec<String>,
       barrier: BarrierCondition,
   }
   ```
2. Add `BarrierCondition` enum: `AllComplete`, `MajorityComplete`,
   `AnyComplete`
3. Add new events to `PipelineEvent`:
   - `ParallelAgentCompleted { agent_id: String, output: String }`
   - `ParallelAgentFailed { agent_id: String, error: String }`
4. Add new action to `PipelineAction`:
   - `SpawnParallelAgents { specs: Vec<ParallelAgentSpec> }`
   where `ParallelAgentSpec { role: String, prompt: String, context: String }`
5. Keep `step()` pure -- the new transitions only update the completed
   list and check the barrier condition

**Acceptance criteria**:
- Unit test: `ParallelExecution` with 3 agents, `AllComplete` barrier,
  completing all 3 transitions to next phase
- Unit test: `MajorityComplete` transitions after 2 of 3
- Unit test: one agent failure in `AllComplete` halts the pipeline

### Task 2.2: Add VerdictMerge Phase

**File**: `crates/roko-acp/src/pipeline.rs`
**What**: Phase that merges outputs from parallel review agents into a
single verdict.

**Steps**:
1. Add `VerdictMerge` variant to `PipelinePhase`:
   ```rust
   VerdictMerge { outputs: Vec<(String, String)> }  // (role, output)
   ```
2. Add `MergeComplete { merged_verdict: String }` to `PipelineEvent`
3. Add `MergeVerdicts { outputs: Vec<(String, String)> }` to `PipelineAction`
4. Transition: `ParallelExecution` -> barrier met -> `VerdictMerge`
5. Transition: `VerdictMerge` + `MergeComplete { verdict: approved }` ->
   `Committing`
6. Transition: `VerdictMerge` + `MergeComplete { verdict: revise }` ->
   `Implementing` (if iterations remain)

**Acceptance criteria**:
- Unit test: Full pipeline with parallel review phase completes:
  `Strategizing -> Implementing -> Gating -> ParallelExecution -> VerdictMerge -> Committing`
- Unit test: merged revise verdict sends back to Implementing

### Task 2.3: Wire Full Template to Use Parallel Review

**File**: `crates/roko-acp/src/pipeline.rs`
**What**: Make the Full template actually spawn Architect + Auditor +
Scribe in parallel for the review phase.

**Steps**:
1. Modify the `Gating -> GatesPassed` transition for `Full` template:
   instead of `SpawnReviewer`, emit `SpawnParallelAgents` with specs for
   Architect (deep review), Auditor (security/correctness), and Scribe
   (documentation check)
2. Set barrier to `AllComplete` (all three must finish)
3. On barrier met, transition to `VerdictMerge`
4. Keep Standard template unchanged (single reviewer)

**Acceptance criteria**:
- Unit test: Full template emits `SpawnParallelAgents` with 3 specs
  after gates pass
- Unit test: Standard template still emits `SpawnReviewer` (single agent)
- Full transition table test covers the parallel path end-to-end

### Task 2.4: Implement Parallel Agent Spawning in Runner

**File**: `crates/roko-acp/src/runner.rs`
**What**: When the pipeline emits `SpawnParallelAgents`, spawn agents
concurrently with `tokio::JoinSet`.

**Steps**:
1. Add `handle_spawn_parallel()` method that creates a `JoinSet`
2. For each `ParallelAgentSpec`, spawn an agent task via the existing
   `spawn_role_agent()` function
3. As each agent completes, feed `ParallelAgentCompleted` or
   `ParallelAgentFailed` back to the pipeline state machine
4. Emit ACP session updates (ToolCall / ToolCallUpdate) for each
   parallel agent's progress
5. Track per-agent cost in `WorkflowRun.total_cost_usd`

**Acceptance criteria**:
- Integration test: spawn 2 mock agents in parallel, both complete
- Verify both agents' ToolCall updates appear in the ACP event stream
- Cost is sum of both agents' costs

### Task 2.5: Implement VerdictMerge in Runner

**File**: `crates/roko-acp/src/runner.rs`
**What**: When the pipeline emits `MergeVerdicts`, combine the parallel
review outputs into a single approve/revise decision.

**Steps**:
1. Add `handle_merge_verdicts()` method
2. Parse each output through `parse_structured_review_verdict()` from
   `roko-gate`
3. Merge strategy: if any reviewer rejects -> revise; if all approve ->
   approve; mixed -> take majority with highest-severity findings
4. Concatenate findings from all reviewers
5. Feed `MergeComplete` with the merged verdict back to the pipeline

**Acceptance criteria**:
- Unit test: 3 approve -> merged approve
- Unit test: 1 reject + 2 approve -> merged revise with reject findings
- Unit test: 2 revise + 1 approve -> merged revise with combined findings

### Task 2.6: Add Parallel Progress to ACP Session Updates

**File**: `crates/roko-acp/src/types.rs`
**What**: Extend session update types to show parallel agent progress.

**Steps**:
1. Add `ParallelProgress` variant to `SessionUpdate`:
   ```rust
   ParallelProgress {
       total_agents: u32,
       completed_agents: u32,
       agent_statuses: Vec<AgentStatus>,
   }
   ```
   where `AgentStatus { role: String, status: ToolCallStatus }`
2. Emit `ParallelProgress` updates from the runner whenever a parallel
   agent completes
3. Add corresponding `PlanEntry` updates showing each parallel agent
   as a sub-step

**Acceptance criteria**:
- ACP client receives `ParallelProgress` updates during parallel execution
- Progress shows correct completed/total counts
- Plan entries show individual agent status

---

## Phase 3: Workflow Template Completion (4 tasks)

**Objective**: Implement the Research, Review-Only, Documentation, and
Custom templates from the planned-but-unbuilt template set.

**Duration estimate**: 3-4 days

### Task 3.1: Add Research Workflow Template

**File**: `crates/roko-acp/src/pipeline.rs`
**What**: Add Research template: Research -> Synthesize -> [Optional Writer].

**Steps**:
1. Add `Research` variant to `WorkflowTemplate`
2. Add `Researching` and `Synthesizing` phases to `PipelinePhase`
3. Add transitions:
   - `Pending + Start` (Research) -> `Researching` + `SpawnResearcher`
   - `Researching + AgentCompleted` -> `Synthesizing` + `SpawnSynthesizer`
   - `Synthesizing + AgentCompleted` -> `Complete` + `Done`
4. Add new actions: `SpawnResearcher { topic }`, `SpawnSynthesizer { research_output }`
5. Update `auto_select()`: prompts containing "research", "investigate",
   "analyze", "explain", "compare" trigger Research template
6. Update `from_config()` to accept `"research"`

**Acceptance criteria**:
- Unit test: Research template flows through Research -> Synthesize -> Complete
- Unit test: `auto_select("research the differences between X and Y")` -> Research
- Existing template tests pass unchanged

### Task 3.2: Add Review-Only Workflow Template

**File**: `crates/roko-acp/src/pipeline.rs`
**What**: Add Review-Only template: Git Diff -> Reviewer -> Verdict.
No code changes, read-only.

**Steps**:
1. Add `ReviewOnly` variant to `WorkflowTemplate`
2. Add transitions:
   - `Pending + Start` (ReviewOnly) -> `Reviewing` + `SpawnReviewer { diff_context }`
   - `Reviewing + ReviewApproved` -> `Complete` + `Done`
   - `Reviewing + ReviewRevise` -> `Complete` + `Done` (report findings, don't fix)
3. `has_strategy()` -> false, `has_review()` -> true
4. Update `auto_select()`: prompts containing "review", "audit", "check"
   without implementation words trigger ReviewOnly
5. Update `from_config()` to accept `"review_only"`

**Acceptance criteria**:
- Unit test: ReviewOnly template never enters Implementing phase
- Unit test: review findings are reported but no implementation spawned
- `auto_select("review the changes in this PR")` -> ReviewOnly

### Task 3.3: Add Documentation Workflow Template

**File**: `crates/roko-acp/src/pipeline.rs`
**What**: Add Documentation template: Changed Files -> Scribe -> Critic ->
[Fix Loop] -> Commit.

**Steps**:
1. Add `Documentation` variant to `WorkflowTemplate`
2. Add `Scribing` and `Critiquing` phases to `PipelinePhase`
3. Add transitions:
   - `Pending + Start` (Documentation) -> `Scribing` + `SpawnScribe`
   - `Scribing + AgentCompleted` -> `Critiquing` + `SpawnCritic`
   - `Critiquing + ReviewApproved` -> `Committing` + `Commit`
   - `Critiquing + ReviewRevise` -> `Scribing` + `SpawnScribe` (with feedback)
4. Add new actions: `SpawnScribe { files, context }`, `SpawnCritic { docs_diff }`
5. Update `auto_select()`: prompts with "document", "docs", "README",
   "changelog" trigger Documentation template

**Acceptance criteria**:
- Unit test: Documentation template flows Scribe -> Critic -> Commit
- Unit test: critic rejection loops back to Scribe
- Scribe agent only has write access to documentation files

### Task 3.4: Add Custom Workflow Template Parser

**File**: `crates/roko-acp/src/pipeline.rs`
**What**: Allow users to define custom step sequences in roko.toml.

**Steps**:
1. Add `Custom { steps: Vec<CustomStep> }` variant to `WorkflowTemplate`
2. Define `CustomStep { phase: String, role: String, config: Value }`
3. Add `fn from_toml(table: &toml::Table) -> Result<WorkflowTemplate>`
   that parses:
   ```toml
   [[workflow.steps]]
   phase = "implement"
   role = "implementer"

   [[workflow.steps]]
   phase = "gate"

   [[workflow.steps]]
   phase = "review"
   role = "quick_reviewer"
   ```
4. Validate step sequence: must contain at least "implement" and "gate"
5. Map custom steps to `PipelinePhase` transitions dynamically

**Acceptance criteria**:
- Parse a 4-step custom workflow from TOML
- Reject TOML missing "implement" step
- Custom workflow executes phases in defined order

---

## Phase 4: MCP Federation (5 tasks)

**Objective**: Enable MCP servers to discover and query each other via a
local registry, enabling compound queries like "symbols changed in PR #42."

**Duration estimate**: 3-4 days

### Task 4.1: Build MCP Server Registry

**File**: `crates/roko-mcp-stdio/src/registry.rs` (new file)
**What**: Local registry where MCP servers register their capabilities
and can discover peers.

**Steps**:
1. Define `McpServerEntry { name: String, pid: u32, tools: Vec<String>,
   socket_path: PathBuf, registered_at: DateTime<Utc> }`
2. Define `McpRegistry` backed by a file at `.roko/mcp-registry.json`
3. Implement `register(entry: McpServerEntry) -> Result<()>` with
   file-locking for concurrent writes
4. Implement `discover(tool_name: &str) -> Option<McpServerEntry>`
   that finds the server exposing a given tool
5. Implement `health_check()` that removes stale entries (pid not running)
6. Add `pub mod registry;` to `crates/roko-mcp-stdio/src/lib.rs`

**Acceptance criteria**:
- Unit test: register 3 servers, discover by tool name
- Unit test: health check removes entry for non-existent pid
- File locking prevents corruption under concurrent writes

### Task 4.2: Add Cross-Server Tool Call Client

**File**: `crates/roko-mcp-stdio/src/federation.rs` (new file)
**What**: Client that routes tool calls to peer MCP servers via the
registry.

**Steps**:
1. Define `FederatedClient { registry: McpRegistry }`
2. Implement `call_tool(server_name: &str, tool: &str, args: Value)
   -> Result<Value>` that:
   - Discovers the server via registry
   - Connects via Unix domain socket (or falls back to spawning stdio)
   - Sends JSON-RPC request and awaits response
   - Applies timeout (default 30s) and circuit breaker (3 failures -> open)
3. Implement `call_tool_by_name(tool: &str, args: Value) -> Result<Value>`
   that auto-discovers which server provides the tool
4. Add `pub mod federation;` to `crates/roko-mcp-stdio/src/lib.rs`

**Acceptance criteria**:
- Integration test: server A calls tool on server B via federation
- Timeout triggers after 30s with clear error message
- Circuit breaker opens after 3 consecutive failures

### Task 4.3: Add Federation to roko-mcp-code

**File**: `crates/roko-mcp-code/src/lib.rs`
**What**: Enable code intelligence server to query GitHub MCP for PR
context when analyzing symbols.

**Steps**:
1. Add optional `FederatedClient` to the code server's state
2. Add `github_enriched_context` tool that:
   - Takes `{ symbol_name, pr_number }` args
   - Calls `symbol_lookup` locally for code context
   - Calls `github_get_pr` via federation for PR diff
   - Merges results: "Symbol X was modified in PR #N, here is the change"
3. Register the code server with the MCP registry on startup
4. If federation client is not available, the tool returns code-only results

**Acceptance criteria**:
- When GitHub MCP is running: `github_enriched_context` returns merged result
- When GitHub MCP is not running: returns code-only result with warning
- New tool appears in `tools/list` response

### Task 4.4: Add Federation to roko-mcp-github

**File**: `crates/roko-mcp-github/src/main.rs`
**What**: Enable GitHub MCP to query code intelligence for impact analysis.

**Steps**:
1. Add optional `FederatedClient` to the GitHub server's state
2. Add `pr_impact_analysis` tool that:
   - Takes `{ pr_number }` args
   - Calls `github_get_pr` locally for the PR diff
   - Extracts changed function names from the diff
   - Calls `call_graph` via federation for each changed function
   - Returns: affected functions, call chains, test coverage gaps
3. Register the GitHub server with the MCP registry on startup

**Acceptance criteria**:
- `pr_impact_analysis` returns call graph data for changed functions
- When code MCP is not running, returns diff-only analysis
- Tool handles large PRs (>50 files) without timeout

### Task 4.5: Add Federation Config to roko.toml

**File**: `crates/roko-core/src/config/schema.rs`
**What**: Configuration for MCP federation behavior.

**Steps**:
1. Add `[mcp.federation]` section to config schema:
   ```toml
   [mcp.federation]
   enabled = true
   registry_path = ".roko/mcp-registry.json"
   timeout_ms = 30000
   circuit_breaker_threshold = 3
   ```
2. Parse in config loader
3. Pass to MCP servers via environment variables or command-line args
4. When `enabled = false`, federation client is not constructed

**Acceptance criteria**:
- Config section parses correctly
- `enabled = false` disables all federation features
- Default values work when section is omitted

---

## Phase 5: Learning-Informed MCP (4 tasks)

**Objective**: Record MCP tool call outcomes, feed the learning system,
and use bandit-based selection to improve tool strategy over time.

**Duration estimate**: 2-3 days

### Task 5.1: Add ToolEffectiveness Bandit

**File**: `crates/roko-learn/src/tool_effectiveness.rs` (new file)
**What**: Bandit-based tracker that learns which MCP tool strategies
(keyword vs structural vs hybrid) work best for different query types.

**Steps**:
1. Define `ToolCallRecord { tool: String, strategy: Option<String>,
   query: String, results_count: usize, was_useful: bool,
   latency_ms: u64, timestamp: DateTime<Utc> }`
2. Define `ToolEffectivenessBandit` with per-tool-per-strategy Thompson
   sampling (reuse the bandit math from `CascadeRouter`)
3. Implement `observe(record: &ToolCallRecord)` that updates the bandit
4. Implement `recommend_strategy(tool: &str, query: &str) -> String`
   that samples from the posterior
5. Persist to `.roko/learn/tool-effectiveness.json`
6. Add `pub mod tool_effectiveness;` to `crates/roko-learn/src/lib.rs`

**Acceptance criteria**:
- Unit test: after 10 positive observations for "hybrid", it is recommended
  more often than "keyword" (which has 2 positive)
- Persistence round-trip preserves bandit state
- Empty state defaults to uniform random

### Task 5.2: Record MCP Tool Calls in Episode Log

**File**: `crates/roko-acp/src/bridge_events.rs`
**What**: After each ACP dispatch, record the MCP tool calls that occurred
during the agent's work.

**Steps**:
1. Track tool call events during `stream_events_to_editor()`: when
   `CognitiveEvent::ToolCallStart` fires with a tool name matching an MCP
   tool, record `{ tool, args, start_time }`
2. When `CognitiveEvent::ToolCallComplete` fires for that tool, compute
   latency and record result quality (heuristic: non-empty result =
   potentially useful)
3. After dispatch completes, append tool records to the episode:
   add `tool_calls: Vec<ToolCallRecord>` field to `Episode`
4. Feed each record to `ToolEffectivenessBandit.observe()`

**Acceptance criteria**:
- Episode entries in `.roko/episodes.jsonl` include `tool_calls` array
- Each tool call record has non-zero `latency_ms`
- Bandit file is updated after each dispatch

### Task 5.3: Wire Learned Strategy into roko-mcp-code

**File**: `crates/roko-mcp-code/src/lib.rs`
**What**: When `search_code` is called with `strategy: "auto"`, consult
the ToolEffectivenessBandit for the optimal strategy.

**Steps**:
1. Load `ToolEffectivenessBandit` from `.roko/learn/tool-effectiveness.json`
   at server startup
2. When `search_code` is called with `strategy: "auto"` (or strategy
   omitted), call `bandit.recommend_strategy("search_code", &query)`
3. Use the recommended strategy for the search
4. After results are returned, the ACP bridge records the outcome (Task 5.2)
5. Fall back to "hybrid" when bandit has fewer than 10 observations

**Acceptance criteria**:
- First 10 calls use "hybrid" (cold start)
- After training data accumulates, "auto" selects learned-best strategy
- Explicit strategy parameter ("keyword") overrides bandit recommendation

### Task 5.4: Add Tool Effectiveness to Learning Dashboard

**File**: `crates/roko-cli/src/learning_helpers.rs`
**What**: Expose tool effectiveness data via `roko learn tools`.

**Steps**:
1. Add `"tools"` subcommand to the learn command that loads
   `ToolEffectivenessBandit` and prints:
   - Per-tool success rates by strategy
   - Recommended strategy for each tool
   - Total observations count
2. Format as table:
   ```
   Tool           Strategy    Success   Observations   Recommended
   search_code    keyword     45%       20             no
   search_code    hybrid      82%       45             yes
   search_code    structural  60%       15             no
   ```

**Acceptance criteria**:
- `roko learn tools` outputs a readable table
- Shows "no data" when bandit file does not exist
- Handles empty strategy gracefully

---

## Phase 6: Agent-to-Agent Communication & Permission Bridge (5 tasks)

**Objective**: Enable agents within a pipeline to share intermediate
results and establish a permission bridge for ACP bidirectional requests.

**Duration estimate**: 3-4 days

### Task 6.1: Build SharedContextStore for Cross-Agent Access

**File**: `crates/roko-acp/src/shared_context.rs` (new file)
**What**: Thread-safe store that parallel agents can read/write during
execution, enabling real-time context sharing.

**Steps**:
1. Define `SharedContextStore` with `Arc<RwLock<HashMap<String, ContextEntry>>>`
2. `ContextEntry { author_role: String, key: String, value: String,
   timestamp: Instant }`
3. Implement `publish(role: &str, key: &str, value: &str)` -- writes entry
4. Implement `query(key_prefix: &str) -> Vec<ContextEntry>` -- reads
   entries matching prefix
5. Implement `snapshot() -> String` -- renders all entries as markdown
   for injection into agent prompts
6. Add `pub mod shared_context;` to `crates/roko-acp/src/lib.rs`

**Acceptance criteria**:
- Unit test: two concurrent writers, reader sees both entries
- Snapshot renders entries sorted by timestamp
- Empty store returns empty string from snapshot

### Task 6.2: Inject Shared Context into Parallel Agents

**File**: `crates/roko-acp/src/runner.rs`
**What**: When agents in a parallel phase produce intermediate results,
make them available to still-running peers.

**Steps**:
1. Create `SharedContextStore` per parallel execution phase
2. When `ParallelAgentCompleted` fires, publish the agent's key findings
   to the store: `store.publish(role, "findings", &output_summary)`
3. For long-running agents (>30s), inject a mid-turn context refresh:
   if the runner can communicate with the agent (via MCP sampling or
   context injection), push `store.snapshot()` as additional context
4. When all agents complete, include the full snapshot in the
   `MergeVerdicts` action context

**Acceptance criteria**:
- When Architect finishes before Auditor, Auditor's merge context
  includes Architect's findings
- Shared context appears in the VerdictMerge input
- Store is garbage-collected after the parallel phase ends

### Task 6.3: Wire ACP Permission Bridge

**File**: `crates/roko-acp/src/handler.rs`
**What**: Implement the agent-to-editor permission request flow so agents
can ask user approval before destructive actions.

**Steps**:
1. When the runner encounters a destructive action (delete file, run
   dangerous command), construct a `session/request_permission` request
2. Send via `transport.send_request()` with:
   `{ method: "session/request_permission", params: { title, description,
   permissions: [{ name, description, destructive: bool }] } }`
3. Await the editor's response (approve/deny) via the pending request
   registry in `StdioTransport`
4. If approved, proceed with the action
5. If denied, feed `AgentFailed { error: "Permission denied by user" }`
   to the pipeline

**Acceptance criteria**:
- Agent requesting file deletion triggers permission prompt
- User approval allows the action to proceed
- User denial feeds back as agent failure with clear reason
- Non-destructive actions skip the permission check

### Task 6.4: Wire ACP Elicitation for Structured Input

**File**: `crates/roko-acp/src/handler.rs`
**What**: Implement `elicitation/create` so agents can present structured
forms to users for disambiguation.

**Steps**:
1. When the pipeline's strategist needs user input on ambiguous prompts,
   construct an elicitation request:
   `{ method: "elicitation/create", params: { title, description,
   inputs: [{ id, label, type, options, default }] } }`
2. Send via `transport.send_request()` and await response
3. Parse the user's selections from the response
4. Feed into the pipeline as additional context for the implementer

**Acceptance criteria**:
- Strategist can request user to choose between 2 approaches
- User selection appears in implementer's context
- Timeout (60s) falls back to default selection

### Task 6.5: File Change Notifications to Editor

**File**: `crates/roko-acp/src/runner.rs`
**What**: After agents modify files, send `session/update` notifications
with `FileChangeNotification` so the editor can refresh its view.

**Steps**:
1. After each agent completes, collect the list of files changed
   (from git diff or tool call tracking)
2. Emit `SessionUpdate::FileChange(FileChangeNotification { path, kind })`
   for each changed file
3. `FileChangeType`: `Created`, `Modified`, `Deleted`
4. Batch notifications to avoid flooding (max 50 per agent completion)

**Acceptance criteria**:
- ACP client receives file change notifications after agent edits
- Notifications include correct change type (created vs modified)
- Large changesets are capped at 50 notifications

---

## Phase 7: A2A Bridge (4 tasks)

**Objective**: Bridge Roko's internal agent system to Google's Agent-to-Agent
Protocol for cross-organization agent collaboration.

**Duration estimate**: 4-5 days

### Task 7.1: Implement A2A Protocol Types

**File**: `crates/roko-a2a/src/types.rs` (new crate)
**What**: Core A2A protocol types: Agent Card, Task, Message, Artifact.

**Steps**:
1. Create `crates/roko-a2a/` with `Cargo.toml` (deps: serde, serde_json,
   chrono, url, uuid)
2. Define `AgentCard { name, url, description, version, capabilities,
   skills, default_input_modes, default_output_modes, authentication }`
3. Define `A2ATask { id, session_id, status, messages, artifacts, metadata }`
4. Define `TaskStatus`: Submitted, Working, InputRequired, Completed,
   Failed, Canceled
5. Define `A2AMessage { role, parts }` and `A2APart` (Text, File, Data)
6. Define `A2AArtifact { name, description, parts, index }`
7. Add crate to workspace `Cargo.toml`

**Acceptance criteria**:
- All types derive Serialize/Deserialize
- Agent Card JSON matches A2A spec schema
- `cargo test -p roko-a2a` passes with serde round-trip tests

### Task 7.2: Publish Roko Agent Card

**File**: `crates/roko-a2a/src/agent_card.rs`
**What**: Generate and serve Roko's Agent Card describing its capabilities.

**Steps**:
1. Implement `fn build_agent_card(config: &RokoConfig) -> AgentCard` that
   declares capabilities:
   - Skill: "code_implementation" (languages from roko.toml)
   - Skill: "code_review"
   - Skill: "research_and_analysis"
   - Skill: "plan_generation"
2. Support `authentication.schemes: [{ scheme: "bearer" }]`
3. Default input/output modes: `["text/plain", "application/json"]`
4. Add `GET /.well-known/agent.json` route to `roko-serve`

**Acceptance criteria**:
- `GET /.well-known/agent.json` returns valid Agent Card JSON
- Card includes skills matching Roko's configured capabilities
- Authentication section matches configured auth mode

### Task 7.3: Implement A2A Task Reception

**File**: `crates/roko-serve/src/routes/a2a.rs` (new file)
**What**: HTTP endpoints for receiving and processing A2A tasks from
external agents.

**Steps**:
1. Add `POST /a2a/tasks/send` -- receives an A2A task, maps to internal
   pipeline execution
2. Add `GET /a2a/tasks/:id` -- returns task status
3. Add `POST /a2a/tasks/:id/cancel` -- cancels a running task
4. Map A2A task to internal `WorkflowRun`:
   - Extract prompt from task messages
   - Select template based on task metadata or auto-select
   - Execute via the ACP pipeline runner
5. Map internal pipeline completion back to A2A task status
6. Add routes to `build_router()` in `crates/roko-serve/src/routes/mod.rs`

**Acceptance criteria**:
- External agent can submit a coding task via `POST /a2a/tasks/send`
- Task status is retrievable via `GET /a2a/tasks/:id`
- Pipeline completion updates A2A task status to Completed
- Pipeline failure updates A2A task status to Failed with error details

### Task 7.4: Add DelegateExternal Action to Pipeline

**File**: `crates/roko-acp/src/pipeline.rs`
**What**: Enable the pipeline to delegate sub-tasks to external agents
via A2A.

**Steps**:
1. Add `DelegateExternal { task: ExternalTaskSpec }` to `PipelineAction`
   where `ExternalTaskSpec { agent_url: String, skill: String, prompt: String }`
2. Add `ExternalDelegated` phase to `PipelinePhase`
3. Add `ExternalCompleted { output: String }` and
   `ExternalFailed { error: String }` to `PipelineEvent`
4. Transitions:
   - Action `DelegateExternal` -> phase `ExternalDelegated`
   - `ExternalCompleted` -> resume pipeline (e.g., back to Gating)
   - `ExternalFailed` -> `Halted` or retry
5. The runner implements delegation by calling the A2A endpoint on the
   external agent

**Acceptance criteria**:
- Unit test: pipeline transitions through ExternalDelegated phase
- Unit test: external failure halts pipeline with clear reason
- Pipeline state machine remains pure (no I/O in step())

---

## Phase 8: Tracker Integrations (5 tasks)

**Objective**: Bidirectional sync with external project trackers (GitHub
Issues, Sentry, Linear) so Roko tasks can originate from and report back
to external systems.

**Duration estimate**: 3-4 days

### Task 8.1: Define TrackerAdapter Trait

**File**: `crates/roko-core/src/tracker.rs` (new file)
**What**: Generic trait for bidirectional task sync with external systems.

**Steps**:
1. Define the trait:
   ```rust
   #[async_trait]
   pub trait TrackerAdapter: Send + Sync {
       fn kind(&self) -> &str;
       async fn fetch_active(&self) -> Result<Vec<ExternalTask>>;
       async fn update_state(&self, id: &str, state: &str,
           comment: Option<&str>) -> Result<()>;
       async fn create_task(&self, spec: &TaskSpec) -> Result<String>;
       fn state_mapping(&self) -> &StateMapping;
   }
   ```
2. Define `ExternalTask { id, title, description, state, labels, url,
   assignee, metadata }`
3. Define `StateMapping { pending: String, in_progress: String,
   completed: String, failed: String }` that maps Roko states to
   tracker-specific states
4. Define `TaskSpec { title, description, labels, assignee }`
5. Add `pub mod tracker;` to `crates/roko-core/src/lib.rs`

**Acceptance criteria**:
- Trait compiles and is object-safe (`dyn TrackerAdapter`)
- Mock implementation passes basic unit tests
- State mapping covers all Roko task states

### Task 8.2: Implement GitHub Issues TrackerAdapter

**File**: `crates/roko-cli/src/tracker/github.rs` (new file)
**What**: TrackerAdapter that syncs with GitHub Issues.

**Steps**:
1. Implement `GithubTrackerAdapter { owner, repo, token, state_mapping }`
2. `fetch_active()` calls GitHub API (or `gh` CLI) to list open issues
   with label `roko` or configurable label filter
3. `update_state()` adds a comment and optionally closes the issue
4. `create_task()` creates a GitHub issue with labels
5. State mapping defaults: `pending -> open`, `in_progress -> open` (with
   "in progress" label), `completed -> closed`, `failed -> open` (with
   "failed" label)
6. On task completion, post comment: "Completed by Roko. PR: #N"

**Acceptance criteria**:
- `fetch_active()` returns issues from a test repo
- `update_state("closed")` closes the issue and adds comment
- Label-based state tracking works correctly
- Rate limiting handled (reuse roko-mcp-github's retry logic)

### Task 8.3: Implement Sentry TrackerAdapter

**File**: `crates/roko-cli/src/tracker/sentry.rs` (new file)
**What**: TrackerAdapter that ingests Sentry errors as fix tasks.

**Steps**:
1. Implement `SentryTrackerAdapter { org, project, token, state_mapping }`
2. `fetch_active()` calls Sentry API to list unresolved issues with
   assignee or tag filter
3. For each Sentry issue, construct `ExternalTask` with:
   - `description` = stack trace + affected files + error count
   - `metadata` includes: error frequency, first/last seen, affected users
4. `update_state("resolved")` resolves the Sentry issue
5. `create_task()` is a no-op (Sentry issues are external-only)

**Acceptance criteria**:
- `fetch_active()` returns Sentry issues with stack traces
- Stack trace is extracted into task description
- `update_state("resolved")` resolves the issue in Sentry

### Task 8.4: Implement Linear TrackerAdapter

**File**: `crates/roko-cli/src/tracker/linear.rs` (new file)
**What**: Bidirectional sync with Linear issues.

**Steps**:
1. Implement `LinearTrackerAdapter { api_key, team_id, state_mapping }`
2. `fetch_active()` calls Linear GraphQL API to list issues in active
   states with team filter
3. `update_state()` transitions the Linear issue to the mapped state
4. `create_task()` creates a new Linear issue
5. State mapping defaults: `pending -> Backlog`, `in_progress -> In Progress`,
   `completed -> Done`, `failed -> Backlog` (with comment)
6. Poll-based sync with configurable interval (default: 60s)

**Acceptance criteria**:
- GraphQL query fetches active issues
- State transitions map correctly between Roko and Linear
- New tasks created in Linear appear with correct team assignment

### Task 8.5: Wire TrackerAdapter into Plan Execution

**File**: `crates/roko-cli/src/orchestrate.rs` (or `crates/roko-acp/src/runner.rs`)
**What**: When plan execution completes a task, update the corresponding
external tracker.

**Steps**:
1. Load tracker config from `roko.toml`:
   ```toml
   [tracker]
   kind = "github"  # or "linear", "sentry", "none"
   auto_sync = true

   [tracker.github]
   owner = "org"
   repo = "repo"
   label_filter = "roko"
   ```
2. Construct the appropriate `TrackerAdapter` from config
3. After each task completes, call `adapter.update_state()` with the
   new state and a comment summarizing the result
4. After plan completion, post a summary comment to the originating issue
5. On `roko plan run --from-tracker`, call `adapter.fetch_active()` to
   populate the plan from external tasks

**Acceptance criteria**:
- Task completion updates GitHub issue with comment
- `--from-tracker` creates plan tasks from GitHub issues
- `auto_sync = false` disables automatic updates
- Missing tracker config gracefully falls back to no-op

---

## Dependency Graph

```
Phase 1 (Context)
  |
  +---> Phase 2 (Parallel Agents) -- depends on ContextManager from Phase 1
  |         |
  |         +---> Phase 3 (Templates) -- uses parallel execution from Phase 2
  |
  +---> Phase 5 (Learning MCP) -- depends on context tracking from Phase 1
  |
  +---> Phase 6 (A2A Comm) -- depends on SharedContext from Phase 1

Phase 4 (MCP Federation) -- independent, can run in parallel with Phase 2-3

Phase 7 (A2A Bridge) -- independent, can run in parallel with Phase 2-5

Phase 8 (Trackers) -- independent, can run in parallel with all other phases
```

Phases 4, 7, and 8 have no dependencies on other phases and can be
executed in any order. Phases 2 and 3 depend on Phase 1. Phase 5 depends
on Phase 1. Phase 6 depends on Phases 1 and 2.

---

## Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|------------|
| Parallel agent spawning increases cost | High | Gate pass rate threshold before enabling parallelism; budget caps per parallel phase |
| MCP federation adds latency | Medium | Circuit breaker + timeout; federation is optional and disabled by default |
| A2A protocol is still evolving (pre-1.0) | Medium | Implement minimal spec surface; Agent Card + Task send/status only |
| ContextManager over-evicts useful context | Medium | Track eviction-vs-success correlation in context usage log; tune scoring |
| Custom templates allow invalid pipelines | Low | Validate step sequences at parse time; require "implement" + "gate" minimum |
| Tracker APIs rate limit aggressively | Medium | Exponential backoff; batch updates; configurable poll interval |

---

## Summary

40 tasks across 8 phases. Estimated total: 26-34 days of agent work.

| Phase | Tasks | Estimate | Dependencies |
|-------|-------|----------|-------------|
| 1. Context Management | 7 | 4-5 days | None |
| 2. Parallel Agents | 6 | 4-5 days | Phase 1 |
| 3. Workflow Templates | 4 | 3-4 days | Phase 2 |
| 4. MCP Federation | 5 | 3-4 days | None |
| 5. Learning-Informed MCP | 4 | 2-3 days | Phase 1 |
| 6. Agent Communication | 5 | 3-4 days | Phase 1, 2 |
| 7. A2A Bridge | 4 | 4-5 days | None |
| 8. Tracker Integrations | 5 | 3-4 days | None |
