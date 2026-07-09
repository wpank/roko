# ACP & MCP Protocol Stack: Task Breakdown

> Progressive context management, parallel agent execution, workflow template
> completion, MCP federation, learning-informed tool selection, agent-to-agent
> communication, A2A bridge, and tracker integrations. 40 tasks across 8 phases.
>
> Sources: `impl/09-ACP-MCP.md`, `12-ACP-MCP-DEEP-DIVE.md`,
> `02-ACP-AND-WORKFLOW-PATTERNS.md`, codebase analysis

---

## Overview

The ACP subsystem (`crates/roko-acp/`, ~7,500 LOC, 12 files) implements a full
ACP agent server with a pure state machine pipeline. The MCP subsystem spans 5
crates (`roko-mcp-code`, `roko-mcp-github`, `roko-mcp-slack`, `roko-mcp-scripts`,
`roko-mcp-stdio`) providing tool servers over stdio JSON-RPC 2.0.

The ACP pipeline's pure `step()` function is the correct architecture. All tasks
extend this pattern rather than adding side effects. The MCP servers are isolated
subprocesses with no inter-server communication. All tasks add federation and
learning capabilities without breaking the standalone mode.

**Current state**:

| Component | Location | LOC | Status |
|---|---|---|---|
| ACP handler + transport | `crates/roko-acp/src/{handler,transport}.rs` | ~530 | Wired |
| ACP session + config | `crates/roko-acp/src/{session,config}.rs` | ~1,365 | Wired |
| ACP bridge_events | `crates/roko-acp/src/bridge_events.rs` | ~2,800 | Wired |
| ACP pipeline (pure SM) | `crates/roko-acp/src/pipeline.rs` | ~540 | Wired (3 templates: Express/Standard/Full) |
| ACP runner (side effects) | `crates/roko-acp/src/runner.rs` | ~700 | Wired |
| ACP knowledge | `crates/roko-acp/src/knowledge.rs` | ~412 | Wired (unbounded, no budget) |
| ACP workflow wrapper | `crates/roko-acp/src/workflow.rs` | ~160 | Wired |
| ACP adapter | `crates/roko-acp/src/acp_adapter.rs` | ~200 | Wired |
| MCP shared transport | `crates/roko-mcp-stdio/src/lib.rs` | ~252 | Wired (sync only) |
| MCP code intelligence | `crates/roko-mcp-code/src/lib.rs` | ~1,500 | Wired (12 tools) |
| MCP GitHub | `crates/roko-mcp-github/src/main.rs` | ~800 | Wired (14+ tools) |
| MCP Slack | `crates/roko-mcp-slack/src/main.rs` | ~600 | Wired (9 tools) |
| MCP Scripts | `crates/roko-mcp-scripts/src/main.rs` | ~400 | Wired (2 tools) |

**Key gaps**:
1. Context assembly is all-or-nothing; no budget, no eviction, no mid-turn refresh
2. Pipeline only supports serial agents; Full template's parallel review is unimplemented
3. Only 3 of 8 planned workflow templates exist
4. MCP servers are isolated; no federation, no cross-server queries
5. MCP tool call outcomes are not recorded or fed to the learning system
6. No A2A bridge for external agent collaboration
7. No external tracker integration (GitHub Issues, Sentry, Linear)

**Target state**: Budget-aware progressive context, parallel agent phases with
barrier synchronization, 7 workflow templates, federated MCP servers with
cross-server tool calls, bandit-learned tool strategy selection, A2A agent card
and task reception, and bidirectional tracker sync.

---

## Anti-Patterns to Remove

| ID | Anti-Pattern | Where | Severity |
|---|---|---|---|
| AP-UNBOUNDED | Knowledge query returns unbounded results with no token budget | `crates/roko-acp/src/knowledge.rs:65-82` -- `query_dispatch_knowledge()` hardcodes top-5 hits/top-3 playbooks regardless of context window | High |
| AP-ALLORNONE | Context is gathered once at prompt time with no refresh, narrowing, or eviction | `crates/roko-acp/src/bridge_events.rs:738-741` -- inline `append_context()` chain | High |
| AP-SERIAL | Full template declares parallel review but actually spawns single reviewer | `crates/roko-acp/src/pipeline.rs:288-293` -- `Gating + GatesPassed` always emits `SpawnReviewer` (single) | High |
| AP-3TEMPLATES | Only Express/Standard/Full implemented; Research/ReviewOnly/Documentation/Custom planned but missing | `crates/roko-acp/src/pipeline.rs:97-104` -- `WorkflowTemplate` enum has 3 variants | Medium |
| AP-DUPETOKEN | `estimate_tokens()` reimplemented 6 times across crates with slightly different logic | `roko-compose/src/prompt.rs:24`, `roko-compose/src/compaction.rs:239`, `roko-index/src/workspace.rs:1548`, `roko-core/src/query.rs:167`, `roko-cli/src/bench.rs:932`, `roko-cli/src/dispatch/prompt_builder.rs:843` | Medium |
| AP-MCPSYNC | MCP transport is synchronous only; blocks on long operations, no streaming, no Tasks primitive | `crates/roko-mcp-stdio/src/lib.rs:110` -- `serve_stdio()` uses blocking `BufRead::lines()` | Medium |
| AP-NOLEARN | MCP tool call outcomes not recorded; learning system has no data on tool effectiveness | `crates/roko-acp/src/bridge_events.rs` -- tool calls stream through but results are not persisted to episodes | Medium |
| AP-ISOLATED | MCP servers cannot discover or query each other; no federation registry | Each MCP crate is a standalone binary with no peer awareness | Medium |
| AP-ENVAUTH | MCP authentication via environment variables only; no OAuth, no token refresh, no multi-tenant | `GITHUB_TOKEN`, `SLACK_BOT_TOKEN` in MCP server `main.rs` files | Low |
| AP-NOCARRY | Session does not track files touched by agents; subsequent turns lack continuity | `crates/roko-acp/src/session.rs` -- `AcpSession` has `history` but no `touched_files` tracking | Medium |

---

## Phase 1: Progressive Context Management

Everything downstream depends on budget-aware context. Without this, parallel
agents will blow context windows, knowledge injection wastes tokens, and
learning has no signal about what context was useful.

### Task 9.1: Consolidate estimate_tokens into roko-core
**Priority**: P0
**Estimated Effort**: 2 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/query.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/prompt.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/compaction.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-index/src/workspace.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/bench.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch/prompt_builder.rs`
**Depends On**: none

#### Context
`estimate_tokens` is reimplemented 6 times across the workspace (AP-DUPETOKEN). The canonical version is `Budget::estimate_tokens(bytes: usize) -> usize` at `crates/roko-core/src/query.rs:167` which divides by 4 and rounds up. Other versions do `text.len() / 4`, `text.chars().count().div_ceil(4)`, or `(text.len() as u64 / 4).max(1)`. These are all trying to do the same thing with minor variations.

#### Implementation Steps
1. In `crates/roko-core/src/query.rs`, add a standalone public function alongside the existing `Budget::estimate_tokens`:
   ```rust
   /// Estimate token count from text using 4-chars-per-token heuristic.
   pub const fn estimate_tokens_for_text(text: &str) -> usize {
       text.len().div_ceil(4)
   }
   ```
2. Re-export from `crates/roko-core/src/lib.rs`: `pub use query::estimate_tokens_for_text;`
3. Replace each of the 5 other `estimate_tokens` implementations to call `roko_core::estimate_tokens_for_text()`, adjusting signatures where needed (e.g., `roko-cli/src/bench.rs` returns `u64` -- cast the result).
4. For `roko-compose/src/compaction.rs:239` which operates on `&[ChatMessage]`, keep the wrapper that serializes and then calls the core function.

#### Design Guidance
Use `text.len()` (bytes) not `text.chars().count()` since the 4-chars heuristic is calibrated on byte length. This matches the canonical `Budget::estimate_tokens` which takes `bytes: usize`. The function must be `const fn` for compile-time contexts.

#### Verification Criteria
- [ ] `cargo check --workspace` compiles
- [ ] `cargo test --workspace` passes (existing estimate_tokens tests still pass)
- [ ] `grep -rn 'fn estimate_tokens' crates/ --include='*.rs' | grep -v target/ | grep -v test` shows at most 2 definitions (the core one and the ChatMessage wrapper)

---

### Task 9.2: Add TokenBudget Struct and Budget-Aware Knowledge Query
**Priority**: P0
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/knowledge.rs`
**Depends On**: Task 9.1

#### Context
`query_dispatch_knowledge()` at `crates/roko-acp/src/knowledge.rs:65-82` returns unbounded results: hardcoded top-5 knowledge hits and top-3 playbooks regardless of the model's context window size. `render_context_body()` and `render_playbook_context()` render everything they receive with no truncation. The existing `ContextAssembler` in `crates/roko-neuro/src/context.rs:442` has a `max_context_tokens` field but is not used in the ACP path -- it is used by the orchestrate.rs path.

#### Implementation Steps
1. Add `TokenBudget` struct to `knowledge.rs`:
   ```rust
   #[derive(Debug, Clone)]
   pub(crate) struct TokenBudget {
       pub total: usize,
       pub system_prompt: usize,
       pub history: usize,
       pub knowledge: usize,
       pub file_context: usize,
       pub tool_results: usize,
   }

   impl TokenBudget {
       pub fn from_total(total: usize) -> Self {
           // Allocate: 15% system prompt, 30% history, 20% knowledge,
           // 25% file context, 10% tool results
           Self {
               total,
               system_prompt: total * 15 / 100,
               history: total * 30 / 100,
               knowledge: total * 20 / 100,
               file_context: total * 25 / 100,
               tool_results: total * 10 / 100,
           }
       }
   }
   ```
2. Add `budget: usize` parameter to `query_dispatch_knowledge()`:
   ```rust
   pub(crate) async fn query_dispatch_knowledge(
       workdir: &Path,
       prompt: &str,
       budget: usize,
   ) -> DispatchKnowledge
   ```
3. In `render_context_body()`, track cumulative token count. Stop adding knowledge hits when the running total exceeds the budget. Order by score (highest first, which is already the case).
4. In `render_playbook_context()`, allocate per-playbook budget as `budget / playbooks.len()`. Truncate step lists when budget exceeded.
5. Add `tokens_used: usize` field to `DispatchKnowledge` to report actual usage.
6. Update all callers of `query_dispatch_knowledge()` in `bridge_events.rs` (3 call sites at lines 741, 2003, 2045) to pass a default budget of `20_000` tokens (approximately 20% of a 100K context window).

#### Verification Criteria
- [ ] Existing `card_and_context_include_results` test passes unchanged
- [ ] Existing `missing_stores_return_empty_results` test passes unchanged
- [ ] New unit test: `query_dispatch_knowledge` with `budget = 500` returns fewer items than with `budget = 20_000`
- [ ] New unit test: `render_context_body` with 10 large knowledge hits and budget 200 truncates to 2-3 hits

---

### Task 9.3: Build ContextManager with Priority Queue and Eviction
**Priority**: P0
**Estimated Effort**: 6 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/context_manager.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/lib.rs`
**Depends On**: Task 9.2

#### Context
The existing `ContextAssembler` in `crates/roko-neuro/src/context.rs:442` is tightly coupled to the orchestrate.rs path and operates on `KnowledgeStore` + `EpisodeStore` directly. It uses attention scoring, diminishing returns, novelty penalties, and contrarian retrieval. It has the right ideas but is not suitable for direct use in the ACP path because: (a) it depends on `roko-neuro` internals not exported for ACP, (b) it uses a 4K default budget when ACP needs dynamic budgets, (c) it lacks MCP tool results and file context as source types.

The ACP path needs a simpler, trait-based context manager that can accept context items from any source (knowledge, playbooks, files, history, MCP tools) and render them within a token budget.

#### Implementation Steps
1. Define the `ContextSource` trait in `context_manager.rs`:
   ```rust
   pub(crate) trait ContextSource: Send + Sync {
       fn source_name(&self) -> &str;
       fn priority(&self) -> u8;  // 0 = lowest, 255 = highest; higher = evicted last
   }
   ```
2. Define `ContextItem`:
   ```rust
   pub(crate) struct ContextItem {
       pub source_name: String,
       pub priority: u8,
       pub score: f64,         // Relevance score, higher = more relevant
       pub content: String,
       pub token_count: usize,
       pub evictable: bool,    // User-requested context (@-mentions) is not evictable
   }
   ```
3. Define `ContextManager`:
   ```rust
   pub(crate) struct ContextManager {
       budget: TokenBudget,
       items: Vec<ContextItem>,
   }
   ```
4. Implement methods:
   - `new(budget: TokenBudget) -> Self`
   - `add(&mut self, item: ContextItem)` -- inserts into the priority queue
   - `render(&self) -> String` -- renders items within budget, highest-scored first, evicting lowest-scored evictable items when budget exceeded
   - `stats(&self) -> ContextUsageStats` -- returns per-source token counts and eviction counts
5. `ContextUsageStats`:
   ```rust
   pub(crate) struct ContextUsageStats {
       pub total_budget: usize,
       pub total_used: usize,
       pub per_source: Vec<(String, usize)>,
       pub items_evicted: usize,
   }
   ```
6. Add `pub(crate) mod context_manager;` to `lib.rs`.

#### Design Guidance
Do NOT reuse `ContextAssembler` from `roko-neuro`. The ACP context manager is simpler: it is a budget-aware priority queue, not an attention-scored retrieval system. Keep it under 200 lines. The fancy scoring (diminishing returns, novelty penalties, contrarian retrieval) lives in the neuro layer and feeds items into this manager; the manager only does budget fitting and eviction.

#### Verification Criteria
- [ ] Unit test: add 5 items totaling 1000 tokens to a manager with budget 500 -- render returns ~500 tokens, lowest-scored evictable items are dropped
- [ ] Unit test: non-evictable items are always included even when budget is tight
- [ ] Unit test: stats reports correct per-source breakdown
- [ ] Unit test: empty manager renders empty string

---

### Task 9.4: Wire ContextManager into ACP Bridge Events
**Priority**: P0
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/bridge_events.rs`
**Depends On**: Task 9.3

#### Context
In `bridge_events.rs`, the `handle_session_prompt()` function (line ~679) assembles context via inline `append_context()` calls:
1. Queries `query_dispatch_knowledge()` for knowledge + playbooks (line ~741)
2. Gets the mode-specific system prompt (lines ~746-770)
3. Calls `append_context()` to merge knowledge into system prompt (lines ~771-780)
4. Builds conversation history (lines ~785-800)

This needs to be replaced with the `ContextManager` flow.

#### Implementation Steps
1. At the start of `handle_session_prompt()`, construct a `ContextManager` with budget from session config or model's `max_context_tokens` (default 100,000):
   ```rust
   let budget = TokenBudget::from_total(session.context_budget.unwrap_or(100_000));
   let mut ctx = ContextManager::new(budget);
   ```
2. Add knowledge hits as `ContextItem { source_name: "knowledge", priority: 128, evictable: true }`
3. Add playbook context as `ContextItem { source_name: "playbook", priority: 120, evictable: true }`
4. Add file context from @-mentions as `ContextItem { source_name: "file_context", priority: 200, evictable: false }` (user-requested context is never evicted)
5. Add conversation history as `ContextItem { source_name: "history", priority: 100, evictable: true }` with recency-based scoring (recent turns scored higher)
6. Call `ctx.render()` to produce the final context string
7. Replace the existing `append_context()` chain with the manager output
8. Log `ctx.stats()` for debugging via `tracing::debug!`

#### Verification Criteria
- [ ] Run ACP session with a prompt that @-mentions 5 files -- verify file context appears in agent prompt
- [ ] Verify knowledge context is truncated when file context consumes most of the budget
- [ ] Existing ACP unit tests pass
- [ ] `tracing::debug` output shows context stats with per-source breakdown

---

### Task 9.5: Add Context Budget Session Config Option
**Priority**: P1
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/session.rs`
**Depends On**: Task 9.4

#### Context
`AcpSession` in `crates/roko-acp/src/session.rs` currently exposes 9 config options built by `build_config_options()`. The session stores config values but has no `context_budget` field.

#### Implementation Steps
1. Add `pub context_budget: Option<usize>` field to `AcpSession` (default `None` = auto).
2. Add config option #10 to `build_config_options()`:
   - `id: "context_budget"`, `name: "Context Budget"`, `option_type: Select`
   - `options: ["auto", "small (32k)", "medium (64k)", "large (128k)", "max"]`
   - `default: "auto"`
3. In `update_config()`, map the string value to a token count:
   - `"auto"` -> `None` (let ContextManager use model's max_tokens / 2)
   - `"small"` -> `Some(32_000)`
   - `"medium"` -> `Some(64_000)`
   - `"large"` -> `Some(128_000)`
   - `"max"` -> `None` with a flag to use model's full max_tokens
4. Pass to `ContextManager` construction in bridge_events.

#### Verification Criteria
- [ ] ACP `session/new` response includes 10 config options (was 9)
- [ ] Setting context budget to "small" reduces knowledge items in prompt
- [ ] Setting to "max" includes all available context
- [ ] Existing session tests pass

---

### Task 9.6: Multi-Turn Context Carry-Forward
**Priority**: P1
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/session.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/bridge_events.rs`
**Depends On**: Task 9.4

#### Context
`AcpSession` has `history: Vec<ConversationTurn>` (defined in session.rs) that tracks conversation but does not track which files the agent modified. Subsequent turns have no continuity about what was changed unless the user re-mentions files.

#### Implementation Steps
1. Add `pub touched_files: Vec<TouchedFile>` to `AcpSession`:
   ```rust
   pub struct TouchedFile {
       pub path: String,
       pub turn_index: usize,
       pub change_type: String, // "edited", "created", "deleted"
   }
   ```
2. After each prompt completes in `bridge_events.rs`, extract file paths from tool call updates (`ToolCallKind::Edit`, `Create`, `Delete`) and append to `touched_files`.
3. Deduplicate the list by path, cap at 20 files (remove oldest when over limit).
4. In the next prompt's context assembly (Task 9.4 integration), add touched files as `ContextItem { source_name: "touched_file", priority: 150, score: recency_score, evictable: true }` where `recency_score = 1.0 - (turns_ago * 0.15)`.
5. Score touched files by recency: most recently touched = highest score.

#### Verification Criteria
- [ ] ACP session: first prompt edits `src/lib.rs`, second prompt receives `src/lib.rs` content in context without @-mention
- [ ] Files from 3+ turns ago are evicted when budget is tight
- [ ] `touched_files` does not grow beyond 20 entries

---

### Task 9.7: Per-Turn Context Usage Tracking
**Priority**: P2
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/bridge_events.rs`
**Depends On**: Task 9.4

#### Context
The cascade router is updated with bandit observations after each ACP dispatch (in `bridge_events.rs`), but there is no record of what context was provided to the agent. Without this data, the learning system cannot optimize context budgets.

#### Implementation Steps
1. Define `ContextUsageRecord`:
   ```rust
   struct ContextUsageRecord {
       turn_id: String,
       total_budget: usize,
       knowledge_tokens: usize,
       playbook_tokens: usize,
       file_tokens: usize,
       history_tokens: usize,
       items_evicted: usize,
       success: bool,
       timestamp: DateTime<Utc>,
   }
   ```
2. After each prompt completes, construct the record from `ContextManager::stats()`.
3. Append to `.roko/learn/context-usage.jsonl` via JSONL file append.
4. In the cascade router observation, include `context_budget` as a feature in the routing context vector (extend the `RoutingContext` struct).

#### Verification Criteria
- [ ] Run 5 ACP prompts -- `.roko/learn/context-usage.jsonl` has 5 entries
- [ ] Each entry has non-zero `total_budget` and accurate source breakdowns
- [ ] Cascade router context vector includes budget feature

---

## Phase 2: Parallel Agent Execution in ACP Pipeline

The Full template declares parallel review (Architect + Auditor + Scribe) but
implements single-reviewer. This phase extends the pure state machine with
parallel phases while keeping `step()` side-effect-free.

### Task 9.8: Add ParallelExecution Phase to Pipeline State Machine
**Priority**: P0
**Estimated Effort**: 5 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/pipeline.rs`
**Depends On**: none (independent of Phase 1)

#### Context
`PipelinePhase` at `crates/roko-acp/src/pipeline.rs:12-33` has 10 variants, all serial. `PipelineEvent` at line 44-71 has 12 variants, all single-agent. `PipelineAction` at line 75-92 has 8 variants, all single-agent. The `step()` method at line 195 pattern-matches all transitions exhaustively.

#### Implementation Steps
1. Add `ParallelExecution` variant to `PipelinePhase`:
   ```rust
   ParallelExecution {
       agent_ids: Vec<String>,
       completed: Vec<String>,
       results: Vec<(String, String)>, // (agent_id, output)
       barrier: BarrierCondition,
   }
   ```
2. Add `BarrierCondition` enum:
   ```rust
   #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
   pub enum BarrierCondition {
       AllComplete,
       MajorityComplete,
       AnyComplete,
   }
   ```
3. Add `ParallelAgentSpec`:
   ```rust
   #[derive(Debug, Clone)]
   pub struct ParallelAgentSpec {
       pub id: String,
       pub role: String,
       pub prompt: String,
       pub context: String,
   }
   ```
4. Add new events to `PipelineEvent`:
   - `ParallelAgentCompleted { agent_id: String, output: String }`
   - `ParallelAgentFailed { agent_id: String, error: String }`
5. Add new action to `PipelineAction`:
   - `SpawnParallelAgents { specs: Vec<ParallelAgentSpec> }`
6. Add transitions to `step()`:
   - `(ParallelExecution, ParallelAgentCompleted)` -> update completed list, check barrier
   - When barrier met -> transition to next phase (caller determines what)
   - `(ParallelExecution, ParallelAgentFailed)` -> for `AllComplete`, halt; for `MajorityComplete` / `AnyComplete`, check if remaining agents suffice
7. Implement `BarrierCondition::is_met(completed: usize, total: usize) -> bool`.

#### Design Guidance
The state machine must remain pure. `ParallelExecution` only tracks which agents completed and their outputs. The runner handles the actual async spawning. The barrier check is a simple count comparison.

#### Verification Criteria
- [ ] Unit test: `ParallelExecution` with 3 agents, `AllComplete` barrier -- completing all 3 transitions to next phase
- [ ] Unit test: `MajorityComplete` transitions after 2 of 3 complete
- [ ] Unit test: one agent failure with `AllComplete` halts the pipeline
- [ ] Unit test: `AnyComplete` transitions after first completion
- [ ] All existing pipeline tests pass unchanged

---

### Task 9.9: Add VerdictMerge Phase to Pipeline
**Priority**: P1
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/pipeline.rs`
**Depends On**: Task 9.8

#### Context
After parallel review agents complete, their outputs need to be merged into a single approve/revise verdict. This is a new pipeline phase that the runner implements by parsing review outputs.

#### Implementation Steps
1. Add `VerdictMerge` variant to `PipelinePhase`:
   ```rust
   VerdictMerge { outputs: Vec<(String, String)> }  // (role, output)
   ```
2. Add `MergeComplete { merged_verdict: String }` to `PipelineEvent`.
3. Add `MergeVerdicts { outputs: Vec<(String, String)> }` to `PipelineAction`.
4. Transition from `ParallelExecution` when barrier met:
   - Collect all `(agent_id, output)` pairs from the completed list
   - Emit `MergeVerdicts` action, transition to `VerdictMerge` phase
5. Transition from `VerdictMerge` on `MergeComplete`:
   - If verdict contains "approve" -> `Committing` + `Commit`
   - If verdict contains "revise" -> `Implementing` + `SpawnImplementer` (if iterations remain) or `Committing` + `Commit` (accept with caveats)

#### Verification Criteria
- [ ] Unit test: Full pipeline with parallel review completes: `Strategizing -> Implementing -> Gating -> ParallelExecution -> VerdictMerge -> Committing`
- [ ] Unit test: merged revise verdict sends back to Implementing with accumulated findings
- [ ] All existing pipeline tests pass

---

### Task 9.10: Wire Full Template to Parallel Review
**Priority**: P1
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/pipeline.rs`
**Depends On**: Task 9.9

#### Context
The `Gating + GatesPassed` transition at `pipeline.rs:288-293` currently checks `self.template.has_review()` and always emits `SpawnReviewer` (single agent). The Full template should emit `SpawnParallelAgents` instead.

#### Implementation Steps
1. Modify the `Gating + GatesPassed` transition for `Full` template:
   - Instead of `SpawnReviewer`, emit `SpawnParallelAgents` with 3 specs:
     - Architect: deep architectural review, read-only
     - Auditor: security and correctness audit, read-only
     - Scribe: documentation coverage check, read-only
   - Set barrier to `AllComplete`
2. Keep Standard template unchanged (single `SpawnReviewer`).
3. Keep Express template unchanged (skips review).
4. Add `has_parallel_review(&self) -> bool` method to `WorkflowTemplate` that returns `true` only for `Full`.

#### Verification Criteria
- [ ] Unit test: Full template emits `SpawnParallelAgents` with 3 specs after gates pass
- [ ] Unit test: Standard template still emits `SpawnReviewer` (single agent)
- [ ] Full transition table test covers the parallel path end-to-end

---

### Task 9.11: Implement Parallel Agent Spawning in Runner
**Priority**: P1
**Estimated Effort**: 6 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/runner.rs`
**Depends On**: Task 9.10

#### Context
The runner at `crates/roko-acp/src/runner.rs` performs side effects for the pipeline. It currently handles single-agent actions (`SpawnStrategist`, `SpawnImplementer`, etc.). It needs to handle `SpawnParallelAgents` by spawning multiple agents concurrently.

#### Implementation Steps
1. Add `handle_spawn_parallel()` method that creates a `tokio::task::JoinSet`.
2. For each `ParallelAgentSpec`, spawn an agent task. Reuse the existing agent spawning logic (model resolution, system prompt building, tool permissions).
3. As each agent completes, feed `ParallelAgentCompleted` or `ParallelAgentFailed` back to the pipeline state machine via `step()`.
4. Emit ACP session updates (`ToolCall` / `ToolCallUpdate`) for each parallel agent's progress, using the agent's role as the tool call title.
5. Track per-agent cost and add to `WorkflowRun.total_cost_usd`.
6. Handle cancellation: when `CancelToken` fires, cancel all in-flight agents in the `JoinSet`.

#### Verification Criteria
- [ ] Integration test: spawn 2 mock agents in parallel, both complete
- [ ] Verify both agents' ToolCall updates appear in the ACP event stream
- [ ] Cost is sum of both agents' costs
- [ ] Cancellation kills all parallel agents

---

### Task 9.12: Implement VerdictMerge in Runner
**Priority**: P1
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/runner.rs`
**Depends On**: Task 9.11

#### Context
`parse_structured_review_verdict()` from `roko_gate` (`crates/roko-gate/src/review_verdict.rs`) parses agent output into a `ReviewVerdictContext` with verdict (approve/revise/reject), findings with severity, and suggested changes. The runner needs to merge multiple verdicts.

#### Implementation Steps
1. Add `handle_merge_verdicts()` method to the runner.
2. Parse each output through `parse_structured_review_verdict()`.
3. Merge strategy:
   - If any reviewer rejects -> revise
   - If all approve -> approve
   - Mixed -> take majority; if tied, revise (conservative)
4. Concatenate findings from all reviewers, deduplicate by description similarity.
5. Feed `MergeComplete` with the merged verdict back to the pipeline.

#### Verification Criteria
- [ ] Unit test: 3 approve -> merged approve
- [ ] Unit test: 1 reject + 2 approve -> merged revise with reject findings
- [ ] Unit test: 2 revise + 1 approve -> merged revise with combined findings

---

### Task 9.13: Add Parallel Progress to ACP Session Updates
**Priority**: P2
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/types.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/runner.rs`
**Depends On**: Task 9.11

#### Context
`SessionUpdate` at `crates/roko-acp/src/types.rs` has 11 variants covering text chunks, tool calls, plans, and usage. It has no variant for parallel agent progress.

#### Implementation Steps
1. Add `ParallelProgress` variant to `SessionUpdate`:
   ```rust
   ParallelProgress {
       total_agents: u32,
       completed_agents: u32,
       agent_statuses: Vec<ParallelAgentStatus>,
   }
   ```
   where `ParallelAgentStatus { role: String, status: ToolCallStatus }`.
2. Emit `ParallelProgress` updates from the runner whenever a parallel agent completes.
3. Add corresponding `PlanEntry` updates showing each parallel agent as a sub-step.

#### Verification Criteria
- [ ] ACP client receives `ParallelProgress` updates during parallel execution
- [ ] Progress shows correct completed/total counts
- [ ] Plan entries show individual agent status

---

## Phase 3: Workflow Template Completion

Three of the planned 8 templates are implemented. This phase adds Research,
ReviewOnly, Documentation, and Custom.

### Task 9.14: Add Research Workflow Template
**Priority**: P1
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/pipeline.rs`
**Depends On**: none

#### Context
`WorkflowTemplate` at `pipeline.rs:97-104` has 3 variants. The Research template needs two new phases: Researching (agent queries sources) and Synthesizing (agent produces a summary). No gates or commits needed.

#### Implementation Steps
1. Add `Research` variant to `WorkflowTemplate`.
2. Add `Researching` and `Synthesizing` phases to `PipelinePhase`.
3. Add new actions: `SpawnResearcher { topic: String }`, `SpawnSynthesizer { research_output: String }`.
4. Add transitions:
   - `Pending + Start` (Research) -> `Researching` + `SpawnResearcher`
   - `Researching + AgentCompleted` -> `Synthesizing` + `SpawnSynthesizer`
   - `Synthesizing + AgentCompleted` -> `Complete` + `Done`
   - `Researching + AgentFailed` / `Synthesizing + AgentFailed` -> `Halted` + `Halt` (if no retries) or retry
5. Update `auto_select()`: prompts containing "research", "investigate", "analyze", "explain", "compare" (without implementation words like "implement", "fix", "add") trigger Research template.
6. Update `from_config()` to accept `"research"`.
7. `has_strategy()` -> false, `has_review()` -> false for Research.

#### Verification Criteria
- [ ] Unit test: Research template flows through Researching -> Synthesizing -> Complete
- [ ] Unit test: `auto_select("research the differences between X and Y")` -> Research
- [ ] Unit test: `auto_select("implement the research findings")` -> NOT Research (contains "implement")
- [ ] Existing template tests pass unchanged

---

### Task 9.15: Add ReviewOnly Workflow Template
**Priority**: P1
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/pipeline.rs`
**Depends On**: none

#### Context
ReviewOnly is a read-only template: the agent reviews code but does not make changes. It never enters the Implementing phase.

#### Implementation Steps
1. Add `ReviewOnly` variant to `WorkflowTemplate`.
2. Transitions:
   - `Pending + Start` (ReviewOnly) -> `Reviewing` + `SpawnReviewer { diff_context }` (populated from git diff or prompt)
   - `Reviewing + ReviewApproved` -> `Complete` + `Done`
   - `Reviewing + ReviewRevise` -> `Complete` + `Done` (report findings, do NOT spawn implementer)
3. `has_strategy()` -> false, `has_review()` -> true.
4. Update `auto_select()`: prompts containing "review", "audit", "check" without implementation words trigger ReviewOnly.
5. Update `from_config()` to accept `"review_only"`.

#### Verification Criteria
- [ ] Unit test: ReviewOnly template never enters Implementing phase
- [ ] Unit test: review findings are reported but no implementation spawned
- [ ] `auto_select("review the changes in this PR")` -> ReviewOnly

---

### Task 9.16: Add Documentation Workflow Template
**Priority**: P2
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/pipeline.rs`
**Depends On**: none

#### Context
Documentation template: Scribe writes docs, Critic reviews, fix loop, commit. The Scribe role has restricted write access (docs only).

#### Implementation Steps
1. Add `Documentation` variant to `WorkflowTemplate`.
2. Add `Scribing` and `Critiquing` phases to `PipelinePhase`.
3. Add new actions: `SpawnScribe { files: Vec<String>, context: String }`, `SpawnCritic { docs_diff: String }`.
4. Transitions:
   - `Pending + Start` (Documentation) -> `Scribing` + `SpawnScribe`
   - `Scribing + AgentCompleted` -> `Critiquing` + `SpawnCritic`
   - `Critiquing + ReviewApproved` -> `Committing` + `Commit`
   - `Critiquing + ReviewRevise` -> `Scribing` + `SpawnScribe` (with feedback, if iterations remain)
5. Update `auto_select()`: prompts with "document", "docs", "README", "changelog" trigger Documentation.
6. Update `from_config()` to accept `"documentation"`.

#### Verification Criteria
- [ ] Unit test: Documentation template flows Scribing -> Critiquing -> Committing
- [ ] Unit test: critic rejection loops back to Scribing
- [ ] Existing template tests pass

---

### Task 9.17: Add Custom Workflow Template Parser
**Priority**: P3
**Estimated Effort**: 5 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/pipeline.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/schema.rs`
**Depends On**: Tasks 9.14, 9.15, 9.16

#### Context
Users should be able to define custom step sequences in `roko.toml`. A custom template specifies an ordered list of phases and roles.

#### Implementation Steps
1. Add `Custom { steps: Vec<CustomStep> }` variant to `WorkflowTemplate`.
2. Define `CustomStep { phase: String, role: Option<String>, config: Option<serde_json::Value> }`.
3. Add `fn from_toml(table: &toml::Table) -> Result<WorkflowTemplate>` that parses:
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
4. Validate step sequence: must contain at least "implement" and "gate". Reject invalid phase names.
5. Map custom steps to `PipelinePhase` transitions dynamically using a step index counter.
6. Add `[workflow]` section to config schema in `roko-core`.

#### Verification Criteria
- [ ] Parse a 4-step custom workflow from TOML
- [ ] Reject TOML missing "implement" step
- [ ] Custom workflow executes phases in defined order

---

## Phase 4: MCP Federation

Enable MCP servers to discover and query each other. This is independent of
Phases 1-3 and can be implemented in parallel.

### Task 9.18: Build MCP Server Registry
**Priority**: P1
**Estimated Effort**: 4 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-mcp-stdio/src/registry.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-mcp-stdio/src/lib.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-mcp-stdio/Cargo.toml`
**Depends On**: none

#### Context
Each MCP server (`roko-mcp-code`, `roko-mcp-github`, `roko-mcp-slack`, `roko-mcp-scripts`) runs as an isolated subprocess with no awareness of peers. `roko-mcp-stdio` (`crates/roko-mcp-stdio/src/lib.rs`, ~252 LOC) provides the shared transport; it is the natural home for federation infrastructure.

#### Implementation Steps
1. Add deps to `roko-mcp-stdio/Cargo.toml`: `serde`, `serde_json`, `chrono`, `fs2` (file locking).
2. Define `McpServerEntry` in `registry.rs`:
   ```rust
   pub struct McpServerEntry {
       pub name: String,
       pub pid: u32,
       pub tools: Vec<String>,
       pub socket_path: PathBuf,
       pub registered_at: DateTime<Utc>,
   }
   ```
3. Define `McpRegistry` backed by a file at a configurable path (default `.roko/mcp-registry.json`):
   ```rust
   pub struct McpRegistry {
       path: PathBuf,
   }
   ```
4. Implement:
   - `register(entry: McpServerEntry) -> Result<()>` with `fs2::FileExt` file-locking for concurrent writes
   - `unregister(name: &str) -> Result<()>`
   - `discover(tool_name: &str) -> Option<McpServerEntry>` -- finds the server exposing a given tool
   - `list_all() -> Vec<McpServerEntry>` -- returns all registered servers
   - `health_check() -> usize` -- removes entries whose pid is not running (platform-specific check), returns count removed
5. Add `pub mod registry;` to `lib.rs`.

#### Design Guidance
Use file-based registry with advisory locking (`fs2`). The registry is append-heavy and read-heavy, with very few entries (typically 2-5 servers). JSON file is fine for this scale. Health check uses `kill(pid, 0)` on Unix to test process existence.

#### Verification Criteria
- [ ] Unit test: register 3 servers, discover by tool name
- [ ] Unit test: health check removes entry for non-existent pid
- [ ] Unit test: file locking prevents corruption under concurrent writes (spawn 2 threads writing simultaneously)
- [ ] `cargo test -p roko-mcp-stdio` passes

---

### Task 9.19: Add Cross-Server Tool Call Client
**Priority**: P1
**Estimated Effort**: 5 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-mcp-stdio/src/federation.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-mcp-stdio/src/lib.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-mcp-stdio/Cargo.toml`
**Depends On**: Task 9.18

#### Context
With the registry in place, MCP servers need a client to make cross-server tool calls. The client discovers a server via registry, connects via stdio subprocess (spawning the server binary), sends a JSON-RPC request, and returns the result.

#### Implementation Steps
1. Define `FederatedClient`:
   ```rust
   pub struct FederatedClient {
       registry: McpRegistry,
       timeout: Duration,
       circuit_breaker: CircuitBreaker,
   }
   ```
2. Define `CircuitBreaker`:
   ```rust
   struct CircuitBreaker {
       failures: HashMap<String, u32>,
       threshold: u32,  // default 3
   }
   ```
3. Implement `call_tool(server_name: &str, tool: &str, args: Value) -> Result<Value>`:
   - Discover server via registry
   - Check circuit breaker (open = immediate error)
   - Spawn the server binary as a subprocess with stdio
   - Send `tools/call` JSON-RPC request with the tool name and args
   - Apply timeout (default 30s)
   - On success: reset circuit breaker for this server, return result
   - On failure: increment circuit breaker counter, return error
4. Implement `call_tool_by_name(tool: &str, args: Value) -> Result<Value>` that auto-discovers which server provides the tool.
5. Add `pub mod federation;` to `lib.rs`.

#### Design Guidance
Each cross-server call spawns a fresh subprocess (the MCP server binary), sends one request, reads one response, and terminates. This is simple and avoids connection lifecycle management. For high-throughput scenarios, a persistent connection pool could be added later but is not needed now.

#### Verification Criteria
- [ ] Integration test: server A calls tool on server B via federation
- [ ] Timeout triggers after configured duration with clear error message
- [ ] Circuit breaker opens after threshold consecutive failures
- [ ] Circuit breaker resets on successful call

---

### Task 9.20: Add Federation to roko-mcp-code
**Priority**: P2
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-mcp-code/src/lib.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-mcp-code/Cargo.toml`
**Depends On**: Task 9.19

#### Context
`roko-mcp-code` at `crates/roko-mcp-code/src/lib.rs` (~1,500 LOC) exposes 12 code intelligence tools. It has no awareness of other MCP servers. Adding a federated `github_enriched_context` tool enables compound queries.

#### Implementation Steps
1. Add `roko-mcp-stdio` dependency to `roko-mcp-code/Cargo.toml`.
2. Add optional `FederatedClient` to the code server's state (constructed from registry path env var).
3. Add `github_enriched_context` tool that:
   - Takes `{ symbol_name, pr_number }` args
   - Calls `symbol_lookup` locally for code context
   - If `FederatedClient` is available, calls `github_get_pr` via federation for PR diff
   - Merges results: "Symbol X was modified in PR #N, here is the change"
4. Register the code server with the MCP registry on startup.
5. If federation client is not available, the tool returns code-only results with a warning.
6. Add the new tool to the `tools/list` response.

#### Verification Criteria
- [ ] When GitHub MCP is running: `github_enriched_context` returns merged result
- [ ] When GitHub MCP is not running: returns code-only result with warning
- [ ] New tool appears in `tools/list` response

---

### Task 9.21: Add Federation to roko-mcp-github
**Priority**: P2
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-mcp-github/src/main.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-mcp-github/Cargo.toml`
**Depends On**: Task 9.19

#### Context
`roko-mcp-github` at `crates/roko-mcp-github/src/main.rs` (~800 LOC) exposes GitHub API tools. Adding a `pr_impact_analysis` tool that queries code intelligence enables change impact analysis.

#### Implementation Steps
1. Add `roko-mcp-stdio` dependency.
2. Add optional `FederatedClient` to the GitHub server's state.
3. Add `pr_impact_analysis` tool that:
   - Takes `{ pr_number }` args
   - Calls `github_get_pr` locally for the PR diff
   - Extracts changed function/struct names from the diff (simple regex on `fn `, `struct `, `impl `)
   - If `FederatedClient` available, calls `call_graph` via federation for each changed function (max 10)
   - Returns: affected functions, call chains, test coverage gaps
4. Register the GitHub server with the MCP registry on startup.
5. When code MCP is not running, return diff-only analysis.

#### Verification Criteria
- [ ] `pr_impact_analysis` returns call graph data for changed functions
- [ ] When code MCP is not running, returns diff-only analysis
- [ ] Tool handles large PRs (>50 files) by capping analysis to 10 functions

---

### Task 9.22: Add Federation Config to roko.toml
**Priority**: P2
**Estimated Effort**: 2 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/schema.rs`
**Depends On**: Task 9.18

#### Context
Federation behavior should be configurable via `roko.toml`. Currently `roko-core/src/config/schema.rs` defines the `RokoConfig` structure.

#### Implementation Steps
1. Add `McpFederationConfig` to config schema:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct McpFederationConfig {
       pub enabled: bool,                      // default: true
       pub registry_path: String,              // default: ".roko/mcp-registry.json"
       pub timeout_ms: u64,                    // default: 30000
       pub circuit_breaker_threshold: u32,     // default: 3
   }
   ```
2. Add `pub federation: Option<McpFederationConfig>` to the MCP config section.
3. Parse from TOML:
   ```toml
   [mcp.federation]
   enabled = true
   registry_path = ".roko/mcp-registry.json"
   timeout_ms = 30000
   circuit_breaker_threshold = 3
   ```
4. When `enabled = false` or section absent, federation client is not constructed.
5. Pass config to MCP servers via environment variables (`ROKO_MCP_FEDERATION_REGISTRY`, `ROKO_MCP_FEDERATION_TIMEOUT_MS`).

#### Verification Criteria
- [ ] Config section parses correctly
- [ ] `enabled = false` disables all federation features
- [ ] Default values work when section is omitted

---

## Phase 5: Learning-Informed MCP

Record MCP tool call outcomes and use bandit-based selection to improve
tool strategies over time. Depends on Phase 1 for context tracking.

### Task 9.23: Add ToolEffectiveness Bandit
**Priority**: P1
**Estimated Effort**: 5 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/tool_effectiveness.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/lib.rs`
**Depends On**: none

#### Context
`CascadeRouter` at `crates/roko-learn/src/cascade_router.rs` uses Thompson sampling bandits for model selection. The same pattern can be applied to MCP tool strategy selection (`keyword` vs `structural` vs `hybrid` vs `hdc` vs `embedding`).

#### Implementation Steps
1. Define `ToolCallRecord`:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct ToolCallRecord {
       pub tool: String,
       pub strategy: Option<String>,
       pub query: String,
       pub results_count: usize,
       pub was_useful: bool,
       pub latency_ms: u64,
       pub timestamp: DateTime<Utc>,
   }
   ```
2. Define `ToolEffectivenessBandit` with per-tool-per-strategy Thompson sampling:
   ```rust
   pub struct ToolEffectivenessBandit {
       arms: HashMap<String, HashMap<String, BanditArm>>,  // tool -> strategy -> arm
       path: PathBuf,
   }

   struct BanditArm {
       successes: f64,
       failures: f64,
   }
   ```
3. Implement `observe(record: &ToolCallRecord)` that updates the bandit arm (success/failure counts).
4. Implement `recommend_strategy(tool: &str) -> String` that samples from the posterior (Beta distribution) and returns the strategy with the highest sample.
5. Implement `stats(tool: &str) -> Vec<(String, f64, usize)>` returning `(strategy, success_rate, observations)`.
6. Persist to `.roko/learn/tool-effectiveness.json`.
7. Add `pub mod tool_effectiveness;` to `crates/roko-learn/src/lib.rs`.

#### Design Guidance
Reuse the Thompson sampling math from `CascadeRouter` (Beta(alpha, beta) where alpha = successes + 1, beta = failures + 1). Use the `rand` crate's Beta distribution for sampling. When a tool has fewer than 10 observations for any strategy, return "hybrid" as the default.

#### Verification Criteria
- [ ] Unit test: after 10 positive observations for "hybrid" and 2 for "keyword", "hybrid" is recommended more often (sample 100 times, >60% should be "hybrid")
- [ ] Persistence round-trip preserves bandit state
- [ ] Empty state defaults to "hybrid"

---

### Task 9.24: Record MCP Tool Calls in Episode Log
**Priority**: P1
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/bridge_events.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/episode_logger.rs`
**Depends On**: Task 9.23

#### Context
During `stream_events_to_editor()` in `bridge_events.rs`, `CognitiveEvent::ToolCallStart` and `CognitiveEvent::ToolCallComplete` events stream through. MCP tool calls are a subset of these (identified by tool name prefix or a known tool name list). The `Episode` struct in `roko-learn/src/episode_logger.rs` needs a field for tool call records.

#### Implementation Steps
1. Add `tool_calls: Vec<ToolCallRecord>` field to `Episode` in `crates/roko-learn/src/episode_logger.rs` (with `#[serde(default)]` for backward compat).
2. In `stream_events_to_editor()`, track MCP tool calls: when `ToolCallStart` fires, record `{ tool, start_time }`; when `ToolCallComplete` fires for that tool, compute latency and record result quality (heuristic: non-empty result content = potentially useful).
3. Use a `HashMap<String, Instant>` keyed by tool_call_id to track in-flight calls.
4. After dispatch completes, append tool records to the episode before persisting.
5. Feed each record to `ToolEffectivenessBandit.observe()`.

#### Verification Criteria
- [ ] Episode entries in `.roko/episodes.jsonl` include `tool_calls` array when MCP tools were used
- [ ] Each tool call record has non-zero `latency_ms`
- [ ] Bandit file is updated after each dispatch with tool call observations

---

### Task 9.25: Wire Learned Strategy into roko-mcp-code
**Priority**: P2
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-mcp-code/src/lib.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-mcp-code/Cargo.toml`
**Depends On**: Task 9.24

#### Context
`roko-mcp-code`'s `search_code` tool accepts a `strategy` parameter with values `keyword`, `structural`, `hdc`, `embedding`, `hybrid`. When the strategy is omitted or `"auto"`, it should consult the `ToolEffectivenessBandit`.

#### Implementation Steps
1. Add `roko-learn` dependency to `roko-mcp-code/Cargo.toml`.
2. Load `ToolEffectivenessBandit` from `.roko/learn/tool-effectiveness.json` at server startup (pass workdir via env var `ROKO_WORKDIR`).
3. When `search_code` is called with `strategy: "auto"` or strategy omitted, call `bandit.recommend_strategy("search_code")`.
4. Use the recommended strategy for the search.
5. Fall back to "hybrid" when bandit has fewer than 10 total observations.
6. Explicit strategy parameter overrides bandit recommendation.

#### Verification Criteria
- [ ] First 10 calls use "hybrid" (cold start default)
- [ ] After training data accumulates, "auto" selects learned-best strategy
- [ ] Explicit strategy parameter ("keyword") overrides bandit recommendation

---

### Task 9.26: Add Tool Effectiveness to Learning Dashboard
**Priority**: P3
**Estimated Effort**: 2 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/learning_helpers.rs`
**Depends On**: Task 9.23

#### Context
The `roko learn` CLI subcommand shows learning state. A `roko learn tools` subcommand should show tool effectiveness data.

#### Implementation Steps
1. Add `"tools"` match arm to the learn command handler that loads `ToolEffectivenessBandit` from `.roko/learn/tool-effectiveness.json`.
2. Print a formatted table:
   ```
   Tool           Strategy    Success   Observations   Recommended
   search_code    keyword     45%       20             no
   search_code    hybrid      82%       45             yes
   search_code    structural  60%       15             no
   ```
3. Show "No data -- run ACP sessions to accumulate tool effectiveness data" when bandit file does not exist.
4. Handle empty strategy gracefully.

#### Verification Criteria
- [ ] `roko learn tools` outputs a readable table
- [ ] Shows "No data" message when bandit file does not exist
- [ ] Handles empty/partial data gracefully

---

## Phase 6: Agent-to-Agent Communication & Permission Bridge

Enable agents within a pipeline to share intermediate results. Wire the ACP
bidirectional request protocol for permissions and structured input.

### Task 9.27: Build SharedContextStore for Cross-Agent Access
**Priority**: P1
**Estimated Effort**: 4 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/shared_context.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/lib.rs`
**Depends On**: Task 9.8

#### Context
Parallel agents (from Phase 2) need a way to share intermediate results. If the Architect finishes before the Auditor, the Auditor should have access to the Architect's findings for the verdict merge.

#### Implementation Steps
1. Define `SharedContextStore` with `Arc<RwLock<HashMap<String, ContextEntry>>>`:
   ```rust
   pub(crate) struct SharedContextStore {
       entries: Arc<RwLock<HashMap<String, ContextEntry>>>,
   }

   struct ContextEntry {
       author_role: String,
       key: String,
       value: String,
       timestamp: Instant,
   }
   ```
2. Implement `publish(role: &str, key: &str, value: &str)` -- writes entry.
3. Implement `query(key_prefix: &str) -> Vec<ContextEntry>` -- reads entries matching prefix.
4. Implement `snapshot() -> String` -- renders all entries as markdown, sorted by timestamp.
5. Add `pub(crate) mod shared_context;` to `lib.rs`.

#### Verification Criteria
- [ ] Unit test: two concurrent writers (via `tokio::spawn`), reader sees both entries
- [ ] Snapshot renders entries sorted by timestamp
- [ ] Empty store returns empty string from snapshot

---

### Task 9.28: Inject Shared Context into Parallel Agents
**Priority**: P1
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/runner.rs`
**Depends On**: Tasks 9.11, 9.27

#### Context
The runner spawns parallel agents (Task 9.11). When agents complete, their outputs should be published to the `SharedContextStore` so the verdict merge has full context.

#### Implementation Steps
1. Create a `SharedContextStore` per parallel execution phase in the runner.
2. When `ParallelAgentCompleted` fires, publish the agent's output summary to the store: `store.publish(role, "findings", &output_summary)`.
3. When all agents complete, include the full `store.snapshot()` in the `MergeVerdicts` action context.
4. Drop the store after the parallel phase ends (automatic via `Arc` refcount).

#### Verification Criteria
- [ ] When Architect finishes before Auditor, the VerdictMerge input includes Architect's findings
- [ ] Shared context appears in the `MergeVerdicts` action's outputs
- [ ] Store is dropped after the parallel phase ends

---

### Task 9.29: Wire ACP Permission Bridge
**Priority**: P2
**Estimated Effort**: 5 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/handler.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/runner.rs`
**Depends On**: none

#### Context
ACP supports `session/request_permission` (agent -> editor) for user approval of destructive actions. The `StdioTransport` at `crates/roko-acp/src/transport.rs` supports `send_request()` with pending response tracking. The capability is declared but underutilized.

#### Implementation Steps
1. In the runner, when a destructive action is detected (e.g., file deletion, dangerous command), construct a permission request:
   ```json
   {
     "method": "session/request_permission",
     "params": {
       "title": "Delete file",
       "description": "The agent wants to delete src/old_module.rs",
       "permissions": [{ "name": "file_delete", "description": "Delete src/old_module.rs", "destructive": true }]
     }
   }
   ```
2. Send via `transport.send_request()` and await the editor's response.
3. Parse the response: `approved: true/false`.
4. If approved, proceed with the action.
5. If denied, feed `AgentFailed { error: "Permission denied by user for: {action}" }` to the pipeline.
6. Non-destructive actions skip the permission check.
7. Add a configurable timeout (default 60s) for the permission prompt -- if no response, deny.

#### Verification Criteria
- [ ] Agent requesting file deletion triggers permission prompt
- [ ] User approval allows the action to proceed
- [ ] User denial feeds back as agent failure with clear reason
- [ ] Timeout results in denial

---

### Task 9.30: Wire ACP Elicitation for Structured Input
**Priority**: P2
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/handler.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/runner.rs`
**Depends On**: none

#### Context
ACP supports `elicitation/create` for structured form dialogs. When the pipeline's strategist encounters an ambiguous prompt, it could ask the user to choose between approaches.

#### Implementation Steps
1. When the strategist determines multiple valid approaches, construct an elicitation request:
   ```json
   {
     "method": "elicitation/create",
     "params": {
       "title": "Choose approach",
       "description": "The prompt is ambiguous. Please select your preferred approach.",
       "inputs": [
         { "id": "approach", "label": "Approach", "type": "select", "options": ["A: Trait-based", "B: Enum-based"], "default": "A: Trait-based" }
       ]
     }
   }
   ```
2. Send via `transport.send_request()` and await response.
3. Parse the user's selections from the response.
4. Feed into the pipeline as additional context for the implementer.
5. Timeout (60s) falls back to default selection.

#### Verification Criteria
- [ ] Strategist can request user to choose between 2 approaches
- [ ] User selection appears in implementer's context
- [ ] Timeout falls back to default selection

---

### Task 9.31: File Change Notifications to Editor
**Priority**: P2
**Estimated Effort**: 3 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/runner.rs`
**Depends On**: none

#### Context
`FileChangeNotification` and `FileChangeType` already exist in `crates/roko-acp/src/types.rs`. The runner's `detect_file_changes()` function (line 103) already parses `git diff --name-status`. These just need to be emitted as ACP session updates.

#### Implementation Steps
1. After each agent completes in the pipeline runner, call `detect_file_changes()`.
2. For each changed file, emit `CognitiveEvent::ToolCallComplete` with the file change as content, or add a new `SessionUpdate::FileChange` variant if not already present.
3. Batch notifications to avoid flooding: cap at 50 per agent completion.
4. Include the change type (Created, Modified, Deleted, Renamed) in each notification.

#### Verification Criteria
- [ ] ACP client receives file change notifications after agent edits
- [ ] Notifications include correct change type
- [ ] Large changesets are capped at 50 notifications

---

## Phase 7: A2A Bridge

Bridge Roko to Google's Agent-to-Agent Protocol for external agent collaboration.
Independent of all other phases.

### Task 9.32: Implement A2A Protocol Types
**Priority**: P2
**Estimated Effort**: 5 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-a2a/Cargo.toml`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-a2a/src/lib.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-a2a/src/types.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/Cargo.toml` (workspace members)
**Depends On**: none

#### Context
Google's A2A protocol defines Agent Cards, Tasks, Messages, and Artifacts. It is HTTP/JSON-RPC based, which aligns with `roko-serve`. The minimal surface area needed: Agent Card (discovery), Task send/status (collaboration), Message/Part types (content).

#### Implementation Steps
1. Create `crates/roko-a2a/` with `Cargo.toml` (deps: `serde`, `serde_json`, `chrono`, `url`, `uuid`).
2. Define types in `types.rs`:
   - `AgentCard { name, url, description, version, capabilities, skills, default_input_modes, default_output_modes, authentication }`
   - `AgentSkill { id, name, description, input_modes, output_modes }`
   - `A2ATask { id, session_id, status, messages, artifacts, metadata }`
   - `TaskStatus`: `Submitted`, `Working`, `InputRequired`, `Completed`, `Failed`, `Canceled`
   - `A2AMessage { role, parts }` and `A2APart` (`TextPart`, `FilePart`, `DataPart`)
   - `A2AArtifact { name, description, parts, index }`
   - `AuthenticationInfo { schemes: Vec<AuthScheme> }`
3. All types derive `Serialize`, `Deserialize`, `Debug`, `Clone`.
4. Add the crate to workspace `Cargo.toml` members.

#### Verification Criteria
- [ ] `cargo check -p roko-a2a` compiles
- [ ] Serde round-trip tests for all types pass
- [ ] Agent Card JSON matches A2A spec schema structure

---

### Task 9.33: Publish Roko Agent Card
**Priority**: P2
**Estimated Effort**: 3 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-a2a/src/agent_card.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/mod.rs`
**Depends On**: Task 9.32

#### Context
The A2A spec requires agents to publish their capabilities at `/.well-known/agent.json`. `roko-serve` already has ~85 routes; adding one more for agent card is straightforward.

#### Implementation Steps
1. Implement `fn build_agent_card(config: &RokoConfig) -> AgentCard` in `agent_card.rs`:
   - Skills: "code_implementation", "code_review", "research_and_analysis", "plan_generation"
   - Default input/output modes: `["text/plain", "application/json"]`
   - Authentication: `[{ scheme: "bearer" }]`
   - Version from `env!("CARGO_PKG_VERSION")`
2. Add `GET /.well-known/agent.json` route to `roko-serve`'s router.
3. The route handler constructs the card from the server's config and returns it as JSON.

#### Verification Criteria
- [ ] `GET /.well-known/agent.json` returns valid Agent Card JSON
- [ ] Card includes 4 skills
- [ ] Version matches crate version

---

### Task 9.34: Implement A2A Task Reception
**Priority**: P2
**Estimated Effort**: 6 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/a2a.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/mod.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/Cargo.toml`
**Depends On**: Task 9.33

#### Context
External agents submit tasks via `POST /a2a/tasks/send`. The task is mapped to an internal `WorkflowRun` and executed via the ACP pipeline runner.

#### Implementation Steps
1. Add `roko-a2a` dependency to `roko-serve/Cargo.toml`.
2. Create `routes/a2a.rs` with:
   - `POST /a2a/tasks/send` -- receives an `A2ATask`, extracts prompt from messages, selects template (from metadata or auto), executes via pipeline runner, returns task ID
   - `GET /a2a/tasks/:id` -- returns task status (maps pipeline phase to `TaskStatus`)
   - `POST /a2a/tasks/:id/cancel` -- cancels a running task via `CancelToken`
3. Map pipeline completion to A2A task status:
   - `Complete` -> `TaskStatus::Completed` with output as artifact
   - `Halted` / `Cancelled` -> `TaskStatus::Failed` with error detail
   - In-progress phases -> `TaskStatus::Working`
4. Store active tasks in `AppState` with `HashMap<String, A2ATaskState>`.
5. Add routes to `build_router()`.

#### Verification Criteria
- [ ] External agent can submit a coding task via `POST /a2a/tasks/send`
- [ ] Task status is retrievable via `GET /a2a/tasks/:id`
- [ ] Pipeline completion updates A2A task status
- [ ] Cancellation via `POST /a2a/tasks/:id/cancel` works

---

### Task 9.35: Add DelegateExternal Action to Pipeline
**Priority**: P3
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/pipeline.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/runner.rs`
**Depends On**: Task 9.34

#### Context
The pipeline should be able to delegate sub-tasks to external agents via A2A. This adds an outbound path (Roko delegates to others) complementing the inbound path (others delegate to Roko).

#### Implementation Steps
1. Add `DelegateExternal { task: ExternalTaskSpec }` to `PipelineAction` where:
   ```rust
   pub struct ExternalTaskSpec {
       pub agent_url: String,
       pub skill: String,
       pub prompt: String,
   }
   ```
2. Add `ExternalDelegated` phase to `PipelinePhase`.
3. Add `ExternalCompleted { output: String }` and `ExternalFailed { error: String }` to `PipelineEvent`.
4. Transitions:
   - `DelegateExternal` -> `ExternalDelegated`
   - `ExternalCompleted` -> resume pipeline (back to Gating or next phase)
   - `ExternalFailed` -> `Halted` or retry
5. In the runner, implement delegation by POSTing to `{agent_url}/a2a/tasks/send` and polling `GET /a2a/tasks/:id` until completion.

#### Verification Criteria
- [ ] Unit test: pipeline transitions through `ExternalDelegated` phase
- [ ] Unit test: external failure halts pipeline with clear reason
- [ ] Pipeline state machine remains pure (no I/O in `step()`)

---

## Phase 8: Tracker Integrations

Bidirectional sync with external project trackers. Independent of all other
phases.

### Task 9.36: Define TrackerAdapter Trait
**Priority**: P2
**Estimated Effort**: 3 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/tracker.rs`
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/lib.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/Cargo.toml`
**Depends On**: none

#### Context
Roko needs a generalized adapter trait for bidirectional task sync with external systems (GitHub Issues, Sentry, Linear). The trait should be object-safe for dynamic dispatch.

#### Implementation Steps
1. Define the trait in `tracker.rs`:
   ```rust
   #[async_trait]
   pub trait TrackerAdapter: Send + Sync {
       fn kind(&self) -> &str;
       async fn fetch_active(&self) -> Result<Vec<ExternalTask>>;
       async fn update_state(&self, id: &str, state: &str, comment: Option<&str>) -> Result<()>;
       async fn create_task(&self, spec: &TaskSpec) -> Result<String>;
       fn state_mapping(&self) -> &StateMapping;
   }
   ```
2. Define `ExternalTask { id, title, description, state, labels, url, assignee, metadata: HashMap<String, String> }`.
3. Define `StateMapping { pending, in_progress, completed, failed }` mapping Roko states to tracker-specific strings.
4. Define `TaskSpec { title, description, labels: Vec<String>, assignee: Option<String> }`.
5. Add `pub mod tracker;` to `lib.rs`.
6. Add `async-trait` dependency to `Cargo.toml` if not already present.

#### Verification Criteria
- [ ] Trait compiles and is object-safe (`Box<dyn TrackerAdapter>`)
- [ ] `StateMapping` covers all Roko task states
- [ ] Types derive `Serialize`, `Deserialize`, `Debug`, `Clone`

---

### Task 9.37: Implement GitHub Issues TrackerAdapter
**Priority**: P2
**Estimated Effort**: 5 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tracker/mod.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tracker/github.rs`
**Depends On**: Task 9.36

#### Context
GitHub Issues is the most common tracker for Roko's target users. The `roko-mcp-github` crate already has rate-limit-aware HTTP calling logic that can be referenced.

#### Implementation Steps
1. Implement `GithubTrackerAdapter { owner, repo, token, state_mapping, label_filter }`.
2. `fetch_active()`: use `gh` CLI or GitHub REST API to list open issues with the configured label filter (default: `roko`).
3. `update_state()`: add a comment and optionally close the issue (when state maps to "closed").
4. `create_task()`: create a GitHub issue with labels.
5. Default state mapping: `pending -> open`, `in_progress -> open` (add "in-progress" label), `completed -> closed`, `failed -> open` (add "failed" label).
6. On task completion, post comment: "Completed by Roko. Changes: {summary}".
7. Reuse `roko-mcp-github`'s retry/rate-limit patterns where possible (reference, not dependency -- keep it lightweight).

#### Verification Criteria
- [ ] `fetch_active()` returns issues from a configured repo
- [ ] `update_state("completed")` closes the issue and adds comment
- [ ] Label-based state tracking works
- [ ] Missing `GITHUB_TOKEN` returns clear error

---

### Task 9.38: Implement Sentry TrackerAdapter
**Priority**: P3
**Estimated Effort**: 4 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tracker/sentry.rs`
**Depends On**: Task 9.36

#### Context
Sentry errors can be ingested as fix tasks. The adapter fetches unresolved issues with stack traces and converts them to `ExternalTask` descriptions.

#### Implementation Steps
1. Implement `SentryTrackerAdapter { org, project, token, state_mapping }`.
2. `fetch_active()`: call Sentry REST API to list unresolved issues with configurable filters (assignee, tag).
3. For each Sentry issue, construct `ExternalTask` with: `description` = stack trace + affected files + error count, `metadata` = error frequency, first/last seen.
4. `update_state("resolved")`: resolve the Sentry issue via API.
5. `create_task()`: no-op (Sentry issues are external-only).

#### Verification Criteria
- [ ] `fetch_active()` returns Sentry issues with stack traces in description
- [ ] `update_state("resolved")` resolves the issue in Sentry
- [ ] Missing `SENTRY_TOKEN` returns clear error

---

### Task 9.39: Implement Linear TrackerAdapter
**Priority**: P3
**Estimated Effort**: 4 hours
**Files to Create**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tracker/linear.rs`
**Depends On**: Task 9.36

#### Context
Linear uses a GraphQL API. The adapter syncs issues bidirectionally.

#### Implementation Steps
1. Implement `LinearTrackerAdapter { api_key, team_id, state_mapping }`.
2. `fetch_active()`: call Linear GraphQL API to list issues in active states with team filter.
3. `update_state()`: transition the Linear issue to the mapped state via GraphQL mutation.
4. `create_task()`: create a new Linear issue with team assignment.
5. Default state mapping: `pending -> Backlog`, `in_progress -> In Progress`, `completed -> Done`, `failed -> Backlog` (with comment).

#### Verification Criteria
- [ ] GraphQL query fetches active issues
- [ ] State transitions map correctly between Roko and Linear
- [ ] Missing `LINEAR_API_KEY` returns clear error

---

### Task 9.40: Wire TrackerAdapter into Plan Execution
**Priority**: P2
**Estimated Effort**: 4 hours
**Files to Modify**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/schema.rs`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/runner.rs`
**Depends On**: Tasks 9.36, 9.37

#### Context
When plan execution completes a task, the external tracker should be updated. Configuration lives in `roko.toml`.

#### Implementation Steps
1. Add `[tracker]` section to config schema:
   ```toml
   [tracker]
   kind = "github"  # or "linear", "sentry", "none"
   auto_sync = true

   [tracker.github]
   owner = "org"
   repo = "repo"
   label_filter = "roko"
   ```
2. Parse `TrackerConfig` in the config loader.
3. In the ACP runner, after each pipeline completes successfully, call `adapter.update_state()` with the completion state and a summary comment.
4. Add `--from-tracker` flag support: call `adapter.fetch_active()` to populate tasks from external issues.
5. When `auto_sync = false`, skip automatic updates.
6. When `kind = "none"` or config section absent, construct a no-op adapter.

#### Verification Criteria
- [ ] Task completion updates GitHub issue with comment
- [ ] `auto_sync = false` disables automatic updates
- [ ] Missing tracker config gracefully falls back to no-op

---

## Dependency Graph

```
Phase 1 (Context Management) [Tasks 9.1-9.7]
  |
  +---> Phase 2 (Parallel Agents) [Tasks 9.8-9.13]
  |         |
  |         +---> Phase 3 (Templates) [Tasks 9.14-9.17]
  |
  +---> Phase 5 (Learning MCP) [Tasks 9.23-9.26]
  |
  +---> Phase 6 (Agent Communication) [Tasks 9.27-9.31]

Phase 4 (MCP Federation) [Tasks 9.18-9.22] -- independent

Phase 7 (A2A Bridge) [Tasks 9.32-9.35] -- independent

Phase 8 (Trackers) [Tasks 9.36-9.40] -- independent
```

Phases 4, 7, and 8 have no dependencies on other phases. They can be
implemented in any order or in parallel with Phases 1-3.

Phase 2 depends on Phase 1 (context budget needed for parallel agents).
Phase 3 depends on Phase 2 (templates use parallel execution).
Phase 5 depends on Phase 1 (context tracking feeds learning).
Phase 6 depends on Phases 1 and 2 (shared context for parallel agents).

Within each phase, tasks are ordered by dependency (task N+1 may depend on
task N). Independent tasks within a phase can be implemented in parallel.

---

## Summary

40 tasks across 8 phases. Estimated total: 26-34 days of agent work.

| Phase | Tasks | Task IDs | Estimate | Dependencies |
|---|---|---|---|---|
| 1. Context Management | 7 | 9.1-9.7 | 4-5 days | None |
| 2. Parallel Agents | 6 | 9.8-9.13 | 4-5 days | Phase 1 |
| 3. Workflow Templates | 4 | 9.14-9.17 | 3-4 days | Phase 2 |
| 4. MCP Federation | 5 | 9.18-9.22 | 3-4 days | None |
| 5. Learning-Informed MCP | 4 | 9.23-9.26 | 2-3 days | Phase 1 |
| 6. Agent Communication | 5 | 9.27-9.31 | 3-4 days | Phases 1, 2 |
| 7. A2A Bridge | 4 | 9.32-9.35 | 4-5 days | None |
| 8. Tracker Integrations | 5 | 9.36-9.40 | 3-4 days | None |

Critical path: Phase 1 -> Phase 2 -> Phase 3 (15-18 days).
Parallel work: Phases 4, 7, 8 can run concurrently with the critical path.
