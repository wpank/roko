# ISFR Implementation Plans

Implementation plans for integrating ISFR into roko as a keeper agent, upgrading the relay to support feeds, and adding chain-specific tools.

## Documents

| Doc | What |
|-----|------|
| [01-relay-feeds.md](01-relay-feeds.md) | Upgrade agent-relay with topic pub/sub, ring buffer, chain watcher |
| [02-feed-trait.md](02-feed-trait.md) | Feed trait design — how ISFRFeed integrates with taskrunner Wave 4 (097-100) |
| [03-isfr-keeper.md](03-isfr-keeper.md) | ISFR keeper agent — ISFRSource trait, rate fetching, relay integration |
| [04-chain-tools.md](04-chain-tools.md) | Chain-specific ISFR tools following roko-chain/src/tools.rs patterns |
| [05-contracts-deployment.md](05-contracts-deployment.md) | Contract deployment — chain profiles, bootstrapping, ABI generation |
| [06-integration.md](06-integration.md) | End-to-end wiring — how all pieces connect, startup sequence, config |

## Design Principles

1. **Additive to existing relay** — subscribe/publish frames added alongside existing request/response. Zero breaking changes.
2. **Feed trait alignment** — ISFRFeed implements the same `Feed` trait from taskrunner task 097. Not a special case.
3. **ISFRSource as extension point** — trait-based rate sources, easy to add new ones (Aave, Compound, Ethena, staking).
4. **Tool pattern consistency** — ISFR tools follow the same `ToolDef` + `LazyLock` pattern as existing chain domain tools.
5. **Language-agnostic relay** — any WebSocket client can subscribe to ISFR feed topics. Python keepers work alongside Rust agents.
6. **Chain-agnostic** — everything parameterized by `ChainProfile`. Mirage-rs is the default dev profile; daeji, mainnet, or any EVM chain with ERC-8004/8183/ISFR contracts works by swapping the profile.
7. **Demo-ide contracts as canonical ABI source** — ISFROracle v3.0 (550 lines) and ISFRBountyPool (182 lines). ABIs are checked in; deployment is per-profile.

## Relationship to Existing Plans

- **IMPL-06-ISFR.md** (tmp/prds/) — The 7-phase, 12-16 week plan covering oracle through clearing engine. These plans are Phase 1 (oracle + relay) implemented practically, with mirage-rs as the initial chain profile.
- **Taskrunner Wave 4** (tasks 097-100) — Feed trait, CLI integration, graduation. ISFRFeed becomes a third concrete Feed implementation alongside FileWatchFeed and ProviderHealthFeed.
- **roko-chain/src/isfr.rs** — Existing IsfrConfig, ClearingPhase, weighted median, QP solver. The keeper agent uses these types; new tools expose them via the tool system.

## Build Order

```
Phase A: Relay upgrade (01-relay-feeds.md)
  └── bus.rs, protocol changes, chain watcher
  └── relay_client.rs subscribe/publish

Phase B: Feed trait + ISFRFeed (02-feed-trait.md)
  └── Depends on task 097 (Feed trait) landing first
  └── ISFRFeed as third Feed implementation

Phase C: ISFR keeper + tools (03-isfr-keeper.md, 04-chain-tools.md)
  └── ISFRSource trait + mock sources
  └── isfr.* tool definitions
  └── Depends on Phase A (relay pub/sub)

Phase D: Contracts + deployment (05-contracts-deployment.md)
  └── ABI generation from demo-ide
  └── ChainProfile abstraction + mirage bootstrap
  └── Can run in parallel with Phases A-C

Phase E: Integration (06-integration.md)
  └── Depends on all above
  └── Config, startup sequence, demo flow
```

Phases A and D can run in parallel. Phase B depends on task 097. Phase C depends on Phase A. Phase E depends on all.
