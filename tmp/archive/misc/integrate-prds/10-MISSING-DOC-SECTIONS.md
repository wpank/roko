# Previously Missing Doc Sections

The initial survey covered sections 00-02, 05-06, 09-11, 13, 16. These additional sections were found in `/docs/`:

---

## Section 03: Composition (L2 Scaffold)
**Path**: `docs/03-composition/` (14 sub-docs)
**Status**: 7 fully implemented, 4 scaffold, 3 design-only

Key innovations NOT previously captured:
- **Active Inference context selection** (EFE formula, softmax selection)
- **Predictive Foraging** (MVT stopping rule for context assembly)
- **VCG Attention Auction** (8 bidding subsystems for context budget)
- **13-step enrichment pipeline** (conventions, symbols, prior outputs, research, etc.)
- **Affect-modulated retrieval** (Daimon biases context selection)

## Section 04: Verification (L3 Harness)
**Path**: `docs/04-verification/` (13 sub-docs)
**Status**: Gate pipeline wired, innovations scaffold/design

Key innovations NOT previously captured:
- **EvoSkills** — Three-tier skill learning hierarchy with adversarial surrogate verification
- **Forensic AI / Causal Replay** — Content-addressed causal chains for regulatory compliance (EU AI Act, MiFID II, HIPAA)
- **Autonomous test generation** — Verification pipeline using cheap-model convergence
- **Process Reward Models** — Promise + Progress scoring per agent turn
- **14 feedback loops** across 5 speed tiers
- **4-phase evaluation lifecycle** (pre-execution, in-flight, post-execution, retrospective)

## Section 07: Conductor (Cybernetic Regulator)
**Path**: `docs/07-conductor/` (16 sub-docs)
**Status**: 10 watchers implemented, theory comprehensive

Key details NOT previously captured:
- **OODA cybernetic loop** (Observe-Orient-Decide-Act) as conductor design pattern
- **Good Regulator Theorem** implementation (Conant-Ashby 1970)
- **Yerkes-Dodson pressure dynamics** — optimal arousal for performance
- **21 production failure scenarios** mapped to specific responses
- **Conductor learning federation** — learned intervention policies shared across runs

## Section 08: Chain (Korai EVM Layer)
**Path**: `docs/08-chain/` (25 sub-docs, ~75K words)
**Status**: Tier 6 (deferred), 76 implementation items

Key specifications:
- Korai EVM chain (400ms blocks, 2s finality)
- **HDC precompile** (10,240-bit BSC vectors at ~400 gas)
- **Korai Passport** (ERC-721 soulbound identity, 4 tiers)
- **4-tier gossip** (GossipSub → MiroFish → FABRIC → Canonical)
- **8 gossip topics** (knowledge, reputation, job, heartbeat, anomaly, simulation, governance, peer-discovery)
- **mirage-rs EVM simulator** (141 tests, fork mode) — already built
- **x402 micropayments** (HTTP 402 Payment Required flow)
- **ISFR clearing/settlement** (QP solver, KKT certificates)
- **Valhalla privacy** (4 tiers: Public → Confidential → ZK)

## Section 12: Interfaces
**Path**: `docs/12-interfaces/` (19 sub-docs)
**Status**: Spec only, TUI scaffold exists

Key specifications:
- **ROSEDUST design language** — void-black + rose, glass morphism, motion system
- **29-screen TUI dashboard** (inventory across 6 navigation regions)
- **Spectre creature system** — procedurally generated from agent hash, dot-cloud geometry, 6 behavioral state animations
- **A2UI protocol** — agents emit JSONL UI descriptions (12 component types)
- **Web Portal** — React 19 + Next.js 15.5 stack, 9 pages, WebGL Spectre
- **Sonification (reframed)** — 5 musical layers, 8 behavioral state presets
- **CLI scaffolders** — 9 types (domain, gate, scorer, router, policy, substrate, probe, event-source, template)
- **Progressive help** — `roko explain` 3-level system, TeachingError wrapper

## Section 14: Identity & Economy
**Path**: `docs/14-identity-economy/` (16 sub-docs)
**Status**: Tier 5-6 (not started)

Key specifications:
- **ERC-8004 three registries** (Identity, Reputation, Validation)
- **7-domain EMA reputation** with Bayesian Beta foundation and discipline states
- **Knowledge Marketplace** (3-tier with alpha-decay pricing)
- **Commerce Bazaar** (dynamic pricing, service specialization)
- **MPP (Machine Payment Protocol)** with SPT budget delegation
- **KORAI tokenomics** (1% annual demurrage, Ostrom governance framework)
- **Knowledge Futures Market** (pre-sell knowledge before production)

## Section 15: Code Intelligence
**Path**: `docs/15-code-intelligence/` (11 sub-docs)
**Status**: Functional (in-memory), needs wiring

Current code:
- `roko-index` (4 modules, 1,151 lines, 30 tests)
- `roko-lang-rust` (819 lines), `roko-lang-typescript` (917 lines), `roko-lang-go` (600 lines)
- Parser, SymbolExtraction, DependencyGraph, PageRank, HDC fingerprints

Gaps:
- **No MCP server** — agents can't access code intelligence
- **No compose integration** — context assembly doesn't query symbol graph
- **No persistent storage** — in-memory only, lost on restart
- **Heuristic parsers only** — planned tree-sitter migration
- **Import edges only** — no Calls/Implements/Contains relationships

## Section 18: Tools
**Path**: `docs/18-tools/` (17 sub-docs)
**Status**: 16 built-in tools, extensive spec

Key details:
- **3 trust tiers** for tools: Read, Write, Privileged
- **7 safety hooks in chain**: PolicyCage → AllowlistGuard → SpendingLimiter → RateLimiter → RevmSimulator → HallucinationDetector → ResultFilter
- **423+ DeFi tools** (ONE domain plugin, Alloy + TypeScript sidecar)
- **13 chain domain profiles** (profile-specific tool gating)
- **18 agent templates** (6 collaboration, 5 knowledge-base, 7 roko-specific)

## Section 19: Deployment
**Path**: `docs/19-deployment/` (14 sub-docs)
**Status**: Design, Tier 3H

Covers: Native, WASM, Docker, launchd, systemd, Fly.io, edge, daemon, multi-repo, secrets.

## Section 20: Technical Analysis
**Path**: `docs/20-technical-analysis/` (15 sub-docs, 76 citations)
**Status**: Frontier research

Key concept: **Universal Oracle trait** — generalized prediction interface across domains:
- Chain oracles (MA, RSI, Bollinger, MEV detection)
- Coding oracles (build prediction, test failure probability, complexity drift)
- Research oracles (source reliability, contradiction detection)
- Hyperdimensional TA (HDC pattern algebra)

## Section 21: References
**Path**: `docs/21-references/` (25 domain sub-docs)
260+ unique citations across 25 research domains.

---

## Impact on Refactoring Plan

These sections add the following to the scope:

| Category | New Items | Phase |
|----------|-----------|-------|
| Composition innovations (EFE, VCG, foraging) | 3 algorithms | B (structural) |
| Verification innovations (EvoSkills, forensic, auto-test) | 3 features | C (new) |
| Code intelligence MCP server | 1 integration | B (wiring) |
| TUI/Interfaces (ROSEDUST, Spectre, 29 screens) | Major feature | C (new) |
| Conductor learning federation | 1 integration | B (wiring) |
| Oracle trait | 1 new trait | C (new) |
| Chain layer (76 items) | Deferred | Tier 6 |
| Identity/Economy (16 items) | Deferred | Tier 5-6 |
