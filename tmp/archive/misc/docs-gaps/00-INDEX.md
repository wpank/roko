# Architecture Compliance -- Gap Index

The docs are the specification. This directory catalogs where the codebase does not yet match the spec, organized as checklists with verification criteria.

## docs/00-architecture (Kernel)

| File | Scope | Open Items | Priority |
|---|---|---|---|
| `01-type-corrections.md` | Existing types/fields that differ from spec | 9 | P0 |
| `02-missing-kernel-types.md` | Core kernel types specified but not built | 7 | P0 |
| `03-trait-migrations.md` | Trait signatures behind target spec | 6 | P1 |
| `04-config-schema.md` | Config sections specified but absent | 7 | P1-P2 |
| `05-integrations.md` | Documented cross-system wiring not connected | 21 | P1-P3 |
| `06-naming-fixes.md` | Code vocabulary that violates doc terminology | 6 | P0-P1 |
| `07-advanced-systems.md` | Advanced capability types from docs 25-29 | 11 | P2 |
| `08-infrastructure.md` | Test/CI/benchmark gaps from docs 31-32 | 5 | P0-P1 |

## docs/01-orchestration through docs/10-dreams (Subsystems)

| File | Docs Dir | Scope | Open Items | Priority |
|---|---|---|---|---|
| `10-orchestration.md` | 01-orchestration | DAG, executor, snapshots, cross-domain | 11 | P1-P2 |
| `11-agents.md` | 02-agents | Safety, types, providers, roles, temperament | 10 | P0-P2 |
| `12-composition.md` | 03-composition | VCG auction, MVT foraging, HDC dedup, layers | 10 | P0-P2 |
| `13-verification.md` | 04-verification | SPC, PRM, evaluation lifecycle, EvoSkills | 9 | P1-P2 |
| `14-learning.md` | 05-learning | Demurrage, bandits, calibration, Bus loops | 13 | P1-P2 |
| `15-neuro.md` | 06-neuro | Tiers, HDC encoding, cross-domain, backup | 12 | P1-P2 |
| `16-conductor.md` | 07-conductor | Bandit, signals, pressure, federation | 9 | P1-P2 |
| `17-chain.md` | 08-chain | Contracts, gossip, precompile, payments | 12 | P2-P3 |
| `18-daimon.md` | 09-daimon | Tier routing, fatigue, contagion, extraction | 9 | P0-P2 |
| `19-dreams.md` | 10-dreams | Staging, Mattar-Daw, Pearl SCM, rendering | 14 | P1-P2 |

## docs/11-safety through docs/19-deployment (Cross-Cuts & Infrastructure)

| File | Docs Dir | Scope | Open Items | Priority |
|---|---|---|---|---|
| `20-safety.md` | 11-safety | SafetyLayer bypass, custody, taint, LTL, witness DAG | 13 | P0-P2 |
| `21-interfaces.md` | 12-interfaces | Scaffolders, plugins, TUI screens, projections | 16 | P1-P2 |
| `22-coordination.md` | 13-coordination | Agent Mesh, morphogenetic, pheromone enrichment | 8 | P1-P2 |
| `23-identity-economy.md` | 14-identity-economy | EMA reputation, auctions, LMSR, x402 | 9 | P2 |
| `24-code-intelligence.md` | 15-code-intelligence | Search API, MCP server, SQLite, compose integration | 9 | P0-P2 |
| `25-heartbeat.md` | 16-heartbeat | Theta/Delta loops, adaptive clock, T0 probes, POMDP, VCG | 11 | P1-P2 |
| `26-lifecycle.md` | 17-lifecycle | Agent creation CLI, provisioning, budget, backup, demurrage | 10 | P1-P2 |
| `27-tools.md` | 18-tools | MCP servers, safety hooks, profiles, plugin SDK, templates | 10 | P1-P2 |
| `28-deployment.md` | 19-deployment | Release pipeline, Docker, daemon, secrets, observability | 11 | P1-P2 |

## docs/20-technical-analysis and docs/21-references

| File | Docs Dir | Scope | Open Items | Priority |
|---|---|---|---|---|
| `29-technical-analysis.md` | 20-technical-analysis | Oracle impls, HDC codebooks, causal discovery, sheaf/tropical geometry | 15 | P1-P2 |

**Note**: `docs/21-references/` is a bibliography (academic papers, standards, prior art). No code gaps -- reference-only material.

## Aggregate counts

| Priority | Items | Description |
|---|---|---|
| P0 | ~27 | Code contradicts spec -- fix first |
| P1 | ~100 | Spec expects code that doesn't exist -- needed for self-hosting |
| P2 | ~150 | Advanced/roadmap features |
| P3 | ~16 | Nice-to-have, low urgency |
| **Total** | **~293** | |

## Priority guide

- **P0**: Code actively contradicts the spec -- fix before building new things
- **P1**: Spec describes something the code should have but doesn't -- needed for self-hosting
- **P2**: Advanced capabilities -- roadmap items, build after P0-P1 are clean
- **P3**: Nice-to-have integrations -- low urgency

## How to use

**For P0 fixes** (code is wrong today):
1. `01-type-corrections.md` -- Decay::Ttl, Taint enum, duplicate types
2. `06-naming-fixes.md` -- signals.jsonl rename, heartbeat duplicates
3. `08-infrastructure.md` -- doc-internal inconsistencies
4. `11-agents.md` AGT-01 -- SafetyLayer not on primary path
5. `12-composition.md` COMP-01 -- doc says 7 layers, code has 9
6. `18-daimon.md` DAIM-01 -- tier routing defined but not consumed
7. `20-safety.md` SAFE-01/02/03 -- SafetyLayer bypass, custody not persisted, taint not enforced
8. `24-code-intelligence.md` CODE-01/02 -- search API not callable, compose not integrated

**For kernel evolution** (Pulse/Bus/Datum chain):
1. `02-missing-kernel-types.md` -- Topic, Pulse, Bus, Datum, TopicFilter, PolicyOutputs, Probe reconciliation
2. `03-trait-migrations.md` -- Scorer, Gate, Router (additive), Composer, Policy (breaking). Note: TM-06 blocks SAFE-11.

**For subsystem gaps** (per-crate work):
- Files 10-19, each scoped to one doc directory and its corresponding crate(s)
- Files 20-29, each scoped to one doc directory and its corresponding crate(s)

**For Phase 2+ roadmap**:
- `07-advanced-systems.md` -- attention, immune, temporal, goals, energy
- `17-chain.md` -- all chain/Korai items (Tier 6 deferred)
- `23-identity-economy.md` -- all identity/economy items (Tier 6 deferred)
- `25-heartbeat.md` -- Theta/Delta loops, adaptive clock, POMDP, VCG (Phase 2+)
- `29-technical-analysis.md` -- oracle impls, HDC codebooks, causal discovery, sheaf/tropical (Phase 2+ research)

## Acceptance standard

An item is resolved when:
1. The code matches the doc spec
2. The verification command in the checklist passes
3. Existing tests still pass (`cargo test --workspace`)
