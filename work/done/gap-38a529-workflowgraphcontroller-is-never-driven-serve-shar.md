+++
id = "gap-38a529"
kind = "gap"
title = "WorkflowGraphController is never driven: serve shared runs and ACP build it and mark it skipped"
status = "done"
triage = "verified"
severity = "p3"
goal = "hermes"
subsystem = ["roko-execution/workflow", "roko-serve/shared_runs", "roko-acp"]
created = 2026-09-29
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "39feebc07"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:w3d-roko-run"
anchors = ["crates/roko-execution/src/workflow/controller.rs::WorkflowGraphController", "crates/roko-serve/src/routes/shared_runs.rs:519", "crates/roko-acp/src/runner.rs:668", "crates/roko-cli/src/explain.rs:268"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = '! grep -q "WorkflowTermination::Skipped" crates/roko-serve/src/routes/shared_runs.rs crates/roko-acp/src/runner.rs && ! grep -rq "WorkflowGraphController" crates/ --include="*.rs" && grep -qw "fn share_with_a_prompt_runs_it_through_the_runtime" crates/roko-serve/src/routes/shared_runs.rs && cargo test -p roko-serve --lib share_with_a_prompt_runs_it_through_the_runtime'

[closed]
at = 2026-10-02
at_ts = "2026-10-01T23:45:19Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
claimed_at = "2026-10-01T18:58:38Z"
forced = false
evidence = "Gate 6d on 9eacfde5f plus its fixes, re-checked at c9e78d12d and merged as 39feebc07 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean; lib tests pass (roko-cli 3407, roko-agent 2276, roko-core 1980, roko-learn 1228, roko-serve 1012, roko-acp 219, roko-compose 562, roko-execution 192, roko-gateway 43); all eight canaries with the new default-isolation canary, golden_path_suite, secret_canary, C2, graph_plan_callers, cost_dedup, phase0_wiring, run_serve_share, graph_timeout_matrix (6), plan_prepare_full and the gateway pipeline contract pass; bin tests pass; scripts/test_run_evidence_graph.py 9/9 against the gate binary; Cargo.lock unchanged; route snapshot matches. Implemented in this round; the item's notes name the change and its test."
+++

`WorkflowGraphController` (roko-execution) is constructed in two places, and both are stubs. Serve's shared-run route (routes/shared_runs.rs:510-530, commented "#276: WorkflowEngine deleted — resolve template and build a report stub") and the ACP runner (roko-acp/src/runner.rs:668-675) create a controller and immediately set `termination = WorkflowTermination::Skipped`, so no phase ever executes. `roko run` no longer uses it: w3d moved it to a one-task plan run through `run_graph_plan`. CLAUDE.md ("`roko run` uses graph templates via `WorkflowGraphController`") and `roko explain` (explain.rs:268) still describe the controller as the driver.

Fix: execute shared runs and ACP prompts for real (drive the controller's phases, or use `run_graph_plan` as `roko run` does), or remove the controller and correct the docs.

Re-checked 2026-09-29 at d9e79e9d8: unchanged. c41e78c7a touched shared_runs.rs but left the stub in place. The ACP stub is run_with_workflow_engine (roko-acp/src/runner.rs:602), which ACP slash commands and bridge_events call.

## Notes

- 2026-10-01 (wk-serve2): implemented on work/bug-1cb461; cargo verification deferred to the batch check.
  Premise confirmed at `825d45f97`: both callers built the controller and set `Skipped`, so ACP workflow-mode
  prompts failed ("workflow engine reported unsuccessful run"), `/express` and `/full` silently did nothing, and
  serve's prompt shares stored a skipped stub. ACP's working `run_workflow_pipeline` ran only with
  `ROKO_ACP_LEGACY`. History: `run_with_workflow_engine` drove `WorkflowEngine` until #276 deleted it
  (`70a342bff`) and left the stub.
- Decision: deleted the controller, because nothing should drive it. The Graph engine is the only plan executor,
  `roko run` builds its `WorkflowRunReport` from a one-task Graph run (`roko-cli/src/run.rs::run_prompt`), serve's
  run endpoints dispatch through `CliRuntime`, and ACP has its own pipeline.
  - Removed `crates/roko-execution/src/workflow/` (controller, report adapter, templates, review parser; no other
    users) and its re-exports. `git show 825d45f97:crates/roko-execution/src/workflow/review_parser.rs` restores the
    reviewer-output parser if one is wanted.
  - Serve: a share request with a prompt runs it once through `state.runtime.run_once`, the dispatch
    `POST /api/run` uses. `run_prompt` was not used because it installs process-wide signal handlers. Removed the
    request's unused `workflow` and `enabled_gates` fields (serde ignores those keys now). Test `share_with_a_prompt_runs_it_through_the_runtime`.
  - ACP: workflow-mode prompts and `/express` / `/full` always run `run_workflow_pipeline`. Removed
    `run_with_workflow_engine`, `GraphEngineOptions`, `AcpWorkflowRoute` (its GraphCanary/ReplayOnly were
    unimplemented placeholders), the cost sink only that path wrote, and `workflow_template_name`.
    `ROKO_ACP_LEGACY` is now ignored; its env-registry entry says so.
  - Docs: `roko explain` (cells) and `docs/v3/00-INDEX.md` now name the Graph engine. `docs/v2` and the v3 depth
    docs still mention the controller as design history.
