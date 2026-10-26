+++
id = "bug-5f9fa4"
kind = "bug"
title = "A Claude CLI agent prices its usage at the snapshot of its working directory, so a call in a scratch directory warns 'no price snapshot' and grows a process-wide cache"
status = "open"
triage = "unverified"
severity = "p3"
goal = "core"
size = "S"
subsystem = ["roko-agent/claude_cli"]
created = 2026-10-26
updated = 2026-10-26
source = "bug-ee2752 (worker, 2026-10-26): helper calls moved to scratch directories"
discovered_from = "bug-ee2752"
anchors = ["crates/roko-agent/src/claude_cli_agent.rs::priced_observation", "crates/roko-agent/src/provider/claude_cli.rs", "crates/roko-core/src/pricing_snapshot.rs::shared"]
links = { depends_on = [], blocks = [], related = ["bug-ee2752"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn claude_cli_prices_from_the_workspace_not_its_working_directory' crates/roko-agent/src/ && cargo test -p roko-agent claude_cli_prices_from_the_workspace_not_its_working_directory"
+++

## Problem

`ClaudeCliAgent::priced_observation` (`crates/roko-agent/src/claude_cli_agent.rs`) prices a call's usage
observation at `PriceSnapshot::shared(&self.pricing, &self.current_dir)`: the snapshot of the agent's working
directory, not of the workspace. Some calls run in a scratch directory on purpose: the audit's B1 suite author
(`crates/roko-cli/src/audit/b1.rs::FactoryAuthor`) and, since bug-ee2752, every helper call
(`CheapFactoryAgent`: error diagnosis, LLM judge, pack-rung judge, B3 reviewer). For such a call:

- with `[pricing] snapshot = "prices-<date>"` naming a snapshot that is only a file in the workspace's
  `config/prices/` (not the built-in id), `PriceSnapshot::resolve` finds no file in the scratch directory, and
  `shared` logs `no price snapshot: the costs it would price stay unknown` with the scratch path, once per call;
  the observation carries no snapshot price;
- without a pin, the scratch directory has no `config/prices/`, so the call is priced at the built-in copy, even
  when the workspace has a newer snapshot;
- every distinct directory adds an entry, never evicted, to `shared`'s process-wide cache (a parsed snapshot,
  or `None`), so a long-lived `roko serve` grows by one entry per helper call (and per attempt worktree).

The costs recorded for these calls stay right: the Graph dispatch prices them at the run's snapshot
(`dispatch_v2::api_equiv` falls back to the tokens when the observation's snapshot id differs, and
`pricing_snapshot_for` uses the request's `immune_root`). What is wrong is the misleading warning, the
observation's own price, and the cache growth.

## Why it matters

Cost telemetry is one of roko's measured-trust claims; a warning that says costs "stay unknown" when they are
priced misleads whoever reads the log, and a process-lifetime cache keyed by throwaway paths is a slow leak in
the server.

## Where

- `crates/roko-agent/src/claude_cli_agent.rs::priced_observation` (the `current_dir` root).
- `crates/roko-agent/src/provider/claude_cli.rs` (the adapter that builds the agent; `options.effective_immune_root()`
  names the workspace a Graph dispatch serves).
- `crates/roko-core/src/pricing_snapshot.rs::PriceSnapshot::shared` (the cache).

## Plan

Give `ClaudeCliAgent` a pricing root: the workspace (`AgentOptions::immune_root` when set, else the working
directory), set by the adapter, and price at `PriceSnapshot::shared(&self.pricing, &pricing_root)`. It is the only
agent that prices from its working directory (`grep -rn 'PriceSnapshot::shared(' crates/`; every other caller passes
the workspace). Add a unit test: a fake Claude CLI run in a scratch
directory with the workspace as its immune root, under a `[pricing] snapshot` pin that only the workspace has,
prices its observation at that snapshot and logs no warning.

## Done when

The unit test passes; a helper call in a scratch directory under a pinned workspace snapshot logs no
`no price snapshot` warning.
