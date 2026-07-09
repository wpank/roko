# Architecture Context

The `tmp/architecture/` directory (21 files) contains the **current design** for the roko system. These supplement the unified spec (`tmp/unified/`) with implementation-level detail.

**Precedence**: `tmp/unified/` > `tmp/architecture/` > `docs/`

## Architecture Files Index

| File | Topic | Key Concepts |
|---|---|---|
| `tmp/architecture/00-INDEX.md` | Overview + reading order | 12 primitives, dependency graph |
| `tmp/architecture/01-overview.md` | Deployment architecture | 3 tiers: Backbone, Workspace, Remote |
| `tmp/architecture/02-agent-runtime.md` | Agent runtime pipeline | 9-step heartbeat, T0/T1/T2 gating, 3 agent modes, cortical state |
| `tmp/architecture/03-extensions.md` | Extension system | 8 layers, 22 hooks, Connector primitive, Pi compatibility |
| `tmp/architecture/04-gates.md` | Gate/Verify architecture | Pre-action + post-action gates, rung pipeline |
| `tmp/architecture/05-learning.md` | Learning loops | Episodes, playbooks, calibration, cascade routing |
| `tmp/architecture/06-composition.md` | Context assembly | VCG auction, section effects, token budget |
| `tmp/architecture/07-dreams.md` | Dream consolidation | NREM replay, REM imagination, integration |
| `tmp/architecture/08-safety.md` | Safety model | CaMeL IFC, 5-head corrigibility, threat model |
| `tmp/architecture/09-knowledge.md` | Knowledge system | InsightStore, 6 kinds, HDC, Ebbinghaus decay, on-chain |
| `tmp/architecture/10-coordination.md` | Multi-agent coordination | Pheromone/Signal types, relay, stigmergy |
| `tmp/architecture/11-marketplace.md` | Economy | Cell manifests, registry, publish/install |
| `tmp/architecture/12-surfaces.md` | UI architecture | TUI, HTTP, WebSocket, 5 named surfaces |
| `tmp/architecture/13-deployment.md` | Deployment | Docker, Railway, Fly, daemon |
| `tmp/architecture/14-config.md` | Configuration | 60+ params, profiles, schema validation |
| `tmp/architecture/15-testing.md` | Test strategy | 5 layers, property-based, adversarial |
| `tmp/architecture/16-observability.md` | Telemetry | Lenses, StateHub, metrics |
| `tmp/architecture/17-chain.md` | On-chain registries | Solidity contracts, testnet deploy |
| `tmp/architecture/18-arenas.md` | Arena system | Evals, bounties, 7-step flywheel |
| `tmp/architecture/19-visual-composition.md` | Visual composition | Diagram conventions |
| `tmp/architecture/20-roadmap.md` | Implementation roadmap | Phases, critical path |
| `tmp/architecture/21-tui-and-operations.md` | TUI operations | Dashboard, keybindings |

## 12 Primitives (from architecture, maps to unified)

The architecture defines 12 primitives. Here's how they map to unified vocabulary:

| # | Architecture Primitive | Unified Equivalent | Notes |
|---|---|---|---|
| 1 | Agent | Agent (specialization) | Domain merged as ArchetypeManifest field |
| 2 | Extension | Extension (specialization) | 3 tiers: Pi-compatible, Roko-enhanced, Roko-native |
| 3 | **Connector** | Connector (specialization) | NEW — wraps external I/O |
| 4 | Gate | Verify (protocol) | Expanded: pre-action + post-action |
| 5 | **Feed** | Trigger + Connect | Continuous data streams |
| 6 | **Recipe** | Graph (Flow specialization) | Data transformation pipelines |
| 7 | Knowledge Entry | Signal (Kind::Insight/Heuristic/...) | 6 sub-kinds |
| 8 | Arena | *(Phase 3)* | Competitive evaluation |
| 9 | Eval | *(Phase 3)* | Scoring protocol |
| 10 | Signal | Signal + Pulse | Architecture "Signal" = product-layer name for Pheromone |
| 11 | Group | *(Phase 3)* | Multi-agent coordination |
| 12 | Bounty | *(Phase 3)* | Task marketplace |

**Important**: Architecture's "Signal" refers to the product/UI name for what the backend calls "Pheromone" (stigmergic coordination). Unified spec's "Signal" is broader — it's the universal durable datum (what was called Engram). Don't confuse them. In the migration, unified wins: Signal = durable datum.

## Key Architecture Decisions to Preserve

1. **9-step heartbeat** is canonical agent loop (OBSERVE → ... → REFLECT)
2. **T0 reflexes** use `.roko/learn/reflexes.jsonl` — condition-action pairs learned from T2 successes
3. **Three agent modes**: Ephemeral (one task), Persistent (forever), Reactive (sleep until trigger)
4. **Cortical state** persists to `.roko/agents/{id}/cortical.json` on theta tick; resume if < 1 hour old
5. **Extension 8 layers**: Foundation → Perception → Memory → Cognition → Action → Social → Meta → Recovery
6. **Knowledge 6 kinds**: Insight, Heuristic, Warning, CausalLink, StrategyFragment, AntiKnowledge
7. **Ebbinghaus decay** with kind-specific half-lives (Warning: 1h, Heuristic: 90d, etc.)
