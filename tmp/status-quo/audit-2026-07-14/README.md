# Status-quo current-state audit — 2026-07-14

> Execution entry point: [MASTER-EXECUTION-CHECKLIST.md](../MASTER-EXECUTION-CHECKLIST.md)
> turns this audit into dependency-ordered waves, safe multi-agent ownership rules,
> proof/merge gates, and standalone coordinator/worker/reviewer prompts.

## Bottom line

Roko is a substantial, working codebase in a half-migrated state. The work in
`tmp/status-quo` has been very effective at **discovering and planning** the
debt, and the most important execution bootstrap has landed. It has not yet
completed the remediation programme.

The cleanest defensible summary is:

- The July 8 pack is a strong evidence snapshot, but it is no longer current
  completion truth. Ninety-six of its 108 top-level documents name commit
  `5852c93c`; current HEAD is `1649c18b2`, 517 commits later.
- The two headline bootstrap defects are fixed: no-flag `roko plan run` and
  `roko resume` now select Runner v2. Explicit Graph execution is still a
  dry-run/live-dispatch stub.
- The main implementation backlog records **6 of 447 implementation tasks done
  (1.34%)** and **441 ready**. This is remediation-plan completion, not a claim
  that only 1.34% of the existing product is built.
- The narrower runner self-heal programme records **26 of 57 tasks done
  (45.6%)**. Nine additional tasks have credible implementation precursors but
  have not met their full acceptance contract.
- The six document-reconciliation plans record **0 of 71 tasks done**.
- Of 75 issue documents, the current audit substantiates **6 implemented, 21
  partial, 46 open, and 2 historical**.
- Plan authoring is broad—48 epic plans, six document plans, and one superseded
  authoring plan exist—but the backlog is not cleanly executable as one root:
  strict validation returns 23 diagnostics, and cross-plan dependency drift
  needs reconciliation.

There is no honest single percentage for “all of Roko.” The 447-task backlog
mixes near-term correctness work with a large aspirational v2 target, while the
57-task self-heal plan overlaps part of it. Adding those denominators would
double-count work. The dimensioned assessment below is the useful answer.

## Audit boundary

The pre-audit tree contained exactly **298 files**:

| Area | Files | Exhaustive annotation |
|---|---:|---|
| Top-level status pack | 108 | [TOP-LEVEL-DOCUMENT-AUDIT.md](TOP-LEVEL-DOCUMENT-AUDIT.md) |
| Backlog, epics, plans, and coverage | 102 | [BACKLOG-ROADMAP-AUDIT.md](BACKLOG-ROADMAP-AUDIT.md) |
| Issue catalogue | 75 | [ISSUES-SELF-HEAL-AUDIT.md](ISSUES-SELF-HEAL-AUDIT.md) |
| Self-heal docs and manifests | 13 | [ISSUES-SELF-HEAL-AUDIT.md](ISSUES-SELF-HEAL-AUDIT.md) |
| **Total** | **298** | Every pre-audit file is covered |

The code baseline is `main` at `1649c18b2c3d` (2026-07-12), plus an existing
dirty working tree. The working tree contains 18 modified code files and four
modified status documents, as well as untracked status/backlog artifacts. This
audit preserved those changes and did not treat uncommitted work as landed.

## Completion by evidence tier

| Dimension | Recorded state | Interpretation |
|---|---:|---|
| Existing product | Many live subsystems; substantial partial/stub/legacy seams | Mature enough to run, not migration-complete; no responsible scalar percentage |
| Epic plan materialization | 48/48 epic plan files | Planning coverage exists; it is not implementation |
| Implementation backlog | 6/447 done; 441 ready | **1.34% recorded remediation completion** |
| Document reconciliation | 0/71 done | Source/status convergence has not been executed |
| Superseded authoring plan | 96/96 skipped | Correctly excluded provenance, not unfinished implementation |
| Self-heal acceptance | 26/57 done; 31 ready | **45.6% accepted**; 9 ready tasks have partial precursors |
| Issue documents | 6 implemented; 21 partial; 46 open; 2 historical | Broad issue closure is still early |
| Top-level freshness | 96/108 explicitly tied to old baseline | Pack must be re-baselined before being called “current” |

The raw combined manifest fraction—32 accepted entries among 575 active
implementation, DOC, and self-heal entries—must **not** be used as an overall
5.6% score. Self-heal tasks overlap epic tasks, and product implementation
predates both task catalogues.

## What is genuinely fixed

### Execution bootstrap

- `plan run` defaults to `runner-v2` at
  `crates/roko-cli/src/main.rs:1361`.
- `resume` routes to `PlanEngine::RunnerV2` at
  `crates/roko-cli/src/main.rs:2699`.
- E01-T01 through E01-T06 are marked done: default selection, resume routing,
  truthful Graph behavior, live DAG scheduling, concurrency plumbing, and
  gate-failure revision work. Ten E01 tasks remain ready.

These fixes invalidate the headline ordering in many July 8 documents. They do
not make the explicit Graph engine live: `TaskExecutorCell::default()` still
sets `dry_run: true`, and its live branch still reports that dispatch is not
implemented.

### Runner lifecycle self-heal

SH01 has accepted 26 of 28 tasks. The accepted work includes per-attempt
ownership, idempotent terminalization, DAG deadlock detection, unified retry
lifecycle, process-tree cancellation, asynchronous gate/merge ownership, and
separate hard/progress/deadline semantics. Focused regression evidence exists.

The remaining SH01 work is important rather than cosmetic:

- SH01-T06C4 must prove completion-versus-expiry races and resume restoration.
- SH01-T07 must make run/plan/task summaries agree with lifecycle truth.

### Credible but not yet accepted working-tree progress

Nine future-batch tasks have meaningful precursors: safe worktree
reattachment, workspace-lock race handling, timeout records, atomic/JSONL
recovery, ordered StateHub publication and SSE resync, bounded TUI output and
route layout, dispatch routing validation, and process cancellation. They stay
partial until their batch-level acceptance conditions pass.

## What remains materially open

### Correctness and recovery

- SH02 is 0/6 accepted: per-plan capacity, task-attempt worktree ownership,
  durable commit outcomes, queued wakeups, and dirty recovery remain open.
- SH03 is 0/6 accepted: terminal snapshots, rotating checkpoints, complete
  resume state, idempotent ledger semantics, and seed equivalence remain open.
- SH06 is 0/5 accepted: there is no complete crash-chain, interruption/resume,
  connected-TUI, deterministic self-host, or release-gate proof.

These are the main reasons a successful focused run should not yet be equated
with robust unattended self-hosting.

### Security and safety

The current source still supports the high-priority July 8 findings:

- Relay routes remain merged outside the `/api` auth nest.
- Unknown route/method combinations still fall back to `read` scope.
- The default Claude CLI path does not provide the same per-tool Roko safety
  funnel as the OpenAI-compatible tool loop.
- E04 security-perimeter tasks remain unaccepted.

Security work should stay ahead of broad autonomous execution.

### Runtime, telemetry, and budget controls

SH04 and SH05 are 0/N accepted. Structured identities, approval-mode output,
diagnosis/heartbeat/liveness events, async Git refresh, stderr classification,
provider retry symmetry, cost attribution, and pre-dispatch budget enforcement
are still open even though isolated UI/config/persistence improvements exist.

### Documentation and operations

- Thirty-three top-level documents still say Graph is the default; fifteen
  still say resume is hardcoded to Graph.
- The CLI help footer still says “Graph Engine, default” even though the option
  table correctly says Runner v2, and resume help still names the old snapshot.
- Deployment blockers checked in the current tree remain: the expected root
  `roko.toml` is absent/untracked and Docker Compose uses a removed `--listen`
  argument.
- Document-reconciliation manifests are entirely ready rather than done.

## Planning and organization quality

The strongest parts of the work are its breadth, evidence-rich subsystem
traces, native `tasks.toml` plans, dependency intent, and explicit proof gates.
The weakest part is the control plane that should reconcile those layers.

Concrete organization debt:

1. Task totals disagree across documents: 149, 389, 447, and about 628 all
   appear as if they describe the same backlog. The manifests establish 447
   implementation tasks, 71 DOC tasks, and 96 skipped authoring tasks.
2. Top-level issues, epic plans, self-heal plans, commits, and tests do not share
   one stable status registry. Fixes land without rolling up to the pack.
3. `self-heal/COVERAGE.md` claims every issue is mapped but omits issues 60–67.
4. Issue numbers 10–16 collide between live-run findings and subsystem audits;
   the full filename is currently the only safe identifier.
5. `DOC-MANIFEST.md` says backlog directories are empty even though the backlog
   is populated.
6. Checkbox meaning varies by document: audit-step completion, proposed work,
   implementation, and proof are visually identical.
7. Strict backlog validation fails with 23 diagnostics, including missing
   decision/output paths and the planned-but-nonexistent `roko-gateway` crate.
8. Thirty-six cross-plan references do not match any known plan ID, and eleven
   more target P-plans outside this plan root. Per-file parsing success is
   therefore weaker than standalone root executability.

The elegant long-term fix is one machine-readable registry keyed by stable
finding/task ID with state, owning plan, proof command/result, verified commit,
and supersession metadata. Indexes and checkboxes should be generated views of
that registry.

## Recommended closure order

1. **Stabilize the present working tree.** Review and land or deliberately
   split the 18 modified code files; do not let partially accepted work exist
   only as an uncommitted state.
2. **Finish SH01-T06C4 and SH01-T07.** This closes the lifecycle foundation and
   makes all later reporting trustworthy.
3. **Execute SH02, then SH03.** Isolation/durable commits come before higher
   concurrency; complete persistence/recovery comes before unattended runs.
4. **Close E04 security perimeter.** Auth the relay, deny unknown mutations by
   default, and prove the default provider’s safety boundary.
5. **Use SH06 as the release gate.** Require crash, resume, connected telemetry,
   and deterministic self-host evidence before calling self-heal complete.
6. **Reconcile the backlog.** Resolve strict diagnostics and cross-plan
   dependencies, deduplicate overlapping E01/SH/E46–E48 work, and review ready
   tasks whose acceptance may already be partially or fully satisfied.
7. **Run the 71 DOC tasks from current code.** Re-baseline the navigation layer,
   regenerate inventories, and banner superseded documents.
8. **Keep CI-generated truth.** Generate counts, route/command manifests, plan
   status summaries, and stale-doc failures instead of editing them by hand.

## Verification performed for this audit

| Check | Result |
|---|---|
| Inventory | 298 pre-audit files: 108 + 102 + 75 + 13 |
| Exhaustive annotation | All four populations covered by the three linked ledgers |
| Markdown links | 234/234 Markdown files scanned; 0 missing local links |
| Self-heal strict validation | 0 diagnostics in 6 plans; exit 0 |
| Backlog strict validation | 23 diagnostics in 55 plans; exit 1 |
| `cargo fmt --all -- --check` | Pass |
| `cargo check --workspace --all-targets` | Pass, with existing warning debt |
| Diff whitespace | Existing diff check passed; new audit files have no trailing whitespace |

This is a documentation/status audit. It adds current annotations and preserves
the existing source and status-pack changes; it does not relabel partial work as
done or silently modify product behavior.
