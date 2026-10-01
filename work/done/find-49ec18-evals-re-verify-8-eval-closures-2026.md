+++
id = "find-49ec18"
kind = "finding"
title = "8 eval closures (2026-09-05) wired via Runner-v2 dispatch/event loop"
status = "done"
triage = "verified"
severity = "p2"
size = "M"
goal = "learning"
subsystem = ["roko-gate/eval"]
created = 2026-09-05
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d1e3c5681"
source = "tmp/archive/evals-audit/IMPLEMENTATION-CHECKLIST.md#P0 — Close Broken Eval Loops"
discovered_from = "audit:tmp/archive/evals-audit/IMPLEMENTATION-CHECKLIST.md#P0 — Close Broken Eval Loops"
anchors = ["crates/roko-gate/src/benchmark_gate.rs::BenchmarkRegressionGate", "crates/roko-gate/src/benchmark_gate.rs::compare_results", "crates/roko-cli/src/task_parser.rs::VerifyStep", "crates/roko-cli/src/graph_task_dispatch.rs::GraphTaskDispatcher::dispatch", "crates/roko-cli/src/graph_execution/plan_runner.rs:1005"]
links = { depends_on = [], blocks = [], related = ["gap-6e970b", "bug-017c2d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rq 'BenchmarkRegressionGate\\|compare_results' crates/roko-cli/src/graph_task_dispatch.rs crates/roko-cli/src/graph_task_dispatch crates/roko-cli/src/graph_execution && grep -rqw 'fn graph_bench_verify_step_fails_on_regression' crates/roko-cli/src/ && cargo test -p roko-cli graph_bench_verify_step_fails_on_regression"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T19:41:27Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "M"
claimed_at = "2026-10-01T16:12:59Z"
forced = false
evidence = "Gate 6c on 4bacc9d88 and its fix-up 060b62813, merged as d1e3c5681 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 14 crates; lib tests pass (roko-cli 3395, roko-agent 2269, roko-core 1977, roko-learn 1225, roko-serve 1010, roko-gate 697, roko-compose 562, roko-graph 487, roko-dreams 260, roko-execution 245, roko-neuro 239, roko-acp 211, roko-gateway 101); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, merge_proof, plan_conversion, job_lifecycle and job_runner_integration tests pass; bin 445; Cargo.lock unchanged; route inventory tests and --check-snapshot pass. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

## Problem

The 2026-09-05 evals audit closed eight items by wiring them into Runner-v2, which was deleted the
next day (6b5da8616). Six have since been re-attached to the Graph engine. Two have not:

- P0-01, `BenchmarkRegressionGate` (`crates/roko-gate/src/benchmark_gate.rs:90`). It is fully
  implemented: it parses Criterion JSON, keeps baselines in `.roko/bench/baselines/`, and fails when
  a benchmark slows down by more than 10%. Nothing outside its own tests constructs it, so a
  `roko plan run` never detects a performance regression.
- P2-02, the 3-stage eval generation pipeline (`EvalGenerationPipeline`). It lived in
  `crates/roko-cli/src/runner/eval_generation.rs` (added in 8c529dae1) and was deleted with Runner-v2
  in 6b5da8616. No equivalent exists at HEAD.

A related weakness is P0-02, the `EvalGenerator` pre-dispatch step. It runs on Graph only when
`gates.write_eval_artifacts = true` (default false). It writes to `.roko/generated-tests/`, and
nothing in `plan run` executes those files. The code comment at `graph_task_dispatch.rs:3294-3296`
still says they are "read by the Runner-v2 generated-test rung". No production code in roko-cli
uses `GeneratedTestGate`.

## Why it matters

Goal `learning` (Learning loops on the Graph path). This item re-checks audit closures that were
lost with Runner-v2, and it should end with each one either wired on Graph or explicitly declared
out of scope.

Related items:
- `gap-6e970b` (parked, in `work/parked/`): "BenchmarkRegressionGate stub, EvalGenerator
  unconsumed". Its "stub" premise is stale: the gate has had real logic since 8c529dae1.
- `bug-017c2d`: the `EvalGenerator` templates render empty-bodied tests, and old placeholders remain
  in `generated-tests/`.
- `find-4b4344` and `find-34a4b5`: the same Runner-v2 re-verification for gate and learning
  closures.

## Where

- `crates/roko-gate/src/benchmark_gate.rs` holds `BenchmarkRegressionGate` (:90) and its builders
  (`with_threshold_pct`, `with_bench_args`, `with_baseline_dir`, `with_timeout_ms`). It also has
  pure helpers: `parse_criterion_json` (:287), `read_baseline` (:318), `write_baseline` (:325) and
  `compare_results` (:343). Its `verify` (:176) runs `cargo bench` itself in
  `std::env::current_dir()`, not in the task worktree.
- In `crates/roko-cli/src/graph_task_dispatch.rs`:
  - the Graph verify loop runs each task's `VerifyStep`s (`task_parser.rs:769`, with a `phase`
    label);
  - `EvalGenerator` pre-dispatch is at :3290-3330;
  - the fields of `GraphFeedbackContext` are at :1038-1042.
- `crates/roko-cli/src/graph_execution/plan_runner.rs:1005-1063` builds the four re-wired
  components.
- The deleted pipeline can be recovered with
  `git show 6b5da8616^:crates/roko-cli/src/runner/eval_generation.rs` (891 lines). It has three
  stages: template-based generation, a discrimination check that compiles the tests against the
  unmodified code and keeps only the ones that fail, and registration under `generated-tests/` for
  `GeneratedTestGate` (rung 3).
- Entry point: `roko plan run <dir>`.

## Current state

Re-wired on Graph, checked at HEAD:
- P0-04 `CodingOracle`: `plan_runner.rs:1005`, observations at `graph_task_dispatch.rs:1963-1990`;
- P1-01 `GateGamingDetector`: `plan_runner.rs:1013`, used at `graph_task_dispatch.rs:2281`;
- P1-04 `HoldoutExperiment`: `plan_runner.rs:1022-1030`, used at :2358;
- P2-01 `ShadowRunner`: `plan_runner.rs:1038`, used at :3349;
- P4-03 `PromiseTracker`: `graph_task_dispatch.rs:1837`, used for early termination at :2009-2027;
- P0-02 `EvalGenerator`: opt-in only, and its output is unused (see Problem).

Still open: P0-01 (no caller) and P2-02 (no type).

## Plan

The design question is whether Graph plan runs should run benchmark and generated-test checks at
all. Graph verification runs task-authored commands, and there is no rung pipeline for these gates
to plug into.

1. P0-01: recommended wiring is an opt-in verify phase.
   - When a task's `VerifyStep.phase` is `bench` (for example
     `cargo bench -p X -- --output-format json`), run the step as usual, then give its stdout to
     `parse_criterion_json`.
   - Compare against `read_baseline` from `<workspace>/.roko/bench/baselines/<plan>-<task>.json`
     using `compare_results`, and fail the step when any `change_pct` exceeds the threshold (default
     10%).
   - On the first run, write the baseline with `write_baseline` and pass.
   - Use the pure helpers, not `BenchmarkRegressionGate::verify`, because `verify` benches the
     process cwd rather than the task worktree. Keep a `BenchmarkRegressionGate` value (or its
     threshold) so the gate and its config stay the single source.
   - Alternative: declare P0-01 out of scope for plan runs and leave the gate as a library. If you
     choose that, close this finding as partial and revive `gap-6e970b` for any later wiring.
2. P2-02: do not restore the pipeline now. Its Stage 1 is template-based, and the templates render
   empty tests (`bug-017c2d`). Its discrimination check would reject all of them, so it would
   register nothing and cost a `cargo test` per task. Record P2-02 as not applicable until
   LLM-authored test generation exists, and point to the recoverable file above.
3. P0-02: fix the stale comment at `graph_task_dispatch.rs:3294-3296`. Either give
   `.roko/generated-tests/` a consumer (a `generated-tests` verify phase that runs them), or state in
   the config docs that `gates.write_eval_artifacts` only writes files for manual inspection.
4. Update this item: close P0-01 with the commit if wired, and record P2-02 and P0-02 as decided.

## Done when

- A Graph verify step with phase `bench` records a baseline on first run and fails a later run whose
  Criterion means are more than 10% slower. A unit test drives this with canned Criterion JSON, with
  no real `cargo bench`.
- The P2-02 and P0-02 decisions are written in this item, and the stale "Runner-v2 generated-test
  rung" comment is gone.
- Verify (suggested: the old grep passes on a comment):
  `grep -rq 'BenchmarkRegressionGate\|compare_results' crates/roko-cli/src/graph_task_dispatch.rs crates/roko-cli/src/graph_execution && grep -rqw 'fn graph_bench_verify_step_fails_on_regression' crates/roko-cli/src/ && cargo test -p roko-cli graph_bench_verify_step_fails_on_regression`

## Notes

- Do not make a `bench` phase run by default. Benchmarks are slow and noisy, so they must be
  authored into a task explicitly.
- Baselines must live in the main workspace `.roko/`, not in the task worktree, or they are lost when
  the worktree is cleaned up. Never delete worktrees or plan branches.
- The change is confined to the verify loop in `graph_task_dispatch.rs`. Coordinate with
  `find-4b4344` and `gap-7a3527`, which edit the same verify and feedback block, and do not run them
  in parallel.
- 2026-10-01 (wk-learn2): implemented on work/gap-14f08e; cargo verification deferred to the batch check.
  - P0-01 is wired as an opt-in verify phase. A `[[task.verify]]` step with `phase = "bench"` runs as usual; once it
    passes, `graph_task_dispatch/bench_verify.rs::judge_bench_step` judges its output with the new
    `BenchmarkRegressionGate::judge_output` (the gate's own `verify` now uses it too) against
    `<workspace>/.roko/bench/baselines/<plan>-<task>.json`. The first run records the baseline, a later run more than
    10% slower fails, and a `bench` step whose output holds no Criterion result fails. Both verify loops judge (the
    first run and the re-run after an auto-fix). Tests: `graph_bench_verify_step_fails_on_regression` (a Graph dispatch
    whose step prints canned Criterion JSON), `steps_of_other_phases_and_failed_steps_keep_their_verdict`, and roko-gate
    `judge_output_records_a_baseline_then_fails_a_regression`.
  - The `[[verify]]` grep now also searches `crates/roko-cli/src/graph_task_dispatch/`, where the verify loop moved
    from `graph_task_dispatch.rs`. It still finds nothing at BASE.
  - P2-02, decided: not applicable until LLM-authored test generation exists. bug-017c2d removed the empty templates,
    and the property template needs an assertion body that no Graph task authors, so the pipeline would pay a
    `cargo test` per task to register nothing. The deleted pipeline is recoverable with
    `git show 6b5da8616^:crates/roko-cli/src/runner/eval_generation.rs`.
  - P0-02, decided: `gates.write_eval_artifacts` writes, for manual inspection only, the evaluations that pass
    `generate_checked`; nothing in `plan run` executes them, and the config doc says so. The stale "Runner-v2
    generated-test rung" comment is gone. Both landed with bug-017c2d (`91deb4f3d`).

## Original notes

Checklist marks done 2026-09-05 with Files in runner/: P0-01 benchmark gate, P0-02 EvalGenerator pre-dispatch, P0-04 CodingOracle, P1-01 GateGamingDetector, P1-04 holdout, P2-01 ShadowRunner, P2-02 eval pipeline, P4-03 PromiseTracker early termination. Runner-v2 deleted next day.

Imported without verification from:
- `tmp/archive/evals-audit/IMPLEMENTATION-CHECKLIST.md#P0 — Close Broken Eval Loops`
- `tmp/archive/evals-audit/IMPLEMENTATION-CHECKLIST.md#P1 — Wire Existing Code`
- `tmp/archive/evals-audit/IMPLEMENTATION-CHECKLIST.md#P2 — Build Missing Infrastructure`
- `tmp/archive/evals-audit/IMPLEMENTATION-CHECKLIST.md#P4 — Future / Design-Phase Work`

A source claims this was fixed; confirm against current code before closing.

How to verify: grep each type for non-test callers in graph_execution/ or roko-graph; confirm generated-tests/ artifacts and oracle observations appear after a Graph run.

Verified 2026-09-28: 6 of 8 closures are re-wired on the Graph path. CodingOracle, GateGamingDetector, HoldoutExperiment and ShadowRunner are built in graph_execution/plan_runner.rs:920-953 and carried by graph_task_dispatch.rs:948-954. EvalGenerator runs pre-dispatch at graph_task_dispatch.rs:2998 (bug-017c2d covers its placeholder output), and PromiseTracker runs at graph_task_dispatch.rs:1669. Still open: P0-01 BenchmarkRegressionGate (roko-gate/src/benchmark_gate.rs) has no non-test caller (overlaps gap-6e970b), and P2-02 has no EvalGenerationPipeline type anywhere. Severity lowered to p2 for the residual.

Checked 2026-09-29 at d9e79e9d8: P0-01 BenchmarkRegressionGate still has no non-test caller and P2-02 (eval pipeline) still has no type. P0-02 is weaker than recorded: since 725f21e05, Graph dispatch runs EvalGenerator only when gates.write_eval_artifacts is set (default false), writes to .roko/generated-tests/, and the code states nothing in plan run executes those files. CodingOracle, GateGamingDetector, HoldoutExperiment, ShadowRunner (plan_runner.rs:1005-1038) and PromiseTracker (graph_task_dispatch.rs:1837) remain wired.
