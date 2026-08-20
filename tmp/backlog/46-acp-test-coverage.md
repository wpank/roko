# 46 — ACP Test Coverage Gaps

**Priority**: P2 — Known coverage gaps in ACP error handling and cross-provider tool dispatch
**Size**: M (1-3 days)
**Crates**: `roko-acp` (`crates/roko-acp/`), `roko-agent` (`crates/roko-agent/`)
**Depends on**: None

---

## Background

The ACP (Agent Client Protocol) layer has 16 integration tests across two test files (`protocol_conformance.rs`: 11 tests, `telemetry_integration.rs`: 5 tests) that all pass. However, three specific coverage gaps remain unfixed: one error propagation path that lacks a test verifying it works, one silent failure mode when an MCP subprocess crashes, and one cross-provider tool dispatch scenario where a regression could go undetected.

**Gap 1: stdin-EOF clean shutdown path is implemented but has no unit test.**
`StdioTransport::read_message()` in `crates/roko-acp/src/transport.rs:117` returns `Ok(None)` when `read_line` reads 0 bytes (EOF). The handler loop in `run_acp_server_with_transport` at `crates/roko-acp/src/handler.rs:146` correctly handles `Ok(None)` by logging and returning `Ok(())`. The behavior is correct, but there is no test that exercises this path — no test sends EOF to the server and verifies clean exit. A regression in this path (e.g., someone adding an `Err` branch that forgets to handle the None case) would not be caught until integration testing.

**Gap 2: MCP subprocess crashes are silently swallowed.**
`McpClient<StdioTransport>` in `crates/roko-agent/src/mcp/client.rs` holds a `_child: Mutex<Child>` with `kill_on_drop(true)` (line 202, 230). When the spawned MCP server process crashes, the next call to the client will fail with a broken-pipe or EOF error. That error is recorded in `McpErrorAccumulator` (via `McpHandlerResolver`) with `is_transport_error = true`. However, `crates/roko-acp/src/bridge_events.rs` never queries the `McpErrorAccumulator`. The IDE receives no notification that an MCP server has crashed — the session just silently stops responding to tools from that server.

**Gap 3: Cross-provider tool dispatch matrix is incomplete.**
`crates/roko-agent/tests/tool_loop_integration.rs` has one test (`tool_loop_glm_e2e`) that exercises the `read_file` tool through an OpenAI-compatible (GLM) backend. `crates/roko-agent/tests/mock_provider.rs` provides `mock_openai_with_tool_calls()` that sequences one `read_file` call. No test exercises `write_file`, `edit_file`, `bash`, or `glob` through any provider. No test exercises any tool through an Anthropic-format backend (Anthropic JSON fixtures exist in `crates/roko-agent/tests/fixtures/common/` but only cover error responses, not tool call/result sequences). If a translator change breaks tool call deserialization for one provider or one tool, no test catches it.

## Current State

1. **`StdioTransport::read_message()` at `crates/roko-acp/src/transport.rs:117`**: returns `Ok(None)` when `bytes_read == 0` (line 124). The transport test at line 299 (`test_read_eof`) uses `empty()` as the reader and calls `read_message()`, asserting `Ok(None)` — so there IS an existing unit test for the transport's EOF behavior. However, there is no end-to-end test that runs the full `run_acp_server_with_transport` loop with an EOF input and verifies the loop exits cleanly with `Ok(())`.

2. **`McpHandlerResolver` at `crates/roko-agent/src/mcp/handler.rs:23`**: takes an optional `McpErrorAccumulator` via `with_error_accumulator`. When an MCP tool call fails with a transport error, `McpToolHandler::call` records it in the accumulator. The accumulator exposes `snapshot()` and `drain()` methods (`error_accumulator.rs:99`). `McpErrorRecord.is_transport_error` (line 30) is `true` for broken-pipe or EOF errors from a crashed subprocess.

3. **`bridge_events.rs` never queries `McpErrorAccumulator`**: The ACP session does not hold or query an `McpErrorAccumulator`. The `handle_session_prompt_inner` function builds a `McpHandlerResolver` (when MCP servers are configured) but discards any accumulator reference after the tool loop completes.

4. **`crates/roko-agent/tests/mock_provider.rs`**: `mock_openai_with_tool_calls()` at line 22 serves a `read_file` tool call on the first request and a plain text response on the second. The `OpenAiToolCallSequence` responder (line 54) is stateful but hardcoded to `read_file` only.

5. **`crates/roko-agent/tests/tool_loop_integration.rs`**: one test (`tool_loop_glm_e2e`) uses `spawn_chat_server` (a raw TCP server, not wiremock) and exercises only `read_file`. No Anthropic-format fixture exists for tool-call sequences.

6. **`crates/roko-agent/tests/fixture_loading.rs`**: loads JSON from `tests/fixtures/` and drives adapters. The `common/` directory contains only error response fixtures (401, 429, 500). No tool-call or tool-result response fixtures exist for Anthropic format.

7. **`mock_openai_compat()` at line 9**: returns a single static chat completion. Does not support tool calls.

## Implementation Plan

### Fix 1: Add end-to-end stdin-EOF test for the ACP handler loop

In `crates/roko-acp/tests/protocol_conformance.rs`, add a test that runs the full `run_acp_server_with_transport` loop with an `empty()` reader (immediately returns EOF on read):

```rust
#[tokio::test]
async fn stdin_eof_causes_clean_server_exit() {
    use tokio::io::empty;
    let config = AcpConfig::default_for_testing(tmp_workdir());
    let mut transport = StdioTransport::from_io(empty(), Vec::new());
    let result = run_acp_server_with_transport(config, &mut transport).await;
    assert!(result.is_ok(), "ACP server should exit cleanly on EOF, got: {result:?}");
}
```

This test runs the full handler loop (not just the transport layer) and verifies that `Ok(())` is returned when the input is immediately exhausted. The transport test at `transport.rs:299` already tests `read_message()` in isolation; this test covers the integration path.

Estimated: ~15 lines in `protocol_conformance.rs`.

### Fix 2: MCP crash detection and user-visible error surfacing

When `McpErrorAccumulator` collects a transport error (indicating subprocess death), the ACP session should include an error notification in the next `session/update` to the IDE.

**Step 2a**: Pass an `McpErrorAccumulator` instance into `handle_session_prompt_inner` when MCP servers are configured. Currently the accumulator is created inside the tool loop setup and discarded.

**Step 2b**: After the tool loop completes (or after each tool call turn), query the accumulator for transport errors:

```rust
if let Some(ref acc) = mcp_error_acc {
    let transport_errors: Vec<_> = acc.drain().into_iter()
        .filter(|r| r.is_transport_error)
        .collect();
    if !transport_errors.is_empty() {
        let error_text = transport_errors.iter()
            .map(|r| format!("MCP server '{}': {}", r.server_name, r.message))
            .collect::<Vec<_>>()
            .join("\n");
        send_session_update(transport, session_id, SessionUpdate::error_block(error_text)).await?;
    }
}
```

**Step 2c**: Add a unit test using a mock MCP process that exits immediately. Verify that the next ACP session update after the tool call failure contains an error block visible to the IDE. Use `tokio::process::Command` with a short-lived subprocess (`echo` or a script that exits 1) as the MCP server.

Estimated: ~80 lines across `bridge_events.rs` and a new test.

### Fix 3: Cross-provider tool dispatch matrix

Create `crates/roko-agent/tests/tool_matrix.rs` with tests that exercise each standard builtin tool through both provider backends.

**For the OpenAI-compat backend**, extend `mock_openai_with_tool_calls()` in `mock_provider.rs` to accept a configurable tool name and arguments, or add new mock helpers per tool:

```rust
pub async fn mock_openai_with_tool_call(tool: &str, args: Value) -> (MockServer, String) {
    // Returns a tool_calls response with the specified tool on request 0,
    // then a plain stop response on request 1.
}
```

Write tests for `write_file`, `read_file`, `edit_file`, `bash`, and `glob` that:
1. Set up a wiremock server that returns the tool-call sequence
2. Drive the `ToolLoop` through the call
3. Assert the tool was dispatched to the correct handler
4. Assert the tool result was included in the subsequent model request

**For the Anthropic backend**, add fixture files in `crates/roko-agent/tests/fixtures/common/` with Anthropic-format tool call and tool result response bodies, then add tests in `fixture_loading.rs` (or a new `tool_matrix.rs`) that use those fixtures.

Use `roko_std::tool::handlers::handler_for` (already imported in `tool_loop_integration.rs:16`) to get real handlers for each tool, with a tempdir as the worktree.

No live API keys required; all network traffic is intercepted by wiremock or returned from in-process fixtures.

Estimated: ~200 lines in new test file(s) plus fixture JSON files.

## Acceptance Criteria

1. A new test `stdin_eof_causes_clean_server_exit` in `crates/roko-acp/tests/protocol_conformance.rs` passes: running the full `run_acp_server_with_transport` loop with an `empty()` reader returns `Ok(())`.
2. When a mock MCP subprocess crashes (process killed mid-session), the next ACP `session/update` notification includes an error block. A unit test validates this using a process that exits immediately.
3. `crates/roko-agent/tests/tool_matrix.rs` (or equivalent) defines at least one test per builtin tool (`read_file`, `write_file`, `edit_file`, `bash`, `glob`) for the OpenAI-compat backend using a wiremock mock server. All tests pass under `cargo test -p roko-agent`.
4. `cargo test -p roko-acp` and `cargo test -p roko-agent` pass with zero failures.
5. `cargo clippy -p roko-acp -p roko-agent -- -D warnings` is clean.

## Verification Checklist

- [ ] `cargo test -p roko-acp -- stdin_eof` passes
- [ ] MCP crash test: kill a mock subprocess mid-session; verify `session/update` contains error block
- [ ] `cargo test -p roko-agent -- tool_matrix` passes for all 5 tools via OpenAI-compat backend
- [ ] `cargo test -p roko-acp` passes (all 16 existing tests still pass, plus new tests)
- [ ] `cargo test -p roko-agent` passes
- [ ] `cargo clippy -p roko-acp -p roko-agent -- -D warnings` is clean

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-acp/tests/protocol_conformance.rs` | Add `stdin_eof_causes_clean_server_exit` test that exercises the full handler loop with empty reader |
| `crates/roko-acp/src/bridge_events.rs` | Propagate `McpErrorAccumulator` reference after tool loop; query for transport errors and emit error block in `session/update` |
| `crates/roko-agent/tests/mock_provider.rs` | Add configurable mock helpers per tool (`mock_openai_with_tool_call(tool, args)`) |
| `crates/roko-agent/tests/tool_matrix.rs` (new) | Cross-provider tool dispatch matrix for `read_file`, `write_file`, `edit_file`, `bash`, `glob` via OpenAI-compat backend |
| `crates/roko-agent/tests/fixtures/common/` | Add Anthropic-format tool call and tool result fixture JSON files |
