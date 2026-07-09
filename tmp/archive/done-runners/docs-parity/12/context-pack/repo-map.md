# Repo Map — 12 Interfaces

High-value anchors for the topic-12 parity pass.

## Corrected Headlines

- `roko-serve`: 200+ routes / roughly 30K LOC
- topic-level TUI surface: 58K LOC
- port drift: `9090` in serve/daemon code, `6677` in chat + READMEs

## Primary Code Paths

- `crates/roko-cli/src/main.rs`
- `crates/roko-cli/src/config.rs`
- `crates/roko-cli/src/config_cmd.rs`
- `crates/roko-cli/src/tui/`
- `crates/roko-serve/src/routes/`
- `crates/roko-agent-server/src/lib.rs`
- `crates/roko-agent-server/src/features/messaging.rs`
- `crates/roko-serve/README.md`
- `crates/roko-cli/README.md`

## Primary Docs

- `docs/12-interfaces/00-cli-overview.md`
- `docs/12-interfaces/03-progressive-help-and-explain.md`
- `docs/12-interfaces/05-http-api-roko-serve.md`
- `docs/12-interfaces/06-websocket-streaming.md`
- `docs/12-interfaces/07-rosedust-design-language.md`
- `docs/12-interfaces/09-tui-29-screens.md`
- `docs/12-interfaces/17-accessibility-and-current-status.md`
- `docs/12-interfaces/20-ide-integration-strategy.md`

## Fast Checks

```bash
sed -n '191,372p' crates/roko-cli/src/main.rs
sed -n '503,520p' crates/roko-cli/src/main.rs
sed -n '658,677p' crates/roko-cli/src/main.rs
sed -n '55,83p' crates/roko-serve/src/routes/mod.rs
sed -n '36,45p' crates/roko-serve/src/routes/providers.rs
sed -n '29,33p' crates/roko-agent-server/src/features/messaging.rs
rg -n "9090|6677|roko new|explain|Spectre|A2UI|Svelte|sonif|ACP|VS Code" crates docs/12-interfaces tmp/docs-parity/12 --glob '*.rs' --glob '*.md'
```
