# W3-A: Command Categories + bare_mode Filtering

## Context

`roko acp` sends a list of ~47 slash commands to the IDE after session creation. These include
workspace-specific commands (PRD, plan, knowledge, dream, gate) that make no sense in IDE mode.
When `bare_mode = true` in config, these should be filtered out. Currently `build_slash_commands()`
is a static list with no filtering.

## File Locations

Three files:
1. `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/types.rs` — SlashCommand struct
2. `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/session.rs` — build_slash_commands()
3. `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/handler.rs` — send_slash_commands_notification()

## Change 1: Add category to SlashCommand

**File:** `types.rs`

FIND (lines 635-646):
```rust
/// A slash command exposed by the session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlashCommand {
    /// Slash command name.
    pub name: String,
    /// Slash command description.
    pub description: String,
    /// Optional command input metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<CommandInput>,
}
```

REPLACE WITH:
```rust
/// A slash command exposed by the session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlashCommand {
    /// Slash command name.
    pub name: String,
    /// Slash command description.
    pub description: String,
    /// Optional command input metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<CommandInput>,
    /// Command category for filtering.
    /// Known values: "core", "research", "workspace", "build", "agent", "knowledge", "workflow".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
}
```

## Change 2: Change build_slash_commands signature and add categories

**File:** `session.rs`

FIND the function signature (line 1116):
```rust
pub fn build_slash_commands() -> Vec<SlashCommand> {
    vec![
```

REPLACE WITH:
```rust
pub fn build_slash_commands(bare_mode: bool) -> Vec<SlashCommand> {
    let all = vec![
```

Now you need to add `category: Some("xxx".to_owned()),` to EVERY SlashCommand in the vec.
The categories are:

**"core"** (always shown): `status`, `doctor`, `config`, `help`
**"research"** (shown in bare_mode): `research`, `search`, `enhance-prd`, `analyze`
**"workspace"** (hidden in bare_mode): all `prd-*` and `plan-*` commands
**"build"** (hidden in bare_mode): `build`, `test`, `clippy`, `fmt`, `gate`, `review`
**"agent"** (hidden in bare_mode): `agents`, `agent-chat`, `agent-start`, `agent-stop`
**"knowledge"** (hidden in bare_mode): `knowledge*`, `dream`, `replay`, `learn*`, `audit`
**"workflow"** (hidden in bare_mode): `workflow`, `express`, `full`, `review-this`, `pipeline`, `run`

For example, the first few commands become:
```rust
        SlashCommand {
            name: "status".to_owned(),
            description: "Workspace status: signals, agents, runs, knowledge".to_owned(),
            input: None,
            category: Some("core".to_owned()),
        },
        SlashCommand {
            name: "doctor".to_owned(),
            description: "Diagnose workspace bootstrap state".to_owned(),
            input: None,
            category: Some("core".to_owned()),
        },
        SlashCommand {
            name: "config".to_owned(),
            description: "Show roko.toml configuration".to_owned(),
            input: None,
            category: Some("core".to_owned()),
        },
        SlashCommand {
            name: "learn".to_owned(),
            description: "Learning state: episodes, routing, experiments, efficiency".to_owned(),
            input: None,
            category: Some("knowledge".to_owned()),
        },
```

Apply the same pattern to ALL ~47 commands. If unsure about a command's category, use "workspace".

## Change 3: Add filtering at end of build_slash_commands

FIND the closing of the vec (end of function, around line 1426-1427):
```rust
    ]
}
```

REPLACE WITH:
```rust
    ];

    if bare_mode {
        all.into_iter()
            .filter(|cmd| matches!(
                cmd.category.as_deref(),
                Some("core") | Some("research")
            ))
            .collect()
    } else {
        all
    }
}
```

## Change 4: Update send_slash_commands_notification

**File:** `handler.rs`

FIND (lines 363-379):
```rust
async fn send_slash_commands_notification(
    transport: &mut StdioTransport<impl AsyncRead + Unpin, impl AsyncWrite + Unpin>,
    session_id: &str,
) -> Result<()> {
    let commands = crate::session::build_slash_commands();
```

REPLACE WITH:
```rust
async fn send_slash_commands_notification(
    transport: &mut StdioTransport<impl AsyncRead + Unpin, impl AsyncWrite + Unpin>,
    session_id: &str,
    bare_mode: bool,
) -> Result<()> {
    let commands = crate::session::build_slash_commands(bare_mode);
```

## Change 5: Update call sites in handler.rs

**Call site 1 — session/new (line 176):**

FIND:
```rust
            send_slash_commands_notification(transport, &session_id).await
```

REPLACE WITH:
```rust
            send_slash_commands_notification(transport, &session_id, sessions.roko_config.agent.bare_mode).await
```

**Call site 2 — session/resume (line 276):**

FIND:
```rust
            send_slash_commands_notification(transport, &session_id).await?;
```

REPLACE WITH:
```rust
            send_slash_commands_notification(transport, &session_id, sessions.roko_config.agent.bare_mode).await?;
```

## What NOT to Change

- Do NOT modify `CommandInput` struct
- Do NOT modify handler logic beyond the call sites above
- Do NOT add client-side command filtering yet (defer to future batch)

## Verification

After Phase 2:
```bash
# With bare_mode=true config, count commands
echo '{"jsonrpc":"2.0","method":"session/new","id":1,"params":{}}' \
  | cargo run -p roko-cli -- acp --quiet --no-serve --config ~/.nunchi/roko/roko.toml 2>/dev/null \
  | python3 -c "
import sys
for line in sys.stdin:
  line = line.strip()
  if not line: continue
  import json
  try:
    d = json.loads(line)
    u = d.get('params',{}).get('update',{})
    if u.get('sessionUpdate') == 'available_commands_update':
      cmds = u.get('availableCommands', [])
      print(f'Commands: {len(cmds)}')
      for c in cmds:
        print(f'  {c[\"name\"]} [{c.get(\"category\",\"?\")}]')
      break
  except: pass
"
# Should show ~6-8 commands (core + research), not ~47
```

## Estimated Effort

30-45 minutes. The bulk is adding `category` to 47 commands.
