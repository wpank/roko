+++
id = "q-4299a9"
kind = "question"
title = "Should a plan generated in a workspace without plans/ be written to plans/ rather than the legacy .roko/plans/?"
status = "open"
triage = "verified"
severity = "p3"
goal = "visibility"
subsystem = ["roko-serve/plans"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "f99e45dba"
source = "plan:portal-programme/09-acceptance#T04"
discovered_from = "plan:portal-programme/09-acceptance#T04"
anchors = ["crates/roko-serve/src/routes/plans.rs::plans_dir"]
links = { depends_on = [], blocks = [], related = ["bug-9f340c"], supersedes = [], duplicate_of = "" }
+++

## Problem

In a fresh `roko init` workspace, which is the path 09 T03 proves, the portal's first generated plan
is written to `.roko/plans/<slug>/tasks.toml`. The real run's `serve.log` reads "Wrote tasks.toml
(1290 bytes) to /private/tmp/roko-hello-JdC7dN/.roko/plans/a-rust-app-that-prints-hello-world"
(`tmp/portal-audit/evidence/hello-world-real/serve.log`). `plans_dir` returns `plans/` only when it
already exists; otherwise it returns `.roko/plans`, which its own doc comment calls the legacy
location. `.roko/` is roko's runtime-state directory (checkpoints, logs, `runtime/serve.token`), so
the user's first plan sits beside runtime state rather than with their code. bug-9f340c (residual 2)
notes that plans under `.roko/plans` stop being reachable once a `plans/` directory exists.

## Why it matters

Goal `visibility`. This is the first thing a new user makes.

## Where

`crates/roko-serve/src/routes/plans.rs::plans_dir` (and the identical helper in roko-cli that it
mirrors).

## Plan

Options:

- (a) Create `plans/` for the first generated plan.
- (b) Keep `.roko/plans` and list both roots (bug-9f340c).
- (c) Keep it as is, and document it.

## Done when

Will decides. The chosen option then becomes a bug or gap item with a verify command.

## Notes

Raised in `tmp/dogfood/2026-09-28-portal-programme-continuation.md` (09:55 entry) and seen in the 09
real-model run. Filed by plan 09 T04 (see VERDICT).
