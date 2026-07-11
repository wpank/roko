# Self-heal issue coverage

This ledger maps every issue source in `tmp/status-quo/issues` to a new self-heal task or an existing E-series implementation plan. `00-INDEX.md` is the inventory; `40-CRASH-TIMELINE.md` is acceptance evidence for SH06.

## Live-run findings

| Sources | Tasks |
|---|---|
| 01, 02, 04, 06, 07 | SH01-T02/T03/T05; SH03-T03/T04; SH02-T02 |
| 03, 08, 09, 10, 11, 12 | SH04-T01/T03/T04/T05/T06 |
| 13, 14, 15, 16 | SH02-T01/T05; SH04-T04/T05/T07 |

## Subsystem audits

| Issue file | Executable coverage |
|---|---|
| 10-EVENT-LOOP | SH01, SH02-T05, existing E01/E08 |
| 11-AGENT-DISPATCH | SH04-T02, SH05-T02, existing E14 |
| 12-GATE-PIPELINE | SH01-T02, SH02-T02, existing E05 |
| 13-STATE-PERSISTENCE | SH03-T01/T02/T03, existing E02 |
| 14-TUI-DASHBOARD | SH04, existing E09/E10 |
| 15-HTTP-SERVE | existing E04/E09/E10 |
| 16-LEARNING-FEEDBACK | SH05-T04, existing E07/E25 |
| 17-SAFETY-LAYER | existing E04/E34 |
| 18-GRAPH-ENGINE | existing E21/E22 |
| 19-CLI-COMMANDS | existing E18/E37/E42 |
| 20-MERGE-QUEUE | SH02-T02/T03, existing E01 |
| 21-PRD-PLAN-GEN | existing E16 |
| 22-KNOWLEDGE-NEURO | existing E07/E24 |
| 23-CONFIG | SH05-T01, existing E18/E42 |
| 24-DREAMS-DAIMON | existing E07/E23/E24 |
| 25-DUPLICATE-TYPES | existing E03 |
| 26-DEAD-CODE | existing E12 |
| 27-AGENT-SERVER | existing E04/E14/E29 |
| 28-PROCESS-SUPERVISION | SH05-T03, existing E08/E22 |
| 29-PROMPT-COMPOSITION | existing E06 |
| 30-FILESYSTEM-JSONL | SH03-T05, existing E02 |
| 31-E01-RECENT-CHANGES | SH01/SH02/SH03 |
| 32-COLD-SUBSTRATE | existing E02/E24 |
| 33-ERROR-HANDLING | SH01-T02/T03, existing E18 |
| 34-MCP-INTEGRATION | existing E15/E32 |
| 35-COST-BUDGET | SH05-T04, existing E48 |
| 36-DEPENDENCIES | existing E03/E12/E18 |
| 37-STATEHUB-EVENTS | SH03-T06, SH04-T01/T03 |
| 38-TEST-COVERAGE | SH06, existing E18 |
| 39-DAEMON-DEPLOY | existing E43/E18 |

## Crash findings

| Sources | Tasks |
|---|---|
| 40 | SH06-T01/T02/T05 |
| 41, 42, 43 | SH01-T01/T02/T04 |
| 44, 45, 46 | SH01-T06/T07; SH03-T01 |
| 47, 48 | SH01-T01/T03; SH02-T05; SH03-T03 |
| 49, 50, 51 | SH02-T02/T03 |
| 52, 53, 54 | SH03-T03; SH02-T04/T06; SH01-T05 |
| 55 | SH05-T01 |
| 56, 58 | SH03-T02/T05 |
| 57, 59 | SH04-T02/T08 |

## Execution waves

1. SH01 repairs lifecycle, terminalization, DAG quiescence, retry, timeout, and summaries.
2. SH02 establishes concurrency limits, task-owned isolation, durable commits, and recovery.
3. SH03 makes snapshots, ledgers, checkpoints, seeded state, atomic writes, and StateHub reliable.
4. SH04 repairs the connected TUI and live telemetry.
5. SH05 hardens configuration, dispatch, supervision, and cost budgets.
6. SH06 proves crash recovery and self-hosting end to end.

