---
plan: demo-hello-color
estimated_tasks: 5
estimated_parallel_width: 2
estimated_minutes: 6
parallel_safe: true
tags: [demo, rust, multi-role, parallel-tasks, plan-set]
---

# Example: Hello World with Colored Output

This plan demonstrates the multi-role, multi-stage roko plan-execute-gate-persist loop
using a concrete Rust project: a small CLI binary that prints "Hello, World!" in green
with a rose-colored border.

The plan runs five tasks with the two roles that may write files, and shows how
roko schedules parallel work once shared dependencies are satisfied.

## What gets built

`demo/hello-color/` — a standalone Rust binary crate with:

- `research.md` — a comparison of terminal-colour crates, written by a scribe
- `Cargo.toml` — declares a dependency on the `colored` crate
- `src/main.rs` — prints a colored greeting
- `REVIEW.md` — code-quality review notes, written by a scribe
- `README.md` — user-facing documentation written by the scribe role

## Execution shape

```
T01 (scribe: research) ────────────────────────────────────────────┐
                                                                   │ (informational only)
T02 (implementer, no deps) ──► T03 (implementer) ──► T04 (scribe: review)  ─► (done)
                                                  └──► T05 (scribe: README) ─► (done)
```

T01 runs independently to research the best crate for colored terminal output.  Its
findings inform T03 but are not a hard dependency — the implementer role can read the
research output file directly.

T02 and T03 are sequential: T02 scaffolds the crate, T03 adds the colored logic.

T04 (review) and T05 (README) both depend only on T03 and are independent of each
other, so they run in parallel under `max_parallel = 2`. T01 and T02 also start
together.

The first version of this plan gave T01 the `researcher` role and T04 the
`reviewer` role. Neither role may write files, and on the 2026-09-23 run T01
could not create `research.md`. Both tasks now use `scribe`.

## Plan set

This plan is the first half of the `hello-color` set. Its sibling,
`demo-greeting-module`, declares `depends_on_plan = ["demo-hello-color"]`, so
running the set directory runs this plan first.

## Running this plan

```bash
# From the workspace root, this plan alone:
roko plan run plans/demos/hello-color/demo-hello-color --fresh

# Or the whole set, this plan first:
roko plan run plans/demos/hello-color --fresh

# Watch progress in the TUI:
roko dashboard
```

The visible proof that the plan ran end-to-end is the presence of
`demo/hello-color/src/main.rs`, `demo/hello-color/REVIEW.md`, and
`demo/hello-color/README.md` after execution completes. `demo/hello-color/`
is checked in from the 2026-09-23 run, so in this repository T02-T05 find their
files already in place; T01 still writes `research.md`.

## Origin

Converted from `plans/example-hello-world` (2026-09-23). This `plan.md` is its
original text with the roles, paths and run commands updated; its `tasks.toml`
had been emptied to `tasks = []`, so the five tasks were rebuilt from this
description and from the outputs of the 2026-09-23 run (Graph checkpoint
`example-hello-world`, and the files under `demo/hello-color/`).
