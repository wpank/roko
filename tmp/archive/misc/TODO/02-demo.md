# demo/ — Demo Scenario Tasks

**Directory**: `tmp/demo/`
**Status**: ACTIVE — code written, needs end-to-end validation
**Files**: `DEMO-IMPLEMENTATION-PLAN.md`, `tasks/T1.1–T3.8` (17 tasks), `tasks/ERRATA.md`
**Target events**: A16Z Demo (Apr 25), Consensus Miami (May 7)
**Crate**: `crates/roko-demo/`

## Task Status

| Task | Title | Status | Source File |
|------|-------|--------|-------------|
| T1.1 | Real LLM providers (Claude API + Ollama) | DONE | `crates/roko-demo/src/llm.rs` |
| T1.2 | Yield routing scenario skeleton | DONE | `crates/roko-demo/src/scenarios/yield_routing.rs` |
| T1.3 | FeeDistributor contract | UNVERIFIED | `contracts/src/FeeDistributor.sol` (needs `forge test`) |
| T1.4 | Event stream infrastructure | DONE | `crates/roko-demo/src/events.rs`, `ws_server.rs` |
| T2.1 | Wire LLM + events into yield-routing | DONE | `crates/roko-demo/src/scenarios/yield_routing.rs` |
| T2.2 | Knowledge loop integration | DONE | `query_insights()` in yield_routing.rs |
| T2.3 | Fee distribution wiring | DONE | `crates/roko-demo/src/bindings.rs` |
| T2.4 | C-Factor benchmark | DONE | `crates/roko-demo/src/benchmark.rs` |
| T2.5 | InsightBoard getInsight binding | DONE | `crates/roko-demo/src/bindings.rs` |
| T3.1 | TUI demo mode (ratatui) | PARTIAL | `crates/roko-demo/src/tui.rs` — event loop may be incomplete |
| T3.2 | Multi-model labeling | DONE | `LlmProvider.label()` in llm.rs |
| T3.3 | Multi-round tournament | DONE | `crates/roko-demo/src/tournament.rs` |
| T3.4 | Knowledge graph JSON output | DONE | `KnowledgeGraphUpdate` in events.rs |
| T3.5 | Reputation persistence | DONE | `save_reputation()` / `restore_reputation()` |
| T3.6 | One-click agent registration | DONE | `register_agent_cmd()` in main.rs |
| T3.7 | Autonomous agent loop | DONE | `crates/roko-demo/src/autonomous.rs` |
| T3.8 | Adversarial slashing demo | DONE | `AgentSlashed` event variant |

## Remaining Validation Checklist

### P0 — Blocking demo

- [ ] Verify `FeeDistributor.sol` compiles and tests pass (`forge test`)
- [ ] Complete TUI event loop in `tui.rs` (keyboard input, render binding)
- [ ] Run full yield-routing scenario end-to-end (6 contracts deploy, 2 rounds, events stream, fees distribute, reputation accumulates)

### P1 — Demo polish

- [ ] Test autonomous agent loop (T3.7) — free agents competing
- [ ] Test slashing path (T3.8) — validator rejection
- [ ] Wire reputation persistence (T3.5) into CLI flag
- [ ] Integration test tournament mode (T3.3)

### P2 — Nice to have

- [ ] C-Factor benchmark as pre-demo sanity check
- [ ] Wire knowledge graph visualization to dashboard

## Source Files

- **Master plan**: `tmp/demo/DEMO-IMPLEMENTATION-PLAN.md`
- **Task index**: `tmp/demo/tasks/00-INDEX.md`
- **Task specs**: `tmp/demo/tasks/T*.md`
- **Errata**: `tmp/demo/tasks/ERRATA.md`
- **Runner**: `tmp/demo/tasks/run-tasks.sh`
- **Demo crate**: `crates/roko-demo/src/`
- **Scenario TOML**: `demo/scenarios/yield-routing.toml`
