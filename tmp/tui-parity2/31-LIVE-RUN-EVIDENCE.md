# Live run evidence and failure reconstruction

**Inspected run:** `run-1788249612013`
**Plan:** `e2e-provider-test`
**Outcome:** successful provider run; TUI remained open until the user quit
**Observed cost:** `$0.146872`

## Evidence set

- `.roko/events-by-run/301aabb2bf25945fabdfbc40e163cec4bad7e97c399dbcb3565120e9fc206a96.jsonl`
- `.roko/state/state-snapshot.json`
- `.roko/state/status.json`
- `.roko/tui.log`
- `.roko/runner-stderr.log`
- `.roko/roko.log.2026-09-01`

## Timeline

| Time | Observation |
|---|---|
| 07:57:32 | TUI process started. |
| 07:57:32-08:00:12 | Startup cargo-cache warmup consumed 159.472 seconds without a dashboard producer, explaining the long blank/zero frame. |
| 08:00:12 | Run/plan events appeared and the agent was dispatched. |
| 08:00-08:02 | The agent emitted 4 message deltas, 6 tool calls, and 6 tool outputs. Tool output totaled 4,839 bytes, including a 3,346-byte cargo build transcript. |
| 08:02:26 | Provider usage arrived: 216,512 input, 1,239 output, 197,376 cache-read tokens. The late token jump reflects provider event granularity. |
| 08:02 | Turn cost became `$0.146872`. |
| 08:02-08:03 | The task gate ran for 30.984 seconds without streaming output to the committed TUI. |
| 08:03:00 | Run completed successfully. |
| 08:09:12 | User quit the still-open TUI. |

## Root causes of the apparent hang

The watch/StateHub transport was functioning. The missing feedback occurred at the producers:

1. `TuiBridge` was created only after cache warming, leaving no startup state to render.
2. `ToolCall` and `ToolOutput` went only to `RunOutputSink`; approval mode used `NoopSink`, so
   4,839 bytes of useful activity never reached the agent transcript.
3. Tokens and cost are only as live as the provider usage events; the TUI cannot infer them safely.
4. Gate execution returned one completed buffer rather than line events, so the gate panel was idle
   for the entire 30.984-second command.

`fced716b6` addresses the first two problems: it creates the bridge before warmup, publishes
warmup status, and projects bounded tool call/output text into the dashboard. It also replays gate
output on completion. It does **not** turn the gate subprocess into a streaming producer.

## Why earlier runs exited

The 2026-08-31 abort was a real debug-build panic, not intentional TUI lifecycle behavior.
Full effects evaluated a guide-line seed with overflowing multiplication; cleanup attempted to
replace the panic hook while already unwinding and converted the panic into `SIGABRT`. The visual
baseline commit uses wrapping arithmetic and safe cleanup. The successful run inspected here then
confirmed that connected final-state behavior intentionally keeps the TUI open for postmortem
navigation.

## Persisted terminal-state defects found

Before the terminal-projection follow-up, this successful run still left contradictory artifacts:

- `status.json` remained in phase `gate` with `last_event = "task:plan-verify"`.
- The unified snapshot marked the run/plan complete while synthetic `plan-verify` remained
  `running`/`gating`.
- `plan.completed` carried zero cost and zero task counts even though `run.completed` carried the
  correct cost and 1/0 result.
- Connected plan metadata had `started_at_ms = 0` and no changed-file list.

These are projection/finalization problems, not evidence that the successful provider work was
lost. They do make post-run screens and automation untrustworthy until fixed and regression-tested.

## Live-verification boundary

No second paid provider run was started for this audit. Source fixes were exercised with focused
tests and controlled headless captures. The next real plan should verify, in order: visible warmup,
tool transcript updates during a long command, authoritative pause acknowledgement, gate activity,
terminal status convergence, and continuous screenshots.
