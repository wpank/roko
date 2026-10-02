# Roko CLI Reference

## What is Roko?

Roko is a Rust toolkit for building agents that build themselves. You describe a piece of
work — in plain English, or as a structured plan — and Roko orchestrates a fleet of LLM
agents to implement it, validate the result with a gate pipeline (compile, test, lint,
diff), learn from every run, and persist everything so it can resume if interrupted. The
core idea is that Roko uses itself: the same pipeline that implements your features is the
pipeline that implements Roko's own features.

The CLI is the primary control surface. You use it to turn prompts into implementation
plans, review and edit those plans, kick off execution, watch progress, and inspect what
the system learned. Once an execution is running, the HTTP control plane (`roko serve`) and
interactive TUI dashboard (`roko dashboard`) give you live visibility without touching the
terminal.

---

## Table of contents

1. [Installation and first run](#installation-and-first-run)
2. [The 5-minute tour](#the-5-minute-tour)
3. [Core workflow: prompt to shipped code](#core-workflow-prompt-to-shipped-code)
4. [Global flags](#global-flags)
5. [Exit codes](#exit-codes)
6. [Core workflow commands](#core-workflow-commands)
   - [roko init](#roko-init)
   - [roko run](#roko-run)
   - [roko status](#roko-status)
   - [roko doctor](#roko-doctor)
7. [Planning](#planning)
   - [roko plan](#roko-plan)
   - [Removed commands](#removed-commands)
8. [Interactive dashboard](#interactive-dashboard)
9. [Agents](#agents)
10. [Research](#research)
11. [Knowledge](#knowledge)
12. [Learning and feedback](#learning-and-feedback)
13. [Jobs](#jobs)
14. [Benchmarks](#benchmarks)
15. [Configuration](#configuration)
16. [Code intelligence](#code-intelligence)
17. [GitHub integration](#github-integration)
18. [Server and deployment](#server-and-deployment)
19. [Authentication](#authentication)
20. [Utilities](#utilities)
21. [Vision loop](#vision-loop)
22. [Environment variables](#environment-variables)
23. [Config file locations and precedence](#config-file-locations-and-precedence)
24. [Data directory layout](#data-directory-layout)
25. [Troubleshooting](#troubleshooting)
26. [Build requirements](#build-requirements)

---

## Installation and first run

Roko requires Rust 1.91 or later (needed for `alloy` dependencies).

```bash
rustup update stable
cargo build --workspace
```

Once built, initialize a workspace in your project directory:

```bash
cd /your/project
roko init
```

This creates a `.roko/` directory and a `roko.toml` config file. After that, run the
configuration wizard to connect to your LLM backend:

```bash
roko config init
```

Verify everything is wired up:

```bash
roko doctor
```

---

## The 5-minute tour

Here is the fastest path to seeing Roko do something real.

```bash
# Step 1: Initialize a workspace (if you haven't already)
roko init

# Step 2: Run a prompt as checked work
roko run "Add input validation to the login form"

# Step 3: Check what happened
roko status

# Step 4: Open the interactive dashboard for a live view
roko dashboard
```

`roko run` is the entry point for work. It sizes the prompt: a small change runs as one agent
task checked by the gates, and a larger one gets a plan written first
(`plans/<slug>/tasks.toml`), which then runs. For bigger work you want to see first, use
`roko run --plan "<prompt>"`: it always writes a plan, shows it, and asks before running it.
`roko run --plan --dry-run "<prompt>"` writes the plan and stops, so you can review and edit
it, then run it with `roko run plans/<slug>`.

A prompt without a subcommand is a different mode:

```bash
roko "explain what src/auth.rs does"
```

It sends a single turn to the agent, with its tools, and prints the reply. It runs no gates,
so use `roko run` for a change you want checked. When invoked with no arguments and stdin is
a TTY, Roko launches the unified chat REPL.

---

## Core workflow: prompt to shipped code

The full self-hosting workflow looks like this. Each box is a CLI command.

```
 Write a plan                    (agent writes plans/<slug>/: tasks.toml + plan.md)
 ────────────
 roko plan generate "<prompt>"
        │
        ▼
 Enrich with research            (optional)
 ─────────────────────
 roko research enhance-plan <slug>
        │
        ▼
 Review and edit                 (the plan is plain files; change what you like)
 ───────────────
 $EDITOR plans/<slug>/tasks.toml
        │
        ▼
 Validate the plan               (lint tasks.toml without running)
 ──────────────────
 roko plan validate plans/<slug>
        │
        ▼
 Execute the plan                (agents run tasks in parallel, gates validate)
 ─────────────────
 roko run plans/<slug>
        │
        ├──► roko dashboard       (watch live in the TUI)
        │
        ▼
 Inspect results
 ───────────────
 roko status
 roko learn all
 roko knowledge query "<topic>"
        │
        ▼
 Consolidate learning            (offline distillation into knowledge store)
 ────────────────────
 roko knowledge dream run
```

`roko run --plan "<prompt>"` does the first and the execute step in one command: it writes
the plan, shows it on a terminal, and asks before running it (`--yes` skips the question).

You do not need all of these steps for every task. For a quick one-off fix, `roko run` is
enough. For a large feature with multiple sub-tasks and acceptance criteria, the plan-first
workflow gives you a plan you can review before anything runs, reproducibility,
resume-on-interrupt, and automatic replan-on-gate-failure.

---

## Global flags

These flags are `global = true` and can be placed before any subcommand.

| Flag | Type | Default | Description |
|---|---|---|---|
| `--config <path>` | path | `./roko.toml` | Override the config file location. |
| `--role <string>` | string | from config | Set the agent role / persona. |
| `--model <string>`, `--force-model <string>` | string | from config | Force the model for this invocation; explicit selection has highest routing precedence. |
| `--repo <path>` | path | cwd | Set the repository / working directory root. |
| `--resume <id>` | string | — | Resume a previous session by ID. |
| `--effort low\|medium\|high\|max` | enum | from config | Reasoning effort level passed to the agent backend. |
| `--json` | flag | false | Emit JSON output instead of human-readable text. Supported by most commands. |
| `--log-format text\|json` | enum | `text` | Tracing log format. |
| `--quiet` | flag | false | Suppress non-essential output. |
| `--no-replan` | flag | false | Disable re-planning; gate failures become terminal failures. |
| `--headless` | flag | false | Run as a headless daemon (background service). |
| `--color auto\|always\|never` | enum | `auto` | Control ANSI color output. |
| `--timing` | flag | false | Print elapsed time after command execution. Also enabled by `ROKO_TIMING=1`. |
| `--no-serve` | flag | false | Do not start the HTTP control plane in the background. |

<details>
<summary>Color resolution details (auto mode precedence)</summary>

Precedence (highest first):

1. `NO_COLOR` set and non-empty → off
2. `CLICOLOR_FORCE` set and not `"0"` → on
3. `CLICOLOR=0` → off
4. stdout is a TTY → on
5. otherwise → off

</details>

<details>
<summary>Effort level descriptions</summary>

| Value | Description |
|---|---|
| `low` | Minimal reasoning — fast, cheap. |
| `medium` | Balanced reasoning (default). |
| `high` | Thorough reasoning. |
| `max` | Maximum reasoning — slowest, most expensive. |

</details>

---

## Exit codes

| Code | Constant | Meaning |
|---|---|---|
| `0` | `EXIT_SUCCESS` | Successful execution. |
| `1` | `EXIT_AGENT_FAILURE` | Agent or gate failure (logical error in the build). |
| `2` | `EXIT_SYSTEM_ERROR` | System error (I/O, config, infrastructure). |

---

## Core workflow commands

### `roko init`

**When to use this:** The very first thing you do in a new project. Creates the `.roko/`
workspace directory and a `roko.toml` config file.

```
roko init [path] [--cloud] [--profile <name>] [--demo]
```

| Flag | Description |
|---|---|
| `path` | Directory to initialize (default: current dir). |
| `--cloud` | Generate cloud-ready defaults for deployment. |
| `--profile <name>` | Project profile: `rust`, `typescript`, `go`, `python`, `general`. |
| `--demo` | Seed realistic demo data after initialization. |

<details>
<summary>Examples</summary>

```bash
roko init                          # Initialize in the current directory
roko init /path/to/project         # Initialize in a specific directory
roko init --cloud                  # Initialize with cloud-ready defaults
roko init --profile rust           # Initialize with Rust project profile
roko init --demo                   # Initialize and seed demo data
```

</details>

---

### `roko run`

**When to use this:** For any piece of work, from a one-line fix to a feature with many
tasks, and to run a plan directory you already have. It sizes a prompt and runs it as one
checked task or as a plan, so you do not have to pick the path up front.

```
roko run <prompt> [--plan] [--dry-run] [--yes] [--complexity <level>]
                  [--context <path>]... [--no-cascade] [--provider <name>]
                  [--workdir <path>] [--max-retries <n>] [--serve] [--share]
roko run <plan-dir> [--fresh] [--resume-plan [<path>]]
```

| Form | What it does |
|---|---|
| `roko run "<prompt>"` | Size the prompt. A small change runs as one checked task; a larger one gets a plan written first (`plans/<slug>/tasks.toml`), which then runs. |
| `roko run --plan "<prompt>"` | Always write a plan first. On a terminal roko shows the plan and asks before running it; `--yes` skips the question. |
| `roko run --plan --dry-run "<prompt>"` | Write the plan and stop. Review or edit `plans/<slug>/`, then run it with `roko run plans/<slug>`. |
| `roko run --dry-run "<prompt>"` | Show how the prompt would run (size, route, gates). Runs nothing. |
| `roko run plans/<slug>` | Run an existing plan directory; the same run as `roko plan run plans/<slug>`. |
| `roko run plans/` | Run every plan under `plans/`. |

| Arg/Flag | Default | Description |
|---|---|---|
| `<prompt>` or `<plan-dir>` | required | The prompt text, or a plan directory to run. |
| `--plan` | false | Always write a plan first, then run it. |
| `--dry-run` | false | With `--plan`, write the plan and stop. Without it, show how the prompt would run. |
| `--yes` | false | With `--plan`, run the plan without asking first. |
| `--complexity <level>` | sized | Force the size: `trivial` or `simple` run one task; `standard` or `complex` write a plan. |
| `--context <path>` | — | Extra context for the planner (repeatable). |
| `--no-cascade` | false | Disable cascade routing for the run. |
| `--provider <name>` | config | Override the provider for this run. |
| `--workdir <path>` | cwd | Override the working directory. |
| `--max-retries <n>` | config | Retries after a failed attempt. |
| `--serve` | false | Start the HTTP control plane alongside the run (one-task runs only). |
| `--share` | false | Generate a shareable URL, starting serve if needed (one-task runs only). |
| `--fresh` | false | Plan directory only: archive the old run state and start from scratch. |
| `--resume-plan [<path>]` | — | Plan directory only: resume from the Graph checkpoints under `.roko/state/graph/`. |

The global flags (`--model`, `--role`, `--json`, `--effort`, `--quiet`) apply as usual.
`roko run` does not classify intent: a question is not routed to research. Use
`roko think "<question>"` or `roko research topic "<topic>"` for questions. For the full set
of plan-run options (`--max-tasks`, `--budget-override`, `--approval` and the rest), run the
plan with `roko plan run`.

<details>
<summary>Examples</summary>

```bash
roko run "Fix the login bug"
roko run "Refactor db layer" --role architect
roko run --plan "Add OAuth2 login"                 # write a plan, review it, run it
roko run --plan --dry-run "Add OAuth2 login"       # write plans/add-oauth2-login/ and stop
roko run plans/add-oauth2-login                    # run the reviewed plan
roko run plans/add-oauth2-login --resume-plan      # resume an interrupted run
roko run "Deploy to staging" --serve
```

</details>

---

### `roko status`

**When to use this:** After a run completes, or any time you want a quick health summary of
the workspace — signal counts, most recent episode, gate pass/fail ratios.

```
roko status [--workdir <path>] [--cfactor] [--surfaces]
```

| Flag | Description |
|---|---|
| `--workdir <path>` | Directory containing `.roko/` (default: cwd). |
| `--cfactor` | Compute and persist the latest C-Factor snapshot. |
| `--surfaces` | Print the CLI/TUI/backend surface inventory instead of session status. |

<details>
<summary>Examples</summary>

```bash
roko status                        # Show workspace health summary
roko status --json                 # Output status as JSON for scripting
roko status --cfactor              # Compute and show C-Factor metrics
```

</details>

---

### `roko doctor`

**When to use this:** When something is not working and you want a diagnostic report. Checks
for `.roko/`, `roko.toml`, agent command availability, secrets, and optionally the HTTP
control plane.

```
roko doctor [--workdir <path>] [--serve-url <url>]
```

| Flag | Description |
|---|---|
| `--workdir <path>` | Directory containing `roko.toml` and `.roko/` (default: cwd). |
| `--serve-url <url>` | roko-serve base URL or health endpoint to probe. |

---

### `roko layer-check`

Check workspace layer dependency rules — ensures crate imports follow the layered
architecture. Run this before committing changes to `Cargo.toml` files.

```
roko layer-check
```

---

## Planning

### `roko plan`

**When to use this:** Plans are the unit of work. `roko plan generate` (or
`roko run --plan --dry-run`) writes a plan directory from a prompt, and `roko run plans/<slug>`
or `roko plan run` executes it. Use the other `plan` subcommands to inspect, validate,
hand-author and control plans.

#### `roko plan list`

List all plans discovered in the workspace.

```
roko plan list [--workdir <path>]
```

Output includes task count, completion progress, and any persisted run state from
`.roko/state/state-snapshot.json`. Supports `--json`.

#### `roko plan show`

Show details of a specific plan.

```
roko plan show <plan-id> [--workdir <path>]
```

#### `roko plan create`

Create a new plan skeleton.

```
roko plan create <plan-id> --title <title> [--description <text>] [--workdir <path>]
```

#### `roko plan validate`

Lint every `tasks.toml` under a plans directory without executing. Run this before `roko
plan run` to catch schema errors and missing dependency references.

```
roko plan validate [<dir>] [--strict] [--json] [--dag] [--spec-quality]
```

| Arg/Flag | Default | Description |
|---|---|---|
| `<dir>` | `plans/` | Plans root directory, or one plan directory. |
| `--strict` | false | Fail on warnings, not only errors. |
| `--json` | false | Output machine-readable JSON. |
| `--dag` | false | Show DAG analysis: plan, task and edge counts, waves, critical path, dangling dependency references. |
| `--spec-quality` | false | Score each task's spec with the speclint rules; with `--strict`, a hard fail exits 1. |

#### `roko plan run`

**The primary execution command**, with every plan-run option; `roko run plans/<slug>` is the
same run. The Graph engine executes tasks through the complete
agent/gate/replan/worktree/merge/persistence lifecycle. The legacy Runner-v2 engine was removed:
`--engine legacy` (alias `runner-v2`) is still parsed but exits with an error.

```
roko plan run <plans-dir> [--engine graph] [--workdir <path>]
             [--resume-plan [<snapshot>]] [--approval]
             [--max-retries <n>] [--max-tasks <n>] [--dry-run]
             [--fresh] [--force-resume] [--budget-override <usd>] [--no-budget]
```

<details>
<summary>Full flag table</summary>

| Arg/Flag | Default | Description |
|---|---|---|
| `<plans-dir>` | required | Path to the plans directory. |
| `--engine <engine>` | `graph` | `graph` is the only engine. `legacy` (alias `runner-v2`) is rejected with an error. |
| `--workdir <path>` | cwd | Working directory (repo root). |
| `--resume-plan [<path>]` | — | Graph accepts a checkpoint root (one directory per plan) or a manifest file for a single-plan run; its default is `.roko/state/graph/`. Alias: `--resume-state`. |
| `--approval` | false | Launch the connected approval TUI while the plan runs. |
| `--max-retries <n>` | from config | Maximum retry attempts per task (overrides per-task and config values). |
| `--max-tasks <n>` | config/default | Maximum concurrent tasks per plan; `0` keeps configured behavior. |
| `--dry-run` | false | Parse and display the plan without executing. Shows tasks, dependencies, and estimates. |
| `--fresh` | false | Archive existing state and start from scratch. |
| `--force-resume` | false | Graph archives a mismatched fingerprint and starts a new run. |
| `--budget-override <usd>` | config | Override the per-plan cost ceiling; once the plan has spent it, no further task starts. `0` removes the plan ceiling; the per-task and daily ceilings still apply. |
| `--no-budget` | false | Turn off budget enforcement: no plan, per-task or daily ceiling stops a dispatch. |

</details>

The Graph engine honors `--resume-plan`, `--fresh`, `--force-resume`, `--max-retries`,
`--max-tasks`, `--budget-override`, and `--no-budget`. Its checkpoint manifest binds schema
version, plan ID, converted-graph fingerprint, run ID, and Activity log; drift fails closed
unless force-resume archives the old state and starts clean. Graph records actual provider
cost from both successful and unsuccessful paid calls, routes against the tighter Cell or
plan remainder, blocks later Activities at a configured ceiling, fails a hard-limit run if
the last call crosses the ceiling, and reports per-plan and total spend.

<details>
<summary>How Graph plan run works (internal execution flow)</summary>

1. Plans are loaded from `<plans-dir>`. Each plan is a directory containing `tasks.toml`.
2. `ProductionPlanTopology` builds an 11-node subgraph per task (TaskContext + 6 enrichers + Compose + TaskExecutor + Gate + SuccessBoundary).
3. The `GraphEngine` executes nodes in bounded parallel topological waves.
4. Each task runs an agent via `TaskExecutorCell`, then runs gate validation via `GatePipelineCell` (using `CellResources.gates`).
5. Gate failures trigger the replan controller. Failure context drives revised task generation.
6. `run_one_plan` writes the checkpoint's terminal status, and on an interrupt stops in-flight agents.
7. The 12-row `FeedbackSettler` settles completion sinks with exactly-once idempotency.
8. Efficiency events, episodes, and C-factor metrics are written to `.roko/learn/`.

</details>

<details>
<summary>State persistence and resume</summary>

The Graph engine writes checksummed checkpoints to `.roko/state/graph/`. To resume:

```bash
roko plan run plans/ --resume-plan
```

</details>

<details>
<summary>Examples</summary>

```bash
roko plan run plans/                                       # Run all plans (Graph engine, default)
roko plan run plans/my-plan                                # Run one plan directory
roko plan run plans/ --approval                            # Connected approval TUI
roko plan run plans/ --dry-run                             # Preview without executing
roko plan run plans/ --fresh                               # Archive old state and start clean
roko plan run plans/ --resume-plan                         # Resume from last checkpoint
roko plan run plans/ --max-retries 3                       # Override retry limit
```

</details>

#### `roko plan generate`

Write a plan from a prompt. An agent reads the prompt and the codebase and writes
`plans/<slug>/` with a `tasks.toml` and a `plan.md`; nothing runs. The slug comes from the
prompt's words, or from the file's name with `--from-file`. The plan is validated before the
command exits, and a plan that fails validation exits 1. Review or edit the files, then run
the plan with `roko run plans/<slug>`.

```
roko plan generate <prompt...> [--from-file <path>] [--context <path>]...
                   [--from-notes [--tag <tag>]] [--from-backlog <ids>]
```

| Arg/Flag | Description |
|---|---|
| `<prompt...>` | Free-text description of the work. |
| `--from-file <path>` | Read the request from a file instead of inline text. |
| `--context <path>` | Additional context files, directories or globs for the planner's prompt (repeatable). |
| `--from-notes` | Read notes from `.roko/notes/`, cluster them, and write one plan per cluster. |
| `--tag <tag>` | With `--from-notes`, use only notes with this tag. |
| `--from-backlog <ids>` | Write plans from `tmp/backlog/<id>-*.md` specs (one ID or comma-separated IDs), with deterministic slugs. |

The planner model is `[authoring] planner_model`; the global `--model` overrides it.
`roko plan "<prompt>"` is shorthand for `roko plan generate "<prompt>"`.

```bash
roko plan generate "Add OAuth2 login"            # writes plans/add-oauth2-login/
roko plan generate --from-file oauth2-login.md --context src/auth/
roko plan generate --from-backlog 206,120
```

#### `roko plan regenerate`

Regenerate an existing plan's `tasks.toml` from its source document with full metadata.
Tasks keep their ids where they still apply, and tasks already done stay done.

```
roko plan regenerate <plan-dir> [--dry-run]
```

### Removed commands

Plans come straight from a prompt, so the PRD pipeline and the extra entry points are gone.
For one release `roko prd` and `roko do` still parse, print an error that names the
replacement, and exit 1; `roko develop` is an unknown command.

| Removed | Use instead |
|---|---|
| `roko prd idea`, `list`, `status`, `draft new/edit/promote/list`, `plan`, `consolidate` | `roko run --plan "<prompt>"`, or `roko plan generate "<prompt>"` then `roko run plans/<slug>` |
| `roko do "<prompt>"` (alias `d`) | `roko run "<prompt>"`; `roko do --plan` becomes `roko run --plan` |
| `roko develop "<prompt>"` | `roko run --plan "<prompt>"` |
| `roko research enhance-prd <slug>` | `roko research enhance-plan <slug>` |
| `roko backlog import` | `roko plan generate --from-backlog <ids>` |

---

## Interactive dashboard

**When to use this:** While a plan is running, or any time you want a live view of what
the agents are doing, what gates are passing or failing, what is being learned, and the
state of your git working tree.

### `roko dashboard`

Launch the interactive ratatui TUI dashboard.

```
roko dashboard [--page <slug>] [--list-pages] [--text] [--workdir <path>]
               [--high-contrast] [--reduced-motion]
```

| Flag | Description |
|---|---|
| `--page <slug>` | Specific dashboard page slug to render. |
| `--list-pages` | List all available page slugs and exit. |
| `--text` | Force text-mode output instead of the interactive terminal UI. |
| `--high-contrast` | Use high-contrast color scheme (WCAG 2.1 AA). |
| `--reduced-motion` | Disable animations for reduced-motion accessibility. |

The dashboard can also be launched embedded in the server process:

```bash
roko serve --tui   # Zero-copy, reads live state from StateHub, no file polling
```

<details>
<summary>TUI tab structure</summary>

| Tab | Key | Alt key | Content |
|---|---|---|---|
| Dashboard | F1 | `1` | Health gauges, plan progress, cost summary, C-factor |
| Plans | F2 | `2` | Plan tree, task progress, wave overview |
| Agents | F3 | `3` | Agent output, diffs, token burn, parallel pool |
| Git | F4 | `4` | Branch tree, commit graph, worktree list |
| Logs | F5 | `5` | Scrollable log viewer with level filtering |
| Config | F6 | `6` | Config editor / effective config view |
| Inspect | F7 | `7` | Signal DAG inspector, episode replay |
| Marketplace | F8 | `8` | Job browser, creation, assignment |
| Learning | F9 | `9` | Cascade router, model routing, efficiency metrics |
| Providers | F10 | `0` | Provider health, cost, latency, circuit-breaker state |

</details>

<details>
<summary>Global keybindings (all tabs)</summary>

| Key | Action |
|---|---|
| `F1`–`F10` | Switch tab |
| `1`–`9`, `0` | Switch tab (digit aliases) |
| `Alt+1`–`Alt+9` | Switch sub-view within current tab |
| `q` | Quit (shows quit confirm dialog) |
| `Ctrl+C` | Quit immediately |
| `?` | Show help modal |
| `Tab` | Focus next panel |
| `Shift+Tab` | Focus previous panel |
| `n` | Dismiss notification |
| `Ctrl+R` | Refresh |
| `Ctrl+A` | Approve all pending commands |
| `Ctrl+T` | Toggle agent topology panel |
| `Ctrl+X` | Force advance (with confirmation) |
| `Ctrl+D` | Cancel selected plan (with confirmation) |
| `Ctrl+E` | Toggle full-screen post-processing effects |
| `v` | Cycle visual effects preset |
| `Ctrl+G` | Reconcile git state (with confirmation) |
| `u` | Show queue overview |

</details>

<details>
<summary>Dashboard tab (F1) keybindings</summary>

| Key | Action |
|---|---|
| `↑` / `k` | Navigate up (focus-aware: plan tree or agent output) |
| `↓` / `j` | Navigate down (focus-aware) |
| `PgUp` / `PgDn` | Scroll page up/down |
| `Home` / `End` | Scroll to start/end |
| `Enter` | Show plan detail modal |
| `Esc` | Close plan detail |
| `←` / `h` | Drill out |
| `→` / `l` | Drill in |
| `Shift+←` | Previous wave |
| `Shift+→` | Next wave |
| `a` | Switch to Agents detail sub-tab |
| `o` | Switch to Output sub-tab |
| `d` | Switch to Diff sub-tab |
| `e` | Switch to Errors sub-tab |
| `g` | Switch to Git sub-tab |
| `m` | Switch to MCP sub-tab |
| `L` | Switch to Learning sub-tab |
| `P` | Switch to Processes sub-tab |
| `w` | Show wave overview |
| `p` | Toggle pause |
| `i` | Inject a directive (not available yet: shows a warning) |
| `y` | Approve pending command |
| `` ` `` | Cycle agent role tabs |

</details>

<details>
<summary>Plans tab (F2) keybindings</summary>

| Key | Action |
|---|---|
| `↑` / `k` | Navigate up |
| `↓` / `j` | Navigate down |
| `Enter` | Show plan detail modal |
| `Esc` | Close plan detail |
| `e` | Expand/collapse plan tree |
| `w` | Show wave overview |
| `o` | Show queue overview |
| `t` | Open task picker |
| `[` | Previous wave |
| `]` | Next wave |
| `←` / `h` | Drill out |
| `→` / `l` | Drill in |
| `PgUp` / `PgDn` | Scroll page up/down |
| `Home` / `End` | Scroll to start/end |
| `/` | Start filter mode |
| `d` | Diagnose plan (with confirmation) |
| `m` | Merge plan branch (with confirmation) |
| `M` | Merge all done branches (with confirmation) |
| `s` | Soft retry plan |
| `z` | Diagnose plan |
| `S` | Repair plan (preserve) |
| `R` | Reset plan: run it again from scratch (with confirmation) |
| `C` | Cancel plan (with confirmation) |
| `c` | Reverify plan |
| `F` | Force advance |
| `V` | Reverify plan |

During `roko plan run` these keys act on the run:

- pause (`p`) and cancel (`C`, `Ctrl+D`);
- for a plan that failed or was cancelled earlier in the same run, soft retry (`s`) and repair
  (`S`), which resume its checkpoint so the tasks that passed stay done, and reset (`R`), which
  archives its checkpoint and runs every task again. A plan it blocked waits for it again;
- `X` on the Agents tab, which stops the selected agent: its task fails as stopped by the
  operator, its dependants are skipped, and the plan's other tasks run on.

Re-verifying gates, force advance and approvals are rejected with a reason. Once the run has
ended, `roko plan run --resume-plan` re-runs the tasks that did not pass.

</details>

<details>
<summary>Logs tab (F5) keybindings</summary>

| Key | Action |
|---|---|
| `↑` / `k` | Scroll log up |
| `↓` / `j` | Scroll log down |
| `End` / `G` | Scroll to end (tail) |
| `I` | Toggle Info filter |
| `W` | Toggle Warn filter |
| `E` | Toggle Error filter |
| `D` | Toggle Debug filter |
| `A` | Show all log filter levels |
| `/` | Start filter mode |

</details>

<details>
<summary>Modal and dialog keybindings</summary>

**Inject mode** is not available in the TUI yet: `i` shows a warning instead of collecting a
directive. `roko inject` delivers one to a running plan.

**Filter mode** (entered via `/` in Plans or Logs tab):

| Key | Action |
|---|---|
| Any char | Append to filter buffer |
| `Backspace` | Delete last character |
| `Enter` | Accept filter |
| `Esc` | Cancel filter |

**Approval modal** (shown when agent requests tool approval):

| Key | Action |
|---|---|
| `y` / `Y` / `Enter` | Approve command |
| `n` / `N` / `Esc` | Reject command |
| `Ctrl+A` / `A` | Approve all pending |

**Confirm dialog**:

| Key | Action |
|---|---|
| `y` / `Y` / `Enter` | Confirm yes |
| `n` / `N` / `Esc` | Confirm no |

**Plan detail / Task detail modal**:

| Key | Action |
|---|---|
| `Esc` | Close modal |
| `↑` / `k` | Scroll up |
| `↓` / `j` | Scroll down |
| `Tab` | Switch detail sub-tab |
| `q` | Close (task detail only) |

**Task picker modal**:

| Key | Action |
|---|---|
| `Esc` | Close picker |
| `Enter` | Show task detail |
| `↑` / `k` | Navigate up |
| `↓` / `j` | Navigate down |

</details>

---

## Agents

**When to use this:** When you want to manage long-running named agents rather than
ephemeral per-task dispatch. Use `roko agent create` / `start` / `stop` for agents that
should persist across multiple tasks, or `roko agent serve` to start a per-agent HTTP
sidecar for integration with external tools. Use `roko agent chat` for interactive REPL
sessions with a specific agent.

### `roko agent create`

Create a new agent from a manifest. Generates an `AgentExtendedManifest` TOML at
`.roko/agents/<name>/manifest.toml`.

```
roko agent create --name <name> [--domain <domain>] [--template <template>]
                  [--prompt <text>] [--skills <skills>] [--tier <tier>]
                  [--reputation <n>] [--max-concurrent-jobs <n>]
                  [--serve-url <url>] [--workdir <path>]
```

<details>
<summary>Full flag table</summary>

| Flag | Default | Description |
|---|---|---|
| `--name <name>` | required | Human-readable agent name. |
| `--domain <domain>` | `general` | Agent domain: `coding`, `research`, `chain`, `general`. |
| `--template <template>` | — | Strategy template (e.g. `fast-coding`, `deep-research`). |
| `--prompt <text>` | — | Natural-language description of what the agent should do. |
| `--skills <skills>` | — | Comma-separated skill tags (e.g. `"rust,p2p,networking"`). |
| `--tier <tier>` | — | Agent tier: `Unverified`, `Verified`, `Trusted`, `Expert`, `Pioneer`. |
| `--reputation <n>` | `0` | Reputation score (0–100). |
| `--max-concurrent-jobs <n>` | `0` | Maximum concurrent jobs. |
| `--serve-url <url>` | — | Auto-register with roko-serve after creation. |
| `--workdir <path>` | cwd | Working directory. |

</details>

### `roko agent delete`

Delete an agent and clean up its state. Performs an 8-step ordered shutdown: stop
processing, flush pending, backup knowledge, deregister from mesh, release resources,
archive signals, clean state, emit deletion marker.

```
roko agent delete --name <name> [--force] [--workdir <path>]
```

| Flag | Description |
|---|---|
| `--name <name>` | Agent name to delete. |
| `--force` | Skip ordered shutdown and remove immediately. |
| `--workdir <path>` | Working directory. |

### `roko agent list`

List all agents with their status.

```
roko agent list [--workdir <path>]
```

### `roko agent start`

Start a previously created agent.

```
roko agent start --name <name> [--bind <addr>] [--workdir <path>]
```

| Flag | Default | Description |
|---|---|---|
| `--name <name>` | required | Agent name. |
| `--bind <addr>` | `127.0.0.1:0` | Socket address to bind (0 = auto-port). |
| `--workdir <path>` | cwd | Working directory. |

### `roko agent stop`

Stop a running agent.

```
roko agent stop --name <name> [--force] [--workdir <path>]
```

| Flag | Description |
|---|---|
| `--name <name>` | Agent name. |
| `--force` | Force kill (SIGKILL instead of SIGTERM). |

### `roko agent status`

Show detailed status for one agent.

```
roko agent status --name <name> [--workdir <path>]
```

### `roko agent serve`

Start a per-agent HTTP sidecar with 13 routes. The key routes are `/message` (real LLM
dispatch), `/stream` (WebSocket streaming), `/predictions`, `/research`, and `/tasks`.

```
roko agent serve --agent-id <id> [--bind <addr>] [--relay-url <url>]
                 [--chain-rpc-url <url>] [--identity-registry <addr>]
                 [--passport-id <id>] [--wallet-key <key>]
                 [--serve-url <url>]
```

<details>
<summary>Full flag table</summary>

| Flag | Default | Description |
|---|---|---|
| `--agent-id <id>` | required | Unique agent identifier advertised by the runtime. |
| `--bind <addr>` | `127.0.0.1:0` | Socket address to bind (0 = auto-pick free port). |
| `--relay-url <url>` | — | Relay base URL for future relay bridge hook. |
| `--chain-rpc-url <url>` | — | Chain JSON-RPC URL for future chain hooks. |
| `--identity-registry <addr>` | — | ERC-8004 identity registry contract address. |
| `--passport-id <id>` | — | ERC-8004 passport ID for `updateAgentCardUri`. |
| `--wallet-key <key>` | — | Wallet private key for future signing hooks. |
| `--serve-url <url>` | `http://localhost:6677` | roko-serve control plane URL for heartbeat reporting. |

</details>

### `roko agent chat`

Interactive chat REPL with an agent.

```
roko agent chat [--agent <id>] [--serve-url <url>]
```

| Flag | Default | Description |
|---|---|---|
| `--agent <id>` | `nunchi-intelligence` | Agent ID to chat with. |
| `--serve-url <url>` | `http://localhost:6677` | roko-serve base URL. |

---

## Research

**When to use this:** Before or after writing a plan when you want external citations,
academic backing, or up-to-date technical context to inform it. Also useful after
execution to analyze what the agents learned and what could be done better.

### `roko research topic`

Deep-dive research on a topic. Produces `.roko/research/<slug>.md` with citations.

Provider priority:
1. Perplexity deep research (if `--deep` and `PERPLEXITY_API_KEY` set)
2. Gemini grounded search (if `gemini.grounding_model` configured)
3. Perplexity standard search (if `perplexity.default_search_model` configured)
4. Claude CLI fallback (agent with file tools)

```
roko research topic <topic...> [--deep]
```

| Arg/Flag | Description |
|---|---|
| `<topic...>` | The research topic (multiple words joined). |
| `--deep` | Use Perplexity deep research (`sonar-deep-research`, async, 1-10 min). |

```bash
roko research topic "HDC vector encoding"
roko research topic "cascade router bandit algorithms" --deep
```

### `roko research enhance-plan`

Optimize an implementation plan with research-backed task decomposition techniques (citing
SWE-bench, Agentless, etc.). An agent updates the plan's `plan.md` and `tasks.toml` in place:
finer tasks, exact file and line context, runnable verify commands, and the cheapest suitable
model per task. `<plan>` is the plan's directory name under `plans/`.

```
roko research enhance-plan <plan>
```

### `roko research enhance-tasks`

Optimize tasks for efficiency, parallelism, and cheapest viable model. Adds
`context.read_files` line ranges and ensures acceptance criteria are runnable shell commands.

```
roko research enhance-tasks <plan>
```

### `roko research analyze`

Analyze execution episodes for self-learning insights and bandit weight recommendations.
Saves analysis to `.roko/research/execution-analysis.md`.

```
roko research analyze
```

### `roko research list`

List all research artifacts in `.roko/research/`.

```
roko research list
```

### `roko research search`

Direct web search using Perplexity's pure search API. Returns raw results without
synthesis. Requires `PERPLEXITY_API_KEY`.

```
roko research search <query...> [--domains <domains>] [--recency day|week|month|year]
```

| Arg/Flag | Description |
|---|---|
| `<query...>` | The search query (multiple words joined). |
| `--domains <domains>` | Restrict to these domains (comma-separated, e.g. `"docs.rs,github.com"`). |
| `--recency <period>` | Recency filter: `day`, `week`, `month`, `year`. |

---

## Knowledge

**When to use this:** The knowledge store is a durable, confidence-weighted repository of
what Roko has learned across all its runs. Use `roko knowledge query` to search it before
writing a plan (you might find relevant prior art from past runs). Use `roko knowledge dream
run` after a long execution session to distill episodes into structured knowledge. Use the
backup/restore/sync commands for multi-machine setups or disaster recovery.

### `roko knowledge query`

Query the durable knowledge store for a topic. Returns up to 10 matches ranked by
confidence. Supports `--json`.

```
roko knowledge query <topic...> [--workdir <path>]
```

Output fields per entry: index, kind, confidence (0.0–1.0), content, tags, source episodes.

```bash
roko knowledge query "cascade routing bandit"
```

### `roko knowledge stats`

Show aggregate statistics for the knowledge store: total entries, anti-knowledge count,
average confidence, entries by kind, entries by tier, entries by source, oldest/newest
entry. Supports `--json`.

```
roko knowledge stats [--workdir <path>]
```

### `roko knowledge gc`

Run garbage collection on the knowledge store. Removes entries below the minimum confidence
threshold (`DEFAULT_GC_MIN_CONFIDENCE`). Supports `--json`.

```
roko knowledge gc [--workdir <path>]
```

### `roko knowledge backup`

Backup the knowledge store to a directory, with optional genomic bottleneck (export only
the top N entries by confidence).

```
roko knowledge backup <destination> [--workdir <path>] [--force] [--top-n <n>]
```

<details>
<summary>Full flag table and output files</summary>

| Arg/Flag | Description |
|---|---|
| `<destination>` | Directory to write backup files into. |
| `--force` | Overwrite existing backup files in the destination. |
| `--top-n <n>` | Genomic bottleneck: export only the top N entries by confidence. |

Files written:
- `knowledge.jsonl` — knowledge entries
- `knowledge-confirmations.jsonl` — confirmations (if present)
- `manifest.json` — backup metadata (version, timestamp, entry count, source path)

```bash
roko knowledge backup ./backups/2026-04-29 --top-n 1000
```

</details>

### `roko knowledge restore`

Restore the knowledge store from a backup. Applies confidence decay (0.85^N per generation)
and sets all restored entries to `Transient` tier (quarantine).

```
roko knowledge restore <source> [--workdir <path>] [--force] [--types <types>]
                       [--min-confidence <f>] [--generation <n>]
```

<details>
<summary>Full flag table</summary>

| Arg/Flag | Default | Description |
|---|---|---|
| `<source>` | required | Directory created by `roko knowledge backup`. |
| `--force` | false | Overwrite existing local neuro store files. |
| `--types <types>` | — | Filter by knowledge types (comma-separated). |
| `--min-confidence <f>` | — | Only restore entries with confidence >= this threshold (0.0–1.0). |
| `--generation <n>` | `1` | Generation hop count for confidence decay. Decay factor: `0.85^N`. |

```bash
roko knowledge restore ./backups/2026-04-29 --generation 2 --min-confidence 0.5
```

</details>

### `roko knowledge sync`

Sync knowledge with a peer agent via the Mesh protocol. Outbox deltas are written to
`.roko/mesh/outbox/delta-<peer>.jsonl`. Inbox deltas are read from
`.roko/mesh/inbox/delta-<peer>.jsonl`. Received entries get a 0.7x confidence discount
and are set to `Transient` tier. Supports `--json`.

```
roko knowledge sync <peer> [--workdir <path>] [--direction send|receive|both] [--max-send <n>]
```

| Arg/Flag | Default | Description |
|---|---|---|
| `<peer>` | required | Peer agent identifier. |
| `--direction` | `both` | Direction: `send`, `receive`, or `both`. |
| `--max-send <n>` | `100` | Maximum engrams to send per cycle. |

### Dream consolidation

Dream consolidation is the offline process that reads raw episode logs, clusters them,
and distills the patterns into structured knowledge entries and playbooks.

#### `roko knowledge dream run`

Run a dream consolidation cycle immediately. Processes episodes from `.roko/episodes.jsonl`,
clusters them, writes knowledge entries and playbooks to `.roko/neuro/`, and saves a report
to `.roko/dreams/`. Also refreshes the C-factor snapshot. Supports `--json`.

```
roko knowledge dream run [--workdir <path>]
```

Output fields: processed episodes, clusters found, knowledge entries written, playbooks
created, C-factor (overall score), report saved path.

#### `roko knowledge dream report`

Show the latest dream report without running a new cycle. Supports `--json`.

```
roko knowledge dream report [--workdir <path>]
```

#### `roko knowledge dream schedule`

Show when the next dream should fire based on idle threshold and last run time. Supports
`--json`.

```
roko knowledge dream schedule [--workdir <path>]
```

#### `roko knowledge dream journal`

Display recent dream journal entries.

```
roko knowledge dream journal [--limit <n>] [--workdir <path>]
```

| Flag | Default | Description |
|---|---|---|
| `--limit <n>` | `10` | Number of recent entries to display. |

Output per entry: timestamp, cycle ID, agent ID, hypotheses (generated/staged/promoted),
total tokens, early termination flag.

#### `roko knowledge dream archive`

Display recent dream archive entries.

```
roko knowledge dream archive [--limit <n>] [--workdir <path>]
```

| Flag | Default | Description |
|---|---|---|
| `--limit <n>` | `10` | Number of recent entries to display. |

Output per entry: archived_at, entry_id, kind, quality_score, summary.

### Custody chain

The custody chain is an append-only audit log tracking how knowledge entries move through
the system. Use it when you need to understand the provenance of a specific entry.

#### `roko knowledge custody list`

List recent custody records.

```
roko knowledge custody list [--limit <n>] [--workdir <path>]
```

#### `roko knowledge custody show`

Show full details of a custody record by index.

```
roko knowledge custody show <index> [--workdir <path>]
```

#### `roko knowledge custody verify`

Verify the integrity of the custody chain (checks hash linkages).

```
roko knowledge custody verify [--workdir <path>]
```

### `roko knowledge archive`

Move old engrams to cold storage (compressed monthly archives) at `.roko/cold/`. Prompts
for confirmation unless `--quiet` or non-TTY.

```
roko knowledge archive [--older-than <duration>] [--batch-size <n>] [--workdir <path>] [--dry-run]
```

| Flag | Default | Description |
|---|---|---|
| `--older-than <duration>` | `30d` | Archive engrams older than this duration. Formats: `30d`, `7d`, `24h`, `60m`, `3600s`. |
| `--batch-size <n>` | `500` | Maximum engrams to archive per batch. |
| `--dry-run` | false | Print what would be archived without doing it. |

---

## Learning and feedback

**When to use this:** After a batch of runs, check `roko learn all` to see how the cascade
router is adapting, whether any prompt experiments have concluded, and what the efficiency
trend looks like. Use `roko learn tune` to view or adjust the adaptive thresholds that gate
the plan executor.

### `roko learn all`

Show all learning state: cascade router, experiments, efficiency, episodes, and knowledge.

```
roko learn all [--workdir <path>]
```

### `roko learn route`

Show cascade router state from `.roko/learn/cascade-router.json`. Displays total
observations, model slugs, stage transitions, and the latest transition.

The router moves through three stages as it accumulates data:
- `static`: 0–49 observations (uses config defaults)
- `confidence`: 50–199 observations (confidence-interval based)
- `ucb`: 200+ observations (Upper Confidence Bound bandit)

```
roko learn route [--workdir <path>]
```

### `roko learn experiments`

Show experiment state: prompt experiments (`.roko/learn/experiments.json`) and model
experiments (`.roko/learn/model-experiments.json`). Displays running and concluded counts
per experiment, plus winner info.

```
roko learn experiments [--workdir <path>]
```

### `roko learn efficiency`

Show efficiency metrics from `.roko/learn/efficiency.jsonl`. Displays event count, date
range, and most recent event (model, task, plan, gate pass/fail, cost).

```
roko learn efficiency [--workdir <path>]
```

### `roko learn episodes`

Show episode summary from the canonical `.roko/episodes.jsonl`. A pre-V3
`.roko/learn/episodes.jsonl` is used only as a read-only fallback when the canonical log is absent.
Displays episode count, date range, and most recent episode (model, task, pass/fail, cost).

```
roko learn episodes [--workdir <path>]
```

### `roko learn tune`

Display and optionally adjust adaptive thresholds.

```
roko learn tune [<subsystem>] [--dry-run] [--workdir <path>]
```

| Arg | Default | Description |
|---|---|---|
| `<subsystem>` | `gates` | Subsystem to tune: `gates`, `routing`, `budget`. |
| `--dry-run` | false | Display current values without modifying. |

Subsystem details:
- `gates` — Reads `.roko/learn/gate-thresholds.json` (EMA-adjusted per rung)
- `routing` — Reads `.roko/learn/cascade-router.json`
- `budget` — Shows entry count from `.roko/learn/efficiency.jsonl`

---

## Jobs

**When to use this:** When you want to create work items that can be assigned to specific
agents (including remote ones) via the marketplace. The job system is the interface between
Roko's orchestrator and its agent fleet — you post a job, agents bid or are matched, and
the job executes.

### `roko job list`

List all marketplace jobs with optional status filter.

```
roko job list [--status <status>] [--workdir <path>]
```

Status values: `open`, `assigned`, `in_progress`, `completed`, `failed`, `cancelled`.

### `roko job create`

Create a new marketplace job.

```
roko job create <title> [--type <type>] [--description <text>] [--priority <priority>]
                [--auto-execute] [--plan-id <id>] [--workdir <path>]
```

| Arg/Flag | Default | Description |
|---|---|---|
| `<title>` | required | Job title. |
| `--type <type>` | `research` | Job type: `research`, `coding_task`, `chain_monitor`, `chain_analysis`. |
| `--description <text>` | `""` | Job description. |
| `--priority <priority>` | `medium` | Priority: `low`, `medium`, `high`, `critical`. |
| `--auto-execute` | false | Auto-execute when the runner picks it up. |
| `--plan-id <id>` | — | Associated plan ID. |

### `roko job match`

Match a proposed job against registered agents via roko-serve.

```
roko job match <title> [--serve-url <url>] [--description <text>] [--language <lang>]
               [--min-tier <tier>] [--reward <reward>] [--skills <skills>] [--workdir <path>]
```

<details>
<summary>Full flag table</summary>

| Arg/Flag | Default | Description |
|---|---|---|
| `<title>` | required | Job title. |
| `--serve-url <url>` | `http://localhost:6677` | roko-serve base URL. |
| `--min-tier <tier>` | — | Minimum agent tier: `Unverified`, `Verified`, `Trusted`, `Expert`, `Pioneer`. |
| `--reward <reward>` | `""` | Reward string, e.g. `"2500 KORAI"`. |
| `--skills <skills>` | — | Required skills (comma-separated). |
| `--language <lang>` | — | Primary implementation language. |

</details>

### `roko job show`

Show details for a specific job.

```
roko job show <id> [--workdir <path>]
```

### `roko job execute`

Execute a job locally or via roko-serve.

```
roko job execute <id> [--serve-url <url>] [--workdir <path>]
```

### `roko job cancel`

Cancel a job.

```
roko job cancel <id> [--workdir <path>]
```

---

## Benchmarks

**When to use this:** When you want to measure roko's performance against a baseline, run
SWE-bench-style evaluations, or generate learning telemetry without running a live plan.
The `bench demo` command is useful for quick before/after comparisons when tuning config.

### `roko bench demo`

Run a comparative benchmark: naive vs roko-optimized. Uses simulated data by default.

```
roko bench demo [--real] [--workdir <path>]
```

| Flag | Description |
|---|---|
| `--real` | Use real LLM dispatch instead of simulated results. |

```bash
roko bench demo                    # Run with simulated data
roko bench demo --real             # Run with real LLM dispatch
```

### `roko bench swe`

Run a native SWE-bench-style proxy batch.

```
roko bench swe [--dataset <path>] [--batch-size <n>] [--offset <n>]
               --agent-mode gold|empty|prediction-file|command
               [--predictions <path>] [--agent-command <cmd>]
               [--report <path>] [--export-predictions <path>]
               [--no-learning] [--keep-workdirs] [--workdir <path>]
```

<details>
<summary>Full flag table</summary>

| Flag | Default | Description |
|---|---|---|
| `--dataset <path>` | built-in smoke | Local JSONL dataset. If omitted, a built-in two-task smoke dataset is generated. |
| `--batch-size <n>` | `2` | Number of instances to run. |
| `--offset <n>` | `0` | Offset into the dataset. |
| `--agent-mode <mode>` | required | Agent adapter: `prediction-file` or `command` measure an agent; `gold` and `empty` are controls that check the harness and are never recorded as learning. |
| `--predictions <path>` | — | Predictions JSONL for `--agent-mode prediction-file`. |
| `--agent-command <cmd>` | — | Command for `--agent-mode command`. Receives instance JSON on stdin, prints a unified diff. |
| `--report <path>` | — | Scores JSONL output path. |
| `--export-predictions <path>` | — | Write SWE-bench-style predictions JSONL. |
| `--no-learning` | false | Disable episode, efficiency, and C-factor writes. |
| `--keep-workdirs` | false | Keep per-instance benchmark workdirs for debugging. |

</details>

<details>
<summary>Examples</summary>

```bash
roko bench swe --batch-size 2 --agent-mode gold
roko bench swe --dataset ./swe-smoke.jsonl --predictions ./predictions.jsonl --agent-mode prediction-file
roko bench swe --agent-mode command --agent-command './my-agent.sh'
```

</details>

---

## Configuration

**When to use this:** The `roko config` subcommands are the primary way to manage your
setup without hand-editing TOML files. Start with `roko config init` to get a working
configuration, use `roko config show` to inspect the merged result, and `roko config set`
to change individual values.

### `roko config init` (alias: `roko config wizard`)

Interactive wizard: detects installed LLM CLIs, writes global config.

```
roko config init [--yes] [--agent <cmd>] [--model <model>] [--budget <n>] [--role <role>]
                 [--enable-gates] [--path <path>] [--non-interactive]
```

<details>
<summary>Full flag table</summary>

| Flag | Description |
|---|---|
| `--yes` | Skip all confirmation prompts. |
| `--agent <cmd>` | Pre-select agent command (skip picker). |
| `--model <model>` | Pre-set model name (ollama-only convenience). |
| `--budget <n>` | Pre-set token budget (`budget.prompt_token_budget`). |
| `--role <role>` | Ignored: no config key stores a role text any more. |
| `--enable-gates` | Enable default compile+clippy gates. |
| `--path <path>` | Write to this path instead of the resolved global path. |
| `--non-interactive` | Skip all prompts, fail if any answer is missing. |

</details>

### `roko config show`

Print the effective merged config with per-field source tags (shows which layer each value
comes from: CLI flag, env var, project config, or global config).

```
roko config show [--workdir <path>]
```

### `roko config path`

Print the resolved global + project + env config paths.

```
roko config path [--workdir <path>]
```

### `roko config edit`

Open `$EDITOR` on the chosen config file. Flags `--global` and `--project` are mutually
exclusive.

```
roko config edit [--global] [--project] [--workdir <path>]
```

### `roko config set`

Set a dotted key in the chosen config layer.

```
roko config set <key> <value> [--global] [--project] [--workdir <path>]
```

```bash
roko config set agent.command claude
roko config set agent.model claude-opus-4-5 --project
```

### `roko config set-secret`

Store a secret in `~/.roko/.env` as `NAME=VALUE`.

```
roko config set-secret <name> <value>
```

### `roko config check-secrets`

Check `${VAR}` references in config and validate that referenced secrets exist.

```
roko config check-secrets [--workdir <path>]
```

### `roko config validate`

Validate `roko.toml` syntax, schema, and semantic references.

```
roko config validate [--workdir <path>]
```

### `roko config migrate`

Migrate a legacy `roko.toml` into explicit `[providers.*]` and `[models.*]` tables.

```
roko config migrate [--workdir <path>] [--dry-run] [-y]
```

| Flag | Description |
|---|---|
| `--dry-run` | Print the proposed migration without writing changes. |
| `-y` | Skip the confirmation prompt and apply the migration immediately. |

### Providers

#### `roko config providers list`

List configured providers and their current connection status.

```
roko config providers list [--workdir <path>]
```

#### `roko config providers health`

Show persisted provider circuit-breaker health and latency.

```
roko config providers health [--workdir <path>]
```

#### `roko config providers test`

Send a minimal request to verify provider connectivity.

```
roko config providers test [<provider>] [--all] [--workdir <path>]
```

| Arg/Flag | Description |
|---|---|
| `<provider>` | Provider name from `[providers.*]`. Omit when using `--all`. |
| `--all` | Test every configured provider and print a summary table. |

### Models

#### `roko config models list`

List configured models and their capabilities.

```
roko config models list [--workdir <path>]
```

#### `roko config models route`

Show the current routing decision for a model and optionally explain the full routing trace.

```
roko config models route <model> [--explain] [--complexity <tier>] [--workdir <path>]
```

| Arg/Flag | Description |
|---|---|
| `<model>` | Model key or slug to explain. |
| `--explain` | Show the full routing trace instead of only the final decision. |
| `--complexity <tier>` | Complexity tier: `mechanical`, `focused`, `integrative`, `architectural`. |

### Subscriptions and events

#### `roko config subscriptions list`

List all event subscriptions.

```
roko config subscriptions list
```

#### `roko config subscriptions add`

Create a new event subscription.

```
roko config subscriptions add --template <name> --trigger <glob>
```

#### `roko config subscriptions remove`

Delete a subscription.

```
roko config subscriptions remove <id>
```

#### `roko config subscriptions enable` / `roko config subscriptions disable`

Enable or disable a subscription.

```
roko config subscriptions enable <id>
roko config subscriptions disable <id>
```

#### `roko config events`

Inspect configured event sources (cron jobs, file watchers).

```
roko config events [--workdir <path>]
```

### Experiments

#### `roko config experiments`

Manage model A/B experiments.

```
roko config experiments <subcommand>
```

### Plugins

#### `roko config plugins list`

List available and installed plugins.

```
roko config plugins list [--workdir <path>]
```

#### `roko config plugins install`

Install a plugin from a local path or registry.

```
roko config plugins install <source> [--workdir <path>]
```

#### `roko config plugins publish`

Build, sign, and publish a WASM extension directory to an authenticated relay registry.
The bearer token and Ed25519 signing key are read only from
`ROKO_EXTENSION_REGISTRY_PUBLISH_TOKEN` and `ROKO_EXTENSION_REGISTRY_SIGNING_KEY`.

```
roko config plugins publish <source> --publisher <id>
                            [--registry <url>] [--workdir <path>]
```

#### `roko config plugins remove`

Remove an installed plugin by name.

```
roko config plugins remove <name> [--workdir <path>]
```

#### `roko config plugins audit`

Audit installed plugins and report capabilities.

```
roko config plugins audit [--workdir <path>]
```

### Secrets

#### `roko config secrets`

Manage profile-aware secrets (set, get, list, rotate).

```
roko config secrets <subcommand>
```

---

## Code intelligence

**When to use this:** `roko index build` indexes your workspace symbols for faster,
context-aware agent dispatches. The code-intelligence MCP server (`roko-mcp-code`) uses
this index to answer structural queries about the codebase.

### `roko index build`

Build a code index for the workspace.

```
roko index build [--path <path>]
```

### `roko index rebuild`

Drop existing index data and rebuild from source files.

```
roko index rebuild [--path <path>]
```

### `roko index search`

Search the code index.

```
roko index search <query> [--kind <kind>] [--strategy <strategy>] [--limit <n>] [--path <path>]
```

| Arg/Flag | Default | Description |
|---|---|---|
| `<query>` | required | Search query text. |
| `--kind <kind>` | — | Restrict to a symbol kind: `function`, `struct`, `enum`, `trait`, `const`, `type`, `module`, `impl`. |
| `--strategy <strategy>` | `keyword` | Search strategy: `keyword`, `structural`, `hybrid`. |
| `--limit <n>` | `20` | Maximum number of results. |

### `roko index stats`

Show index statistics.

```
roko index stats [--path <path>]
```

---

## GitHub integration

Use the standalone status command to validate repository configuration and inspect the
authenticated account, open `roko/plan/*` pull requests and their CI state, and open task
failure issues. It does not require `roko serve`.

### `roko github status`

```
roko [--json] github status [--workdir <path>]
```

| Arg/Flag | Default | Description |
|---|---|---|
| `--workdir <path>` | current workspace / `--repo` | Directory whose layered `roko.toml` supplies `[github]`. |
| `--json` | off | Emit a structured report for automation. This is a global flag and must precede `github`. |

The command exits successfully with a local diagnostic when `GITHUB_TOKEN` is missing,
empty, or unreadable; remote sections are marked skipped and the output explains how to
enable them. Individual API failures are reported per section instead of hiding the rest of
the status report. See [GitHub Integration](GITHUB-INTEGRATION.md) for credentials, runner
automation, MCP setup, and webhook configuration.

---

## Server and deployment

**When to use this:** `roko serve` starts the HTTP control plane that the TUI, external
dashboards, and remote agent sidecars all connect to. `roko up` is the one-command
equivalent when you want both the server and all configured agents running. Use the
`roko deploy` commands to push to Railway, Fly.io, or Docker.

### `roko up`

Start roko serve + all configured `[[agents]]` in one command.

```
roko up [--workdir <path>]
```

### `roko serve`

Start the HTTP API server on `:6677` (~317 REST routes + SSE + WebSocket).

```
roko serve [--bind <addr>] [--port <port>] [--workdir <path>] [--tui] [--enable-terminal]
```

| Flag | Default | Description |
|---|---|---|
| `--bind <addr>` | `127.0.0.1` | Address to bind. |
| `--port <port>` | `6677` | Port number. |
| `--workdir <path>` | cwd | Working directory. |
| `--tui` | false | Run the interactive TUI dashboard embedded in the server process (reads live state from StateHub, zero-copy, no file polling). |
| `--enable-terminal` | false | Expose the PTY terminal routes. |

### `roko acp`

Start ACP (Agent Client Protocol) server for editor integration. Uses stdio for JSON-RPC;
logs are redirected to a file to avoid corrupting the protocol channel.

```
roko acp [--workdir <path>] [--profile <profile>] [--config <path>] [--log-file <path>]
```

| Flag | Default | Description |
|---|---|---|
| `--workdir <path>` | `.` | Working directory. |
| `--profile <profile>` | `default` | Configuration profile. |
| `--config <path>` | — | Path to `roko.toml`. |
| `--log-file <path>` | `.roko/acp.log` | Log file path. |

### `roko daemon`

Manage daemon mode.

<details>
<summary>Daemon subcommands</summary>

#### `roko daemon start`

```
roko daemon start [--foreground] [--port <port>]
```

#### `roko daemon stop`

```
roko daemon stop
```

#### `roko daemon status`

```
roko daemon status
```

#### `roko daemon logs`

```
roko daemon logs [-f] [-n <lines>]
```

| Flag | Default | Description |
|---|---|---|
| `-f` / `--follow` | false | Stream new log lines as they appear. |
| `-n` / `--lines <n>` | `50` | Number of lines to show. |

#### `roko daemon reload`

SIGHUP equivalent — re-scan subscriptions and templates without restart.

```
roko daemon reload
```

#### `roko daemon restart`

```
roko daemon restart [--port <port>]
```

#### `roko daemon install`

Generate and install the macOS launchd plist.

```
roko daemon install
```

#### `roko daemon uninstall`

Remove the macOS launchd plist.

```
roko daemon uninstall
```

</details>

### `roko deploy`

Deploy to cloud targets.

<details>
<summary>Deploy subcommands</summary>

#### `roko deploy railway`

Deploy to Railway via the public GraphQL API. Creates a Railway project with roko-serve
as the control plane.

```
roko deploy railway [--workdir <path>] [--with-mirage] [--workers <templates>]
```

| Flag | Description |
|---|---|
| `--with-mirage` | Also deploy the chain relay service. |
| `--workers <templates>` | Deploy worker services for these template names (comma-separated). |

#### `roko deploy fly`

Generate `fly.toml` and deploy with Fly.io.

```
roko deploy fly [--workdir <path>]
```

#### `roko deploy docker`

Build the local Docker image and tag it for the configured registry.

```
roko deploy docker [--workdir <path>] [--registry <namespace>]
```

</details>

### `roko worker`

Run as a deployed worker (reads template from env, serves tasks on `:8080` or `$PORT`).

```
roko worker [--port <port>]
```

---

## Authentication

### `roko login`

Authenticate with a roko-serve instance.

```
roko login [<url>] [--api-key] [--check] [--dashboard-url <url>]
```

| Arg/Flag | Default | Description |
|---|---|---|
| `<url>` | `http://localhost:6677` | URL of the roko-serve instance. |
| `--api-key` | false | Login with an API key instead of browser auth. |
| `--check` | false | Non-interactive: validate stored credential only (requires `--api-key`). |
| `--dashboard-url <url>` | `http://localhost:5173` | Dashboard URL for browser auth. Env: `NUNCHI_DASHBOARD_URL`. |

<details>
<summary>Examples</summary>

```bash
roko login                              # Login via browser (Privy)
roko login --api-key                    # Login with an API key (prompts)
roko login --api-key --check            # Validate stored API key credential
roko login https://my-server.com        # Login to a remote server
```

</details>

### `roko logout`

Remove stored credentials.

```
roko logout
```

### `roko whoami`

Show current authentication status.

```
roko whoami
```

---

## Utilities

### `roko resume`

Resume a plan execution from its last checkpoint.

```
roko resume [<run-id>] [--workdir <path>]
```

| Arg/Flag | Description |
|---|---|
| `<run-id>` | Run or plan ID to resume (defaults to most recent snapshot). |
| `--workdir <path>` | Working directory. |

```bash
roko resume                        # Resume from default snapshot
roko resume run_4823               # Resume a specific run by ID
```

### `roko replay`

Walk the lineage DAG rooted at a signal hash and print it. Useful for forensic investigation
of how a particular piece of knowledge or output was produced.

```
roko replay <hash> [--workdir <path>] [--forensic] [--as-of <step>] [--format tree|json]
```

| Arg/Flag | Default | Description |
|---|---|---|
| `<hash>` | required | Signal hash (64 hex chars) to walk. |
| `--forensic` | false | Show forensic detail: timestamps, full hashes, metadata. |
| `--as-of <step>` | — | Filter replay to events from this step forward. |
| `--format tree\|json` | `tree` | Output format. |

### `roko inject`

Send a directive, context or abort to a running `roko plan run`. Each plan run listens on an
owner-only socket of its own, `.roko/runtime/inject/<pid>.sock`, and a client must first present
the token the run wrote beside it. The session names a running plan, by its id or by its Graph
checkpoint run. The command exits 0 only once that run has acknowledged the request:

- a `directive` or `context` is added, once, to the prompt of the next task of that plan to
  start, under an "Operator directive" or "Operator context" heading. A text may be up to 8 KiB,
  and a plan holds at most 8 waiting texts;
- an `abort` cancels that plan, and only it.

A request sent again under the same request ID is answered again rather than delivered twice.
Otherwise the command exits non-zero, writes nothing, and says why:
`inject_transport_unavailable` (no plan run is listening), `inject_unknown_session` (no
listening run has that plan) or `inject_rejected` (the run refused it, did not answer in time, or
has finished). With `--json` it prints one object with `code` and `message`, plus `hint` on a
failure.

```
roko inject <session> <payload> [--kind directive|abort|context] [--workdir <path>]
```

| Arg/Flag | Default | Description |
|---|---|---|
| `<session>` | required | A running plan's ID, or its checkpoint run ID. |
| `<payload>` | required | The text; empty for `abort`. |
| `--kind <kind>` | `directive` | Kind of signal to inject: `directive`, `abort`, `context`. |

### `roko completions`

Generate shell completion scripts.

```
roko completions <shell>
```

```bash
roko completions bash >> ~/.bashrc
roko completions zsh >> ~/.zshrc
roko completions fish > ~/.config/fish/completions/roko.fish
```

### `roko new`

Generate boilerplate for a Roko trait or domain profile. Use this when extending Roko with
a custom gate, scorer, router, or policy.

```
roko new <type> <name> [--output <path>]
```

Supported types:
- `gate` — A new Gate implementation
- `scorer` — A new Scorer implementation
- `router` — A new Router implementation
- `policy` — A new Policy implementation
- `substrate` — A new Substrate implementation
- `composer` — A new Composer implementation
- `domain` — A domain profile scaffold
- `template` — An agent template
- `event-source` — An event source plugin

```bash
roko new gate my-custom-gate
roko new scorer priority-scorer --output ./crates/roko-custom/
```

### `roko explain`

Explain a roko concept with progressive disclosure (3 depth levels). Start at depth 1 and
go deeper as needed.

```
roko explain <topic> [--depth 1|2|3]
```

| Arg/Flag | Default | Description |
|---|---|---|
| `<topic>` | required | Concept to explain. See below for valid topics. |
| `--depth <n>` | `1` | Disclosure depth: 1 = summary, 2 = how it works, 3 = internals. |

Topics: `gates`, `routing`, `cognitive`, `neuro`, `daimon`, `dreams`, `engram`, `cfactor`.

```bash
roko explain gates                 # Summary of the gate pipeline
roko explain routing --depth 2     # How cascade router works
roko explain dreams --depth 3      # Dream consolidation internals
```

---

## Vision loop

**When to use this:** When you want agents to iteratively improve a UI by scoring
screenshots against a goal description. The loop takes a screenshot, evaluates it with a
vision model, makes code changes, and repeats until the score threshold is reached or the
iteration budget is exhausted.

### `roko vision-loop`

```
roko vision-loop <target-file> --goal <text> --url <url>
                 [--max-iter <n>] [--target-score <f>] [--consecutive-target <n>]
                 [--regression-threshold <f>] [--model <model>]
                 [--viewport-width <px>] [--viewport-height <px>] [--wait-ms <ms>]
```

<details>
<summary>Full flag table</summary>

| Arg/Flag | Default | Description |
|---|---|---|
| `<target-file>` | required | Source file to iterate on (e.g. `src/pages/Home.tsx`). |
| `--goal <text>` | required | What the UI should look/feel like. |
| `--url <url>` | required | URL to screenshot (e.g. `http://localhost:5173`). |
| `--max-iter <n>` | `10` | Maximum iterations. |
| `--target-score <f>` | `9.0` | Score threshold (1–10) for early stopping. |
| `--consecutive-target <n>` | `2` | Consecutive target hits before stopping. |
| `--regression-threshold <f>` | `3.0` | Score drop from peak that triggers rollback. |
| `--model <model>` | auto | Vision model key from `roko.toml`. |
| `--viewport-width <px>` | `1280` | Viewport width in pixels. |
| `--viewport-height <px>` | `720` | Viewport height in pixels. |
| `--wait-ms <ms>` | `2000` | Milliseconds to wait after writing (HMR settle time). |

</details>

---

## Environment variables

<details>
<summary>Full environment variable reference</summary>

| Variable | Effect |
|---|---|
| `ROKO_TIMING=1` | Print elapsed time after command execution (same as `--timing`). |
| `ROKO_LOG_RAW=1` | Disable secret redaction in log output (debugging only). |
| `RUST_LOG=<directive>` | Override the tracing filter (e.g. `roko=debug`). |
| `NO_COLOR` | Disable ANSI colors when set and non-empty. |
| `CLICOLOR_FORCE` | Force ANSI colors when set and not `"0"`. |
| `CLICOLOR=0` | Disable ANSI colors. |
| `NUNCHI_DASHBOARD_URL` | Override the dashboard URL for browser auth (`roko login`). |
| `PERPLEXITY_API_KEY` | Required for `roko research search` and Perplexity-backed research. |
| `GEMINI_API_KEY` | Required for Gemini-grounded research. |
| `PORT` | Override the worker server port (used by `roko worker`). |

</details>

---

## Config file locations and precedence

Roko uses a layered config system. Lower numbers override higher numbers:

1. **CLI flags** — `--model`, `--role`, `--effort`, etc. (highest priority)
2. **Environment variables** — `ROKO_MODEL`, `ROKO_ROLE` (if supported by the config loader)
3. **Project config** — `./roko.toml` (or path from `--config`)
4. **Global config** — `~/.roko/config.toml`
5. **Built-in defaults** (lowest priority)

Config file path resolution:
- `--config <path>` → use exactly that path
- Otherwise → search upward from cwd for `roko.toml`
- If not found → fall back to `~/.roko/config.toml`

Use `roko config path` to print the resolved paths, and `roko config show` to see the
merged effective config with per-field source tags.

### Version string

Running `roko --version` prints the short version. `roko --long-version` prints:

```
<semver> (<rustc-version>, <target-triple>, git <short-hash>)
```

Build-time variables are injected via `build.rs`: `ROKO_GIT_HASH`, `ROKO_RUSTC_VERSION`,
`ROKO_TARGET`.

---

## Data directory layout

All runtime data lives under `.roko/` in the workspace root.

```
.roko/
├── config.toml             # Optional project-level config override
├── episodes.jsonl          # Agent turn recording (EpisodeLogger)
├── signals.jsonl           # Signal log (FileSubstrate hot store)
├── plans/                  # Legacy plan location; new plans go to plans/ at the workspace root
├── state/
│   ├── state-snapshot.json # Legacy runner-v2 snapshot (deprecated)
│   └── graph/              # Graph engine checkpoints (resume state)
├── research/               # Research artifacts (.md files)
├── learn/
│   ├── cascade-router.json # CascadeRouter persistence
│   ├── experiments.json    # Prompt experiment store
│   ├── model-experiments.json
│   ├── efficiency.jsonl    # Per-turn efficiency events
│   └── gate-thresholds.json
├── neuro/
│   ├── knowledge.jsonl     # Durable knowledge store
│   └── knowledge-confirmations.jsonl
├── dreams/                 # Dream cycle reports
├── cold/                   # Cold archived engrams
├── mesh/
│   ├── inbox/              # Incoming mesh sync deltas
│   └── outbox/             # Outgoing mesh sync deltas
├── agents/
│   └── <name>/
│       └── manifest.toml   # Agent manifest (roko agent create)
├── daimon/
│   └── affect.json         # Daimon affect state
├── acp.log                 # ACP server log
└── serve-tui.log           # TUI mode tracing log
```

---

## Troubleshooting

Common errors and what to do about them:

| Error pattern | What it means | Fix |
|---|---|---|
| `.roko/` or `roko.toml` not found | Workspace not initialized | `roko init` |
| `agent not found` / `unknown agent` | No agents registered | `roko agent list` to see what exists |
| `plan not found` / `no plans found` | No plan files in the directory | `roko plan list`, or write one with `roko plan generate "<prompt>"` |
| `connection refused` / `connect error` | roko-serve is not running | `roko serve` in another terminal |
| Gate failures on every task | Config or code problem, not an agent problem | `roko doctor` then check `.roko/learn/gate-thresholds.json` |
| Run interrupted, want to continue | Normal state for long plans | `roko plan run plans/ --resume-plan` |

Run `roko doctor` for a comprehensive diagnostic that checks all prerequisites at once.

---

## Build requirements

Roko requires Rust 1.91 or later (needed for `alloy` dependencies).

```
rustup update stable
cargo build --workspace
```

Pre-commit checks — CI will reject code that fails any of these:

```bash
cargo +nightly fmt --all                              # Format (nightly, matches CI)
cargo clippy --workspace --no-deps -- -D warnings     # Lint (must pass clean)
cargo test --workspace                                # Tests (must pass)
```

Do not push without running all three.
