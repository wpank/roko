+++
id = "gap-85f102"
kind = "gap"
title = "LLM-judge and fact-check gate rungs never run in production"
status = "open"
triage = "verified"
severity = "p2"
subsystem = ["roko-gate/oracles"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-gate/src/llm_judge_gate.rs::JudgeOracle", "crates/roko-gate/src/llm_judge_gate.rs::LlmJudgeGate"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'cargo test -p roko-gate llm_judge'
+++

`LlmJudgeGate` (rung 6) has no `JudgeOracle` implementation outside tests (`llm_judge_gate.rs:348`, `:359`, `tests/rungs.rs:141`), and `FactCheckGate` (rung 5) was wired only on the removed Runner-v2 gate path.
`ProductionGateService` uses `RungExecutionConfig::default()` and the Graph engine runs authored verify steps instead of the rung ladder, so the semantic rungs of the 7-rung pipeline never execute.
Fix: a production judge oracle, an injectable `RungExecutionConfig` on `ProductionGateService`, and an opt-in (advisory by default) post-verify judge step in the Graph engine.
