# Demo plans

Plans for showing roko to people and for exercising one feature at a time.
Each one passes `roko plan validate` (the larger ones draw `PLAN_TIER_SIZE`
warnings under `--strict`), writes only under `demo/`, and has
`status = "fixture"` in its `[meta]`.

**Start with [`demo-hello-world`](parallel-plans/demo-hello-world/)**, from
Hermes, the portal or the terminal: one task, about a minute, under a cent on
the default model ladder (see "Run from Hermes" below).

## Catalogue

| Plan | What it shows | Tasks | Expected result | Agent and cost |
|---|---|---|---|---|
| [`parallel-plans/demo-hello-world`](parallel-plans/demo-hello-world/) | **Start here.** A one-file Rust program; the free portal walkthrough | 1: implementer | `demo/hello-world/main.rs` compiles and prints `hello world` | Fake agent (free, seconds), or a real model: $0.005, 46 s from Hermes on 2026-10-15 |
| [`demo-hello`](demo-hello/) | The smallest real run: one task, one verify step, TUI approval | 1: implementer | `demo/roko-plan-smoke-output.md` with the three required lines, the last dated | Real model; $0.012, 42 s from Hermes on 2026-10-15 |
| [`parallel-plans/demo-print-hello`](parallel-plans/demo-print-hello/) | The second, independent plan of the `parallel-plans` set | 1: implementer | `demo/print-hello/main.rs` prints a line starting with `hello` | Fake agent, or a real model: $0.004, 49 s from Hermes on 2026-10-15 |
| [`demo-fibonacci`](demo-fibonacci/) | Implementer-to-scribe hand-off on a unit-tested program | 2: implementer, scribe | `demo/fibonacci/` prints `0 1 1 2 3 5 8 13 21 34`, its tests pass, and it has a README | Real model; $0.019, 59 s from Hermes on 2026-10-15 |
| [`demo-parallel-integration`](demo-parallel-integration/) | Two sibling tasks run at once, then a join task reads both | 3: implementer | `left.json`, `right.json` and `combined.json` (total 42) under `demo/parallel-integration/` | Real model; $0.016, 37 s from Hermes on 2026-10-15 (outputs checked in, so little to do) |
| [`_cli/demo-resume-recovery`](_cli/demo-resume-recovery/) | A run stops after task 1 and a second process resumes it; terminal only | 2: implementer | The first run stops at 1/2; the resumed run completes with both stage files | Real model; $0.46 on 2026-09-19 |
| [`hello-color/demo-hello-color`](hello-color/demo-hello-color/) | Five tasks with two roles, and two pairs of tasks running in parallel | 5: scribe, implementer | The `demo/hello-color/` crate prints a coloured greeting, with research, review and README files | Real model; $0.017, 2 min from Hermes on 2026-10-15 (crate checked in, so little to do) |
| [`hello-color/demo-greeting-module`](hello-color/demo-greeting-module/) | A plan that depends on another plan (`depends_on_plan`) | 1: implementer | `src/greeting.rs` with a passing unit test; the program's output is unchanged | Real model; not recorded |
| [`demo-multistage`](demo-multistage/) | A five-stage non-code workflow ending in a read-only review | 5: scribe, implementer, quick-reviewer | `sh demo/multistage-plan/validate.sh` prints `multi-stage demo: PASS` | Real model; $0.50 on 2026-09-21 |
| [`demo-incident-tabletop`](demo-incident-tabletop/) | Repository evidence turned into a timed incident exercise | 4: scribe, implementer, quick-reviewer | Risk register, scenario and facilitator guide under `demo/incident-tabletop/` | Real model; $2.03 on 2026-09-21 |
| [`demo-release-readiness`](demo-release-readiness/) | A release go/no-go decision run as an evidence workflow | 4: scribe, implementer, quick-reviewer | `inventory.json`, `go-no-go.md` and `check.sh` under `demo/release-readiness/` | Real model; $0.064, 4 min from Hermes on 2026-10-15 |
| [`demo-full-stack`](demo-full-stack/) | Six tasks build, audit and document an axum web server | 6: scribe, implementer | `demo/hello-server/` builds and serves `/`, `/health` and `/info`; `COMPLETION.md` signs off | Real model; no recorded run; its builds fetch crates |

Costs are what the run recorded, with whatever model it used. The 2026-10-15
runs were started from Hermes' Plans tab in a clone of this repository, on
the workspace ladder: `cerebras-gptoss` for mechanical and focused tasks,
`glm-4-7` for integrative ones.

Four demos read this repository's own files (`demo-multistage`,
`demo-incident-tabletop`, `demo-release-readiness`, `demo-full-stack`), so they
only run here. The others also run in an empty scratch workspace. The
`hello-color` set and `demo-full-stack` build with Cargo, which fetches their
crates on first use.

### Outputs and reruns

A demo's output belongs on its run's branch, not on yours. When an output is
checked in, the demo's checks pass before any agent has run, and roko's spec
gate refuses such a plan (`HF3: verify passes on the unchanged base`); that is
why `demo-hello`'s last line carries the run's UTC date, and why the outputs
old runs committed for `demo-release-readiness` and `demo-resume-recovery` are
gone. Some older outputs are still checked in (`parallel-integration`,
`incident-tabletop`, `multistage-plan`, `hello-color`): those demos run, but
their agents find little to change. To watch the work happen from nothing, use
a scratch workspace (see below) or delete the output directory first.

### Whole-plan checks

Every demo that writes a `.rs` file has its own `[meta] verify`, which checks
the demo's output once its tasks have passed. Without one, a plan in a Cargo
workspace gets roko's default whole-plan check (fmt, clippy and tests of the
packages it may touch). In this repository that means a cold build of the
whole workspace, because roko cannot read the package graph offline here
(bug-a7e018).

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
resumes at stage two. It lives under `_cli/`, which plan discovery skips,
because only a terminal can set the variable its second run needs; name its
directory to run it:

```sh
./target/debug/roko plan run plans/demos/_cli/demo-resume-recovery --fresh --max-retries 0
ROKO_RESUME_DEMO_READY=1 ./target/debug/roko plan run plans/demos/_cli/demo-resume-recovery
```

## Run from Hermes

With the roko plugin, Hermes' Plans tab lists every plan `roko serve` finds,
the demos among them, and starts one with a spending cap ("Run plan…").

- The workspace must let a chat host start runs: `[serve.mcp]
  allow_without_data_llm = true` or an `[agent.data_llm]` table in
  `roko.toml`. Without either, roko refuses every run Hermes starts ("a run
  from a chat host needs the data-model boundary"). roko serve reads
  `roko.toml` per run, so the change needs no restart.
- A run Hermes starts may not use a CLI agent (Claude Code, Codex and the
  like), so the ladder's API rungs do the work; the demos above ran on
  `cerebras-gptoss` and `glm-4-7`.
- Start with `demo-hello-world`. A $0.25 cap is plenty for the one-task demos
  and `demo-fibonacci`, `demo-parallel-integration` and the `hello-color` set;
  `demo-release-readiness` ran for $0.06.
- Each run leaves its output on its own branch, `roko/batch/<run>`, and the
  run card offers to merge it into your current branch. For a demo, look at
  the diff and leave it there: merged demo output is junk on your branch, and
  it makes the next run of that demo pass before any agent starts, which roko
  refuses.

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
- `_cli/` holds demos that only a terminal can run (`demo-resume-recovery`).
  Discovery skips it the same way, so Hermes and the portal do not offer them;
  `roko plan run plans/demos/_cli/<demo>` still runs one.

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
| `plans/demo-hello`, `demo-parallel-integration`, `demo-resume-recovery`, `demo-multistage`, `demo-incident-tabletop`, `demo-full-stack` | `plans/demos/` under the same names (`demo-resume-recovery` since 2026-10-15 under `plans/demos/_cli/`) |
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
- Do not check the demo's output in, and make every implementer's checks fail
  on the unchanged repository: roko refuses a plan whose checks already pass
  (check with `roko plan validate --spec-quality --dynamic plans/demos/<name>`).
- If a task writes Rust, give the plan a `[meta] verify` that checks the
  demo's own output. Whole-plan checks run under `sh`, so use POSIX syntax
  there (no `[[` or `<<<`).
- Name a file that another plan creates in the task description, not in
  `read_files`: `read_files` must exist when the plan loads.
- Check it with `roko plan validate --strict plans/demos/<name>`.
