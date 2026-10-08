Status: draft · budget 850 words · owner spec-ce1484

# 4 From plan to integrated work

This section follows a plan to integrated work, through the plan loop (hours) and the attempt loop inside it
(minutes).

## 4.1 Checks written before the work

Planning is feedforward: it fixes what counts as done before any agent starts. A request (a prompt or a written spec)
becomes a plan file of small tasks: the planner model writes it (`roko plan generate`) or a person does, and the author
can review and edit it before it runs. A small request can run as one checked task (`roko run`). Each task names its
role, the files it may write (its write set), its dependencies and its checks: acceptance criteria and verify commands
that pass on exit 0. The plan's author writes the checks; the implementing agent never does. A feature's or fix's check must be red on the base,
failing on the unchanged code, or it cannot tell a solution from no change. Before dispatch, the spec check refuses a
plan in which a check can never fail or, when its optional base run is on, every check already passes on the base.

## 4.2 Scheduling and isolation

The Graph engine runs the plan as a DAG, one node per task. A task starts once its dependencies settle and a slot is
free, unless its write set overlaps a running task's, so tasks that write the same files never run together. Each task
gets its own git worktree by default; the operator's checkout is never touched.

## 4.3 The attempt

The model follows a fixed precedence (flag, task hint, escalation-ladder rung, learned router, default), and each
choice records its reason. Each call reserves its share (`max_turn_usd`) against the plan's spending ceiling before it
runs. Three steps then judge the result:

1. **The pre-verify screen** reads the diff before any test runs. It rejects tampering (a deleted or emptied test, a
   removed assertion, a skip marker, an edited check) and empty or stub-only changes, since coding agents do edit
   tests to make them pass [@zhong2025impossiblebench].
2. **The gate rungs** run the task's verify commands, then the project's required checks, stopping at the first
   failure. Any shell command can be a check. Plan complexity sets how many of the seven rungs run; failures add
   more. A gate that could not run never passes.
3. **The verdict record** settles the attempt once, with its outcome, its blame (agent, infrastructure, harness or
   nobody) and the learning label (§6). The acceptance check is never an LLM judge (§6.3).

## 4.4 Retry, escalation and failover

A failed attempt is retried at once, in a new conversation carrying only distilled feedback (compile errors, failing
tests, trimmed gate output, a short diagnosis), not the failed transcript: models err more when their context holds
their own earlier errors [@sinha2025illusion].

Blame decides what changes. After two failures blamed on the agent, a task climbs one rung of the escalation ladder,
cheapest model first, at most twice.[^4-ladder] An infrastructure failure moves the call sideways: provider failover
sends it to the same model on another provider, then to configured fallbacks, within the same attempt and without
spending a retry, so an outage never pushes work to a costlier tier. Failure on the top rung goes to a person.
Cheap-first cascades [@chen2024frugalgpt] are only as reliable as their escalation signal [@rajput2026cheap]; Roko's
is an executable check plus blame.

**Table 4.1.** The attempt loop's regulator card.

| Field | The attempt loop (L1) |
|---|---|
| Goal | A verified pass for this task |
| Reference | Checks written by the planner before the work, red on the base |
| Sensor | The pre-verify screen, then the gate rungs |
| Comparator | The verdict (outcome and blame), with distilled errors |
| Actuator | The next prompt, then the model tier, or a sideways provider switch |
| Bounds | Never edits a check, exceeds its budgets or writes outside its worktree |
| Clock | Once per attempt (minutes) |
| Records | One verdict record with its learning label; cost |

## 4.5 Integration and delivery

A passed attempt is committed to its plan branch, which later attempts start from; an attempt that conflicts there
fails with the conflicted paths named, and its retry starts from the plan branch, where the other work is already in
place. When every task has passed, delivery merges the plan branch's verified tip with the run branch in a separate
checkout and runs the whole-plan check on that merge. The check gates both the plan's success and its delivery: the
run branch moves only once it passes. Plans are delivered one at a time, so each failure traces to one plan, and a
resumed run continues delivery at its earliest unproved step. Merges without a textual conflict can still break the
build [@brun2011proactive], so the checks run on each attempt and the whole-plan check on each delivery's merge. The
operator takes the run branch with one merge.

## 4.6 Checkpoints, resume and supervision

A run can stop and resume without losing verified work: the Graph engine recomputes deterministic steps, replays
recorded model and tool outputs, and carries over attempt numbers, retry feedback, spend and ladder standing. A task
whose definition changed is queued again. After a crash, the worst case is one duplicate model call.

Gates judge artifacts; the run supervisor judges trajectories. Its watchers read each attempt's messages and tool
calls for looping, repeated failures, drift from the spec and overruns, and every five seconds it makes one of five
decisions per attempt: continue, nudge, force-advance, restart, or fail the run. Operators can pause, resume, cancel
or retry a plan; pause and cancel leave durable receipts.

[^4-ladder]: Design defaults (`1f860d408`): the rungs gpt-oss-120b, glm-4.7, gpt-5.4-mini and Claude Sonnet in
    `crates/roko-core/src/config/routing.rs`; the climb rule in `crates/roko-cli/src/graph_task_dispatch/ladder.rs`.
