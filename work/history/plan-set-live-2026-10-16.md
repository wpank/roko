# Live plan set: three independent plans side by side (2026-10-16)

> Backlog task 3119 (work item gap-c013c5, package PK16). One paid run, under decision 3114's $2 cap
> (Will, 2026-10-15). The operator was a Claude agent; it made no intervention. The full write-up, with
> the harness and its fences, is `tmp/status-quo/2026-10-16-live/golden-path.md` in the main checkout, and
> the derived evidence is in `tmp/status-quo/2026-10-16-live/evidence/plan-set/` (both gitignored).

## Result

**All three plans started before the first finished.** All three were delivered into the run's batch
branch, and the run cost $0.142.

| | |
|---|---|
| Commit | `5e32264ce` (binary `roko 0.1.0 … git 5e32264ce`, dev profile, clean tree) |
| Command | `roko --json plan run plans --max-parallel-plans 3 --no-tui --log-file <evidence>/run.jsonl`, run from the scratch repo |
| Run id | `graph-3cc12de7-d6eb-498b-8f65-018a128e9831` (event log and `--log-file`); per-plan run records `graph-ps-dates-54761af0-259f-4e73-966f-b0fe4ddcdb9e`, `graph-ps-text-78ea777f-786b-4005-b395-1ff27ba58084`, `graph-ps-units-13699c8a-0fae-44cd-81eb-06e1acb784d7` |
| Exit | 0, `run.completed` outcome `succeeded`; wall time 35.1 s |
| Worktrees | per-task worktrees on (the default); `max_plan_usd = 0.6` per plan, `max_turn_usd = 0.25` |
| Admission | `plan_set_loaded`: all three plans in wave 0, `conflicts_with = []` and `depends_on = []` for each; no `plan waits` line |
| Deliveries | 3 of 3 `delivered` into `roko/batch/20261016T144458Z-651be4f6`: ps-dates (fast-forward, `98e24e3`), then ps-units (merge `d85e470`), then ps-text (merge `aac41bd`) |
| Cost | $0.141548 by roko (`total_cost_usd`; 7 verdicts, all `cost.source = provider_usage`); $0.144020 by the tee proxy's wire count, the difference being 8 rung-probe calls and 1 helper call |
| Interventions | 0 |

## Timeline (from `dashboard.plan_started` and `dashboard.plan_completed`, UTC)

| Plan | Started | Completed | Tasks and attempts (all on the cheap rung, gpt-oss-120b) |
|---|---|---|---|
| ps-dates | 14:44:59.159 | 14:45:08.431 | D1 passed; D2 passed |
| ps-text | 14:44:59.167 | 14:45:20.209 | X1 passed; X2 failed its verify once, then passed |
| ps-units | 14:44:59.183 | 14:45:09.122 | U1 passed; U2 passed |

All three plans started within 24 ms of each other, and the last of them started 9.2 s before the first
finished, so all three plans started before the first finished.

## Fixture

The scratch git repo lived outside roko's tree (in the operator's scratchpad), with three Python packages,
`units`, `datesx` and `textx`, and their unit tests. The tests were committed in the seed, and each was
checked red on the seed and green against a reference solution before the plans were frozen. There were three
plans, one per package. Each had two implementer tasks with disjoint `files`, one verify step per task (the
task's test module) and a `[meta] verify` scoped to its own package. No task depended on another plan.

## Checks after the run

On an export of the batch branch, outside any roko process, the run added exactly the six declared files
(`units/length.py`, `units/temperature.py`, `datesx/leap.py`, `datesx/duration.py`, `textx/caesar.py`,
`textx/rle.py`). No file under any `tests/` directory changed. All three packages' suites pass: 13, 13 and
11 tests.

## What this does not show

The tasks are small Python tasks: one seed, one run. The run shows that plan-set scheduling admits three
independent plans at once under per-task worktrees and delivers each in turn. It does not measure throughput
against a serial run. For q-61b3f8 (whether plan-set parallelism should be on by default), the evidence is
that it worked on its first live run with disjoint `files`. Footprint admission had nothing to refuse, so
the refusal path is not exercised here.
