# docs/v2 Core Source Coverage and Ownership

Source corpus: all Markdown files under `docs/v2/**`.

Plan: `tmp/status-quo/backlog/plans/DOC-v2-core/tasks.toml`

> CTRL-09 control-plane reconciliation. This ledger supersedes the former
> “primary implementation targets” interpretation of DOC-v2-core. E01-E45 own
> product behavior; DOCV2-T01 through T10 are documentation acceptance roll-ups
> that execute only after their named owner plans are merged.

## Contract

- Source Markdown files covered: **34**.
- Executable tasks preserved: **10**; task IDs and `ready` status are unchanged.
- Product implementation tasks in this plan: **0**.
- Every roll-up has `role = "scribe"`, writes only explicit `docs/v2/*.md`
  files, and depends on the canonical implementation plans for its claims.
- Files are disjoint across the ten writers. Shared sources may be read for
  consistency, but only the writer named below may edit them.
- A roll-up reconciles merged implementation, tests, Git history, and accepted
  owner evidence. Unsupported normative material remains with an explicit
  target/deferred label; it is never silently presented as current behavior.
- Documentation defects do not authorize product changes. A missing capability
  returns to its canonical owner; the roll-up records the verified limitation.
- Completion still requires independent documentation review. No CTRL-09 task
  is marked done by this control-plane change.

## Task and product-owner map

| Roll-up | Sole write ownership | Canonical product-owner prerequisites |
|---|---|---|
| `DOCV2-T01` | `01-SIGNAL`, `02-CELL` | E03, E13, E19 Signal, E20 Cell, E29 Connect, E31 Trigger, E33 Observe, E34 IFC/capabilities |
| `DOCV2-T02` | `03-GRAPH`, `04-EXECUTION`, `27-ORCHESTRATOR` | E01 execution, E05 gates, E08 conductor, E21 Graph, E22 runtime, E45 runner/Mori |
| `DOCV2-T03` | `05-AGENT` | E06 compose, E08 conductor, E17 ACP, E23 agent autonomy, E44 cross-cuts |
| `DOCV2-T04` | `06-MEMORY`, `07-LEARNING`, `26-CROSS-CUTS` | E06 compose, E07 knowledge, E09 observability, E24 memory, E25 learning, E44 cross-cuts |
| `DOCV2-T05` | `08-GATEWAY`, `09-FEEDS`, `10-GROUPS`, `11-CONNECTIVITY` | E14 providers, E15 MCP, E26 gateway, E27 feeds, E28 groups, E29 connectivity, E36 payments, E39 identity |
| `DOCV2-T06` | `12-EXTENSIONS`, `13-TRIGGERS`, `14-TOOLS` | E04 security, E14 providers/tools, E15 MCP, E17 ACP, E30 extensions, E31 triggers, E32 tools/plugins |
| `DOCV2-T07` | `15-TELEMETRY`, `16-SECURITY`, `17-AUTH`, `18-PAYMENTS`, `19-CONFIG` | E02 storage, E04 security, E09 observability, E18 docs/config/ops, E33 telemetry, E34 IFC, E35 auth, E36 payments, E42 config |
| `DOCV2-T08` | `20-SURFACES`, `21-MARKETPLACE` | E10 frontend, E33 telemetry, E36 payments, E37 surfaces, E38 marketplace, E39 identity |
| `DOCV2-T09` | `22-REGISTRIES`, `23-ARENAS`, `24-DEFI` | E11 chain/ISFR, E39 registries, E40 arenas/evals, E41 DeFi |
| `DOCV2-T10` | `00-INDEX`, `25-DEPLOYMENT`, `28-ROADMAP`, all five public guides | all nine earlier DOCV2 roll-ups; E01, E10, E11, E16, E17, E18, E43 deployment, E45 runner/Mori |

All E19-E45 implementation plans occur in at least one prerequisite mapping.
The local DAG makes `DOCV2-T10` the final global-status/public-guide pass after
all topical writers have completed.

## Coverage ledger

| Source file | Writer | Read-only cross-checks | Subject |
|---|---|---|---|
| `docs/v2/00-INDEX.md` | `DOCV2-T10` | `DOCV2-T01` | Global vocabulary, document map, naming, and integrated status. |
| `docs/v2/01-SIGNAL.md` | `DOCV2-T01` | — | Signal/Pulse, Store/Bus, scoring, demurrage, taint, lineage, graduation/projection. |
| `docs/v2/02-CELL.md` | `DOCV2-T01` | — | Cell, nine protocols, schemas, capabilities, protocol algebra, Verify oracle. |
| `docs/v2/03-GRAPH.md` | `DOCV2-T02` | — | Graph, typed DAG, hot graphs, snapshots, and merge queues. |
| `docs/v2/04-EXECUTION.md` | `DOCV2-T02` | — | Flow lifecycle, execution, replay, errors, degradation, budgets, cancellation. |
| `docs/v2/05-AGENT.md` | `DOCV2-T03` | — | Lifecycle, vitality, modes, timescales, EFE, probes, PAD/energy, goals. |
| `docs/v2/06-MEMORY.md` | `DOCV2-T04` | — | Memory, demurrage, heuristics, HDC, dreams, temporal graph, pheromones. |
| `docs/v2/07-LEARNING.md` | `DOCV2-T04` | — | Learning loops, router, c-factor, playbooks, anti-metrics, safety. |
| `docs/v2/08-GATEWAY.md` | `DOCV2-T05` | — | Gateway pipeline, cache, providers, cost, batch API, routes. |
| `docs/v2/09-FEEDS.md` | `DOCV2-T05` | — | Feeds, recipes, registry, subscriptions, dashboard, chain advertisement. |
| `docs/v2/10-GROUPS.md` | `DOCV2-T05` | — | Groups, identity, coordination, membership, shared context, APIs. |
| `docs/v2/11-CONNECTIVITY.md` | `DOCV2-T05` | — | Connect, connectors, relay, subscriptions, backpressure, reconnection. |
| `docs/v2/12-EXTENSIONS.md` | `DOCV2-T06` | — | Extension layers, hooks, loading, dependency resolution, lifecycle. |
| `docs/v2/13-TRIGGERS.md` | `DOCV2-T06` | — | Sources, bindings, debounce, filtering, watchers, APIs, config. |
| `docs/v2/14-TOOLS.md` | `DOCV2-T06` | — | Tool/Cell catalog, MCP, safety, schemas, handler parity. |
| `docs/v2/15-TELEMETRY.md` | `DOCV2-T07` | `DOCV2-T08` | Lens/Observe, StateHub, c-factor, projections, telemetry config. |
| `docs/v2/16-SECURITY.md` | `DOCV2-T07` | — | IFC, immune pipeline, capabilities, sandboxing, CaMeL, corrigibility. |
| `docs/v2/17-AUTH.md` | `DOCV2-T07` | — | Auth paths, teams, invitations, JWKS, tokens, auth-as-Verify. |
| `docs/v2/18-PAYMENTS.md` | `DOCV2-T07` | — | x402, MPP, pricing, relay payments, disputes, marketplace economics. |
| `docs/v2/19-CONFIG.md` | `DOCV2-T07` | — | Config provenance, priority, reload, migration, profiles, secrets. |
| `docs/v2/20-SURFACES.md` | `DOCV2-T08` | — | CLI, HTTP, TUI, dashboard, visual authoring, Workbench, Inbox. |
| `docs/v2/21-MARKETPLACE.md` | `DOCV2-T08` | — | Identity, reputation, commerce, composability, forks, economics. |
| `docs/v2/22-REGISTRIES.md` | `DOCV2-T09` | — | ERC-8004, proofs, InsightStore, witness, gossip, job market. |
| `docs/v2/23-ARENAS.md` | `DOCV2-T09` | — | Arenas, evals, bounties, scoring, leaderboards, APIs, events. |
| `docs/v2/24-DEFI.md` | `DOCV2-T09` | — | ISFR, yield products, VCG, multi-chain data, risk, reflection. |
| `docs/v2/25-DEPLOYMENT.md` | `DOCV2-T10` | — | Tiers, daemon, packaging, brain transfer, secrets, workers, monitoring. |
| `docs/v2/26-CROSS-CUTS.md` | `DOCV2-T04` | `DOCV2-T03` | Memory/Daimon/Dreams transforms, arbitration, safety, gate cascade. |
| `docs/v2/27-ORCHESTRATOR.md` | `DOCV2-T02` | — | Runner-v2, event loop, persistence, Mori parity, migration. |
| `docs/v2/28-ROADMAP.md` | `DOCV2-T10` | — | Integrated phase status, dependency graph, mappings, milestones. |
| `docs/v2/ACP-INTEGRATION-GUIDE.md` | `DOCV2-T10` | — | ACP JSON-RPC, sessions, notifications, workflow, knowledge, transport. |
| `docs/v2/API-REFERENCE.md` | `DOCV2-T10` | — | Public HTTP/API and route contract. |
| `docs/v2/ARCHITECTURE-GUIDE.md` | `DOCV2-T10` | `DOCV2-T01`–`T04` | Public architecture and integrated code mapping. |
| `docs/v2/CLI-REFERENCE.md` | `DOCV2-T10` | — | Public CLI contract and examples. |
| `docs/v2/INTEGRATION-GUIDE.md` | `DOCV2-T10` | — | Setup, config, providers, MCP, gates, events, deployment, engine. |

## Validation

Run from the repository root:

```sh
cargo run -q -p roko-cli --bin roko -- plan validate --strict tmp/status-quo/backlog/plans
```

```sh
python3 - <<'PY'
from pathlib import Path
import tomllib

manifest = Path("tmp/status-quo/backlog/plans/DOC-v2-core/tasks.toml")
data = tomllib.loads(manifest.read_text())
ledger = Path("tmp/status-quo/backlog/source-coverage/docs-v2-core.md").read_text()
sources = {str(path) for path in Path("docs/v2").glob("**/*.md")}
context = {
    item["path"]
    for task in data["task"]
    for item in task["context"]["read_files"]
    if item["path"].startswith("docs/v2/")
}
writes = [path for task in data["task"] for path in task["files"]]
owners = {dep for task in data["task"] for dep in task["depends_on_plan"]}

assert len(sources) == 34
assert sources <= context
assert sources <= set(writes)
assert len(writes) == len(set(writes)) == 34
assert all(path in ledger for path in sources)
assert len(data["task"]) == data["meta"]["total"] == 10
assert data["meta"]["done"] == 0
assert all(task["status"] == "ready" for task in data["task"])
assert all(task["role"] == "scribe" for task in data["task"])
assert all(task.get("ownership") == "documentation-acceptance-roll-up" for task in data["task"])
assert all(path.startswith("docs/v2/") for path in writes)
assert all(next(iter(Path("tmp/status-quo/backlog/plans").glob(f"E{n:02d}-*/tasks.toml"))).parent.name in owners for n in range(19, 46))
print("34 sources; 34 disjoint doc writers; 10 ready roll-ups; E19-E45 all mapped")
PY
```
