+++
id = "gap-33d458"
kind = "gap"
title = "MCP plan_generate and POST /api/plans/generate take no max_usd, so a chat's cap on plan writing falls back to [authoring] max_generation_usd ($2), and the start answer does not say which cap applies"
status = "done"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-serve/plans"]
created = 2026-10-20
updated = 2026-10-20
last_verified = 2026-10-20
last_verified_rev = "d926f2b69"
source = "round-v7-plugin-wow#P-7"
discovered_from = "agent:plugin-wow (round v7, P-7)"
anchors = ["crates/roko-serve/src/routes/plans/authoring.rs::start_plan_generation", "crates/roko-cli/src/serve_runtime.rs::generate_plan"]
links = { depends_on = [], blocks = [], related = ["gap-18bfa2", "gap-fe4225"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn mcp_plan_generate_takes_a_spend_cap' crates/roko-serve/src && cargo test -p roko-serve --lib -- plan_generate_takes_a_spend_cap"

[closed]
at = 2026-10-20
at_ts = "2026-10-20T21:06:05Z"
commit = "ee0bb0e0d"
by = "roko-planner (round v7)"
executor = "claude-agent"
via = "manual"
forced = false
evidence = "Merged 55d0acda8 (work/gap-33d458: ee0bb0e0d). POST /api/plans/generate and MCP plan_generate take an optional max_usd (finite, above 0; over MCP at most [serve.mcp] max_run_usd via mcp_cap, which run_cap now uses). routes::plans::GenerationCap resolves the cap that applies (the request's, else [authoring] max_generation_usd, 0 none) and serve passes it through CliRuntime::generate_plan_reporting (new max_usd parameter) into PlanRequest.max_usd; both start answers carry max_usd (null when uncapped) and max_usd_source (request|config); docs/v3/26-HTTP-API.md updated. Post-merge verify at d926f2b69 (contains 55d0acda8): grep OK; cargo test -p roko-serve --lib plan_generate_takes_a_spend_cap: 2 passed (HTTP and MCP). Branch: roko-serve lib 1075 passed, 0 failed; roko-serve plan_authoring integration 19 passed; clippy -p roko-serve -p roko-cli -D warnings clean; nightly fmt. $0 smoke through a real roko serve (fake claude_cli planner, replies $0.10 with no plan): HTTP max_usd 0.05 -> answer 0.05/request, progress.max_usd 0.05, stopped after 1 call ('reached its spend cap: its 1 planner call(s) cost $0.1000 of $0.05'); without max_usd -> 2.0/config, 3 calls; MCP 0.05 -> 0.05/request; MCP 9.0 refused above the 5.00 ceiling; HTTP 0 -> 422."
+++

## Problem

roko's plan pipeline takes a cap per request (gap-18bfa2: `PlanRequest.max_usd`, `roko plan generate --max-usd`), but
roko serve never passes one. `ServeRuntime::generate_plan` builds its `PlanRequest` with `max_usd: None`, and neither
the MCP tool `plan_generate` nor `POST /api/plans/generate` takes a `max_usd`. So when a chat says "cap the plan
writing at $0.60", the cap that applies is `[authoring] max_generation_usd` ($2 by default). The start answer does
not say which cap applies either, so a host cannot tell the person.

Found by plugin-wow (round v7, P-7), on the routes that gained `spec_path` in gap-fe4225.

## Why it matters

Cost control is part of roko's pitch: the person names what a step may spend, and roko holds to it. A chat host that
plans for a person needs to set that cap and say what it is.

## Where

- `crates/roko-serve/src/routes/plans/authoring.rs`: `GenerateRequest`, `generate_plan`, `start_plan_generation`.
- `crates/roko-serve/src/routes/mcp.rs`: the `plan_generate` tool's schema and its handler.
- `crates/roko-serve/src/runtime.rs`: `CliRuntime::generate_plan_reporting`.
- `crates/roko-cli/src/serve_runtime.rs`: `generate_plan`, which builds the `PlanRequest`.

## Plan

1. `POST /api/plans/generate` and MCP `plan_generate` take an optional `max_usd`: finite and above 0. Over MCP it
   is also at most `[serve.mcp] max_run_usd`, as for the run tools.
2. Serve resolves the cap that applies: the request's, else `[authoring] max_generation_usd` (0 sets none). It passes
   that cap to the runtime, and the runtime passes it to the pipeline as `PlanRequest.max_usd`.
3. The start answer gives `max_usd` (null when uncapped) and `max_usd_source` (`request` or `config`).

## Done when

The tests in `[[verify]]` pass. Over HTTP and over MCP, a request's `max_usd` reaches the runtime and is echoed with
source `request`. Without one, the config's cap is used and echoed with source `config`. A cap of 0, a negative cap or
a non-number is refused, and over MCP so is a cap above `max_run_usd`.
