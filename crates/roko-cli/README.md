# roko-cli

The `roko` binary: the command line for everything roko does. It runs prompts and plans, shows runs
as they go, reports what roko has learned, manages providers and models, and serves the HTTP control
plane and portal.

## Install

From a clone of this workspace:

```bash
cargo install --path crates/roko-cli                   # release build into ~/.cargo/bin/roko
cargo install --path crates/roko-cli --debug --force   # a debug build, quicker to compile
```

The binary is named `roko`, not `roko-cli`. While you work on the CLI itself, a shell function that
always runs the latest source saves reinstalling after every change:

```zsh
roko() {
  cargo run --quiet --manifest-path ~/path/to/roko/Cargo.toml -p roko-cli -- "$@"
}
```

## Commands

The main ones follow. `roko help <command>` documents each command, and
[docs/v2/CLI-REFERENCE.md](../../docs/v2/CLI-REFERENCE.md) has every flag.

### Work

```
roko init [path]                           # create .roko/ and a roko.toml for the project
roko run "<prompt>"                        # size the prompt: one checked task, or a plan written first
roko run --plan "<prompt>"                 # always write a plan first, show it, run it once you confirm
roko run --plan --dry-run "<prompt>"       # write the plan and stop
roko plan generate "<prompt>"              # write plans/<slug>/ (tasks.toml and plan.md); runs nothing
roko plan validate plans/<slug>            # lint a plan without running it
roko run plans/<slug>                      # run a plan directory (the same run as `roko plan run`)
roko plan run plans/<slug> --resume-plan   # resume after an interruption
roko diagnose <plan-id>                    # why a plan failed, task by task
```

### Watch

```
roko plan status plans/<slug>              # a plan's last run: each task, the plan check, where the work went
roko status                                # what is running, how the last runs ended, what they spent
roko dashboard                             # the full-screen terminal dashboard
roko serve                                 # the HTTP control plane and portal on 127.0.0.1:6677
```

### Learn and inspect

```
roko learn all                             # what roko has learned in this workspace
roko knowledge query "<topic>"             # search the durable knowledge store
roko replay <hash>                         # walk the signal log's lineage from one signal
roko config providers list                 # the configured providers
roko doctor                                # check the workspace's setup
```

Plain `roko` in a terminal opens an interactive chat, and `roko "<prompt>"` sends one turn and
prints the reply, with no checks; use `roko run` for a change you want checked. `roko agent chat
--agent <name>` talks to a running agent.

## Dashboard

`roko dashboard` opens a ratatui TUI with ten tabs on `F1`–`F10` (or the number keys): Dashboard,
Plans, Agents, Git, Logs, Config, Inspect, Marketplace, Learning and Providers. `?` lists the keys.

## Configuration

`roko` reads the nearest `roko.toml` at or above the working directory, layered over the global
`~/.roko/config.toml`, and `ROKO_CONFIG=<file>` replaces the project file. The main README's
[Configuration](../../README.md#configuration) section has a minimal `roko.toml`.

## Exit codes

`roko run` exits non-zero when the run fails, so it works as a check in a pipeline.

## Testing

```bash
cargo test -p roko-cli
cargo clippy -p roko-cli --all-targets -- -D warnings
```

## SWE-bench scripts

[`scripts/`](scripts/README.md) holds driver scripts for SWE-bench runs, described in their own
README.
