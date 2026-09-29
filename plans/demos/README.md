# Demo plans

Plans for showing roko to people and for exercising one feature at a time.
Each one passes `roko plan validate --strict`, writes only under `demo/`, and
has `status = "fixture"` in its `[meta]`.

## Catalogue

| Plan | What it shows | Tasks | Expected result | Agent and cost |
|---|---|---|---|---|
| [`demo-hello`](demo-hello/) | The smallest real run: one task, one verify step, TUI approval | 1: implementer | `demo/roko-plan-smoke-output.md` with the three required lines | Real model; not recorded |
| [`parallel-plans/demo-hello-world`](parallel-plans/demo-hello-world/) | A one-file Rust program; the free portal walkthrough | 1: implementer | `demo/hello-world/main.rs` compiles and prints `hello world` | Fake agent (free, seconds) or a real model |
| [`parallel-plans/demo-print-hello`](parallel-plans/demo-print-hello/) | The second, independent plan of the `parallel-plans` set | 1: implementer | `demo/print-hello/main.rs` prints a line starting with `hello` | Fake agent or a real model |
| [`demo-fibonacci`](demo-fibonacci/) | Implementer-to-scribe hand-off on a unit-tested program | 2: implementer, scribe | `demo/fibonacci/` prints `0 1 1 2 3 5 8 13 21 34`, its tests pass, and it has a README | Real model; not recorded |
| [`demo-parallel-integration`](demo-parallel-integration/) | Two sibling tasks run at once, then a join task reads both | 3: implementer | `left.json`, `right.json` and `combined.json` (total 42) under `demo/parallel-integration/` | Real model; $0.43 on 2026-09-21 |
| [`demo-resume-recovery`](demo-resume-recovery/) | A run stops after task 1 and a second process resumes it | 2: implementer | The first run stops at 1/2; the resumed run completes with both stage files | Real model; $0.46 on 2026-09-19 |
| [`hello-color/demo-hello-color`](hello-color/demo-hello-color/) | Five tasks with two roles, and two pairs of tasks running in parallel | 5: scribe, implementer | The `demo/hello-color/` crate prints a coloured greeting, with research, review and README files | Real model; $5.78 on 2026-09-23 (earlier version whose research task could not write its file) |
| [`hello-color/demo-greeting-module`](hello-color/demo-greeting-module/) | A plan that depends on another plan (`depends_on_plan`) | 1: implementer | `src/greeting.rs` with a passing unit test; the program's output is unchanged | Real model; not recorded |
| [`demo-multistage`](demo-multistage/) | A five-stage non-code workflow ending in a read-only review | 5: scribe, implementer, quick-reviewer | `sh demo/multistage-plan/validate.sh` prints `multi-stage demo: PASS` | Real model; $0.50 on 2026-09-21 |
| [`demo-incident-tabletop`](demo-incident-tabletop/) | Repository evidence turned into a timed incident exercise | 4: scribe, implementer, quick-reviewer | Risk register, scenario and facilitator guide under `demo/incident-tabletop/` | Real model; $2.03 on 2026-09-21 |
| [`demo-release-readiness`](demo-release-readiness/) | A release go/no-go decision run as an evidence workflow | 4: scribe, implementer, quick-reviewer | `inventory.json`, `go-no-go.md` and `check.sh` under `demo/release-readiness/` | Real model; not recorded |
| [`demo-full-stack`](demo-full-stack/) | Six tasks build, audit and document an axum web server | 6: scribe, implementer | `demo/hello-server/` builds and serves `/`, `/health` and `/info`; `COMPLETION.md` signs off | Real model; no recorded run; its builds fetch crates |

Costs are what the plan's Graph checkpoint recorded for its last run, with
whatever model that run used.

Four demos read this repository's own files (`demo-multistage`,
`demo-incident-tabletop`, `demo-release-readiness`, `demo-full-stack`), so they
only run here. The others also run in an empty scratch workspace. The
`hello-color` set and `demo-full-stack` build with Cargo, which fetches their
crates on first use.

Several demos have run here before, and their outputs are checked in under
`demo/`. A rerun starts from those files, so its agents may find little or
nothing to change. To watch the work happen from nothing, use a scratch
workspace (see below) or delete the output directory first.

## Run from the command line

Build once with `cargo build -p roko-cli --bin roko`, then from the repository
root:

```sh
./target/debug/roko plan run plans/demos/demo-hello --approval --fresh
```

- `--approval` opens the TUI; without it you get plain logs.
- `--fresh` archives the plan's earlier run state, so every task runs again.
- `--model <slug>` pins the model. The earlier instructions ran the larger
  demos with `--model glm-5-1`.
- `--dry-run` lists the tasks without running them.

Each plan set runs as a unit:

```sh
# Both plans at the same time.
./target/debug/roko plan run plans/demos/parallel-plans --max-parallel-plans 2 --fresh

# demo-hello-color first, then demo-greeting-module.
./target/debug/roko plan run plans/demos/hello-color --fresh
```

Run on its own before `demo-hello-color` has succeeded, `demo-greeting-module`
stops before any agent starts: `depends on plan 'demo-hello-color', which is
not in the selected plan set and is not complete`.

The resume demo takes two commands. The first stops at 1/2; the second
resumes at stage two:

```sh
./target/debug/roko plan run plans/demos/demo-resume-recovery --fresh --max-retries 0
ROKO_RESUME_DEMO_READY=1 ./target/debug/roko plan run plans/demos/demo-resume-recovery
```

### Demo configs

Each top-level demo and each plan set has a `roko.toml` whose only gate is a
no-op base rung, so the plan's own verify steps decide acceptance rather than
the workspace's compile, clippy and test gates. Plans in a set share the set's
file.

`roko plan run` does not read `--config`: the Graph runner loads `ROKO_CONFIG`,
or else the nearest `roko.toml` above the working directory. To use a demo
config, set the variable:

```sh
ROKO_CONFIG=plans/demos/demo-hello/roko.toml ./target/debug/roko plan run plans/demos/demo-hello --approval --fresh
```

`ROKO_CONFIG` replaces the workspace `roko.toml` instead of layering on it, so
providers and models then come only from your global config and API-key
environment. Without it, the workspace `roko.toml` and its gates apply.

## Run in the portal

`roko serve` from the repository root lists the demos in the plan rail under
three groups: `demos`, `demos/hello-color` and `demos/parallel-plans`. Select a
plan and run it, or press a group's ▶ to run only that group. The server uses
the workspace `roko.toml`; its `[conductor] max_parallel_plans` (default 1)
decides whether the `parallel-plans` group runs both plans at once.

For a clean demo, give the portal a workspace of its own and copy in the plans
you want, then open the `portal:` link that `roko serve` prints:

```sh
REPO=<roko repo>; ROKO="$REPO/target/debug/roko"
WS=~/dev/roko-scratch/demo; mkdir -p "$WS/plans" && cd "$WS"
git init -q -b main && "$ROKO" init && printf '.roko/\ntarget/\n' >.gitignore
cp -R "$REPO/plans/demos/parallel-plans" plans/
git add -A && git commit -qm demo
"$ROKO" serve
```

To run both `parallel-plans` plans at once there, add `max_parallel_plans = 2`
under `[conductor]` in the workspace's `roko.toml`.

To run without a model, copy `plans/portal-programme/_harness/fake-claude`
into the workspace, set `command` under `[providers.claude_cli]` in its
`roko.toml` to that file's absolute path, and set `max_review_cycles = 0` under
`[gates]`. The fake agent writes each file named after `ARTIFACT` in a prompt,
and a `.rs` file gets a hello-world program. `demo-hello-world` and
`demo-print-hello` pass with it; the other demos check real content, so their
verify steps fail. The untracked local helper `tmp/try-portal.sh --fake` builds
such a workspace.

## Keeping demos out of whole-tree runs

- The demos stay visible so that `roko plan list` and the portal can run them.
  As a result, `roko plan run plans/` and the portal's Run all load them too,
  as they did when they sat directly under `plans/`. roko has no marker that
  keeps a listed plan out of a whole-tree run, so name the plan or plan set you
  want.
- `status = "fixture"` marks a demo as an example: `plans/INDEX.md` leaves
  fixture plans out of the backlog totals. That index reads only top-level
  plan directories, so it no longer lists the demos; this file is their
  catalogue.
- `_fixtures/` starts with `_`. Plan discovery skips directories whose names
  do not start with a letter or digit, as it skips `archive/` and `_meta/`, so
  the fixtures never appear in `roko plan list`, the portal or a whole-tree run.

## Fixtures

`_fixtures/` holds plans left behind by manual portal and API tests on
2026-09-24 and 2026-09-25, kept byte for byte. They are inputs for generation
and validator tests, not demos. A `tasks.toml` that the current parser rejects
is stored as `tasks.invalid.toml`, because CI validates every tracked
`tasks.toml` (`.github/workflows/plan-validate.yml`).

| Fixture | Contents | Use |
|---|---|---|
| `flow-test-gen`, `flow-test-gen2`, `flow-test-gen3` | `plan.md` with the prompt `add error handling` (identical apart from the name) | Generation input |
| `flow-gen-live` | `plan.md` with the prompt `add logging to errors` | Generation input |
| `audit-gen`, `audit-gen2` | `plan.md` with the prompt `test` (identical apart from the name) | Generation input |
| `test-plan-generation` | `plan.md` with the prompt `test plan generation` | Generation input |
| `audit-test`, `modal-test`, `test-check` | A one-line `plan.md` and a `[[tasks]]` table without `title` or `role` | Rejected with PLAN_002 and PLAN_034 |
| `test` | `tasks = []` | Rejected with PLAN_002 and PLAN_034 |
| `flow-test-exec` | `task = []` with `meta.total = 1` | Rejected with PLAN_002 and PLAN_BUDGET_TOTAL |

To see the validator reject one:

```sh
d=$(mktemp -d) && cp plans/demos/_fixtures/modal-test/tasks.invalid.toml "$d/tasks.toml"
./target/debug/roko plan validate "$d"
```

To generate a plan from a prompt, run
`roko plan generate --from-file <fixture>/plan.md` in a scratch workspace. In
this repository, prompts such as `add error handling` describe work across the
whole codebase.

## Where the old plans went

| Before | Now |
|---|---|
| `plans/demo-hello`, `demo-parallel-integration`, `demo-resume-recovery`, `demo-multistage`, `demo-incident-tabletop`, `demo-full-stack` | `plans/demos/` under the same names |
| `plans/archive/demo-release-readiness` | `plans/demos/demo-release-readiness`, minus two `read_files` line ranges that ran past the end of their files |
| `plans/hello-world` (a prompt only) | `plans/demos/parallel-plans/demo-hello-world` |
| `plans/scratch-test` | `plans/demos/parallel-plans/demo-print-hello` |
| `plans/test-fibonacci` | `plans/demos/demo-fibonacci` |
| `plans/example-hello-world` (`tasks = []`) | `plans/demos/hello-color/demo-hello-color` |
| `plans/test-greeting-module` | `plans/demos/hello-color/demo-greeting-module` |
| The other twelve leftovers | `plans/demos/_fixtures/` under the same names |

The five converted plans each record their original prompt and tasks in an
Origin section of their `plan.md`. A plan's id is its leaf directory name, so
the moved demos keep their ids: API paths such as `/api/plans/demo-hello` and
checkpoints under `.roko/state/graph/demo-hello/` still apply.

## Adding a demo

- Name the directory `demo-<something>`. Plan ids are leaf directory names and
  must be unique under `plans/`; a duplicate makes discovery fail for every
  plan.
- In `[meta]`, set `status = "fixture"` and `skip_enrichment = true`.
- Write only under `demo/<name>/`. Give tasks that write files the
  `implementer` or `scribe` role; the other roles cannot write files.
- Name a file that another plan creates in the task description, not in
  `read_files`: `read_files` must exist when the plan loads.
- Check it with `roko plan validate --strict plans/demos/<name>`.
