+++
id = "gap-4ba837"
kind = "gap"
title = "A pass the gate took from the try's own checks (gap-6df2bf, self_checked) is drawn for audit in the passed stratum, so its false-green rate can't be told apart"
status = "done"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
last_verified_rev = "373cd403d"
source = "round-v7 lead (2026-10-26, gap-6df2bf follow-up)"
discovered_from = "agent:team-lead (round v7, gap-6df2bf review)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/audit_select.rs::stratum_of"]
links = { depends_on = [], blocks = [], related = ["gap-6df2bf"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_self_checked_pass_is_drawn_in_its_own_stratum' crates/roko-cli/src && cargo test -p roko-cli --lib -- a_self_checked_pass_is_drawn_in_its_own_stratum"

[closed]
at = 2026-10-26
at_ts = "2026-10-26T15:36:46Z"
commit = "373cd403d"
by = "roko-exact (round v7)"
executor = "unknown"
forced = false
evidence = "373cd403d: audit_select's STRATA gains passed_self_checked, and draw() puts a passed verdict with any self_checked step there, at passed's \u03c0 (not a census). Verify: a_self_checked_pass_is_drawn_in_its_own_stratum passes (T1 plain pass drawn as passed, T2 with a self_checked step as passed_self_checked, same \u03c0); audit_select:: and commands::audit 13/13; nightly fmt and clippy -p roko-cli -D warnings clean."
+++

## Problem

Since gap-6df2bf, the gate can take a step's pass from the try's own checks of the same tree instead of running the
step again. Its verdict row then says `self_checked: true`. The audit lottery draws such a green attempt in the
`passed` stratum, like any other pass, so the audit estimates can't tell its false-green rate apart. Yet that rate
is exactly what tells whether reusing a self-check is safe.

## Plan

- Draw a passed attempt whose verdict has a `self_checked` step in a stratum of its own, `passed_self_checked`, at
  the same π as `passed`, not a census.
- The run's report lists the stratum with the others.

## Done when

The test in `[[verify]]` passes: a passed attempt with a self_checked step is drawn as `passed_self_checked`, and
one without as `passed`, at the same π.
