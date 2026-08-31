# Roko five-minute development audit

Date: 2026-08-31

This directory began as the design package for making routine Claude, Codex, and Roko
self-development loops dramatically faster. Documents 00–09 preserve the audit evidence and
design contract, with current status annotations distinguishing later source implementation from
runtime proof; no production settings, gates, or tests were changed while the baseline evidence
was collected. The approved P0 implementation was merged to `main` in `a58bdbacb`.
The expanded implementation is code-complete through integration snapshot `52d5f4df4`, but its
single final verification/rebase/merge batch is still pending. Document 11 is the authoritative
implemented-versus-verified ledger for that expanded work.

## Bottom line

The pictured run was not delayed by tests. It ran no tests and no clippy. It lost time to:

1. A coding agent compiling inside its sandbox after making a tiny edit.
2. A shared Cargo target that the Codex sandbox could not write, forcing cold builds in /tmp.
3. The runner compiling the same task again, twice, with one check targeting the wrong Rust target.
4. A scheduler admission bug that performed dispatch preparation 1,109 times for two real launches.
5. A second agent running until the 600-second timeout even though its useful edit already existed.
6. A seven-microtask plan that repeats agents, worktrees, prompts, gates, and commits.

Disabling tests alone would have saved zero seconds in that run.

The previous T0 reflex-store implementation was different: it was a large, multi-crate
persistence/concurrency/safety feature, not a five-minute patch. Cached test execution was
short compared with implementation, repeated compile/link cycles, Cargo-lock contention, and
the correctness issues found during integration review. The five-minute target must therefore
be an explicit fast lane for eligible work, not a blanket promise for every release-grade change.

## Recommended operating model

- FAST lane: one coherent patch, 90-second edit budget, no agent-owned compilation, exactly one
  runner-owned impacted check, and real CLI/API/screenshot evidence. Five-minute SLO.
- RELEASE lane: focused regression tests plus impacted reverse dependents before merge; broad
  suites run asynchronously or nightly.
- Mechanical lane: deterministic edit or reflex when possible; do not launch a premium model for
  a one-line enum/string/config change.
- High-risk lane: persistence, safety, auth, migrations, concurrency, scheduler, and payment work
  retain focused invariant tests. These tasks are not forced into a dishonest five-minute box.

## Documents

- [00-executive-audit.md](00-executive-audit.md): why the work was slow and what matters most.
- [01-baseline-evidence.md](01-baseline-evidence.md): exact measurements from the pictured run.
- [02-five-minute-loop.md](02-five-minute-loop.md): the time-boxed workflow and task contract.
- [03-verification-policy.md](03-verification-policy.md): T0–T3 risk-based verification.
- [04-roko-self-hosting.md](04-roko-self-hosting.md): Roko runtime and architecture bottlenecks.
- [05-feedback-harness.md](05-feedback-harness.md): logs, endpoints, screenshots, and run bundles.
- [06-benchmark-scorecard.md](06-benchmark-scorecard.md): proof that speedups are real and safe.
- [07-rollout.md](07-rollout.md): reversible changes first, then P0/P1/P2 implementation.
- [08-decisions-needed.md](08-decisions-needed.md): decisions resolved by P0 and choices that still
  require benchmark or policy approval.
- [09-additional-live-run-findings.md](09-additional-live-run-findings.md): provider proof, timeout
  usage loss, Codex tool-policy degradation, TUI overhead, terminal persistence, log health,
  worktree safety, and implementation sizing.
- [10-p0-implementation.md](10-p0-implementation.md): what was implemented, how to run it, and
  the verification/deferred-work record for commit `a58bdbacb`.
- [11-implementation-status.md](11-implementation-status.md): the current integration ledger for
  lean builds/hot paths, evidence, impact gates, bounded context, run APIs, hard deadlines,
  timeout salvage, terminal convergence, final verification, and explicit residuals.
- [Tracked FAST operator guide](../../docs/v2/29-FAST-DEVELOPMENT.md): the canonical documentation
  shipped on `main` in documentation reconciliation commit `c28b2d618`.
- [prompts/fast-implement.md](prompts/fast-implement.md): reusable Claude/Codex implementation prompt.
- [prompts/fast-diagnose.md](prompts/fast-diagnose.md): reusable bounded diagnosis prompt.
- [schemas/session.schema.json](schemas/session.schema.json): proposed run-bundle summary schema.
- [endpoints/core-get.txt](endpoints/core-get.txt): safe GET discovery/query seed list.

## Assumptions used

1. Five minutes means patch-to-evidence for normal XS/local work.
2. Exhaustive verification moves out of the interactive loop rather than being deleted.
3. A failed five-minute task stops with a bundle and precise blocker; it does not silently consume
   30–90 minutes.
4. FAST mode starts opt-in until its scorecard proves it is at least as reliable as the current path.
5. This package extends the existing dev.sh and backlog #228 evidence-bundle direction instead of
   creating an unrelated permanent toolchain.

## Current implementation

The narrow P0 batch is on `main` at `a58bdbacb`. Start it with:

```bash
./dev.sh fast plans/<plan-directory>
```

FAST remains opt-in. Each task must author exactly one verification command; the runner fails
closed otherwise. The expanded integration adds the features below, but the final tree still must
complete its one batched release verification before merge. Benchmark the fixtures in
[06-benchmark-scorecard.md](06-benchmark-scorecard.md) before changing the default.

Current status:

- [x] Scheduler admission and exact-attempt ownership before expensive preparation.
- [x] Patch-only FAST prompt and one runner-owned authored verification command.
- [x] Safe target-aware Cargo narrowing and exact FAST-only verification deduplication.
- [x] Fresh-worktree frontend fallback and bounded private evidence bundles.
- [x] Durable terminal settlement before best-effort timeout evidence export.
- [x] Lean feature-gated development graph and runtime hot-path fixes.
- [x] Impact-selected gates, bounded reverse dependents, and one observable compiler owner.
- [x] Bounded/cohesive FAST plan context and fail-closed unsupported provider restrictions.
- [x] Run-scoped event/task/gate/log/metrics/artifact APIs with cursor pagination and SSE.
- [x] Wake-driven scheduling, hard startup interposition, typed timeout-diff gate salvage, and
  convergent terminal projections.
- [ ] Run the coordinator's final compile/test/clippy/smoke batch on the rebased integration.
- [ ] Run the representative cold/warm scorecard and evaluate promotion criteria.
- [ ] Add an offline repair command for historical runs created before per-run indexes.
- [ ] Add a Codex operation-level broker for restrictive built-in operation policy.
- [ ] Promote FAST or enable auto-merge; both remain explicitly deferred policy decisions.
