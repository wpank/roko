# impl2 gap-fix PRDs: index

Audit date: 2026-04-22. These six PRDs address every wiring gap found in the
exhaustive audit of the roko codebase against the claims in `CLAUDE.md`. The
audit confirmed that most subsystems are substantially correct; what remains
are specific, bounded integration gaps — no new crates are needed, only new
call sites, config routing, and a handful of stub replacements.

Raw evidence is in `06-audit-evidence.md`.

---

## PRDs in this set

| # | File | Topic | Tasks | Priority |
|---|------|-------|-------|----------|
| 01 | `01-chain-integration.md` | Wire roko-chain into agent runtime | 7 | 1 — highest |
| 02 | `02-config-unification.md` | Unify two config systems; retire dead sections | 12 | 1 — highest |
| 03 | `03-event-bridge-and-serve-gaps.md` | EventBus/StateHub bridge, roko chat, sidecar /research, mcp-code, subscriptions, serve health | 6 | 2 |
| 04 | `04-gates-safety-supervisor.md` | Gate rungs 3-6, safety for Claude CLI, ProcessSupervisor.spawn(), VCG, learning bidders | 7 | 2 |
| 05 | `05-learning-neuro-corrections.md` | Distillation no-op, CLAUDE.md status table, experiments seed, index cache | 5 | 3 |
| 07 | `07-dead-code-backend-gaps.md` | Dead code cleanup (pool, orchestrator modules, custody, serve scaffolding) and backend gaps (Ollama, Perplexity, Codex, scheduler, serve health) | 9 | 3 |

Total: 46 tasks across 6 PRDs.

---

## Dependency graph

PRDs 01 and 02 are the root work. Everything else can proceed in parallel once
they land, because 03 depends on the EventBus/StateHub changes that come from
correctly reading `server.port` (02), and 04's VCG fix requires knowing which
context is passed through the corrected config path (02).

```
01-chain-integration    ──────────────────────────────────────┐
                                                               │
02-config-unification   ──┬───────────────────────────────────┤
                           │                                   │
                           ├── 03-event-bridge-and-serve-gaps ─┤
                           │                                   │
                           └── 04-gates-safety-supervisor ─────┤
                                                               │
05-learning-neuro-corrections (no external deps) ─────────────┤
                                                               │
07-dead-code-backend-gaps (no external deps) ─────────────────┘
```

01 and 02 can be worked simultaneously — they touch distinct files. 03 and 04
can both start once 02 lands but are otherwise independent. 05 is fully
independent and can start at any time. 07 is fully independent and can start
at any time — it shares some gate-related scope with 04 (Gap A references
rung inputs) but touches distinct files; coordinate if both are in flight.

---

## Priority order

**1. Chain integration (PRD 01) + Config unification (PRD 02) in parallel.**

These two eliminate the largest structural gaps. Chain integration puts a
functioning `ChainClient` in the agent runtime path for the first time.
Config unification removes the parallel config universe that exists only in
TOML but is never consulted at runtime — a correctness hazard as the codebase
grows.

**2. Event bridge + serve gaps (PRD 03) + Gates / safety / supervisor (PRD 04)
in parallel, after PRD 02 lands.**

PRD 03 makes the live dashboard actually reflect orchestrator state. PRD 04
makes the gate pipeline correct and closes the three safety gaps.

**3. Learning and neuro corrections (PRD 05), any time.**

These are isolated fixes with no cross-PRD dependencies. Distillation
currently fires but never persists (silent no-op on missing API key); the
other items are documentation corrections and small bootstrap gaps.

**4. Dead code cleanup and backend gaps (PRD 07), any time.**

Structural hygiene: dead infrastructure removal, backend wiring, code
duplication consolidation. No cross-PRD dependencies except light overlap
with PRD 04 on gate-related scope (PRD 04 wires rung oracles; PRD 07
addresses the pool and orchestrator dead code that surrounds them).

---

## Total scope

- 46 implementation tasks
- 5 files touched in roko-chain (new wiring, no new code)
- 3 new call sites in `orchestrate.rs`
- 1 config reader consolidated (two roots collapsed to one)
- 2 stubs replaced with real implementations (`/research` response body,
  `ProcessSupervisor::spawn()` call)
- 1 CLAUDE.md section corrected (dreams/daimon status)
- 9 dead code / backend gap items addressed (pool wiring, 10 dead orchestrator
  modules, custody logging, serve scaffolding, Ollama/Perplexity/Codex
  backends, scheduler dedup, serve health tracking)

No new crates. No new external dependencies beyond what is already in
`Cargo.lock`. The `alloy-backend` feature for `roko-chain` is already gated
behind an optional feature flag; enabling it for `roko-cli` is the only
`Cargo.toml` change required.

---

## Additional findings from second audit pass

Quick reference for the 19 new findings documented in PRD 07:

1. **MultiAgentPool never used in dispatch** — pool initialized but `pre_spawn_warm`/`promote_warm`/`add_active`/`run_task` never called (PRD 07-A)
2. **merge_queue.rs dead** — MergeQueue/MergeRequest exported, zero callers (PRD 07-B)
3. **mesh_relay.rs dead** — MeshRelay for multi-node sync, zero callers (PRD 07-B)
4. **repair.rs dead** — RepairEngine, zero callers (PRD 07-B)
5. **progress.rs dead** — ProgressTracker, zero callers; PlanRunner has its own (PRD 07-B)
6. **5 orchestrator safety modules dead** — loop_guard, capability_tokens, sandboxing, taint_propagation, permit; real safety lives in roko-agent (PRD 07-B)
7. **AuditChain slot always None** — ParallelExecutor has the slot but orchestrate.rs never calls `with_audit_chain()` (PRD 07-B)
8. **CustodyLogger writer never called** — CLI readers work but no records are ever written (PRD 07-C)
9. **RelayHealth dead type** — comment says "exposed via GET /api/relay/health" but route does not exist (PRD 07-D)
10. **truth_map.rs not declared in lib.rs** — runtime doc registry with zero callers (PRD 07-D)
11. **Ollama bypasses provider system** — hardcoded `command == "ollama"` branch skips adapters (PRD 07-E)
12. **PerplexityToolLoopBackend not wired** — implements LlmBackend but factory returns error (PRD 07-F)
13. **Codex JSON-RPC/WebSocket not implemented** — HTTP fallback only, acknowledged in comments (PRD 07-G)
14. **Scheduler code duplication** — `roko serve` and `roko daemon` start cron independently (PRD 07-H)
15. **Provider health not wired in serve dispatch** — CLI tracks health, serve does not (PRD 07-I, also PRD 03-F)
16. **Subscriptions only trigger from WebhookReceived** — in-process events do not fire subscriptions (PRD 03-E)
17. **Rung 3 SymbolGate stubs** — stubs when `symbol_signal` or `source_roots` are None (PRD 04-A, expanded)
18. **Rung 4 GeneratedTestGate stubs** — stubs when `generated_test_artifacts` not wired (PRD 04-A, expanded)
19. **Rung 4 VerifyChainGate stubs** — stubs when no `verify_script` tag present (PRD 04-A, expanded)
