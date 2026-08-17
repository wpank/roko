# ACP: Editor Compatibility Issues

> **Source**: roko-acp tests, ACP spec 0.12.2, web research, codebase analysis
> **References**: `tmp/acp-features/00-ACP-FEATURES.md`, `crates/roko-acp/src/`
> **Created**: 2026-08-15

## Editor Support Matrix

| Editor | Config Method | Tested? | Known Issues |
|---|---|---|---|
| Zed | `~/.config/zed/settings.json` under `agent_servers` | Yes (in production) | Working per `00-ACP-FEATURES.md`; Zed-specific wire format for `session/request_permission` response verified in unit test |
| Cursor | Settings JSON under `agent.customAgents` | Config documented, not integration-tested | Dual method name `session/config/update` / `session/set_config_option` suggests Cursor compatibility shim |
| JetBrains | `acp.json` via "Add Custom Agent" dialog | Not tested | No JetBrains-specific code or tests in crate; may need `acp.json` generation helper |
| Neovim | Manual stdio launch | Not tested | Mentioned in `lib.rs` doc comment but no config example or tests |
| VS Code | No native ACP support | Not tested | ACP not natively supported by VS Code; would need extension |
| Microsoft Terminal | ACP client via Codex CLI | Not tested | Mentioned in ecosystem docs; no roko-specific testing |

## Protocol Conformance

Roko implements ACP spec version **0.12.2** with protocol version **1**. Key conformance points:

- 8 integration tests in `crates/roko-acp/tests/protocol_conformance.rs`
- 14+ unit tests in `types.rs`, `transport.rs`, `handler.rs`
- Tests cover: `initialize`, `session/new`, `session/list`, `session/prompt`, `session/cancel`, unknown method, invalid session, malformed JSON, startup resilience (no config, malformed config, missing credentials)

## Zed-Specific Issues

### What works

- **Stdio JSON-RPC transport**: newline-delimited JSON over stdin/stdout, matching Zed's `ShellBuilder` + `Stdio::piped()` pattern.
- **Permission wire format**: explicit test in `types.rs` (line 1235) verifies deserialization of what "Zed actually sends" for `session/request_permission` responses:
  ```json
  { "outcome": { "type": "selected", "optionId": "allow_always" } }
  ```
- **Config options**: model selector, effort level, temperament, routing mode, gate toggles -- all serialized as `configOptions` in `session/new` response.
- **Slash commands**: 31 of 35 commands wired, sent via `available_commands_update` notification after `session/new`.
- **Startup error surfacing**: `handler.rs` line 34 writes a JSON-RPC error to stdout on fatal startup failure so Zed shows a meaningful message instead of "server shut down unexpectedly".
- **Content block alias**: `ContentBlock::Text` accepts the `"content"` type alias inbound (line 511 in `types.rs`), handling legacy Zed message formats.
- **Config hot-reload**: `ConfigWatcher` uses `notify::RecommendedWatcher` to detect `roko.toml` changes and push `config_option_update` / `server/config_sources_update` notifications to the IDE without restart.
- **`session/resume`**: supported in handler, sends slash commands and config options after resume.

### Known gaps

1. **Working directory**: Zed issue [#46138](https://github.com/zed-industries/zed/issues/46138) reports that custom agent server launch does not respect the project's working directory. Roko uses `AcpConfig.workdir` set by the CLI, but if Zed does not pass `cwd` correctly, file operations and `roko.toml` resolution may fail silently.

2. **MCP passthrough**: `mcpServers` are stored per session (`SessionNewParams`) but not forwarded to the underlying agent dispatch (feature checklist item 8 "MCP server passthrough" is marked "Not started"). Zed issue [#52254](https://github.com/zed-industries/zed/issues/52254) reports that remote ACP is incompatible with remote MCP -- MCP servers passed through ACP may not be accessible.

3. **Custom shell interference**: Zed issue [#47991](https://github.com/zed-industries/zed/issues/47991) -- agent servers do not work with custom terminal shells (fish, nushell). If Zed uses the user's shell to spawn `roko-cli acp`, shell init scripts may write to stdout and corrupt the JSON-RPC stream. Roko has no mitigation for this.

4. **Agent registration after restart**: Zed issue [#50807](https://github.com/zed-industries/zed/issues/50807) -- `claude-acp` agent server not registered after every Zed restart. If this affects custom agents generally, Roko may intermittently not appear in the agent picker.

5. **File change notifications**: ACP supports `file_change` session updates but roko-acp does not emit them (feature checklist section 5: "File change notifications" = "Not started"). Zed may rely on these to refresh its file tree after agent edits.

6. **Conversation history accumulation**: Multi-turn context is not persisted across prompts within a session (`session.rs` stores in-memory state only). Zed users expect multi-turn conversations but each `session/prompt` starts fresh context.

7. **`session/set_mode` variant naming**: ACP v2 draft standardizes on `snake_case` method names. Roko already handles `session/set_mode` but the v2 spec also consolidates `session/set_config_option` (already aliased in handler). No v2-specific negotiation exists yet.

## Cursor-Specific Issues

### What works

- **Config documented**: `00-ACP-FEATURES.md` includes the exact `agent.customAgents` JSON block.
- **Dual method support**: handler.rs line 385 accepts both `session/config/update` and `session/set_config_option`, covering both Cursor's method naming and the canonical ACP name.
- **`configId` alias**: `ConfigUpdateParams.option_id` has `#[serde(alias = "configId")]` for Cursor's field naming convention.

### Known gaps

1. **Team-level MCP**: Cursor dashboard team-level MCP servers are not supported in ACP mode (Cursor limitation, not roko).

2. **Authentication flow**: Cursor forum reports "unauthenticated" errors with custom ACP agents in JetBrains (Cursor agent + JetBrains IDE). Roko returns `auth_methods: []` (no auth required), which may conflict with Cursor's expectations when it acts as an agent in other editors.

3. **No Cursor-specific integration tests**: all protocol conformance tests use the generic `TestHarness` without simulating Cursor-specific wire format differences.

4. **`new_value` vs `value`**: `ConfigUpdateParams.new_value` -- Cursor may send `"value"` instead. No alias exists for this field (unlike `configId` / `option_id` which is aliased).

## JetBrains-Specific Issues

### What works

- Listed in `lib.rs` module doc as a supported editor.
- ACP protocol itself is editor-agnostic; JetBrains uses the same stdio JSON-RPC.

### Known gaps

1. **No `acp.json` generation**: JetBrains expects an `acp.json` file for custom agent configuration. Roko does not generate this. Users must manually create it with the correct command path and args.

2. **Full path requirement**: JetBrains docs emphasize using the "full path to the agent executable" in the `command` parameter. The config examples in `00-ACP-FEATURES.md` use absolute paths, which is correct, but a `roko acp --emit-config jetbrains` helper would reduce user friction.

3. **No JetBrains-specific tests**: no tests simulate JetBrains wire format quirks or `acp.json`-based configuration.

4. **IDE Services / JetBrains Central**: organizations using JetBrains IDE Services may restrict custom agents. No guidance in roko docs for enterprise IT setup.

5. **Logging integration**: JetBrains docs recommend collecting logs for troubleshooting. Roko logs to `.roko/acp.log` (file-based via `tracing_appender`), but the log path may not be obvious to JetBrains users. A startup message or `acp.json` metadata field could help.

## Common ACP Integration Problems

Based on web research and the ACP ecosystem:

1. **Stdout pollution**: any output to stdout that is not valid JSON-RPC corrupts the transport. Roko's `run_acp_server` correctly uses file-based logging (`tracing_appender`) and only writes JSON-RPC to stdout. However, dependencies or `eprintln!` calls in linked crates could leak. The codebase-wide `eprintln!` -> `tracing` migration (batch 2026-08-12) mitigates this.

2. **Shell initialization output**: when editors use the user's shell to spawn the agent, shell init scripts (`.zshrc`, `.bashrc`) may print banners or warnings to stdout. Roko has no "silent mode" flag to suppress this at the shell level.

3. **Process lifecycle**: editors expect the ACP process to stay alive and respond indefinitely. If `roko-cli acp` panics or hits an unhandled error, the editor shows "server shut down unexpectedly" with no useful detail. Roko's error wrapping in `handler.rs` (lines 30-48) provides a JSON-RPC error on startup failure, but runtime panics in async tasks may not be caught.

4. **ACP v2 migration**: the v2 draft (July 2026) introduces breaking changes including new method naming conventions and extensible enum variants. Roko implements v1 (protocol version 1, spec 0.12.2). No v2 negotiation or feature detection exists yet. Editors adopting v2 will need a server-side migration.

5. **Windows/WSL compatibility**: Zed issues [#47340](https://github.com/zed-industries/zed/issues/47340) and [#48754](https://github.com/zed-industries/zed/issues/48754) report that external agents do not work on Windows or WSL. Roko is macOS/Linux-focused and has no Windows CI or testing.

6. **Config file resolution**: the 4-layer config resolution (global -> project -> env -> default) works well but is opaque to users. The `configSources` field in `InitializeResult` (always serialized, even empty) helps, and `configWarnings` surfaces parse errors. However, there is no editor UI to browse or edit config files from within the agent panel.

## Test Gaps for Editor Compatibility

- [ ] **Zed wire format regression tests**: only one test (`permission_response_round_trip`) verifies Zed's actual wire format. Need tests for Zed's `session/new`, `session/prompt`, and `session/config/update` payloads as actually sent by current Zed stable.
- [ ] **Cursor wire format tests**: no tests simulate Cursor's `agent.customAgents` launch sequence or verify Cursor-specific field aliases (`configId`, potential `value` vs `new_value`).
- [ ] **JetBrains launch sequence**: no test simulates `acp.json`-based launch or JetBrains' specific `initialize` payload shape.
- [ ] **Neovim/Terminal integration test**: no test verifies the agent works when spawned from a terminal emulator or Neovim plugin.
- [ ] **Multi-turn conversation test**: no integration test verifies that sending multiple `session/prompt` requests to the same session produces coherent multi-turn context. The current test sends one prompt per session.
- [ ] **Concurrent session stress test**: no test verifies behavior when an editor opens multiple sessions simultaneously (common in JetBrains with multiple projects open).
- [ ] **File change notification test**: no test for `file_change` session updates (feature not implemented).
- [ ] **MCP passthrough test**: no integration test verifies that `mcpServers` from `session/new` are forwarded to the agent dispatch.
- [ ] **Config hot-reload propagation test**: `ConfigWatcher` tests exist but no integration test verifies that editing `roko.toml` while a session is active causes the editor to see updated config options.
- [ ] **Stderr isolation test**: no test verifies that non-JSON output (warnings, panics) does not leak to stdout and corrupt the JSON-RPC stream.
- [ ] **Session persistence across restart**: `session/load` and `session/resume` exist but no test verifies that a session created in one ACP process can be loaded in another (editor restart scenario).
- [ ] **`ContentBlock::Unknown` forward-compat test**: the `#[serde(other)]` catch-all exists but is only indirectly tested via the audio block test. Need explicit tests for future content block types that editors might send.
- [ ] **ACP v2 negotiation test**: no test for protocol version 2 negotiation or graceful fallback when an editor sends v2-only messages.
