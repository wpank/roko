# Batch AR02: Relay binary

You are a fresh coding agent. Zero prior chat context. Read these first:

- `tmp/agent-registry/implementation-pack/context-pack/00-READ-FIRST.md`
- `tmp/agent-registry/implementation-pack/context-pack/01-TARGET-STATE.md`
- `tmp/agent-registry/02-relay-design.md`
- `tmp/agent-registry/implementation-pack/context-pack/02-CODE-MAP.md`

Also inspect:

- `crates/roko-agent-server/src/lib.rs`
- `crates/roko-agent-server/src/features/messaging.rs`
- `apps/mirage-rs/src/http_api/mod.rs`

## Task

Create the standalone relay binary at `apps/agent-relay/`.

The relay is responsible only for:

- outbound agent WS hello/connect
- live directory
- message forwarding
- relay-hosted Agent Cards
- optional dashboard WS event stream

## Suggested subagent split

- explorer: inspect current axum/tokio patterns in the repo and recommend a
  minimal relay shape
- worker A: relay state + routes + health endpoints
- worker B: agent WS protocol + message forwarding path
- worker C: tests for hello -> message -> response

## Write scope

- `apps/agent-relay/**`
- any minimal workspace manifest updates needed

## Constraints

1. Keep relay scope narrow. Do not drag in control-plane behavior.
2. Keep state in memory for MVP.
3. Match the relay concepts in `tmp/agent-registry/02-relay-design.md`.

## Acceptance criteria

- relay builds as its own binary
- `GET /relay/health` returns `ok`
- `GET /relay/agents` lists connected agents
- `GET /relay/cards/{id}` serves pushed cards
- message forwarding path works end-to-end against a test agent connection

## Verification

At minimum:

```bash
cargo check -p agent-relay --all-targets
cargo test -p agent-relay
```
