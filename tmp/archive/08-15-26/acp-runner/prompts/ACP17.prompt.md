# Batch ACP17 — Elicitation forms

## Goal

Implement the elicitation system for showing structured forms to the user via the editor.

## Target files

- `crates/roko-acp/src/elicitation.rs` — Elicitation implementation

## Implementation details

### Core function

```rust
pub async fn request_elicitation(
    transport: &mut StdioTransport,
    session_id: &str,
    message: &str,
    schema: serde_json::Value,
) -> Result<Option<serde_json::Value>>
```

Sends `elicitation/create` request to the editor and returns the form data if accepted, None if rejected.

### JSON-RPC message

```json
{
    "jsonrpc": "2.0",
    "id": N,
    "method": "elicitation/create",
    "params": {
        "sessionId": "sess_...",
        "mode": "form",
        "message": "Configure gate pipeline for this workspace",
        "requestedSchema": { ... }
    }
}
```

Response:
```json
{
    "jsonrpc": "2.0",
    "id": N,
    "result": {
        "outcome": "accept",
        "data": { ... }
    }
}
```

### Pre-built schemas

1. **Gate pipeline configuration**

```rust
pub fn gate_config_schema() -> serde_json::Value
```

Schema with fields:
- `compile_target`: enum ["workspace", "current-crate", "affected-crates"]
- `test_filter`: string (test name regex)
- `clippy_deny_warnings`: boolean
- `max_gate_retries`: integer (0-5)
- `session_budget_usd`: number (0.01-100.0)
- `enabled_gates`: array of enum ["compile", "test", "clippy", "fmt", "doc", "coverage"]

Required: `compile_target`, `enabled_gates`

2. **Research source selection**

```rust
pub fn research_source_schema() -> serde_json::Value
```

Schema with fields:
- `sources`: array of enum ["web", "perplexity", "knowledge_store", "codebase", "documentation"]
- `depth`: enum ["shallow", "medium", "deep"]
- `max_results`: integer (1-50)

Required: `sources`

### Schema validation

```rust
pub fn validate_elicitation_response(
    schema: &serde_json::Value,
    data: &serde_json::Value,
) -> Result<()>
```

Basic validation: check required fields are present, types match. Full JSON Schema validation is not required — basic type checking is sufficient.

## Verification

```bash
cargo check -p roko-acp
cargo clippy -p roko-acp --no-deps -- -D warnings
```

## Done when

- `request_elicitation` sends correct JSON-RPC and handles accept/reject
- Gate config and research source schemas are defined
- Basic response validation works
- All types serialize correctly
