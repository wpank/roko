+++
id = "gap-33d458"
kind = "gap"
title = "MCP plan_generate and POST /api/plans/generate take no max_usd, so a chat's cap on plan writing falls back to [authoring] max_generation_usd ($2), and the start answer does not say which cap applies"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-serve/plans"]
created = 2026-10-20
updated = 2026-10-20
last_verified = 2026-10-20
source = "round-v7-plugin-wow#P-7"
discovered_from = "agent:plugin-wow (round v7, P-7)"
anchors = ["crates/roko-serve/src/routes/plans/authoring.rs::start_plan_generation", "crates/roko-cli/src/serve_runtime.rs::generate_plan"]
links = { depends_on = [], blocks = [], related = ["gap-18bfa2", "gap-fe4225"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn mcp_plan_generate_takes_a_spend_cap' crates/roko-serve/src && cargo test -p roko-serve --lib -- plan_generate_takes_a_spend_cap"
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
