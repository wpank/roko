+++
id = "bug-16b798"
kind = "bug"
title = "A plan generation that fails after its retries always says the output 'did not contain a parseable toml block' and, when it holds [meta] or [[task]], blames the fencing and suggests a more capable model, even when every draft parsed and failed the structural budget (Undertow phase 2: three fenced drafts)"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/plan-generate"]
created = 2026-10-20
updated = 2026-10-20
last_verified = 2026-10-20
source = "round-v7-hermes-pilot#E-p2-plan"
discovered_from = "agent:hermes-pilot (round v7 phase 2, op c31152b2)"
anchors = ["crates/roko-cli/src/plan_generate/pipeline.rs::generate_plan"]
links = { depends_on = [], blocks = [], related = ["bug-b59d8e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_budget_failure_names_the_budget_not_the_fencing' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_budget_failure_names_the_budget_not_the_fencing"
+++

## Problem

When plan generation fails after its retries, `generate_plan` always ends the error with:

```
The agent output (10944 bytes) did not contain a parseable ```toml block.
hint: Try again, or create plans/p2-undertow-rumble/tasks.toml manually.
hint: The model output TOML without proper fencing. Try a more capable model or check the plan_generate system prompt.
```

It prints the first line whatever the last error was. It adds the fencing hint whenever the output contains `[meta]` or
`[[task]]`.

In round v7's phase 2 (roko 5de243ec8, operation `c31152b2`), all three drafts were fenced and parsed
(`has_toml_block=true`, toml_block_len 10932, 11454 and 11097). Every one of them failed the structural budget
(`PLAN_FRAGMENTED_OWNERSHIP` on `core/CMakeLists.txt`). The message names the real error on its "Last error" line, then
contradicts it with two lines blaming the fencing and suggesting a more capable model.

## Why it matters

The person reads this error in the chat or on the roko page. "Try a more capable model" sends them to a pricier planner
for a failure no model choice fixes, and "create tasks.toml manually" sends them to hand-edit plans. The real fix lies
in the spec's structure or in roko's budget.

## Where

- `crates/roko-cli/src/plan_generate/pipeline.rs`: `generate_plan`, at the "Plan generation failed after retries" error
  (`toml_hint`).

## Plan

Say why the last draft was refused, by kind:
- No fenced TOML block: the current text.
- TOML that didn't parse: the parse error.
- A draft that parsed but failed validation or the budget: the findings, and that a different model is unlikely to
  help. Point at the spec's structure or the budget.

Print the fencing hint only when no draft had a fenced TOML block.

## Done when

A generation that fails only on budget findings says so, with no fencing or model hint. Checked by the test in
`[[verify]]`.

## Notes

- Evidence: `tmp/hermes-roko-v7/pilot/evidence/p2-plan/operation.jsonl` and `roko-log-excerpt.txt`.
