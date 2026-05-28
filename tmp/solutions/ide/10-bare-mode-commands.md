# Issue: bare_mode Still Exposes Workspace Commands

## Problem Statement

When `agent.bare_mode = true`, the session still returns 50+ `availableCommands`
in the `available_commands_update` notification. These include workspace-specific
commands (PRDs, plans, knowledge, gates) that make no sense in an IDE context.

## Observed Behavior

With `bare_mode = true`, the commands update still includes:
- `prd-idea`, `prd-draft`, `prd-list`, `prd-status`, `prd-plan`, `prd-consolidate`
- `plan-list`, `plan-show`, `plan-generate`, `plan-validate`, `plan-run`, `plan-resume`
- `knowledge`, `knowledge-stats`, `knowledge-gc`, `knowledge-backup`
- `dream`, `replay`, `learn-router`, `learn-episodes`, `learn-tune`
- `gate`, `clippy`, `fmt`, `build`, `test`
- `agents`, `agent-chat`, `agent-start`, `agent-stop`
- `workflow`, `express`, `full`, `review-this`, `pipeline`

## Impact

- Clutters the IDE's command palette if it displays these
- Wastes tokens in the system prompt (commands list is included in agent context)
- Confusing for users who see "plan-run" in an IDE that doesn't support plans

## Expected Behavior

With `bare_mode = true`, only a minimal set of commands should be exposed:
- `status` — basic health check
- `help` — list available commands
- Maybe `config` — show current config

## Root Cause

The `available_commands_update` is likely constructed from a static list or
workspace scanner that doesn't check `bare_mode`. Need to find where commands
are registered and add a filter.

## Proposed Solution

### Filter commands by mode

```rust
fn build_available_commands(config: &RokoConfig) -> Vec<AvailableCommand> {
    let all_commands = register_all_commands();

    if config.agent.bare_mode {
        all_commands.into_iter()
            .filter(|cmd| cmd.category == CommandCategory::Core)
            .collect()
    } else {
        all_commands
    }
}
```

### Categorize commands

```rust
enum CommandCategory {
    Core,       // status, help, config
    Workspace,  // PRDs, plans, knowledge
    Build,      // gate, clippy, test, build
    Agent,      // agents, agent-chat
    Research,   // research, search
}

struct AvailableCommand {
    name: String,
    description: String,
    category: CommandCategory,
    #[serde(skip)]
    requires_workspace: bool,
}
```

### IDE-specific: allow command filtering in session/new

```json
{
  "method": "session/new",
  "params": {
    "commandFilter": ["core", "research"]  // Only show these categories
  }
}
```

## Implementation Location

| File | Change |
|------|--------|
| `crates/roko-acp/src/handler.rs` (send_slash_commands_notification) | Filter by bare_mode |
| `crates/roko-acp/src/commands/` | Add category to command registration |

## Priority

Low. The IDE can ignore commands it doesn't support. But fixing this reduces
token waste in the system prompt and provides a cleaner UX.
