# Residual Audit Tracker

Source: `tmp/subsystem-audits/05-01/43-remaining-issues.md`, dated 2026-05-01. This file is a continuation tracker from a worktree/branch-specific audit, not current code truth by itself. The unique value is the issue IDs and the architectural extraction sequence.

> Status-quo audit · re-verified against code at HEAD `5852c93c05` on 2026-07-08. Key update: the foundation-contract types this tracker treated as "fragments" are now **substantially materialized in code** (47 files reference them). The residual work has shifted from "invent the types" to "consolidate + adopt on the live path + retire the 931KB legacy `orchestrate.rs`."

## Foundation-contract adoption status (the core of this tracker)

The 05-01 audit is the root of the foundation-contract work. Re-verifying each named type against the current tree:

| Contract type | Exists? | Where (evidence) | Live-path adoption |
|---|---|---|---|
| `DispatchPlan` | **Yes** | `crates/roko-core/src/dispatch_plan.rs`; also `roko-core/src/foundation.rs` | Referenced across 47 files incl. `roko-cli/src/dispatch_v2.rs`, `dispatch/{mod,factory,model_routing}.rs` |
| `RunLedger` | **Yes** | `crates/roko-runtime/src/run_ledger.rs` | Used by `effect_driver.rs`, `workflow_engine.rs` |
| `DispatchResolver` | **Yes (fragmented)** | `roko-cli/src/dispatch/` tree | Present in dispatch factory path |
| `ModelCallService` | **Yes** | `crates/roko-agent/src/model_call_service.rs` | Referenced by serve routes (`providers.rs`, `gateway.rs`), learn, cli dispatch |
| `RoutingContext` | **Yes** | `roko-learn/src/{cascade_router,model_router,routing_extras}.rs` | Wired into cascade/model routing + runtime feedback |
| `GateStatus` / `CommitOutcome` | **Partial** | gate/runtime paths | Re-verify against `roko-gate` + `roko-runtime/effect_driver.rs` |

**Net**: the greenfield-invention phase is done. The tracker's closing line ("remaining work is centralization and live-path adoption, not greenfield invention") is now the *whole* story — the types exist; the split is between `dispatch_v2.rs` (new) and the still-present 931KB `orchestrate.rs` (legacy).

## Carry-Forward Items (re-verified)

| Item | Source claim | Current disposition | Action |
|---|---|---|---|
| T5-35 | Extract `dispatch_agent_with` into `select_dispatch_model`, `build_dispatch_prompt`, `launch_dispatched_agent`, `record_dispatch_outcome` | **Still relevant.** `dispatch_agent_with` still lives ONLY in `crates/roko-cli/src/orchestrate.rs` (931,748 bytes, unchanged since Jul 2). The new `dispatch_v2.rs` did not absorb it. | Decide keep/shrink strategy for `orchestrate.rs` FIRST; the split now is dispatch_v2 vs orchestrate.rs. |
| T5-36 | Migrate serve route LLM dispatch to `ModelCallService` | **Advanced.** `ModelCallService` now exists and is referenced by `roko-serve/src/routes/{providers,gateway}.rs`. Keep closed unless new raw-`reqwest` LLM calls appear in handlers. | Spot-check route handlers for direct provider HTTP calls. |
| T5-38 | Collapse config into validated model | Still relevant. `ValidatedConfig`/provenance concepts exist, but loader/CLI/schema provenance remains split (the "twenty config loaders" finding). | Fold into config migration work; see [61-CONFIG-ENV-MATRIX.md](61-CONFIG-ENV-MATRIX.md). |
| T5-40 | RunLedger migration | **Materialized** — `roko-runtime/src/run_ledger.rs` exists. Contract consolidation is the residual (used by effect_driver + workflow_engine; is orchestrate.rs still on a parallel ledger?). | Resolve in [47-FOUNDATION-TYPES-REDESIGN.md](47-FOUNDATION-TYPES-REDESIGN.md); confirm single ledger writer. |
| T5-42 | Typed-message blocker | Needs re-verification against ACP/agent provider message types (roko-acp ContentBlock now has Image/resource_link variants — the blocker may have moved). | Add to ACP/provider parity tests. |
| T5-41 | Demo automation | Superseded by frontend route/E2E parity work. | Keep only if current demo-app test plan adopts it. |
| 17 pre-existing failures | Old branch had failures across CLI/core/serve | Historical baseline, not current pass/fail. | Do not quote as current without rerunning. Use [71-CI-RELEASE-PROOF-GAPS.md](71-CI-RELEASE-PROOF-GAPS.md). |

## Residual unknowns still open (P0/P1)

These are the audit unknowns that code inspection could NOT close and that carry real risk:

1. **[P0] Which dispatch path actually runs?** `dispatch_v2.rs` (new foundation-contract path) coexists with the 931KB `orchestrate.rs` (legacy, holds the only `dispatch_agent_with`). Runtime entry points may route through either. Until `orchestrate.rs` is shrunk/retired, "foundation contracts adopted" is only half true. Verify: trace `roko do`/`plan run` from `main.rs` to the actual dispatch call site.
2. **[P0] Single RunLedger writer?** `RunLedger` exists in roko-runtime, but does the legacy path write a parallel ledger? Two writers = split truth (mirrors the double-`episodes.jsonl`-write class of bug from mega-parity). Verify: grep all `RunLedger::` write sites + any `orchestrate.rs` ledger emission.
3. **[P1] GateStatus/CommitOutcome centralization** — these are the least-materialized of the six contracts. Verify gate verdicts flow through one status type into the ledger, not string interpolation.
4. **[P1] ModelCallService is the sole LLM egress?** Confirm no route handler or agent backend bypasses it with direct `reqwest`/provider SDK calls (T5-36 was "audit-only closed" but the audit predates current route growth).

## Principles Worth Keeping (unchanged — still the doctrine)

- One backlog item per commit.
- No drive-by formatting.
- Reuse `ModelCallService` and `DispatchResolver`.
- Skeletons do not count as migrations. (Directly relevant: `TaskExecutorCell` and the 7 PassthroughCells are skeletons counted as "shipped" elsewhere.)
- Missing usage/cost/context must stay unknown, not zero.
- Missing config should be restrictive.
- Structured serializers over string-interpolated payloads.

## Cross-cutting drift for the navigation layer

- CLAUDE.md still names `orchestrate.rs` as the **Wired** orchestration home ("Plan discovery + DAG executor → Wired → orchestrate.rs"). Reality: `orchestrate.rs` is a 931KB legacy monolith that the foundation-contract migration is trying to *retire*; live work runs through Runner v2 (`roko-cli/src/runner/`) + `dispatch_v2.rs`. Nav docs should mark `orchestrate.rs` as legacy/being-decomposed, not the canonical executor.
- The foundation types (`DispatchPlan`, `RunLedger`, `ModelCallService`, `RoutingContext`) are undocumented in CLAUDE.md's crate table despite being the central contract layer. This is a genuine documentation gap for anyone navigating the dispatch path.
