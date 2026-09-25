---
plan: portal-plan-execution
---

# Portal plan execution — live progress wiring

The plan editor page (`/work/editor`) has a working Execute button that posts
to the API and shows a toast notification. After that the UI goes static: the
task list does not update as the plan runs and agent output is invisible.

The infrastructure for live updates already exists. The global StateHub SSE
stream fans `task_started`, `task_completed`, `task_failed`, and
`agent_output_line` events into a Zustand store that every component can read.
This plan connects those events to the editor page in two implementation tasks
followed by a reviewer pass.

1. An implementer wires the live task status map from the dashboard store into
   the task list, adding a coloured `StatusLED` pill to each row while a plan
   is executing.
2. An implementer adds a collapsible agent output streaming panel below the
   task list that shows `AgentOutputStream` for the currently running agent.
3. A reviewer audits both changes, checks for stale-closure and hook-ordering
   issues, and writes a verdict document.

All changes are confined to the portal app. No new SSE connections are opened;
the existing global `useStateHubSSE` connection supplies all events.
