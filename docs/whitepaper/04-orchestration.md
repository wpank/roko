Status: reviewed · budget 850 words · owner spec-ce1484

# 4 From plan to integrated work

This section follows a plan to integrated work, through the plan loop (hours) and the attempt loop inside it
(minutes).

## 4.1 Checks written before the work

A request (a prompt or a written spec) becomes a plan file of small tasks, written by the planner model
(`roko plan generate`) or a person and reviewable before it runs; a small request can run as one checked task
(`roko run`). Each task names its role, the files it expects to write (its write set), its dependencies, its
acceptance criteria and its checks: verify commands that pass on exit 0, written by the plan's author, never by the
implementing agent. A check for new behaviour must fail on the base, or it cannot tell a solution from no change; the
spec check refuses a plan in which a check can never fail or a task's checks all pass on the base.

## 4.2 Scheduling and isolation

The Graph engine runs the plan as a DAG, one node per task. A task starts once its dependencies settle and a slot is
free, unless its write set overlaps a running task's. Each task gets its own git worktree by default, shared by its
retries, and a run leaves the operator's checkout as it found it, apart from its records under `.roko/`.

## 4.3 The attempt

The model follows a fixed precedence (flag, task hint, escalation-ladder rung, learned router, default), and each
choice records its reason. Under a spending ceiling, a model call that the attempt's share of the plan's budget cannot
cover is refused, or cut off for a CLI agent. Three steps then judge the result:

1. **The pre-verify screen** reads the diff before any test runs. It rejects tampering (a deleted or emptied test, a
   removed assertion, a skip marker, an edited check) and empty or stub-only changes.
2. **The gate rungs** run the task's verify commands, then the project's required checks and any optional ones the
   task asks for, stopping at the first failure. Any shell command can be a check; a gate that could not run never
   passes.
3. **The verdict record** settles the attempt once, with its outcome, its blame (agent, infrastructure, harness or
   nobody) and the learning label (§6). The acceptance check is never an LLM judge (§6.3).

## 4.4 Retry, escalation and failover

A failed attempt is retried at once, in a new conversation carrying only distilled feedback (compile errors, failing
tests, trimmed gate output, a short diagnosis), not the failed transcript: models err more when their context holds
their own earlier errors [@sinha2025illusion].

Blame decides what changes. A task starts on its tier's rung of the escalation ladder, the cheapest for mechanical and
focused work, and climbs one rung after two failures blamed on the agent, at most twice.[^4-ladder] An infrastructure
failure goes to provider failover instead, within the attempt and spending no retry: the same model on another
provider, then, on the ladder, a rung above, never a cheaper model. A task that fails its last retry on its highest
reachable rung stops, and the dashboard asks a person to split or replan it. Cheap-first cascades
[@chen2024frugalgpt] are only as reliable as their escalation signal [@rajput2026cheap]; Roko's is an executable check
plus blame.

**Table 4.1.** The attempt loop's regulator card.

| Field | The attempt loop (L1) |
|---|---|
| Goal | A verified pass for this task |
| Reference | Checks written by the planner before the work, red on the base |
| Sensor | The pre-verify screen, then the gate rungs |
| Comparator | The verdict (outcome and blame), with distilled errors |
| Actuator | The next prompt, then the model tier, or provider failover |
| Bounds | Never edits a check, exceeds its budgets or writes outside its worktree |
| Clock | Once per attempt (minutes) |
| Records | One verdict record with its learning label; cost |

## 4.5 Integration and delivery

A passed attempt is committed to its plan branch; one that conflicts there fails with the conflicted paths named, and
its retry starts from a fresh checkout of the branch. When every task has passed, delivery merges the plan branch's
verified tip with the run branch in a separate checkout and runs the whole-plan check on that merge, since clean
merges can still break the build [@brun2011proactive]. The check gates the plan's success and its delivery: the run
branch takes the plan only once it passes. A plan that stops short delivers only its verified tasks' work, marked
partial, without a whole-plan check. Plans are delivered one at a time, so each failure traces to one plan, and a
resumed run continues delivery at its earliest unproved step. The operator takes the run branch with one merge.

## 4.6 Checkpoints, resume and supervision

A run can stop and resume without losing verified work: the Graph engine replays recorded model and tool outputs and
carries over attempt numbers, retry feedback, spend and ladder standing. A checkpoint is bound to its plan: an edited
plan resumes only if forced, and then starts afresh. After a crash, the worst case is one duplicate model call.

Gates judge artifacts; the run supervisor judges trajectories. Its watchers read each attempt's messages and tool
calls for looping, repeated failures, drift from the spec and overruns. Every five seconds it lets each attempt
continue, restarts it or fails the run, posting other advice to the dashboard. Operators can pause, resume, cancel or
retry a plan; pause and cancel leave durable receipts.

[^4-ladder]: Design defaults (`5e32264ce`): the rungs gpt-oss-120b, glm-4.7, gpt-5.4-mini and Claude Sonnet, and each
    tier's start rung, in `crates/roko-core/src/config/routing.rs`; the climb rule in
    `crates/roko-cli/src/graph_task_dispatch/ladder.rs`; the failover order in
    `crates/roko-cli/src/graph_task_dispatch/failover.rs`.
