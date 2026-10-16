Status: reviewed · budget 550 words · owner spec-ce1484

# 7 Roko in use

This section describes how Roko is used on its own development, gives one recorded figure from that use, and follows
one task through the attempt and plan loops.

## 7.1 Self-hosting

Roko is built to develop itself through the flow it offers its users (§4). A plan is reviewed, run with
`roko plan run`, resumed from its checkpoint if interrupted, and read back through the dashboard and status commands;
`roko run` runs ad-hoc work as a one-task plan, and a bounded quick mode runs a small self-edit under a deadline and a
turn cap, checked by the task's own verify command.

Findings from outside a run enter the same way. Browser reviews of Roko's running web portal produced findings that
became plans Roko ran, and one of those plans turned a legibility requirement into a computed check of text
contrast, so a property once judged by eye became an executable check.[^7-cases]

## 7.2 The portal build

Roko ran most of the build of its own web portal: 16 plans and 173 tasks, 168 of them gate-verified, for $174.87 of
recorded agent spend. Frontier sessions wrote and audited those plans, fixed Roko during the run, made its parallel
worktrees, merged their work by hand and supervised; their cost is not in that figure. The figure counts what Roko
recorded (the agents it dispatched, their attempts and their gate verdicts) over 25.6 agent-hours, with a median task
cost of $0.83 and a 90th percentile of $1.83.[^7-portal] Every task named its model, Claude Sonnet run through the
Claude CLI, so the build exercised checks written in advance, retries and resume, not model routing.

## 7.3 One task on the escalation ladder

In a capped run on a small Python package, two five-task plans ran against tests written before the work that failed
on the starting code. One task had to reproduce the standard library's text-wrapping functions exactly without
importing them. It failed twice on the cheapest model, gpt-oss-120b, both times at its turn cap, so its third attempt
ran one rung up, on gpt-5.4-mini, and its record names the reason: escalation. Two failures were then counted on that
rung, a finished answer that output screening withheld and a turn cap hit by a failover substitute, and the task
climbed to the top rung, Claude Sonnet, where it passed in 17 turns. Its standing on the ladder, kept with its retry
feedback beside the plan's checkpoint, carried across the run's two resumes. The task that depended on it then passed
on its first attempt, the whole-plan check passed on the merge, and the plan was delivered to its run branch. The run
was not unattended: an operator stepped in four times, clearing the isolation controls that blank answers from the
middle rung's model had set, and changing the ladder to avoid that rung.[^7-ladder]

These are records of use, not experiments: no run had a comparison arm, and this paper draws no comparative
conclusion.

[^7-cases]: `evidence/2026-09-29-field-cases.md` (sha256 `d03e50476fb0`), CASE-006: the portal's browser passes of
    2026-09-28 and 2026-09-29 and the plans built from their findings.
[^7-portal]: `evidence/2026-09-29-b7-real-run-evidence.md` (sha256 `799b6a2b6184`), "TL;DR" and the "Isolation and
    merge" row: Roko's attempt, verdict and cost records for the portal plans, attempts to 2026-09-29 07:41Z, costs as
    recorded. The frontier sessions that wrote the plans and supervised the run are not in the figure.
[^7-ladder]: `evidence/2026-10-02-live-cheap-model-run.md` (sha256 `813172c96b88`): "Setup", the run table (the
    second plan and its two resumes), the "Escalation" row, "What broke" 4 and 5, and "Operator interventions" 1 to
    4. The causes it found are traced in `evidence/2026-10-02-live-defect-root-causes.md` (sha256 `5df7d5221539`).
