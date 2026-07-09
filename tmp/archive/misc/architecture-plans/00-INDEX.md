# Architecture plans

Implementation plans for the nunchi dashboard, organized by dependency layer.
Each plan is self-contained: an agent with no prior knowledge of the codebase
can pick up any plan and implement it by reading the referenced files.

## Repositories

| Repo | Path | What |
|------|------|------|
| roko | `/Users/will/dev/nunchi/roko/roko/` | Backend: CLI, HTTP control plane (`roko-serve`), agent runtime |
| nunchi-dashboard | `/Users/will/dev/nunchi/nunchi-dashboard/` | Frontend: React + Vite + TanStack Query + Zustand |
| mirage-rs | Within roko workspace, `apps/mirage-rs/` | Local devnet + agent relay at `:8545` |

## Layer map

```
Layer 0  Infrastructure           (done / in-progress)
Layer 1  Dashboard resilience     (no roko-serve dependency)
Layer 2  Agent creation + control
Layer 3  Agent streaming + steering
Layer 4  Plan execution UI
Layer 5  Self-hosting             (agents implement architecture)
Layer 6+ Extended surfaces        (knowledge, arenas, defi, meta)
```

## Plan index

| # | Title | Layer | Effort | Depends on | Status |
|---|-------|-------|--------|------------|--------|
| 01 | Dashboard resilience | 1 | M (2-3 days) | -- | ✅ Backend done (relay health fix); dashboard tasks 1.1-1.5 complete |
| 02 | Agent creation and control | 2 | M (2-3 days) | 01 | ✅ Backend done (agents, templates, secrets, team routes); frontend 2.1 pending |
| 03 | Agent streaming and steering | 3 | L (3-5 days) | 01, 02 | ✅ Backend done (WS, SSE, heartbeats, sidecar); frontend pending |
| 04 | [Plan execution UI](04-plan-execution.md) | 4 | M+L (4-5 days) | 01 | ✅ Backend done (pause, resume, gates, chat, estimate); frontend pending |
| 05 | [Self-hosting loop](05-self-hosting.md) | 5 | L (3-5 days) | 02, 03, 04 | ✅ Backend done (reviews, diff, submit review); 5.1-5.5 workflow pending |
| 06 | [Architecture implementation](06-architecture-implementation.md) | 6+ | XL (8-10 weeks with 4 agents) | 05 | Planned; audited 2026-04-25 |
| 07 | [Docs parity closure](07-docs-parity-closure.md) | 6+ | XL (4-6 weeks with 4 agents) | 06 | Planned |
| 08 | [End-to-end acceptance and parity gates](08-end-to-end-acceptance.md) | 0-6+ | L (1-2 weeks) | 01-07 | Planned |
| 09 | [Implementation tracker](IMPLEMENTATION-TRACKER.md) | all | Generated tracker | 01-08 | Generated 2026-04-25 |
| 10 | [Concern index](CONCERN-INDEX.md) | all | Generated concern routing | 09 | Generated 2026-04-25 |
| 11 | [Quality report](QUALITY-REPORT.md) | all | Generated verification report | 09, 10 | Generated 2026-04-25 |
| 12 | [Generalized cybernetic agent architecture gaps](generalized-cybernetic-agent-architecture-gaps.md) | roles/prompts/context | Audit + migration plan | 06, 07 | Added 2026-04-25 |

Effort: S = half day, M = 2-3 days, L = 3-5 days, XL = weeks (multi-agent).

### Plan 04 summary

8 tasks covering plan list/detail/creation pages, execution monitor with real-time events, gate results display, pause/resume, conversation-as-plan-editor, and cost estimation. Introduces 4 new roko-serve routes (`pause`, `resume`, `chat`, `estimate`) and 7 new types in `plan_types.rs`.

### Plan 05 summary

5 tasks for the self-hosting loop: generate roko plans from architecture docs, deploy persistent coding agents on Railway, implement review/approve workflow (3 new routes), execute the first architecture plan end-to-end, and iterate on friction points.

### Plan 06 summary (updated 2026-04-25)

13 phases (A-M) plus a parallel Orchestrator Gaps track covering all 21 architecture docs and 41 DeFi batches. Updated with a 2026-04-25 audit addendum so implementers extend existing modules instead of rebuilding components that now exist.

- **"What already exists" reconciliation tables** for every phase — prevents duplicate work. Found that ~40% of originally planned tasks already exist (types, routes, registries, watchers, etc.)
- **Rust struct definitions and algorithm pseudocode** for every task
- **Concrete acceptance criteria** with testable checkboxes on every task
- **Edge case resolutions**: PE oscillation, regime hysteresis, cache TTL regime sync, reflex confidence, JWKS stale-while-revalidate, A-MAC contradiction detection, cross-user invitation flow, pheromone decay, dispute escalation, lineage tracking, recursive safety bounds
- **Architecture decisions documented**: no roko-gateway crate, relay = roko-serve, groups vs teams distinction, off-chain-first bounties, three-tier dashboard deployment, epistemic aesthetics system

~137 tasks total before docs-parity closure. 4 agents in parallel = ~8-10 weeks estimated for the newer architecture surface.

### Plan 07 summary (added 2026-04-25)

Closes the gap between the newer `tmp/architecture` docs and the older, wider `docs/` corpus. Covers all 422 markdown docs in `docs/`, all 22 newer architecture docs, the DeFi gap plans, and the current Rust/Solidity workspace. Produces a file-level coverage ledger, resolves "built but not wired" gaps, and adds implementation batches for old-doc domains not fully represented in Plan 06: core kernel primitives, orchestration extraction, safety, code intelligence, tools/plugins, lifecycle, deployment, TUI/CLI/SDK, dreams, daimon, technical-analysis oracles, and references-to-runtime.

### Plan 08 summary (added 2026-04-25)

Defines the final parity gate. Implementation is not complete until build, test, route, dashboard, agent, relay, self-hosting, DeFi, auth, persistence, observability, and documentation traceability checks all pass. This plan converts "done" into a repeatable acceptance harness.

### Generated granular plan set (added 2026-04-25)

The broad plans above are now complemented by a generated granular plan set:

- [IMPLEMENTATION-TRACKER.md](IMPLEMENTATION-TRACKER.md) lists every source document and its generated plan file. Future Codex runs should update checkboxes in the source-specific plan file as implementation completes.
- [CONCERN-INDEX.md](CONCERN-INDEX.md) splits all work by implementation concern: API, realtime, storage, auth, chain, runtime, dashboard support, verification, config/deployment, knowledge/learning, and architecture contracts.
- [QUALITY-REPORT.md](QUALITY-REPORT.md) records scope, task counts, self-assessment rubric, and validation results.
- `arch-*.md` files map every heading in `/Users/will/dev/nunchi/roko/roko/tmp/architecture/*.md` to an implementation task.
- `dash-prd-*.md` files map every heading in `/Users/will/dev/nunchi/nunchi-dashboard/docs/prd/*.md` to backend-support work or an explicit no-backend/deferral rationale.
- `COVERAGE-MATRIX.json` is the machine-readable proof: every generated task has source line range, target artifacts, concern labels, verification commands, and a 9.6/10 self-assessment score.

**Orchestrator gaps** (20-orchestrator-gaps.md) also updated with:
- Current state reconciliation table (10 items already built, 10 gaps remaining)
- 12 spec clarifications resolving all audit ambiguities

### Generalized cybernetic agent architecture gaps (added 2026-04-25)

[generalized-cybernetic-agent-architecture-gaps.md](generalized-cybernetic-agent-architecture-gaps.md)
audits the hardcoded role, prompt, context assembly, context injection, and
learning-policy surfaces against the full Roko/Nunchi docs corpus. It calls out
the main rigidity points: compiled `AgentRole` semantics, static role identity
strings, separate compose/serve prompt systems, Mori-era live prompt layout
leaks, orchestrator-centric context injection, partial VCG/active-inference
wiring, missing policy manifests, and dashboard projections that do not yet
expose prompt/context policy decisions. It proposes the target move to
`RoleProfile`, `AgentBlueprint`, `PromptPolicy`, `ContextPolicy`,
`CognitiveWorkspace`, context bidders, capability leases, HDC-backed role
morphology, and first-class policy update ledgers.

## How to read each plan

Every plan follows this structure:

1. **Goal** -- one sentence describing the end state.
2. **Current state** -- what exists today, with file paths.
3. **Tasks** -- numbered checklist. Each task has:
   - Source files to read (absolute paths)
   - Target files to create or modify (absolute paths)
   - API contract (request/response shapes)
   - Acceptance criteria (testable, not subjective)
4. **Dependencies** -- which plans must ship first.

## Execution standard for Codex agents

Every checklist item in these plans must be treated as an implementation
contract, not a suggestion. A Codex agent executing a task with no previous
context must do the following before editing code:

1. Read the plan task and every `Read` source listed in it.
2. Run local discovery commands instead of relying on stale assumptions:
   `rg --files`, `rg -n`, `cargo metadata`, route inventories, and existing
   tests for the relevant crates.
3. Prefer extending existing modules over creating parallel implementations.
   If the plan names a file that already has equivalent behavior, add the
   missing behavior, tests, route wiring, or runtime integration there.
4. Produce or update an executable acceptance gate. A task is not done when
   code compiles; it is done when the task-specific gate proves the documented
   end-to-end behavior.
5. Record doc coverage in `.roko/parity/docs-ledger.json` once Plan 07 exists.
   A source doc can only be `implemented_by`, `covered_by`, `deferred`, or
   `reference_only` under the rules in Plan 07.
6. Leave no silent placeholders. Any intentional stub must have a ledger row,
   owner, dependency, deferred reason, and future acceptance command.

If a task uncovers contradictory docs, the newer `tmp/architecture` docs win
for runtime shape, but older `docs/` requirements still need one of:
implementation, explicit compatibility mapping, or a documented deferral with a
gate. Do not delete or ignore old requirements without adding ledger evidence.

## Key service URLs (local dev)

| Service | URL | Notes |
|---------|-----|-------|
| mirage-rs (devnet + relay) | `http://127.0.0.1:8545` | Always running |
| Relay API | `http://127.0.0.1:8545/relay` | Agent registry, events |
| roko-serve | `http://127.0.0.1:6677` | Optional workspace backend |
| Dashboard dev server | `http://localhost:5173` | Vite |
| Agent sidecar (example) | `http://127.0.0.1:8081` | Per-agent, port varies |

## Key env vars (dashboard)

| Var | Default | Purpose |
|-----|---------|---------|
| `VITE_CHAIN_URL` | `http://127.0.0.1:8545` | mirage-rs base URL |
| `VITE_ROKO_URL` | same as CHAIN_URL | roko-serve base URL |
| `VITE_ROKO_API_URL` | (none) | explicit roko-serve API URL |
| `VITE_ROKO_WS_URL` | (none) | explicit roko-serve WS URL |
