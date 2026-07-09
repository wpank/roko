# CLI, Chat & TUI: Goals

## End State

One rendering pipeline, one chat loop, shared primitives. Plus: Generative Canvas — a visual workflow builder that feels like a video game / Lego set.

## Key Properties

- **One chat event loop**: Converge the 2 near-identical loops in chat_inline.rs.
- **Shared rendering primitives**: Inline and fullscreen TUI share code.
- **Tool output visibility**: ToolCallBlock wired to all rendering paths.
- **Generative Canvas**: Visual workflow builder with 3 views:
  - Recipe view (linear, Apple Shortcuts-style)
  - Graph view (DAG with typed cables)
  - Timeline view (Gantt waterfall)
- **12 primitive object types**: Agent, Extension, Connector, Gate, Feed, Recipe, Plan, Scorer, Arena, Group, Knowledge, Config.
- **DAW-inspired controls**: Rotary knobs (floats), steppers (integers), toggles (booleans), segmented controls (enums).
- **Intuitive and interactive**: Feels like building in a video game. Drag, connect, test, iterate.

## What Exists Today

- `chat_inline.rs` (4,100 lines, 2 near-identical loops: `run_chat_inline` at line 932, `run_unified_inline` at line 1193)
- `roko agent chat` on TTY already routes to `run_chat_inline` (full inline UX) — the separate `run_chat_repl` in `chat.rs` is only the non-TTY fallback
- Inline + fullscreen TUI share zero code (same `Theme` struct loaded independently)
- `ToolCallBlock` (`inline/primitives/tool_call.rs`, 241 lines) exists but `push_tool_outputs()` does not use it — builds its own rendering from scratch
- No visual workflow builder

## From v2 UX Showcase (9 Scenarios)

### Rendering Primitives (16 message types, all need TUI equivalents)
- **ToolCall card** — collapsible, with kind icon (10 types), status dot, role badge, location, cost, tokens, mcpServer pill, neuro pill
- **DiffBlock** — file path header, old (red −) / new (green +) lines, hunk count
- **TerminalBlock** — command header, streaming output with colors, running/exit status
- **PlanList** — checklist with status icons (completed/in_progress/pending/failed), role badges, priority pills, replan badge, counter (N/M)
- **PermissionRequest** — title, description, scope tags, allow/reject buttons
- **KnowledgeCard** — score pills, source paths, text, hit count
- **RouterTrace** — policy, candidates with score bars + reasons
- **SwarmGrid** — N agent cards with progress bars, gate dots, winner crown
- **CallGraph** — indented tree with fn → file references
- **AgentChat** — role badge → recipient, colored left border
- **StepCard** — numbered step (N/M), title, thought text
- **GateRow** — horizontal gate strip with status dots + durations
- **ModeChange / PhaseChange** — inline pills with from→to
- **Checkpoint** — commit hash, file count, Restore button

### Layout (4-column in editor, equivalent TUI panels)
- **ThreadsSidebar** — project tree with agents, worktrees, status dots
- **EditorPeek** — code with agent cursor highlighting + gate gutter marks
- **RightRail panels** — Cost, Router, Knowledge, MCP, EpisodeScrubber, PermissionScope
- **PhaseStrip** — FSM breadcrumb (8 phases) with active indicator
- **ConfigBar** — 11 controls (model, effort, temp, routing, workflow, review, compile, test, clippy, max_iter, mode)
- **StatusBar** — connection, version, phase, gate count, role legend
- **Brand toggle** — roko (orange) vs neutral (purple) theme

## Gap

- Converge chat loops (medium) — extract shared `ChatSession`, `render_viewport()`, `handle_input_key()` so both `run_chat_inline` and `run_unified_inline` share one implementation
- Share rendering primitives (medium) — inline and TUI loaded Theme separately; no shared markdown/tool-call renderer
- Wire `ToolCallBlock` to `push_tool_outputs()` (small) — currently `push_tool_outputs()` bypasses the existing `tool_call.rs` primitive
- Agent following UI (cursor tracking + traversal trace)
- Multi-agent progress panel (N parallel threads)
- Synthesis/comparison table renderer
- Plan card renderer (task list with status indicators)
- Generative Canvas (very large — entirely new frontend)

---

## Sources

- `crates/roko-cli/src/chat_inline.rs` — both inline chat loops, slash commands, session persistence paths
- `crates/roko-cli/src/chat.rs` — `run_chat_repl` (non-TTY fallback), `extract_clean_text`
- `crates/roko-cli/src/unified.rs` — `cmd_unified_chat`, `cmd_oneshot_inline`
- `crates/roko-cli/src/agent_serve.rs` — `AgentCmd::Chat` routing to `run_chat_inline`
- `crates/roko-cli/src/inline/primitives/tool_call.rs` — `ToolCallBlock` definition
- `crates/roko-cli/src/tui/theme.rs` — ROSEDUST palette constants
