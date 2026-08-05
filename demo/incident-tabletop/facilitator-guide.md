# Agent Orchestration Incident Tabletop: Provider Failure Cascade

## Purpose

This guide enables a human facilitator to run a tabletop exercise that
simulates a cascading failure across the agent-orchestration stack—from
LLM-provider outages through timeout kills, gate failures, worktree
conflicts, and interrupted plan state.  The scenario is derived from two
upstream artifacts:

- **Risk register**: `demo/incident-tabletop/risk-register.csv`
- **Scenario definition**: `demo/incident-tabletop/scenario.json`

The facilitator reads this guide aloud, injects events at the prescribed
minutes, and records participant decisions and evidence.  Participants
practice diagnosing, communicating, and recovering without modifying
production systems.

## Preparation

Before convening the exercise, the facilitator must:

1. **Print or share** the risk register
   (`demo/incident-tabletop/risk-register.csv`) so every participant can
   reference RISK-01 through RISK-05 during the exercise.
2. **Open** the scenario definition
   (`demo/incident-tabletop/scenario.json`) to confirm inject timing and
   expected actions.
3. **Assign one person to each role** listed in § Roles below.  If fewer
   than six participants are available, combine QA Lead and Reliability
   Engineer, but keep Incident Commander as a dedicated seat.
4. **Set a timer** visible to all participants.  The exercise runs
   approximately 30 minutes of scenario time (injects at minutes 0, 4, 9,
   15, and 22) plus debrief.
5. **Prepare an evidence log** (shared doc or whiteboard) to capture
   decisions, rationale, and artifact references as the exercise
   progresses.  This log becomes the primary retained evidence.

## Roles

| Role | Responsibility |
|------|---------------|
| **Incident Commander** | Owns the response timeline, convenes the bridge, approves all recovery decisions, and ensures stakeholder communication cadence is maintained. |
| **Agent Runtime Lead** | Diagnoses provider and dispatch failures, validates fallback model configuration, and confirms tier-based escalation is active. |
| **Operations Engineer** | Monitors timeout thresholds, assesses partial workspace state from killed tasks, and decides between tuning timeouts and resuming from checkpoint. |
| **QA Lead** | Triages gate failures, determines whether to retry with an escalating model or adjust adaptive thresholds, and documents the trade-off. |
| **Platform Infrastructure Lead** | Manages worktree isolation, enforces `workspace_lock_secs`, and resolves merge conflicts caused by overlapping agent file access. |
| **Reliability Engineer** | Identifies the last durable BLAKE3 DAG checkpoint, executes plan resume, and confirms state recovery before incident closure. |

Every role is named in `demo/incident-tabletop/scenario.json` under the
`roles` array.

## Timeline

The following table lists one row per inject from
`demo/incident-tabletop/scenario.json`.  For each inject the facilitator
prompt (what the facilitator says or does) is separated from the expected
participant action (how the room should respond).

| Minute | Risk | Event (Facilitator Prompt) | Expected Participant Action |
|--------|------|---------------------------|----------------------------|
| 0 | RISK-01 | **Facilitator announces:** "The primary LLM provider begins returning HTTP 503 errors. Agent dispatch tasks stall and several return incomplete output that is persisted as signal." | Incident Commander opens the response bridge and assigns roles. Agent Runtime Lead verifies provider status, confirms the failure is not transient, and enables tier-based escalation to the fallback model configured in `roko.toml`. Incident Commander broadcasts the initial status to all leads. |
| 4 | RISK-02 | **Facilitator announces:** "A long-running agent task exceeds the 600-second `task_attempt_secs` threshold and is killed mid-execution, leaving partial workspace state that may corrupt the signal DAG." | Operations Engineer assesses the partial state and determines whether it is safe to retain. Incident Commander decides whether to tune `[timeouts]` in `roko.toml` for the affected task class or to discard the partial work and recover via `--resume-plan` from the last checkpoint. The decision and rationale are communicated to all leads. |
| 9 | RISK-03 | **Facilitator announces:** "`cargo test` returns a non-zero exit code during the gate run. The 7-rung pipeline short-circuits on first failure and the task cannot pass verification, stalling the plan." | QA Lead triages the test failure to distinguish a genuine defect from a flaky test. Incident Commander weighs two options: retry the task with an escalating model, or relax the adaptive threshold after confirming the failure is repeated. The chosen path and its safety trade-off are documented and shared with the bridge. |
| 15 | RISK-04 | **Facilitator announces:** "Two parallel agents modify overlapping file paths within a shared worktree, producing merge conflicts and corrupted build artifacts that block downstream tasks." | Platform Infrastructure Lead halts parallel execution and invokes the worktree manager to create isolated git worktrees per task. Before resuming, the lead verifies that `workspace_lock_secs` guards file access and that no two agents target the same paths. Incident Commander confirms isolation is in place before unblocking work. |
| 22 | RISK-05 | **Facilitator announces:** "The orchestrator process receives SIGTERM during plan execution before a durable checkpoint is written. In-progress plan execution is at risk of being lost entirely." | Reliability Engineer identifies the last persisted content-addressed BLAKE3 DAG record and issues `roko plan run plans/ --resume-plan`. Incident Commander confirms with each role lead that their recovered state is consistent, then schedules a post-incident review before formally closing the incident. |

**Facilitator instructions between injects:**

- At every 5-minute mark, prompt the Incident Commander: *"Provide a
  status update to all role leads covering decisions made, actions taken,
  and any open questions."*
- After each inject, allow 2–3 minutes for discussion before moving the
  timer forward.
- If participants reach an impasse, the facilitator may offer a hint by
  reading the `evidence` field from the corresponding inject in
  `demo/incident-tabletop/scenario.json`.

## Debrief

After the final inject (minute 22) and any closing statements from the
Incident Commander, the facilitator leads a structured debrief.  Use the
questions below to draw out lessons; record answers in the evidence log.

1. Did tier-based escalation activate quickly enough after the provider
   failure, or was there a delay that could have extended outage impact?
2. When the task was killed at the timeout threshold, was the decision to
   retain or discard partial workspace state made with sufficient
   information?  What additional telemetry would have helped?
3. Was the gate-failure triage process rigorous enough to distinguish
   genuine defects from flaky tests, or did uncertainty lead to an
   premature threshold relaxation?
4. How effectively did the worktree manager prevent cross-contamination
   after the merge conflict was detected?  Were there any gaps in
   `workspace_lock_secs` enforcement?
5. Was the BLAKE3 DAG checkpoint recent enough to enable meaningful
   recovery via `--resume-plan`?  How much work was at risk of loss?
6. Did the Incident Commander maintain the five-minute communication
   cadence throughout the exercise?  Where did communication break down?
7. Which single risk, if it had been pre-mitigated, would have reduced
   the overall incident severity the most?

## Evidence to retain

After the exercise, preserve the following artifacts for audit and
continuous improvement:

| Artifact | Description |
|----------|-------------|
| `demo/incident-tabletop/risk-register.csv` | The source risk register mapping each inject to its operational signal and mitigation. |
| `demo/incident-tabletop/scenario.json` | The canonical scenario definition with injects, roles, and success criteria. |
| Evidence log (shared doc) | Real-time record of decisions, rationale, and trade-offs captured during the exercise. |
| Role assignment sheet | Who held each role during the exercise, for accountability traceability. |
| Debrief notes | Answers to the seven debrief questions, action items, and improvement recommendations. |
| Timing log | Actual wall-clock timestamps for each inject and status-update cadence check. |
