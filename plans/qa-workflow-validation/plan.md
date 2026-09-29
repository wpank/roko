---
plan: qa-workflow-validation
---

# QA workflow validation

This plan validates that the recent portal and workflow changes work correctly
by creating three executable test scripts (API, lifecycle, CLI) and a reviewer
task that runs them and writes a results document.

The four tasks form a linear-then-fan-in dependency: T01 must complete before
T02 and T03 can run in parallel; T04 depends on all three and produces the final
verdict.

No production code is modified by this plan. All outputs land under
`demo/qa-validation/`.
