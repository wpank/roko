#!/usr/bin/env bash
# Acceptance check for 03-backend-live-events.
#
# Executes the two-task fixture plan `live-a` through POST /api/plans/{id}/execute
# on a scratch `roko serve`, captures GET /api/events for the whole run, and
# asserts the run was observable live: per-task deltas in order, the agent
# visible while it worked, usage attributed to the task, totals non-zero, and a
# terminal run_completed. Deterministic and free: the agent is ./fake-claude.
#
# Usage (from the repo root, after `cargo build -p roko-cli`):
#   bash plans/portal-programme/_harness/live-events-check.sh
# Prints one PASS/FAIL line per assertion and a final LIVE-EVENTS-CHECK verdict.

source "$(cd "$(dirname "$0")" && pwd)/lib.sh"

require_binary
make_workspace
start_server
start_capture

check "execute on a directory plan returns 202" [ "$(api POST /api/plans/live-a/execute)" = 202 ]
check "the run finishes within 180s" wait_idle live-a 180
stop_capture
api GET /api/statehub/snapshot >/dev/null
cp "$WS/last.json" "$WS/snapshot.json"

check "both tasks wrote their artifacts" test -f "$WS/out/live-a-t01.txt" -a -f "$WS/out/live-a-t02.txt"
check "task events reach the server stream" \
    sse has task_started plan_id=live-a task_id=T02
check "status is live: T01 completes before T02 starts" \
    sse before task_completed plan_id=live-a task_id=T01 -- task_started plan_id=live-a task_id=T02
check "no task is reported complete twice" \
    sse count task_completed plan_id=live-a task_id=T01 --eq 1
check "the agent is visible while it works (heartbeat between spawn and completion)" \
    sse between agent_heartbeat task_id=T01 -- agent_spawned task_id=T01 -- agent_completed task_id=T01
check "input tokens are attributed to the task" \
    sse positive efficiency_event value plan_id=live-a task_id=T01 metric=input_tokens
check "cost is attributed to the task" \
    sse positive efficiency_event value plan_id=live-a task_id=T01 metric=cost_usd
check "snapshot token total is non-zero" \
    jcheck "$WS/snapshot.json" 'd["data"]["stats"]["total_input_tokens"] > 0'
check "snapshot cost total is non-zero" \
    jcheck "$WS/snapshot.json" 'd["data"]["stats"]["cost_usd_total"] > 0'
check "run_completed reports success with elapsed time" \
    sse positive run_completed duration_ms outcome=succeeded

finish "LIVE-EVENTS-CHECK"
