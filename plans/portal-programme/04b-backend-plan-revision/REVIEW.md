# 04b-backend-plan-revision — Acceptance Review

## Check output

```
PASS revise returns 202 with an operation id
PASS the response names the plan
PASS the revision finishes within 120s
PASS the operation reports the revised plan
PASS the plan's source now carries the new task
PASS the revised plan is valid
PASS revision completion is announced on the event stream
PASS the revised plan runs, new task included
PASS an invalid revision is still accepted for processing (202)
PASS an invalid revision fails its operation
PASS a failed revision leaves the plan byte-identical
PASS revision failure is announced on the event stream
PASS blank feedback is rejected (422)
PASS an unknown plan is 404
PASS a plan cannot be revised while it runs (409)
REVISION-CHECK: PASS (15 checks)
```

## Verdict

REVISION-CHECK: PASS
