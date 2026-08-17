# Batch ACP18 — Lifecycle integration tests

## Goal

Write comprehensive end-to-end lifecycle tests covering config options, slash commands, session management, and error handling.

## Target files

- `crates/roko-acp/tests/lifecycle.rs` — Integration tests

## Implementation details

### Test infrastructure

Reuse the TestClient from ACP08 (protocol_conformance.rs). If needed, extract it to a `tests/common/mod.rs` helper.

### Test cases

1. **test_config_option_change_flow**
   - Create session
   - Read initial config options
   - Send `session/config/update` to change `agent_mode` to "plan"
   - Verify response includes updated config options
   - Verify dependent options were updated (auto_correct disabled in plan mode)

2. **test_slash_command_flow**
   - Create session
   - Send `session/prompt` with `/status` as text
   - Verify response includes status information
   - Send `session/prompt` with `/budget` as text
   - Verify response includes budget information

3. **test_session_load_resume**
   - Create session A
   - Send a prompt to session A
   - Create new session B
   - Load session A by ID
   - Verify session A state is restored

4. **test_multi_session_concurrent**
   - Create 3 sessions
   - Verify each has independent config state
   - Change mode on session 1, verify sessions 2 and 3 unchanged
   - List sessions, verify count is 3

5. **test_error_invalid_session_id**
   - Send `session/prompt` with session_id "nonexistent"
   - Verify JSON-RPC error with code SESSION_NOT_FOUND (-32000)

6. **test_error_unknown_method**
   - Send request with method "foobar/unknown"
   - Verify JSON-RPC error with code METHOD_NOT_FOUND (-32601)

7. **test_error_malformed_json**
   - Send raw string that is not valid JSON
   - Verify JSON-RPC error with code PARSE_ERROR (-32700)

8. **test_config_option_validation**
   - Send `session/config/update` with invalid option_id
   - Verify error response
   - Send `session/config/update` with invalid value for a select option
   - Verify error response

9. **test_legacy_set_mode**
   - Create session
   - Send `session/set_mode` with mode_id "research"
   - Verify config_options reflect the mode change

10. **test_available_commands_update**
    - Create session in "code" mode
    - Verify `/plan` is in available commands
    - Change mode to "plan"
    - Verify `/plan` is hidden (dynamic filtering)

## Verification

```bash
cargo test -p roko-acp
```

## Done when

- All 10 test cases compile and pass
- Tests cover config options, commands, sessions, errors
- Tests use the shared TestClient infrastructure
- Edge cases are covered (invalid inputs, concurrent sessions)
