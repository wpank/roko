# 01-Orchestration Parity Analysis

Post-audit refresh of `docs/01-orchestration/` against the live codebase.

Generated: 2026-04-18  
Refreshed: 2026-04-18 (`PU01`, run `pu-run-20260418-112124`)

---

## Batch Posture

Batch `01` is not a request to invent a new orchestration architecture.

It is a narrowed execution brief for short, real code batches:

- wire one already-built capability into one live runtime path,
- harden one recovery or monitoring seam,
- keep each batch small enough for one agent to finish in about 90 minutes,
- and stop as soon as the work starts requiring new abstractions, broad agent semantics, or Phase 2 design.

The audit result here is simple: orchestration is already wired. The debt is integration sprawl, especially `crates/roko-cli/src/orchestrate.rs`, not the absence of core concepts.

This pack now uses a strict three-way split:

- `live now` for behavior already on a runtime path,
- `small seam` for gaps that fit in a 1-3 day batch,
- `deferred` for theory, Phase 2 work, or cross-arc cleanup.

---

## Hard Corrections

- `crates/roko-cli/src/orchestrate.rs` is **17,087 lines**.
- Plan discovery is runtime-wired through `PlanRunner::from_plans_dir()`.
- Snapshot/resume is runtime-wired through `PlanRunner::from_snapshot()` and `from_snapshots()`.
- `ParallelExecutor` is the live runtime executor. `UnifiedTaskDag` is shipped code, but it is not the control point of the main runtime loop today.
- `roko-conductor -> roko-learn` is a real layer crossing, confirmed in `crates/roko-conductor/Cargo.toml`.
- The shared runtime event bus still has exactly **2** `RokoEvent` variants: `PlanRevision` and `PrdPublished`.
- Docs `12` and `13` are mostly target-state framing. Stigmergy and cross-domain orchestration are **deferred**, not batch-01 implementation goals.

---

## Reality Model

| Bucket | Meaning in this pack | Examples |
|--------|----------------------|----------|
| Live now | Present tense is allowed | plan discovery, `ParallelExecutor`, snapshot/resume, worktrees, merge queue, conductor baseline |
| Small seam | Valid `O1-O5` work only | recovery trust checks, speculative dispatch, one DAG surface, one bounded conductor effect, worktree hygiene |
| Deferred | Do not schedule here | stigmergy subsystem, cross-domain runtime, semantic merge, templates, sagas, distributed recovery |

If a proposed fix does not fit the middle row, it should leave batch `01`.

---

## Section Index

| File | Docs Covered | Post-Audit Posture | Notes |
|------|--------------|--------------------|-------|
| [A-core-orchestration.md](A-core-orchestration.md) | 00-06 | `rewrite` + `narrow` | Core loop is wired; DAG/speculation gaps stay narrow |
| [B-isolation-merge.md](B-isolation-merge.md) | 07-08 | `keep` + `narrow` | Worktrees and merge queue already exist; only runtime hardening remains |
| [C-persistence-recovery.md](C-persistence-recovery.md) | 09-10 | `rewrite` | Snapshot/resume is live; remaining work is integrity validation |
| [D-monitoring-conductor.md](D-monitoring-conductor.md) | 11 | `rewrite` | Conductor is real; background response and layer crossings are the real seams |
| [E-coordination-domains.md](E-coordination-domains.md) | 12-13 | `defer` | Stigmergy and cross-domain orchestration are Phase 2+ |
| [BATCHES.md](BATCHES.md) | — | `rewrite` | Five executable batches plus one explicit deferred lane |
| [SOURCE-INDEX.md](SOURCE-INDEX.md) | — | `rewrite` | Refreshed anchors for runtime paths and overstated docs |

---

## Post-Audit Gap Picture

The useful distinction is no longer "what orchestration features are missing?"

It is:

- what is already live and should stop being described as pending,
- what is implemented but only needs one small runtime hook,
- and what is still theory and must stay deferred.

### Already live

- `ParallelExecutor` tick/apply loop
- plan discovery
- snapshot save and resume
- per-plan worktree lifecycle
- merge queue
- conductor/watcher loop

### Small, real seams

- snapshot trust validation before restore
- event-log integrity checks in recovery flow
- runtime dispatch for speculative actions
- one operator-visible DAG use on a live path
- one bounded background conductor effect beyond logging
- one better worktree liveness/health check in unattended runs

None of these seams justify new kernel nouns, new domain models, or a scheduler rewrite.

### Deferred from batch `01`

- formal stigmergy model
- cross-domain orchestration
- chain-domain execution
- saga coordinator
- semantic merge strategies
- template system
- plan repair engine
- distributed recovery / CRDT state

---

## Recommended Execution Order

See [BATCHES.md](BATCHES.md) for the detailed contract.

The short version:

`O1 -> O5 -> O2 -> O3 -> O4`

Why this order:

- `O1` hardens trust boundaries first.
- `O5` improves unattended-runtime hygiene without widening scope.
- `O2`, `O3`, and `O4` all touch `orchestrate.rs`; narrower batches reduce conflict and keep the extraction target visible.

`O6` is not an implementation batch. It is the explicit deferred lane for docs `12-13`.

---

## Execution Boundaries

Treat these as carry-forward items, not invitations to widen batch `01`:

| Item | Better Home | Why |
|------|-------------|-----|
| event-enum unification / generic bus work | foundation cleanup | real issue, but not owned by orchestration runtime |
| domain-specific gate suites | `04-verification` | gate semantics live there |
| domain-specialized agent behavior | `02-agents` | orchestration can route, but agents own behavior |
| adaptive routing economics | `05-learning` | learning owns policy adaptation |
| formal stigmergy model | later architecture/learning work | current runtime only has indirect coordination channels |
| cross-domain chain execution | Phase 2+ roadmap | current runtime is still code-centric |

Batch `01` should only leave behind narrow runtime contracts and clear deferrals.

---

## Success Definition

Batch `01` is successful when:

- the parity pack treats orchestration as already wired,
- each executable batch is a small live-path or hardening patch,
- `orchestrate.rs` is described as the main integration hotspot,
- docs `12-13` are explicitly marked deferred,
- the shared runtime bus is no longer described as a rich, already-unified event substrate,
- and `SOURCE-INDEX.md` points editors at real code anchors instead of inherited placeholders or stale ranges.
