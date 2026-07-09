# Repo Map — 12 Interfaces

High-value paths for batch `12`.

## Primary code anchors

- `crates/roko-cli/src/main.rs`
- `crates/roko-cli/src/config.rs`
- `crates/roko-cli/src/config_cmd.rs`
- `crates/roko-cli/src/agent_config.rs`
- `crates/roko-cli/src/agent_episode.rs`
- `crates/roko-cli/src/agent_exec.rs`
- `crates/roko-cli/src/agent_spawn.rs`
- `crates/roko-cli/src/daemon.rs`
- `crates/roko-cli/src/tui/`
- `crates/roko-serve/src/routes/`
- `crates/roko-agent-server/src/lib.rs`
- `crates/roko-agent-server/src/features/messaging.rs`
- `crates/roko-agent-server/src/registration.rs`
- `crates/roko-serve/README.md`
- `crates/roko-cli/README.md`

## Primary docs

- `docs/12-interfaces/01-cli-command-reference.md`
- `docs/12-interfaces/02-roko-new-scaffolders.md`
- `docs/12-interfaces/03-progressive-help-and-explain.md`
- `docs/12-interfaces/05-http-api-roko-serve.md`
- `docs/12-interfaces/06-websocket-streaming.md`
- `docs/12-interfaces/07-rosedust-design-language.md`
- `docs/12-interfaces/09-tui-29-screens.md`
- `docs/12-interfaces/17-accessibility-and-current-status.md`
- `docs/12-interfaces/20-ide-integration-strategy.md`

## Fastest verification searches

```bash
sed -n '180,760p' crates/roko-cli/src/main.rs
rg -n "9090|6677|serve_url" crates/roko-cli crates/roko-serve docs/12-interfaces tmp/docs-parity/12 --glob '*.rs' --glob '*.md'
rg -n 'route\\(\"/(stream|message|predictions|research|tasks|health)' crates/roko-agent-server/src --glob '*.rs'
rg -n "/api/events|/api/dashboard|/api/ws|/api/models|/api/routing/explain" crates/roko-serve/src/routes --glob '*.rs'
rg -n "29 screens|PostFX|Rosedust|Spectre|A2UI|sonif|ACP|VS Code" docs/12-interfaces crates --glob '*.md' --glob '*.rs'
```
