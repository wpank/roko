# Chat Inline Decomposition

**Priority**: P2
**Size**: L (3-5 days)

---

## Problem

`crates/roko-cli/src/chat_inline.rs` is 5,698 lines — the second-largest source file
in the codebase after `event_loop.rs`. It contains two near-identical ~700-line chat
event loops (`run_unified_inline` and `run_chat_inline`), a 246-line `extract_clean_text`
function that parses 13 response formats, 58 slash command handlers, input handling,
viewport rendering, session management, HTTP conversion, and a phase machine.

Every feature addition (tool rendering, cost display, streaming tokens) requires
duplicate implementation across both loops. Built UI primitives like `ToolCallBlock`
(241 lines) exist but are not wired — the rendering just builds its own instead.

This is the same god-file pattern that `event_loop.rs` suffers from, but in a different
module and affecting different concerns (interactive chat vs plan execution).

### What already exists

| Component | Location | Status |
|---|---|---|
| `StreamingState` | `chat_inline.rs` | EXISTS (append, scroll, usage) |
| `ToolCallBlock` primitive | `inline/primitives/tool_call.rs` | EXISTS (not wired) |
| `CostWaterfall` primitive | `inline/primitives/` | EXISTS (not wired) |
| `push_tool_outputs()` | `chat_inline.rs` | EXISTS (custom, ignores primitive) |
| `extract_clean_text()` | `chat.rs` + `chat_inline.rs` | EXISTS (duplicated, 246 lines) |

### What is missing

1. **Shared `ResponseRenderer` trait** — Each rendering mode (inline, one-shot, REPL,
   TUI, `roko run`) renders responses differently with no shared interface. The trait
   should unify tool output, text, cost, and gate verdict rendering.

2. **Single chat loop** — `run_unified_inline` and `run_chat_inline` are ~1,400 lines
   of near-identical event loop logic. They should be consolidated into one
   parameterized loop.

3. **`extract_clean_text` consolidation** — The 246-line function handling 13 response
   formats should be replaced by per-backend typed deserialization, or at minimum
   deduplicated to a single location.

4. **Wiring of built primitives** — `ToolCallBlock` and other rendering primitives
   exist but are not integrated into the chat rendering path.

---

## Proposed extraction targets

### Target A: `chat/response_renderer.rs`
Define a `ResponseRenderer` trait with methods for `render_tool_output`,
`render_text`, `render_cost`, `render_gate`. Implement for inline mode first,
then adapt other modes.

Estimated: ~400 lines. Risk: low.

### Target B: consolidate chat loops
Merge `run_unified_inline` and `run_chat_inline` into a single parameterized
function. The differences between the two loops should be captured in a config
struct, not duplicated code.

Estimated: ~700 lines removed. Risk: medium (two large functions merging).

### Target C: `chat/text_parser.rs`
Move `extract_clean_text` to a single location. Replace format-specific parsing
with typed deserialization where possible.

Estimated: ~250 lines. Risk: low.

### Target D: wire built primitives
Connect `ToolCallBlock`, `CostWaterfall` to the rendering path via the new
`ResponseRenderer` trait.

Estimated: ~200 lines. Risk: low.

---

## Acceptance criteria

1. `chat_inline.rs` is under 3,000 lines.
2. Only one chat event loop function exists (not two duplicates).
3. `extract_clean_text` exists in exactly one file.
4. `ToolCallBlock` is used by the rendering path (not zero references).
5. `cargo test -p roko-cli` passes with zero failures after each extraction.
6. `cargo clippy -p roko-cli -- -D warnings` is clean.

---

## References

- `crates/roko-cli/src/chat_inline.rs` — the file being decomposed (5,698 lines)
- `crates/roko-cli/src/chat.rs` — contains duplicate `extract_clean_text`
- `crates/roko-cli/src/inline/primitives/` — built but unwired rendering primitives
- `tmp/workflow/10-cli-chat-tui-audit.md` — original July 2026 audit identifying this problem
