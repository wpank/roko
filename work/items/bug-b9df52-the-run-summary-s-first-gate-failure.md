+++
id = "bug-b9df52"
kind = "bug"
title = "The run summary's first_gate_failure milestone fires on the spec gate's expected red-on-base step: a run whose task checks all passed reads \"a gate failed on task T1\""
status = "open"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-serve/routes"]
created = 2026-10-20
updated = 2026-10-20
last_verified = 2026-10-20
source = "round-v7-hermes-pilot#E-p2-run-c91b63ea"
discovered_from = "agent:hermes-pilot (round v7 phase 2, run c91b63ea)"
anchors = ["crates/roko-serve/src/routes/runs.rs::is_gate_event"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn spec_gate_red_on_base_steps_are_not_a_gate_failure' crates/roko-serve/src && cargo test -p roko-serve --lib -- spec_gate_red_on_base_steps_are_not_a_gate_failure"
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
