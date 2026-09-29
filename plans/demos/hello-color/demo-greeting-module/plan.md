---
plan: demo-greeting-module
estimated_tasks: 1
estimated_parallel_width: 1
estimated_minutes: 2
parallel_safe: false
tags: [demo, rust, plan-set, cross-plan-dependency]
---

# Greeting module demo

Adds a `greeting` module to the crate that `demo-hello-color` builds: one
implementer task writes `demo/hello-color/src/greeting.rs` with a unit test and
wires it into `src/main.rs`, keeping the coloured output unchanged.

The task declares `depends_on_plan = ["demo-hello-color"]`, which is what this
demo shows:

- `roko plan run plans/demos/hello-color` runs both plans of the set,
  `demo-hello-color` first.
- Running this plan on its own before `demo-hello-color` has succeeded in the
  workspace stops before any agent starts, naming the missing prerequisite.
  Once `demo-hello-color` has a succeeded checkpoint, this plan runs alone.

## Origin

Converted from `plans/test-greeting-module`, left by a manual portal test on
2026-09-24. Its `plan.md` held the prompt `add a greeting module to
hello-color`, and its `tasks.toml` held one implementer task (`T01`, `Add
greeting module`, files `["src/greeting.rs"]`) as a `[[tasks]]` table, which
the current parser rejects. The file path now points into
`demo/hello-color/`.
