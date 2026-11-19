+++
id = "gap-d2b605"
kind = "gap"
title = "docs/v3/28-CLI.md leaves out flags of roko run, roko plan run and roko plan generate"
status = "done"
triage = "verified"
severity = "p3"
goal = "release"
size = "S"
subsystem = ["docs"]
created = 2026-10-27
updated = 2026-10-27
last_verified = 2026-10-27
last_verified_rev = "20b475a5f"
source = "gap-171238 (2026-10-27)"
discovered_from = "gap-171238"
anchors = ["docs/v3/28-CLI.md"]
lane = "docs"
links = { depends_on = [], blocks = [], related = ["gap-171238"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "for f in '--yes`, `-y' --no-holdout --no-audit; do sed -n '/^### `roko run`$/,/^#/p' docs/v3/28-CLI.md | grep -qF \"| \\`$f\" || exit 1; done && for f in --no-holdout --no-audit --max-parallel-plans --fail-fast; do sed -n '/^#### `roko plan run`$/,/^#/p' docs/v3/28-CLI.md | grep -qF \"| \\`$f\" || exit 1; done && sed -n '/^#### `roko plan generate`$/,/^#/p' docs/v3/28-CLI.md | grep -qF '| `--max-usd'"

[closed]
at = 2026-10-27
at_ts = "2026-10-27T09:17:15Z"
commit = "20b475a5f"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-27T08:30:56Z"
forced = false
evidence = "docs/v3/28-CLI.md gains rows, written from the clap fields and the code behind them, for --no-holdout, --no-audit and -y in roko run, for --no-holdout, --no-audit, --max-parallel-plans and --fail-fast in roko plan run, and for --max-usd in roko plan generate; it also corrects rows of those tables the code contradicts (--complexity values, --dry-run on a plan directory, --approval, --no-budget and --budget-override 0, <source...> and --from-file). The section-scoped [[verify]] passes at 20b475a5f; it fails on the previous text and on a doc with the --no-holdout row under roko plan run alone, which the old verify let pass."
+++

## Problem

The flag tables and synopses of `docs/v3/28-CLI.md` leave out flags that the clap definitions have:

- `roko run` (`crates/roko-cli/src/main.rs::Command::Run`): `--no-holdout` (maximize mode for this
  run alone, decision 4115) and `--no-audit` (no green attempt drawn for a deep audit in this run,
  decision 7103); also the short form `-y` of `--yes`.
- `roko plan run` (`crates/roko-cli/src/commands/plan.rs::PlanCmd::Run`): `--no-holdout`,
  `--no-audit`, `--max-parallel-plans <N>` (1 to 64 plans at once) and `--fail-fast` (start no
  further plans after the first one fails).
- `roko plan generate` (`PlanCmd::Generate`): `--max-usd <USD>` (stop writing a plan once its
  planner calls cost this much; default `[authoring] max_generation_usd`, 0 for no cap).

How to see it: `grep -nE -- '--(no-holdout|no-audit|max-parallel-plans|fail-fast|max-usd)' docs/v3/28-CLI.md`
finds only the prose mention of `--no-audit` in the `roko audit` section that gap-171238 added; no
flag table has a row for any of them.

## Why it matters

The CLI reference is where an operator looks a flag up (as in gap-171238). `--no-audit` and
`--no-holdout` change what a run spends and what it learns; `--max-parallel-plans` and
`--fail-fast` change how a plan set runs.

## Where

- `crates/roko-cli/src/main.rs::Command::Run`: the `yes`, `no_holdout` and `no_audit` fields and
  their doc comments.
- `crates/roko-cli/src/commands/plan.rs::PlanCmd`: `Run` (`no_holdout`, `no_audit`,
  `max_parallel_plans`, `fail_fast`) and `Generate` (`max_usd`).
- `docs/v3/28-CLI.md`: §`roko run`, §`roko plan run`, §`roko plan generate`.

## Current state

Checked 2026-10-27 at 0191f7858 by comparing those clap fields with the three sections.

## Plan

Add each flag to its command's synopsis and flag table, written from its doc comment. While there,
compare the other flag tables with `--help` of a built binary.

## Done when

Each of the five flags has a row in a flag table of 28-CLI.md, and `[[verify]]` passes. The verify
reads each section on its own: `--yes`, `-y`, `--no-holdout` and `--no-audit` in §`roko run`;
`--no-holdout`, `--no-audit`, `--max-parallel-plans` and `--fail-fast` in §`roko plan run`;
`--max-usd` in §`roko plan generate`.

## Notes

Docs only. Found while doing gap-171238.

2026-10-27: the first verify passed with a row for each flag anywhere in the file, so a row for
`--no-holdout` and `--no-audit` under `roko plan run` alone would have satisfied it, and it did
not check `-y`. It now reads each section from its heading to the next heading, and is still a
static check.
