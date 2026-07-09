# Demo + CLI UX Implementation Plan

**Goal:** Upgrade roko's CLI to a Claude Code-tier experience using ratatui inline viewport, then use those same primitives for the investor demo.

---

## Architecture: Inline Ratatui

The core insight: **ratatui `Viewport::Inline(N)` renders N lines at the bottom of the terminal without entering alternate screen.** Completed output scrolls up into terminal history via `insert_before()`. This is exactly how Claude Code works (React/Ink), but in Rust with ~20MB memory vs ~300MB.

```
Terminal scrollback (grows upward via insert_before)
┌──────────────────────────────────────────────────────┐
│ ◆ agent    auditor@v1 · eid://roko/auditor.v1       │ ← completed block
│ │ predict  $0.043 · 12.4s · route: haiku            │    (scrollback)
│ │ gates    secret_scan ✔  cost_ceiling ✔             │
│ │ actual   $0.031 (-28%) · routed to haiku           │
│ └ chain    anchored block #4,821                     │
│                                                      │
│ ◆ agent    researcher@v2 · eid://roko/researcher.v2  │ ← completed block
│ │ predict  $0.038 · 10.1s · route: haiku             │    (scrollback)
│ │ knowledge loaded 9 engrams (4 agents, 0.93 conf)   │
│ │ actual   $0.022 (-42%) · routed to haiku           │
│ └ deposited 1 new engram → /finance/q3               │
├──────────────────────────────────────────────────────┤
│ ◆ running  analyst@v1 · step 3/7                     │ ← inline viewport
│ │ The Q3 earnings data shows margin compression      │    (live, redraws)
│ │ across mid-cap fintech names, particularly in...█  │
│ ├─────────────────────────────────────────────────── │
│ │ cost: $0.018  tokens: 2,341  ━━━━━━━━░░ 62%       │ ← status bar
└──────────────────────────────────────────────────────┘
```

### Why this works

| Feature | Fullscreen TUI (`roko dashboard`) | Inline TUI (`roko run`, `roko audit`, `roko chat`) |
|---|---|---|
| Screen mode | Alternate screen | Normal scrollback |
| History | Lost on exit | Preserved in terminal |
| Widget reuse | Yes | Yes (same ratatui primitives) |
| Copy-paste | Hard (TUI captures input) | Normal terminal behavior |
| Composable | No (owns the screen) | Yes (pipe, redirect, nest) |
| Demo-friendly | Requires tab navigation | Linear narrative flow |

### Crate stack

```toml
# Already in Cargo.toml:
ratatui = { version = "0.29", features = ["scrolling-regions"] }
crossterm = { version = "0.28", features = ["event-stream"] }

# Add:
cliclack = "0.3"           # Clack-style prompts (intro/outro/select/spinner)
tui-markdown = "0.3"       # Render markdown as ratatui Text
# OR:
streamdown-render = "..."  # Purpose-built streaming markdown for LLM output
```

---

## Output Primitives

These are the reusable building blocks. Every `roko` command composes from this set.

### Primitive 1: `RunBlock` — Completed run summary

Pushed into scrollback via `insert_before` when a run finishes. The clack-style structured output from the demo doc.

```
◆ agent      auditor@v1  ·  eid://roko/auditor.v1  (attested)
│ predict    $0.043  ·  12.4s  ·  route: haiku → sonnet (verification)
│ gates      secret_scan ✔   cost_ceiling ✔   policy:prod-sec ✔
│ knowledge  loaded 7 engrams from /infra/payments-svc (3 agents, 0.91 conf)
│ actual     $0.031  (-28% vs predicted)  ·  routed to haiku
│ chain      anchored block #4,821  ·  mirage-rs local
└ deposited  2 new engrams → /infra/payments-svc
```

Rendered as a ratatui `Paragraph` with styled `Line`/`Span` sequences. Reusable across `roko run`, `roko audit`, `roko plan run`, `roko agent chat`.

### Primitive 2: `StreamingBlock` — Live agent output

Rendered in the inline viewport. Updates on every token delta. Supports markdown via tui-markdown or streamdown.

```
◆ streaming  analyst@v1  ·  step 3/7
│ The Q3 earnings data shows margin compression across
│ mid-cap fintech names, particularly in payment processing
│ where interchange revenue declined 12% QoQ...█
```

Features:
- Token-by-token append with cursor
- Markdown formatting (headers, code blocks, lists) as tokens arrive
- Auto-scroll to bottom
- Manual scroll-back with arrow keys (pauses auto-scroll)

### Primitive 3: `ToolCallBlock` — Tool invocation display

Collapsed by default (one line), expandable. Pushed to scrollback on completion.

```
│ ▸ ReadFile  src/payments/handler.rs  (247 lines, 0.3s)
```

Expanded:
```
│ ▾ ReadFile  src/payments/handler.rs  (247 lines, 0.3s)
│   │ fn handle_payment(req: PaymentRequest) -> Result<Receipt> {
│   │     let secret = env::var("AWS_SECRET")?;  // ← line 14
│   │     ...
│   │ }
```

### Primitive 4: `GateBlock` — Gate pipeline progress

Rendered in the inline viewport during gate execution. Each rung updates in-place.

```
◆ gates      7 rungs  ·  policy: prod-sec
├ compile    ✔  0 errors (142 crates, 2.1s)
├ clippy     ✔  0 warnings (0.8s)
├ test       ━━━━━━░░░░ 4/11 tests  (running...)
├ secret_scan  ⏳ pending
├ diff       ⏳ pending
├ llm_judge  ⏳ pending
└ verify     ⏳ pending
```

Rung states: `⏳ pending` → `━━━ running` → `✔ pass` / `✖ fail` / `⚠ warn`

### Primitive 5: `CostMeter` — Real-time cost display

Rendered in the status bar (bottom line of inline viewport).

```
cost: $0.031  tokens: 4,821 in / 1,203 out  cache: 87%  route: haiku  ━━━━━━━━░░ 62%
```

Updates on every token delta. Shows:
- Running cost USD
- Token counts (input/output)
- Cache hit rate
- Active model
- Progress bar (if step count known)

### Primitive 6: `KnowledgeBlock` — Knowledge query result

Shown before agent dispatch, inline.

```
│ knowledge  loaded 7 engrams from /finance/q3
│            ├ "Q3 interchange revenue declined 12% QoQ" (0.94 conf, researcher@v2)
│            ├ "Margin compression in mid-cap fintech" (0.91 conf, analyst@v1)
│            └ +5 more (avg 0.88 conf, 3 agents)
```

Collapsed (default): one line with count. Expanded: shows top engrams with confidence and source agent.

### Primitive 7: `PredictionBlock` — Cost/time/route prediction

Shown before dispatch.

```
│ predict    $0.043  ·  12.4s  ·  route: haiku → sonnet (verification only)
│            reason: task complexity LOW (0.94 conf), haiku sufficient
│            baseline: $1.34 (opus, no cache, no routing) → 31.2x savings
```

Collapsed (default): one line. Expanded: shows routing rationale and baseline comparison.

### Primitive 8: `AuditStepBlock` — Audit pipeline step

For `roko audit`. Each step is a self-contained block.

```
├── step 2/8  secret scan
│   scanning 47 files for credential patterns...
│   ✖  CRITICAL: AWS_SECRET_ACCESS_KEY found in deploy/env.yaml:14
│   ⚠  audit halted — violation recorded, remediation required
```

### Primitive 9: `ReplanBlock` — Gate failure + auto-replan

When a gate fails and the system replans automatically.

```
│ gate       test ✖  assertion error in handler.rs:42
│ replan     escalating to sonnet (confidence: 0.67 → requires 0.85)
│ retry      attempt 2/3  ━━━━━━━━━━ done in 4.2s
│ gate       test ✔  all assertions pass
│ actual     $0.058 (replan cost: +$0.027, still 23x vs baseline)
```

### Primitive 10: `SessionSummary` — End-of-session roll-up

Shown at the end of a multi-run session (demo, plan run, audit).

```
◆ session summary
│ runs          3
│ total cost    $0.084  (baseline: $2.61, savings: 31.1x)
│ cache hit     87%  (↑ from 0% on first run)
│ route drift   haiku 94% → 97%  (opus not needed for this domain)
│ knowledge     10 engrams deposited, 16 loaded
│ gates         24/24 passed  (2 replans, both succeeded)
└ verdict       domain /finance/q3 is haiku-safe at 0.95 confidence
```

### Primitive 11: `CostWaterfall` — Decomposed savings

Shows where the cost reduction comes from.

```
◆ cost waterfall (this session)
│ baseline (opus, no cache, no routing)     $2.61
│ ├── prompt caching                       -$1.31  (5.0x)
│ ├── cascade routing (haiku)              -$0.78  (3.1x)
│ ├── knowledge pre-load                   -$0.29  (1.4x)
│ └── gate early-exit                      -$0.14  (1.2x)
│ actual session cost                       $0.084
│ savings ratio                             31.1x
└ methodology: real tokens, real prices, this session
```

### Primitive 12: `ChatInput` — Input box

Rendered at the bottom of the inline viewport. Supports:
- Multi-line input (shift+enter)
- History (up/down arrows)
- Autocomplete for commands (`/help`, `/share`, `/cost`)
- Vim mode (optional)

```
╭───────────────────────────────────────────────────╮
│ > Draft a Q3 earnings analysis for the CRO█       │
╰───────────────────────────────────────────────────╯
```

### Primitive 13: `SpinnerLine` — Inline spinner

For operations that don't have a progress bar.

```
│ ◌ compiling gate pipeline... (2.1s)
```

Uses ratatui `Atmosphere::spinner()` (already exists) for braille animation: `⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏`

### Primitive 14: `DiffBlock` — Inline diff display

Shows file changes from agent actions.

```
│ ▸ diff  3 files changed, +42 -17
│   deploy/env.yaml:14  removed AWS_SECRET_ACCESS_KEY
│   deploy/env.yaml:15  added ref to AWS Secrets Manager ARN
│   src/handler.rs:42   updated credential loading path
```

### Primitive 15: `ChainAnchor` — Chain confirmation

```
│ chain      anchored block #4,821 · tx 0xab3f...7e21 · mirage-rs local
│            identity: eid://roko/auditor.v1 (SOVEREIGN tier, passport #7)
│            proof: episode hash 0x8f2a...3c19 stored on-chain
```

### Primitive 16: `ProgressTree` — Hierarchical progress

For `roko plan run` with multiple tasks.

```
◆ plan  deploy-audit  ·  8 tasks  ·  3 waves
├ wave 1  ━━━━━━━━━━ 3/3 ✔
│ ├ T01 dependency-scan    ✔  $0.012  2.1s
│ ├ T02 secret-scan        ✔  $0.008  1.4s
│ └ T03 policy-check       ✔  $0.031  4.2s
├ wave 2  ━━━━━━░░░░ 1/3
│ ├ T04 integration-test   ━━━━━━ running (6.2s)
│ ├ T05 diff-review        ⏳ blocked by T04
│ └ T06 cost-analysis      ⏳ blocked by T04
└ wave 3  ⏳ 0/2
  ├ T07 episode-log        ⏳
  └ T08 chain-anchor       ⏳
```

### Primitive 17: `ApprovalPrompt` — Interactive approval

For human-in-the-loop decisions (agent tool approval, destructive actions).

```
╭────────────────────────────────────────────────────╮
│  ⚠  Agent wants to execute: rm -rf deploy/staging  │
│                                                    │
│  [y] Allow   [n] Deny   [e] Edit command           │
╰────────────────────────────────────────────────────╯
```

Uses `cliclack::confirm()` or custom ratatui widget.

### Primitive 18: `ErrorBlock` — Structured error display

```
│ ✖  gate failed: compile (rung 1/7)
│    error[E0308]: expected `i32`, found `String`
│      --> src/handler.rs:42:18
│    42 │     let cost: i32 = calculate_cost();
│       │                     ^^^^^^^^^^^^^^^^ expected i32, found String
│
│    retry in 10s (attempt 1/3, exponential backoff)
```

---

## How Primitives Compose for Each Command

### `roko run "<prompt>"`

```
[ChatInput]           ← user types prompt
[PredictionBlock]     ← cost/route prediction
[KnowledgeBlock]      ← engrams loaded from neuro store
[StreamingBlock]      ← live agent output (inline viewport)
  [ToolCallBlock]*    ← tool calls during execution (pushed to scrollback)
[GateBlock]           ← gate pipeline (in viewport, then scrollback)
[RunBlock]            ← final summary (scrollback)
[CostMeter]           ← status bar throughout
```

### `roko audit deployment <svc>`

```
[AuditStepBlock]*     ← 8 steps, sequential
  [ToolCallBlock]*    ← tool calls within steps
  [GateBlock]         ← per-step gates
[ReplanBlock]?        ← if gate failure triggers replan
[RunBlock]            ← per-step summary
[SessionSummary]      ← audit roll-up
[ChainAnchor]         ← chain anchoring of audit trail
```

### `roko plan run <dir>`

```
[ProgressTree]        ← hierarchical plan progress (inline viewport)
  [RunBlock]*         ← per-task completion (scrollback)
  [ReplanBlock]*      ← gate failures + replans
  [ErrorBlock]*       ← permanent failures
[SessionSummary]      ← plan roll-up
[CostWaterfall]       ← decomposed savings
```

### `roko chat --agent <name>` (Claude Code-like)

```
loop {
  [ChatInput]         ← user types message
  [StreamingBlock]    ← live agent response (inline viewport)
    [ToolCallBlock]*  ← tool calls (collapsed, pushed to scrollback)
  [CostMeter]         ← per-turn cost in status bar
}
[SessionSummary]      ← on exit
```

### `roko agent list`

```
[AgentTable]          ← formatted table with identity, status, last-seen
```

### `roko status`

```
[SessionSummary]      ← current workspace state
[CostWaterfall]       ← cumulative savings
```

---

## Mori-Diffs Feature Mapping

How the inline primitives support features from `tmp/mori-diffs/`:

| Mori-Diff Feature | Primitive(s) Used |
|---|---|
| Agent dispatch visibility (01) | `PredictionBlock`, `RunBlock`, routing reason |
| Plan execution progress (02) | `ProgressTree`, `GateBlock`, backoff timer in `SpinnerLine` |
| Persistence/resume display (03) | `RunBlock` on resume shows "resuming from task N" |
| Learning feedback (04) | `SessionSummary` shows routing drift, knowledge tier progression |
| Composition auction visibility (09) | `PredictionBlock` expanded: prompt budget allocation per section |
| Dreams consolidation (10) | New `DreamBlock` primitive: shows hypnagogia/imagination results |
| Parallel merge (11) | `ProgressTree` shows merge queue status per plan |
| Affect routing (12) | `PredictionBlock` shows affect state: "confidence: high, stress: low" |
| Knowledge lifecycle (13) | `KnowledgeBlock` shows tier (seedling/established/canonical) |
| Gate failure feedback (02) | `ErrorBlock` with parsed file:line:code, `ReplanBlock` for auto-retry |
| Observability facade (16) | All primitives emit `DashboardEvent` for TUI + HTTP + SSE consumers |
| Stability/chaos (22) | `ErrorBlock` + `ReplanBlock` visualize chaos test results |

---

## Reuse Between Inline CLI and Full-Screen Dashboard

The key architectural win: **primitives are ratatui widgets that work in both viewport modes.**

```rust
/// A completed run summary. Renders the same in inline and fullscreen.
pub struct RunBlockWidget<'a> {
    data: &'a RunBlockData,
    expanded: bool,
}

impl Widget for RunBlockWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Same rendering code in both modes
        let lines = self.data.to_styled_lines(self.expanded);
        Paragraph::new(lines).render(area, buf);
    }
}
```

In inline mode: rendered via `insert_before()` into scrollback.
In fullscreen: rendered in the Agents tab (F3) or Plans tab (F2).

This means:
- Building the demo output formatter also builds half the dashboard widgets
- `roko chat` and `roko dashboard` share the same `StreamingBlock` for agent output
- Gate progress looks identical whether you're in `roko run` or watching F2

---

## Decisions

- **Pretty by default**: TTY-detected. If stdout is a TTY → full inline ratatui. If piped/CI → plain text with ANSI stripped.
- **Chat first**: `roko chat` is the hero deliverable. It IS the demo. The demo primitives are what chat renders.
- **Viewport height**: Dynamic — 10 lines default, grows up to 20 for streaming, shrinks for short status. Configurable via `ROKO_VIEWPORT_HEIGHT`.
- **Markdown**: Both `tui-markdown` (for static blocks in scrollback) and `streamdown-render` (for live streaming in viewport).
- **Dashboard chat embed**: Tabled for now. Inline chat is the priority.

---

## Implementation Phases

### Phase 1: Inline Engine + Core Primitives

The foundation everything else builds on. The `InlineTerminal` wrapper manages the viewport lifecycle and the `insert_before` scrollback.

**New files:**
```
crates/roko-cli/src/inline/
  mod.rs              — Public API: InlineTerminal, InlineEvent
  engine.rs           — Event loop: tokio::select over agent_rx, input_rx, tick
  terminal.rs         — InlineTerminal wrapper (Viewport::Inline + insert_before)
  primitives/
    mod.rs            — Re-exports all primitives
    run_block.rs      — Primitive 1: completed run summary
    streaming.rs      — Primitive 2: live agent output with markdown
    tool_call.rs      — Primitive 3: tool invocation (collapsible)
    gate_block.rs     — Primitive 4: gate pipeline progress
    cost_meter.rs     — Primitive 5: real-time cost status bar
    knowledge.rs      — Primitive 6: knowledge query result
    prediction.rs     — Primitive 7: cost/time/route prediction
    audit_step.rs     — Primitive 8: audit pipeline step
    replan.rs         — Primitive 9: gate failure + auto-replan
    session_summary.rs — Primitive 10: end-of-session roll-up
    cost_waterfall.rs — Primitive 11: decomposed savings
    chat_input.rs     — Primitive 12: input box (multiline, history, autocomplete)
    spinner.rs        — Primitive 13: inline spinner
    diff_block.rs     — Primitive 14: inline diff display
    chain_anchor.rs   — Primitive 15: chain confirmation
    progress_tree.rs  — Primitive 16: hierarchical progress
    approval.rs       — Primitive 17: interactive approval prompt
    error_block.rs    — Primitive 18: structured error display
  theme.rs            — Clack symbols + ROSEDUST palette + typography rules
  markdown.rs         — Streaming markdown: streamdown for live, tui-markdown for static
  plaintext.rs        — Non-TTY fallback renderer (same data, no ratatui)
```

**Key types:**
```rust
/// Inline terminal that renders a live viewport at the bottom
/// and pushes completed blocks into terminal scrollback.
pub struct InlineTerminal {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    viewport_height: u16,
    is_tty: bool,
}

impl InlineTerminal {
    pub fn new() -> Result<Self> {
        let is_tty = std::io::stdout().is_terminal();
        if !is_tty {
            // Plain text mode — no ratatui, just formatted println
            return Ok(Self { /* plaintext backend */ });
        }
        let height = std::env::var("ROKO_VIEWPORT_HEIGHT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(10);
        let terminal = ratatui::init_with_options(TerminalOptions {
            viewport: Viewport::Inline(height),
        });
        Ok(Self { terminal, viewport_height: height, is_tty })
    }

    /// Push a completed block into scrollback (above the viewport)
    pub fn push(&mut self, block: &dyn InlineBlock) -> Result<()> {
        let height = block.height();
        self.terminal.insert_before(height, |buf| {
            block.render(buf.area, buf);
        })
    }

    /// Redraw the live viewport area (streaming text + status bar)
    pub fn draw(&mut self, f: impl FnOnce(&mut Frame)) -> Result<()> {
        self.terminal.draw(f)
    }

    /// Restore terminal on drop
    pub fn restore(&mut self) { ratatui::restore(); }
}

/// Trait for anything that can be pushed into scrollback
pub trait InlineBlock {
    fn height(&self) -> u16;
    fn render(&self, area: Rect, buf: &mut Buffer);
    /// Plain text fallback for non-TTY
    fn to_plain(&self) -> String;
}
```

### Phase 2: `roko chat` — Claude Code-Like Experience (HERO)

This is the primary deliverable. The chat becomes the demo.

**Upgrade `crates/roko-cli/src/chat.rs`** from line-oriented REPL to inline ratatui.

**Architecture:**
```
┌─────────────────────────────────────────────────┐
│ Terminal scrollback (grows via insert_before)    │
│                                                 │
│ You: Summarize Q3 fintech earnings              │ ← user message block
│                                                 │
│ ◆ researcher@v2 · eid://roko/researcher.v2      │ ← agent header
│ │ predict  $0.043 · route: haiku                │
│ │ knowledge  7 engrams loaded                   │
│ │                                               │
│ │ The Q3 earnings data shows significant margin │ ← agent response (markdown)
│ │ compression across mid-cap fintech names:     │
│ │                                               │
│ │ | Company    | Revenue | Margin |             │
│ │ |------------|---------|--------|             │
│ │ | Stripe     | $4.2B   | 23%   |             │
│ │ | Block      | $5.1B   | 18%   |             │
│ │                                               │
│ │ ▸ ReadFile earnings/q3.csv (142 lines, 0.2s) │ ← collapsed tool call
│ │ ▸ Search "fintech margin 2026" (3 results)   │ ← collapsed tool call
│ │                                               │
│ │ actual  $0.031 (-28%) · 4,821 tokens · haiku │
│ └──────────────────────────────────────────────│
├─────────────────────────────────────────────────┤
│ ◌ Thinking... (2.3s)                            │ ← live viewport
│ │ Analyzing the data you provided, I can see    │    (streaming)
│ │ that the trend continues into...█             │
│ ├──────────────────────────────────────────────│
│ │ $0.018 · 2,341 tokens · haiku · ━━━━━━░░ 62%│ ← status bar
│ ╰───────────────────────────────────────────── │
│ > █                                             │ ← input (below viewport)
└─────────────────────────────────────────────────┘
```

**Event loop:**
```rust
// Simplified — real version in engine.rs
loop {
    tokio::select! {
        // User pressed Enter in ChatInput
        Some(msg) = input_rx.recv() => {
            // Push user message to scrollback
            term.push(&UserMessageBlock { text: msg.clone() })?;
            // Show spinner in viewport
            state.phase = Phase::Thinking;
            // Dispatch to agent
            agent_tx.send(msg).await?;
        }

        // Agent token delta
        Some(delta) = agent_rx.recv() => {
            match delta {
                AgentEvent::TokenDelta(t) => {
                    state.streaming_buffer.push_str(&t);
                    state.phase = Phase::Streaming;
                }
                AgentEvent::ToolCallStart { name, input } => {
                    // Push current streaming to scrollback
                    term.push(&state.take_streaming_block())?;
                    state.active_tool = Some(ToolCallBlock::new(name, input));
                    state.phase = Phase::ToolCall;
                }
                AgentEvent::ToolCallDone { result, duration } => {
                    // Push collapsed tool call to scrollback
                    let tool = state.active_tool.take().unwrap();
                    tool.set_result(result, duration);
                    term.push(&tool)?;
                    state.phase = Phase::Streaming;
                }
                AgentEvent::Usage(usage) => {
                    state.cost_meter.update(usage);
                }
                AgentEvent::Done => {
                    // Push final streaming block + run summary to scrollback
                    term.push(&state.take_streaming_block())?;
                    term.push(&state.build_run_block())?;
                    state.phase = Phase::Input;
                }
            }
        }

        // 30fps redraw tick
        _ = tick.tick() => {
            term.draw(|frame| {
                match state.phase {
                    Phase::Input => render_input(frame, &state),
                    Phase::Thinking => render_spinner(frame, &state),
                    Phase::Streaming => render_streaming(frame, &state),
                    Phase::ToolCall => render_tool_progress(frame, &state),
                }
                render_status_bar(frame, &state.cost_meter);
            })?;
        }

        // Crossterm key event
        Some(key) = key_rx.recv() => {
            handle_key(key, &mut state)?;
        }
    }
}
```

**Chat features:**
- Multi-line input (shift+enter for newline)
- Up/down for history
- `/` commands: `/help`, `/share`, `/cost`, `/clear`, `/model <name>`, `/gate <on|off>`
- Ctrl+C interrupts current generation (like Claude Code)
- Streaming markdown rendering (code blocks with syntax highlighting)
- Collapsible tool calls (click or `e` to expand)
- Cost tracking per message and session cumulative
- Knowledge query before each dispatch (shown as KnowledgeBlock)
- Gate results shown inline after agent output
- Session persistence (`.roko/chat-sessions/{id}.json`)

### Phase 3: Wire `roko run` Through Inline Engine

Now `roko run "<prompt>"` uses the same engine as chat, but single-shot:

```
$ roko run "Fix the failing test in src/auth.rs"

◆ researcher@v2 · eid://roko/researcher.v2 (attested)
│ predict  $0.043 · 12.4s · route: haiku → sonnet (verification)
│ knowledge  loaded 3 engrams from /code/auth (2 agents, 0.89 conf)
│
│ I'll fix the failing test. The issue is in the credential
│ validation logic...
│
│ ▸ ReadFile src/auth.rs (247 lines, 0.2s)
│ ▸ Edit src/auth.rs:42 (+3 -1)
│ ▸ RunTests src/auth.rs (11 pass, 0 fail, 1.8s)
│
│ gates    compile ✔  test ✔  clippy ✔  diff ✔
│ actual   $0.031 (-28%) · 4,821 tokens · haiku
│ chain    anchored block #4,821 · mirage-rs
└ deposited 1 new engram → /code/auth
```

Same primitives, just no input loop.

### Phase 4: `roko audit` Subcommand

```
$ roko audit deployment payments-svc --rev=abc123 --policy=prod-sec

◆ audit  deployment payments-svc · rev abc123 · policy prod-sec
│
├── step 1/8  dependency scan
│   ✔  0 vulnerabilities (142 crates, 2.1s)
│
├── step 2/8  secret scan
│   ✖  CRITICAL: AWS_SECRET_ACCESS_KEY in deploy/env.yaml:14
│   ⚠  audit halted — violation recorded in episode log
│
│   "The agent didn't close the loop — the coordination plane did."
│
├── step 3/8  policy compliance  (blocked by step 2 violation)
│   ...
│
│ Ctrl+C to abort, or wait for remediation agent...
```

### Phase 5: `--share` + Shareable URL

- Every command can `--share`: persist output as `RunTranscript` JSON
- `roko serve` route `GET /runs/{id}` renders self-contained HTML
- Same visual language (clack symbols, ROSEDUST colors in CSS)
- `cloudflared tunnel` for public URL
- Print share URL at end: `share  https://abc123.trycloudflare.com/runs/7f3a`

### Phase 6: Response Cache + Demo Polish

- Blake3-keyed file cache at dispatch layer
- Pre-seeded /finance/ domain knowledge + cached responses
- `demo-magic` script + asciinema recording as backup
- Practice script with timing notes

---

## Symbols & Theme Reference

### Clack-style symbols (no emoji)
```
◆  section start (filled diamond)
◇  section start (empty, for pending)
│  continuation line
├  branch (more items follow)
└  last item
✔  pass / success (green)
✖  fail / error (red)
⚠  warning (yellow)
ℹ  info (blue)
❯  prompt arrow
→  routing / flow arrow
·  separator (interpunct U+00B7)
━  progress bar fill
░  progress bar empty
⏳ pending / waiting
◌  spinner base
▸  collapsed (right triangle)
▾  expanded (down triangle)
█  cursor
```

### ROSEDUST color palette (matches existing TUI theme)
```rust
const TEXT: Color     = Color::Rgb(165, 142, 158);  // primary text
const DIM: Color      = Color::Rgb(145, 120, 138);  // secondary text
const ROSE: Color     = Color::Rgb(185, 120, 148);  // accent / highlight
const SAGE: Color     = Color::Rgb(125, 158, 140);  // success / pass
const WARNING: Color  = Color::Rgb(195, 155, 95);   // warning
const EMBER: Color    = Color::Rgb(195, 110, 85);   // error / fail
const DREAM: Color    = Color::Rgb(120, 115, 165);  // info / chain
const BG: Color       = Color::Rgb(22, 18, 24);     // background
const BG_HL: Color    = Color::Rgb(34, 28, 36);     // highlighted background
```

### Typography
```
Labels:    bold, ROSE accent
Values:    regular, TEXT
Secondary: DIM
Pass:      SAGE + ✔
Fail:      EMBER + ✖
Warning:   WARNING + ⚠
Pending:   DIM + ⏳
Running:   ROSE + spinner
Cost:      SAGE when under prediction, EMBER when over
```

---

## What This Enables Long-Term

Once the inline engine + primitives exist:

1. **Every `roko` command gets beautiful output for free** — just compose primitives
2. **`roko chat` becomes Claude Code-tier** — streaming markdown, tool calls, cost tracking
3. **`roko plan run` shows live progress** without needing the full dashboard
4. **`roko dashboard` reuses the same widgets** — no duplication
5. **Demo is just running real commands** — no fake output, no separate demo binary
6. **`--share` works everywhere** — any command can persist its output as a shareable page
7. **Non-TUI fallback** — if stdout isn't a TTY (CI, pipe), primitives render as plain text with ANSI stripped
8. **Observability facade** — primitives emit `DashboardEvent`s, consumed by TUI, HTTP/SSE, and inline CLI simultaneously

---

## Decisions (Answered)

| Question | Answer |
|---|---|
| Default output mode | Pretty by default, TTY-detected. Pipe → plain text. |
| Priority | Chat first — `roko chat` is the hero. Demo primitives serve chat. |
| Viewport height | Dynamic, 10 default, configurable via `ROKO_VIEWPORT_HEIGHT` |
| Dashboard chat embed | Tabled for now. Inline chat is the priority. |
| Markdown renderer | Both: `streamdown-render` for live streaming, `tui-markdown` for static blocks |
| Build approach | Demo-first pragmatism — build what ships, generalize as we go |
| Binary name | `roko` (no rename) |
| Chain/identity | Real mirage-rs + formatted display |
| Python SDK | No — Rust only |
| `--share` | Yes, good to have |
| Response cache | Yes, file-backed blake3 cache |
| Meeting format | Screen share call |
