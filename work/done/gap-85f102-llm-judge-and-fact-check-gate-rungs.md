+++
id = "gap-85f102"
kind = "gap"
title = "LLM-judge and fact-check gate rungs never run in production"
status = "done"
triage = "verified"
severity = "p2"
goal = "core"
subsystem = ["roko-gate/oracles"]
created = 2026-09-28
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "f88210c84"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-gate/src/llm_judge_gate.rs::JudgeOracle", "crates/roko-gate/src/agent_judge.rs::AgentJudgeOracle", "crates/roko-gate/src/llm_judge_gate.rs::LlmJudgeGate", "crates/roko-gate/src/production_service.rs:331"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'RungExecutionConfig::default()' crates/roko-gate/src/production_service.rs && grep -rlE 'impl .*JudgeOracle for' crates/*/src | grep -qv 'crates/roko-gate/src/llm_judge_gate.rs' && grep -rq 'AgentJudgeOracle' crates/roko-cli/src && grep -rqw 'fn a_failing_judge_blocks_only_when_configured' crates/roko-cli/src && cargo test -p roko-cli --lib a_failing_judge_blocks_only_when_configured"

[closed]
at = 2026-10-02
at_ts = "2026-10-02T08:27:41Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T16:11:59Z"
forced = false
evidence = "opt-in [gates] llm_judge scores a passing attempt's diff through AgentJudgeOracle, advisory by default, blocking when configured; a_failing_judge_blocks_only_when_configured passes (wk-gates 9d0055bbc); gate 6i passed at f4347b8eb (cargo check, clippy -D warnings, lib tests of roko-acp/cli/core/gate/learn/runtime/serve, canaries C1-C8 and integration tests, roko-acp integration tests, roko-cli doctor, learning_wiring_census and graph_plan_callers tests, 447 bin tests, run_evidence py); two fixes in 09aa7e009 re-tested; merged in f88210c84"
+++

`LlmJudgeGate` (rung 6) has no `JudgeOracle` implementation outside tests (`llm_judge_gate.rs:348`, `:359`, `tests/rungs.rs:141`), and `FactCheckGate` (rung 5) was wired only on the removed Runner-v2 gate path.
`ProductionGateService` uses `RungExecutionConfig::default()` and the Graph engine runs authored verify steps instead of the rung ladder, so the semantic rungs of the 7-rung pipeline never execute.
Fix: a production judge oracle, an injectable `RungExecutionConfig` on `ProductionGateService`, and an opt-in (advisory by default) post-verify judge step in the Graph engine.

Checked 2026-09-29 at d9e79e9d8: still open. Graph dispatch now has a best-effort quality_judge (graph_task_dispatch.rs:2274-2280, from 725f21e05) that asks a cheap model for a score only when verify failed and passes it to GateGamingDetector; it never gates and does not run LlmJudgeGate or FactCheckGate. None of the three fix parts exist yet: a production JudgeOracle, an injectable RungExecutionConfig on ProductionGateService, and an opt-in post-verify judge step.

## Notes

2026-10-01 (wk-gates): partly implemented on work/bug-951930; cargo verification deferred to the batch check. The
first two of the three fix parts are done:
- A production judge oracle: `roko-gate/src/agent_judge.rs::AgentJudgeOracle` sends the LLM-judge prompt to a roko
  `Agent` and reads the first number in its answer as the score. A failed run or an answer without a number is an
  error, which the gate turns into an advisory pass unless it is blocking.
- An injectable rung config: `ProductionGateService` and `DefaultGateService` take a `RungExecutionConfig` through
  `with_rung_config` instead of always using the default.
Left: nothing in production hands an oracle to either service yet (`plan_runner.rs::plan_cell_resources` still
builds `ProductionGateService::new()`), and the opt-in, advisory post-verify judge step on the default Graph path
does not exist. That step needs a `[gates]` setting, the dispatcher's cheap agent wrapped in `AgentJudgeOracle`, and
the attempt's diff as the `JudgePayload`. The `[[verify]]` command lost the parenthetical note that made it a shell
syntax error, and it now also requires `AgentJudgeOracle` to be used in roko-cli, so it keeps failing until that
wiring lands. (`cargo test -p roko-gate llm_judge` runs the unit tests with test oracles, which pass while the rungs
never run in production.)

2026-10-02 (wk-gates): the third part is implemented on work/bug-951930; cargo verification deferred to the batch
check. Three new `[gates]` keys:
- `llm_judge` (default `false`) adds a judge step to Graph verify, `graph_task_dispatch/judge_step.rs`. Once every
  verify step passed, the dispatcher's cheap helper model, wrapped in `AgentJudgeOracle` behind an `LlmJudgeGate`,
  scores the attempt's diff (`attempt_diff`, then `patch`) against the task's title and description.
- `llm_judge_min_score` (default `0.8`) is the passing score.
- `llm_judge_blocking` (default `false`) makes a low score, or a judge that cannot answer, fail the attempt. The
  failure goes into the attempt's failures, so the retry feedback carries the judge's reason.
The verdict is advisory by default: logged, shown on the dashboard (`gate_result`), counted in the gate metrics and
appended to `.roko/learn/judge-calibration.jsonl`. An attempt with no diff, or a run without a helper model, is not
judged. Documented in `docs/v3/04-EXECUTION.md`. Test: `a_failing_judge_blocks_only_when_configured`. A fake CLI edits
the repo for the attempt, and an OpenAI-compatible mock is the helper model. The test checks four cases: off, advisory
low score, blocking low score and blocking passing score. The verify now also runs that test.
Not done, and not part of this item's fix list: production has no fact-check `SearchOracle`, and the rich topology's
`ProductionGateService` (`plan_cell_resources`) still gets the default rung config.
