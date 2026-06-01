# W3-B: Panic Hook for Terminal Restore

**Priority**: P1 — prevents demo crashes
**Effort**: 15 minutes
**Files to modify**: 1 file
**Dependencies**: None

## Problem

If the process panics while in raw mode, the terminal stays in raw mode (no echo, no line editing). The `InlineTerminal::Drop` impl exists but may not run during a panic if the panic handler aborts or the stack doesn't unwind properly.

## Root Cause

`crates/roko-cli/src/inline/terminal.rs` line 52 calls `enable_raw_mode()`. The `Drop` impl (lines 176-180) calls `disable_raw_mode()`, which works for normal drops but NOT for panics that abort or unwind past the terminal's scope.

## Fix

Set a panic hook BEFORE entering raw mode that ensures terminal restoration.

### File: `crates/roko-cli/src/chat_inline.rs` (or wherever the chat session is started)

Find where `InlineTerminal::new()` is called (the entry point to chat mode). Before that call:

```rust
// Set panic hook to restore terminal before panic output
let default_hook = std::panic::take_hook();
std::panic::set_hook(Box::new(move |info| {
    // Restore terminal first — errors here are ok to ignore
    let _ = crossterm::terminal::disable_raw_mode();
    let _ = crossterm::execute!(
        std::io::stdout(),
        crossterm::cursor::Show
    );
    // Then run the default panic handler (prints backtrace, etc.)
    default_hook(info);
}));

// NOW safe to enter raw mode
let mut term = InlineTerminal::new()?;
```

After the chat session ends (when InlineTerminal is dropped), restore the default panic hook:

```rust
// After chat session ends, restore default panic hook
let _ = std::panic::take_hook(); // Remove our custom hook
// The default hook is automatically restored
```

Actually, simpler: just leave the custom hook in place. It's harmless — if raw mode isn't active, `disable_raw_mode()` is a no-op.

### Alternative: Do it in InlineTerminal::new()

Better encapsulation — put the panic hook in the terminal itself:

```rust
// In crates/roko-cli/src/inline/terminal.rs

pub fn new() -> io::Result<Self> {
    // ... existing code ...

    // Set panic hook before enabling raw mode
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(std::io::stdout(), crossterm::cursor::Show);
        default_hook(info);
    }));

    enable_raw_mode()?;
    // ... rest of constructor ...
}
```

## Agent Prompt

```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W3-B-panic-hook.md and implement all changes described in it. Add panic hook that calls disable_raw_mode() in InlineTerminal::new() in crates/roko-cli/src/inline/terminal.rs, before enable_raw_mode(). Do NOT run cargo build/test/clippy/fmt — compilation is deferred. Mark the checklist items as done.
```

## Commit

This batch is committed with all Wave 3 batches together. Do not commit individually.

## Checklist

- [x] Add panic hook that calls `disable_raw_mode()` before entering raw mode
- [x] Hook runs default panic handler after terminal restoration
- [x] Verify: panic during chat leaves terminal usable (deferred)
- [x] Verify: normal chat exit still works (deferred)
