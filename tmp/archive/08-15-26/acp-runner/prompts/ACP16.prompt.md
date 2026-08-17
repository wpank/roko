# Batch ACP16 — Slash commands

## Goal

Define the 8 slash commands and implement command parsing and dispatch.

## Target files

- `crates/roko-acp/src/commands.rs` — Command definitions and dispatch

## Implementation details

### Static command list

```rust
pub fn build_available_commands() -> Vec<SlashCommand>
```

Returns 8 commands:

1. **/plan** — "Create a multi-step implementation plan as a task DAG"
   - Input hint: "what to build or change"

2. **/gate** — "Run the gate pipeline now (compile, test, clippy)"
   - Input hint: "optional: specific gate — compile, test, clippy, all"

3. **/learn** — "Show what the agent has learned — knowledge entries with confidence scores"
   - Input hint: "optional: topic to filter by"

4. **/inspect** — "Inspect internal state: sessions, tokens, routing, daimon"
   - Input hint: "optional: subsystem to inspect"

5. **/replay** — "Walk the signal DAG by hash"
   - Input hint: "signal hash to start from"

6. **/heuristics** — "Show active heuristics with confidence scores and evidence"
   - No input

7. **/status** — "Show agent status — PAD state, vitality, active watchers, knowledge stats"
   - No input

8. **/budget** — "Show remaining token/cost budget and projected session cost"
   - No input

### Dynamic filtering

```rust
pub fn dynamic_commands(
    commands: &[SlashCommand],
    config_state: &SessionConfigState,
) -> Vec<SlashCommand>
```

Adjusts available commands based on context:
- Hide `/plan` when already in plan mode
- Hide `/gate` when gate pipeline is disabled
- Hide `/learn` when knowledge store is disabled
- Hide `/heuristics` when knowledge store is disabled

### Command parsing

```rust
pub fn parse_slash_command(prompt_text: &str) -> Option<(String, Option<String>)>
```

Parses `/command [input]` from prompt text:
- `/plan implement user auth` → `("plan", Some("implement user auth"))`
- `/status` → `("status", None)`
- `regular text` → `None`

### Command dispatch

```rust
pub async fn dispatch_command(
    command: &str,
    input: Option<&str>,
    session: &AcpSession,
) -> Result<Vec<CognitiveEvent>>
```

Each command generates a series of CognitiveEvents that get streamed back via bridge_events. For now, implement simple placeholder responses:
- `/status` → TokenChunk with formatted status text
- `/budget` → TokenChunk with budget info
- Others → TokenChunk with "Command X executed" placeholder

## Verification

```bash
cargo check -p roko-acp
cargo clippy -p roko-acp --no-deps -- -D warnings
```

## Done when

- 8 slash commands defined with correct descriptions
- Dynamic filtering works based on config state
- Command parsing extracts command + input
- Dispatch returns appropriate events
