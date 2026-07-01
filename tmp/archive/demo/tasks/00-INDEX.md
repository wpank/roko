# Demo Implementation Tasks — Index

> Each task is a self-contained spec for a fresh Claude/Codex agent.
> **IMPORTANT**: The runner script (`run-tasks.sh`) automatically prepends
> [ERRATA.md](ERRATA.md) to every task prompt. The errata corrects contract
> API signatures, Rust syntax, and TOML schema issues found during audit.

## Quick Start

```bash
cd /Users/will/dev/nunchi/roko/roko/tmp/demo/tasks

# Preview what would run:
./run-tasks.sh --dry-run

# Run a single task:
./run-tasks.sh T1.3

# Run a batch:
./run-tasks.sh --batch 1

# Run everything in order:
./run-tasks.sh
```

## Dependency Graph

```
T2.5 (InsightBoard bindings) ──────────────┐
T1.3 (FeeDistributor contract) ────────────┤
T1.1 (LLM Providers) ─────────────────┐    │
T1.4 (Event Stream) ──────────────┐   │    │
T1.2 (Yield Skeleton) ────────┐   │   │    │
                               │   │   │    │
                               ▼   ▼   ▼    │
                           T2.1 (Full Yield + Events)
                               │             │
                       ┌───────┤             │
                       ▼       ▼             ▼
                    T2.2    T2.3          T3.1 (TUI)
                  (Knowledge) (Fees)      T3.2 (Labels)
                       │                  T3.5 (Persist)
                       ▼                  T3.6 (Register)
                    T2.4 ★ (C-Factor — Apr 15)
                       │
               ┌───────┼───────┐
               ▼       ▼       ▼
            T3.3     T3.4    T3.7
          (Tourney) (Graph) (Autonomous)
                               │
                               ▼
                            T3.8 (Adversarial)
```

## File Inventory

| File | Purpose |
|------|---------|
| [ERRATA.md](ERRATA.md) | **Read first** — contract API corrections, Rust syntax fixes, TOML schema |
| [run-tasks.sh](run-tasks.sh) | Runner script — assembles prompt (errata + task), runs claude |
| [DEMO-IMPLEMENTATION-PLAN.md](../DEMO-IMPLEMENTATION-PLAN.md) | Master plan (architecture, scenarios, timeline) |

## All Tasks

### Tier 1: Foundation (no cross-dependencies)

| # | File | Description | Est. |
|---|------|-------------|------|
| T2.5 | [T2.5-insightboard-enhancements.md](T2.5-insightboard-enhancements.md) | Add getInsight binding to Rust (Solidity already has it) | S |
| T1.3 | [T1.3-fee-distributor-contract.md](T1.3-fee-distributor-contract.md) | FeeDistributor.sol: 40/30/20/10 split + forge tests | S-M |
| T1.1 | [T1.1-real-llm-providers.md](T1.1-real-llm-providers.md) | Claude API + Ollama + Multi LLM providers | M |
| T1.4 | [T1.4-event-stream-infrastructure.md](T1.4-event-stream-infrastructure.md) | DemoEvent enum + NDJSON + WebSocket emitters | M |
| T1.2 | [T1.2-yield-routing-skeleton.md](T1.2-yield-routing-skeleton.md) | New yield-routing scenario: all 6 contracts, 2-round spine | L |

> **Conflict note**: T1.1 and T1.4 both modify `main.rs`. Run them sequentially.
> T2.5 and T1.3 are fully isolated (Solidity / bindings only). Safe first.

### Tier 2: Integration

| # | File | Depends | Description | Est. |
|---|------|---------|-------------|------|
| T2.1 | [T2.1-full-yield-routing-with-llm.md](T2.1-full-yield-routing-with-llm.md) | T1.1,T1.2,T1.4 | Wire events + LLM into yield routing, update Scenario trait | L |
| T2.2 | [T2.2-knowledge-loop-integration.md](T2.2-knowledge-loop-integration.md) | T2.1,T2.5 | InsightBoard query before, post after execution | M |
| T2.3 | [T2.3-fee-distribution-wiring.md](T2.3-fee-distribution-wiring.md) | T1.3,T2.1 | Wire FeeDistributor into yield routing | S |
| T2.4 | [T2.4-cfactor-benchmark.md](T2.4-cfactor-benchmark.md) | T2.2 | `benchmark c-factor` command **★ DUE APR 15** | M |

### Tier 3: Polish & Stretch

| # | File | Depends | Description | Est. |
|---|------|---------|-------------|------|
| T3.1 | [T3.1-tui-demo-mode.md](T3.1-tui-demo-mode.md) | T1.4,T2.1 | ratatui terminal UI (4-panel layout) | M-L |
| T3.2 | [T3.2-multi-model-labeling.md](T3.2-multi-model-labeling.md) | T1.1,T2.1 | Per-agent model labels in events | S |
| T3.3 | [T3.3-multi-round-tournament.md](T3.3-multi-round-tournament.md) | T2.1,T2.2,T2.3 | N-round tournament with learning curve | M |
| T3.4 | [T3.4-knowledge-graph-json.md](T3.4-knowledge-graph-json.md) | T2.2 | Knowledge graph JSON for dashboard | S |
| T3.5 | [T3.5-reputation-persistence.md](T3.5-reputation-persistence.md) | T2.1 | Save/restore reputation across runs | S |
| T3.6 | [T3.6-one-click-agent-registration.md](T3.6-one-click-agent-registration.md) | T1.2 | `register-agent` CLI command | S |
| T3.7 | [T3.7-autonomous-agent-loop.md](T3.7-autonomous-agent-loop.md) | T2.1,T2.2 | Independent agent processes, no orchestrator | L |
| T3.8 | [T3.8-adversarial-agent-slashing.md](T3.8-adversarial-agent-slashing.md) | T2.1,T2.2,T2.3 | Bad knowledge + slashing demo | M |

## Runner Execution Order

The runner (`run-tasks.sh`) executes in this exact order:

| Batch | Tasks | Why this order |
|-------|-------|---------------|
| 1 | T2.5 → T1.3 → T1.1 → T1.4 → T1.2 | Isolated tasks first, then main.rs changes in sequence |
| 2 | T2.1 | Single integration task — touches Scenario trait + all spines |
| 3 | T2.2 → T2.3 | Knowledge loop, then fee wiring |
| 4 | T2.4 | C-Factor benchmark (★ critical deadline) |
| 5 | T3.1 → T3.2 → ... → T3.8 | Polish in order of demo impact |

## Critical Path to A16Z Demo (Apr 25)

Minimum 9 tasks:
```
T2.5 + T1.3 + T1.1 + T1.4 + T1.2 → T2.1 → T2.2 → T2.4★
                                            └→ T2.3
```

## Known Issues & Mitigations

| Issue | Mitigation |
|-------|-----------|
| Multiple tasks edit `main.rs` | Runner runs them sequentially; errata documents conflict zones |
| T2.2 and T2.5 both add getInsight binding | T2.5 runs first (Batch 1); T2.2 should check if already present |
| `ratatui`/`crossterm` not in workspace deps | T3.1 spec tells agent to add them as direct deps, not `{ workspace = true }` |
| Pseudocode in T3.7/T3.8 uses wrong alloy patterns | Errata documents correct patterns; agent told to read existing scenarios |
| StubLlm C-Factor is artificial | Noted in T2.4 — real improvement requires real LLM backends |
