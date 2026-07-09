# ACP Runner Rules (read first)

## Core rules

1. **No prior chat** — This prompt is self-sufficient. Do not reference external conversations.
2. **Scope locked** — Only modify files under `crates/roko-acp/` unless explicitly told otherwise (ACP07 may touch `crates/roko-cli/src/main.rs`).
3. **Repo reality** — Use `rg` or `grep` to verify current state before editing. Never assume file contents.
4. **No external SDK deps** — All ACP protocol types are defined inline in `roko-acp/src/types.rs`. Do NOT add dependencies on external ACP/JSON-RPC SDK crates.
5. **stdout = protocol channel** — All logging MUST go to files or stderr. Any non-JSON output on stdout corrupts the protocol stream. Use `tracing` with a file appender.
6. **Commit message format** — `acp(ACPnn): <batch title>` (e.g., `acp(ACP01): Scaffold roko-acp crate + workspace wire`).
7. **Subagents OK** — Spawn workers with disjoint write scopes when beneficial.
8. **Substantive only** — No placeholder `todo!()` macros or `unimplemented!()` in public APIs unless the batch explicitly says to stub. Every function must have a real implementation or a clear `// Stubbed — wired in batch ACPnn` comment.
9. **No destructive git** — The runner handles branch lifecycle. Do not create branches, commit, or push.
10. **Reuse existing patterns** — Roko has established patterns for Substrate, ProcessSupervisor, StateHub, CostLens, etc. Wire into them, don't reinvent.

## Rust conventions

- `#[derive(Debug, Clone, Serialize, Deserialize)]` on all types
- `#[serde(rename_all = "camelCase")]` for ACP protocol types (JSON uses camelCase)
- `#[serde(tag = "sessionUpdate")]` for discriminated unions
- Use `thiserror` for error types
- Use `tokio` for async runtime
- Use `tracing` for structured logging
- All public items need doc comments (`///`)

## Dependency rules

- Allowed deps: tokio, serde, serde_json, tracing, tracing-subscriber (with file appender), uuid, chrono, thiserror, anyhow
- Allowed workspace deps: roko-core, roko-agent, roko-orchestrator, roko-compose, roko-gate, roko-fs, roko-runtime, roko-conductor, roko-learn, roko-neuro, roko-daimon, roko-primitives
- Do NOT add: any external JSON-RPC crate, any ACP SDK crate, tower, hyper, axum (this is stdio, not HTTP)

## File organization

```
crates/roko-acp/
├── Cargo.toml
└── src/
    ├── lib.rs              # Module declarations + re-exports
    ├── types.rs            # All ACP protocol types (JSON-RPC, sessions, updates)
    ├── transport.rs        # Stdio transport (read/write JSON-RPC messages)
    ├── handler.rs          # Main dispatch loop (method → handler)
    ├── session.rs          # Session state management
    ├── config.rs           # AcpConfig struct
    ├── config_options.rs   # 7 session config options
    ├── commands.rs         # 8 slash commands
    ├── elicitation.rs      # Structured form dialogs
    ├── permissions.rs      # Permission request/response bridge
    ├── bridge_fs.rs        # File system bridge (editor-mediated)
    ├── bridge_terminal.rs  # Terminal bridge (editor-mediated)
    ├── bridge_events.rs    # Cognitive event → session/update streaming
    ├── bridge_plan.rs      # Plan phase → plan notifications
    ├── bridge_gates.rs     # Gate results → tool call cards
    └── bridge_usage.rs     # Token/cost → usage notifications
```
