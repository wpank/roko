# 06 — Chat UX Overhaul: Premium Inline REPL

**Status**: spec
**Scope**: `crates/roko-cli/src/chat_inline.rs`, `crates/roko-cli/src/inline/`

## Overview

Transform `roko chat` from a functional REPL into a premium, dopamine-inducing
terminal experience. These are the high-impact UX improvements that make the
difference between "works" and "delightful."

---

## Feature 6A: Animated Thinking Indicator

**Current**: Static spinner `⠋ Thinking... (3.4s)`
**Target**: Multi-phase indicator with context-aware labels and elapsed animation.

```
Phase 1 (0-2s):   ⠋ Connecting...          (0.8s)
Phase 2 (2-8s):   ⠹ Thinking...            (4.2s)
Phase 3 (8-15s):  ⠼ Still thinking...      (12.1s)
Phase 4 (15s+):   ⠧ Deep in thought...     (23.4s)
```

- Elapsed time counter pulses (bright → dim → bright) every 2s
- Spinner speed varies: fast (100ms) while connecting, slower (200ms) while thinking
- Phase labels configurable via theme

**Implementation**: ~20 lines in `handle_thinking_phase()` — just time-based label switching.

---

## Feature 6B: Markdown Syntax Highlighting in Responses

**Current**: Agent responses rendered as plain styled text with bar prefix.
**Target**: Code blocks rendered with language-aware syntax coloring.

````
│ Here's the fix:
│
│ ```rust
│   fn main() {                    ← BONE color
│       println!("hello");         ← strings in SAGE
│       let x = 42;               ← numbers in ROSE
│   }
│ ```
│
│ This handles the edge case by...
````

- Use `syntect` (already in dependency tree via roko-index) for tokenization
- Support: rust, python, javascript, typescript, json, toml, bash, go
- Fall back to plain monospace for unknown languages
- Inline code (backtick) rendered with dim background

**Implementation**: Extend `inline/markdown.rs` — it already parses markdown blocks,
just needs syntect integration for the code fence content.

---

## Feature 6C: Response Streaming with Word-Level Rendering

**Current**: Tokens appear character-by-character in the viewport.
**Target**: Words appear smoothly, with a subtle cursor/caret at the insertion point.

- Buffer tokens until whitespace, then flush the whole word
- Show a blinking block cursor at the end of streaming text
- Smooth auto-scroll (jump by line, not by character)

**Implementation**: ~15 lines in `StreamingState::append()` to buffer partial words.

---

## Feature 6D: Multi-line Input with Shift+Enter

**Current**: Single-line input only. Enter always submits.
**Target**: Shift+Enter inserts a newline. Visual indicator shows line count.

```
❯ Write a function that:           [3 lines]
│ 1. Parses JSON
│ 2. Validates schema
│ 3. Returns errors█
```

- Input area expands vertically (up to 6 lines, then scrolls internally)
- Line count badge in the status bar
- Alt+Enter also inserts newline (for terminals that eat Shift+Enter)
- Ctrl+Enter always submits (alternative to Enter on single line)

**Implementation**: Change `InputState.buffer` to track newlines, update `render_input`
to show multiple lines, adjust layout constraints dynamically.

---

## Feature 6E: Conversation Context Display

**Current**: No indication of conversation state between turns.
**Target**: Show turn count, context window usage, and conversation continuity.

Status bar addition:
```
$0.0310  ·  4821 in / 1203 out  ·  haiku  ·  turn 3  ·  12% ctx
```

- Turn counter increments on each exchange
- Context % estimates how much of the model's window is used
- Warning color when context > 80%

**Implementation**: Add `turn_count` and `context_estimate` to `ChatSession`,
update `render_status_bar`.

---

## Feature 6F: Message Timestamps

**Current**: No timestamps on messages.
**Target**: Subtle timestamps on each user/agent exchange.

```
❯ explain the borrow checker                              2:34 PM
◆ roko
│ The borrow checker is Rust's...
└                                                         2:34 PM (1.2s)
```

- User messages: wall clock time (right-aligned)
- Agent responses: wall clock + latency in parentheses
- Color: TEXT_GHOST (very subtle, doesn't compete with content)

**Implementation**: ~10 lines in `push_agent_response` and the Enter handler.

---

## Feature 6G: Scrollback Search (Ctrl+R)

**Current**: No way to search previous messages.
**Target**: Ctrl+R opens reverse search through scrollback.

```
(reverse-i-search)`borrow`: explain the borrow checker
```

- Standard readline-style reverse incremental search
- Matches against both user messages and agent responses in history
- Esc to cancel, Enter to accept (populates input buffer)
- Highlight matched substring in search results

**Implementation**: New `SearchState` struct, additional Phase variant or overlay mode.
~80 lines for the search logic, ~30 for rendering.

---

## Feature 6H: Copy-Friendly Code Blocks

**Current**: Code blocks in scrollback have bar prefixes (`│`), hard to copy.
**Target**: When the terminal supports it, code blocks are rendered without
bar prefixes in scrollback, or a "copy" hint is shown.

Alternative approach: detect code fence blocks in scrollback and render them with a
subtle box border instead of the bar prefix:

```
│ Here's the fix:
│
│ ┌─ rust ─────────────────────────┐
│ │ fn main() {                    │
│ │     println!("hello");         │
│ │ }                              │
│ └────────────────────────────────┘
```

**Implementation**: Extend `render_markdown_with_bar` to use box-drawing chars
around code fences.

---

## Priority Order

1. **6B** Markdown syntax highlighting — highest visual impact
2. **6D** Multi-line input — most requested UX gap
3. **6A** Animated thinking — easy win, immediately noticeable
4. **6F** Timestamps — trivial to add, adds professionalism
5. **6E** Context display — useful for long conversations
6. **6C** Word-level streaming — subtle but polished
7. **6H** Copy-friendly code — developer QoL
8. **6G** Scrollback search — powerful but complex
