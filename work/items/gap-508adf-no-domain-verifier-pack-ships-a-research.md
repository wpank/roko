+++
id = "gap-508adf"
kind = "gap"
title = "No domain verifier pack ships: a research, ops or data task gets no workspace rungs and ends unverified unless the user writes [gates.packs.<domain>] by hand"
status = "open"
triage = "verified"
severity = "p3"
goal = "features"
size = "M"
subsystem = ["roko-cli/pack_rungs", "roko-core/config"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
source = "status-quo-2026-10-15#finding-39"
anchors = ["crates/roko-cli/src/graph_task_dispatch/pack_rungs.rs::domain_profile", "crates/roko-core/src/config/loader.rs", "crates/roko-cli/src/commands/tune.rs::cmd_config_preset"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["dec-55532d", "gap-1625c0"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn shipped_research_pack_gives_a_research_task_rungs' crates/roko-cli/ && cargo test -p roko-cli shipped_research_pack_gives_a_research_task_rungs"
+++

## Problem

The verifier-pack machinery is complete: packs keyed by `TaskDomain` in `[gates.packs.<domain>]` (9119, 9120), domain
profiles in `[profiles.<domain>]` (9125, `pack_rungs::domain_profile`), and rung kinds for research, ops and data.
But nothing ships a pack: no preset, no `roko init` template and no shipped config defines `[gates.packs.<domain>]`
or a domain profile; every `packs.insert` outside tests is the schema sentinel at
`crates/roko-core/src/config/loader.rs:1879`. Since PK74 (9120), a non-code task with no pack no longer runs
`[[gates.rungs]]`, so a research, ops or data task gets no workspace rungs and ends `unverified` unless the user writes
a pack. Decision 9109 (answered, option a) set the order: research first, then ops, then data. Status-quo assessment
2026-10-15 at 7b6a5af44, finding 39 (`tmp/status-quo/2026-10-15/backlog/A10-91-92-domains-parked.md` lines 125-129 and
row 9109).

## Why it matters

Goal `features`: roko is meant to be domain-agnostic (Roko behind Hermes runs research and ops tasks), and "done means
checked" doesn't hold for a domain with no checks. Will's 2026-10-15 decision 3 confirmed dec-55532d's PK74 choice
as shipped, which makes a shipped pack the way to get those tasks checked.

## Where

- `crates/roko-cli/src/graph_task_dispatch/pack_rungs.rs` (`domain_profile`, the rung kinds and `is_built`).
- `crates/roko-cli/src/commands/tune.rs::cmd_config_preset` (`roko config preset …`) or the `roko init` template.
- `crates/roko-core/src/config/loader.rs` (pack loading).

## Current state

The rung kinds for research were built first (A10). gap-1625c0 notes that `roko init` writes no gate rungs for a
Python project either.

## Plan

1. Ship a research pack as data: a `[gates.packs.research]` rung list (the built research rungs, for example citation
   and source checks) and a `[profiles.research]` profile, applied by `roko config preset gates research` and offered
   by `roko init`.
2. Test `shipped_research_pack_gives_a_research_task_rungs`: with only the shipped preset applied, a
   `TaskDomain::Research` task resolves a non-empty rung list from the pack.
3. Ops and data packs follow in the same shape (decision 9109's order); file them when research lands.

## Done when

The `[[verify]]` command passes, and `roko config preset gates research --dry-run` prints the pack.

## Notes

Don't change the PK74 rule (a non-code task with no pack runs no code rungs). Receipt rungs are not built
(`pack_rungs::is_built` leaves out `RungKind::Receipt`), so the shipped pack must not require one.
