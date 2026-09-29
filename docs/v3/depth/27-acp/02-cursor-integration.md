# 27.02 -- Cursor Integration

> Depth file for [27-ACP.md](../../27-ACP.md).

---

## Editor Launch

Cursor launches the ACP server as a child process via stdio. The server binary
is `roko acp`, which calls `run_acp_server()` with an `AcpConfig`:

```rust
pub struct AcpConfig {
    pub workdir: PathBuf,             // Editor's workspace root
    pub profile: String,              // Named config profile
    pub config_path: Option<PathBuf>, // Explicit --config path
    pub global_config_path: Option<PathBuf>, // --global-config
    pub log_file: PathBuf,            // ACP log output
}
```

The editor passes arguments via command-line flags. The server logs to the
configured file (default `.roko/acp.log`) since stdout is reserved for
JSON-RPC traffic.

## Workspace Detection

On startup, the ACP server:

1. Canonicalizes `workdir` and creates `.roko/` if missing
2. Checks for `roko.toml` in the workspace root
3. Falls back to global config at `~/.roko/config.toml`
4. Logs provider availability warnings

If no `roko.toml` is found:

```
WARN no roko.toml found in ACP workdir -- using implicit global config
     from ~/.roko/config.toml. To configure: add roko.toml to the
     project root, or set --global-config in your editor's ACP settings.
```

## Config Layering

The ACP server merges multiple config sources with explicit precedence:

1. **Editor config** (`--config`) -- highest priority for project-specific settings
2. **Workspace config** (`roko.toml`) -- project root file
3. **Global config** (`--global-config` or `~/.roko/config.toml`)
4. **Environment** (`ROKO_CONFIG` env var)
5. **Built-in defaults**

The `config_sources()` method reports active files with prefixed labels:

```json
["global:~/.roko/config.toml", "project:/workspace/roko.toml"]
```

### Inheritance Rules

Global config provides providers and models that the editor config can inherit
without overriding:

```rust
fn merge_inherited_config(config: &mut RokoConfig, global: RokoConfig) {
    for (name, provider) in global.providers {
        config.providers.entry(name).or_insert(provider);
    }
    for (name, model) in global.models {
        config.models.entry(name).or_insert(model);
    }
    // default_model inherited only if local hasn't explicitly set one
}
```

This means: editor-level `bare_mode = true` is preserved even when the global
config sets `default_model = "global-model"`.

## Config Hot Reload

A `ConfigWatcher` monitors the workspace config file for changes. On each
request, the handler checks `config_watcher.changed()` and reloads if needed:

```rust
if config_watcher.changed() {
    let (refreshed, reload_warning) = config.load_roko_config_with_warning();
    sessions.replace_roko_config(refreshed);
    // Push updated config options to each session
    // Push configSources notification if source list changed
}
```

This enables live config updates without restarting the ACP server.

## Bare Mode

Bare mode controls whether the agent uses its built-in system prompt or roko's
9-layer SystemPromptBuilder. Resolution logic:

```rust
pub fn resolve_bare_mode(config_override: Option<bool>, workdir: &Path) -> bool {
    // 1. Explicit config override (bare_mode = false means full mode)
    // 2. Workspace auto-detection
    // 3. Default to true for Cursor-style IDEs
}
```

When `bare_mode = true`, the Claude CLI's `--system-prompt` replaces its
built-in prompt. When `false`, `--append-system-prompt` is used instead.

## Slash Commands

The ACP server advertises available slash commands to the IDE via
`session/update` notifications:

```json
{
  "sessionUpdate": "available_commands_update",
  "availableCommands": [
    {
      "name": "/plan",
      "description": "Generate an implementation plan"
    },
    {
      "name": "/research",
      "description": "Research a topic"
    }
  ]
}
```

The command set varies based on bare mode and session configuration.

## Session Lifecycle in Cursor

1. **Editor opens** -- Cursor spawns `roko acp` as stdio child process
2. **Handshake** -- `initialize` request/response
3. **Session creation** -- `session/new` when user starts a chat
4. **Prompt dispatch** -- `session/prompt` for each user message
5. **Live updates** -- `session/update` notifications stream plan progress
6. **Session persistence** -- Sessions are persisted to disk after each prompt
7. **Reconnection** -- `session/load` or `session/resume` after editor restart
8. **GC** -- Old sessions are cleaned up after 7 days

## Provider Readiness Check

On startup, the server validates that at least one provider has resolvable
credentials:

```rust
fn check_provider_readiness(config: &RokoConfig) -> Option<String> {
    if config.providers.is_empty() {
        return Some("no providers configured");
    }
    for (_, provider) in &config.providers {
        if provider.kind == ProviderKind::ClaudeCli { return None; }
        if api_key_env_is_set(provider) { return None; }
    }
    Some("no provider has resolvable credentials")
}
```

CLI-based providers (ClaudeCli, Hermes, OpenClaw) do not need API key env vars.
The warning is included in the `initialize` response as `configWarnings`.

## Image Capabilities

The `initialize` response advertises whether the resolved default model
supports image input:

```rust
let prompt_capabilities = advertised_prompt_capabilities_for_model(
    resolved.provider_kind,
    resolved.profile.is_some_and(|p| p.supports_vision),
);
```

This allows Cursor to enable or disable image attachment UI elements based
on model capabilities.

## Source

- `crates/roko-acp/src/config.rs` -- AcpConfig, config layering, source resolution
- `crates/roko-acp/src/handler.rs` -- Initialize handler and dispatch loop
- `crates/roko-acp/src/config_watch.rs` -- Config file watcher
- `crates/roko-acp/src/session.rs` -- Session management and bare mode
