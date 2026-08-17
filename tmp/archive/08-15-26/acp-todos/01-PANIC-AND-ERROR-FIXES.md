# ACP: Panic & Error Handling Fixes

> **Source**: `crates/roko-acp/src/bridge_events.rs` (8,430 LOC)
> **References**: `tmp/acp-features/00-ACP-FEATURES.md`, `tmp/acp-runner/`
> **Created**: 2026-08-15

---

## P0 -- Server Crash (fix immediately)

### 1. `.expect()` on `provider_health_registry` during prompt dispatch
- **File**: `crates/roko-acp/src/bridge_events.rs:1675`
- **Code**:
  ```rust
  let provider_health = Arc::clone(
      session
          .provider_health_registry
          .as_ref()
          .expect("ACP provider health initialized before prompt"),
  );
  ```
- **Context**: Production code inside `handle_session_prompt`. If the session's `provider_health_registry` is `None` (e.g. initialization race, config error), the server panics and kills the ACP process.
- **Fix**: Return `BridgeEventsError::Pipeline(anyhow!("provider health registry not initialized"))` instead of panicking. The caller already maps `BridgeEventsError` to an RPC error.
- **Effort**: 5min

### 2. `.expect()` on `provider_rate_limiter` during prompt dispatch
- **File**: `crates/roko-acp/src/bridge_events.rs:1681`
- **Code**:
  ```rust
  let provider_rate_limiter = Arc::clone(
      session
          .provider_rate_limiter
          .as_ref()
          .expect("ACP provider rate limiter initialized before prompt"),
  );
  ```
- **Context**: Same function as #1. If the rate limiter is `None`, the server panics.
- **Fix**: Same pattern -- return a `BridgeEventsError` instead.
- **Effort**: 5min

### 3. `.expect("stdout was piped")` in slash-command dispatch
- **File**: `crates/roko-acp/src/bridge_events.rs:4923`
- **Code**:
  ```rust
  let stdout = child.stdout.take().expect("stdout was piped");
  let stderr = child.stderr.take().expect("stderr was piped");
  ```
- **Context**: Production code in the slash-command dispatch path (not under `#[cfg(test)]`). If `Command::stdout(Stdio::piped())` was not set (or someone refactors the Command construction), this panics and crashes the ACP server mid-prompt.
- **Fix**: Match on the `Option`, return `Err(anyhow!("slash command process stdout not piped"))` on `None`.
- **Effort**: 5min

### 4. `.expect("stdout was piped")` in shell-command dispatch
- **File**: `crates/roko-acp/src/bridge_events.rs:5241`
- **Code**:
  ```rust
  let mut stdout_lines =
      tokio::io::BufReader::new(child.stdout.take().expect("stdout was piped")).lines();
  let mut stderr_lines =
      tokio::io::BufReader::new(child.stderr.take().expect("stderr was piped")).lines();
  ```
- **Context**: Production code in a second shell-command dispatch path. Same issue as #3.
- **Fix**: Return `Err(anyhow!(...))` instead of panicking.
- **Effort**: 5min

### 5. `.expect("valid char boundary")` in `extract_at_mentions`
- **File**: `crates/roko-acp/src/bridge_events.rs:5753`
- **Code**:
  ```rust
  let ch = text[end..].chars().next().expect("valid char boundary");
  ```
- **Context**: Production code parsing user-provided prompt text for `@` mentions. While `text[end..]` should never produce an empty slice within the `while` loop guard (`end < text.len()`), a logic bug or concurrent modification would panic the server on user input.
- **Fix**: Use `unwrap_or(' ')` or a `let Some(ch) = ... else { break; }` pattern. The while-loop invariant makes this theoretically safe, but defensive code is appropriate for user-input parsing.
- **Effort**: 3min

### 6. `unreachable!()` in `unique_tool_name`
- **File**: `crates/roko-acp/src/bridge_events.rs:3694`
- **Code**:
  ```rust
  unreachable!("suffix search should always find a unique tool name")
  ```
- **Context**: Production code. The for-loop iterates 0..1000 and appends `_N` suffixes. If all 1000 names collide (e.g. MCP server registers 1000+ identically-named tools), this panics. Unlikely but not impossible with a malicious/buggy MCP server.
- **Fix**: Return an error or log + return a fallback name.
- **Effort**: 5min

### 7. `unreachable!()` in `map_event_to_update`
- **File**: `crates/roko-acp/src/bridge_events.rs:5369`
- **Code**:
  ```rust
  CognitiveEvent::Complete { .. }
  | CognitiveEvent::Failure { .. }
  | CognitiveEvent::MaxTokens
  | CognitiveEvent::PermissionRequest { .. } => {
      unreachable!("terminal/async cognitive events are handled before update mapping")
  }
  ```
- **Context**: Production code. The assumption is that `stream_events_to_editor` handles terminal events before calling `map_event_to_update`. If a new `CognitiveEvent` variant is added and the stream handler is not updated, this panics at runtime.
- **Fix**: Replace with a `warn!` and return a no-op `SessionUpdate` (e.g. an empty AgentMessageChunk), or return `Option<SessionUpdate>` and skip sending.
- **Effort**: 10min

---

## P1 -- Silent Failures

### 8. Efficiency event serialization failure silently dropped
- **File**: `crates/roko-acp/src/bridge_events.rs:631`
- **Code**:
  ```rust
  Err(err) => {
      tracing::warn!(error = %err, "failed to serialize efficiency event");
      return;
  }
  ```
- **Context**: `emit_acp_efficiency_event` runs inside `task::spawn_blocking`. If serialization fails, the efficiency event is silently lost. No metric is recorded, no cost is tracked, the cost-budget accounting drifts.
- **Fix**: At minimum, increment a counter metric. Consider propagating the error to the session so `accumulated_cost_usd` is still updated (or use a fallback serialization).
- **Severity**: P1 -- silent cost-tracking drift
- **Effort**: 15min

### 9. Efficiency event file write failure silently dropped
- **File**: `crates/roko-acp/src/bridge_events.rs:643`
- **Code**:
  ```rust
  if let Err(err) = writeln!(f, "{line}") {
      tracing::warn!(error = %err, "failed to write efficiency event");
  }
  ```
- **Context**: Same function. If the JSONL append fails (disk full, permission denied), the event is lost.
- **Fix**: Same as #8.
- **Severity**: P1
- **Effort**: 5min

### 10. Efficiency event file open failure silently dropped
- **File**: `crates/roko-acp/src/bridge_events.rs:647`
- **Code**:
  ```rust
  Err(err) => {
      tracing::warn!(error = %err, "failed to open efficiency.jsonl");
  }
  ```
- **Context**: Same function. Cannot open the file at all.
- **Fix**: Same as #8.
- **Severity**: P1
- **Effort**: 5min

### 11. Dream consolidation failure silently swallowed
- **File**: `crates/roko-acp/src/bridge_events.rs:559`
- **Code**:
  ```rust
  if let Err(err) = runner.consolidate_now() {
      warn!(?err, "background dream consolidation failed");
  }
  ```
- **Context**: Background task. If dream consolidation fails repeatedly, no one notices. No retry, no metric, no health signal.
- **Fix**: Acceptable for background/best-effort work, but should increment a failure counter or emit a health event. Low urgency.
- **Severity**: P1
- **Effort**: 10min

### 12. ACP MCP tool-loop stream error silently dropped
- **File**: `crates/roko-acp/src/bridge_events.rs:3418`
- **Code**:
  ```rust
  StreamChunk::Error(error) => {
      warn!(error = %error, "ACP MCP tool-loop stream error");
  }
  ```
- **Context**: Inside `forward_tool_loop_stream_chunks`. Stream errors from the tool loop are logged but not forwarded to the editor as a `CognitiveEvent::Failure`. The user sees no indication of the error.
- **Fix**: Send a `CognitiveEvent::Failure { message }` to surface the error to the editor.
- **Severity**: P1
- **Effort**: 10min

### 13. Backend tool-loop error falls through to plain streaming silently
- **File**: `crates/roko-acp/src/bridge_events.rs:2659` and `3393`
- **Code** (line 2659):
  ```rust
  ToolLoopStopReason::BackendError(error) => {
      warn!(
          session_id,
          error = %error,
          "Anthropic builtin tool loop backend error, falling through to plain streaming"
      );
      return Ok(Some(false));
  }
  ```
- **Code** (line 3393):
  ```rust
  ToolLoopStopReason::BackendError(error) => {
      warn!(
          session_id,
          error = %error,
          "builtin tool loop backend error, falling through to plain streaming"
      );
      return Ok(false);
  }
  ```
- **Context**: When the tool loop's backend (provider) errors out, the code silently falls through to a simpler streaming path. The user/editor has no visibility that their session degraded from tool-loop mode to plain streaming.
- **Fix**: Emit a `CognitiveEvent::Failure` or a status event so the editor can display a degradation notice.
- **Severity**: P1
- **Effort**: 10min

### 14. Model stream attempt failure silently logged
- **File**: `crates/roko-acp/src/bridge_events.rs:2879`
- **Code**:
  ```rust
  ModelStreamEvent::AttemptFailed { model, error } => {
      warn!(
          session_id,
          model,
          error = %error,
          "model stream attempt failed"
      );
      Ok(ModelStreamForward::Continue)
  }
  ```
- **Context**: When a model stream attempt fails (e.g. provider returns 500), it is logged but the editor is not notified. The stream continues to retry, which is correct, but the user gets no feedback about retries.
- **Fix**: Send a status event or `TokenChunk` with a retry notice.
- **Severity**: P1 (acceptable for now if retries succeed; becomes visible when all attempts fail)
- **Effort**: 10min

### 15. Session MCP config write failure silently returns None
- **File**: `crates/roko-acp/src/bridge_events.rs:3492`
- **Code**:
  ```rust
  Err(error) => {
      warn!(path = %path.display(), error = %error, "failed to write session MCP config");
      None
  }
  ```
- **Context**: If the MCP config file cannot be written, downstream dispatch may use stale or no MCP tools. The caller gets `None` but does not distinguish "no MCP needed" from "MCP write failed."
- **Fix**: Return `Result<Option<PathBuf>>` so the caller can decide whether to abort or continue without MCP.
- **Severity**: P1
- **Effort**: 10min

### 16. Cascade router save failure after observation silently dropped
- **File**: `crates/roko-acp/src/bridge_events.rs:1148-1154`
- **Code**:
  ```rust
  if let Err(error) = router.save(&router_path) {
      warn!(
          path = %router_path.display(),
          error = %error,
          "failed to persist cascade router after ACP observation"
      );
  }
  ```
- **Context**: In `record_cascade_observation`. If the router state cannot be saved, the learning observation is computed but not persisted. The next session will use stale routing data.
- **Fix**: Acceptable for background learning, but should increment a metric. Low urgency.
- **Severity**: P1
- **Effort**: 5min

### 17. Experiment outcome recording failure silently dropped (2 locations)
- **File**: `crates/roko-acp/src/bridge_events.rs:1752-1759` and `2306-2314`
- **Code** (line 1754):
  ```rust
  warn!(
      experiment_id = %assignment.experiment_id,
      variant_id = %assignment.variant_id,
      error = %error,
      "failed to persist rejected ACP experiment outcome"
  );
  ```
- **Code** (line 2309):
  ```rust
  warn!(
      experiment_id = %assignment.experiment_id,
      variant_id = %assignment.variant_id,
      error = %error,
      "failed to persist ACP experiment outcome"
  );
  ```
- **Context**: Experiment outcomes (success/failure) not saved. A/B experiment stats drift from reality.
- **Fix**: Acceptable for best-effort learning, but the experiment comparison data becomes unreliable.
- **Severity**: P1
- **Effort**: 5min

---

## P2 -- Data Loss / Sync Issues

### 18. Usage update send failure silently dropped
- **File**: `crates/roko-acp/src/bridge_events.rs:2172-2178`
- **Code**:
  ```rust
  if let Err(error) = send_session_update(transport, &session.session_id, update).await {
      warn!(
          session_id = %session.session_id,
          error = %error,
          "failed to send ACP usage update"
      );
  }
  ```
- **Context**: The usage/cost update notification to the editor fails. The editor won't display correct token/cost stats for this turn. Session cost budget accounting still works (separate path), so no budget drift, but the editor's view is stale.
- **Fix**: Acceptable since the transport error likely means the editor disconnected. Could escalate to a `Failure` event so the session knows the editor is gone.
- **Severity**: P2
- **Effort**: 10min

### 19. Budget status update send failure silently dropped
- **File**: `crates/roko-acp/src/bridge_events.rs:2297-2302`
- **Code**:
  ```rust
  warn!(
      session_id = %session.session_id,
      error = %error,
      "failed to send ACP budget status update"
  );
  ```
- **Context**: Same transport-error pattern as #18. The editor misses the budget-remaining notification.
- **Fix**: Same as #18.
- **Severity**: P2
- **Effort**: 5min

### 20. Session title update send failure silently dropped
- **File**: `crates/roko-acp/src/bridge_events.rs:2359-2363`
- **Code**:
  ```rust
  warn!(
      session_id = %session.session_id,
      error = %error,
      "failed to send ACP session title update"
  );
  ```
- **Context**: The auto-generated session title does not reach the editor. Session still works, title is just "Untitled."
- **Fix**: Acceptable. Cosmetic loss.
- **Severity**: P2
- **Effort**: 3min

### 21. Workspace trust persistence failure silently dropped
- **File**: `crates/roko-acp/src/bridge_events.rs:1304-1309`
- **Code**:
  ```rust
  Err(error) => warn!(
      session_id = %session.session_id,
      action = ?action,
      error = %error,
      "always-allow retained for this session but workspace persistence failed"
  ),
  ```
- **Context**: User grants "always allow" for a permission action, but the workspace trust file cannot be written. The grant works for the current session but won't persist across restarts, requiring the user to re-approve next time.
- **Fix**: Surface this to the user as a non-fatal warning in the editor UI. The current warn-only approach means the user has no idea their "always allow" didn't persist.
- **Severity**: P2
- **Effort**: 10min

### 22. Episode log read failure silently dropped in provenance builder
- **File**: `crates/roko-acp/src/bridge_events.rs:4112-4117`
- **Code**:
  ```rust
  Err(err) => {
      warn!(
          workdir = %workdir.display(),
          error = %err,
          "episode log read failed"
      );
  }
  ```
- **Context**: In `build_provenance`. If episodes cannot be read, provenance chains will be missing episode-sourced evidence. The system prompt will have less context, potentially degrading agent quality.
- **Fix**: Acceptable for degraded-but-functional operation. Could return an indicator that provenance is incomplete.
- **Severity**: P2
- **Effort**: 5min

### 23. Dream routing advice load failure silently dropped
- **File**: `crates/roko-acp/src/bridge_events.rs:4053-4058` and `4061-4066`
- **Code**:
  ```rust
  Ok(Err(err)) => {
      warn!(workdir = %workdir.display(), error = %err, "dream routing advice load failed");
      Vec::new()
  }
  Err(err) => {
      warn!(workdir = %workdir.display(), error = %err, "dream routing advice task failed");
      Vec::new()
  }
  ```
- **Context**: Dream routing patterns not available for provenance. Reduces system prompt quality.
- **Fix**: Same as #22.
- **Severity**: P2
- **Effort**: 5min

---

## P3 -- Cosmetic / Minor

### 24. Permission request serialization failure sends null payload
- **File**: `crates/roko-acp/src/bridge_events.rs:1240-1248`
- **Code**:
  ```rust
  .unwrap_or_else(|error| {
      warn!(
          session_id = %session.session_id,
          action = ?action,
          error = %error,
          "failed to serialize permission request; sending null payload"
      );
      serde_json::Value::Null
  });
  ```
- **Context**: If `RequestPermissionParams` cannot be serialized (extremely unlikely since all fields are simple strings/enums), a null payload is sent to the editor. The editor will likely show a broken permission dialog.
- **Fix**: Low urgency. Could construct a minimal fallback JSON manually.
- **Severity**: P3
- **Effort**: 5min

### 25. `.expect("PermissionReplyChannel mutex poisoned")` (3 locations)
- **File**: `crates/roko-acp/src/bridge_events.rs:242`, `254`, `264`
- **Code**:
  ```rust
  .lock().expect("PermissionReplyChannel mutex poisoned")
  ```
- **Context**: Production code in `PermissionReplyChannel::reply()`, `is_consumed()`, and `receiver_is_closed()`. A poisoned mutex means another thread panicked while holding the lock. The lock guard is extremely short (just `.take()` or `.is_none()`), so poisoning is essentially impossible unless there is already a panic elsewhere. This is the standard Rust idiom for `Mutex::lock()` on non-contended, panic-free locks.
- **Fix**: Could replace with `.lock().unwrap_or_else(|e| e.into_inner())` to recover from poisoning by taking the inner value. Low urgency because poisoning implies a prior panic that is the real issue.
- **Severity**: P3
- **Effort**: 5min

### 26. MCP server discovery warnings (6 locations)
- **File**: `crates/roko-acp/src/bridge_events.rs:3521`, `3539`, `3558`, `3572`, `3593`, `3607`
- **Code**: Various `warn!()` calls for MCP server transport unsupported, spawn failed, initialize failed, initialize timeout, tools/list failed, tools/list timeout.
- **Context**: These are all correctly structured: the error is logged, a `McpServerStatus::failed(...)` is pushed to the status list, and the loop continues to the next server. The statuses are forwarded to the editor via `CognitiveEvent::McpStatus`. This is the correct pattern.
- **Fix**: None needed. These are properly handled degradation paths.
- **Severity**: P3 (no fix needed)
- **Effort**: 0min

### 27. Permission flow warn-and-Reject patterns (7 locations)
- **File**: `crates/roko-acp/src/bridge_events.rs:1263`, `1285`, `1316`, `1338`, `1346`, `1369`, `1376`, `1386`, `1426`, `1437`
- **Context**: All of these are in `request_permission`. When the editor sends a malformed response, disconnects, times out, or cancels, the code logs a warning and returns `PermissionDecision::Reject`. This is the correct fail-closed behavior for a security-sensitive permission gate.
- **Fix**: None needed. These are intentional fail-closed defaults.
- **Severity**: P3 (no fix needed)
- **Effort**: 0min

### 28. Event stream closed without completion event
- **File**: `crates/roko-acp/src/bridge_events.rs:1493-1497`
- **Code**:
  ```rust
  let Some(event) = maybe_event else {
      warn!(
          session_id,
          "ACP event stream closed without an explicit completion event"
      );
      // ... returns EndTurn or Cancelled
  };
  ```
- **Context**: In `stream_events_to_editor`. If the cognitive event channel closes without a `Complete` event, the code gracefully infers the stop reason and returns a valid `StreamResult`. This is correct defensive handling.
- **Fix**: None needed. The warn is informational.
- **Severity**: P3 (no fix needed)
- **Effort**: 0min

### 29. Permission requester disappeared before receiving decision
- **File**: `crates/roko-acp/src/bridge_events.rs:1551-1555`
- **Code**:
  ```rust
  if !reply.reply(decision) {
      warn!(
          session_id,
          "permission requester disappeared before receiving the decision"
      );
  }
  ```
- **Context**: The tool loop (requester) timed out or was cancelled before the editor responded. The decision cannot be delivered. The tool loop already handles this (fail-closed on oneshot `RecvError`). This warn is purely informational.
- **Fix**: None needed.
- **Severity**: P3 (no fix needed)
- **Effort**: 0min

### 30. Tool call warn-and-deny patterns (4 locations)
- **File**: `crates/roko-acp/src/bridge_events.rs:3800`, `3815`, `3846`, `3860`
- **Context**: `AcpBuiltinToolHandler::execute()` logs warnings when tools are denied/rejected and returns `ToolResult::Err(ToolError::PermissionDenied(...))`. This is correct: the error is propagated through the tool result, not swallowed.
- **Fix**: None needed.
- **Severity**: P3 (no fix needed)
- **Effort**: 0min

### 31. Various ACP session info/routing warn patterns
- **File**: `crates/roko-acp/src/bridge_events.rs:851`, `1043`, `1058`, `1084`
- **Context**: Informational warnings about experiment model resolution, rate-pressure degradation, cascade unconfigured model fallback. All return correct fallback values.
- **Fix**: None needed. These are operational observability warnings.
- **Severity**: P3 (no fix needed)
- **Effort**: 0min

### 32. Slash command process termination failures
- **File**: `crates/roko-acp/src/bridge_events.rs:4933` and `4943`
- **Code**:
  ```rust
  warn!(session_id, %error, "failed to terminate slash command process tree");
  ```
- **Context**: After cancellation or stream completion, the slash command child process tree could not be killed. This means orphan processes. The warn is appropriate but could be escalated to `error!` since orphan processes consume resources.
- **Fix**: Escalate to `error!()`. Consider a follow-up kill attempt.
- **Severity**: P3
- **Effort**: 3min

### 33. Slash command stdout/stderr read errors
- **File**: `crates/roko-acp/src/bridge_events.rs:5118`, `5133`, `5268`, `5283`
- **Code**:
  ```rust
  warn!(session_id, error = %e, "error reading slash command stdout");
  ```
- **Context**: IO errors while streaming slash command output. The affected stream (stdout or stderr) is marked as done. This is correct -- the command may have been killed or the pipe broken.
- **Fix**: None needed.
- **Severity**: P3 (no fix needed)
- **Effort**: 0min

### 34. File context resolution warnings (production, non-critical)
- **File**: `crates/roko-acp/src/bridge_events.rs:5587`, `5605`, `5630`, `5638`
- **Context**: Warnings when file resources or `@`-mentions cannot be resolved (file outside workdir, read error, canonicalize error). The resolved context simply omits the failed resource. The prompt proceeds with whatever context was successfully resolved.
- **Fix**: None needed. Graceful degradation is correct here.
- **Severity**: P3 (no fix needed)
- **Effort**: 0min

### 35. Safety violation warn-only for `Warn` severity
- **File**: `crates/roko-acp/src/bridge_events.rs:1944` and `2220`
- **Code** (line 1944):
  ```rust
  ViolationSeverity::Warn => {
      warn!(
          session_id = %session.session_id,
          violation = ?violation.violation_type,
          message = %violation.message,
          "ACP pre-dispatch safety warning"
      );
      None
  }
  ```
- **Context**: Pre-dispatch and post-dispatch safety checks with `Warn` severity are logged but not surfaced to the editor or the user. Block-severity violations are handled correctly. The `Warn` path means "proceed but note it."
- **Fix**: Consider forwarding `Warn`-severity violations as a status event or metadata on the response. Low urgency.
- **Severity**: P3
- **Effort**: 15min

---

## Summary

| Severity | Count | Fix-needed | Already-correct |
|----------|-------|------------|-----------------|
| P0       | 7     | 7          | 0               |
| P1       | 10    | 10         | 0               |
| P2       | 6     | 3-4        | 2-3             |
| P3       | 12    | 2-3        | 9-10            |
| **Total**| **35**| **22-24**  | **11-13**       |

**Estimated total effort**: ~2.5 hours for all P0+P1 fixes, ~1 hour for P2, ~30min for actionable P3 items.

**Priority**: Fix P0 items #1-#7 first (all `.expect()`/`unreachable!()` in production code). These are the only paths that crash the ACP server process.

### Test-only code (excluded from findings)

All `.expect()` and `panic!()` calls below line 5824 (`#[cfg(test)] mod tests`) are in test code and are correct -- tests should panic on unexpected failures. These include:
- 126 `.expect()` calls in test functions (e.g. `tempfile::tempdir().expect("create tmpdir")`)
- 9 `panic!()` calls in test match arms (e.g. `other => panic!("expected permission request, got {other:?}")`)

No `.unwrap()` calls exist anywhere in this file (production or test).
