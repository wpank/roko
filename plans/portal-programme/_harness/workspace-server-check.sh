#!/usr/bin/env bash
# Acceptance check for 03b-backend-workspace-server.
#
# While `roko serve` runs for a workspace it owns it: it advertises itself,
# serves its hub over .roko/runtime/hub.sock, runs one plan run at a time, and
# CLI commands become its clients. This check drives all of that on a scratch
# workspace with the deterministic fake agent (./fake-claude).
#
# Usage (from the repo root, after `cargo build -p roko-cli`):
#   bash plans/portal-programme/_harness/workspace-server-check.sh
# Prints one PASS/FAIL line per assertion and a final WORKSPACE-SERVER-CHECK verdict.

source "$(cd "$(dirname "$0")" && pwd)/lib.sh"

# Each helper proves its own precondition, so no assertion can pass vacuously.
SERVE_SEEN=""
SOCKET_SEEN=""
SLOW_STARTED=""
ALL_RUN=""
advertised() {
    jcheck "$WS/.roko/runtime/serve.json" "d['pid'] == $SERVER_PID and d['url'].endswith(':$PORT')" || return 1
    SERVE_SEEN=1
}
socket_served() { test -S "$WS/.roko/runtime/hub.sock" || return 1; SOCKET_SEEN=1; }
submitted() { grep -q "submitted to roko serve" "$1"; }
client_run_ok() { submitted "$WS/client.out" && [ "$CLIENT_RC" = 0 ]; }
client_run_wrote_artifacts() { submitted "$WS/client.out" && test -f "$WS/out/live-a-t02.txt"; }
fake_calls() { grep -c ' task ' "$WS/.roko/fake-claude.log" 2>/dev/null || echo 0; }
resume_skipped_agents() { [ "$RESUME_CODE" = 202 ] && [ "$CALLS_BEFORE" -gt 0 ] && [ "$CALLS_BEFORE" = "$(fake_calls)" ]; }
fresh_ran_agents() { [ "$FRESH_CODE" = 202 ] && [ "$(fake_calls)" -gt "$CALLS_BEFORE_FRESH" ]; }
resume_route_skipped_agents() {
    [ "$ROUTE_CODE" = 202 ] && [ "$CALLS_BEFORE_ROUTE" -gt 0 ] && [ "$CALLS_BEFORE_ROUTE" = "$(fake_calls)" ]
}
slow_started() { wait_fake 30 || return 1; SLOW_STARTED=1; }
cancel_stopped_agent() { [ -n "$SLOW_STARTED" ] && wait_no_fake 20; }
cancelled_without_artifact() { [ -n "$SLOW_STARTED" ] && test ! -f "$WS/out/live-slow.txt"; }
all_run_sse() { [ -n "$ALL_RUN" ] && sse "$@"; }
all_run_lacks() { [ -n "$ALL_RUN" ] && ! sse "$@" 2>/dev/null; }
client_exited_nonzero() { submitted "$WS/ctrlc.out" && [ -n "${CLIENT_RC:-}" ] && [ "$CLIENT_RC" != 0 ]; }
ctrlc_cancelled_server_run() { submitted "$WS/ctrlc.out" && wait_no_fake 20; }
PAR_RUN=""
parallel_overlap() {
    [ -n "$PAR_RUN" ] &&
        sse before plan_started plan_id=par-b -- plan_completed plan_id=par-a &&
        sse before plan_started plan_id=par-a -- plan_completed plan_id=par-b
}
parallel_both_succeeded() {
    [ -n "$PAR_RUN" ] && sse has plan_completed plan_id=par-a success=true &&
        sse has plan_completed plan_id=par-b success=true
}
removed_after_seen() { [ -n "$1" ] && test ! -e "$2"; }

require_binary
make_workspace
start_server
start_capture

# ── The server advertises itself and describes its workspace ───────────────
check "serve.json names this server's pid and port" advertised
check "the hub socket is served" socket_served
BRANCH="$(git -C "$WS" rev-parse --abbrev-ref HEAD)"
api GET /api/status >/dev/null
check "GET /api/status reports the workspace branch" jcheck "$WS/last.json" "d.get('git_branch') == '$BRANCH'"

# ── Read-only commands attach instead of locking ───────────────────────────
check "roko dashboard --text works while the server owns the workspace" \
    run_in_ws "$ROKO_BIN" dashboard --text
check "roko plan list works while the server owns the workspace" \
    run_in_ws "$ROKO_BIN" plan list
check "roko plan validate works while the server owns the workspace" \
    run_in_ws "$ROKO_BIN" plan validate plans/live-a

# ── roko plan run becomes a client ─────────────────────────────────────────
run_in_ws "$ROKO_BIN" plan run plans/live-a --no-tui >"$WS/client.out" 2>&1
CLIENT_RC=$?
check "plan run through the server exits 0" client_run_ok
check "plan run says it submitted to the server" submitted "$WS/client.out"
check "the client's run happened in the server (its events are on the server stream)" \
    sse has task_completed plan_id=live-a task_id=T02
check "the client's run wrote its artifacts" client_run_wrote_artifacts

# ── Resume: an unchanged, completed plan dispatches no agent ───────────────
start_capture events-resume.sse
CALLS_BEFORE="$(fake_calls)"
RESUME_CODE="$(api POST /api/plans/live-a/execute '{"resume":true}')"
wait_idle live-a 60 || true
stop_capture
check "a resumed run of a plan the CLI completed succeeds (one shared checkpoint)" \
    sse has plan_completed plan_id=live-a success=true
check "a resumed run of an unchanged plan dispatches no agent" resume_skipped_agents
start_capture events-fresh.sse
CALLS_BEFORE_FRESH="$(fake_calls)"
FRESH_CODE="$(api POST /api/plans/live-a/execute)"
wait_idle live-a 90 || true
stop_capture
check "executing a completed plan again succeeds" sse has plan_completed plan_id=live-a success=true
check "a plain execute runs the plan fresh" fresh_ran_agents
check "an execute publishes exactly one plan_started" sse count plan_started plan_id=live-a --eq 1
check "that plan_started carries the plan's task count" sse has plan_started plan_id=live-a tasks_total=2

# ── One run per workspace; cancel through a member plan really cancels ─────
start_capture events-cancel.sse
code="$(api POST /api/plans/execute '{"target":"slow-plans/live-slow"}')"
SLOW_RUN="$(jget "$WS/last.json" 'd["id"]')"
check "POST /api/plans/execute runs a workspace-relative target" \
    [ "$code" = 202 -a -n "$SLOW_RUN" ]
check "the response carries the run order" jcheck "$WS/last.json" 'd["order"] == ["live-slow"]'
check "the slow agent starts" slow_started
check "execute conflicts while another run is active" [ "$(api POST /api/plans/live-b/execute)" = 409 ]
check "a set run conflicts while another run is active" [ "$(api POST /api/plans/execute '{}')" = 409 ]
check "status finds the run through a member plan id" [ "$(api GET /api/plans/live-slow/status)" = 200 ]
check "cancel through a member plan id is accepted" [ "$(api POST /api/plans/live-slow/cancel)" = 200 ]
check "cancel stops the agent process" cancel_stopped_agent
check "the cancelled task never wrote its artifact" cancelled_without_artifact
check "run_completed reports the run as cancelled" sse has run_completed outcome=cancelled
wait_idle "$SLOW_RUN" 30 || true
check "a target outside the workspace is rejected" \
    [ "$(api POST /api/plans/execute '{"target":"../"}')" = 400 ]
check "an unknown plan id is a 404" [ "$(api POST /api/plans/execute '{"plans":["no-such-plan"]}')" = 404 ]

# ── Run all: every plan under plans/, in plan-set order ────────────────────
start_capture events-all.sse
code="$(api POST /api/plans/execute '{}')"
ALL_RUN="$(jget "$WS/last.json" 'd["id"]')"
check "run-all is accepted" [ "$code" = 202 -a -n "$ALL_RUN" ]
check "run-all reports its order" jcheck "$WS/last.json" 'd["order"] == ["live-a", "live-b", "live-nested"]'
check "run-all takes max_parallel_plans from the workspace config (1)" \
    jcheck "$WS/last.json" 'd.get("max_parallel_plans") == 1'
check "run-all finishes within 120s" wait_idle "$ALL_RUN" 120
stop_capture
check "run-all starts live-a before live-b" \
    all_run_sse before plan_started plan_id=live-a -- plan_started plan_id=live-b
check "run-all completes live-b" all_run_sse has plan_completed plan_id=live-b success=true
check "run-all runs the plan inside a plan set" all_run_sse has plan_completed plan_id=live-nested success=true
check "the nested plan's run wrote its artifact" test -f "$WS/out/live-nested.txt"
check "run-all leaves plans outside plans/ alone" all_run_lacks has plan_started plan_id=live-slow

# ── A set run with max_parallel_plans runs independent plans at once ───────
start_capture events-parallel.sse
code="$(api POST /api/plans/execute '{"target":"par-plans","max_parallel_plans":2}')"
PAR_RUN="$(jget "$WS/last.json" 'd["id"]')"
check "a set run accepts max_parallel_plans" [ "$code" = 202 -a -n "$PAR_RUN" ]
check "the response echoes the limit and the order" \
    jcheck "$WS/last.json" 'd["max_parallel_plans"] == 2 and d["order"] == ["par-a", "par-b"]'
PAR_CONFLICT="$(api POST /api/plans/live-b/execute)"
check "a parallel set run finishes within 90s" wait_idle "$PAR_RUN" 90
stop_capture
check "the whole parallel set is one run (409 for another execute)" [ "$PAR_CONFLICT" = 409 ]
check "both plans start before either finishes" parallel_overlap
check "both parallel plans succeed" parallel_both_succeeded
check "a zero plan limit is rejected (422)" \
    [ "$(api POST /api/plans/execute '{"target":"par-plans","max_parallel_plans":0}')" = 422 ]

# ── A plan inside a plan set executes by id ────────────────────────────────
start_capture events-nested.sse
check "a plan inside a plan set is accepted for execution" [ "$(api POST /api/plans/live-nested/execute)" = 202 ]
wait_idle live-nested 60 || true
stop_capture
check "the nested plan's own run succeeds" sse has plan_completed plan_id=live-nested success=true

# ── Resume and the per-plan read routes find directory plans ───────────────
start_capture events-resume-route.sse
CALLS_BEFORE_ROUTE="$(fake_calls)"
ROUTE_CODE="$(api POST /api/plans/live-nested/resume)"
wait_idle live-nested 60 || true
stop_capture
check "POST /resume accepts a plan inside a plan set" [ "$ROUTE_CODE" = 202 ]
check "the resumed nested plan completes" sse has plan_completed plan_id=live-nested success=true
check "resuming a completed plan dispatches no agent" resume_route_skipped_agents
check "the costs route finds a directory plan" [ "$(api GET /api/plans/live-a/costs)" = 200 ]
check "the gates route finds a plan inside a plan set" [ "$(api GET /api/plans/live-nested/gates)" = 200 ]
check "the costs route still answers 404 for an unknown plan" [ "$(api GET /api/plans/no-such-plan/costs)" = 404 ]

# ── Ctrl-C on a client cancels its server run ──────────────────────────────
start_capture events-ctrlc.sse
(cd "$WS" && exec python3 -c 'import os, signal, sys; signal.signal(signal.SIGINT, signal.SIG_DFL); os.execvp(sys.argv[1], sys.argv[1:])' \
    "$ROKO_BIN" plan run slow-plans/live-slow --no-tui) >"$WS/ctrlc.out" 2>&1 &
CLIENT_PID=$!
CLIENT_RC=""
check "the client's slow agent starts" wait_fake 30
kill -INT "$CLIENT_PID" 2>/dev/null
waited=0
while kill -0 "$CLIENT_PID" 2>/dev/null && [ "$waited" -lt 20 ]; do sleep 1; waited=$((waited + 1)); done
if kill -0 "$CLIENT_PID" 2>/dev/null; then kill -9 "$CLIENT_PID" 2>/dev/null; else wait "$CLIENT_PID"; CLIENT_RC=$?; fi
check "the client exits non-zero after Ctrl-C" client_exited_nonzero
check "Ctrl-C cancelled the server run" ctrlc_cancelled_server_run
stop_capture

# ── Shutdown withdraws the advertisement ───────────────────────────────────
check "the server stops on Ctrl-C" stop_server
check "serve.json is removed on shutdown" removed_after_seen "$SERVE_SEEN" "$WS/.roko/runtime/serve.json"
check "the hub socket is removed on shutdown" removed_after_seen "$SOCKET_SEEN" "$WS/.roko/runtime/hub.sock"

finish "WORKSPACE-SERVER-CHECK"
