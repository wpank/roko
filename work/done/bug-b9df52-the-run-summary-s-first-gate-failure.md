+++
id = "bug-b9df52"
kind = "bug"
title = "The run summary's first_gate_failure milestone fires on the spec gate's expected red-on-base step: a run whose task checks all passed reads \"a gate failed on task T1\""
status = "done"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-serve/routes"]
created = 2026-10-20
updated = 2026-10-21
last_verified = 2026-10-21
last_verified_rev = "173b06d94"
source = "round-v7-hermes-pilot#E-p2-run-c91b63ea"
discovered_from = "agent:hermes-pilot (round v7 phase 2, run c91b63ea)"
anchors = ["crates/roko-serve/src/routes/runs.rs::is_gate_event"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn spec_gate_red_on_base_steps_are_not_a_gate_failure' crates/roko-serve/src && cargo test -p roko-serve --lib -- spec_gate_red_on_base_steps_are_not_a_gate_failure"

[closed]
at = 2026-10-21
at_ts = "2026-10-21T01:08:28Z"
commit = "173b06d94"
by = "roko-speed"
executor = "claude-agent"
via = "manual"
size = "S"
claimed_at = "2026-10-20T23:57:33Z"
forced = false
evidence = "173b06d94. routes/runs.rs: is_spec_gate_event (spec_gate_started, spec_gate_step, spec_gate_finished) is left out of the first_gate_failure milestone, since the spec gate's checks must fail on the base; a task it refuses still ends the run refused (spec.gate.refused), and the /gates listing and the gate counts are unchanged. The literal verify passes: spec_gate_red_on_base_steps_are_not_a_gate_failure (run c91b63ea's shape: four red-on-base steps, then T1 and T2 pass: milestones started, first_task_accepted, completed; the same run with T2's own check failing once: 'gate verify[0:test] failed on task T2', not T1). run_summary_reports_verdict_cost_and_milestones still passes. Full lib suites, with bug-11e54b: roko-cli 3951, roko-core 2046, roko-learn 1391, roko-serve 1076 of 1077 (the failure is bug-904a1c's known JWKS flake, jwks.rs:878, at load about 100); fmt and clippy clean."
+++

## Problem

Round v7, Undertow phase 2, fresh run `c91b63ea` (roko d7d1fbb8b). The run passed 4 of 4 tasks at their first tries,
and every task check passed. Yet `GET /api/runs/c91b63ea…/summary` lists this milestone:

```json
{"at": 1792523436372, "kind": "first_gate_failure", "text": "a gate failed on task T1"}
```

Its time, 21:10:36, falls before the plan started (21:11:32). The event behind it is the spec gate's first step on
the base:

```json
{"type": "spec_gate_step", "task_id": "T1", "step": 1, "run": 1, "passed": false, "cached": false, "ms": 80}
```

On the base, a spec check must fail (red-on-base). So `passed: false` there is the gate's expected result, not a
failed gate. The summary reports a failure in a run that had none, and the Hermes plugin's receipt and milestones
read from that summary.

## Where

- `crates/roko-serve/src/routes/runs.rs`: `is_gate_event` (line 2041) is
  `kind.starts_with("gate.") || kind.starts_with("gate_") || kind.contains("gate")`. That matches
  `spec_gate_started`, `spec_gate_step` and `spec_gate_finished`.
- The summary loop uses it near line 2485: any gate event with `passed: false` (or a kind containing "failed")
  becomes the `first_gate_failure` milestone, `"{gate} failed on task {task_id}"`.

## Plan

- Leave the spec gate's own events out of `first_gate_failure`, or give them their own milestone. For example:
  "spec gate: 8 checks red on base, as required". A spec gate step that *passes* on the base is the one worth
  flagging.
- Keep the existing summary test, which has a real `gate test failed on task t1`. Add a case with `spec_gate_step`
  `passed: false` rows and no task gate failure, and assert there is no `first_gate_failure`.

## Done when

A run whose only `passed: false` gate rows are its spec gate's red-on-base steps has no `first_gate_failure`
milestone. Checked by the test in `[[verify]]`.
