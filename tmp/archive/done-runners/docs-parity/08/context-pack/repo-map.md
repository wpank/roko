# Repo Map — Batch 08

Quick reference for the parity refresh.

## Workspace Facts

- Rust LOC: `322,088`
- Workspace members: `36`
- Topic posture: chain is still Phase 2+ in product terms

## High-Value Paths

| Path | Why It Matters |
|---|---|
| `crates/roko-chain/src/` | minimal live chain primitives |
| `contracts/src/` | Solidity demo contract surface |
| `contracts/test/` | proof the demo contracts are exercised |
| `docs/08-chain/` | source docs being narrowed |
| `tmp/docs-parity/08/` | parity materials being refreshed |
| `tmp/refinements-audit/03-moat-audit.md` | audit direction for deferrals |

## Notes

- Keep shipped wording tied to the small `roko-chain` foundation plus demo contracts.
- Keep `contracts/src/` in demo/precursor language.
- Do not treat `docs/08-chain/` as evidence that the larger chain stack ships.
- WebSocket and SSE already exist elsewhere in the repo; they do not make the
  chain gossip design current.
