+++
id = "bug-4ed3c2"
kind = "bug"
title = "roko plan run ignores --config and loads the workspace roko.toml; only ROKO_CONFIG selects another config"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
size = "S"
subsystem = ["roko-cli/plan-run", "roko-cli/graph-execution"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d5c1dc6be"
source = "session:roko-b6 2026-09-29 portal close-out"
discovered_from = "session:roko-b6 2026-09-29 portal close-out"
anchors = ["crates/roko-cli/src/commands/plan.rs::cmd_plan_run_engine", "crates/roko-cli/src/graph_execution/plan_runner.rs::run_graph_plan", "crates/roko-cli/src/graph_execution/plan_runner.rs::GraphPlanRunParams", "crates/roko-core/src/config/loader.rs::find_config_path", "crates/roko-cli/src/main.rs::resolve_config_for_workdir"]
links = { depends_on = [], blocks = [], related = ["gap-6bc156"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn plan_run_uses_the_config_flag' crates/roko-cli/ && cargo test -p roko-cli plan_run_uses_the_config_flag"
+++

## Problem

`--config <file>` is a global flag ("Override the config file (default: `./roko.toml`)", `main.rs:336-338`), but
`roko plan run` never reads it. The run loads whatever `roko.toml` the workspace resolves to, with no warning. The only
way to run a plan under another config is the `ROKO_CONFIG` environment variable.

Expected: `roko --config other.toml plan run plans/` uses `other.toml` for providers, models, budget, gates and
runner settings, or refuses the flag. Actual: the flag is dropped silently and the run uses the workspace config.

## Why it matters

Goal `core`. A run the operator believes is bound by one config (a cheaper model set, a lower budget, stricter gates)
runs under another, and nothing says so. Evidence and cost figures from such a run are attributed to the wrong
settings.

## Where

- `crates/roko-cli/src/commands/plan.rs`: the `PlanCmd::Run` arm of `cmd_plan` (:462-676) never reads `cli.config`.
  `cmd_plan_run_engine` (:2518) loads `load_config_unified(workdir)` for `live_agent_output` (:2551) and builds
  `GraphPlanRunParams`, which has no config-path field (`graph_execution/plan_runner.rs:642`).
- `crates/roko-cli/src/graph_execution/plan_runner.rs::run_graph_plan` (:701) loads the runtime config with
  `load_config_validated(workdir)` (:824).
- `crates/roko-core/src/config/loader.rs::find_config_path` (:1888): `ROKO_CONFIG`, else the ancestor walk from
  the workdir. Both loaders above go through it.
- `warn_graph_unsupported_flags` (`commands/plan.rs:2439`) warns about other dropped flags, not this one.
- Commands that do honour the flag: `main.rs::resolve_config_for_workdir` (:4268), `commands/github.rs:31`, setup,
  doctor and the unified chat (`main.rs:3647`).

## Current state

Checked at `d5c1dc6be` by reading the code. `grep cli.config crates/roko-cli/src/commands/plan.rs` finds nothing.
When a live `roko serve` owns the workspace, `plan run` is forwarded to it (`commands/plan.rs:569-592`), and the
server's own config applies.

## Plan

1. Add `config_path: Option<PathBuf>` to `GraphPlanRunParams`. `cmd_plan_run_engine` sets it from `cli.config`.
2. In `run_graph_plan` and `cmd_plan_run_engine`, load that file when it is set: `load_config_file(path, …)`, or a
   validated equivalent of `load_config_validated` for an explicit path. Otherwise keep the current discovery.
   Fail if the file does not exist.
3. When the run is forwarded to a live server, refuse `--config` with a clear message, since the server's config
   applies.
4. Alternative: set `ROKO_CONFIG` from `--config` once in `main`. This is smaller, but it leaks into every child
   process, agents and verify commands included. Prefer explicit threading.
5. Add `plan_run_uses_the_config_flag`: a mock-agent run with `--config` pointing at a second file, asserting that a
   setting only that file has takes effect (for example a model or a budget visible in the checkpoint or the output),
   or that a missing `--config` file fails the run.

## Done when

- `roko --config <file> plan run <dir>` runs under `<file>`, and a missing file is an error.
- The `[[verify]]` command passes.

## Notes

- The checkpoint fingerprint includes the config, so resuming under a different `--config` will not match the
  checkpoint. That is expected.
- Other subcommands (`plan generate`, `prd plan`, …) were not audited for the same problem.
- gap-6bc156 (inert `max_concurrent_plans` keys) is a related config-trust problem in the same runner.
