# 26 - Agent Output Rendering and Streaming Experience Audit

**Date**: 2026-09-01
**Scope**: `crates/roko-cli/src/tui/views/agents_view.rs`, `crates/roko-cli/src/tui/state.rs`, `crates/roko-cli/src/tui/segment.rs`, `crates/roko-cli/src/tui/ansi.rs`

---

## 1. How is agent output stored?

Agent output is stored as **flat `Vec<String>` line buffers** with no structured metadata. There are three distinct storage paths that the TUI merges at render time:

| Storage | Type | Source | Limit |
|---|---|---|---|
| `AgentRow::output_lines` | `Vec<String>` | Episodes or snapshot `task_outputs` | `MAX_AGENT_OUTPUT_LINES = 50` |
| `AgentRow::last_output_line` | `String` | Last line of output (single string) | 1 line |
| `AgentStream::chunks` | `VecDeque<String>` | WebSocket live tail | `MAX_AGENT_STREAM_CHUNKS = 200` |
| `TuiState::task_output_tails` | `HashMap<String, Vec<String>>` | Push-mode per-task tails from `DashboardData` / `DashboardSnapshot` | 50 (via `bounded_output_lines`) |

The collection priority in `collect_agent_output_lines()` is:
1. Live push-mode `task_output_tails` for the agent's current task
2. Agent row `output_lines` from `tui_state.agents`
3. Episode `extra` fields (`stderr`, `agent_stderr`, `output`, `stdout`, `agent_output`, `output_tail`)

**Finding**: Output is stored as raw text lines with no structural annotations (no message roles, no tool call boundaries, no thinking markers embedded in the data model). All semantic classification happens at render time via heuristic text parsing. The 50-line cap on `output_lines` means most of an agent's output history is discarded before the TUI ever sees it.

**Gap**: No persistent scrollback buffer or virtual-list architecture for large outputs. Once lines roll past the 50-line window, they are gone from the TUI's perspective (though episodes.jsonl may still have them).

---

## 2. Is output parsed into semantic segments?

**Yes**, via `segment.rs`. The `parse_segment_groups()` function classifies every line into one of 8 `SegmentKind` variants:

| Kind | Detection heuristic |
|---|---|
| `Thinking` | Default fallback -- any line that matches nothing else |
| `Heading` | Starts with `# `, `## `, or `### ` |
| `ToolUse` | Starts with one of: `"> "`, `"$ "`, `"Running "`, `"Reading "`, `"Writing "`, `"Editing "`, `"Created "` |
| `Code` | Inside ```` ``` ```` fences, or 4+ leading spaces after a blank line |
| `Success` | Contains checkmark, `PASS`, `APPROVE`, or starts with `ok ` |
| `Error` | Contains `ERROR`, `FAILED`, `REVISE`, `error[`, or starts with `error` |
| `Blank` | Empty after trim |
| `TurnMarker` | Starts with `"----"` (four or more horizontal rule characters) |

Consecutive lines of the same kind are merged into `SegmentGroup` objects. Each group gets a **styled header line** rendered above it (e.g., ` TOOL USE --------`), making the output visually segmented.

**Preprocessing** applies two transformations before classification:
- `insert_prompt_breaks()`: Inserts newlines before sentence-starting words (`"Now "`, `"Let "`, `"I'll "`, etc.) to break up long reasoning lines.
- `split_on_sentences()`: Hard-wraps lines longer than 120 characters at sentence boundaries.

**Finding**: The heuristics are fragile and text-based. They rely on the agent output containing recognizable prefixes, which works when the CLI provider (Claude) uses those exact markers, but would misclassify output from providers using different conventions. There is no structured schema from the provider that carries segment boundaries.

**Gap**: No explicit thinking-block detection from the provider's `<thinking>` tags. The parser has no way to distinguish extended thinking content from regular reasoning text unless it happens to match the prefix heuristics.

---

## 3. Is there syntax highlighting for code blocks in output?

**No**. Code blocks are identified (fenced or indented) and given a single uniform style:

```rust
SegmentKind::Code => Style::default().fg(theme.success).bg(Theme::BG_SECONDARY),
```

All code is rendered in green-on-dark-background regardless of language. There is no tree-sitter, syntect, or any other syntax highlighting engine integrated. The ```` ```rust ```` language annotation in fences is consumed only for fence detection, not for language-specific coloring.

**Finding**: ANSI escape sequences embedded in the raw output *are* preserved and translated to ratatui styles via `ansi.rs`. This means if the provider or shell command emits colored output (e.g., `cargo` with ANSI colors), those colors survive into the TUI. But this is passthrough, not intelligent highlighting.

---

## 4. Is markdown in output rendered (bold, lists, headers)?

**Partially**.

- **Headers** (`# `, `## `, `### `): Detected by `is_heading()` and rendered in accent color with bold modifier. They also get a `HEADING` group header label.
- **Bold / italic / strikethrough**: Not parsed or rendered. Raw `**bold**` or `_italic_` text appears as literal characters.
- **Lists** (bullet or numbered): Not detected as a separate segment kind. They fall through to `Thinking` and render in dim foreground.
- **Links**: Not parsed. Appear as raw `[text](url)`.
- **Inline code**: Not detected. Backtick-delimited inline code appears as plain text.
- **Tables**: Not detected. Appear as plain text.

**Finding**: The markdown rendering is limited to header detection and code-fence detection. Everything else is treated as thinking text. This is far from what users expect when reading agent output that is typically rich markdown.

---

## 5. How does streaming look? Character-by-character or chunk-by-chunk?

**Chunk-by-chunk**, via the `AgentStream` mechanism.

The `push_agent_chunk()` method appends whole string chunks to a `VecDeque<String>` (max 200 entries). The live stream panel shows the tail of this deque:

```rust
let visible_lines = inner.height as usize;
let start = chunks.len().saturating_sub(visible_lines);
chunks[start..].join("\n")
```

The chunks are rendered as a raw `Paragraph` with no semantic styling -- just plain white text in the "Live Stream" panel at the bottom of the output area.

**Finding**: Streaming is chunk-granularity (whatever the WebSocket delivers per message). There is no character-by-character typewriter effect. New chunks simply appear in the deque and the panel auto-scrolls to show the latest entries. The main output panel above uses poll-based updates from `DashboardData` or `DashboardSnapshot`, which means it can lag behind the live stream by one refresh cycle.

**Gap**: The live stream panel gets **no semantic styling at all** -- it is a raw text dump. The styled segment parsing only applies to the main output panel above it, creating a visual discontinuity where the same content looks different in the two panels.

---

## 6. Is there a typing/cursor animation?

**No**. There is no blinking cursor, typing animation, or any visual indicator that content is actively being generated.

The only dynamic indicators are:
- The `[TAIL]` / `[PINNED line N]` label in the output panel title, colored green (tail) or warning (pinned).
- The "Live Stream" panel title shows `connected` (accent), `done` (green), `connecting...` (warning), or `no stream` (muted).
- Agent status chips in the roster change between `active`, `idle`, `done`, `failed`.

**Finding**: When an agent is generating output, the user sees lines appearing in the stream panel but there is no cursor or animation to indicate in-progress generation. The output just grows silently.

---

## 7. How are tool calls visualized in the output?

Tool calls are detected via **prefix matching** in `is_tool_use()`:

```rust
fn is_tool_use(trimmed: &str) -> bool {
    [">> ", "> ", "$ ", "Running ", "Reading ", "Writing ", "Editing ", "Created "]
    .iter()
    .any(|prefix| trimmed.starts_with(prefix))
}
```

When detected, lines get:
- A `TOOL USE` header label in info color (teal) with bold+dim styling
- Content rendered in `theme.info` (teal) foreground

**Finding**: Tool calls are not rendered as structured cards or expandable blocks. They appear as teal-colored text inline with the rest of the output. There is no distinction between different tool types (read file vs. write file vs. bash command). Tool call arguments are not formatted or highlighted separately from the tool name.

**Gap**: The detection is entirely heuristic. If a provider emits tool calls without these exact prefixes (e.g., structured JSON tool calls from the API), they would not be recognized. There is no integration with the actual tool call/result schema from `roko-agent`.

---

## 8. How are tool results shown?

Tool results are **not distinguished from tool calls or general output**. The raw text of a tool result (stdout/stderr from a command, file contents from a read, etc.) flows into the same line buffer and gets classified by the same heuristics.

Specific patterns:
- Compiler errors → classified as `Error` (red, bold) if they contain `error[` or `ERROR`
- Test passes → classified as `Success` (green, bold) if they contain `PASS` or a checkmark
- Everything else → `Thinking` (dim) or `Code` (if indented/fenced)

**Finding**: There is no visual boundary between "tool was called" and "tool returned this result." A tool call line like `$ cargo test` and its output lines appear in sequence with no separator or grouping beyond whatever the segment classifier happens to detect. Long tool results (e.g., 500 lines of test output) are truncated by the 50-line output cap before the TUI sees them.

---

## 9. Is there a way to search/filter within agent output?

**No**. There is no search or filter capability for agent output.

The TUI has search/filter for:
- **Logs tab**: `LogSearch` mode (triggered by `/` on the Logs tab) with regex pattern, next/prev match navigation (`n`/`N`), and toggle between highlight and filter modes.
- **Plans tab**: `PlanFilter` mode (triggered by `/` on the Plans tab) for filtering the plan tree.

But the Agents tab has **no equivalent**. The `FocusZone::AgentOutput` focus zone supports scroll (up/down/page) and pin/unpin (`[TAIL]` vs `[PINNED]`) but no text search.

**Finding**: Given that agent output is the primary thing users watch, the absence of search is a significant gap. Users cannot search for specific error messages, function names, or patterns in the output stream.

---

## 10. How does multi-agent output switching work?

Agent selection is controlled by `view_state.selected` (set via the agent roster on the left panel). The role tabs at the top of the right panel provide a sub-tab selector across 7 fixed roles:

```rust
pub(crate) const ROLE_TABS: &[(&str, &str)] = &[
    ("implementer", "1:impl"),
    ("strategist", "2:strat"),
    ("architect", "3:arch"),
    ("auditor", "4:audit"),
    ("critic", "5:crit"),
    ("conductor", "6:cond"),
    ("researcher", "7:res"),
];
```

When the user selects a different agent in the roster (arrow keys) or switches role tabs (number keys 1-7), `collect_agent_output_lines()` fetches the output for the newly selected agent. The output panel immediately replaces its content.

The `agent_output_cache` (`RefCell<HashMap<String, CachedRender>>`) caches parsed/styled output per agent ID. When switching back to a previously viewed agent, if the raw byte length has not changed, the cached styled lines are reused without re-parsing.

**Finding**: Switching is instantaneous (cached) but the previous agent's output disappears entirely. There is no split-pane or picture-in-picture mode to watch two agents simultaneously. The cache is keyed by agent ID and pruned when agents leave the roster.

**Gap**: No side-by-side comparison of outputs from different agents working on related tasks.

---

## 11. What happens when output is very long (1000+ lines)?

The output **never reaches 1000+ lines in the TUI**. Hard caps enforce truncation at multiple levels:

| Layer | Cap | Effect |
|---|---|---|
| `MAX_AGENT_OUTPUT_LINES` | 50 | `AgentRow::output_lines` keeps only the last 50 lines from episodes/snapshots |
| `bounded_output_lines()` | 50 | `task_output_tails` capped to last 50 from `VecDeque` source |
| `MAX_AGENT_STREAM_CHUNKS` | 200 | Live `AgentStream` keeps only the last 200 chunks |
| `Paragraph::scroll()` | `u16::MAX` | Scroll offset capped to `u16::MAX` (65535 lines theoretical max) |

The main output panel supports scroll via `agent_scroll`:
- `None` means auto-tail (always shows latest output, indicator: `[TAIL]` in green)
- `Some(offset)` means pinned scroll (indicator: `[PINNED line N]` in warning color)

Users can scroll up to pin, and new output will not auto-scroll. But with only 50 lines of history, there is very little to scroll through.

**Finding**: The 50-line cap is extremely aggressive. A single `cargo test` run can easily produce hundreds of lines, and the user only sees the last 50. The preprocessing step (`insert_prompt_breaks` + `split_on_sentences`) can actually expand lines (splitting long lines into multiple shorter ones), which means the effective content cap is even less than 50 original output lines.

**Gap**: No virtual scrolling, no "load more" mechanism, no way to access full output history from the TUI. Users must go to episodes.jsonl or the filesystem to see complete output.

---

## 12. Proposals

### 12a. Collapsible thinking blocks

**Current state**: Thinking lines render in dim text with a `THINKING` header. They are always fully expanded and can dominate the visible output area.

**Proposal**: Introduce collapsible segment groups. Each `SegmentGroup` could carry an `expanded: bool` state. Thinking blocks would default to collapsed (showing the header + first line as a summary) with a keybind to toggle expansion. This would let users focus on tool calls, code, and results without scrolling past pages of reasoning.

**Implementation notes**:
- Add `collapsed_groups: HashSet<usize>` to `TuiState` keyed by group index within the current agent's output.
- In `render_groups()`, emit only the header line for collapsed groups, with a `[+N lines]` indicator.
- Keybind: `Enter` or `Space` on a focused group to toggle.
- Requires a focus-within-output model (currently the output is one monolithic `Paragraph`).

### 12b. Syntax-highlighted code blocks

**Current state**: All code is rendered in uniform green-on-dark.

**Proposal**: Integrate `syntect` for code block syntax highlighting. The language hint from ```` ```rust ```` fence markers is already consumed by the parser. Pass it to syntect to produce styled spans.

**Implementation notes**:
- Add `syntect` as a dependency to `roko-cli`.
- In `render_content_line()` for `SegmentKind::Code` groups, use the `SyntaxSet` and `ThemeSet` from syntect to produce colored spans.
- Cache the syntax set (it is expensive to initialize) in `TuiState` or a static.
- Fall back to the current uniform green when no language is specified or syntect fails.
- Concern: syntect adds ~2-3MB to binary size and ~10-20ms per highlight pass. The caching in `CachedRender` already avoids re-parsing unchanged output, so this should be acceptable.

### 12c. Tool call cards

**Current state**: Tool calls are teal text inline with output. No structure, no arguments display, no result boundary.

**Proposal**: Render tool calls as bordered mini-cards with:
- Tool name in bold header
- Arguments displayed as key-value pairs (or truncated path for file operations)
- Collapsible result section below the card
- Status indicator (spinner while pending, checkmark on success, X on failure)
- Duration if available

**Implementation notes**:
- Requires structured tool call data in the output model, not just text heuristics. The `roko-agent` dispatcher knows the actual tool call schema -- this data should propagate to the TUI via `DashboardSnapshot` or episodes.
- Alternative: Parse the structured JSON tool call format if the provider emits it, rather than relying on prefix matching.
- Cards could be rendered as nested `Block` widgets within the output paragraph, though ratatui's `Paragraph` does not natively support inline blocks. A custom widget or line-level block rendering would be needed.

### 12d. Streaming cursor

**Current state**: No visual indicator of active generation beyond the `connected` label.

**Proposal**: Add a blinking block cursor (`_`) at the end of the last line in the live stream panel when the stream is connected and not completed. Use the existing TUI tick rate (250ms) to toggle visibility.

**Implementation notes**:
- Add `cursor_visible: bool` to `AgentStream`, toggled every other tick.
- In `render_live_stream_panel()`, append a styled cursor span to the last chunk when `stream.connected && !stream.completed`.
- Style: inverse video or bright accent color, toggled between visible and invisible each tick.

### 12e. Additional proposals

**Output search (/)**: Mirror the Logs tab search implementation for the agent output panel. `/` enters search mode, `n`/`N` navigate matches, regex support.

**Increased output buffer**: Raise `MAX_AGENT_OUTPUT_LINES` from 50 to at least 500. The memory cost is trivial (500 lines * ~100 bytes = ~50KB per agent). The current 50-line cap makes the output panel nearly useless for reviewing anything beyond the most recent activity.

**Structured output model**: Replace `Vec<String>` output storage with a typed enum model that carries segment kind, tool call metadata, timestamps, and provider turn boundaries as data rather than recovering them via text heuristics at render time.

**Split/diff view**: Allow comparing output from two agents side by side, especially useful when an auditor and implementer are working on the same task.

---

## Summary of key gaps

| # | Gap | Severity | Effort |
|---|---|---|---|
| 1 | 50-line output cap discards most output before TUI sees it | **High** | Low (change constant) |
| 2 | No search/filter in agent output | **High** | Medium (mirror LogSearch) |
| 3 | No syntax highlighting for code blocks | Medium | Medium (syntect integration) |
| 4 | No markdown rendering (bold, lists, links, inline code) | Medium | Medium |
| 5 | Tool calls not rendered as structured cards | Medium | High (needs data model changes) |
| 6 | No streaming cursor or generation indicator | Medium | Low |
| 7 | Live stream panel has no semantic styling | Medium | Low (reuse segment.rs) |
| 8 | No collapsible thinking blocks | Medium | Medium (needs focus-within-output) |
| 9 | No side-by-side multi-agent comparison | Low | High |
| 10 | Segment classification is heuristic-only, no structured provider data | Low | High (pipeline change) |

## Files examined

- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/views/agents_view.rs` -- Full agents view rendering (roster, role tabs, output panel, live stream, topology)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/state.rs` -- `AgentRow`, `AgentStream`, `CachedRender`, `TuiState` agent output fields, `render_agent_output_lines()`, `push_agent_chunk()`, `collect_agent_output_lines()`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/segment.rs` -- `SegmentKind`, `SegmentGroup`, `parse_segment_groups()`, `render_groups()`, `render_cached_output()`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/ansi.rs` -- ANSI SGR parsing (`parse_ansi_line()`) for preserving terminal colors
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/input.rs` -- `FocusZone::AgentOutput`, `LogSearch`/`PlanFilter` modes (agent output has neither)
