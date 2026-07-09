# Roko Architecture (for ACP bridge authors)

## Where roko-acp fits

`roko-acp` is a **presentation layer** — it sits alongside `roko-cli` and `roko-serve` as an alternative frontend. The entire cognitive pipeline runs identically:

```
Editor ←stdio→ roko-acp ──→ roko-orchestrator ──→ roko-agent ──→ LLM
                   │              │                    │
                   │              ├──→ roko-gate        │
                   │              ├──→ roko-compose      │
                   │              ├──→ roko-learn        │
                   │              └──→ roko-conductor    │
                   │                                    │
                   └── bridges (fs, terminal, perms) ◄──┘
```

## Key crates to know

| Crate | What | You'll use it for |
|-------|------|-------------------|
| `roko-core` | Signal + 6 traits, types, config, tools, errors | `Engram`, `AgentRole`, config types |
| `roko-agent` | LLM backends, dispatch, tool loop | Agent spawning, response streaming |
| `roko-orchestrator` | Plan DAG, executor, merge queue | Plan execution, task management |
| `roko-compose` | Prompt assembly, 9 templates | System prompt building |
| `roko-gate` | 11 gates, 7-rung pipeline | Gate execution and results |
| `roko-fs` | FileSubstrate (JSONL), GC, layout | Signal/episode persistence |
| `roko-runtime` | ProcessSupervisor, event bus, cancellation | Process lifecycle, CancelToken |
| `roko-conductor` | 10 watchers, circuit breaker | Auto-correction, diagnosis |
| `roko-learn` | Episodes, playbooks, bandits, routing | Learning state queries |
| `roko-neuro` | Durable knowledge store | Knowledge queries |
| `roko-daimon` | Affect engine, somatic markers | PAD state |
| `roko-primitives` | HDC vectors, tier routing | Model tier types |

## Key types

- `roko_core::config::RokoConfig` — workspace configuration from `roko.toml`
- `roko_core::types::AgentRole` — agent role enum (Architect, Implementer, Reviewer, etc.)
- `roko_runtime::CancelToken` — cooperative cancellation
- `roko_gate::GateResult` — gate pass/fail with details
- `roko_learn::CascadeRouter` — model tier routing
- `roko_learn::CostLens` — token/cost accumulator

## Workspace layout

```
.roko/
├── roko.toml              # Config
├── signals.jsonl           # Signal log
├── episodes.jsonl          # Episode log
├── state/                  # Executor snapshots
├── learn/                  # Learning state (cascade-router.json, etc.)
├── prd/                    # PRD documents
└── research/               # Research artifacts
```
