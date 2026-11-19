+++
id = "gap-407f38"
kind = "gap"
title = "roko learn rungs and roko learn patterns print their old formats, not the learn views' style"
status = "done"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "M"
subsystem = ["roko-cli/commands"]
created = 2026-11-19
updated = 2026-11-19
last_verified = 2026-11-19
last_verified_rev = "da0196aec"
source = "docs sweep on branch work/readme-v8, checked with roko at befee2f9b (2026-11-19)"
anchors = ["crates/roko-cli/src/commands/learn.rs::cmd_learn_rungs", "crates/roko-cli/src/commands/learn.rs::cmd_learn_patterns", "crates/roko-cli/src/learn_view.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-cb1a87"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'println!(\"Rungs over' crates/roko-cli/src/commands/learn.rs && ! grep -q 'format_for_prompt(20)' crates/roko-cli/src/commands/learn.rs && grep -rqw 'fn learn_rungs_and_patterns_read_as_learn_views' crates/roko-cli/ && cargo test -p roko-cli learn_rungs_and_patterns_read_as_learn_views"

[closed]
at = 2026-11-19
at_ts = "2026-11-19T15:24:25Z"
commit = "da0196aec"
executor = "claude-agent"
size = "M"
claimed_at = "2026-11-19T14:03:28Z"
forced = false
evidence = "da0196aec on work/gap-407f38 (with 56322ee2f). roko learn rungs, roko learn patterns and patterns --graduate render through learn_view (rungs_lines, patterns_lines, graduation_lines, read_from) like the other learn views: the ◆ roko header and its context, a part per rung or pattern with the facts the old formats gave (rungs: model and role, tries and verified passes, median time a try and per verified pass, share on the model, cost a try and per verified pass, tries without a price, time-bound stops, refused calls, and the ladder's finding as a closing paragraph; patterns: the 20 most frequent unresolved, digest, class, times and plans, last seen, the check and key unless the digest says them, fix and hint one line each on a terminal cut at a word; --graduate: the fix whole and the suggested check), the files read on the closing line, Next to --graduate when a pattern is ready; no prompt text; --json unchanged. Red first: learn_rungs_and_patterns_read_as_learn_views (tests/learn_paths_fixture.rs, a run written by roko's telemetry writer and a pattern file from the store) failed on the old 'Rungs over 1 run(s):' header before the wiring; it passes now. rungs_and_patterns_lay_out_like_the_other_learn_views pins both layouts exactly. learn_view lib 2, bin learn 50, learn_paths_fixture 3, learn_paths 7, reflexes_cli 3; workspace clippy clean; the three views checked on a copy of Undertow's 32 runs and 20 patterns, in a pipe and at 80 columns."
+++

## Problem

The `roko learn` views that gap-cb1a87 restyled print in roko's design system. Each has a `◆ roko  learn <view>`
header, a line on what it shows, labelled parts and a closing "Read from …" line (`crates/roko-cli/src/learn_view.rs`).
Two views were left in their old formats.

- `roko learn rungs` prints "Rungs over 32 run(s):" and then one long line per rung, which wraps mid-word on a
  100-column terminal. For example: "Rung mid (gpt-5.4-mini, implementer): 56 tries, 37 verified passes, median 2.3 min
  a try, 102 s per verified pass, 35% on the model, $0.13 a try, $0.20 per verified pass".
- `roko learn patterns` prints the text roko puts into prompts: "## Prior Verify Failure Patterns", "Use these concise
  prior failures as constraints; do not treat them as full logs.", then numbered entries with raw "Fix:" excerpts cut
  off mid-word.

Both were seen on a real build's records (Undertow) with the binary at befee2f9b on 2026-11-19.

## Why it matters

`roko learn all`, `router`, `reflexes` and `probes` read one way and these two another. `patterns` shows a person the
prompt text meant for a model. README.md and the CLI reference list both views (branch work/readme-v8).

## Where

- `crates/roko-cli/src/commands/learn.rs::cmd_learn_rungs`: prints `RungReport::lines()` under "Rungs over N run(s):".
- `crates/roko-cli/src/commands/learn.rs::cmd_learn_patterns`: prints `ErrorPatternStore::format_for_prompt(20)`.
  Its `--graduate` branch has its own plain format.
- `crates/roko-cli/src/learn_view.rs`: the shared helpers (`section`, `paragraph`, `until`) the other views use.

## Current state

gap-cb1a87 (done) restyled the four views it named: all, loops, self-model fit and probes list. `rungs` and `patterns`
were outside its scope. `--json` output exists for both and stays as it is.

## Plan

1. `rungs`: render through `learn_view`.
   - A header and a context line ("how each rung of the model ladder did, over N runs").
   - A part per rung: model and role, tries and verified passes, time and cost per try and per verified pass.
   - The "Read from …" line.
2. `patterns`: render through `learn_view`.
   - A part per pattern: its key, how often and in how many plans, when it was last seen, the check that failed and a
     one-line digest, not the prompt text.
   - Give `--graduate` the same look.
3. Add `learn_rungs_and_patterns_read_as_learn_views`, a test over a small fixture that checks both views' header, their
   parts and their "Read from" line, as `learn_paths_fixture` does for the others.

## Done when

The `[[verify]]` command passes: neither old renderer is left, and the named test passes.

## Notes

Keep the facts each view gives today, and keep `--json` unchanged.
