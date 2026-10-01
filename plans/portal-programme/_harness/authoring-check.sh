#!/usr/bin/env bash
# Acceptance check for 04-backend-plan-authoring.
#
# Drives the authoring API on a scratch `roko serve`: create a directory plan,
# read and save its TOML source losslessly, reject invalid saves without touching
# the file, refuse a save while a run includes the plan, validate a saved plan and
# an unsaved draft, generate a plan from a sentence, and execute it to a compiled
# program that prints "hello world". The agent is ./fake-claude, so the run is
# deterministic and free; the real-model version of the same flow is the
# operator's final check.
#
# Usage (from the repo root, after `cargo build -p roko-cli`):
#   bash plans/portal-programme/_harness/authoring-check.sh
# Prints one PASS/FAIL line per assertion and a final AUTHORING-CHECK verdict.

source "$(cd "$(dirname "$0")" && pwd)/lib.sh"

listed() { api GET /api/plans >/dev/null; jcheck "$WS/last.json" "any(p['id'] == '$1' for p in d)"; }
plan_validates() { [ -n "$PLAN" ] && cli_validate "plans/$PLAN"; }
hello_runs() { (cd "$WS" && ./hello/hello-bin | grep -q 'hello world'); }
# The scaffold pins no model: the routing ladder picks one from the task's role
# and tier (and a `rung` when the author sets one).
tasks_carry_detail() {
    [ "$(api GET "/api/plans/$PLAN/tasks")" = 200 ] &&
        jcheck "$WS/last.json" 'd["tasks"][0]["verify"][0]["command"] and d["tasks"][0]["role"] and d["tasks"][0]["tier"]'
}
save_refused_during_run() {
    [ "$RUN_CODE" = 202 ] && [ -n "$LIVE_BODY" ] && [ "$(api PUT /api/plans/live-a/source "$LIVE_BODY")" = 409 ]
}
generated_and_legacy_untouched() {
    [ -n "$GEN" ] && [ -f "$WS/plans/$GEN/tasks.toml" ] &&
        same_sha "$LEGACY_BEFORE" "$WS/plans/legacy-fixture/tasks.toml"
}
# source_variant KIND: print a PUT body built from the saved source ($WS/source0.json).
#   good   rename the first task, add T02 depending on it, keep a new comment line
#   dangle as good, but T02 depends on a task that does not exist
#   broken a TOML syntax error
source_variant() {
    python3 - "$WS/source0.json" "$1" <<'PY'
import json, re, sys, tomllib
text = json.load(open(sys.argv[1]))["toml"]
kind = sys.argv[2]
first = tomllib.loads(text)["task"][0]["id"]
dep = "NOPE" if kind == "dangle" else first
text = "# edited by the authoring check\n" + text
text = re.sub(r'(?m)^total = \d+', 'total = 2', text, count=1)
text = re.sub(r'(?m)^title = ".*"', 'title = "Renamed first task"', text, count=1)
text += f'''
[[task]]
id = "T02"
title = "Second task"
description = "Write the second file. ARTIFACT out/second.txt"
status = "ready"
role = "implementer"
tier = "focused"
files = ["out/second.txt"]
depends_on = ["{dep}"]

[[task.verify]]
phase = "structural"
command = "test -f out/second.txt"
fail_msg = "out/second.txt was not written"
'''
if kind == "broken":
    text += "\n[[task\n"
print(json.dumps({"toml": text}))
PY
}

require_binary
make_workspace
add_legacy_fixture
start_server
start_capture

# ── Create ────────────────────────────────────────────────────────────────
code="$(api POST /api/plans '{"title":"Scratch authoring"}')"
PLAN="$(jget "$WS/last.json" 'd["id"]')"
check "POST /api/plans creates a plan (201) and returns its slug" [ "$code" = 201 -a -n "$PLAN" ]
check "the created plan is a directory plan" test -f "$WS/plans/$PLAN/tasks.toml"
check "the created plan is listed" listed "$PLAN"
check "the created plan passes roko plan validate" plan_validates
check "tasks carry verify commands, a role and a tier for the routing ladder" tasks_carry_detail

# ── Source: read and save, losslessly ─────────────────────────────────────
check "GET source returns the plan's TOML" [ "$(api GET "/api/plans/$PLAN/source")" = 200 ]
cp "$WS/last.json" "$WS/source0.json"
check "the source parses as a plan" \
    jcheck "$WS/source0.json" 'isinstance(d["toml"], str) and "[[task]]" in d["toml"]'
GOOD="$(source_variant good)"
check "PUT source saves a valid edit (200)" [ "$(api PUT "/api/plans/$PLAN/source" "$GOOD")" = 200 ]
check "the save reports no errors" jcheck "$WS/last.json" 'd["saved"] is True and d["errors"] == []'
api GET "/api/plans/$PLAN/source" >/dev/null
check "the saved source comes back byte-for-byte, comment included" \
    python3 -c 'import json, sys; sent = json.loads(sys.argv[1])["toml"]; got = json.load(open(sys.argv[2]))["toml"]; sys.exit(0 if sent == got else 1)' "$GOOD" "$WS/last.json"
check "the saved plan passes roko plan validate" plan_validates

BEFORE="$(sha "$WS/plans/$PLAN/tasks.toml")"
check "a save with an unknown dependency is rejected with 422" \
    [ "$(api PUT "/api/plans/$PLAN/source" "$(source_variant dangle)")" = 422 ]
check "the 422 names the offending task" \
    jcheck "$WS/last.json" 'd["code"] == "invalid_plan" and any(x.get("task_id") == "T02" for x in d["diagnostics"])'
check "a save with a TOML syntax error is rejected with 422" \
    [ "$(api PUT "/api/plans/$PLAN/source" "$(source_variant broken)")" = 422 ]
check "rejected saves left the file byte-identical" same_sha "$BEFORE" "$WS/plans/$PLAN/tasks.toml"

# ── A plan cannot change under a run ──────────────────────────────────────
api GET /api/plans/live-a/source >/dev/null
cp "$WS/last.json" "$WS/live-a-source.json"
LIVE_BODY="$(python3 -c 'import json, sys; print(json.dumps({"toml": json.load(open(sys.argv[1]))["toml"]}))' "$WS/live-a-source.json" 2>/dev/null)"
RUN_CODE="$(api POST /api/plans/live-a/execute)"
check "a save while a run includes the plan is refused (409)" save_refused_during_run
wait_idle live-a 90 || true

# ── Validate ──────────────────────────────────────────────────────────────
check "validate a saved plan returns 200" [ "$(api POST "/api/plans/$PLAN/validate")" = 200 ]
check "the saved plan is valid" jcheck "$WS/last.json" 'd["valid"] is True and d["errors"] == []'
check "validate with {} (the portal's old body) validates the saved plan" \
    [ "$(api POST "/api/plans/$PLAN/validate" '{}')" = 200 ]
check "the {} validation is valid, with warnings as a list" \
    jcheck "$WS/last.json" 'd["valid"] is True and d["errors"] == [] and isinstance(d["warnings"], list)'
check "validate an unsaved invalid draft still returns 200" \
    [ "$(api POST "/api/plans/$PLAN/validate" "$(source_variant dangle)")" = 200 ]
check "the draft is reported invalid with an error on the task" \
    jcheck "$WS/last.json" 'd["valid"] is False and any(x.get("task_id") == "T02" and x["severity"] == "error" and x["rule_id"] for x in d["diagnostics"])'
check "the draft's errors list one line per error diagnostic" \
    jcheck "$WS/last.json" 'len(d["errors"]) == sum(x["severity"] == "error" for x in d["diagnostics"]) and all(isinstance(e, str) for e in d["errors"])'
check "validating a draft did not write it" same_sha "$BEFORE" "$WS/plans/$PLAN/tasks.toml"

# ── Generate ──────────────────────────────────────────────────────────────
check "generate rejects a body with neither prompt nor slug" [ "$(api POST /api/plans/generate '{}')" = 422 ]
check "generate rejects a body with both prompt and slug" \
    [ "$(api POST /api/plans/generate '{"prompt":"x","slug":"y"}')" = 422 ]
LEGACY_BEFORE="$(sha "$WS/plans/legacy-fixture/tasks.toml")"
code="$(api POST /api/plans/generate '{"prompt":"a rust app that prints hello world"}')"
GEN="$(jget "$WS/last.json" 'd["plan_id"]')"
OP="$(jget "$WS/last.json" 'd["id"]')"
check "generate from a sentence returns 202 with the new plan's slug" [ "$code" = 202 -a -n "$GEN" ]
check "the generation operation finishes within 120s" wait_operation "$OP" 120
check "the operation reports the plan it produced" \
    jcheck "$WS/last.json" "d['status'] == 'completed' and d['result']['slug'] == '$GEN' and d['result']['task_count'] >= 1"
check "generation completion is announced on the event stream" \
    sse has event_log_entry event_type=plan_generate.completed plan_id="$GEN"
check "generation did not rewrite other plans" generated_and_legacy_untouched
check "validate the generated plan returns 200" [ "$(api POST "/api/plans/$GEN/validate")" = 200 ]
check "the generated plan reports valid" jcheck "$WS/last.json" 'd["valid"] is True'

# ── Run what was generated ────────────────────────────────────────────────
check "execute the generated plan (202)" [ "$(api POST "/api/plans/$GEN/execute")" = 202 ]
check "the generated plan's run finishes within 180s" wait_idle "$GEN" 180
check "the result is a program that prints hello world" hello_runs

finish "AUTHORING-CHECK"
