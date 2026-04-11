# 16 — CLI / TUI Rendering Convergence

> Cross-cutting plan covering `tmp/workflow/10-cli-chat-tui-audit.md`. Targets the rendering god files.

---

## Status (2026-05-01)

**PARTIAL.** Chat session unified for one path. Two chat loops still present. `extract_clean_text` still in chat.rs. TUI dashboard separate from inline rendering. Built primitives (ToolCallBlock, CostWaterfall, DiffBlock) wired in chat but not in TUI.

**What's done:**

- `chat_inline.rs::run_unified_inline` — unified inline chat with tool output, cost meter, streaming spinner
- `inline/primitives/streaming.rs` — `StreamingState` with append API
- `inline/primitives/cost_meter.rs` — per-session cost
- `inline/markdown.rs` — markdown rendering with bar
- Unified `ClaudeStreamEvent` parsing in `crates/roko-agent/src/provider/claude_cli/stream.rs`
- ROSEDUST theme

**What's not:**

- `chat_inline.rs` still 4,100 LOC with two chat loops (`run_chat_inline` HTTP-sidecar + `run_unified_inline` direct)
- `chat.rs` still 659 LOC parallel REPL implementation (`run_chat_repl`)
- `extract_clean_text` (246 lines, 13 response shapes) still alive
- Built-but-unused primitives:
  - `tool_call.rs` (241 LOC) — ToolCallBlock — not wired into `push_tool_outputs`
  - `cost_waterfall.rs` (180 LOC)
  - `diff_block.rs` (177 LOC)
  - `replan_block.rs` (181 LOC partial)
  - `session_summary.rs` (172 LOC partial)
- TUI rendering (`tui/app.rs` 4101 LOC, `tui/state.rs` 4968 LOC, `tui/dashboard.rs` 6382 LOC) does not consume `RuntimeProjection` directly — independently loads `.roko/` files
- TUI does not render tool outputs (Agents tab F3 shows raw text)
- No shared `ResponseRenderer` trait

---

## Goal

After this plan:

- Single chat loop in `chat_inline.rs` (~1500 LOC down from 4100)
- `roko chat` (REPL) is either deleted or a 30-LOC wrapper
- `extract_clean_text` is gone
- All built primitives are wired
- TUI consumes `RuntimeProjection::dashboard_view` (per plan 10 § 6)
- One `ResponseRenderer` trait with `InlineRenderer`, `PlainRenderer`, `TuiRenderer` implementations
- TUI Agents tab renders `ToolCallBlock` from the unified primitive

---

## Why This Exists (Anti-Patterns Eliminated)

- **#7 Copy-Paste Between Runtimes** — two chat loops; two terminal systems
- **#10 God file** — `chat_inline.rs` 4100 LOC, `tui/app.rs` 4101 LOC, `tui/state.rs` 4968 LOC, `tui/dashboard.rs` 6382 LOC
- **#3 Build Another Runtime** — TUI loads disk independently from inline rendering

---

## Existing Code — Read These First

- `crates/roko-cli/src/chat_inline.rs` — the god file
- `crates/roko-cli/src/chat.rs` — REPL parallel
- `crates/roko-cli/src/inline/primitives/` — wired and unwired primitives
- `crates/roko-cli/src/tui/dashboard.rs` — disk-loading dashboard
- `crates/roko-cli/src/tui/app.rs` — main loop
- `crates/roko-cli/src/tui/ws_client.rs` — agent WebSocket consumer

---

## Implementation Steps

### Step 1 — Define `ResponseRenderer` trait

```rust
// crates/roko-cli/src/render/mod.rs
pub trait ResponseRenderer: Send {
    fn render_text(&mut self, text: &str);
    fn render_thinking_delta(&mut self, text: &str);
    fn render_tool_call(&mut self, call: &ToolCallEvent);
    fn render_tool_output(&mut self, output: &ToolOutputEvent);
    fn render_cost(&mut self, summary: &CostSummary);
    fn render_gate_verdict(&mut self, verdict: &GateVerdict);
    fn render_error(&mut self, error: &str, suggestions: &[String]);
    fn finalize(&mut self);
}

pub struct ToolCallEvent { pub tool: String, pub args_summary: String, pub correlation_id: String }
pub struct ToolOutputEvent { pub tool: String, pub correlation_id: String, pub output: String, pub success: bool, pub duration_ms: u64 }
```

Implementations:

- `InlineRenderer` (uses `inline/primitives/`) — for `roko` and `roko run`
- `PlainRenderer` (no styling) — for `roko -q` / non-TTY / `roko chat --plain`
- `TuiRenderer` (writes to `TuiState`) — for `roko dashboard` Agents tab F3

### Step 2 — Extract one canonical chat loop

**File:** `crates/roko-cli/src/chat_inline.rs`

Today `run_chat_inline` (HTTP-sidecar) and `run_unified_inline` (direct) duplicate ~700 LOC each. Extract:

```rust
// crates/roko-cli/src/chat_session_loop.rs
pub trait ChatBackend: Send {
    async fn send_turn(&mut self, prompt: String) -> Result<DispatchResult>;
    async fn cancel(&self) -> Result<()>;
}

pub struct DirectModelCallBackend { service: Arc<ModelCallService>, session_id: Option<String> }
pub struct HttpSidecarBackend { client: HttpClient, agent_id: String, base_url: String }

impl ChatBackend for DirectModelCallBackend { /* via ModelCallService */ }
impl ChatBackend for HttpSidecarBackend     { /* via HTTP /message */ }

pub async fn run_chat_loop<R: ResponseRenderer, B: ChatBackend>(
    renderer: &mut R,
    backend: &mut B,
    session: &mut ChatSession,
    cancel: CancelToken,
) -> Result<()> { /* ~600 LOC of one canonical loop */ }
```

`run_chat_inline` and `run_unified_inline` become 30-LOC wrappers that build the right backend and renderer:

```rust
pub async fn run_unified_inline(auth: AuthMethod) -> Result<()> {
    let mut backend = DirectModelCallBackend::new(ServiceFactory::for_chat(...).await?);
    let mut renderer = InlineRenderer::new(Theme::from_env())?;
    let mut session = ChatSession::load_or_new()?;
    run_chat_loop(&mut renderer, &mut backend, &mut session, CancelToken::new()).await
}

pub async fn run_chat_inline(agent_id: String, serve_url: String) -> Result<()> {
    let mut backend = HttpSidecarBackend::new(agent_id, serve_url);
    let mut renderer = InlineRenderer::new(Theme::from_env())?;
    let mut session = ChatSession::load_or_new()?;
    run_chat_loop(&mut renderer, &mut backend, &mut session, CancelToken::new()).await
}
```

After: `chat_inline.rs` shrinks from 4100 LOC to ~1500 LOC (input handling + slash commands stay; chat loop + backends extracted).

### Step 3 — Decide: delete `roko chat` or thin-wrap

Per plan 11 § Step 4. If keeping:

```rust
// crates/roko-cli/src/chat.rs (after)
pub async fn run_chat_repl(agent_id: String, serve_url: String) -> Result<()> {
    let mut backend = HttpSidecarBackend::new(agent_id, serve_url);
    let mut renderer = PlainRenderer::new();
    let mut session = ChatSession::ephemeral();
    chat_session_loop::run_chat_loop(&mut renderer, &mut backend, &mut session, CancelToken::new()).await
}
```

`extract_clean_text` is no longer needed (the backend produces typed `DispatchResult` events). Delete.

### Step 4 — Wire `ToolCallBlock` (and other primitives)

**File:** `crates/roko-cli/src/inline/primitives/tool_call.rs`

`ToolCallBlock` has collapsed/expanded views; not used by `push_tool_outputs`. Wire it:

```rust
// crates/roko-cli/src/render/inline.rs (InlineRenderer impl)
fn render_tool_call(&mut self, event: &ToolCallEvent) {
    let block = ToolCallBlock::new_collapsed(
        event.tool.clone(),
        event.args_summary.clone(),
        event.correlation_id.clone(),
    );
    self.terminal.push_lines(block.lines(&self.theme));
    self.pending_calls.insert(event.correlation_id.clone(), block);
}

fn render_tool_output(&mut self, event: &ToolOutputEvent) {
    if let Some(block) = self.pending_calls.get_mut(&event.correlation_id) {
        block.set_output(event.output.clone(), event.success, event.duration_ms);
        // re-render in place via terminal redraw
        self.terminal.replace_block_by_id(&event.correlation_id, block.lines(&self.theme));
    }
}
```

Same approach for `CostWaterfall`, `DiffBlock`, `ReplanBlock`, `SessionSummary`. Each becomes a method on `InlineRenderer`.

### Step 5 — TUI consumes `RuntimeProjection`

**Files:** `tui/dashboard.rs`, `tui/state.rs` (per plan 10 § 6)

```rust
// crates/roko-cli/src/tui/dashboard.rs (after)
pub struct DashboardData {
    projection: Arc<RuntimeProjection>,           // injected
}

impl DashboardData {
    pub fn refresh(&mut self) {
        // No more disk loading. Pull from projection.
        let view = self.projection.dashboard_view();
        self.runs = view.active_runs;
        self.cost_today = view.total_cost_today;
        // ...
    }
}
```

After: `dashboard.rs` shrinks from 6382 LOC to maybe 800 LOC. `state.rs` from 4968 LOC to ~1500 LOC (UI state only, no disk).

### Step 6 — TUI Agents tab renders tool outputs

**File:** `tui/views/agents.rs` (or wherever F3 lives)

Today F3 shows raw agent stdout. Use the same `ToolCallBlock`-aware rendering pipeline. The Agents tab's `AgentStreamClient` (`ws_client.rs`) already receives `StreamChunk::ToolCall { ... }` and `StreamChunk::ToolOutput { ... }` (after plan 10 extends `RuntimeEvent`). Render them via shared primitives, not raw text.

### Step 7 — Add tests

```rust
#[tokio::test]
async fn chat_loop_unified() {
    let mut renderer = TestRenderer::default();
    let mut backend = MockBackend::with_responses(vec![dispatch_result_with_tool_call()]);
    let mut session = ChatSession::ephemeral();

    backend.queue_user_input("test prompt");
    chat_session_loop::run_chat_loop(&mut renderer, &mut backend, &mut session, CancelToken::new()).await?;
    assert_eq!(renderer.tool_calls_rendered(), 1);
    assert_eq!(renderer.text_rendered(), "test response");
}

#[tokio::test]
async fn tui_dashboard_reads_projection() {
    let projection = test_projection_with_runs(vec![test_run(), test_run()]);
    let mut data = DashboardData::new(projection.clone());
    data.refresh();
    assert_eq!(data.runs.len(), 2);
    // No file system access during refresh
}
```

### Step 8 — Delete extract_clean_text

Per plan 01 § Step 7. This plan provides the proof that nothing inside CLI rendering paths still needs it.

---

## Anti-Patterns To Avoid

| ID | Manifestation | How to avoid |
|---|---|---|
| #7 Copy-paste | Re-implementing renderer in test | Test against `TestRenderer` |
| #10 God file | `chat_inline.rs` keeps growing because slash commands stay in it | Extract slash commands to `slash_commands.rs` (separate plan) |

---

## Things NOT To Do

1. **Don't merge `roko` (inline) and `roko dashboard` (TUI).** Different mental models. Keep both; share primitives only.
2. **Don't add live token streaming via WebSocket inside chat.** It already streams via the `ModelCallService::stream` (after plan 01 § Step 1). WebSocket is for `roko serve`-side observation.
3. **Don't make `ResponseRenderer` async.** Renderers are CPU-bound; async adds complexity without benefit.
4. **Don't break the ratatui `Viewport::Inline` mode.** `roko` and `roko run` rely on it for "scrollback friendly" rendering.
5. **Don't move primitives to a new crate.** They depend on `inline/terminal.rs` + theme; keeping them in `roko-cli/src/inline/` is fine.
6. **Don't keep `extract_clean_text` "behind a feature flag for legacy support".** After this plan, no callers exist; the function is dead.
7. **Don't add a per-mode session persistence.** One `ChatSession` schema; both modes load/save the same way (or `ephemeral` for REPL).

---

## Tests / Proof Criteria

```bash
# 1. Single chat loop
rg 'fn run_chat_inline|fn run_unified_inline|fn run_chat_repl' crates/roko-cli/src/ --type rust
# expected: 3 results, all <= 50 LOC each (just wrappers)

# 2. extract_clean_text gone
rg 'extract_clean_text' crates/ --type rust
# expected: 0

# 3. Primitives wired
rg 'ToolCallBlock|CostWaterfall|DiffBlock' crates/roko-cli/src/render/ --type rust
# expected: usage in InlineRenderer impl

# 4. TUI uses projection
rg 'load_from_disk' crates/roko-cli/src/tui/ --type rust
# expected: 0 (or only in tests)

# 5. chat_inline.rs is below 1500 LOC
wc -l crates/roko-cli/src/chat_inline.rs
# expected: < 1500
```

Functional proofs:

- [ ] All 2 unit tests above pass
- [ ] `roko` interactive: typing a prompt that triggers a tool call shows collapsed `⚙ tool_name preview (+N lines)`; pressing space expands to full output
- [ ] `roko chat --plain`: same conversation, plain text rendering, no markdown
- [ ] `roko dashboard` Agents tab F3: agent's tool calls appear as `ToolCallBlock`, not raw text
- [ ] `roko -q "do something"`: one-shot output is plain text without markdown bars
- [ ] After 5 prompts in chat: cost summary renders via `SessionSummary` primitive
- [ ] TUI startup time < 500ms (was ~2s with disk loading); verify via `time roko dashboard --duration 0`

---

## Dependencies

- **Plan 01 (ModelCallService)** — for typed `DispatchResult` everywhere
- **Plan 10 (Observability)** — for `RuntimeEvent` + `RuntimeProjection`
- **Plan 11 (Entry Point Convergence)** — for `roko chat` decision

---

## Estimated Effort

**L.** ~1.5-2 weeks. Risk: TUI is sensitive; visual regressions need manual eye-balling.

- Step 1 (ResponseRenderer trait + impls) — M (3 days)
- Step 2 (extract chat loop) — L (4-5 days; biggest)
- Step 3 (chat REPL) — S (1 day)
- Step 4 (wire primitives) — M (2-3 days)
- Step 5 (TUI on projection) — M (3-4 days; sensitive)
- Step 6 (TUI tool outputs) — S (1 day)
- Step 7 (tests) — S (1 day)
- Step 8 (delete extract_clean_text) — S (half day)
