# Project Orientation

Roko is a Rust toolkit for building agents that build themselves. 18 crates, ~177K LOC.

## Crate Map

| Crate | Path | Role |
|---|---|---|
| roko-core | `crates/roko-core/` | Signal + 6 verb traits, types, config, tools, errors |
| roko-agent | `crates/roko-agent/` | LLM backends, pools, MCP, tool loop, safety |
| roko-agent-server | `crates/roko-agent-server/` | Per-agent HTTP sidecar |
| roko-serve | `crates/roko-serve/` | HTTP control plane (~85 routes) |
| roko-orchestrator | `crates/roko-orchestrator/` | Plan DAG, parallel executor, merge queue |
| roko-gate | `crates/roko-gate/` | 11 gates, 7-rung pipeline, adaptive thresholds |
| roko-compose | `crates/roko-compose/` | Prompt assembly, 9 templates, enrichment |
| roko-conductor | `crates/roko-conductor/` | 10 watchers, circuit breaker, diagnosis |
| roko-learn | `crates/roko-learn/` | Episodes, playbooks, bandits, model routing |
| roko-cli | `crates/roko-cli/` | CLI binary: all subcommands, ratatui TUI |
| roko-fs | `crates/roko-fs/` | FileSubstrate (JSONL), GC, layout |
| roko-std | `crates/roko-std/` | Defaults, 19 builtin tools, mock dispatcher |
| roko-runtime | `crates/roko-runtime/` | ProcessSupervisor, event bus, cancellation |
| roko-primitives | `crates/roko-primitives/` | HDC vectors, tier routing |
| roko-neuro | `crates/roko-neuro/` | Durable knowledge store, distillation |
| roko-mcp-code | `crates/roko-mcp-code/` | Code-intelligence MCP server |
| roko-index | `crates/roko-index/` | Parser + graph + HDC indexing |
| roko-dreams | `crates/roko-dreams/` | Offline consolidation |
| roko-daimon | `crates/roko-daimon/` | Affect engine, somatic markers |
| roko-chain | `crates/roko-chain/` | Chain witness primitives |

## Architecture Pattern

**Unified model**: 1 noun (Signal) + 9 protocol traits + 10 specializations.
Universal loop: OBSERVE → RETRIEVE → ANALYZE → GATE → SIMULATE → VALIDATE → EXECUTE → VERIFY → REFLECT.

**Naming precedence**: `tmp/unified/` (canonical) > `tmp/architecture/` (current design) > `docs/` (legacy).

## Key Paths

| What | Path | Priority |
|---|---|---|
| **Unified spec** | `tmp/unified/` (22 files) | **CANONICAL — overrides everything** |
| **Architecture** | `tmp/architecture/` (21 files) | **Current design — supplements unified** |
| **Unified depth** | `tmp/unified-depth/` (21 dirs) | Algorithms, research, implementation detail |
| **Migration phases** | `tmp/unified-migration/` (4 files) | The checklist driving this runner |
| Workspace root | `/Users/will/dev/nunchi/roko/roko/` | — |
| CLI source | `crates/roko-cli/src/` | — |
| Orchestrator | `crates/roko-cli/src/orchestrate.rs` | — |
| Legacy docs | `docs/` (422 files, 8.8MB) | Historical only — use for algorithms, not naming |
