Status: draft · budget 550 words · owner spec-ce1484

# 7 Roko in use

This section describes how Roko is used on its own development, gives one recorded figure from that use, and follows
one task through the attempt and plan loops.

## 7.1 Self-hosting

Roko develops itself through the flow it offers its users. A request becomes a plan file of small tasks, each carrying
checks written at planning time; the plan is reviewed, run with `roko plan run` on the Graph engine, resumed from its
checkpoint if interrupted, and read back through the dashboard and status commands. Ad-hoc work takes the same path: `roko run` writes a one-task plan and runs it on the same
engine, and a bounded quick mode runs a small self-edit under a deadline and a turn cap, checked by the task's own
verify command. A task can name the work items it finishes, and a sync step closes them once the task passes, so plan
runs keep the project's record of open work current.

Findings from outside a run enter the same way. Browser reviews of Roko's running web portal produced findings that
became plans Roko ran, and one of those plans turned a legibility requirement into a computed check of text
contrast, so a property once judged by eye became an executable check.[^7-cases]

## 7.2 The portal build

Roko ran most of the build of its own web portal: 16 plans and 173 tasks, 168 of them gate-verified, for $174.87 of
recorded agent spend. Frontier sessions wrote those plans and supervised the run; their cost is not in that figure.
The figure counts what Roko recorded (the agents it dispatched, their attempts and their gate verdicts) over 25.6
agent-hours, with a median task cost of $0.83 and a 90th percentile of $1.83.[^7-portal]

Every task named its model, Claude Sonnet run through the Claude CLI, so the build exercised the attempt and plan loops
(checks written in advance, retries, a worktree per attempt, integration and resume) rather than model routing.
Because planning and supervision ran outside Roko, the figure is not a cost per verified success in the sense of §6,
which counts planning.

## 7.3 One task on the escalation ladder

In a capped run on a small Python package, two five-task plans ran against tests that the planner had written before
the work and that failed on the starting code. One task had to reproduce the standard library's text-wrapping
functions exactly without importing them. It failed twice on the cheapest model, gpt-oss-120b, both times at its turn
cap. Two failures blamed on the agent move a task one rung up the escalation ladder, so its third attempt ran on
gpt-5.4-mini (this run's ladder had no middle rung), and its record names the reason: escalation. The task's standing
on the ladder is stored with its retry feedback beside the plan's checkpoint, so it carried across the run's two
resumes. The task passed on the top rung, Claude Sonnet, in 17 turns. The task that depended on it then passed on its
first attempt, the whole-plan check passed on the merge, and the plan was delivered to its run branch.[^7-ladder]

These are records of use, not experiments: none of these runs had a comparison arm, and this paper draws no
comparative conclusion from them.

[^7-cases]: `evidence/2026-09-29-field-cases.md` (sha256 `d03e50476fb0`), CASE-006: the portal's browser passes of
    2026-09-28 and 2026-09-29 and the plans built from their findings.
[^7-portal]: `evidence/2026-09-29-b7-real-run-evidence.md` (sha256 `799b6a2b6184`), "TL;DR": Roko's attempt, verdict
    and cost records for the portal plans, attempts to 2026-09-29 07:41Z, costs as recorded. The frontier sessions
    that wrote the plans and supervised the run are not in the figure.
[^7-ladder]: `evidence/2026-10-02-live-cheap-model-run.md` (sha256 `813172c96b88`): "Setup", the run table (the
    second plan and its two resumes) and the "Escalation" row.
