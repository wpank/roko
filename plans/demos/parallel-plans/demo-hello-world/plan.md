---
plan: demo-hello-world
estimated_tasks: 1
estimated_parallel_width: 1
estimated_minutes: 1
parallel_safe: true
tags: [demo, hello-world, parallel-plans, fake-agent, start-here]
---

# Start here: hello world demo

The demo to run first, from Hermes, the portal or the terminal. It is the
smallest end-to-end run that produces a working program: one implementer task
writes `demo/hello-world/main.rs`, and the verify step compiles it with `rustc`
and checks that it prints exactly `hello world`. On the default model ladder it
takes about a minute and costs well under a cent.

The plan's `[meta] verify` runs the same check on the merged result. Without it,
a plan that writes a `.rs` file gets the Cargo workspace's default whole-plan
check (fmt, clippy and tests of the packages it may touch), which in the roko
repository is a build of the whole workspace.

The program is demo output and says so in its first line. The run leaves it on
its own branch (`roko/batch/<run>`): look at the diff, and do not merge it. If
it lands on your branch, the next run of this demo finds its check already
passing, and roko refuses a plan whose checks pass before an agent has started.

It is one half of the `parallel-plans` set. Its sibling, `demo-print-hello`,
writes a different directory, so the two plans can run at the same time.

The task also passes with the deterministic fake agent
(`plans/portal-programme/_harness/fake-claude`), which writes a hello-world
program for every `.rs` path named after `ARTIFACT` in a prompt. That makes
this plan a free walkthrough of the portal's run view.

## Origin

Converted from `plans/hello-world`, a plan the portal's create flow wrote on
2026-09-25 with only a `plan.md` whose body was the prompt `hello world`.
