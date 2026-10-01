+++
id = "q-9852b5"
kind = "question"
title = "Should a run's cancel stop its gate commands now that they join the agent PID registry?"
status = "done"
triage = "verified"
severity = "p3"
goal = "core"
size = "S"
subsystem = ["roko-cli/graph_execution"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d1e3c5681"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-b367bf"
anchors = ["crates/roko-cli/src/graph_execution/plan_runner.rs::live_agent_process_trees", "crates/roko-gate/src/cancel_safe_command.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-b367bf"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -n 'register_spawned_pid' crates/roko-gate/src/cancel_safe_command.rs"

[[verify]]
command = "cargo test -p roko-cli --lib a_runs_cancel_stops_its_own_gate_command_only"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T19:41:37Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T17:37:48Z"
forced = false
evidence = "Gate 6c on 4bacc9d88 and its fix-up 060b62813, merged as d1e3c5681 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 14 crates; lib tests pass (roko-cli 3395, roko-agent 2269, roko-core 1977, roko-learn 1225, roko-serve 1010, roko-gate 697, roko-compose 562, roko-graph 487, roko-dreams 260, roko-execution 245, roko-neuro 239, roko-acp 211, roko-gateway 101); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, merge_proof, plan_conversion, job_lifecycle and job_runner_integration tests pass; bin 445; Cargo.lock unchanged; route inventory tests and --check-snapshot pass. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

## Problem

With gap-b367bf, gate commands register in roko_agent's PID registry, so find-65ff6b's cancel (which stops a run's registered agent processes, scoped per serve run) now also stops that run's gate commands. That is probably the desired behaviour, but it is untested, and an unscoped CLI run owns every unscoped process.

## Plan

Confirm the intended behaviour, then add a test that a run's cancel stops its own gate command and leaves another run's alone.

## Done when

- A decision is recorded, and the test exists.

## Notes

- Reported on 2026-10-01 by the worker on gap-b367bf, during the evening close-out round.
- 2026-10-01 (wk-scheduler): decided yes, for the run's own scope (team-lead's call). A gate command is the run's
  work like its agents, and once the run stops its verdict teaches nothing (bug-82cbef settles it as cancelled). A
  gate command registers under the spawn scope of the thread that starts it, so a `roko serve` run's cancel stops its
  own gate commands and leaves other runs' and the server's generation, revision and chat agents alone
  (find-65ff6b). A CLI run has no scope and owns every unscoped process of its process, gate commands included,
  which is right: the runner lock allows one plan executor at a time.
- 2026-10-01 (wk-scheduler): implemented on work/bug-28b604; cargo verification deferred to the batch check.
  Test `a_runs_cancel_stops_its_own_gate_command_only` in plan_runner.rs runs two gate commands on two scoped
  threads; one run's cancel (`terminate_in_flight_agents`) SIGTERMs its own command and not the other run's.
