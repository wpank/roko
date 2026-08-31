# Rollout

Status through the post-integration cache, offline-index, and benchmark-tooling commits listed in
[11-implementation-status.md](11-implementation-status.md): Stage 0, Stage 1, and most source work
in Stages 2–3 are implemented as an opt-in FAST lane. Unchecked items are intentionally deferred or
require final runtime/benchmark evidence; source completion must not be inferred to mean the
rebased tree has passed its final batch.

## Stage 0: reversible experiment

No production defaults changed. `./dev.sh fast` now owns the reversible experiment:

- [x] Select FAST explicitly rather than changing the normal lane.
- [x] Use the existing `target/debug/roko`; never build Roko through Cargo inside the loop.
- [x] Run headless and reject native TUI/approval overrides that violate the automation contract.
- [x] Set a 90-second agent cap and a bounded plan deadline with settlement headroom.
- [x] Set retries to zero and default to one task at a time.
- [x] Skip agent Cargo, broad local verification, preflight, frontend building, startup warmup,
  critical-path GC, and target cleanup.
- [x] Require exactly one authored target-aware verify command.
- [x] Capture a private bounded evidence bundle.
- [ ] Score five cold and five warm runs for each representative fixture.

The design fragment in [config/fast-overrides.toml](config/fast-overrides.toml) remains reference
material; use the supported `./dev.sh fast` wrapper instead of treating the fragment as a complete
configuration.

## Stage 1: P0 implementation

### Batch A: scheduler truth

- [x] Atomic attempt reservation before preparation.
- [x] Wake scheduling on permit release, agent/gate completion, retry readiness, and deadlines.
- [x] One dispatch per exact attempt invariant.
- [x] Actual-launch counters separate from queued/prepared counts.
- [x] Terminal TUI/dashboard/status/ledger/PID convergence, immutable elapsed duration, and
  cleanup-degraded ownership preservation are source-implemented.

Acceptance still to measure:

- [ ] doctor-network fixture emits one launch for one attempt.
- [ ] No repeated model/prompt/hook work while waiting for capacity in representative runs.
- [ ] TUI and bundle agree on duration and terminal state.

### Batch B: one verification owner

- [x] FAST prompt ends after patch handoff.
- [x] Explicit FAST gate policy and no-warm-cache controls.
- [x] Changed-target classification with safe fallback for ambiguous diffs.
- [x] Exact Cargo semantic deduplication in FAST. Arbitrary shell-wrapper equivalence is deferred.
- [x] One per-repository compile owner with lock-wait/cache-mode/command/duration spans.
- [x] Narrow Codex writable target opt-in confined to the canonical repository target.

Acceptance still to measure:

- [ ] One-line `main.rs` fixture performs one `--bin` check.
- [ ] No Cargo process runs inside the provider session in representative runs.
- [ ] Cache denial cannot silently trigger an untracked `/tmp` cold build.

### Batch C: bundle minimum

- [x] Run ID on runner events and per-run indexes for new canonical runtime records.
- [x] Bounded stdout/stderr, optional events, Git diff, timings, and summary artifacts.
- [x] Terminal/event validation when an events stream is present.
- [x] Safe GET/OpenAPI collection plus opt-in CLI, text, Roko screenshot, and PNG adapter hooks.
- [x] Machine/cache snapshot.
- [x] Bounded dry-run-first repair for pre-index historical events, separate from HTTP/startup.

Acceptance:

- [x] One command produces a private bounded bundle on pass, failure, and timeout.
- [x] Strict schema-v2 cross-artifact validator.
- [ ] Explicit cancellation/live-run fixture in the final integrated verification batch.

## Stage 2: P1 development ergonomics

- [x] Conservative impact graph and bounded reverse-dependent selection.
- [x] Nextest fast/slow/live profiles.
- [x] Feature-gate Alloy/chain, full provider/serve, ACP, and frontend embedding; release/Docker
  jobs request their full contract explicitly.
- [x] Move npm install/build out of normal Cargo checks.
- [x] Lean `dev-fast` profile and pinned toolchain.
- [ ] Select the long-term worktree/cache strategy from real cold/warm benchmarks.
- [x] Deterministic fixed-SHA benchmark automation with isolated cold targets, bounded warm targets,
  evidence bundles, and raw/p50/p95 scorecards.
- [x] Plan policy resolves exact files/ranges/symbol anchors and rejects missing/broad context.
- [x] Cohesive task generation, a four-task FAST ceiling emitted to the generator, and rejection of
  same-file fragmentation/duplicate verification.
- [x] First-class safe timeout-diff salvage through ordinary safety/gate ownership and durable
  content-fingerprint resume.

## Stage 3: P2 observability and hot paths

- [x] Run-indexed event/task/gate/log/metrics/artifact APIs and run-filtered SSE for new runs.
- [x] Bounded, change-selected text/PNG screenshot evidence hooks.
- [x] OpenAPI-driven safe GET collection.
- [x] Provider/runner evidence normalization into bundle metrics and per-run records.
- [x] Buffered nonterminal per-run logs with lifecycle-boundary flush.
- [x] Bounded prompt/source context off the Tokio hot path.
- [x] In-memory global learning/router updates.
- [x] Size/age/revision-aware cache status and dry-run-first pruning, protected by active leases and
  kept outside dispatch; warm targets remain available outside explicit cleanup.
- [ ] Historical benchmark dashboard with regression alerts.
- [x] Explicit bounded offline repair for global events created before per-run indexes.

## Existing backlog mapping

Reuse rather than duplicate:

- #112: plan-run screenshots.
- #115: structured log wire verification.
- #151/#152: TUI PNG and screenshot comparison.
- #170: adaptive verify scoping.
- #184: artifact freshness.
- #197: structured reviewer JSON.
- #207/#209: routing hints/provider proof.
- #212/#215: run IDs and run-scoped event endpoints.
- #218: structured gate failures.
- #228: dogfood session evidence bundle.
- #230: chain/Alloy feature gating.
- #231: cross-crate impact scoping.
- #58: async/runtime hot-path fixes.

Several “done” backlog features are not proven end-to-end in the pictured path. The bundle must
distinguish code-present from runtime-proven.

## Expected impact

These remain projections until the scorecard is run; the P0 merge is not itself benchmark proof.

| Change | Likely saving on pictured small task |
|---|---:|
| No startup workspace warm/critical GC | about 35–45s |
| No agent-owned cold compile | about 5–6m per affected task |
| One targeted gate, no duplicate | about 1–2m |
| One cohesive task instead of seven | several provider/gate cycles |
| Admission before preparation | removes 1,108 phantom preparations |
| 90s edit handoff | prevents 600s verification timeout |
| Headless FAST automation | avoids false post-run TUI elapsed time |

Savings are not strictly additive, but the combined warm path is plausibly 2–4 minutes for this
class of feature.

## Promotion criteria

Make FAST the local default only when:

- [ ] 20+ representative fixture runs exist.
- [ ] p50/p95 targets hold cold and warm.
- [ ] One-dispatch invariant holds 100%.
- [ ] Bundles validate 100%.
- [ ] No increase in escaped regressions.
- [ ] Full CI pass rate stays at baseline.
- [ ] Operators can switch to RELEASE explicitly and see the selected lane.

The P0 change was explicitly reviewed and merged; it did not enable automatic merges. Do not
auto-merge by default until a separate decision is made in
[08-decisions-needed.md](08-decisions-needed.md).
