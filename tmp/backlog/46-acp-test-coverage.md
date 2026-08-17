# ACP Test Coverage Gaps

**Priority**: P2
**Size**: M (1-3 days)

---

## Problem

The ACP (Agent Client Protocol) layer has 180 unit tests that pass, but three specific
coverage gaps are known and unfixed:

**Gap 1: stdin-EOF does not produce a clean exit.**
The `StdioTransport` in `crates/roko-acp/src/transport.rs` reads incoming JSON-RPC messages
line-by-line. When the editor closes stdin (e.g. when Cursor is shut down mid-session), the
`read_line` future returns `Ok(0)` to signal EOF. The current handler loop does not
distinguish this from a transport error; it either hangs waiting for the next message or
propagates an error code to the caller instead of performing a clean orderly shutdown.
This shows up as a failure in the 63-test shell-based integration suite.

**Gap 2: MCP subprocess crashes are silently swallowed.**
`McpClient<StdioTransport>` in `crates/roko-agent/src/mcp/client.rs` holds a `Child`
process handle (`_child: Mutex<Child>`) with `kill_on_drop(true)`. When the spawned MCP
server process crashes (exits unexpectedly), the next call to `roundtrip` will fail with a
broken-pipe or EOF error from `stdin.write_all`. That error surfaces as
`McpError::Transport(...)` inside `McpToolHandler::call` in `handler.rs`.

`McpErrorAccumulator` exists (`crates/roko-agent/src/mcp/error_accumulator.rs`) and is
wired to `McpHandlerResolver`. Transport errors from crashed servers are recorded by the
accumulator. However, the ACP session never queries the accumulator's crash-category errors
and never emits a user-visible `session/update` with an error block. From the IDE's
perspective the server silently stops responding — no visible indication.

**Gap 3: No cross-provider tool dispatch matrix.**
The mock infrastructure in `crates/roko-agent/tests/mock_provider.rs` provides
`mock_openai_compat()` (wiremock-backed mock server) and `mock_openai_with_tool_calls()`
(a stateful responder that plays back a tool-call/result sequence). Anthropic mock responses
exist as JSON fixtures in `tests/fixtures/`. Tests for individual adapters exist
(`openai_parity.rs`, `cursor_parity.rs`, `codex_parity.rs`), but no test verifies that
the five standard builtin tools (`read_file`, `write_file`, `edit_file`, `bash`, `glob`)
work end-to-end through both the OpenAI-compat backend and the Anthropic API path using
mock providers. If a translator change breaks tool call deserialization on one backend but
not another, no test catches it.

---

### What already exists

| Component | Location | Status |
|---|---|---|
| `StdioTransport` (ACP) | `crates/roko-acp/src/transport.rs` | EXISTS — `BufReader<Stdin>`, `read_line`-based loop; no EOF branch |
| ACP handler main loop | `crates/roko-acp/src/handler.rs` | EXISTS — `run_acp_server_inner` main dispatch loop |
| `McpErrorAccumulator` | `crates/roko-agent/src/mcp/error_accumulator.rs` | EXISTS — records errors, `snapshot()` / `drain()` API |
| `McpHandlerResolver` with error accumulator | `crates/roko-agent/src/mcp/handler.rs:50` | EXISTS — `with_error_accumulator()` wired; accumulator polled is caller's responsibility |
| `McpErrorRecord.is_transport_error` | `crates/roko-agent/src/mcp/error_accumulator.rs:31` | EXISTS — flag distinguishes transport vs server errors |
| `mock_openai_compat()` | `crates/roko-agent/tests/mock_provider.rs:9` | EXISTS — wiremock server, single response |
| `mock_openai_with_tool_calls()` | `crates/roko-agent/tests/mock_provider.rs:22` | EXISTS — stateful tool-call/result sequence responder |
| Anthropic JSON fixtures | `crates/roko-agent/tests/fixtures/` | EXISTS — recorded response bodies |
| `fixture_loading.rs` | `crates/roko-agent/tests/fixture_loading.rs` | EXISTS — loads fixture JSON, drives adapters |
| ACP unit tests (180) | `crates/roko-acp/tests/` | PASS — `protocol_conformance.rs`, `telemetry_integration.rs` |
| Shell integration suite | `crates/roko-acp/tests/` (shell scripts) | PARTIAL — 63 tests, 3 known failures |

---

### What is missing

**Fix 1: Clean EOF handling in `handler.rs` / `transport.rs`**

`StdioTransport::receive_message` (or whatever the read method is called in the ACP
transport) should detect the `Ok(0)` return from `read_line` (zero bytes = EOF) and return
a distinct variant — either a new `TransportError::Eof` or `Ok(None)` from an
`Option`-returning method. The handler loop in `run_acp_server_inner` must handle this
variant by performing graceful shutdown: cancel in-flight sessions, flush any pending
session updates, and return `Ok(())` rather than an error. This is the same pattern used
by well-behaved language server processes.

**Fix 2: MCP crash detection and user-visible error surfacing**

When `McpErrorAccumulator` collects a record with `is_transport_error = true` that
corresponds to a broken-pipe or EOF error (indicating subprocess death), the ACP session
should emit a `session/update` notification with an error content block to the editor.
The right place is after each tool loop turn: query the accumulator for transport errors,
and if any are present, include a structured error block in the next outbound update.
Alternatively, a background task that watches `Child::wait()` (already held in
`_child: Mutex<Child>`) can proactively detect subprocess exit and push a notification
without waiting for the next tool call to fail.

**Fix 3: Cross-provider tool dispatch matrix**

Create `crates/roko-agent/tests/tool_matrix/` (or a new test file in the existing
`tests/` directory) with tests that:

- Use `mock_openai_compat()` to back an OpenAI-compat provider adapter.
- Drive the tool loop through `write_file`, `read_file`, `bash`, `edit_file`, and `glob`
  tool calls in sequence, using `mock_openai_with_tool_calls()` to return the sequence.
- Assert that each tool call is correctly deserialized, dispatched to the builtin handler,
  and that the tool result is included in the next model turn as expected.
- Repeat the same sequence using an Anthropic fixture (the mock HTTP server returns the
  recorded fixture body for tool-call and tool-result turns).

The tests do not need live API keys. All network traffic is intercepted by wiremock or
returned from in-process fixtures.

---

## Where to make changes

| File | Change |
|---|---|
| `crates/roko-acp/src/transport.rs` | Add `Eof` variant to `TransportError`; distinguish `Ok(0)` from `read_line` |
| `crates/roko-acp/src/handler.rs` | Handle `TransportError::Eof` in the main dispatch loop with graceful shutdown |
| `crates/roko-acp/src/bridge_events.rs` | After each tool loop turn, query the `McpErrorAccumulator` for transport errors and emit a user-visible error block |
| `crates/roko-agent/src/mcp/client.rs` | Optionally expose `child_exit_status()` or a watch channel so callers can proactively detect process death |
| `crates/roko-agent/tests/tool_matrix.rs` (new) | Cross-provider tool dispatch matrix for OpenAI-compat and Anthropic backends |

---

## Acceptance criteria

1. Closing stdin on a running `roko acp` process (e.g. `echo "" | roko acp`) causes the
   process to exit with status 0 rather than hanging or exiting with a non-zero error code.
   The corresponding unit test in `crates/roko-acp/tests/protocol_conformance.rs` passes.
2. When a mock MCP subprocess crashes (process killed mid-session), the next ACP
   `session/update` notification includes an error block visible to the IDE client. A unit
   test validates this using a process that immediately exits.
3. `crates/roko-agent/tests/tool_matrix.rs` (or equivalent) defines at least one test per
   builtin tool (`read_file`, `write_file`, `edit_file`, `bash`, `glob`) for both the
   OpenAI-compat and Anthropic backends using mock providers. All tests pass under
   `cargo test -p roko-agent`.
4. `cargo test -p roko-acp` and `cargo test -p roko-agent` pass with zero failures.
5. `cargo clippy -p roko-acp -p roko-agent -- -D warnings` is clean.

---

## References

- `crates/roko-acp/src/transport.rs` — `StdioTransport`, `TransportError`, `read_line` loop
- `crates/roko-acp/src/handler.rs` — `run_acp_server_inner`, main message dispatch loop
- `crates/roko-agent/src/mcp/client.rs:198` — `StdioTransport`, `_child: Mutex<Child>`, `kill_on_drop(true)`
- `crates/roko-agent/src/mcp/error_accumulator.rs` — `McpErrorAccumulator`, `McpErrorRecord.is_transport_error`
- `crates/roko-agent/src/mcp/handler.rs` — `McpHandlerResolver`, `with_error_accumulator`
- `crates/roko-agent/tests/mock_provider.rs` — wiremock mock servers for OpenAI-compat
- `crates/roko-agent/tests/fixture_loading.rs` — fixture-based adapter tests
- `crates/roko-acp/tests/protocol_conformance.rs` — existing ACP unit tests (180 pass)
