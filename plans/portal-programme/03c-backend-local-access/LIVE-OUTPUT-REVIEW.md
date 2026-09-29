# Live Output Acceptance Review

## Task

T20 — Run the acceptance check and record the result.

## What was run

```
cargo build -p roko-cli && bash plans/portal-programme/_harness/live-output-check.sh
```

The check brings its own scratch workspace under `/tmp`, uses `fake-claude` in live mode
(streams a text block and a Write tool call before the SLOW wait), and exercises two
server configurations:

- **Default** (`live_agent_output = "tool_steps"`): asserts tool steps arrive mid-task
  with no tool input and nothing unscreened.
- **Trusted** (`live_agent_output = "trusted"` on a loopback bind): asserts unscreened
  text and a tool result arrive mid-task, and the startup line announces `trusted`.

## Fix applied

The startup announcement `live agent output: tool_steps` / `live agent output: trusted`
was emitted via `info!("{live_msg}")` in `crates/roko-serve/src/lib.rs`. The tracing
stderr layer is opt-in (requires `--verbose`, `ROKO_LOG`, or `RUST_LOG`); without it
the message never reached `serve.log`.

Fix: added `println!("{live_msg}")` immediately before the existing `info!` call so the
announcement always appears on stdout, which the harness captures into `serve.log`.

File changed: `crates/roko-serve/src/lib.rs` (one line added, no assertions weakened).

## Full check output

```
PASS the run is accepted
PASS the run finishes within 120s
PASS the default mode is announced at startup
PASS a tool step arrives while the task is still running
PASS the step names the tool
PASS steps carry the target only, never the tool input
PASS the default mode streams nothing unscreened
PASS the screened transcript still arrives
PASS the server stops on Ctrl-C
PASS the trusted run is accepted
PASS the trusted run finishes within 90s
PASS trusted mode on a loopback server is announced at startup
PASS unscreened text arrives before the task completes
PASS the tool result arrives unscreened
PASS the screened transcript still arrives in trusted mode
LIVE-OUTPUT-CHECK: PASS (15 checks)
```

## Verdict

LIVE-OUTPUT-CHECK: PASS
