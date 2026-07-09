# Tmp Source Ranking

Decides which `tmp/` material should influence implementation, and how much to trust it.
**Reconciliation pass 2026-07-08** against git HEAD `5852c93c05`. Tiers below reflect *verified*
authority, not just recency.

Ranking axes: **Authoritative** (design intent to act on) · **Current** (post-dates the docs/v2
freeze 2026-05-05 / v2-depth 2026-05-08) · **Stale** (superseded or overtaken by code) ·
**Scratch** (artifacts/archives, not design). A source can be authoritative-but-partly-stale.

## Tier 1 — Highest Authority (act on these)

| Source | Date | Authority | Verified status | Use as |
|---|---|---|---|---|
| `tmp/tmp-feedback/2/` | **2026-05-08** | Authoritative + Current | Reconciled: 8 FIXED, 11 PARTIAL, 14 STILL-OPEN (see `67-TMP-FEEDBACK-2-CROSSWALK.md`). | Primary roadmap/defect input; feed still-open P0/P1 into `.roko/GAPS.md`. |
| `tmp/solutions/REVISED-BEST-SOLUTION-AFTER-DEMO.md` | 2026-04-28 | Authoritative | Chosen path (M0-0 execution-contract repair first) — still unadopted; the doc-08/21 stub confirms M0-0 not done. | Roadmap ordering: truthfulness rails before features. |
| `tmp/subsystem-audits/05-01/` | 2026-05-01 | Authoritative + more specific than docs/v2 | Foundation types (`DispatchPlan`/`RunLedger`/`GateStatus`/`CommitOutcome`/`RoutingContext`) exist but fragmented across crates; contract-unification NOT done. | Roadmap input for the Wave 0–5 contract consolidation. |
| `tmp/doc-convergence/` | 2026-04-30/05-01 | Authoritative (process) | **Never executed** — all phases PENDING, `output/` only `.gitkeep`. | The designed fix for exactly this status-quo doc-staleness; run it or retire it. |

## Tier 2 — Implemented / Canonical-elsewhere (reference, don't re-drive)

| Source | Date | Status | Note |
|---|---|---|---|
| `tmp/relay-bus/` | 2026-05-08 | **Implemented**; canonical successor is `docs/v2-depth/12-connectivity/` (committed same day 13:14). | Use tmp only for decision rationale. Residual: `resume_after` still "planned" (`01-relay-wire-protocol.md:283`); no hits in `crates/` — spot-check `apps/agent-relay/src`. |
| `tmp/daiji-tmp/tmp/` | 2026-05-08 | Mixed: relay part implemented; 7-contract + ISFR redesign targets the **daeji** repo (cross-repo, unverifiable here). | Ecosystem roadmap with explicit repo ownership. `roko-relay-current-state.md` is now historical (refuted by the shipped pub/sub bus). |

## Tier 3 — Useful, needs per-item reverification

| Source | Status |
|---|---|
| `tmp/solutions/self-developing/` (May 6) | 23 self-dev UX problems; partly fixed (`ChatAgentSession` at `crates/roko-cli/src/chat_session.rs`), most open. Ledger: `68-SELF-DEVELOPING-CROSSWALK.md`. |
| `tmp/solutions/ide/` (May 4) | 7 ACP/IDE bugs; ACP parity cluster still open (see crosswalk 01/18/19/31/34). |
| `tmp/solutions/perf/` (May 1) | 15 bottlenecks / 18 plans; only B01 verified fixed. |
| `tmp/ux/implementation-plans/` | More current than older `tmp/ux/ux-followup`; use for TUI/frontend plans. |
| `tmp/prds/impl2/` | PRD implementation checklist; older than feedback wave 2. |
| `tmp/taskrunner/` | `implemented=100, wired=0` = not done; automation archaeology. |
| `tmp/unified-migration-runner/` | Runner logs + blocked/pending inventory; automation did NOT complete migration. |
| `tmp/solutions/` top level (`solution-1/2/3`, `A/B/C`, `FINAL-SOLUTION`, `MY-TAKE-SHORTEST-PATH`) | Migration-decision record; superseded by `REVISED-BEST-SOLUTION-AFTER-DEMO.md`. |
| `tmp/subsystem-audits/` per-subsystem dirs (acp/cli-chat-tui/gate-pipeline/…) | AUDIT/GOALS/PLAN triplets; still-valid design input, correct for types that now exist. |

## Tier 4 — Superseded / Archive / Scratch

| Source | Status |
|---|---|
| `tmp/unified/` (`v1-archive`, `v2-archive`) | Ancestor of docs/v2-depth (1:1 numbering). SUPERSEDED — **except `v2-archive/22-COGNITIVE-MAPPING.md`, never absorbed** (see `20-TMP-NEWEST.md`). |
| `tmp/solutions/demo-running/` (May 5) | 46-batch run log; scenario redesign never shipped. SUPERSEDED. |
| `tmp/tmp-folder/` | Dogfood scratch: toy `calculator` workspace + a full sample `.roko/` state. SCRATCH (keep `.roko/` as a regression fixture). Jun 23 dir-mtime = created-then-deleted content (open question). |
| `tmp/research3/` | Korai/ISFR business + chain strategy (April). SCRATCH-ARCHIVE; contextualizes docs/v2 FEEDS/PAYMENTS/MARKETPLACE/DEFI, not the code migration. |
| `tmp/ethereal/` | Single external status-update.md (capability claims). SCRATCH; use as a claims-to-verify checklist. |
| `tmp/screenshots/` | Apr 26 deck + mori UI PNGs. Pure archive. |
| `tmp/demo-uis/v1..v17` | Static demo prototypes, not routed surfaces. Archive. |

## Corrections to carry forward (verified 2026-07-08)

1. **The default `plan run` is a dry-run stub, not "Wired".** `engine="graph"` default →
   `TaskExecutorCell` (`crates/roko-graph/src/cells/task_executor.rs:32,83`) never dispatches an
   agent; live branch is unimplemented. `tmp/tmp-feedback/2` docs 08/21 are STILL-OPEN P0. Do NOT
   trust CLAUDE.md's "Plan discovery + DAG executor: Wired" for the default surface.
2. **Safety finding partly stale, but not fully closed.** Acceptance contracts now validate and can
   reject (`orchestrate.rs:11877,14690`) with bundled-role fail-closed; but per-agent YAML contract
   loading and bash-denylist interception of the Claude-CLI subprocess remain open (doc 24 PARTIAL,
   not FIXED).
3. **05-01 core types are fragmented, not absent.** `DispatchPlan` (roko-core), `RunLedger`/
   `CommitOutcome` (roko-runtime), `GateStatus` (roko-gate + CLI inline), `RoutingContext`
   (roko-learn) exist; the gap is **consolidation into one canonical contract**, not creation.
4. **relay-bus is the success story.** Its 9 decisions folded into `docs/v2-depth/12-connectivity`
   and shipped in `apps/agent-relay/src/bus.rs`. Only residual: `resume_after` (still "planned").
5. **doc-convergence did NOT generate docs/v2-depth.** v2-depth descends from `tmp/unified/`;
   doc-convergence is the *unexecuted* successor plan targeting docs/v3.
6. **`eprintln!` debt grew** from 147 (doc 27) to 299 non-test sites — regression, not progress.

## Ranked designs that must feed the roadmap

1. Default-engine execution honesty (crosswalk 08/21) — **the P0**.
2. ACP parity cluster (crosswalk 01/18/19/31/34) — the largest coherent unbuilt design.
3. `REVISED-BEST-SOLUTION-AFTER-DEMO.md` M0-0→M0-C ordering — adopt in the roadmap doc.
4. `subsystem-audits/05-01` contract consolidation (Wave 1 types) — decide (ADR) then centralize.
5. `doc-convergence/` — run (retargeted at current docs) or formally retire.
6. relay `resume_after` freeze residual; `22-COGNITIVE-MAPPING.md` absorb-or-drop.
