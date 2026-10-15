---
plan: demo-hello
---

# Roko plan-runner demo

This deliberately small plan proves the agent, task execution, verification,
state persistence, and approval TUI path without launching the full repository
backlog.

The task writes `demo/roko-plan-smoke-output.md`, whose last line carries the
day's UTC date, so every run makes a change you can see in its diff. A copy from
an August run, without a date, is checked in; the date keeps the task's check
from passing before an agent has done anything, which would make roko refuse
the plan (spec gate HF3). Leave the run's output on its own branch: it is demo
output, not something to merge.
