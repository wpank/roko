+++
id = "gap-33828a"
kind = "gap"
title = "roko serve cannot list terminal-started runs with origin, times and cost, so a host's Runs list and today's spend leave them out"
status = "open"
triage = "unverified"
severity = "p2"
goal = "hermes"
size = "M"
subsystem = ["roko-serve/routes"]
created = 2026-10-17
updated = 2026-10-17
source = "round v6 plugin-runs (P-3) after dogfood-will's Run 4"
anchors = ["crates/roko-serve/src/routes/runs.rs::get_dashboard_runs", "crates/roko-runtime/src/run_registry.rs::RunManifest"]
links = { depends_on = ["bug-c872f6"], blocks = [], related = ["bug-c872f6"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn run_list_returns_terminal_runs_with_origin_times_and_cost' crates/roko-serve/src && cargo test -p roko-serve --lib -- run_list_returns_terminal_runs_with_origin_times_and_cost"
+++

## Problem

A host such as the Hermes plugin cannot list the runs started from a terminal (`roko run`, `roko plan run`) or show
their spend. roko serve's run index holds a terminal run only under its event id, which differs from its record id
(bug-c872f6), with no start time, end time or cost. Getting each one would take a summary call, and its spend cannot
be joined to its records. The run manifest has no `origin` field either, so a host cannot tell a terminal run from one
it started.

Seen in round v6 (plugin-runs, P-3, after dogfood-will's Run 4): the plugin's Runs list showed none of Run 4's
terminal runs, and its "today" counter read $0.20 while roko's own daily ledger (`.roko/learn/costs.jsonl`) held $0.56.
The plugin now labels its counters `from: "hermes"` and reads the ledger when the workdir is shared. That is a
workaround: a remote plugin has no way to read the ledger.

## Why it matters

The pitch is a harness people use from many places: a terminal, an editor (ACP), Hermes. A run that one surface
cannot see makes every other surface's history and spend incomplete, and a spend counter that leaves out real spend
undercuts the cost story.

## Where

- `crates/roko-serve/src/routes/runs.rs::get_dashboard_runs`: the existing run listing.
- `crates/roko-runtime/src/run_registry.rs::RunManifest`: no `origin`.
- bug-c872f6: a terminal plan run gets two unrelated ids.

## Current state

Open. The plugin works around it (labels plus the local ledger), see `tmp/hermes-roko-v6/findings/plugin-runs.md`.

## Plan

1. Fix bug-c872f6 first (one id per run).
2. Record `origin` (`cli`, `serve`, `acp`, `mcp`) in every run's manifest.
3. A run list route, e.g. `GET /api/runs?since=<iso>&origin=<o>`: id, origin, plan, start and end, state, cost,
   verified tasks; bounded and paged.
4. Optionally `GET /api/costs/today` over the ledger the daily limit uses, for remote hosts.

## Done when

`[[verify]]` passes: the run list returns a terminal-started run with origin `cli`, its start and end and its cost.
