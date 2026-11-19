+++
id = "bug-a1c99a"
kind = "bug"
title = "A fresh roko init leaves .roko/engrams.jsonl and .roko/plans/, so roko doctor warns about both, and its fix is roko init"
status = "done"
triage = "verified"
severity = "p2"
goal = "release"
size = "M"
subsystem = ["roko-cli/init"]
created = 2026-11-19
updated = 2026-11-19
last_verified = 2026-11-19
last_verified_rev = "b395e783c"
source = "docs sweep on branch work/readme-v8, checked with roko at befee2f9b (2026-11-19)"
anchors = ["crates/roko-cli/src/commands/util.rs::cmd_init", "crates/roko-fs/src/layout.rs::top_level_dirs", "crates/roko-cli/src/doctor.rs::check_plans_dir_conflict", "crates/roko-fs/src/file_substrate.rs::open"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn fresh_init_leaves_no_legacy_files' crates/roko-cli/ && cargo test -p roko-cli --test doctor -- fresh_init_leaves_no_legacy_files reinit_keeps_the_signal_log init_moves_a_legacy_signal_log_into_signals_jsonl && cargo test -p roko-cli --lib -- plans_dir_conflict_counts_only_plans && d=\"$(mktemp -d)\" && git -C \"$d\" init -q && mkdir \"$d/h\" && cargo build -q -p roko-cli && HOME=\"$d/h\" target/debug/roko init \"$d\" >/dev/null 2>&1 && test ! -e \"$d/.roko/engrams.jsonl\" && test ! -d \"$d/.roko/plans\""

[closed]
at = 2026-11-19
at_ts = "2026-11-19T13:40:59Z"
commit = "b395e783c"
executor = "claude-agent"
size = "M"
claimed_at = "2026-11-19T12:58:48Z"
forced = false
evidence = "b395e783c on work/bug-a1c99a. Confirmed first: in a workspace with signals.jsonl and no engrams.jsonl, roko init moved every Signal row into engrams.jsonl and deleted signals.jsonl (reinit_keeps_the_signal_log red: 'init moved the signal log'); FileSubstrate::open reads engrams.jsonl only while signals.jsonl is absent and writes to signals.jsonl, so the next write hid the moved rows. Fix: init gives the workspace signals.jsonl (empty when it has no log) and never touches it otherwise; a legacy engrams.jsonl moves into it, its rows first and the newer signals.jsonl rows after, through a temporary file, which recovers a workspace an earlier init split and makes doctor's 'fix: roko init' true; .roko/plans/ left RokoLayout::top_level_dirs (init and doctor's layout check no longer need it; the enrichment pipeline makes .roko/plans/<plan>/ when it writes there); plans_dir_conflict warns only when .roko/plans/ holds a plan (dirs with tasks.toml or plan.md); docs/v3/01-SIGNAL.md and roko explain name signals.jsonl. Red first in tests/doctor.rs: fresh_init_leaves_no_legacy_files (init wrote engrams.jsonl) and reinit_keeps_the_signal_log; green with init_moves_a_legacy_signal_log_into_signals_jsonl (split log whole again, doctor legacy ok) and the unit test plans_dir_conflict_counts_only_plans. The [[verify]] names --test doctor instead of building every roko-cli test target. Checks: tests/doctor.rs 8 passed; lib doctor+init 76; bin init 12; roko-fs 257; tests/e2e.rs 1; workspace clippy clean; CLI check on a scratch repo: no .roko/engrams.jsonl, no .roko/plans/, signals.jsonl present."
+++

## Problem

A new user who follows the README's quick start, or the plugin's `hermes roko init`, gets two `roko doctor` warnings on
a workspace roko has just made.

1. **`state_legacy_files`:** "1 legacy storage file(s) detected … engrams.jsonl", with "fix: roko init". `roko init`
   is what wrote that file, so the suggested fix recreates the problem.
2. **`plans_dir_conflict`:** "both plans/ and .roko/plans/ exist … the canonical location is plans/". It fires as soon
   as the user puts a plan in `plans/`, as step 4 of the quick start does. `roko init` creates an empty
   `.roko/plans/`.

Reproduced with the binary at befee2f9b on 2026-11-19. In a scratch git repo with no keys, `roko init` was followed by
`cp -R plans/demos/parallel-plans/demo-hello-world plans/` and then `roko doctor`. The new `.roko/` held an empty
`engrams.jsonl` and an empty `plans/`.

A second effect, by reading the code (not reproduced):

- `cmd_init` still treats `engrams.jsonl` as the canonical log. In a workspace that has `signals.jsonl` but no
  `engrams.jsonl`, it moves every row that parses as a Signal from `signals.jsonl` into `engrams.jsonl`, and deletes
  `signals.jsonl` when every row moved.
- `FileSubstrate::open` replays `engrams.jsonl` only while `signals.jsonl` is absent, and writes go to
  `signals.jsonl`.
- So after the next write, the moved history is no longer replayed by `roko status --details` or `roko replay`.
- `roko init`'s own advice ("move roko.toml aside and run `roko init` again") and doctor's fix ("roko init") both lead
  there.

## Why it matters

This is the first-run path that the README, docs/v2 and the guides send new users down. A doctor that warns on a
workspace roko made, and points back at the command that made the warning, costs trust in the first minutes. The
re-init path can lose the signal history without a word.

## Where

- `crates/roko-cli/src/commands/util.rs::cmd_init`: the `engrams.jsonl` block. It creates an empty file, or migrates
  `signals.jsonl` into it.
- `crates/roko-fs/src/layout.rs::top_level_dirs`: it lists `plans_dir()` (`.roko/plans/`), which `ensure_dirs` creates
  on every init. `RokoLayout::plan_dir` says that directory holds per-plan enrichment artifacts, and it has one reader,
  `crates/roko-cli/src/commands/plan.rs:3340`.
- `crates/roko-cli/src/doctor.rs::check_plans_dir_conflict`: it counts every directory under `.roko/plans/` as a plan.
- `crates/roko-cli/src/doctor.rs`, the `state_legacy_files` check: it lists `engrams.jsonl` with `fix: roko init`.
- `crates/roko-fs/src/file_substrate.rs::open`: the read-only fallback to `engrams.jsonl`.

## Current state

`signals.jsonl` has been the signal log's canonical name for a while (CLAUDE.md: "a legacy `engrams.jsonl` is read only
as a fallback"), and `cmd_init` was never moved over.

## Plan

1. `cmd_init`: stop creating `engrams.jsonl`. If a workspace has only `engrams.jsonl`, migrate it the other way: into
   `signals.jsonl`, then remove the old file. Never move rows out of `signals.jsonl`.
2. `.roko/plans/`: create it when the enrichment step first writes there, not in `top_level_dirs()`. Have
   `check_plans_dir_conflict` count only directories that hold a `tasks.toml`, so enrichment artifacts are not taken for
   plans.
3. `state_legacy_files`: suggest a fix that removes the warning, such as the migration in step 1 or a manual step.
4. Add `fresh_init_leaves_no_legacy_files`. It should init a scratch repo, put a plan in `plans/`, and assert that
   `roko doctor` reports both checks as ok. Add a re-init test as well: a workspace with `signals.jsonl` keeps it after
   `roko init`.

## Done when

The `[[verify]]` command passes:

- a fresh `roko init` leaves no `.roko/engrams.jsonl` and no `.roko/plans/`;
- the named test passes, covering doctor's two checks and the re-init case.

## Notes

The read-only fallback in `FileSubstrate::open` stays for old workspaces. Check `roko demo setup` and the plugin's
`hermes roko init` (which runs `roko init` when the project has no `roko.toml`) after the change.
