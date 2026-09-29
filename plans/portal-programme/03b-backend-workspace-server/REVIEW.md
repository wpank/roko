# 03b Backend Workspace Server — Review

## Verdict

WORKSPACE-SERVER-CHECK: PASS (58 checks)

---

## Full PASS/FAIL Output

```
PASS serve.json names this server's pid and port
PASS the hub socket is served
PASS GET /api/status reports the workspace branch
PASS roko dashboard --text works while the server owns the workspace
PASS roko plan list works while the server owns the workspace
PASS roko plan validate works while the server owns the workspace
PASS plan run through the server exits 0
PASS plan run says it submitted to the server
PASS the client's run happened in the server (its events are on the server stream)
PASS the client's run wrote its artifacts
PASS a resumed run of a plan the CLI completed succeeds (one shared checkpoint)
PASS a resumed run of an unchanged plan dispatches no agent
PASS executing a completed plan again succeeds
PASS a plain execute runs the plan fresh
PASS an execute publishes exactly one plan_started
PASS that plan_started carries the plan's task count
PASS POST /api/plans/execute runs a workspace-relative target
PASS the response carries the run order
PASS the slow agent starts
PASS execute conflicts while another run is active
PASS a set run conflicts while another run is active
PASS status finds the run through a member plan id
PASS cancel through a member plan id is accepted
PASS cancel stops the agent process
PASS the cancelled task never wrote its artifact
PASS run_completed reports the run as cancelled
PASS a target outside the workspace is rejected
PASS an unknown plan id is a 404
PASS run-all is accepted
PASS run-all reports its order
PASS run-all takes max_parallel_plans from the workspace config (1)
PASS run-all finishes within 120s
PASS run-all starts live-a before live-b
PASS run-all completes live-b
PASS run-all runs the plan inside a plan set
PASS the nested plan's run wrote its artifact
PASS run-all leaves plans outside plans/ alone
PASS a set run accepts max_parallel_plans
PASS the response echoes the limit and the order
PASS a parallel set run finishes within 90s
PASS the whole parallel set is one run (409 for another execute)
PASS both plans start before either finishes
PASS both parallel plans succeed
PASS a zero plan limit is rejected (422)
PASS a plan inside a plan set is accepted for execution
PASS the nested plan's own run succeeds
PASS POST /resume accepts a plan inside a plan set
PASS the resumed nested plan completes
PASS resuming a completed plan dispatches no agent
PASS the costs route finds a directory plan
PASS the gates route finds a plan inside a plan set
PASS the costs route still answers 404 for an unknown plan
PASS the client's slow agent starts
PASS the client exits non-zero after Ctrl-C
PASS Ctrl-C cancelled the server run
PASS the server stops on Ctrl-C
PASS serve.json is removed on shutdown
PASS the hub socket is removed on shutdown
WORKSPACE-SERVER-CHECK: PASS (58 checks)
```

---

## Design as Built

When `roko serve` starts in a workspace it advertises itself by writing
`.roko/runtime/serve.json` (PID + URL) and binding a Unix socket at
`.roko/runtime/hub.sock`.

**Server process** owns the workspace:
- Holds the exclusive runner lock (`roko.runner.lock`) for the duration of any
  plan run.
- Runs plan executions sequentially (or in parallel up to `max_parallel_plans`)
  inside the server process.
- Publishes `DashboardEvent` frames to the `StateHub` which streams them to SSE
  clients on `GET /api/events`.
- Removes `serve.json` and `hub.sock` on clean shutdown (Ctrl-C / SIGTERM).

**CLI process** (a second `roko` invocation in the same workspace):
- Read-only commands (`dashboard --text`, `plan list`, `plan validate`) call
  `read_lock_unless_served()`, which detects the server and skips the workspace
  lock entirely — the server guarantees consistency.
- `roko plan run` calls `discover_workspace_server()`, gets the server endpoint,
  connects to `hub.sock` (IPC mirror), POSTs `POST /api/plans/execute` (or
  `/resume`) to the server, prints `"submitted to roko serve at … as run …"`,
  then follows the run via the hub IPC stream.
- Ctrl-C in the client POSTs `POST /api/plans/{id}/cancel` to the server and
  exits non-zero.
- The server's runner lock is never contested by clients; only the server
  acquires it.

**Discovery safety**: `discover_workspace_server()` validates the endpoint with
two independent checks: (1) PID cross-check with `.roko/runtime/roko.lock` to
reject stale files from a crashed server, (2) TCP connect to the server's port
within one second. A server that fails either check is treated as absent and
the CLI proceeds normally (acquiring its own locks).

---

## Fixes Applied During This Run

Two categories of failures were found and fixed:

### 1. `reqwest::blocking` panic in debug mode (`serve_client.rs`)

**Symptom**: `dashboard --text`, `plan list`, `plan validate`, `plan run` all
panicked with "Cannot drop a runtime in a context where blocking is not allowed"
when a server was running.

**Root cause**: `reqwest 0.12`'s blocking client, in debug builds, calls an
internal `wait::enter()` function that creates and immediately drops a temporary
`tokio::runtime::Runtime` as a runtime-context guard. Dropping a `Runtime`
inside the outer multi-thread tokio async executor triggers tokio's panic
guard in `blocking/shutdown.rs:52`. This affects both `discover_workspace_server`
(which built a `reqwest::blocking::Client` for the health probe) and
`WorkspaceServerClient` (which held a `reqwest::blocking::Client` throughout the
async `run_plan_via_server` function).

**Fix**:
- `discover_workspace_server`: replaced the `reqwest::blocking::Client` health
  probe with a raw `std::net::TcpStream::connect_timeout` call. A successful TCP
  connect is sufficient to confirm the server is listening; no HTTP client is
  created or dropped.
- `WorkspaceServerClient`: converted from `reqwest::blocking::Client` to the
  async `reqwest::Client`. The three HTTP methods (`submit_plan_run`,
  `cancel_plan_run`, `plan_run_finished`) are now `async fn` with `.await` on
  each `send()`, `text()`, and `json()` call. The `block_in_place` wrappers in
  `run_plan_via_server` and `follow_run_text`/`follow_run_tui` were replaced with
  direct `.await`.

### 2. SSE captures included historical events (`lib.sh`)

**Symptom**: "an execute publishes exactly one plan_started" failed with 3
matches; "run-all leaves plans outside plans/ alone" failed because the capture
contained a `plan_started plan_id=live-slow` from an earlier cancelled run.

**Root cause**: `start_capture` connected to `GET /api/events` with no cursor
parameter, which defaults to seq=0. The server replays all retained events since
startup, so every new capture accumulated events from all previous runs in the
same server session.

**Fix**: Changed `start_capture` in `lib.sh` to use `?n=9999999999`. Since no
test run emits anywhere near that many events, `requested_seq >= next_seq` is
always true, so the server skips replay entirely and streams only events
generated after the connection is made. The harness behaviour is otherwise
unchanged.

---

## What the Check Cannot See

**TUI attach** (`--no-tui` was used for all automated checks): The check drives
`roko plan run … --no-tui` which uses the text-mode event follower. The TUI
path (`follow_run_tui`) spawns the ratatui `App` on a blocking thread via
`std::thread::spawn`, connects to the hub IPC mirror, and displays live
task/plan progress. This path was NOT exercised by the automated harness because
it requires a terminal. Manual verification was not performed in this session;
the code path exists at `crates/roko-cli/src/serve_client.rs:follow_run_tui`
and shares the same hub-IPC and cancellation machinery that the text-mode path
exercises (and which passes).
