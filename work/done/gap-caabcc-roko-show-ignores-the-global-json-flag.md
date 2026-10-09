+++
id = "gap-caabcc"
kind = "gap"
title = "roko show ignores the global --json flag"
status = "done"
triage = "verified"
severity = "p2"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/commands"]
created = 2026-10-08
updated = 2026-10-08
last_verified = 2026-10-08
last_verified_rev = "ad7a3a337"
source = "stash-triage-2026-10-08 (roko-worktree-archive/2026-10-08-stash-triage/TRIAGE.md)"
discovered_from = "archive/stash-2026-09-21-main-2"
anchors = ["crates/roko-cli/src/commands/show.rs::cmd_show"]
links = { depends_on = [], blocks = [], related = ["bug-194c8f", "bug-331613"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'fn render_json' crates/roko-cli/src/commands/show.rs && cargo test -p roko-cli --bin roko show_json_"

[closed]
at = 2026-10-08
at_ts = "2026-10-08T13:21:52Z"
commit = "3a32cf329"
forced = false
evidence = "gate 24 (2026-10-08): its verify passes; check, nightly fmt, clippy -D warnings, CI feature checks, nextest --workspace --lib (15223 passed), touched crates' full tests, roko-acp integration, roko-cli bin (457) and CI canaries all pass"
+++

## Problem

`roko show [subject] --json` printed the text view: `cmd_show` (`crates/roko-cli/src/commands/show.rs`) never read
the global `--json` flag, so a script got aligned text instead of JSON for every subject (overview, costs, agents,
knowledge, plans, learning, history, and a work-item id).

## Why it matters

`show` is the read-only inspection command that scripts and the portal can use; a flag that is accepted and
silently ignored is the pattern parked bug-194c8f and bug-331613 describe for ~15 commands. This item fixes `show`.

## Where

`crates/roko-cli/src/commands/show.rs::cmd_show` and the renderers next to it.

## Current state

The 2026-09-21 stash `archive/stash-2026-09-21-main-2` carried JSON renderers for `show` that were never committed
(triage: `roko-worktree-archive/2026-10-08-stash-triage/salvage-03-show-json.patch`). They predate the `--since`
window.

## Plan

Branch on `cli.json` after `load_show_state`: one JSON object per view over the same data and the same `--since`
window as its text, with `subject`, `workdir` and `window` at the top level.

## Done when

- [ ] `roko show <subject> --json` prints one JSON object for every subject and for a work-item id.
- [ ] `[[verify]]` passes.
