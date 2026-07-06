# agent-registry/ — ERC-8004 + Relay Refactor

**Directory**: `tmp/agent-registry/`
**Status**: DONE — 7/8 batches complete, 1 minor docs gap
**Files**: 7 docs (00-INDEX through 06-decisions-and-context)

## Batch Status

| Batch | Title | Status | Key Source Files |
|-------|-------|--------|------------------|
| AR01 | Contracts & mirage fork | DONE | `contracts/src/IdentityRegistry.sol`, `ReputationRegistry.sol`, `ValidationRegistry.sol` |
| AR02 | Relay binary | DONE | `apps/agent-relay/src/{lib,main,protocol,state}.rs` |
| AR03 | `roko agent serve` CLI | DONE | `crates/roko-cli/src/agent_serve.rs` |
| AR04 | Agent relay client + chain registration | DONE | `crates/roko-agent-server/src/features/relay_client.rs` |
| AR05 | Mirage proxy + Docker | DONE | `apps/mirage-rs/src/rpc.rs` (relay proxy routes) |
| AR06 | Demo UI + quickstart | DONE | `apps/mirage-rs/static/quickstart.sh`, JS modules |
| AR07 | Remote demo verification | PARTIAL | Verification gates passed; runbook not persisted |
| AR08 | Dashboard migration | DONE | `nunchi-dashboard/src/services/mirage-api.ts` (8004+relay discovery) |

## Critical Path — All 6 Items Verified

1. Mirage boots with ERC-8004 contracts
2. Relay accepts agent connections
3. `roko agent serve` exists
4. Agent appears via relay and/or 8004
5. Dashboard can message that agent
6. Works locally and on Railway

## Remaining Checklist

- [ ] Persist AR07 remote demo runbook to repo (low priority — verification gates passed, just needs documentation)

## Deferred Items (Intentional)

- Relay auth beyond MVP
- 1-click deploy UX redesign
- Full `roko-serve` deletion
- Non-stub reputation/validation registries

## Source Files

- **Architecture**: `tmp/agent-registry/01-architecture.md`
- **Relay design**: `tmp/agent-registry/02-relay-design.md`
- **Migration plan**: `tmp/agent-registry/03-migration-plan.md`
- **Deployment**: `tmp/agent-registry/04-deployment-and-dev.md`
- **Contracts**: `tmp/agent-registry/05-contracts-and-identity.md`
- **Decisions**: `tmp/agent-registry/06-decisions-and-context.md`
