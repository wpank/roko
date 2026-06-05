# ACP UX Gap Analysis — Screenshot Parity

**Reference**: Mock screenshot showing incident triage workflow with full pipeline UX

## Element-by-Element Audit

### 1. Phase Badges (Strategizing / Implementing / Gating / Auto-fixing / etc.)

**Screenshot shows**: Colored pills inline in the message stream: `⊕ Strategizing`, `♦ Implementing`, `⊕ Gating`, `🔧 Auto-fixing iter 2`, `⊕ Gating iter 2`

**Current state**: `workflow_plan_entries()` in runner.rs (line 378) emits `PlanEntry` objects with plain text content like "Strategy brief", "Implementation", "Run gates". These render as a sidebar checklist — NOT inline badges in the message stream.

**What's missing**:
- Phase transitions need to emit `AgentMessageChunk` with styled badge text inline (e.g., `**⊕ Strategizing**`) so they appear in the message flow
- Iteration count not tracked — pipeline.rs `PipelineState` doesn't count which gate/autofix cycle it's on
- Phase badges need emoji/icon per phase type

**Fix**: Emit an `AgentMessageChunk` with a styled badge string at each phase transition, PLUS continue emitting `Plan` entries for the sidebar. Track `iteration: u32` in `PipelineState`.

---

### 2. Knowledge Store Card (2 hits with scores)

**Screenshot shows**: Dotted-border card titled "Knowledge from neuro store — 2 hits" with:
- `0.94  Playbook: P1 5xx → confirm scope → bisect deploy window → minimal hotfix`
- `0.81  Episode #2812 — Last 5xx spike: nil deref after refactor. Same shape.`

**Current state**: ACP never queries roko-neuro. No knowledge injection. No card emitted.

**What's missing**:
- Query `roko-neuro` knowledge store at dispatch time with the user's prompt
- Render results as a `ToolCall` card (kind: `Other`, title: "Knowledge from neuro store")
- Include hits with relevance scores in content blocks
- Also inject knowledge context into the system prompt for the agent

**Fix**: Before dispatch, query neuro store. If hits found:
1. Emit `ToolCall { kind: Other, title: "Knowledge from neuro store — N hits", content: [...scores...] }`
2. Inject knowledge text into agent system prompt
3. Emit `ToolCallUpdate` when complete

---

### 3. Permission Dialog (Allow / Always Allow / Reject)

**Screenshot shows**: Card with "Edit services/checkout.go?" showing `L13-17 · production-tracked file` and buttons: Allow, Always Allow, Reject

**Current state**: ACP types.rs has no permission request types. The ACP spec supports `session/request_permission` but roko hasn't implemented it. Agents currently run with `--dangerously-skip-permissions` (no permission checks at all).

**What's missing**:
- `RequestPermission` and `PermissionResponse` types
- `session/request_permission` method in handler.rs
- Bridge that intercepts file edit tool calls and asks permission before executing
- File metadata (line range, tracking status) in the permission card

**Fix**: Add types + handler method. Before file writes in pipeline, emit permission request and wait for response. This is a bidirectional RPC call (agent → client → agent).

---

### 4. Narrative Text Between Phases

**Screenshot shows**: Rich explanatory text between phase badges:
- "P1 incident. I'll triage in three steps: confirm scope from logs, bisect the deploy window, then ship a minimal hotfix."
- "Smoking gun: 14:18 refactor returns early on error without writing the response..."
- "Permission granted. Following the agent — watch the editor jump to checkout.go:13."
- "Two tests fail. The fix is right — those tests asserted the old (broken) behaviour."

**Current state**: `AcpWorkflowEventConsumer` emits `TokenChunk` events from `CoreRuntimeEvent::AgentOutput`, which streams the agent's response text. However, the pipeline phases themselves don't produce narrative — only the final agent output does.

**What's partially works**: Agent responses DO stream as `AgentMessageChunk`. But the pipeline runner doesn't emit its own narrative between phases (e.g., "Two tests fail. Moving to auto-fixer.").

**What's missing**:
- After each phase transition, emit a brief narrative `AgentMessageChunk` explaining what happened and what's next
- After gate failure, emit the gate error summary as readable text (not just a tool_call_update)
- After knowledge hits, emit agent thinking about the matches

**Fix**: In `run_workflow_pipeline()`, after each phase result, emit a narrative `AgentMessageChunk` summarizing the outcome. E.g., after `GateFailed`: emit "Two tests fail. The fix is right — those tests asserted the old (broken) behaviour. Moving to auto-fixer to update them."

---

### 5. Iteration Tracking (iter 2)

**Screenshot shows**: `🔧 Auto-fixing iter 2`, `⊕ Gating iter 2`

**Current state**: `PipelineState` in pipeline.rs has `max_iterations` but no `current_iteration` counter. `WorkflowRun` in workflow.rs has `agents_spawned` but no per-phase iteration count.

**What's missing**:
- `current_iteration: u32` field in `PipelineState`
- Increment on each gate-fail → auto-fix → re-gate cycle
- Include iteration number in phase badge text and plan entry content

**Fix**: Add `current_iteration` to `PipelineState`. Increment in `step()` when transitioning `Gating → AutoFixing`. Include in emitted plan entries and badge text.

---

### 6. Context Chips (@ mentions)

**Screenshot shows**: Bottom of input area has `@ services/checkout.go ×`, `@ branch diff ×`, `@ datadog logs (last 10m) ×`. Top of user message shows same as attached context.

**Current state**: `SessionPromptParams` has `prompt: Vec<ContentBlock>` which can include `ContentBlock::Resource { resource: ResourceRef::File { uri } }`. But `include_context` is just a bool. There's no concept of named context providers like "branch diff" or "datadog logs".

**What's missing**:
- Context provider registry — ACP should advertise available context providers at `session/new`
- Provider types: files (existing), git diff, log sources, custom MCP resources
- Context items should be rendered in the prompt to the agent
- MCP servers can serve as context providers (datadog MCP, git MCP)

**Fix**: At `session/new`, report available context providers in capabilities. When `session/prompt` includes `ContentBlock::Resource` items, resolve each to actual content:
- `file://path` → read file
- `git://diff` → run `git diff` and include output
- `mcp://server/resource` → query MCP server for the resource
Inject resolved context into the agent's prompt.

---

### 7. Token Counter in Status Bar (24,850 / 200k)

**Screenshot shows**: Status bar with `24,850 / 200k` showing current tokens / context window size

**Current state**: `UsageUpdate` type exists with `used: u64` and `size: u64` fields. But it's NEVER emitted — dead code. `UsageInfo` type has full breakdown (input/output/thought/cached) but isn't connected.

**What's missing**:
- Accumulate token usage from each provider response
- Emit `UsageUpdate` after each response chunk completes
- Include `CostInfo` when pricing is known
- Track cumulative session usage (not just per-turn)

**Fix**: This is already covered by batch R5_F03. Just ensure `UsageUpdate` is emitted as a `session/update` notification, not just stored internally.

---

### 8. Replay Button

**Screenshot shows**: `⟳ replay` button in status bar

**Current state**: Not supported in ACP spec. Would need custom extension.

**What could work**: After pipeline completes, include a `session/update` with `SessionInfoUpdate` that includes a replay URL pointing to `http://localhost:6677/episodes/{id}`. The editor could render this as a button. Or expose `/replay` slash command.

**Fix**: Low priority. Link to roko dashboard episode viewer.

---

## Summary: What Needs to Be Added

### New Batches Needed (beyond the 10 already defined)

| ID | What | Critical for Screenshot |
|----|------|------------------------|
| R3_F04 | Permission request bridge (request_permission types + handler) | Yes — permission dialog |
| R5_F05 | Knowledge store query + card emission at dispatch time | Yes — knowledge card |
| R7_F04 | Phase badge inline emission + iteration tracking | Yes — phase badges |
| R7_F05 | Narrative text emission between pipeline phases | Yes — explanatory text |
| R7_F06 | Context provider registry + resolution at prompt time | Yes — @ mentions |

### Updates to Existing Batches

| Batch | Addition |
|-------|----------|
| R5_F03 (cost tracking) | Also emit `UsageUpdate` notification to editor |
| R7_F02 (file changes) | Include file metadata (line range) in notifications |
| R3_F02 (system prompts) | Include knowledge context in prompt assembly |
