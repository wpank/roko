# Batch ACP12 — Gate result bridge

## Goal

Map gate execution events to ACP tool call notifications that render as cards in the editor.

## Target files

- `crates/roko-acp/src/bridge_gates.rs` — Gate result bridge

## Implementation details

### Gate event mapping

When a gate starts:
```rust
pub fn gate_started_notification(gate_name: &str, tool_call_id: &str) -> SessionUpdate
```
→ Returns `SessionUpdate::ToolCall` with status `InProgress`, kind `Other`

When a gate completes:
```rust
pub fn gate_completed_notification(
    gate_name: &str,
    tool_call_id: &str,
    passed: bool,
    summary: &str,
    duration_ms: u64,
) -> SessionUpdate
```
→ Returns `SessionUpdate::ToolCallUpdate` with status `Completed` (or `Failed`), content is a markdown summary

### Markdown summary format

For compile gate:
```markdown
## Compile Gate: ✓ PASSED

- **Target**: `roko-acp`
- **Time**: 4.2s
- **Warnings**: 0
- **Errors**: 0
```

For test gate:
```markdown
## Test Gate: ✗ FAILED

- **Time**: 12.7s
- **Passed**: 145/147
- **Failed**: 2
- **Failures**:
  - `test_session_cancel`
  - `test_concurrent_sessions`
```

For clippy gate:
```markdown
## Clippy Gate: ✓ PASSED

- **Time**: 8.1s
- **Warnings**: 0
```

### Helper function

```rust
pub fn format_gate_summary(gate_name: &str, passed: bool, details: &str, duration_ms: u64) -> String
```

Builds the markdown from structured gate data.

### Tool call ID convention

Gate tool call IDs: `gate_{gate_name}_{sequence}` (e.g., `gate_compile_001`)

## Verification

```bash
cargo check -p roko-acp
cargo clippy -p roko-acp --no-deps -- -D warnings
```

## Done when

- Gate started → ToolCall notification
- Gate completed → ToolCallUpdate notification with markdown summary
- Markdown formatting matches the examples
- Tool call IDs follow naming convention
