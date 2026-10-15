---
plan: demo-resume-recovery
---

# Resume recovery demo

This example deliberately stops after its first task, then resumes in a second
Roko process. It proves that a completed task's accepted Git tip survives the
process boundary and is included in the final merge.

It runs from a terminal only: the second run needs `ROKO_RESUME_DEMO_READY=1`,
which neither a chat host such as Hermes nor the portal can set, so from there
it could only end failed at 1/2. It therefore sits under `plans/demos/_cli/`,
which plan discovery skips (so `roko plan list`, the portal and Hermes do not
offer it); name its directory to run it:

```sh
roko plan run plans/demos/_cli/demo-resume-recovery --fresh --max-retries 0
ROKO_RESUME_DEMO_READY=1 roko plan run plans/demos/_cli/demo-resume-recovery
```
