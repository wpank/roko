# 22 — Chat inline decomposition

**Priority**: P2 — Code health: the god-file makes every chat feature require duplicate implementation
**Size**: L (3-5 days)
**Crates**: `crates/roko-cli/` (`src/chat_inline.rs`, `src/chat.rs`, `src/inline/`)
**Depends on**: None

---

## Background

Roko is a Rust agent toolkit. The `roko chat` command provides an interactive chat REPL with inline rendering using the ratatui TUI library. This REPL renders streaming responses at the bottom of the terminal, pushes completed turns into terminal scrollback, and handles slash commands, multi-line input, cost tracking, and session management.

All of this logic lives in a single file: `crates/roko-cli/src/chat_inline.rs`. At 5,698 lines it is the second-largest source file in the codebase. The file contains two near-identical event loops (`run_chat_inline` and `run_unified_inline`), a 246-line text parser (`extract_clean_text`) that is duplicated from another file, 58 slash command handlers, and an inline tool output renderer that ignores purpose-built rendering primitives that already exist in `crates/roko-cli/src/inline/primitives/`.

The immediate consequence is that every new chat feature — tool rendering, cost display, streaming tokens — must be implemented twice across the two loops. The underlying rendering primitives exist and are documented but are not wired into the chat path.

The goal of this item is to decompose `chat_inline.rs` into focused modules that can be maintained and extended independently. This is a pure refactoring: no user-visible behavior changes, no new features.

## Current State

1. **`chat_inline.rs`** is at `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/chat_inline.rs` and is 5,698 lines as of 2026-08-19.

2. **Two near-identical event loops exist in the same file:**
   - `run_chat_inline` starts at line 1183, ends at line 1531 (348 lines). It connects to a running `roko serve` HTTP sidecar and uses HTTP to dispatch messages.
   - `run_unified_inline` starts at line 1575, ends at line 1907 (332 lines). It builds an agent session locally from config and dispatches directly without a serve process.
   - Line 1676 contains the comment "Main event loop (identical structure to run_chat_inline)", confirming the duplication is known.
   - Both loops: check TTY, initialize terminal, push a welcome banner, enter a `loop { draw; poll_events; handle_key_event }` structure, and call the same set of rendering helpers.

3. **`extract_clean_text` is duplicated:**
   - The canonical implementation lives in `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/chat.rs` at line 603 (246 lines, handles plain text, JSON objects with `result`/`content` fields, JSON arrays of content blocks, and JSONL streaming protocol).
   - `chat_inline.rs` line 28 already imports it: `use crate::chat::{self, extract_clean_text};`. The duplication is not in `chat_inline.rs` itself, but in the fact that `chat.rs` alone defines this function — if it were moved to a shared text-parsing module, both callers would import from one place rather than through `chat.rs`.

4. **`ToolCallBlock` primitive exists but is not wired into the chat rendering path:**
   - `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/inline/primitives/tool_call.rs` defines `ToolCallBlock` at line 18 (241 lines total). It has `from_start`, `set_result`, `render_collapsed`, and `render_expanded` methods.
   - The `mod.rs` comment at `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/inline/primitives/mod.rs` line 6 says: "`roko chat`: StreamingBlock + ToolCallBlock + RunBlock + CostMeter" — but `ToolCallBlock` has zero references inside `chat_inline.rs`.
   - Instead, `chat_inline.rs` has its own custom `push_tool_outputs` function at line 4552 that builds rendering from scratch.

5. **`CostWaterfallData` exists but is not wired into chat rendering:**
   - `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/inline/primitives/cost_waterfall.rs` defines `CostWaterfallData` at line 37.
   - It is not referenced in `chat_inline.rs`.

6. **No `ResponseRenderer` trait exists:**
   - Searching `crates/roko-cli/src/` for `ResponseRenderer` returns zero results. Each rendering context (chat inline, run inline, plan run) renders tool output, text, cost, and gate verdicts independently.

7. **`runner/inline_output.rs` already uses `ToolCallBlock`** at line 135 and 141, showing the pattern is established for the plan-run path but not wired into chat.

## Implementation Plan

Perform the decomposition in four independent, sequentially ordered steps. Run `cargo test -p roko-cli` and `cargo clippy -p roko-cli -- -D warnings` after each step before proceeding.

### Step A: Define `ResponseRenderer` trait in a new file

Create `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/inline/response_renderer.rs`:

```rust
//! `ResponseRenderer` — shared interface for rendering agent responses inline.
//!
//! Implemented by: inline chat, unified chat, run, plan run.

use ratatui::text::Line;
use crate::dispatch_v2::ToolOutput;

/// Trait for rendering agent response components into terminal output.
pub trait ResponseRenderer {
    /// Render a completed tool call result.
    fn render_tool_output(&self, output: &ToolOutput) -> Vec<Line<'static>>;

    /// Render agent text content.
    fn render_text(&self, text: &str) -> Vec<Line<'static>>;

    /// Render a cost/usage summary line.
    fn render_cost(&self, model: &str, tokens: u64, cost_usd: f64, elapsed_s: f64) -> Vec<Line<'static>>;
}
```

Add `pub mod response_renderer;` to `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/inline/mod.rs`.

The trait does not need to be implemented yet — just defined. This step establishes the shared interface so subsequent steps can target it.

**Risk**: Low. Trait definition with no impl.

### Step B: Consolidate the two chat event loops

The two loops (`run_chat_inline` lines 1183-1531, `run_unified_inline` lines 1575-1907) differ only in how they set up the backend before entering the loop:

- `run_chat_inline` builds an HTTP client and calls `resolve_chat_backend` (line 1198).
- `run_unified_inline` calls `build_unified_inline_agent_session` (line 1586) and works with a local `ChatAgentSession`.

The loop body itself (draw → poll → handle key events) is structurally identical.

Refactoring approach:

1. Create an enum or struct `ChatBackendConfig` that holds either the HTTP backend URL or the local agent session:
   ```rust
   enum ChatBackendConfig {
       Http { url: String, client: reqwest::Client, is_sidecar: bool },
       Local { session: ChatAgentSession, system_message: Option<String>, cost_table: CostTable },
   }
   ```

2. Extract the shared loop body into a private function `run_chat_loop(backend: ChatBackendConfig, mut term: InlineTerminal, theme: Theme) -> Result<()>`.

3. Rewrite `run_chat_inline` to call `run_chat_loop(ChatBackendConfig::Http { ... }, ...)`.

4. Rewrite `run_unified_inline` to call `run_chat_loop(ChatBackendConfig::Local { ... }, ...)`.

The existing `ChatSession` struct (already present in the file) can continue to hold all per-loop state. The dispatch path in each arm of the loop can be guarded by matching on `backend`.

**Risk**: Medium. Two large functions merging. Test both `roko chat --agent <name>` (HTTP path) and `roko chat` (local path) after this step.

### Step C: Move `extract_clean_text` to `chat/text_parser.rs`

`extract_clean_text` in `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/chat.rs` (line 603) is the only location this function exists. It should live in a dedicated module so the location makes its scope clear and it doesn't get re-duplicated.

1. Create `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/chat/text_parser.rs` (or if `chat.rs` is a file rather than a module directory, just note it stays in `chat.rs` — confirm with `ls`).

   Actually, `chat.rs` is a single file. If moving to a submodule requires converting `chat.rs` to a module directory, that is optional. The minimal change is to add a doc comment to `extract_clean_text` marking it as the canonical location and ensuring no other copy is introduced.

2. Verify with `grep -rn "extract_clean_text" crates/roko-cli/src/` that there is exactly one definition and all callers use `crate::chat::extract_clean_text`.

**Risk**: Low. No behavioral change.

### Step D: Wire `ToolCallBlock` into the chat rendering path

Replace the custom `push_tool_outputs` function in `chat_inline.rs` (line 4552) with calls to `ToolCallBlock`.

Current `push_tool_outputs` signature (line 4552):
```rust
fn push_tool_outputs(
    term: &mut InlineTerminal,
    theme: &Theme,
    tool_outputs: &[ToolOutput],
) -> std::io::Result<()>
```

Replacement:
1. Import `ToolCallBlock` from `crate::inline::primitives::ToolCallBlock`.
2. In the function body, build a `ToolCallBlock` per `ToolOutput`, call `block.render_collapsed(&theme)` (or `render_expanded` based on the existing logic), and push the resulting lines to `term`.
3. Remove or keep the old `push_tool_outputs` as a thin wrapper around the new primitive — the function signature does not need to change.

Verify `ToolCallBlock` is now referenced in `chat_inline.rs` (currently has zero references).

**Risk**: Low. Tool rendering output may change visually — compare before/after in a real chat session.

## Acceptance Criteria

1. `chat_inline.rs` is under 3,000 lines after completing all four steps.
2. Only one chat event loop function exists (the refactored `run_chat_loop` or equivalent — not two independent near-duplicate loops).
3. `extract_clean_text` is defined in exactly one file, with all callers importing from that location.
4. `ToolCallBlock` has at least one reference inside `chat_inline.rs` (currently zero).
5. `cargo test -p roko-cli` passes with zero failures after each step.
6. `cargo clippy -p roko-cli -- -D warnings` is clean after each step.

## Verification Checklist

- [ ] Run `wc -l crates/roko-cli/src/chat_inline.rs` — confirm line count is under 3,000
- [ ] Run `grep -c "^pub async fn run_chat_inline\|^pub async fn run_unified_inline" crates/roko-cli/src/chat_inline.rs` — confirm at most 2 (ideally replaced by `run_chat_loop`)
- [ ] Run `grep -rn "fn extract_clean_text" crates/roko-cli/src/` — confirm exactly one definition
- [ ] Run `grep -n "ToolCallBlock" crates/roko-cli/src/chat_inline.rs` — confirm at least one match
- [ ] Run `cargo test -p roko-cli 2>&1 | tail -5` — confirm zero failures
- [ ] Run `cargo clippy -p roko-cli -- -D warnings 2>&1 | tail -10` — confirm no warnings
- [ ] Start `roko chat` (local path, `run_unified_inline`) and send a message; confirm response renders
- [ ] Start `roko serve` and then `roko chat --agent default`; confirm HTTP path still works

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/chat_inline.rs` | Consolidate two event loops; wire `ToolCallBlock`; reduce from 5,698 to under 3,000 lines |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/inline/mod.rs` | Add `pub mod response_renderer;` |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/inline/response_renderer.rs` | New file: define `ResponseRenderer` trait |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/chat.rs` | Add doc comment marking `extract_clean_text` as the canonical location (no logic change) |
