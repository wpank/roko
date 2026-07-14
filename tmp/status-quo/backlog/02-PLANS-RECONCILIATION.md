# 02 — Plans ↔ Status-Quo Reconciliation

> **Current-control-plane notice (2026-07-14, CTRL-15):** This document's body is
> the July 9 baseline analysis at `5852c93c05`; its claims that
> `architecture-core-queue` is missing and that the generated index contains
> only 29 plans/120 tasks are superseded. The reviewed recovery now includes the
> 24-task architecture queue, so a current generated `plans/INDEX.md` reports
> 30 executable plans/144 tasks. The original sealed 120-task population is
> reconciled row-for-row in
> [`plans/_meta/EXECUTION-OWNERSHIP.md`](../../../plans/_meta/EXECUTION-OWNERSHIP.md):
> 99 retained owners and 21 zero-write acceptance roll-ups. Historical analysis
> below is preserved for provenance and must not be used as current inventory.
>
> **The bridge between the authored `plans/` backlog and the status-quo findings.**
> - Repo HEAD: `5852c93c05` on `main`
> - Date: 2026-07-09
> - Plans authored: all `2026-05-08` (P08–P34) / `2026-05-06..08` (side queues)
> - Status-quo audit HEAD: `5852c93c05` — **the same HEAD as now.** Every file:line
>   citation in docs `24`, `27`, `95`, `96–106` reflects *current* code. This is the
>   decisive currency fact: where the audit says an issue is open, the authored plan that
>   targets it is **still CURRENT** (unfixed) even though it was written two months ago.
> - Sources: [24-OPEN-ISSUE-LEDGER](../24-OPEN-ISSUE-LEDGER.md),
>   [27-IMPLEMENTATION-BACKLOG](../27-IMPLEMENTATION-BACKLOG.md),
>   [95-ENGINE-DRIFT](../95-ENGINE-DRIFT.md),
>   [103-DUPLICATE-TYPES-CENSUS](../103-DUPLICATE-TYPES-CENSUS.md),
>   [104-DEAD-CODE-AND-FACADE-CENSUS](../104-DEAD-CODE-AND-FACADE-CENSUS.md),
>   traces [96–102](../96-TRACE-RUNNER-V2-EXECUTION.md).

---

## 0. Executive summary

- **31 plan directories on disk**, 29 executable + 2 superseded (`self-dev-ux` 55 tasks,
  `self-dev-extras` 11 tasks). **Executable total: 120 tasks** (per `plans/INDEX.md`), plus
  66 superseded tasks = **186 authored tasks total**.
- **A 32nd plan, `architecture-core-queue` (Q01–Q20, ~20 packets), is referenced everywhere
  but MISSING from the main `plans/` tree** — it exists only inside `.claude/worktrees/*/plans/`.
  `_meta/IMPLEMENTATION_ORDER.md` names it as "the larger architecture queue" and
  `architecture-defi-critical-path/tasks.toml` hard-depends on its `Q14` chain foundation.
  **This is a broken dependency: the DeFi plan can never satisfy its prerequisite from the
  committed tree.** See §1.1.
- Currency verdict: the P08–P34 queue was written *against an earlier snapshot of the same
  problems the audit re-confirmed at this HEAD*, so it is **overwhelmingly still-open**, but
  it is a **UX/repair queue, not a structural-fix queue**. It barely touches the deep
  P0/P1 findings (no-DAG, adaptive-gate-not-live, storage split-brain, duplicate types,
  dead-code island, auth perimeter, prompt bypass).
- **Coverage gap: of ~44 P0/P1 findings, only 4 are fully addressed by an existing plan and
  ~6 partially; ~34 have NO plan.** Those 34 are the new-epic backlog (§3).

---

## 1. Plan inventory (every dir)

`[meta].status` is `ready` for all P08–P34 + the three side queues; `superseded` for the
two self-dev plans. "Currency" is spot-checked against HEAD `5852c93c05`.

| Plan | Status | #Tasks | Scope (one line) | Status-quo finding it addresses | Currency |
|---|---|---|---|---|---|
| **P08-search-command-fix** | ready | 4 | Rewrite Perplexity client to single-query body; drop `date_range` | **P0** "research search 100% broken" (batch body → HTTP 422) [24/27] | **CURRENT** — fully on-target |
| **P09-tool-alias-fix** | ready | 3 | Normalize Claude tool aliases → canonical in `parse_allowed_tools_csv` | **P1** "Tool-alias bug strips tools on non-Claude" (`openai_compat.rs:252`) [24/27] | **CURRENT** — `parse_allowed_tools_csv` confirmed still present, unnormalized |
| **P10-slash-command-flags** | ready | 5 | Fix `/plan-resume`→`--resume-plan`, `--model` passthrough, register `/develop` | ACP UX; partial overlap w/ P0 resume-flag semantics; `/develop` (dup P29) | **CURRENT** — `/develop` absent from `session.rs` (confirmed) |
| **P11-runner-v2-default** | ready | 5 | Make RunnerV2 the default engine; TOML validation post-generate | **P0** "Default plan run is a dry-run graph" [24/27/95] | **CURRENT** — `main.rs:1361 default_value="graph"` confirmed unchanged. **Does NOT fix `roko resume` (separate P0)** |
| **P12-runner-parallelism** | ready | 5 | Read `max_parallel`; track N tasks/handles per plan; dispatch N ready tasks/tick | **P1** "No live task DAG; intra-plan parallelism does not exist" [24/96] | **CURRENT but SHALLOW** — patches the per-plan FSM; audit's real fix is a live `UnifiedTaskDag` (dead today). Refresh needed |
| **P13-rate-limit-retry** | ready | 4 | Classify HTTP errors (429/5xx) in all send/stream paths | **P1** "429-no-retry" [99-TRACE] | **CURRENT** |
| **P14-gate-rung-fix** | ready | 3 | Push concrete gate steps for advanced rungs; upgrade activation log | **P1** "Live gate path shallow — adaptive gates NOT live; rungs 3-6 stub-pass" [24/101] | **CURRENT but SHALLOW** — confirmed `gate_dispatch.rs:104` still `RungExecutionInputs::default()`, never calls `enrich_rung_config`. Plan pushes rung *steps* but does NOT port enrichment/oracles/EMA-per-rung. Refresh needed |
| **P15-error-recovery-wiring** | ready | 5 | Wire `classify_agent_crash` into runner-v2 + `do_cmd.rs`; crash_class ledger | **P1** "Runner v2 legacy holdouts" (partial — recovery only) [24/96] | **CURRENT** — but conductor loop / tasks.toml-replan / worktree holdouts untouched |
| **P16-safety-contracts** | ready | 5 | `disallowed_tools` on dispatch req + `AgentContract.forbidden_tool_names()` | **P0** "safety funnel bypassed on default provider" (partial) + **P1** safety [24/99] | **CURRENT but PARTIAL** — adds contract-driven denial list; does NOT close the Claude-CLI subprocess per-tool bypass nor promote post-checks to Block |
| **P17-cli-output-format** | ready | 6 | `CliOutput` wrapper; replace `eprintln!`; downgrade dup-slug warnings | Cosmetic UX; not a P0/P1 finding | **CURRENT (low-value)** — no audit evidence either way |
| **P18-tui-agent-data** | ready | 5 | Fix agent_id match; publish Efficiency/Diagnosis events to TUI bridge | Touches "dashboard emptiness" symptom but NOT the root cause | **CURRENT but SHALLOW** — root cause is TUI file-scraping its own `DashboardSnapshot` (103 row 1), which this doesn't fix |
| **P19-cascade-router-acp** | ready | 6 | `cascade_select_model()` in ACP bridge; load DaimonState; record decision | Routing UX; ≠ **P1** "CascadeRouter LinUCB never persisted" (that's persistence) | **CURRENT** — orthogonal to the persistence bug, which stays open |
| **P20-zero-config** | ready | 5 | Consult builtin registry in preflight/ACP; suppress false dup-slug warnings | Zero-config model UX; not a P0/P1 | **CURRENT (UX)** |
| **P21-acp-streaming** | ready | 5 | Stream stdout/stderr lines in `run_slash_command`; wire `AcpProgressSink` | ACP streaming UX | **CURRENT (UX)** |
| **P22-acp-tool-permission** | ready | 5 | Real `ToolContext` + `denied_tools` check in `AcpBuiltinToolHandler` | **P1** "ACP permission gate has zero prod callers" [24/27/75] | **CURRENT** — confirmed `request_permission` only in `types.rs`/`bridge_events.rs`, not called by `builtin_tools.rs`. On-target |
| **P23-prd-pipeline-fix** | ready | 6 | PRD draft agent read-only tools; block-on-invalid; link plans↔PRDs by slug | Self-hosting PRD loop UX [98-TRACE] | **CURRENT (UX)** |
| **P24-workspace-paths** | ready | 4 | Prefer top-level `plans/`; doctor orphan-tmp + plans-conflict checks | **P0** "docs teach stale paths" (partial) + doctor UX | **CURRENT (UX)** |
| **P25-mcp-acp-passthrough** | ready | 4 | `mcp_config` on core AgentConfig; wire into ACP session + tool-loop | MCP passthrough for the **ACP** path (CLAUDE only claims orchestrate path wired) | **CURRENT** — ACP path still needs it |
| **P26-hdc-similarity-lookup** | ready | 4 | `query_similar_episodes`; **query before dispatch in `orchestrate.rs`** | **P2** "HDC compiled out"; per-episode retrieval | **STALE (misaimed)** — T3 wires into `orchestrate.rs`, which is **dead-by-default** (`legacy-orchestrate` OFF). Must re-target `runner/event_loop.rs`. `query_similar_episodes` confirmed absent |
| **P27-provider-error-ux** | ready | 4 | Provider-agnostic auth errors + doctor detected-providers summary | Provider UX; not a P0/P1 | **CURRENT (UX)** |
| **P28-image-support** | ready | 5 | ACP image capability from model vision; image injection both dispatch paths | Vision feature (dup of superseded `self-dev-ux` H01/H02) | **CURRENT (feature)** |
| **P29-develop-command-wire** | ready | 3 | Register `/develop` slash command + dispatch mapping | **Duplicate of P10 T3/T4** | **CURRENT (redundant)** — fold into P10 |
| **P30-onboarding-doctor** | ready | 4 | OpenAI/Gemini key checks; per-provider validation; init/setup hints | Onboarding UX | **CURRENT (UX)** |
| **P31-note-and-context** | ready | 3 | Route bare `roko plan <args>`→generate; note-cluster; `--from-notes` | Plan-gen UX (dup of `self-dev-extras` D05/D06) | **CURRENT (UX)** |
| **P32-cli-polish** | ready | 2 | `skip_serializing_if` on ModelProfile bools; swap hourglass emoji | Cosmetic | **CURRENT (trivial)** — likely still applicable |
| **P33-model-ux** | ready | 1 | `max_tokens` auto-recovery retry in `CodexAgent::run` | Provider UX (dup `self-dev-extras` D07) | **CURRENT (UX)** |
| **P34-verification-sweep** | ready | 4 | cargo check / clippy / test / release-binary smoke | Meta-verification gate | **CURRENT (always applies)** — run last |
| **architecture-defi-critical-path** | ready | 3 | Chain registry indexer, serve registry/passport routes, DeFi verify | **P1** "Chain/ISFR integration split"; Phase 2+ | **BLOCKED** — hard-depends on missing `architecture-core-queue#Q14` (see §1.1). Chain is Phase 2+ regardless |
| **e2e-smoke** | ready | 2 | `#[must_use]` on `generate_share_token` + unit test | Trivial smoke exemplar | **CURRENT (trivial)** — possibly already done; verify |
| **self-dev-extras** | superseded | 11 | Provider auto-detect, doctor, PRD validate, plan shorthand, max_tokens… | Consolidated into P08–P34 | **SUPERSEDED** — do not run |
| **self-dev-ux** | superseded | 55 | ACP tools, cascade guard, image blocks, distillation/efficiency/dream hooks, builtin registry | Consolidated into P08–P34 | **SUPERSEDED** — but note several H-tasks (H03 distillation, H05 cascade-in-ACP, H06 auto-dream) were **only partially** re-homed; audit-open items may hide here |

### 1.1 The missing `architecture-core-queue` (Q01–Q20)

- **Referenced by:** `_meta/IMPLEMENTATION_ORDER.md` (§Separate Queues) and three `source_ref`
  lines in `architecture-defi-critical-path/tasks.toml` (`#Q14-chain-registries-defi-foundation`).
- **Present only in worktrees:** `.claude/worktrees/agent-{aefd7c48,aad01731,ab986004}/plans/architecture-core-queue/{tasks.toml,plan.md}`.
- **Contents (from worktree copy):** 20 architecture packets, `queue_kind = "architecture_implementation"`,
  handoff for `tmp/architecture-plans/06-architecture-implementation.md`:
  - Q01 config-schema-contracts · Q02 role/prompt/context/workspace contracts ·
    Q03 agent-runtime-lifecycle · Q04 tick-pipeline/clock/cortical · Q05 extension-hooks-loader ·
    Q06 connectors/feeds/recipes · Q07 relay-envelopes/rooms/replay · Q08 gateway-request-pipeline ·
    Q09 model-routing/auth/secrets · Q10 knowledge-lifecycle/A-MAC · Q11 learning-feedback/neuro ·
    Q12 pheromones/dreams · Q13 evals/arenas/leaderboard-gates · Q14 chain-registries/DeFi-foundation ·
    Q15 groups/coordination · Q16 dashboard-projections · Q17 visual-composition/authoring ·
    Q18 meta-lineage/recursive-safety · Q19 chain-contract-deployment-deferral · Q20 agent-job-marketplace-economy-deferral.
- **Notably, Q01/Q02/Q09/Q11/Q16 map directly onto the deep P0/P1 findings** (foundation
  contracts, auth/secrets, learning feedback, dashboard projections). So the structural fixes
  the new epics need were *partly designed already* — but the queue is **not in the runnable
  tree**. **Action: recover it from a worktree and re-commit under `plans/`, or supersede it
  explicitly and lift Q14 into `architecture-defi-critical-path`.**

---

## 2. Currency roll-up

| Tag | Meaning | Count | Plans |
|---|---|---|---|
| **CURRENT (on-target)** | Issue confirmed open at HEAD; plan aims correctly | 6 | P08, P09, P11, P13, P22, P25 |
| **CURRENT but SHALLOW/PARTIAL** | Issue open, but plan patches the symptom, not the audit's root fix | 5 | P12, P14, P15, P16, P18 |
| **CURRENT (UX/feature/cosmetic)** | Real but low-severity; not a P0/P1 finding | 14 | P10, P17, P19, P20, P21, P23, P24, P27, P28, P30, P31, P32, P33, P34 |
| **CURRENT (redundant)** | Duplicates another plan | 1 | P29 (⊂ P10) |
| **STALE (misaimed)** | Targets the dead-by-default `orchestrate.rs` path | 1 | P26 |
| **BLOCKED** | Depends on the missing `architecture-core-queue` | 1 | architecture-defi-critical-path |
| **CURRENT (trivial)** | Tiny; likely done or one-commit | 1 | e2e-smoke |
| **SUPERSEDED** | Do not run | 2 | self-dev-ux, self-dev-extras |

**No plan is confidently LIKELY-DONE** — because the audit re-verified these exact bugs at the
*current* HEAD, the repair queue is essentially 100% unstarted. Only `e2e-smoke`/`P32` are
trivial enough to possibly be incidentally satisfied (verify before running). Net: **~29
executable plans, ~11 STALE-or-shallow-or-blocked**, the remainder open-and-current but
UX-weighted.

---

## 3. Coverage-gap matrix — P0/P1 findings with NO plan (the new epics)

Legend: ✅ has a fully-aimed plan · 🟡 partial/shallow plan · ❌ **no plan → NEW EPIC**.

### 3.1 P0 (from [24-OPEN-ISSUE-LEDGER](../24-OPEN-ISSUE-LEDGER.md))

| P0 finding | Plan? | New epic needed |
|---|---|---|
| Default plan run is a dry-run graph | ✅ P11 | — (verify P11 also flips `main.rs:1361`) |
| `roko resume` ignores snapshots (hardcodes Graph) | ❌ | **E-RESUME**: route `roko resume`→RunnerV2 auto-resume |
| Relay proxy fully unauthenticated (`/relay/*` outside `/api`) | ❌ | **E-PERIMETER** |
| Read-scope auth fallback authorizes writes (`middleware.rs:385`) | ❌ | **E-PERIMETER** |
| Per-tool safety funnel bypassed on default Claude-CLI provider | 🟡 P16 | **E-SAFETY-UNIVERSAL** (subprocess loop bypass unclosed) |
| `research search` 100% broken | ✅ P08 | — |
| Runtime source-of-truth ambiguous (v1/v2/graph/workflow) | ❌ | **E-ENGINE-DECISION** (decision doc + collapse) |
| Foundation contracts fragmented (DispatchPlan/RunLedger/GateStatus/RoutingContext) | ❌ | **E-CONTRACTS** (⊃ 103 dup-types) |
| Docs teach unsafe/stale commands | ❌ | **E-DOCS-TRUTH** |
| Demo/API hard breaks: 4 frontend→serve 404s + camelCase drift | ❌ | **E-FRONTEND-CONTRACT** |
| Storage divergence: signals vs engrams, `executor.json` vs `state-snapshot.json`, 44 MB events firehose | ❌ | **E-STORAGE-CONVERGE** |
| Source docs lack status/provenance tags | ❌ | **E-DOC-PROVENANCE** |
| Ops docs overstate deployment readiness | ❌ | **E-OPS-PROOF** |
| Maintained root docs (README/CLAUDE) stale | ❌ | **E-DOCS-TRUTH** |

**P0 with no dedicated plan: 11 of 14** (only default-run + research-search covered; safety partial).

### 3.2 P1 (selected structural — full list in [24](../24-OPEN-ISSUE-LEDGER.md))

| P1 finding | Plan? | New epic |
|---|---|---|
| Graph Engine lacks parity (no live dispatch/gates/resume) | 🟡 P11/P12 | **E-ENGINE-DECISION / E-GRAPH-PARITY** |
| ACP permission gate zero callers | ✅ P22 | — |
| Tool-alias casing strips tools on non-Claude | ✅ P09 | — |
| Safety post-checks are Warn-only (SecretLeak/PathEscape don't Block) | ❌ | **E-SAFETY-UNIVERSAL** |
| Custody verify is false audit assurance | ❌ | **E-CUSTODY** |
| Cold-substrate archival copies-not-moves (unbounded) | ❌ | **E-STORAGE-CONVERGE** |
| `config show --effective` prints secrets unredacted | ❌ | **E-PERIMETER** |
| Worker callback has no auth header | ❌ | **E-PERIMETER** |
| Runner v2 holdouts: conductor unwired, replan prompt-only, worktree isolation unwired | 🟡 P15 | **E-RUNNER-HOLDOUTS** |
| No live task DAG; intra-plan parallelism absent (`UnifiedTaskDag` dead) | 🟡 P12 | **E-LIVE-DAG** |
| Live gate path shallow; adaptive gates NOT live (no `enrich_rung_config`, rungs 3-6 stub-pass) | 🟡 P14 | **E-GATE-ADAPTIVE-LIVE** |
| Gate stubs can look like passes (inflate EMA) | 🟡 P14 | **E-GATE-ADAPTIVE-LIVE** |
| CascadeRouter LinUCB state never persisted (dual writers) | ❌ | **E-LEARNING-DURABLE** |
| `events.jsonl` write-only firehose (97% feed_tick) nothing reads | ❌ | **E-STORAGE-CONVERGE** |
| Builtin tool count vs handlers (37 defs / 16 handlers) | ❌ | **E-TOOL-REGISTRY** |
| Live prompt path bypasses canonical builder (`PromptAssembler` ≠ `SystemPromptBuilder`/VCG) | ❌ | **E-PROMPT-UNIFY** (103 row 12) |
| Episode roots split (root/learn/memory logs) | ❌ | **E-STORAGE-CONVERGE** |
| Event model split (4 EventBus + StateHub + DashboardEvent…) | ❌ | **E-EVENT-TAXONOMY** (103 row 8) |
| Server auth scopes under-protect routes | ❌ | **E-PERIMETER** |
| Learning feedback loses fidelity (default model source, zeroed totals) | ❌ | **E-LEARNING-DURABLE** |
| Frontend DataHub migration incomplete | ❌ | **E-FRONTEND-CONTRACT** |
| Crate boundaries drift (`roko-runtime → roko-gate` layering violation) | ❌ | **E-LAYERING** |
| CI/release gates under-scoped | ❌ | **E-CI-PROOF** |
| (+ API-docs partial, provider/tool dispatch not universal, chain/ISFR split, data contracts hand-maintained, examples status, env-var ownership) | ❌ | E-DOCS-TRUTH / E-TOOL-REGISTRY / E-CHAIN-AUTHORITY / E-CONTRACTS |

**P1 with no dedicated plan: ~24 of ~30** (only ACP-permission + tool-alias fully covered).

### 3.3 Census-derived structural epics (103 / 104 — zero plans)

| Finding | Epic |
|---|---|
| Duplicate types: GateVerdict×4, DashboardSnapshot×3, StateHub×2, RetentionPolicy×3, AgentState×4, TaskStatus×3, GateFeedback×3, EventBus×4, Cell×2, PromptAssembler×2 | **E-CONTRACTS** + **E-EVENT-TAXONOMY** + **E-PROMPT-UNIFY** + **E-DASHBOARD-UNIFY** |
| Dead-code: `legacy-runner-v2` façade, orphan `state_hub.rs`/`pulse_bus.rs`, ~52 K-LOC `legacy-orchestrate` island, `roko-orchestrator/safety` 3.4 K dead dup | **E-DEAD-CODE-PURGE** |
| Demurrage income dead (taxes-only), conductor unwired, 6/17 hooks fire | **E-RUNNER-HOLDOUTS** / **E-ECONOMY** |
| Observability: TUI scrapes files vs core snapshot (dashboard emptiness) | **E-DASHBOARD-UNIFY** |

---

## 4. Recommendations

### Keep as-is (on-target, run in order)
`P08`, `P09`, `P11`, `P13`, `P22`, `P25` — plus `P34` last. Verify P11 actually flips the
clap `default_value` (currently still `"graph"`).

### Refresh before running (shallow vs the audit's root fix)
- **P12** → widen to a real `UnifiedTaskDag` schedule (feed **E-LIVE-DAG**), not just N-per-plan.
- **P14** → add `enrich_rung_config` port + neutral/Skipped stubs + EMA-per-real-rung
  (feed **E-GATE-ADAPTIVE-LIVE**); current tasks only push rung *steps*.
- **P15** → extend beyond crash-classify to the conductor/replan/worktree holdouts
  (feed **E-RUNNER-HOLDOUTS**).
- **P16** → close the Claude-CLI subprocess per-tool bypass + promote post-checks to Block
  (feed **E-SAFETY-UNIVERSAL**).
- **P18** → make TUI consume `roko-core::DashboardSnapshot` (feed **E-DASHBOARD-UNIFY**).

### Re-target / supersede
- **P26** → **re-target from `orchestrate.rs` to `runner/event_loop.rs`** (STALE misaim), or
  fold into **E-LEARNING-DURABLE**.
- **P29** → **fold into P10** (duplicate `/develop` wiring).
- **`architecture-core-queue`** → **recover from worktree and re-commit under `plans/`**, or
  formally supersede and re-home Q14 into `architecture-defi-critical-path` (currently BLOCKED).
- **self-dev-ux / self-dev-extras** → keep superseded, but **grep H03/H05/H06 items** — some
  hooks were only partially re-homed and may still be open.

### NEW plans/epics required (the ~34-finding gap)
Ordered by "can the system lie to users / expose it" per doc 27:

1. **E-PERIMETER** — relay auth, deny-by-default scope fallback, config-secret redaction,
   worker callback token. *(P0 ×2 + P1 ×3)*
2. **E-ENGINE-DECISION** — pick the production engine; resume routing (**E-RESUME**); collapse
   docs. *(P0 ×3)*
3. **E-STORAGE-CONVERGE** — signals↔engrams, `state-snapshot.json` reader, events-firehose
   cap, episode-root unification, cold-substrate move-not-copy. *(P0 ×1 + P1 ×4)*
4. **E-GATE-ADAPTIVE-LIVE** — enrichment/oracles/EMA/thresholds on the live path.
5. **E-LIVE-DAG** — real intra-plan task DAG + parallelism.
6. **E-SAFETY-UNIVERSAL** — provider-agnostic pre-check + Block-severity post-checks.
7. **E-CONTRACTS** — one foundation-type layer (GateVerdict/DispatchPlan/RunLedger/RoutingContext).
8. **E-PROMPT-UNIFY** — one prompt-assembly surface (kill the `PromptAssembler` split).
9. **E-EVENT-TAXONOMY** / **E-DASHBOARD-UNIFY** — one EventBus + one DashboardSnapshot.
10. **E-DEAD-CODE-PURGE** — façade + orphans + ~52 K legacy island + layering violation
    (**E-LAYERING**).
11. **E-CUSTODY**, **E-TOOL-REGISTRY**, **E-LEARNING-DURABLE**, **E-RUNNER-HOLDOUTS**,
    **E-FRONTEND-CONTRACT**, **E-DOCS-TRUTH**, **E-DOC-PROVENANCE**, **E-OPS-PROOF**,
    **E-CI-PROOF**, **E-CHAIN-AUTHORITY**, **E-ECONOMY**.

Every new epic must close on a proof gate (doc 25 / 64), per doc 27's rule: *a type/route
existing is not "done" — the default user path must exercise it.*
