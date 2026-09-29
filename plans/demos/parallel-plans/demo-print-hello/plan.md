---
plan: demo-print-hello
estimated_tasks: 1
estimated_parallel_width: 1
estimated_minutes: 1
parallel_safe: true
tags: [demo, hello-world, parallel-plans, fake-agent]
---

# Print hello demo

One implementer task writes `demo/print-hello/main.rs`, a program that prints a
one-line greeting starting with `hello`; the verify step compiles it with
`rustc` and checks the output.

It is the other half of the `parallel-plans` set. It shares nothing with
`demo-hello-world`, so a set run with `max_parallel_plans = 2` starts both
plans at once. Like its sibling, it passes with the deterministic fake agent.

## Origin

Converted from `plans/scratch-test`, left by a manual portal test on
2026-09-25. Its `tasks.toml` held one task (`T01`, title `Hello`, description
`Print hello`, files `["test.txt"]`) written as a `[[tasks]]` table, which the
current parser rejects.
