# S-term-4: Terminal lifecycle hardening — spawn fail closes WS + cleanup ZDOTDIR

## Task
Two related fixes:
(a) On PTY spawn failure, emit `CommandEvent::SpawnFailed { reason }` and close the IO WebSocket cleanly.
(b) Delete the temporary `ZDOTDIR` directory on `Cancelled` and `SpawnFailed`, not just on normal `Exited`.

## Runner Context
Runner audit-2026-05-01, group S. Depends on S-term-1. Wave 2.

## Source plan
`tmp/subsystem-audits/implementation-plans/26-terminal-demo-truth.md` § Phase 5 A & B.

## Read first

```bash
rg 'spawn_failed|SpawnFailed|ZDOTDIR' crates/roko-serve/src/terminal*.rs -n
rg 'fn spawn|fn create_session' crates/roko-serve/src/terminal*.rs -n
```

## Exact changes

### A. Spawn failure closes WS

In the session-create / spawn flow:

```rust
match try_spawn_pty(...).await {
    Ok(child) => { /* normal flow */ }
    Err(e) => {
        // Emit typed event to any subscribers
        let _ = self.events_tx.send(CommandEvent::SpawnFailed {
            session_id: self.session_id.clone(),
            reason: format!("{e}"),
        });
        // Close the IO WebSocket if it's already connected
        if let Some(io_tx) = &self.io_ws_tx {
            let _ = io_tx.send(WsCloseFrame::error(format!("spawn failed: {e}"))).await;
        }
        return Err(e);
    }
}
```

### B. Cleanup on cancel / spawn-failed

Wherever the temp `ZDOTDIR` (or equivalent shell-config temp dir) is created:

```rust
let zdotdir = tempfile::tempdir()?;
// ... assign to env when spawning ...

// Wrap session lifetime so the tempdir is dropped on every termination path:
struct PtySession {
    _zdotdir: tempfile::TempDir,    // dropped on session drop
    // ...
}
```

If the cleanup is currently a manual `std::fs::remove_dir_all` only on `Exited`, add the same call to `Cancelled` and `SpawnFailed` paths. Or — preferably — own the `TempDir` so RAII handles it.

### C. Tests

```rust
#[tokio::test]
async fn spawn_failure_emits_typed_event_and_closes() {
    let session = TerminalSession::test_with_failing_spawn().await;
    let mut events = session.subscribe_events();
    let _err = session.start_command("nonexistent-command").await;
    let event = events.recv().await.unwrap();
    assert!(matches!(event, CommandEvent::SpawnFailed { .. }));
}

#[tokio::test]
async fn cancel_cleans_up_zdotdir() {
    let session = TerminalSession::test_with_zdotdir().await;
    let zdotdir = session.zdotdir_path().to_path_buf();
    session.start_command("sleep 60").await.unwrap();
    session.cancel().await;
    assert!(!zdotdir.exists(), "ZDOTDIR should be deleted on cancel");
}
```

## Write Scope
- `crates/roko-serve/src/terminal.rs` (or `terminal/mod.rs`)
- `crates/roko-serve/src/terminal/session.rs` (if split)
- `crates/roko-serve/src/command_events.rs` (only if `SpawnFailed` enum needs adjustment)

## Verify

```bash
rg 'CommandEvent::SpawnFailed|TempDir|zdotdir' crates/roko-serve/src/terminal*.rs
# Expect: at least 3 hits

rg 'spawn_failure_emits_typed_event_and_closes|cancel_cleans_up_zdotdir' crates/roko-serve/
# Expect: 2 hits
```

## Do NOT

- Do NOT bundle with S-term-5.
- Do NOT leave the IO WebSocket half-open after spawn failure.
- Do NOT delete other shell-config files outside the tempdir.
- Do NOT silently swallow the spawn error before emitting the event.
