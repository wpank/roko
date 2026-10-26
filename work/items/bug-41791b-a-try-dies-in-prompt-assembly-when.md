+++
id = "bug-41791b"
kind = "bug"
title = "A try dies in prompt assembly when an explicit symbol anchor is gone from its declared files, as a stale range did before bug-fa22cc: a try that renamed or removed the symbol leaves every later try of its task dead before any model call"
status = "open"
triage = "unverified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-10-26
updated = 2026-10-26
source = "round-v7 roko-exact (found fixing bug-fa22cc)"
discovered_from = "agent:roko-exact (round v7)"
anchors = ["crates/roko-cli/src/plan_policy.rs::render_declared_context"]
links = { depends_on = [], blocks = [], related = ["bug-fa22cc", "bug-c5f406"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_missing_anchor_never_fails_a_try' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_missing_anchor_never_fails_a_try"
+++

## Problem

A try dies in prompt assembly when an explicit symbol anchor in its task's `context.symbols` is not found in any of
its declared files, as they stand in the try's checkout. `plan_policy::render_declared_context` returns "explicit
symbol anchors were not found in declared context: <anchor>", which becomes `PreValidationFailed`: a harness_error
before any model call.

The anchors are checked against the try's checkout. A retry reuses its last try's checkout, so a try that renamed or
removed the symbol leaves every later try of its task dead, at $0, as a stale line range did before bug-fa22cc. This
is the same failure as bug-fa22cc's: pre-validation checks the plan's context against files the plan's own work has
changed.

It hasn't been seen live yet. Phase 5.1's T2 declares `symbols = ["undertow_render — offline renderer entry point"]`,
which `explicit_symbol_anchor` reads as the anchor `undertow_render`. T2 survives only because its spec, which it
also declares, names the symbol.

**Expected:** a missing anchor never fails the try. The declared files render without the anchor's lines, and the
prompt says which anchors were not found.

## Why it matters

A task that renames what its plan anchored can never be retried. Each dead try counts toward the task's retries.

## Where

- `crates/roko-cli/src/plan_policy.rs::render_declared_context`: the `missing` anchors return `Err`.
- The plan check at load (`validate_plan_context`, PLAN_CONTEXT_SYMBOL) should stay: before anything runs, a
  missing anchor is a planning error.

## Plan

In `render_declared_context`, name the missing anchors in a line of the rendered context rather than return an
error. Test it with a declared file whose anchored function a try renamed.

## Done when

The test in `[[verify]]` passes: a try whose declared files no longer hold an anchored symbol renders its context,
naming the missing anchor, and does not fail.

## Notes

- Related: bug-fa22cc (stale ranges, the same failure), bug-c5f406.
- Found while fixing bug-fa22cc. Predicted from the code path; not yet seen live.
