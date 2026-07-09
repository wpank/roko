# Batch AR03: `roko agent serve`

You are a fresh coding agent. Zero prior chat context. Read these first:

- `tmp/agent-registry/implementation-pack/context-pack/00-READ-FIRST.md`
- `tmp/agent-registry/implementation-pack/context-pack/01-TARGET-STATE.md`
- `tmp/agent-registry/implementation-pack/context-pack/02-CODE-MAP.md`

Also inspect:

- `crates/roko-agent-server/src/lib.rs`
- `crates/roko-agent-server/src/state.rs`
- `crates/roko-cli/src/main.rs`
- `crates/roko-cli/tests/smoke.rs`

## Task

Add a production CLI entrypoint that starts `roko-agent-server`.

Target command:

```bash
roko agent serve --agent-id demo-1
```

The command should be able to run an agent server with the existing feature
surfaces and be structured so later batches can add relay and chain hooks.

## Suggested subagent split

- explorer: inspect current CLI command tree and existing tests that already
  exercise `AgentServer::builder()`
- worker A: CLI command parsing and command wiring
- worker B: runtime/config setup for dispatcher/backend/store integration
- worker C: smoke/integration tests

## Write scope

- `crates/roko-cli/src/main.rs`
- `crates/roko-cli/src/agent_serve.rs` new file if helpful
- related CLI tests

## Constraints

1. Do not route this through `roko-serve`.
2. Keep the command centered on `roko-agent-server`.
3. Expose the flags that later batches need:
   - `--agent-id`
   - `--bind`
   - `--relay-url`
   - optional chain/passport/wallet flags

## Acceptance criteria

- command exists and parses
- command can start an agent server that answers `/health`
- implementation is structured for later relay and chain registration hookup

## Verification

At minimum:

```bash
cargo check -p roko-cli
cargo test -p roko-cli --test agent_serve
```
