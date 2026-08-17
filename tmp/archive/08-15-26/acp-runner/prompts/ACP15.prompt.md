# Batch ACP15 — Session config options

## Goal

Implement the 7 session config options that render as native UI controls in the editor's settings panel.

## Target files

- `crates/roko-acp/src/config_options.rs` — Config option definitions and handlers

## Implementation details

### Config option definitions

Implement `build_config_options(state: &SessionConfigState) -> Vec<ConfigOption>` that returns all 7 options:

1. **agent_mode** (select, category: "mode")
   - Values: code, plan, research, review, auto
   - Default: code
   - Description: "Agent operating mode"

2. **model_tier** (select, category: "model")
   - Values: auto, t0, t1, t2, t3
   - Default: auto
   - Description: "Model routing strategy"

3. **thinking** (select, category: "thought_level")
   - Values: auto, off, brief, verbose
   - Default: auto
   - Description: "Thinking/reasoning visibility"

4. **gate_pipeline** (toggle, category: "_roko_verification")
   - Default: true
   - Description: "Run compile/test/clippy gates after code changes"

5. **auto_correct** (toggle, category: "_roko_verification")
   - Default: true
   - Description: "Conductor watchers auto-fix gate failures"

6. **knowledge_store** (toggle, category: "_roko_knowledge")
   - Default: true
   - Description: "Query durable knowledge store for context"

7. **daimon** (toggle, category: "_roko_affect")
   - Default: false
   - Description: "Enable affect engine (PAD state, somatic markers)"

### Config update handler

```rust
pub fn handle_config_update(
    state: &mut SessionConfigState,
    option_id: &str,
    new_value: serde_json::Value,
) -> Result<Vec<ConfigOption>>
```

This function:
1. Validates the new value for the option
2. Updates `SessionConfigState`
3. Returns the full updated config options list (some options may have dependent changes)

### Dependent updates

When `agent_mode` changes:
- In "plan" mode: disable "auto_correct" (plans don't auto-correct)
- In "research" mode: disable "gate_pipeline" (no code to gate)

When `model_tier` changes to "t0":
- Disable "thinking" (T0 is pattern match, no LLM thinking)

### Legacy set_mode handler

```rust
pub fn handle_set_mode(
    state: &mut SessionConfigState,
    mode_id: &str,
) -> Result<Vec<ConfigOption>>
```

Translates the legacy `session/set_mode` to a config update on `agent_mode`.

### Unit tests

- Test `build_config_options` returns 7 options
- Test changing mode updates dependent options
- Test toggle values serialize correctly
- Test invalid option ID returns error

## Verification

```bash
cargo test -p roko-acp --lib
cargo clippy -p roko-acp --no-deps -- -D warnings
```

## Done when

- All 7 config options are defined with correct types and values
- Config updates modify SessionConfigState correctly
- Dependent option updates work (mode → gate, model → thinking)
- Legacy set_mode handler works
- Unit tests pass
