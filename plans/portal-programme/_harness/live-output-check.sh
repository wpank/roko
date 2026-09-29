#!/usr/bin/env bash
# Acceptance check for the live agent output of 03c-backend-local-access (T12-T20).
#
# Each tool call streams live as a step (its name and target only). On a loopback
# server with `[serve] live_agent_output = "trusted"`, unscreened text and tool results
# stream too. Either way the screened transcript still arrives at the end. The agent is
# ./fake-claude in live mode (.roko/fake-claude-live): it streams a text block and a
# Write tool call before its SLOW wait, then the tool result after writing.
#
# Usage (from the repo root, after `cargo build -p roko-cli`):
#   bash plans/portal-programme/_harness/live-output-check.sh
# Prints one PASS/FAIL line per assertion and a final LIVE-OUTPUT-CHECK verdict.

source "$(cd "$(dirname "$0")" && pwd)/lib.sh"

step_before_completion() {
    sse before agent_output plan_id=live-a task_id=T01 'content~"kind":"tool_start"' \
        'content~"live":true' 'content~"target":"out/live-a-t01.txt"' \
        -- task_completed plan_id=live-a task_id=T01
}
step_names_tool() {
    sse has agent_output plan_id=live-a task_id=T01 'content~"live":true' 'content~"tool":"Write"'
}
steps_carry_no_input() {
    step_before_completion 2>/dev/null &&
        ! sse has agent_output 'content~"live":true' 'content~"input"' 2>/dev/null
}
nothing_unscreened() {
    step_before_completion 2>/dev/null &&
        ! sse has agent_output 'content~"screened":false' 2>/dev/null
}
screened_transcript() {
    sse has agent_output plan_id="$1" task_id=T01 'content~"kind":"text"' 'content!~"live":true'
}
unscreened_text_before_completion() {
    sse before agent_output plan_id=live-b task_id=T01 'content~"screened":false' \
        'content~"kind":"text"' -- task_completed plan_id=live-b task_id=T01
}
announced_trusted() {
    grep -q 'live agent output: trusted' "$WS/serve.log" && ! grep -q 'trusted ignored' "$WS/serve.log"
}
unscreened_tool_result() {
    sse has agent_output plan_id=live-b task_id=T01 'content~"screened":false' \
        'content~"kind":"tool_result"'
}

require_binary
make_workspace
touch "$WS/.roko/fake-claude-live"

# ── The default: tool steps only ───────────────────────────────────────────
start_server
start_capture events-steps.sse
code="$(api POST /api/plans/live-a/execute)"
check "the run is accepted" [ "$code" = 202 ]
check "the run finishes within 120s" wait_idle live-a 120
stop_capture
check "the default mode is announced at startup" grep -q 'live agent output: tool_steps' "$WS/serve.log"
check "a tool step arrives while the task is still running" step_before_completion
check "the step names the tool" step_names_tool
check "steps carry the target only, never the tool input" steps_carry_no_input
check "the default mode streams nothing unscreened" nothing_unscreened
check "the screened transcript still arrives" screened_transcript live-a
check "the server stops on Ctrl-C" stop_server

# ── Trusted output on a loopback server ────────────────────────────────────
python3 - "$WS/roko.toml" <<'PY'
import sys
path = sys.argv[1]
text = open(path).read()
assert "[serve.auth]" in text
open(path, "w").write(text.replace("[serve.auth]", '[serve]\nlive_agent_output = "trusted"\n\n[serve.auth]', 1))
PY
start_server
start_capture events-trusted.sse
code="$(api POST /api/plans/live-b/execute)"
check "the trusted run is accepted" [ "$code" = 202 ]
check "the trusted run finishes within 90s" wait_idle live-b 90
stop_capture
check "trusted mode on a loopback server is announced at startup" announced_trusted
check "unscreened text arrives before the task completes" unscreened_text_before_completion
check "the tool result arrives unscreened" unscreened_tool_result
check "the screened transcript still arrives in trusted mode" screened_transcript live-b

finish "LIVE-OUTPUT-CHECK"
