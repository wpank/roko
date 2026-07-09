# Roko Architecture Specification

> Canonical architecture for the Nunchi agent platform.
> Split from `roko-architecture-redesign-v2.md` for maintainability.

## Document Map

| # | Document | Scope | Status |
|---|----------|-------|--------|
| 01 | [Overview and Problem](01-overview.md) | System diagram, deployment tiers, design principles | Ported from v2 |
| 02 | [Agent Runtime](02-agent-runtime.md) | AgentRuntime struct, 9-step pipeline, modes, timescales, T0/T1/T2 gating, adaptive clock algorithm, cortical state persistence, **acceptance criteria** | Ported from v2 + gaps filled + **AC added 2026-04-25** |
| 03 | [Extensions](03-extensions.md) | Extension trait, 8 layers, 22 hooks, loading/discovery, dependency resolution, **decision enum variants, hook timeout, AgentContext, connector discovery, acceptance criteria** | Ported from v2 + gaps filled + **spec clarifications 2026-04-25** |
| 04 | [Connectivity and Relay](04-connectivity.md) | In-process agents, remote agents, relay protocol, cross-user communication, message routing, relay scalability, disconnection recovery, reconnection | Ported from v2 + gaps filled |
| 05 | [Feeds and Data Streams](05-feeds.md) | Raw/derived/composite/meta feeds, ERC-8004 advertisement, feed registry, pagination, dashboard chain subscriptions | Ported from v2 + gaps filled |
| 06 | [Paid Feeds and MPP](06-paid-feeds.md) | x402, MPP sessions, payment gating, reputation pricing, feed marketplace, practical examples | Ported from v2 |
| 07 | [Inference Gateway](07-gateway.md) | 12 subsystems, pipeline, InferenceHandle, CascadeRouter, concurrency/backpressure, provider fallback, proxy for isolated agents | Ported from v2 + gaps filled |
| 08 | [Authentication](08-auth.md) | Privy, API keys, agent tokens (full lifecycle + revocation), wallet signatures, scopes, relay auth, JWKS caching | Ported from v2 + gaps filled |
| 09 | [Knowledge and Pheromones](09-knowledge.md) | InsightStore, knowledge publish/validate/challenge/decay, HDC embeddings, pheromone deposits, stigmergy, dream consolidation | NEW |
| 10 | [Groups and Coordination](10-groups.md) | Group identity, membership, coordination protocol, shared context, cluster pipelines | NEW |
| 11 | [Arenas, Evals, and Bounties](11-arenas.md) | Arena registry, task sources, scoring functions, leaderboards, eval registry, bounty market, clearing | NEW |
| 12 | [DeFi Infrastructure](12-defi.md) | ISFR oracle, yield perpetuals, cooperative clearing, multi-chain, bridge architecture | NEW |
| 13 | [Meta Layer](13-meta.md) | Meta-agents, generators, lineage tracking, recursive safety monitoring | NEW |
| 14 | [On-Chain Registries](14-registries.md) | ERC-8004 agent passport, reputation registry, knowledge registry, arena/eval/bounty contracts, event indexer | NEW |
| 15 | [Dashboard Architecture](15-dashboard.md) | Data layer, subscription manager, aggregation service, page-to-data mapping, adaptive density, epistemic aesthetics, performance targets | Ported from v2 + gaps filled |
| 16 | [Secrets and Configuration](16-config.md) | **Complete roko.toml schema reference** (30+ sections from schema.rs), secret management, load precedence, env expansion, config versions | **Rewritten 2026-04-25** from skeleton to full reference |
| 17 | [Deployment](17-deployment.md) | Railway, Fly, local dev, agent creation UX, scaling tiers | Ported from v2 |
| 18 | [Implementation Roadmap](18-roadmap.md) | Phases 1-10, dependencies, crate mapping, migration from v1, **bardo naming map, 48-task summary, dependency graph, critical path** | Updated from v2 + integration plan folded |
| 19 | [Visual Composition and Authoring](19-visual-composition.md) | Plan mutation protocol, conversation-as-plan-editor, template registry, extension compilation, gate testing, authoring API contracts, cost projection | NEW |
| 20 | [Orchestrator and Learning Gaps](20-orchestrator-gaps.md) | Structured reviews, compile error classification, error pattern sharing, post-gate reflection, context scoping, warm spawning, 10 conductor watchers, neuro→cascade router, episode clustering, A-MAC admission, **current state reconciliation, 12 spec clarifications** | Folded from integration plan + **updated 2026-04-24** |
| 21 | [TUI and Operations](21-tui-and-operations.md) | DaimonState visualization, heartbeat status view, knowledge browser, justfile, E2E test harness, self-healing supervisor, **conductor watcher config, implementation state table** | Folded from integration plan + **updated 2026-04-25** |

## Primitive vocabulary

The canonical primitive vocabulary is **12 primitives** (per dashboard PRD 23, superseding the 10-primitive vocabulary in PRD 20):

| # | Primitive | Status vs PRD 20 |
|---|-----------|------------------|
| 1 | Agent | Restructured -- Domain merged in as `ArchetypeManifest` field |
| 2 | Extension | Kept (three-tier: Pi-compatible, Roko-enhanced, Roko-native) |
| 3 | **Connector** | NEW -- external system I/O (VenueAdapter, ChainRpc, MCP, databases) |
| 4 | Gate | Expanded (pre-action + post-action) |
| 5 | **Feed** | NEW -- continuous data streams (price feeds, event watchers, CI status) |
| 6 | **Recipe** | NEW -- data transformation pipelines (indicator chains, P&L attribution, HDC encoding) |
| 7 | Knowledge Entry | Kept |
| 8 | Arena | Kept |
| 9 | Eval | Kept |
| 10 | Signal | Renamed from Pheromone at product layer; backend keeps internal pheromone naming |
| 11 | Group | Kept |
| 12 | Bounty | Kept |

Each primitive has a defined shape (verb set), Rust trait mapping, and dashboard authoring surface. See PRD 23 for the full composition matrix and DeFi struct mapping.

## Reading order

For a full understanding, read in order: 01 -> 02 -> 03 -> 04 -> 05 -> 07 -> 08 -> 09 -> 14 -> 15.
For implementation priority, start with 18 (roadmap) then read the phase-relevant docs.

## Source references

- Original v2 monolith: `tmp/roko-architecture-redesign-v2.md`
- ~~Bardo integration plan: `tmp/bardo-integration-plan.md`~~ -> **Folded into docs 18, 20, 21** (original file retained for reference but all content is now in this doc set)
- Dashboard PRDs: `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/`
- **Dashboard PRD 23 (universal primitives):** `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/23-universal-primitives.md` -- defines the 12-primitive vocabulary, composition matrix, DeFi struct mapping, and authoring surfaces
- **Architecture cross-reference:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/plan/architecture-cross-reference.md` -- maps all 22 architecture docs to dashboard needs, identifies conflicts, lists ~160 new endpoints
- **Dashboard-roko integration:** `/Users/will/dev/nunchi/nunchi-dashboard/docs/plan/dashboard-roko-integration.md` -- three-tier deployment model, interaction modes, API contracts
- DeFi gap analysis: `tmp/defi/gap/`
- PRDs (roko): `tmp/04-21-26/PRDs/`
