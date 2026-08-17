# agentchain-v2 — DEPRECATED

> **Archived**: 2026-08-15
> **Previous location**: `tmp/agentchain-v2/`

## Status

All bespoke chain verticals described in this directory are **deprecated**:

- **daeji** (02-daeji/) — Separate chain project (node/BFT/precompiles/consensus). Owned by daeji team in a separate repo. Design docs here are historical reference only — do not implement from these.
- **ISFR** (03-isfr/) — Internet Secured Funding Rate vertical. **Fully removed** from the roko runtime as of 2026-08-13. Code, routes, tools, and config all deleted. See `CLAUDE.md` item 17.
- **Kora/Korai** (referenced in 04-markets/) — Chain-native marketplace and financial products. Phase 2+ aspirational design. No implementation exists or is planned until daeji devnet is live.
- **01-roko/** — Roko agent runtime design docs from this era are superseded by the current `CLAUDE.md` and `docs/v2/` architecture spec.
- **04-markets/** — Agent marketplace design. The job marketplace (`roko job` CLI) exists independently; the chain-native marketplace described here is Phase 2+.

## What to use instead

| Topic | Current source |
|---|---|
| Roko architecture | `CLAUDE.md`, `docs/v2/` |
| Gap tracking | `.roko/GAPS.md` |
| Chain status | `.roko/GAPS.md` § "Chain Modules" |
| ISFR removal tracking | `CLAUDE.md` item 17 |
| daeji design (if needed) | `tmp/archive/08-15-26/agentchain-v2/02-daeji/` (read-only reference) |

## Rule

**Do not implement anything from these docs.** All chain/daeji/ISFR/Kora work is either removed, deferred to Phase 2+, or owned by a separate team. Focus roko development on the self-hosting workflow described in `CLAUDE.md`.
