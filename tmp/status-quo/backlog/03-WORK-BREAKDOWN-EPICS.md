# 03 — Work-Breakdown: Master Roadmap (18 Epics)

> **What this doc is:** the synthesis layer over `backlog/epics/E01..E18-*.md`. It fixes the
> cross-epic dependency DAG, assigns each epic to a milestone with entry/exit criteria, names the
> critical path and the parallelizable tracks, and gives a single recommended execution order that
> interleaves the new epics with the pre-existing `plans/` queue (P08–P34 + side queues).
>
> - Repo HEAD: `5852c93c05` on `main` · authored 2026-07-09
> - Inputs: `epics/E01..E18`, `02-PLANS-RECONCILIATION.md`, `04-EXECUTION-READINESS.md`,
>   `references/PLANNING-METHODOLOGY.md`
> - Task-count rule: an epic's `#tasks` is its authored `[meta].total` (executable native tasks).
>   Where an epic *reconciles* pre-existing plans (P08/P09/P16/P19/P22/P25/P28…), those plan tasks
>   are counted under the plans total in §6, not double-counted here.

---

## 1. Epic summary table

| Epic | Title | #tasks | Milestone | Depends-on (epics) | Covered by existing plan(s) | One-line goal |
|---|---|---:|---|---|---|---|
| **E01** | Execution Engine | 10 | **M0** | — (root) | P11 (partial), P12, P15 (adjacent) | Make bare `roko plan run` spawn real agents on Runner v2, honestly + resumably. |
| **E02** | Storage Convergence | 12 | M1 | E03 (soft, GateVerdict shape), E01 (soft) | P24 (adjacent, plans-dir only) | One canonical writer per `.roko/` store that the readers actually read (fix empty dashboards). |
| **E03** | Type Consolidation | 7 | M1 | — | **none** (pure gap) | Canonicalize 5 runtime-critical dup type families + `From` adapters; unblocks E02, E10. |
| **E04** | Security Perimeter | 19 | M0 subset / **M2** full | P16, P22 (substrate) | P16 (partial), P22 (partial) | Close 3 exploitable P0s + safety-funnel enforcement + custody hash-chain. |
| **E05** | Gate Adaptivity Live | 8 | M0 minimum / **M1** full | E01; E05-T08→E02 | P14 (superseded — wrong engine) | Make the live gate path honest: real inputs, skipped≠pass, per-rung EMA persisted. |
| **E06** | Compose / Prompt Unify | 9 | M1 | E01 | **none** (gap) | Route Runner v2 through the canonical 12-slot builder; kill the 4-surface prompt split. |
| **E07** | Learning & Knowledge | 10 | M2 | E01; E07-T09→P19 | P19 (ACP arm), P26 (separate) | Make learning loops durable + closed (LinUCB persist, knowledge income, HDC on). |
| **E08** | Conductor Supervision | 7 | M2 | E01 | **none** (gap) | Wire the built conductor into the live runner: anomaly supervision + real `conductor_load`. |
| **E09** | Observability | 9 | M2 | E01 | **none** (gap) | Thread the built `MetricRegistry` into RunConfig; rotate logs; trim the events firehose. |
| **E10** | Frontend / API Contract | 7 | M2 | **E03**; E01 (T03) | P18 (TUI, different surface) | Fix 4 frontend↔serve 404s, casing drift, double-SSE, replay-drop on the web demo. |
| **E11** | Chain / ISFR | 5 | **M3+** | E11-T01 recovers `architecture-core-queue`; E11-T04⇄E03-T07 | architecture-defi-critical-path (BLOCKED) | Recover the core queue, implement `get_logs`, reach 13-contract deploy parity. |
| **E12** | Dead-Code Cleanup | 9 | M0 subset / **M3+** gated | **E05, E06, E08** (T07); E01/E04 (T06); E03 (T05) | **none** (gap) | Delete the ~52K-LOC legacy island — only after its live value is ported out. |
| **E13** | v2 Spec-Debt (Lens) | 3 | **M3+** | E09-T09 (T01), E01 (T03) | **none** (gap) | Build `trait Lens` + `MetricRegistry` adapter; resolve Cell↔Block naming. Gates nothing. |
| **E14** | Providers & Tools | 7 | M1 | E01 | P13, P09 (=E14 alias), P28 (partial) | Honest dispatch path: retries retry, tools survive, 37 advertised == executable. |
| **E15** | MCP Config & Passthrough | 6 | M1 | — (soft P25) | P25 (partial; E15-T4 supersedes P25-T4) | Make MCP passthrough actually deliver tools (`{"mcpServers":{}}` normalizer + env + parity). |
| **E16** | PRD Self-Hosting | 2 | M1 | E01, E14 (=P09) | P08 + P09 + P23 (13 tasks cover ~90%) | Close the generative front-half loop (idea→draft→research→plan); 2 gap tasks only. |
| **E17** | ACP Completion | 6 | M2 | **E04** (permission), **E07** (learning), **E15** (MCP) | P19/P21/P22/P25/P28 (substrate) | Make an editor ACP turn behave like a `plan run` turn: consent-gated, learning-informed, MCP-equipped, honest. |
| **E18** | Docs, Config, CI & Ops | 13 | M2 | E01 (doc rewrites); own T05–T08 fixes | P17/P20/P30/P32/P33 (adjacent) | Make the repo trustworthy + shippable: fix CI gates, secrets, MSRV, then rewrite the lying docs. |

**Epic task total: 149** (executable native tasks across E01–E18). See §6 for the grand total incl. `plans/`.

---

## 2. Cross-epic dependency DAG

Edges are epic-level "must-precede" relationships distilled from each epic's `depends_on` /
cross-epic notes. `⇄` = coordinate (mutually-exclusive deletion). Dashed = soft/interim-shippable.

```
                                   ┌─────────────────────────┐
                                   │   E01  EXECUTION ENGINE  │  ◀── M0 root
                                   │  (gates ~everything)     │
                                   └───────────┬─────────────┘
        ┌──────────────┬───────────────┬───────┼────────────┬───────────────┬────────────┐
        ▼              ▼               ▼        ▼            ▼               ▼            ▼
   ┌─────────┐   ┌──────────┐    ┌─────────┐ ┌───────┐  ┌────────┐    ┌─────────┐  ┌─────────┐
   │ E05     │   │ E06      │    │ E14     │ │ E15   │  │ E09    │    │ E07     │  │ E16     │
   │ gates   │   │ compose  │    │ prov/   │ │ MCP   │  │ obs    │    │ learn   │  │ PRD     │
   │ live    │   │ unify    │    │ tools   │ │       │  │        │    │         │  │(+E14)   │
   └────┬────┘   └────┬─────┘    └─────────┘ └───┬───┘  └───┬────┘    └────┬────┘  └─────────┘
        │             │                          │          │              │
        │             │        E03 (types) ──────┼──────────┼──────────────┤
        │             │          │   │           │          │              │
        │             │          ▼   └──► E02 (storage) ─────┘              │
        │             │        E10 (frontend, needs E03) ──► (M2)           │
        │             │                                                     │
        │             │       E04 (security P0s; substrate P16/P22)         │
        │             │          │                                          │
        │             │          └───────────┐          ┌──────────────────┘
        │             │                       ▼          ▼          ▼
        │             │                    ┌────────────────────────────┐
        │             │                    │  E17  ACP COMPLETION        │  (needs E04+E07+E15)
        │             │                    └────────────────────────────┘
        │             │
   E08 (conductor) ───┤
        │             │
        └──────┬──────┴───────────────► ┌──────────────────────────────┐
               │  (E05+E06+E08 done)    │ E12  DEAD-CODE CLEANUP        │  gated deletions
               │                        │  T07 delete orchestrate.rs    │
               │   E03 ─► E12-T05 (HDC) │  T06←E01/E04 · T05←E03        │
               │   E04 ─► E12-T06 (safety)  E11-T04 ⇄ E03-T07            │
               │                        └──────────────────────────────┘
               ▼
         E09-T09 ──► E13 (Lens / spec-debt)  ── M3+, gates NOTHING
         E11-T01 recovers architecture-core-queue ──► DeFi critical-path (Phase 2+)
         E18 doc rewrites (T10–T13) ── need E01 + E18's own T05–T08 fixes ── M2
```

**Known gate facts (all confirmed against the epics):**
- **E01** is M0 and transitively gates every other epic (all dispatch through the live engine).
- **E03** gates **E02** (E02-T01 encodes verdicts as Engrams once E03 fixes the GateVerdict shape;
  interim typed `gate-verdicts.jsonl` unblocks if E03 slips) and **E10** (E10-T05 references E03's
  canonical `DashboardEvent`).
- **E05 + E06 + E08** must precede **E12** deletions (E12-T07 deletes `orchestrate.rs` only after
  gate-adaptivity, compose-enrichment, and conductor value is ported out).
- **E04** gates **E17** permission (E17-T01 is the same reply-channel chain as E04-T12→T14 — execute once).
- **E15** gates **E17** MCP (E17-T03 threads session MCP into the Anthropic path).
- **E13** is M3+ and **gates nothing** — every downstream v2 concept is either owned by a correctness
  epic (E01/E03/E05) or triaged Aspirational→defer.
- **E11-T04 ⇄ E03-T07** delete the same dead `Engram` stub — schedule in only one wave.
- **E12-T04 (HDC de-dup) ← E03**; **E12-T06 (drop roko-orchestrator) ← E01+E04**.

---

## 3. Milestones (entry / exit criteria)

### M0 — Bootstrap (self-execution becomes possible)

**Scope:** E01 (core) + the E04 *enforcing-safety* subset + the E05 *honest-gates* minimum + the
verify runner (already confirmed working, `04-EXECUTION-READINESS §M0.2/M0.7`).

- **E01** full (esp. E01-T01 default flip, E01-T02 resume, E01-T09 regression lock).
- **E04 subset (unattended-safe):** P16 (deny-list plumbing) + E04-T05 (Block SecretLeak/PathEscape)
  + E04-T06 (safety funnel on the default Claude-CLI path) + E04-T07 (custody hash-chain). Relay/scope
  P0s (E04-T01/T02) only if the loop is driven through `roko-serve`.
- **E05 minimum:** E05-T02 (stubs → Skipped, not `pass`) + E05-T03 (skipped excluded from `passed`/EMA)
  — the "no stub-pass" honesty floor from `04 §M0.3`.

**Entry:** repo builds on rustc 1.91+ (`rustup update stable`).
**Exit (cite `04-EXECUTION-READINESS §3`):** `roko plan run plans/<x> --engine runner-v2` (and, post
E01-T01, the *bare* default) **reliably executes a real plan and reports honest pass/fail** — after a
run `git status --porcelain` is non-empty, `.roko/episodes.jsonl` grew, `.roko/state/state-snapshot.json`
was written, and a complex-tier task with a failing symbol/verify check reports **fail/skip, not a
stub-pass**. The one-command smoke in `04 §5` passes.

### M1 — Correctness & Convergence

**Scope:** E03 (types), E02 (storage), E05 full, E06 (compose), E14 (providers/tools), E15 (MCP),
E16 (PRD front-half).

**Entry:** M0 exit green.
**Exit:** writer-path == reader-path for verdicts/executor-state/thresholds/episodes (E02 DoD); exactly
one bare `struct GateVerdict`/`DashboardSnapshot` (E03); default `plan run` exercises the 12-slot builder
(E06-T03); advertised builtins == executable handlers and one 429 no longer aborts a turn (E14); a
`.mcp.json` server actually reaches the agent as `mcpServers` (E15-T1); `idea→draft→plan` produces a
parseable `tasks.toml` with real `prd status` columns (E16).

### M2 — Completeness

**Scope:** E07 (learning), E08 (conductor), E09 (observability), E10 (frontend), E17 (ACP), E04 full
(remaining serve/relay/scope/rate-limit tasks), E18 (docs/CI/ops).

**Entry:** M1 exit green (E03/E02/E14/E15 landed — E10 needs E03; E17 needs E04+E07+E15; E18 doc
rewrites need E01 + E18's own T05–T08).
**Exit:** LinUCB survives restart + knowledge `balance>0` (E07); conductor aborts a ghost-turn loop
before wall-clock (E08); `.roko/metrics/prometheus.txt` carries `roko_gate_verdicts_total`, logs rotate
(E09); the 4 frontend 404s resolve and one SSE manager remains (E10); an ACP turn is consent-gated +
learning-informed + MCP-equipped (E17); a release tag runs clippy+test+`cargo deny` and the docs pass
the grep-guard with corrected counts/nouns/engine (E18).

### M3+ — Long-horizon

**Scope:** E11 (chain, opt-in), E12 (dead-code cleanup, gated), E13 (spec-debt / Lens).

**Entry:** M2 exit green; specifically E12's gated deletions require **E01, E03, E04, E05, E06, E08**
merged with their "live value extracted" acceptance green; E13-T01 requires E09-T09.
**Exit:** `architecture-core-queue` recovered + DeFi path resolvable and `get_logs` real (E11); the
legacy island (`orchestrate.rs`, `roko-orchestrator`, `legacy-orchestrate`, orphans, layering violation)
deleted with the workspace still green (E12); `rg 'trait Lens'` is no longer 0 and the Cell↔Block
decision doc exists (E13).

---

## 4. Critical path & parallelizable tracks

### Critical path (longest dependency chain)

The legacy-island retirement is the deepest chain, because a single deletion (E12-T07) can only start
after **three independent porting epics** finish, each of which needs E01:

```
E01  ──►  E05 (gate adaptivity ported)  ┐
     ├──►  E06 (compose enrichment ported) ├──►  E12-T07 delete orchestrate.rs  ──►  E12-T08 delete legacy-orchestrate feature
     └──►  E08 (conductor supervision ported) ┘
```

Two other chains of equal depth (~4 epics) run in parallel and set the M2→M3 boundary:
- **ACP:** `E01 → E04 → E17-T01 → E17-T06` (consent chain then conformance capstone).
- **Lens:** `E01 → E09 (T01→T09) → E13-T01 → E13-T02`.

Net critical path length ≈ **4 epic-depth**; E01 is the universal head, and the E05/E06/E08 → E12 spine
is what pushes cleanup to M3+.

### Parallelizable tracks (mutually dep-free, file-disjoint per the worktree/file-exclusivity rule)

Once **E01** lands, these run concurrently in separate worktrees (union of touched files is disjoint,
per `PLANNING-METHODOLOGY §3`):

1. **Track A — Security & ACP substrate** — **E04** (`roko-serve` middleware/relay, `roko-agent/safety`,
   `roko-acp`, custody). File-disjoint from the runner/core work; the M0 subset runs first, the rest at M2.
2. **Track B — Providers, Tools & MCP** — **E14** (`roko-std/tool/*`, `roko-agent/provider|translate`)
   + **E15** (`orchestrate.rs` MCP writer, `roko-mcp-code`). Hot dispatch-path correctness; touches
   neither serve routes nor storage.
3. **Track C — Correctness/Types spine** — **E03 → E02** (`roko-core`, `roko-fs`, serve storage readers)
   with **E05/E06** on the runner gate/compose path. E03 must lead (it gates E02 and E10).

Secondary concurrent surfaces after their gates: **E10** (frontend, after E03; `demo/demo-app` is
its own tree), **E09** (observability, after E01), **E16** (PRD front-half, after E01+E14).

**File-exclusivity caution:** E02, E03, and E04 all touch `roko-serve` readers/routes — keep E02's
reader repoints and E04's middleware edits in *dependency order* (E03 signature changes precede E02
consumers), never as siblings in one parallel group (`PLANNING-METHODOLOGY §3.7`).

---

## 5. Recommended execution order

Interleaves the pre-existing `plans/` queue (P08–P34) with the new epics. "Refresh" plans are the
shallow ones `02-PLANS-RECONCILIATION §4` flags; they feed their owning epic rather than run raw.

**Wave 0 — M0 bootstrap (serial, single-plan):**
1. **E01-T01** (flip default engine) — the single self-hosting unblocker. Then E01-T02 (resume), E01-T09 (regression lock).
2. **P16** + **E04-T05/T06/T07** (unattended-safe enforcement) · **E05-T02/T03** (honest gates floor).
3. Run the `04 §5` smoke; confirm real edits + honest fail. **← M0 exit.**

**Wave 1 — M1 correctness (parallel tracks A/B/C after E01):**
4. **E03** (types) leads Track C → then **E02** (storage) → **E05** full → **E06** (compose).
5. Track B: **P13** + **P09**(=E14 alias) then **E14** (T01 handler-parity, T02–T07) · **E15** (T1 normalizer first).
6. **E16** (needs E01+E14/P09): land **P08**+**P23** then E16-T1/T2.
7. Finish E01-T04..T08 (real DAG, worktree, gate enrichment) — refreshes **P12** (→ real `TaskDag`) and **P15** (→ replan/worktree holdouts). **← M1 exit.**

**Wave 2 — M2 completeness (parallel):**
8. **E04** remainder (relay/scope/rate-limit — needs P22 for the ACP arm) · **E10** (needs E03; land P18 adjacent) · **E09** (observability).
9. **E07** (learning; adopt **P19** for the ACP arm, keep **P26** separate) · **E08** (conductor).
10. **E17** (needs E04+E07+E15; sequence after **P21**, over **P22/P25/P28** substrate).
11. **E18**: run T01–T09 (CI/config/ops) first, then the E01-gated doc rewrites T10–T13. **← M2 exit.**

**Wave 3 — M3+ long-horizon (gated):**
12. **E11-T01** recover `architecture-core-queue` into `plans/` — **prerequisite for all chain work**
    (unblocks the BLOCKED `architecture-defi-critical-path`); then E11-T02 `get_logs`, E11-T03 deploy parity.
13. **E12** gated deletions in strict order: T05 (after E03) · T06 (after E01+E04) · **T07 (after E05+E06+E08)** · T08 (after T07). Coordinate E12-T01 orphan-delete with **E03-T01** and E11-T04 with E03-T07 (do not double-delete).
14. **E13** (Lens after E09-T09; Cell↔Block decision after E01) · then **P34-verification-sweep** last (always-applies meta gate).

---

## 6. Total task count

| Bucket | Tasks |
|---|---:|
| New epics E01–E18 (executable native tasks) | **149** |
| Existing executable plans P08–P34 + 3 side queues (`plans/INDEX.md`) | 120 |
| `architecture-core-queue` (recovered by E11-T01; Q01–Q24) | 24 |
| **Executable grand total (epics + committed/recovered plans)** | **~293** |
| Superseded (do-not-run: `self-dev-ux` 55 + `self-dev-extras` 11) | 66 |
| **Authored total incl. superseded** | **~359** |

> Note: E16's 2 tasks are the gap-only count; the ~13 tasks that cover its findings live in P08/P09/P23
> and are counted under the plans bucket (not double-counted). E14 similarly reconciles P09/P13/P28.
> E04 treats P16/P22 as prerequisites (counted under plans). This avoids inflating the epic total.
