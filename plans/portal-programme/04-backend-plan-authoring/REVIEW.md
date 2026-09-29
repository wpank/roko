# 04-backend-plan-authoring — Acceptance Review

## Check run

Date: 2026-09-29  
Binary: `target/debug/roko` built from working-tree head  
Command: `cargo build -p roko-cli && bash plans/portal-programme/_harness/authoring-check.sh`

### Full output

```
PASS POST /api/plans creates a plan (201) and returns its slug
PASS the created plan is a directory plan
PASS the created plan is listed
0 diagnostics in 1 plan
PASS the created plan passes roko plan validate
PASS tasks carry verify commands and model_hint
PASS GET source returns the plan's TOML
PASS the source parses as a plan
PASS PUT source saves a valid edit (200)
PASS the save reports no errors
PASS the saved source comes back byte-for-byte, comment included
0 diagnostics in 1 plan
PASS the saved plan passes roko plan validate
PASS a save with an unknown dependency is rejected with 422
PASS the 422 names the offending task
PASS a save with a TOML syntax error is rejected with 422
PASS rejected saves left the file byte-identical
PASS a save while a run includes the plan is refused (409)
PASS validate a saved plan returns 200
PASS the saved plan is valid
PASS validate an unsaved invalid draft still returns 200
PASS the draft is reported invalid with an error on the task
PASS validating a draft did not write it
PASS generate rejects a body with neither prompt nor slug
PASS generate rejects a body with both prompt and slug
PASS generate from a sentence returns 202 with the new plan's slug
PASS the generation operation finishes within 120s
PASS the operation reports the plan it produced
PASS generation completion is announced on the event stream
PASS generation did not rewrite other plans
PASS validate the generated plan returns 200
PASS the generated plan reports valid
PASS execute the generated plan (202)
PASS the generated plan's run finishes within 180s
PASS the result is a program that prints hello world
AUTHORING-CHECK: PASS (33 checks)
```

### Note on the stale binary

The first run of the check (before `cargo build`) used a pre-existing binary compiled
from an earlier commit.  That binary's `CreatePlanRequest` still required a `description`
field (removed in the working-tree implementation), so `POST /api/plans` returned 400 and
all 33 checks failed.  After a fresh `cargo build -p roko-cli` the working-tree code was
compiled and all checks passed with no code changes.

## AUTHORING-CHECK: PASS

---

## Real-model check

This is the operator's acceptance test described in `tmp/portal-audit/00-README.md`:
*"Open the portal. Type "a rust app that prints hello world". Watch it build."*

Run it against a fresh workspace (not the repository itself) so the workspace lock does not
conflict with any existing `roko serve` process.

### Prerequisites

- `cargo build -p roko-cli` (binary at `target/debug/roko`, or install with `cargo install`)
- A valid `ANTHROPIC_API_KEY` in the environment or `~/.roko/.env`
- The fake-claude check already passed (proves the plumbing is correct)

### Commands

```bash
# 1. Create a fresh workspace outside the repo.
mkdir -p ~/roko-demo && cd ~/roko-demo

# 2. Initialise it.
/path/to/roko init

# 3. Cap spend at $1.50 so the run stops if something loops.
#    Edit roko.toml and add:
#      [budget]
#      max_plan_usd = 1.50
cat >> roko.toml <<'EOF'

[budget]
max_plan_usd = 1.50
EOF

# 4. Start the serve process in the background (default port 6677).
/path/to/roko serve &
SERVE_PID=$!

# 5. Generate a plan from the one-sentence prompt.
curl -s -X POST http://localhost:6677/api/plans/generate \
     -H 'content-type: application/json' \
     -d '{"prompt":"a rust app that prints hello world"}' | tee gen.json
PLAN_ID=$(python3 -c 'import json,sys; print(json.load(open("gen.json"))["plan_id"])')
OP_ID=$(python3 -c 'import json,sys; print(json.load(open("gen.json"))["id"])')

# 6. Poll the operation until it finishes (should be < 60 s).
while true; do
  STATUS=$(curl -s "http://localhost:6677/api/operations/$OP_ID" | python3 -c 'import json,sys; print(json.load(sys.stdin)["status"])')
  echo "operation status: $STATUS"
  [ "$STATUS" != "running" ] && break
  sleep 3
done

# 7. Validate the generated plan.
curl -s -X POST "http://localhost:6677/api/plans/$PLAN_ID/validate" | python3 -m json.tool

# 8. Execute the generated plan (real Claude agent, costs real money up to $1.50).
curl -s -X POST "http://localhost:6677/api/plans/$PLAN_ID/execute"

# 9. Wait for completion (the hello-world compile + run should take < 2 min).
while true; do
  FINISHED=$(curl -s "http://localhost:6677/api/plans/$PLAN_ID/status" | python3 -c 'import json,sys; print(json.load(sys.stdin).get("finished", False))' 2>/dev/null)
  echo "finished: $FINISHED"
  [ "$FINISHED" = "True" ] && break
  sleep 5
done

# 10. Check that the artefact was produced and prints the right text.
./hello/hello-bin | grep 'hello world' && echo "REAL-MODEL-CHECK: PASS" || echo "REAL-MODEL-CHECK: FAIL"

# Cleanup
kill $SERVE_PID
```

### What to check

| Step | Expected |
|---|---|
| `POST /api/plans/generate` | Returns 202 with `plan_id` and `id` (op id) |
| Operation poll | Finishes with `status: completed` and `result.task_count >= 1` |
| `POST /api/plans/{id}/validate` | Returns `{ "valid": true }` |
| `POST /api/plans/{id}/execute` | Returns 202 |
| Plan status poll | `finished: true` within 2 minutes |
| `./hello/hello-bin` | Prints a line containing `hello world` |

If all six pass: the programme's thesis — *a sentence becomes a plan that runs* — is proved
against the real model.
