---
plan: demo-hello-world
estimated_tasks: 1
estimated_parallel_width: 1
estimated_minutes: 1
parallel_safe: true
tags: [demo, hello-world, parallel-plans, fake-agent]
---

# Hello world demo

The smallest end-to-end run that produces a working program: one implementer
task writes `demo/hello-world/main.rs`, and the verify step compiles it with
`rustc` and checks that it prints exactly `hello world`.

It is one half of the `parallel-plans` set. Its sibling, `demo-print-hello`,
writes a different directory, so the two plans can run at the same time.

The task also passes with the deterministic fake agent
(`plans/portal-programme/_harness/fake-claude`), which writes a hello-world
program for every `.rs` path named after `ARTIFACT` in a prompt. That makes
this plan a free walkthrough of the portal's run view.

## Origin

Converted from `plans/hello-world`, a plan the portal's create flow wrote on
2026-09-25 with only a `plan.md` whose body was the prompt `hello world`.
