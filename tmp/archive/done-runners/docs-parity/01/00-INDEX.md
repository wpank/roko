# 01-Orchestration Parity Refresh

Post-audit refresh of `docs/01-orchestration/` against the live codebase.

Generated: 2026-04-18  
Refreshed: 2026-04-18 (`PU01`, run `pu-run-20260418-130603`)

---

## Batch Posture

Batch `01` is not an excuse to redesign orchestration.

The audit result is narrower than the original refinement pass:

- orchestration is already wired end-to-end,
- `crates/roko-cli/src/orchestrate.rs` is the real integration debt at **17,087 lines**,
- and the useful follow-on work is a short list of runtime seams that one agent can prove in a focused session.

This pack uses a strict split:

- `live now`: already on a runtime path; present tense is allowed
- `small seam`: one bounded runtime hook or hardening patch
- `deferred`: target-state or Phase 2+ material that must stay out of batch `01`

---

## Hard Corrections

- `orchestrate.rs` is **17,087** lines, not a thin harness.
- Plan discovery is already wired through `PlanRunner::from_plans_dir()`.
- Snapshot/resume is already wired through `PlanRunner::from_snapshot()` and `from_snapshots()`.
- `ParallelExecutor` is the live execution control point. `UnifiedTaskDag` is shipped support code, not the owner of the main runtime loop.
- Worktrees are real runtime subsystems. `MergeQueue` is implemented code, but it is not clearly the active control point of the current runtime path.
- `roko-conductor -> roko-learn` is a real layer violation, confirmed in `crates/roko-conductor/Cargo.toml:15`.
- The shared runtime bus still has exactly **2** live `RokoEvent` variants: `PlanRevision` and `PrdPublished`.
- Docs `12-13` are mostly target-state framing. Stigmergy and cross-domain orchestration stay **deferred** in this pack.

---

## Section Index

| File | Docs Covered | Audit Posture | What Changed |
|------|--------------|---------------|--------------|
| [A-core-orchestration.md](A-core-orchestration.md) | 00-06 | `rewrite` + `narrow` | core loop is wired; keep only bounded executor and DAG seams active |
| [B-isolation-merge.md](B-isolation-merge.md) | 07-08 | `keep` + `narrow` | worktrees and merge queue exist; only runtime hygiene remains |
| [C-persistence-recovery.md](C-persistence-recovery.md) | 09-10 | `rewrite` | snapshot/resume is live; remaining work is trust validation |
| [D-monitoring-conductor.md](D-monitoring-conductor.md) | 11 | `rewrite` | conductor is real; the layer crossing and one bounded response are the real seams |
| [E-coordination-domains.md](E-coordination-domains.md) | 12-13 | `defer` | stigmergy and cross-domain runtime stay future-state |
| [BATCHES.md](BATCHES.md) | — | `rewrite` | active tasks narrowed to 30-90 minute code batches |
| [SOURCE-INDEX.md](SOURCE-INDEX.md) | — | `rewrite` | refreshed to current source anchors and line numbers |

---

## Post-Audit Gap Picture

The useful question is no longer "what orchestration features are missing?"

It is:

- what is already live and should stop being described as pending,
- what is implemented but still has one real runtime seam,
- and what is still conceptual and must stay deferred.

### Live now

- `ParallelExecutor` tick/apply loop
- plan discovery
- snapshot save and resume
- per-plan worktree lifecycle
- merge queue
- conductor checks and background watcher runner

### Small seams

- validate persisted state before restore
- call event-log integrity checks on the real recovery path
- make speculative executor actions runtime-reachable
- expose one DAG-derived signal on one live path
- turn one background conductor finding into one bounded runtime effect
- improve unattended worktree liveness with one safe health signal

### Deferred

- formal stigmergy
- cross-domain orchestration
- chain-domain execution
- templates, sagas, semantic merge, plan repair
- distributed recovery or CRDT state

If a proposed fix does not fit the middle list, it should leave batch `01`.

---

## Recommended Execution Order

See [BATCHES.md](BATCHES.md) for the detailed contract.

Default order:

`O1 -> O5 -> O2 -> O3 -> O4`

Why this order:

- `O1` hardens the recovery trust boundary first.
- `O5` improves unattended runtime hygiene without widening the layer.
- `O2`, `O3`, and `O4` all touch `orchestrate.rs`; keeping each seam small reduces conflicts.

`O6` is the explicit deferred lane for docs `12-13`. It is not a code batch.

---

## Carry-Forward Boundaries

These are real findings, but they do not belong to orchestration batch `01`:

| Item | Better Home | Why |
|------|-------------|-----|
| event-enum unification / generic bus work | foundation cleanup | real issue, but not owned by the orchestration runtime |
| domain-specific gates | `04-verification` | orchestration can route, but verification owns gate semantics |
| domain-specialized agent behavior | `02-agents` | agent backends own behavior |
| adaptive routing economics | `05-learning` | learning owns policy adaptation |
| formal stigmergy model | later architecture work | current runtime only has indirect coordination channels |
| cross-domain execution | Phase 2+ | current runtime is still code-centric |

---

## Success Definition

Batch `01` is successful when:

- the parity pack treats orchestration as already wired,
- every active batch fits inside one focused 30-90 minute session,
- `orchestrate.rs` is named plainly as the main integration hotspot,
- docs `12-13` are explicitly deferred,
- and the source index points editors at real code anchors instead of stale placeholders.
