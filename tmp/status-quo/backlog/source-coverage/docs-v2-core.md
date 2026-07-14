# docs/v2 Core Source Coverage

Source corpus: all Markdown files under `docs/v2/**`.

Plan: `tmp/status-quo/backlog/plans/DOC-v2-core/tasks.toml`

## Summary

- Source Markdown files covered: **34**
- Executable tasks authored: **10**
- Coverage rule: every source file below appears in this ledger and in at least one `[[task]].context.read_files` entry.
- Dependency rule: local DAG edges use `depends_on`; E01-E18 prerequisites use `depends_on_plan`.
- Validation rule: each task has real shell verification that greps this ledger for its relevant source docs and validates the plan directory.

## Task Groups

| Task | Domain | Source docs | Primary implementation targets |
|---|---|---|---|
| `DOCV2-T01` | Signal/Cell kernel | `00-INDEX`, `01-SIGNAL`, `02-CELL`, `ARCHITECTURE-GUIDE` | `roko-core` kernel modules |
| `DOCV2-T02` | Graph/execution/orchestrator | `03-GRAPH`, `04-EXECUTION`, `27-ORCHESTRATOR`, `ARCHITECTURE-GUIDE` | `roko-graph`, `roko-runtime`, `roko-cli/runner` |
| `DOCV2-T03` | Agent runtime | `05-AGENT`, `26-CROSS-CUTS`, `ARCHITECTURE-GUIDE` | `roko-core`, `roko-runtime`, `roko-daimon`, runner config |
| `DOCV2-T04` | Memory/learning/dreams | `06-MEMORY`, `07-LEARNING`, `26-CROSS-CUTS`, `ARCHITECTURE-GUIDE` | `roko-neuro`, `roko-learn`, `roko-dreams`, `roko-compose` |
| `DOCV2-T05` | Gateway/feeds/groups/connectivity | `08-GATEWAY`, `09-FEEDS`, `10-GROUPS`, `11-CONNECTIVITY` | `roko-agent`, `roko-core`, `roko-serve`, relay |
| `DOCV2-T06` | Extensions/triggers/tools | `12-EXTENSIONS`, `13-TRIGGERS`, `14-TOOLS` | `roko-core`, `roko-std`, `roko-agent`, `roko-acp` |
| `DOCV2-T07` | Telemetry/security/auth/payments/config | `15-TELEMETRY`, `16-SECURITY`, `17-AUTH`, `18-PAYMENTS`, `19-CONFIG` | `roko-core`, `roko-serve`, `roko-chain` |
| `DOCV2-T08` | Surfaces/marketplace | `20-SURFACES`, `21-MARKETPLACE`, `15-TELEMETRY` | projection contracts, dashboard, jobs/marketplace models |
| `DOCV2-T09` | Registries/arenas/DeFi | `22-REGISTRIES`, `23-ARENAS`, `24-DEFI` | `roko-chain`, Solidity contracts, chain/isfr/bench routes |
| `DOCV2-T10` | Deployment/roadmap/guides | `25-DEPLOYMENT`, `28-ROADMAP`, `CLI-REFERENCE`, `API-REFERENCE`, `INTEGRATION-GUIDE`, `ACP-INTEGRATION-GUIDE`, `ARCHITECTURE-GUIDE` | CLI, OpenAPI, deployment, ACP, public docs |

## Coverage Ledger

| Source file | Task coverage | Notes |
|---|---|---|
| `docs/v2/00-INDEX.md` | `DOCV2-T01` | Global v2 vocabulary, one rule, document map, naming decisions. |
| `docs/v2/01-SIGNAL.md` | `DOCV2-T01` | Signal/Pulse, Store/Bus, Kind, scoring, demurrage, taint, lineage, graduation/projection. |
| `docs/v2/02-CELL.md` | `DOCV2-T01` | Cell trait, nine protocols, TypeSchema, capabilities, Store-Bus duality, Verify oracle. |
| `docs/v2/03-GRAPH.md` | `DOCV2-T02` | Graph, typed DAG, TOML graph definitions, hot graphs, Flow snapshots and merge queues. |
| `docs/v2/04-EXECUTION.md` | `DOCV2-T02` | Engine, Flow lifecycle, node execution, replay, error algebra, degradation, budgets, cancellation. |
| `docs/v2/05-AGENT.md` | `DOCV2-T03` | Agent lifecycle, vitality, modes, cognitive timescales, EFE, T0 probes, PAD/energy, goals. |
| `docs/v2/06-MEMORY.md` | `DOCV2-T04` | Memory specialization, demurrage, heuristics, HDC, dreams, temporal graph, pheromones. |
| `docs/v2/07-LEARNING.md` | `DOCV2-T04` | Predict-publish-correct loops, CascadeRouter, c-factor, playbooks, anti-metrics, safety. |
| `docs/v2/08-GATEWAY.md` | `DOCV2-T05` | Inference gateway pipeline, cache, provider call, cost tracking, batch API, routes. |
| `docs/v2/09-FEEDS.md` | `DOCV2-T05` | Feeds, recipes, registry, lifecycle, subscription, dashboard integration, on-chain advertisement. |
| `docs/v2/10-GROUPS.md` | `DOCV2-T05` | Group primitive, identity, coordination modes, membership, shared context, APIs. |
| `docs/v2/11-CONNECTIVITY.md` | `DOCV2-T05` | Connect protocol, connectors, relay, exoskeleton protocols, subscriptions, backpressure, reconnection. |
| `docs/v2/12-EXTENSIONS.md` | `DOCV2-T06` | Extension layers, hooks, decisions, loading, dependency resolution, lifecycle, config. |
| `docs/v2/13-TRIGGERS.md` | `DOCV2-T06` | Trigger sources, bindings, debounce, filtering, conductor watchers, APIs, config. |
| `docs/v2/14-TOOLS.md` | `DOCV2-T06` | Built-in Cell/tool catalog, MCP integration, safety hooks, schema, tool parity. |
| `docs/v2/15-TELEMETRY.md` | `DOCV2-T07`, `DOCV2-T08` | Lens/Observe, StateHub, c-factor, dashboard contracts, telemetry config. |
| `docs/v2/16-SECURITY.md` | `DOCV2-T07` | Taint lattice, immune pipeline, capability intersection, sandboxing, CaMeL, corrigibility. |
| `docs/v2/17-AUTH.md` | `DOCV2-T07` | Auth paths, team sharing, invitations, JWKS, token lifecycle, auth-as-Verify. |
| `docs/v2/18-PAYMENTS.md` | `DOCV2-T07` | x402, MPP, pricing, relay payments, disputes, feed marketplace economics. |
| `docs/v2/19-CONFIG.md` | `DOCV2-T07` | Config-as-Signal, source priority, compose/verify, reload graph, migrations, profiles, secrets. |
| `docs/v2/20-SURFACES.md` | `DOCV2-T08` | CLI, HTTP, TUI, dashboard, visual authoring, Workbench, Agent Inbox, Generative Canvas. |
| `docs/v2/21-MARKETPLACE.md` | `DOCV2-T08` | Agent passport, reputation, commerce, DAW composability, fork chains, transparent economics. |
| `docs/v2/22-REGISTRIES.md` | `DOCV2-T09` | ERC-8004, ZK-HDC proofs, InsightStore, chain witness, gossip networking, job market. |
| `docs/v2/23-ARENAS.md` | `DOCV2-T09` | Arenas, evals, bounties, scoring, leaderboards, contracts, APIs, events. |
| `docs/v2/24-DEFI.md` | `DOCV2-T09` | ISFR, yield perpetuals, VCG clearing, multi-chain data, risk, TradingReflect, APIs. |
| `docs/v2/25-DEPLOYMENT.md` | `DOCV2-T10` | Deployment tiers, daemon lifecycle, packaging, brain export/import, secrets, Railway/Fly/Docker, worker mode. |
| `docs/v2/26-CROSS-CUTS.md` | `DOCV2-T03`, `DOCV2-T04` | Memory/Daimon/Dreams endofunctors, VCG arbitration, safety wrapper, gate failure cascade. |
| `docs/v2/27-ORCHESTRATOR.md` | `DOCV2-T02` | Runner-v2 architecture, event loop, persistence, Mori parity, migration strategy. |
| `docs/v2/28-ROADMAP.md` | `DOCV2-T10` | Phase status, dependency graph, current-state reconciliation, milestones. |
| `docs/v2/ACP-INTEGRATION-GUIDE.md` | `DOCV2-T10` | ACP JSON-RPC, sessions, streaming notifications, pipeline workflow, knowledge, config, transport. |
| `docs/v2/API-REFERENCE.md` | `DOCV2-T10` | Public HTTP/API contract and route reference. |
| `docs/v2/ARCHITECTURE-GUIDE.md` | `DOCV2-T01`, `DOCV2-T02`, `DOCV2-T03`, `DOCV2-T04`, `DOCV2-T10` | Long-form architecture guide and current code mapping across kernel, runtime, learning, agent, and public docs. |
| `docs/v2/CLI-REFERENCE.md` | `DOCV2-T10` | Public CLI contract and command examples. |
| `docs/v2/INTEGRATION-GUIDE.md` | `DOCV2-T10` | End-to-end setup, config, providers, MCP, gates, learning, events, deployment, WorkflowEngine integration. |

## Validation Commands

Run these from the repository root:

```sh
cargo run -q -p roko-cli --bin roko -- plan validate tmp/status-quo/backlog/plans/DOC-v2-core
```

```sh
python3 - <<'PY'
from pathlib import Path
import re

plan = Path("tmp/status-quo/backlog/plans/DOC-v2-core/tasks.toml").read_text()
ledger = Path("tmp/status-quo/backlog/source-coverage/docs-v2-core.md").read_text()
sources = sorted(str(p) for p in Path("docs/v2").glob("**/*.md"))

missing_from_ledger = [p for p in sources if p not in ledger]
missing_from_plan = [p for p in sources if p not in plan]
print(f"sources={len(sources)}")
print(f"missing_from_ledger={missing_from_ledger}")
print(f"missing_from_plan={missing_from_plan}")
raise SystemExit(1 if missing_from_ledger or missing_from_plan else 0)
PY
```
