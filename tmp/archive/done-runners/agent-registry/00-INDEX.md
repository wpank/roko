# Agent Discovery, Relay, and Registry

These notes replace the literal fix proposed in
[GitHub Issue #15](https://github.com/Nunchi-trade/roko/issues/15).
That issue correctly identifies the UX problem, but it proposes solving it in
the wrong layer by adding an `endpoint` field to mirage's in-process Rust
`AgentEntry`.

## Vision in one sentence

Use ERC-8004 as the durable source of agent identity, use a small relay as the
durable source of live reachability, and make `mirage-rs + agent-relay`
the default way to run and test the network locally and on Railway.

## Start here

1. [01-architecture.md](01-architecture.md) — core protocol shape and default deployment shape
2. [03-migration-plan.md](03-migration-plan.md) — implementation order and verification gates
3. [06-decisions-and-context.md](06-decisions-and-context.md) — decisions, scenarios, and file map

## Documents

| # | File | Contents |
|---|---|---|
| 01 | [01-architecture.md](01-architecture.md) | Protocol architecture, default runtime shape, why Issue #15 is the wrong abstraction |
| 02 | [02-relay-design.md](02-relay-design.md) | Relay responsibilities, protocol, auth posture, and deployment modes |
| 03 | [03-migration-plan.md](03-migration-plan.md) | Phase order, critical path, and local/remote verification flows |
| 04 | [04-deployment-and-dev.md](04-deployment-and-dev.md) | Local dev, Docker, Railway, Kauri dashboard wiring, and demo workflows |
| 05 | [05-contracts-and-identity.md](05-contracts-and-identity.md) | ERC-8004 target contract surface, Agent Card schema, wallet-optional policy |
| 06 | [06-decisions-and-context.md](06-decisions-and-context.md) | Firm decisions, deferred items, scenarios, and relevant code paths |
