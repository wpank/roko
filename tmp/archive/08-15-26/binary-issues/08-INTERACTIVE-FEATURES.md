# 08 — Interactive Features & Power-User UX

**Status**: spec
**Scope**: `crates/roko-cli/src/chat_inline.rs`, `crates/roko-cli/src/tui/`

## Overview

Power-user features that reward expertise and make repeated use faster.
These are the features that make users say "I can't go back to the old way."

---

## Feature 8A: Command Palette (Ctrl+K)

A fuzzy-searchable command palette (like VS Code's Ctrl+Shift+P) accessible
from both chat and dashboard modes.

```
┌─ Command Palette ─────────────────────────────────┐
│ > sw                                               │
│                                                    │
│   Switch model          Change the active LLM      │
│ > Switch provider       Change auth/provider       │
│   Show cost summary     Display session costs      │
│   Start new session     Clear context, fresh start │
│                                                    │
└────────────────────────────────────────────────────┘
```

**Commands available:**
- All slash commands (mapped from SLASH_COMMANDS)
- Model switching (`/model <name>`)
- Provider info
- Theme switching (if implemented)
- Copy last response
- Export conversation
- Open dashboard
- Clear context

**Implementation**:
- Reuse `fuzzy_match` from 05-COMPLETION
- Modal overlay in chat_inline.rs (~80 lines)
- Command registry: `Vec<(name, description, action)>`
- Esc to close, Enter to execute, arrow keys to navigate

---

## Feature 8B: Conversation Export

Export the current conversation to a file in various formats.

```
/export markdown   → saves to .roko/exports/chat-2026-04-27-1430.md
/export json       → saves structured JSON with metadata
/export clipboard  → copies to system clipboard (if available)
```

Markdown format:
```markdown
# Roko Chat — 2026-04-27 14:30

**Model**: claude-sonnet-4-6 | **Turns**: 5 | **Cost**: $0.0312

---

## User
explain the borrow checker

## Roko
The borrow checker is Rust's...

---

## User
show me an example
...
```

**Implementation**: Track conversation as `Vec<(Role, String, Instant)>` in ChatSession.
Export command serializes to file. ~60 lines.

---

## Feature 8C: Inline Image/Diagram Display

When the agent outputs a mermaid diagram or ASCII art, render it inline
with proper formatting:

```
│ Here's the architecture:
│
│ ┌──────────┐     ┌──────────┐     ┌──────────┐
│ │  Client  │────▸│  Router  │────▸│  Agent   │
│ └──────────┘     └──────────┘     └──────────┘
│                       │
│                       ▾
│                  ┌──────────┐
│                  │   Gate   │
│                  └──────────┘
```

- Detect ASCII art blocks (lines with box-drawing chars)
- Preserve alignment (don't word-wrap these blocks)
- Optional: detect mermaid fences and render as ASCII

**Implementation**: Extend markdown renderer to detect and preserve ASCII art blocks.
~20 lines of detection logic.

---

## Feature 8D: Smart Retry on Error

When a dispatch fails, offer inline retry options instead of just showing the error:

```
│ error    connection refused: is `roko serve` running?
│
│   [r] Retry   [s] Switch provider   [q] Cancel
```

- Single keypress to retry (r), switch provider (s), or cancel (q)
- Retry reuses the same prompt
- Switch provider cycles through available AuthMethods
- Works for both HTTP and direct dispatch failures

**Implementation**: New `Phase::Error { prompt, error }` variant. ~40 lines.

---

## Feature 8E: Input History Persistence

Persist input history across sessions to `~/.roko/chat_history`:

- Save last 500 entries
- One entry per line (newlines escaped)
- Load on startup, append on submit
- Deduplicate consecutive identical entries

**Implementation**: ~30 lines. Read file in `InputState::new()`, append in `submit()`.

---

## Feature 8F: Configurable Keybindings

Allow users to customize keybindings via `roko.toml`:

```toml
[chat.keys]
submit = "Enter"
newline = "Shift+Enter"
interrupt = "Ctrl+C"
history_up = "Ctrl+P"     # emacs style
history_down = "Ctrl+N"
clear = "Ctrl+L"
palette = "Ctrl+K"
```

**Implementation**: Parse keybindings from config, map to actions. ~50 lines
for the config parsing, ~20 for the key matcher.

---

## Feature 8G: Agent Persona Indicators

When chatting with different agents (via `roko chat --agent <name>`),
show visual differentiation:

```
◆ architect                          (DREAM color)
│ Based on the system requirements...
└

◆ implementer                        (ROSE color)
│ Here's the code change...
└
```

- Agent name styled with role-specific color (from `Theme::role_accent`)
- Different prompt symbol per role type
- Subtitle showing agent's domain/specialty

**Implementation**: ~15 lines. Use existing `Theme::role_accent()` in
`push_agent_response`.

---

## Feature 8H: Notification Bell on Long Operations

When a response takes > 10 seconds, ring the terminal bell when it completes:

```rust
// After response received, if elapsed > 10s:
print!("\x07"); // BEL character
```

- Configurable threshold via `chat.notification_threshold_s` in roko.toml
- Can be disabled with `chat.notifications = false`
- Only triggers when terminal is in background (if detectable)

**Implementation**: 3 lines in the response handler.

---

## Priority Order

1. **8E** History persistence — most impactful for daily use
2. **8A** Command palette — power-user unlock
3. **8B** Conversation export — practical utility
4. **8H** Notification bell — trivial to add, useful for long ops
5. **8D** Smart retry — error recovery UX
6. **8G** Agent persona — visual polish for multi-agent use
7. **8C** Inline diagrams — niche but impressive
8. **8F** Configurable keybindings — flexibility for power users
