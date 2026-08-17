# ACP: Error Handling Fixes (Non-bridge_events Modules)

> **Source**: All roko-acp modules except bridge_events.rs
> **References**: `tmp/acp-features/00-ACP-FEATURES.md`, `tmp/acp-runner/`
> **Created**: 2026-08-15

## Summary

| Module | P0 | P1 | P2 | Total |
|---|---|---|---|---|
| runner.rs | 0 | 4 | 1 | 5 |
| session.rs | 0 | 3 | 0 | 3 |
| handler.rs | 0 | 2 | 0 | 2 |
| transport.rs | 0 | 2 | 0 | 2 |
| acp_adapter.rs | 0 | 1 | 0 | 1 |
| config.rs | 0 | 0 | 0 | 0 |
| config_watch.rs | 0 | 0 | 0 | 0 |
| pipeline.rs | 0 | 1 | 0 | 1 |
| workflow.rs | 0 | 0 | 0 | 0 |
| knowledge.rs | 0 | 0 | 0 | 0 |
| event_forward.rs | 0 | 0 | 0 | 0 |
| builtin_tools.rs | 0 | 0 | 0 | 0 |
| **Total** | **0** | **13** | **1** | **14** |

No P0 (crash) issues found in production code. All `.expect()`, `.unwrap()`, and `panic!()` calls
in these modules are inside `#[cfg(test)]` blocks.

## Clean Modules (No Production Issues)

The following modules have no error handling issues in production code:

- **config.rs** -- All `unwrap_or_default()` and `unwrap_or_else()` calls are safe
  fallbacks with sensible defaults. The `current_dir().unwrap_or_default()` in the
  `Default` impl is the only edge case, but `PathBuf::default()` (empty path) is
  acceptable for a `Default` impl that is rarely used in practice.

- **config_watch.rs** -- All errors are logged with `tracing::warn!` and the watcher
  degrades gracefully (returns `Self` with `_watcher: None`). This is correct
  best-effort behavior.

- **workflow.rs** -- Pure data module with no I/O, no `unwrap()`, no `warn!()`.

- **knowledge.rs** -- All errors from knowledge/playbook queries are caught with
  `warn!()` and converted to empty result sets. This is correct: knowledge is
  advisory and should never block dispatch.

- **event_forward.rs** -- Pure mapping module. No error paths except the
  `unwrap_or_else` on `serde_json::to_string` for MCP statuses (line 97), which
  falls back to a count string. This is correct behavior.

- **builtin_tools.rs** -- All tool handlers return `Result<String, String>` with
  proper error propagation. The `unwrap_or_else` calls on `canonicalize()` (lines
  282, 286, 303, 306) are correct fallbacks for symlink resolution.

## P0 -- Server Crash

None. All `expect()` / `unwrap()` / `panic!()` calls across these 12 modules are
inside `#[cfg(test)]` blocks.

## P1 -- Silent Failures

### P1-01: runner.rs:1839 -- Reviewer failure silently treated as approved

```rust
// runner.rs line 1839 (production, inside run_standard_review)
Err(e) => {
    warn!(error = %e, "reviewer failed, treating as approved");
    run.pipeline.step(PipelineEvent::ReviewApproved {
        summary: "Review skipped (agent error)".into(),
    })
}
```

**Severity**: P1 -- Silent failure. A code review agent crash is silently converted
to "approved", allowing unreviewed code to be committed.

**Fix**: Surface the error to the user via a `CognitiveEvent::TokenChunk` warning
badge, and record the skip in the workflow run metadata. Consider treating it as
`ReviewRevise` with a finding that says "Review could not be completed" so the
pipeline retries or the user is explicitly warned.

---

### P1-02: runner.rs:1909 -- Architect reviewer failure silently dropped

```rust
// runner.rs line 1909 (production, inside run_thorough_review)
Err(e) => {
    warn!(error = %e, "architect reviewer failed, continuing");
}
```

**Severity**: P1 -- Silent failure. The architect review leg of a "thorough" review
fails and the error is only logged. The overall review may still approve if the
auditor approves, even though one of two mandatory reviewers crashed.

**Fix**: Track which review legs failed and surface in the review summary. If one
reviewer fails, the review should be treated as "partially reviewed" and the user
should be notified.

---

### P1-03: runner.rs:1936 -- Auditor reviewer failure silently dropped

```rust
// runner.rs line 1936 (production, inside run_thorough_review)
Err(e) => {
    warn!(error = %e, "auditor reviewer failed, continuing");
}
```

**Severity**: P1 -- Silent failure. Same pattern as P1-02 but for the auditor leg.
Both reviewers can fail and the code is still committed as "approved" since
`all_approved` starts as `true` and is only set to `false` on non-approved output.

**Fix**: Initialize `all_approved = false` when a reviewer errors, or accumulate
a "reviewer_error" finding. At minimum, emit a visible warning to the user.

---

### P1-04: runner.rs:2143 -- Adaptive threshold save failure silently dropped

```rust
// runner.rs line 2143 (production)
fn save_thresholds(thresholds: &AdaptiveThresholds, path: &Path) {
    if let Err(e) = thresholds.save(path) {
        warn!(error = %e, "failed to save adaptive gate thresholds");
    }
}
```

**Severity**: P1 -- Silent failure. Gate threshold learning data is lost. Over many
runs this means thresholds regress to defaults and the adaptive system never improves.

**Fix**: Retry once. If still failing, emit a structured warning so the user knows
learning data is not being persisted.

---

### P1-05: session.rs:1317-1330 -- Session persistence failure silently dropped

```rust
// session.rs line 1317 (production, SessionManager::persist_session)
if let Err(e) = std::fs::create_dir_all(&sessions_dir) {
    tracing::warn!(error = %e, "failed to create sessions directory");
    return;
}
// ... line 1325:
if let Err(e) = std::fs::write(&path, json) {
    tracing::warn!(path = %path.display(), error = %e, "failed to persist session");
}
// ... line 1329:
Err(e) => {
    tracing::warn!(error = %e, "failed to serialize session for persistence");
}
```

**Severity**: P1 -- Silent failure. Session state (conversation history, config,
cost tracking) is lost on disk write failure. The user's next session resume will
load stale or missing data.

**Fix**: Return `Result` from `persist_session` and let the caller (handler.rs)
decide whether to warn the user or retry.

---

### P1-06: session.rs:519-521 -- Workspace trust deserialization failure silently returns empty set

```rust
// session.rs line 519 (production, AcpSession::load_workspace_trust)
std::fs::read_to_string(&path)
    .ok()
    .and_then(|data| serde_json::from_str(&data).ok())
    .unwrap_or_default()
```

**Severity**: P1 -- Silent failure. If `permissions.json` exists but is malformed,
the parse error is silently swallowed and all "always allow" decisions are lost.
The user has to re-approve every tool action.

**Fix**: Log a warning when the file exists but fails to parse, so the user knows
their trust file is corrupt.

---

### P1-07: session.rs:215/233 -- Config fallback warnings only logged, not surfaced to IDE

```rust
// session.rs lines 215 and 233 (production, SessionConfigState::from_roko_config_with_warnings)
tracing::warn!("{message}");
warnings.push(message);
```

**Severity**: P1 (minor) -- The warnings are pushed to a Vec that is returned and
eventually shown in the initialize response, so this is only a P1 if the IDE does
not display `configWarnings`. The `tracing::warn!()` goes to the log file but the
user may not see it. The pattern is correct in design but borderline: the fallback
to "first ready model" can pick a completely different model than the user intended.

**Fix**: No code change needed; the warnings Vec is already surfaced in the
initialize response. Consider making the fallback more explicit in the IDE.

---

### P1-08: handler.rs:209/447 -- Config options serialization failure silently returns empty array

```rust
// handler.rs line 209 (production)
let options_value = serde_json::to_value(&options)
    .unwrap_or_else(|_| serde_json::json!([]));
// handler.rs line 447 (production)
let options = serde_json::to_value(session.config_options())
    .unwrap_or_else(|_| serde_json::json!([]));
```

**Severity**: P1 -- Silent failure. If `ConfigOption` serialization fails (unlikely
but possible with custom Serialize impls), the IDE receives an empty config options
array and all dropdowns disappear. The error is completely swallowed.

**Fix**: Log a `warn!()` in the `unwrap_or_else` closure so the serialization
failure is at least recorded.

---

### P1-09: handler.rs:644 -- Logging initialization failure silently swallowed

```rust
// handler.rs line 644 (production)
let _ = tracing::subscriber::set_global_default(subscriber);
```

**Severity**: P1 (minor) -- If `set_global_default` fails (e.g. another subscriber
is already registered), all subsequent log calls go nowhere. The `let _ =` silently
discards the error.

**Fix**: This is intentional for the case where tests or other code sets a global
subscriber first. In production this is always the first call. Consider logging to
stderr as a last resort if it fails: `if let Err(e) = ... { eprintln!("..."); }`.

---

### P1-10: transport.rs:229 -- Poisoned mutex silently returns None

```rust
// transport.rs line 228 (production, handle_incoming_response)
Err(_) => {
    warn!("pending request registry is poisoned");
    None
}
```

**Severity**: P1 -- Silent failure. When the pending request registry mutex is
poisoned (a previous thread panicked while holding it), inbound responses are
silently dropped. The caller waiting on the response will hang until timeout.

**Fix**: This is a reasonable defensive pattern for a poisoned mutex. Consider
adding a counter or flag to indicate the transport is degraded, and reject new
outbound requests early.

---

### P1-11: transport.rs:237 -- Response delivery failure silently dropped

```rust
// transport.rs line 236 (production, handle_incoming_response)
if sender.send(response).is_err() {
    warn!(request_id, "response receiver dropped before delivery");
}
```

**Severity**: P1 (minor) -- The response is logged but lost. The receiver was
dropped (likely cancelled), so the caller no longer cares. This is correct behavior
for the cancellation case.

**Fix**: No change needed. The `warn!` is appropriate here.

---

### P1-12: acp_adapter.rs:164 -- Event forwarding failure silently dropped

```rust
// acp_adapter.rs line 164 (production, EventConsumer::consume)
let _ = self.sender.try_send(cognitive_event);
```

**Severity**: P1 -- Silent failure. If the channel is full or closed, cognitive
events (agent output, gate results, completion signals) are silently lost. The IDE
will show stale or incomplete progress.

**Fix**: Add a `debug!` or `warn!` log when `try_send` fails. Consider using
a bounded channel with backpressure or increasing the buffer. Also track dropped
event counts as a metric.

---

### P1-13: pipeline.rs:371-378 -- Invalid state transitions silently return Done

```rust
// pipeline.rs line 371 (production, PipelineState::step)
(phase, event) => {
    tracing::warn!(
        phase = ?phase,
        event = ?event,
        "unexpected pipeline event for current phase"
    );
    // Stay in current phase, no action.
    PipelineAction::Done
}
```

**Severity**: P1 -- Silent failure. An invalid state machine transition (a logic
bug) is logged but then the pipeline returns `Done`, which means the workflow
silently terminates without completing its actual work. The user sees a "completed"
result that is actually incomplete.

**Fix**: Return `PipelineAction::Halt { reason }` instead of `Done` so the pipeline
reports a halted state rather than falsely claiming success. Alternatively, track
it as a distinct `PipelinePhase::Error` variant.

## P2 -- Data Loss / Sync

### P2-01: runner.rs:657 + lines 1155/1163/1171/1299/1324/1422/1449/1495/1496/1511/1512 -- Event channel drops silently via `let _ =`

```rust
// runner.rs line 657 (production, WorkflowEventPublisher::publish)
fn publish(&self, event: CognitiveEvent) {
    let _ = self.sender.try_send(event);
}

// runner.rs lines 1155, 1163, 1171, 1299, 1324, 1422, 1449, 1495, 1496, 1511, 1512
let _ = event_sender.send(CognitiveEvent::TokenChunk(badge)).await;
let _ = event_sender.send(CognitiveEvent::ToolCallStart { ... }).await;
// ... etc (12 occurrences total)
```

**Severity**: P2 -- Data loss. The pipeline runner uses `let _ =` on every event
send, meaning if the channel is closed (session cancelled, transport error), ALL
subsequent events are silently lost. This includes:
- Token streaming chunks (user sees incomplete output)
- Gate results (user doesn't know if tests passed)
- Commit notifications (user doesn't know the commit hash)
- Completion events (IDE doesn't know the workflow finished)

The `try_send` variant (line 657) also drops events when the channel buffer is full,
which can happen during bursts of file-change notifications.

**Fix**: At minimum, check `send` results and set a flag to indicate the session
is disconnected. Stop spawning new pipeline phases when the event channel is dead.
For the `try_send` path, consider increasing the channel capacity or switching to
an unbounded channel for critical events (Complete, Failure).
