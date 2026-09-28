+++
id = "find-49ec18"
kind = "finding"
title = "[evals re-verify] 8 eval closures (2026-09-05) wired via Runner-v2 dispatch/event loop"
status = "open"
triage = "verified"
severity = "p2"
subsystem = ["roko-gate/eval"]
created = 2026-09-05
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/archive/evals-audit/IMPLEMENTATION-CHECKLIST.md#P0 — Close Broken Eval Loops"
discovered_from = "audit:tmp/archive/evals-audit/IMPLEMENTATION-CHECKLIST.md#P0 — Close Broken Eval Loops"
anchors = ["crates/roko-gate/src/benchmark_gate.rs::BenchmarkRegressionGate", "crates/roko-cli/src/graph_execution/plan_runner.rs:920", "crates/roko-cli/src/graph_task_dispatch.rs:2998"]
links = { depends_on = [], blocks = [], related = ["gap-6e970b", "bug-017c2d"], supersedes = [], duplicate_of = "" }
+++
Checklist marks done 2026-09-05 with Files in runner/: P0-01 benchmark gate, P0-02 EvalGenerator pre-dispatch, P0-04 CodingOracle, P1-01 GateGamingDetector, P1-04 holdout, P2-01 ShadowRunner, P2-02 eval pipeline, P4-03 PromiseTracker early termination. Runner-v2 deleted next day.

Imported without verification from:
- `tmp/archive/evals-audit/IMPLEMENTATION-CHECKLIST.md#P0 — Close Broken Eval Loops`
- `tmp/archive/evals-audit/IMPLEMENTATION-CHECKLIST.md#P1 — Wire Existing Code`
- `tmp/archive/evals-audit/IMPLEMENTATION-CHECKLIST.md#P2 — Build Missing Infrastructure`
- `tmp/archive/evals-audit/IMPLEMENTATION-CHECKLIST.md#P4 — Future / Design-Phase Work`

A source claims this was fixed; confirm against current code before closing.

How to verify: grep each type for non-test callers in graph_execution/ or roko-graph; confirm generated-tests/ artifacts and oracle observations appear after a Graph run.

Verified 2026-09-28: 6 of 8 closures are re-wired on the Graph path. CodingOracle, GateGamingDetector, HoldoutExperiment and ShadowRunner are built in graph_execution/plan_runner.rs:920-953 and carried by graph_task_dispatch.rs:948-954. EvalGenerator runs pre-dispatch at graph_task_dispatch.rs:2998 (bug-017c2d covers its placeholder output), and PromiseTracker runs at graph_task_dispatch.rs:1669. Still open: P0-01 BenchmarkRegressionGate (roko-gate/src/benchmark_gate.rs) has no non-test caller (overlaps gap-6e970b), and P2-02 has no EvalGenerationPipeline type anywhere. Severity lowered to p2 for the residual.
