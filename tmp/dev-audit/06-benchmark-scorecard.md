# Benchmark scorecard

## Purpose

Speed changes must be measured on representative fixtures and must not increase escaped
regressions. “Feels faster” and a single warm run are insufficient.

## Execution status

The P0 implementation is merged at `a58bdbacb`, and the expanded code is integrated through
`52d5f4df4`, but this representative scorecard has not yet been run. The score command aggregates
real bundles and retains failures/timeouts; it does not manufacture cold/warm evidence. Historical
P0 smokes and the coordinator's pending final batch are mechanics/release evidence, not promotion
data.

- [x] Create an opt-in FAST command and bounded evidence wrapper.
- [x] Preserve the pictured-run baseline and define fixtures/fields/targets.
- [x] Implement deterministic bundle scoring with failure/timeout retention and p50/p95 output.
- [ ] Run five cold and five warm repetitions for each selected fixture and comparison lane.
- [ ] Publish raw bundle links and p50/p95 results.
- [ ] Measure escaped regressions and full-CI baseline before promoting FAST.

## Baseline from pictured run

| Metric | Baseline |
|---|---:|
| Startup to runner | about 44s including maintenance/warm |
| Actual dispatches | 2 |
| Reported calls | 1,109 |
| Mechanical T1 agent | 368.6s |
| T1 gate | 107.9s |
| T2 terminal timeout | about 600s |
| Completed plan tasks | 1 / 7 |
| Evidence correctness | TUI stale after terminal |

## Fixtures

Run each with a fixed base SHA:

1. One-line enum/string/config change.
2. Local type block with derives/imports.
3. Pure store/matching logic.
4. CLI parser plus human/JSON output.
5. HTTP endpoint behavior.
6. Persistence/concurrency invariant.
7. TUI/web visual change.

Compare:

- Manual Codex fast prompt.
- Manual Claude fast prompt.
- Current Roko.
- Roko FAST.

Run cold and warm five times each. Preserve every bundle.

## Scorecard fields

Identity:

- fixture, tier, model, effort
- base SHA
- changed files and LOC
- cache strategy and target identity

Latency:

- startup_ms
- capacity_wait_ms
- context_ms
- prompt_ms
- first_edit_ms
- agent_ms
- cargo_lock_wait_ms
- compile_ms
- targeted_test_ms
- smoke_ms
- bundle_ms
- total_ms

Provider:

- actual launches
- tool calls
- prompt/current-context/cumulative input/cached input/output tokens
- cost
- retries and timeouts

Correctness:

- dispatches per attempt
- changed-file scope
- compile/test/smoke result
- endpoint pass count
- event validity
- screenshot/evidence completeness
- terminal/exit consistency
- human intervention
- escaped regression discovered later

## Targets

| Metric | Target |
|---|---:|
| XS FAST p50 | at or below 300s |
| XS FAST p95 | at or below 600s |
| Startup p95 | at or below 5s |
| Dispatches per attempt | exactly 1 |
| Mechanical agent p95 | at or below 90s |
| Mechanical prompt/input | at or below 20k tokens unless justified |
| Warm focused compile | at or below 15–30s |
| Duplicate semantic verification | 0 |
| Post-task cargo clean | 0 |
| Terminal/evidence validity | 100% |
| Evidence bundle overhead | under 5% or 2s, whichever is larger |
| Escaped regressions | no increase |
| Full CI pass rate | at least baseline |

## Experiments

### A. Agent-owned versus runner-owned compilation

- Same fixture/model/base.
- Variant 1 lets the agent run Cargo.
- Variant 2 ends after patch and lets the runner run one command.
- Measure provider time, compile time, cache reuse, and timeout salvage.

Expected: runner-owned wins because compilation is no longer charged to/provider-limited by the
agent and is not duplicated.

### B. Cache strategy

- Shared stable worktree plus incremental target, serialized Cargo.
- Per-revision target plus CARGO_INCREMENTAL=0 and sccache/base normalization.

Measure cold/warm p50/p95, disk growth, lock waits, and hit rate. Adopt one by workload; do not keep
the ineffective hybrid.

### C. Task decomposition

- Current seven serial tasks.
- One cohesive task.
- Two coherent slices.

Measure provider launches, prompt/tool tokens, compile count, total wall time, regressions, and
reviewability.

### D. Verification policy

- Current canonical plus authored gates.
- Task-verify-only.
- Impact-selected semantic gate.

Run full CI afterward to measure escaped regressions, not only local speed.

### E. Model/effort

Use identical prompt/context and compare fast/balanced/frontier models at low/medium reasoning.
Select the cheapest/fastest model that stays within the regression threshold for each tier.

## Reporting

Publish p50, p95, cold/warm split, and raw bundle links. Never average away:

- timeouts
- failed terminal states
- human intervention
- cold-cache runs
- missing evidence

A timeout is its full deadline, not a discarded outlier.
