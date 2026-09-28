#!/usr/bin/env bash
# Acceptance check for 04b-backend-plan-revision.
#
# Revises a plan in place from one sentence through POST /api/plans/{id}/revise:
# the operation reports the result, the plan gains the task, stays valid and runs.
# A revision the agent gets wrong fails the operation and leaves the file
# byte-identical; blank feedback, unknown plans and running plans are refused.
# The agent is ./fake-claude (its revise mode appends task T99).
#
# Usage (from the repo root, after `cargo build -p roko-cli`):
#   bash plans/portal-programme/_harness/revision-check.sh
# Prints one PASS/FAIL line per assertion and a final REVISION-CHECK verdict.

source "$(cd "$(dirname "$0")" && pwd)/lib.sh"

revised_source_has_task() {
    [ "$(api GET /api/plans/live-b/source)" = 200 ] && jcheck "$WS/last.json" '"Added by revision" in d["toml"]'
}
revised_plan_valid() {
    [ "$(api POST /api/plans/live-b/validate)" = 200 ] && jcheck "$WS/last.json" 'd["valid"] is True'
}
revised_plan_runs() {
    [ "$(api POST /api/plans/live-b/execute)" = 202 ] && wait_idle live-b 90 && test -f "$WS/out/revised.txt"
}
bad_revision_failed() { wait_operation "$OP2" 120 && jcheck "$WS/last.json" 'd["status"] == "failed"'; }
accepted() { [ "$code" = 202 ] && [ -n "$OP" ]; }
refused_while_running() { [ "$RUN_CODE" = 202 ] && [ "$(api POST /api/plans/live-a/revise '{"feedback":"x"}')" = 409 ]; }

require_binary
make_workspace
start_server
start_capture

# ── A revision that works ──────────────────────────────────────────────────
code="$(api POST /api/plans/live-b/revise '{"feedback":"add a task that writes out/revised.txt"}')"
OP="$(jget "$WS/last.json" 'd["id"]')"
check "revise returns 202 with an operation id" accepted
check "the response names the plan" jcheck "$WS/last.json" 'd["plan_id"] == "live-b"'
check "the revision finishes within 120s" wait_operation "$OP" 120
check "the operation reports the revised plan" \
    jcheck "$WS/last.json" "d['status'] == 'completed' and d['result']['slug'] == 'live-b' and d['result']['task_count'] == 2"
check "the plan's source now carries the new task" revised_source_has_task
check "the revised plan is valid" revised_plan_valid
check "revision completion is announced on the event stream" \
    sse has event_log_entry event_type=plan_revise.completed plan_id=live-b
check "the revised plan runs, new task included" revised_plan_runs

# ── A revision the agent gets wrong ────────────────────────────────────────
BEFORE="$(sha "$WS/plans/live-a/tasks.toml")"
code2="$(api POST /api/plans/live-a/revise '{"feedback":"BREAK the dependencies"}')"
OP2="$(jget "$WS/last.json" 'd["id"]')"
check "an invalid revision is still accepted for processing (202)" [ "$code2" = 202 ]
check "an invalid revision fails its operation" bad_revision_failed
check "a failed revision leaves the plan byte-identical" same_sha "$BEFORE" "$WS/plans/live-a/tasks.toml"
check "revision failure is announced on the event stream" \
    sse has event_log_entry event_type=plan_revise.failed plan_id=live-a

# ── Guards ─────────────────────────────────────────────────────────────────
check "blank feedback is rejected (422)" [ "$(api POST /api/plans/live-b/revise '{"feedback":"   "}')" = 422 ]
check "an unknown plan is 404" [ "$(api POST /api/plans/no-such-plan/revise '{"feedback":"x"}')" = 404 ]
RUN_CODE="$(api POST /api/plans/live-a/execute)"
check "a plan cannot be revised while it runs (409)" refused_while_running
wait_idle live-a 90 || true

finish "REVISION-CHECK"
