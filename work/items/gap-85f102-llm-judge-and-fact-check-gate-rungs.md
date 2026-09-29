+++
id = "gap-85f102"
kind = "gap"
title = "LLM-judge and fact-check gate rungs never run in production"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
subsystem = ["roko-gate/oracles"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-gate/src/llm_judge_gate.rs::JudgeOracle", "crates/roko-gate/src/llm_judge_gate.rs::LlmJudgeGate", "crates/roko-gate/src/production_service.rs:331"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'RungExecutionConfig::default()' crates/roko-gate/src/production_service.rs && grep -rlE 'impl .*JudgeOracle for' crates/*/src | grep -qv 'crates/roko-gate/src/llm_judge_gate.rs'  (cargo test -p roko-gate llm_judge runs the existing unit tests with test oracles, which pass while the rungs never run in production)"
+++

`LlmJudgeGate` (rung 6) has no `JudgeOracle` implementation outside tests (`llm_judge_gate.rs:348`, `:359`, `tests/rungs.rs:141`), and `FactCheckGate` (rung 5) was wired only on the removed Runner-v2 gate path.
`ProductionGateService` uses `RungExecutionConfig::default()` and the Graph engine runs authored verify steps instead of the rung ladder, so the semantic rungs of the 7-rung pipeline never execute.
Fix: a production judge oracle, an injectable `RungExecutionConfig` on `ProductionGateService`, and an opt-in (advisory by default) post-verify judge step in the Graph engine.

Checked 2026-09-29 at d9e79e9d8: still open. Graph dispatch now has a best-effort quality_judge (graph_task_dispatch.rs:2274-2280, from 725f21e05) that asks a cheap model for a score only when verify failed and passes it to GateGamingDetector; it never gates and does not run LlmJudgeGate or FactCheckGate. None of the three fix parts exist yet: a production JudgeOracle, an injectable RungExecutionConfig on ProductionGateService, and an opt-in post-verify judge step.
