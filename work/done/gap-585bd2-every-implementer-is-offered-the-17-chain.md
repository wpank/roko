+++
id = "gap-585bd2"
kind = "gap"
title = "Every implementer is offered the 17 chain tools, transfer and swap included, whatever the task domain"
status = "done"
triage = "verified"
severity = "p2"
goal = "release"
size = "M"
subsystem = ["roko-std/tools", "roko-cli/graph_task_dispatch"]
created = 2026-09-29
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "9c0b9aed0"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (20:01, wk-bench-rokoarm's report on gap-b7ab99)"
anchors = ["crates/roko-std/src/roles.rs::compose_profile", "crates/roko-std/src/roles.rs::domain_profile", "crates/roko-std/src/tool/handlers.rs::chain_handler_for", "crates/roko-cli/src/task_parser.rs::TaskDef", "crates/roko-cli/Cargo.toml"]
lane = "rust-cold"
parent = "spec-ba7bea"
links = { depends_on = [], blocks = [], related = ["gap-7a3527", "bug-12153c", "gap-b7ab99"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_coding_task_is_offered_no_chain_tools' crates/roko-cli/src/ && cargo test -p roko-cli --lib a_coding_task_is_offered_no_chain_tools"

[closed]
at = 2026-09-30
by = "coordinator (session 7622b882)"
evidence = "Merged in batch 13 (ca834b178: chain tools offered only to chain-domain tasks by domain tool ownership) and batch 15a (1e54497e4: effective_agent_contract resolves the project default domain at all three call sites). Batch 15a gate on 9005da604, re-assembled as 8a2ee8bca with only settle's rustfmt commit changing two files' formatting (MAIN 9c0b9aed0 has the same code): cargo check --workspace --tests clean; nightly fmt clean; clippy -p roko-agent -p roko-cli -p roko-compose -p roko-core -p roko-learn -p roko-serve --keep-going -D warnings clean; lib tests pass: roko-cli 3190 (two flakes, the turn_policy escalated-timeout test and graph_run_routing_observations_survive_a_crash, pass alone and in their module), roko-agent 2268, roko-core 1952, roko-learn 1204, roko-serve 986, roko-compose 560; cargo test -p roko-cli --test learning_wiring_census: 2 passed. Verify: its test passes in that run and its static checks pass on MAIN."
+++

## Problem

roko-cli builds with the `chain` feature by default: `crates/roko-cli/Cargo.toml:15` has `default = ["acp", "chain", "hdc"]`, and :19 turns on `roko-std/chain`. The tool catalog therefore holds 17 `chain.*` tools:

- add_liquidity, approve, balance, confirm_insight, gas_estimate;
- get_pool_info, get_position, post_insight, remove_liquidity;
- search_insights, simulate_tx, swap, transfer;
- wallet_create, wallet_export_address, wallet_info, wallet_list.

OpenAI-compatible providers see them as `chain__DOT__*` (`roko-agent/src/translate/openai.rs:164`). The ViabilityBench Roko arm's implementer was offered all 17 on a Python task.

Domain tool profiles exist in `crates/roko-std/src/roles.rs` (:173-330): `DomainToolProfile`, with coding, chain, research and general profiles; `domain_profile(domain)`; and `compose_profile(role, domain, overrides)`. Nothing outside that file calls `compose_profile` or `domain_profile`. So tool exposure ignores the task's domain (`TaskDef::domain`, task_parser.rs:107).

## Why it matters

Least privilege (epic spec-ba7bea): a coding task is offered `transfer`, `approve`, `swap` and `wallet_create`.

- **Today.** With no live chain backend, the handlers fail with "not wired" (`handlers.rs`, test `chain_catalog_handlers_fail_explicitly_without_a_live_backend`). The cost is 17 extra tool schemas on every turn, and a distraction for cheap models.
- **With a backend attached,** a coding agent could move funds.

This is the tool-side twin of gap-7a3527's `gates.domain_gates`: the task domain should select gates, and it doesn't select tools either.

## Where

- The unused profiles: `roles.rs::compose_profile` and `domain_profile`.
- The chain handlers: `roko-std/src/tool/handlers.rs::chain_handler_for`.
- The task's domain: `TaskDef::domain`.
- The place Graph dispatch assembles a task's tool definitions for the tool loop. Find it first; it's in roko-cli or roko-agent's dispatcher.

## Current state

At BASE (4315add32), the role-based deny lists (`denied_tools_for_role`) are the only tool filter. Domain profiles are built and tested, but not wired.

## Plan

1. Where dispatch builds a task's tool list, apply `compose_profile(role profile, domain_profile(domain), overrides)`. Take the domain from the task, then the plan, then `project.default_domain`, then "coding".
2. Offer `chain.*` tools only to the chain domain, or to a task that asks for them explicitly.
3. Record the number of tools offered in the dispatch record, so benchmarks can check it.
4. Add `a_coding_task_is_offered_no_chain_tools`: a coding task's tool list holds no `chain.` tool, and a chain-domain task gets the read tools.

## Done when

- [ ] A coding or Python task is offered no chain tools. A chain-domain task still gets them, per its profile.
- [ ] The `[[verify]]` command passes.

## Notes

- `project.default_domain` is currently dropped when roko.toml is loaded (bug-12153c). A default taken from it depends on that fix.
- Don't turn off the `chain` feature as the fix: the profile decides exposure, not the build.
- 2026-09-30 (wk-guard2): `compose_profile`'s allowlist is deliberately not applied wholesale. It lists builtins only, and its exclusions would deny `write_file` to smart-contract and report tasks. Only domain tool ownership (the chain tools) is enforced, and plan step 3 (counting the offered tools in the dispatch record) is skipped.
- 2026-09-30 (wk-childenv): Implemented on `work/bug-0d9ac4` at `af5bbe617` (domain profiles own tool prefixes; Graph
  dispatch's contract denies other domains' tools; merged in batch 13) and on `work/bug-0d9ac4b` at `2ba58d8f4`
  (`effective_agent_contract` takes `project.default_domain` through `TaskDef::effective_domain`); cargo verification
  deferred to the batch check. bug-12153c is done, so the project default now applies.
