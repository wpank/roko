+++
id = "gap-b312c7"
kind = "gap"
title = "A live, read-only view of a run started elsewhere: roko plan status <dir> --follow redraws the plan's status in place from the run's records and event index until the run ends"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "M"
subsystem = ["roko-cli/commands"]
created = 2026-11-13
updated = 2026-11-13
last_verified = 2026-11-13
source = "round v8 roko-recovery finding (phase 7's terminal capture had to poll plan status)"
discovered_from = "agent:roko-recovery (round v8)"
anchors = ["crates/roko-cli/src/plan_status_view.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

## Problem

roko has no live, read-only view of a run that another process started: roko serve, the Hermes plugin, the portal
or another terminal. During round v8's phase 7, the guide's terminal surface had to poll `roko plan status` every
2 s from a script (guide-v8's `term_capture.py`) to look live. The options today:

- `roko plan status` and `roko status` are single snapshots.
- `roko dashboard` is the older full-screen TUI, not the run view's style.
- The run view `roko run` draws (`run_view::RunView::start_replaying` can mirror a hub's events) only follows a run
  in its own process or serve's hub.

## Why it matters

Goal tooling. Watching a run from a second terminal is ordinary. The run view already knows how to draw from
events, and a run's event index (`.roko/events-by-run/<sha256(run id)>.jsonl`) is flushed on every lifecycle event.

## Where

`crates/roko-cli/src/plan_status_view.rs`, `crates/roko-cli/src/graph_execution/run_view.rs`, and the run index
reader in `roko-fs`'s `run_index.rs`.

## Plan

1. `roko plan status <dir> --follow`: redraw the plan's status view in place while its run goes, from the run's
   records and event index, read-only. Stop when the run ends, then print the summary and Next.
2. Keep the snapshot without `--follow`.

## Done when

`roko plan status <dir> --follow` follows a run started by serve to its end and writes nothing. Not this round
(the lead, round v8).
