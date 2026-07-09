# Workspace Topology

All crates, their paths, and approximate sizes.

| Crate | Path | LOC | Role |
|-------|------|-----|------|
| roko-agent-server | crates/roko-agent-server/src/ | 3314 | Per-agent HTTP sidecar |
| roko-agent | crates/roko-agent/src/ | 44225 | Agent backends, tool loop, safety, MCP |
| roko-chain | crates/roko-chain/src/ | 2787 | Chain witness primitives (Phase 2+) |
| roko-cli | crates/roko-cli/src/ | 83065 | CLI binary: all subcommands, TUI |
| roko-compose | crates/roko-compose/src/ | 17080 | Prompt assembly, templates, enrichment |
| roko-conductor | crates/roko-conductor/src/ | 6212 | Watchers, circuit breaker, diagnosis |
| roko-core | crates/roko-core/src/ | 29482 | Kernel: Signal + 6 traits, types, config |
| roko-daimon | crates/roko-daimon/src/ | 2696 | Behavior primitives (Phase 2+) |
| roko-demo | crates/roko-demo/src/ | 5863 | — |
| roko-dreams | crates/roko-dreams/src/ | 6355 | Offline consolidation (Phase 2+) |
| roko-fs | crates/roko-fs/src/ | 4593 | FileSubstrate (JSONL), GC, layout |
| roko-gate | crates/roko-gate/src/ | 11190 | 11 gates, 7-rung pipeline, adaptive thresholds |
| roko-index | crates/roko-index/src/ | 1964 | Parser + graph + HDC indexing |
| roko-lang-go | crates/roko-lang-go/src/ | 600 | Language support |
| roko-lang-rust | crates/roko-lang-rust/src/ | 819 | Language support |
| roko-lang-typescript | crates/roko-lang-typescript/src/ | 917 | Language support |
| roko-learn | crates/roko-learn/src/ | 36355 | Episodes, playbooks, bandits, routing |
| roko-mcp-code | crates/roko-mcp-code/src/ | 422 | Code-intelligence MCP server |
| roko-mcp-github | crates/roko-mcp-github/src/ | 2650 | MCP integration |
| roko-mcp-scripts | crates/roko-mcp-scripts/src/ | 758 | MCP integration |
| roko-mcp-slack | crates/roko-mcp-slack/src/ | 922 | MCP integration |
| roko-mcp-stdio | crates/roko-mcp-stdio/src/ | 251 | MCP integration |
| roko-neuro | crates/roko-neuro/src/ | 7939 | Durable knowledge store, distillation |
| roko-orchestrator | crates/roko-orchestrator/src/ | 12852 | Plan DAG, parallel executor, merge queue |
| roko-plugin | crates/roko-plugin/src/ | 1078 | — |
| roko-primitives | crates/roko-primitives/src/ | 518 | HDC vectors, tier routing |
| roko-runtime | crates/roko-runtime/src/ | 1888 | ProcessSupervisor, event bus, cancellation |
| roko-serve | crates/roko-serve/src/ | 22608 | HTTP control plane (~200 routes) |
| roko-std | crates/roko-std/src/ | 4867 | Defaults, builtin tools, mock dispatcher |

## Key paths

- **Workspace root**: `/Users/will/dev/nunchi/roko/roko/`
- **All crates**: `/Users/will/dev/nunchi/roko/roko/crates/`
- **CLI source**: `crates/roko-cli/src/`
- **Orchestrator**: `crates/roko-cli/src/orchestrate.rs`
- **Agent dispatcher**: `crates/roko-agent/src/dispatcher/mod.rs`
- **System prompt builder**: `crates/roko-compose/src/system_prompt_builder.rs`

## Do not touch

- `bardo-backup/` — read-only reference material
- `.roko/` — runtime data directory (gitignored)
- `tmp/` — runner artifacts (gitignored)
- `target/` — build artifacts
