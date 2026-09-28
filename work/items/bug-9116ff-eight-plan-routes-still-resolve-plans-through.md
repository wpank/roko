+++
id = "bug-9116ff"
kind = "bug"
title = "Eight plan routes still resolve plans through the flat-file find_plan"
status = "done"
triage = "verified"
severity = "p2"
subsystem = ["roko-serve/plans"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-29
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-serve/src/routes/plans.rs::find_plan"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[repro]]
command = 'test "$(grep -c "find_plan(&state.workdir" crates/roko-serve/src/routes/plans.rs)" -eq 0'

[[verify]]
command = 'cargo test -p roko-serve --test plan_discovery'

[closed]
at = 2026-09-29
by = "plan:portal-programme/03b-backend-workspace-server#T17-T18"
run_id = "graph-03b-backend-workspace-server-0eb76a35-67fe-4086-9cfe-3846dfe7aaaf"
evidence = "WORKSPACE-SERVER-CHECK PASS (58 checks). 'PASS the costs route finds a directory plan', 'PASS the gates route finds a plan inside a plan set', 'PASS POST /resume accepts a plan inside a plan set', and 'PASS the resumed nested plan completes' confirm that resume and all previously-broken read routes now resolve directory plans through runtime discovery. 'PASS the costs route still answers 404 for an unknown plan' confirms correct 404 behaviour is preserved."
+++

Listing, task reads and execute now go through `CliRuntime` (working tree), but eight handlers still call `find_plan` (`routes/plans.rs:1602`), which only probes `{id}.json` / `{id}.toml` and parses a flat schema.
Call sites: resume (`:475`) and `:636`, `:675`, `:873`, `:966`, `:1113`, `:1209`, `:1294` (costs, gates, estimate, task diff, chat, reviews list and submit).
For directory plans (`plans/<slug>/tasks.toml`, the real layout) these return 404 or an empty plan.
Fix: route each through `runtime.load_plan_summary` / `load_plan_tasks` and delete `find_plan`; `plans/portal-programme/01-backend-plan-service` T07 removes it only "unless still needed".
