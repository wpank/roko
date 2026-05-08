# Batch ACP08 — Protocol conformance tests

## Goal

Write integration tests that verify the ACP protocol lifecycle end-to-end.

## Target files

- `crates/roko-acp/tests/protocol_conformance.rs` — Integration tests

## Implementation details

### Test infrastructure

Create a test harness that:
1. Creates a pair of in-memory async channels (simulating stdin/stdout)
2. Spawns the ACP handler on one end
3. Sends JSON-RPC messages on the other end
4. Reads and verifies responses

```rust
struct TestClient {
    writer: /* write to server's stdin */,
    reader: /* read from server's stdout */,
    next_id: u64,
}

impl TestClient {
    async fn send_request(&mut self, method: &str, params: Value) -> Value;
    async fn send_notification(&mut self, method: &str, params: Value);
    async fn read_notification(&mut self) -> (String, Value); // (method, params)
}
```

### Test cases

1. **test_initialize** — Send `initialize`, verify:
   - Response has `protocol_version: 1`
   - Agent info has `name: "roko"`
   - Capabilities include `load_session: true`

2. **test_session_new** — After init, send `session/new`, verify:
   - Response has a session_id starting with `sess_`
   - Config options list is returned (may be empty initially)

3. **test_session_list** — Create 2 sessions, send `session/list`, verify count

4. **test_session_prompt_basic** — Send `session/prompt` with text, verify:
   - Receive at least one `session/update` notification
   - Final response has `stop_reason: "end_turn"`
   - Response has usage info

5. **test_session_cancel** — Start a prompt, send `session/cancel`, verify:
   - Final response has `stop_reason: "cancelled"`

6. **test_unknown_method** — Send request with unknown method, verify:
   - Error response with code `-32601` (METHOD_NOT_FOUND)

7. **test_invalid_session** — Send prompt with non-existent session ID, verify:
   - Error response with code `-32000` (SESSION_NOT_FOUND)

8. **test_malformed_json** — Send invalid JSON, verify:
   - Error response with code `-32700` (PARSE_ERROR)

### Dependency

You may need to add `tokio-test` or use `#[tokio::test]` with tokio's test runtime.

If the transport struct requires refactoring to accept generic AsyncRead/AsyncWrite (not just stdin/stdout), do that refactoring in this batch.

## Verification

```bash
cargo test -p roko-acp
```

## Done when

- All 8 test cases compile and pass
- Tests use in-memory channels (no actual stdin/stdout)
- Protocol conformance is verified end-to-end
