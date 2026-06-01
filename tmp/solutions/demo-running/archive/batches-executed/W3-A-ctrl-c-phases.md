# W3-A: Add Ctrl+C Handler to All Chat Phases

**Priority**: P1 — prevents demo crashes
**Effort**: 30 minutes
**Files to modify**: 1 file
**Dependencies**: None

## Problem

Running `roko` (chat mode) and getting an error leaves the terminal completely unresponsive. Ctrl+C does nothing. The Phase::Error handler has a `_ => {}` wildcard that silently eats Ctrl+C.

## Root Cause

`crates/roko-cli/src/chat_inline.rs` has TWO Phase::Error handlers (the code has two event loop paths). Both have `_ => {}` wildcards at the end of their match arms.

## Existing RAII Guard

Good news: `InlineTerminal` at `crates/roko-cli/src/inline/terminal.rs` already has a `Drop` impl (lines 176-180) that calls `restore()` which calls `disable_raw_mode()`. So the terminal WILL be restored if the struct is dropped. The problem is that the event loop never exits because Ctrl+C is eaten.

## Exact Code to Change

### File: `crates/roko-cli/src/chat_inline.rs`

### Change 1: Phase::Error handler #1 (lines 1323-1338)

**Before**:
```rust
Phase::Error { ref prompt, .. } => {
    match key.code {
        KeyCode::Char('r') => {
            let retry_prompt = prompt.clone();
            session.phase = Phase::Thinking;
            session.thinking_started = Some(Instant::now());
            dispatch_prompt(&mut session, &retry_prompt);
        }
        KeyCode::Char('q') | KeyCode::Esc => {
            term.push_blank()?;
            session.phase = Phase::Input;
        }
        _ => {}  // ← LINE 1337: EATS Ctrl+C
    }
}
```

**After**:
```rust
Phase::Error { ref prompt, .. } => {
    match key.code {
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            session.phase = Phase::Done;
            break;
        }
        KeyCode::Char('r') => {
            let retry_prompt = prompt.clone();
            session.phase = Phase::Thinking;
            session.thinking_started = Some(Instant::now());
            dispatch_prompt(&mut session, &retry_prompt);
        }
        KeyCode::Char('q') | KeyCode::Esc => {
            term.push_blank()?;
            session.phase = Phase::Input;
        }
        _ => {}
    }
}
```

### Change 2: Phase::Error handler #2 (lines 1692-1704)

**Same pattern** — add the Ctrl+C handler before the existing matches:
```rust
Phase::Error { ref prompt, .. } => match key.code {
    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
        session.phase = Phase::Done;
        break;
    }
    KeyCode::Char('r') => {
        // ... existing retry code
    }
    KeyCode::Char('q') | KeyCode::Esc => {
        term.push_blank()?;
        session.phase = Phase::Input;
    }
    _ => {}
},
```

### Change 3: Audit ALL other phases for missing Ctrl+C

Search for other `_ => {}` wildcards in the event loop. For each one, check if Ctrl+C is handled. The key phases to check:

- **Phase::Thinking** — should Ctrl+C cancel the pending request and return to Input
- **Phase::Streaming** — should Ctrl+C stop streaming and return to Input
- **Phase::Input** — likely already has Ctrl+C or Ctrl+D handling

For Thinking/Streaming, add:
```rust
KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
    session.phase = Phase::Done;
    break;
}
```

### Import check

Make sure `KeyModifiers` is imported:
```rust
use crossterm::event::{KeyCode, KeyModifiers, /* ... */};
```

## Agent Prompt

```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W3-A-ctrl-c-phases.md and implement all changes described in it. Two Phase::Error handlers in crates/roko-cli/src/chat_inline.rs need Ctrl+C handling added before the _ => {} wildcard. Also audit Phase::Thinking and Phase::Streaming. Do NOT run cargo build/test/clippy/fmt — compilation is deferred. Mark the checklist items as done.
```

## Commit

This batch is committed with all Wave 3 batches together. Do not commit individually.

## Verification (deferred to Phase 2)

After compilation: Ctrl+C from error state exits cleanly; terminal is restored.

## Checklist

- [x] Add Ctrl+C to Phase::Error handler #1 (line ~1337)
- [x] Add Ctrl+C to Phase::Error handler #2 (line ~1703)
- [x] Audit Phase::Thinking for Ctrl+C handling
- [x] Audit Phase::Streaming for Ctrl+C handling
- [x] Ensure `KeyModifiers` is imported
- [ ] Verify: Ctrl+C exits from error state
- [ ] Verify: terminal is restored after exit
- [ ] Pre-commit checks pass
