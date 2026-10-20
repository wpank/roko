+++
id = "bug-06ac5f"
kind = "bug"
title = "plan import and doctor report a stopped roko serve as running: serve_holding_workspace trusts the PID in roko.lock and serve.json with no liveness or lock check"
status = "open"
triage = "unverified"
severity = "p3"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/doctor"]
created = 2026-10-18
updated = 2026-10-18
source = "round-v6-dogfood-desktop#L5-mori"
discovered_from = "agent:dogfood-desktop (round v6 showcase take L5, 2026-10-18)"
anchors = ["crates/roko-cli/src/doctor.rs::serve_holding_workspace", "crates/roko-cli/src/workspace_lock.rs::workspace_lock_owner_pid"]
links = { depends_on = [], blocks = [], related = ["bug-dfe4c5"], supersedes = [], duplicate_of = "" }
+++

## Problem

`roko plan import` (and `roko doctor`) say a roko serve "runs in this workspace" when it has stopped.
`doctor::serve_holding_workspace` takes the PID written in `.roko/runtime/roko.lock` and the one in
`.roko/runtime/serve.json`, and when they match it reports that serve as holding the workspace. It never checks
that the process is alive or that it still holds the lock. A serve that ends without cleaning up leaves both files
behind. Examples: a TERM it doesn't finish handling, or the terminal it was started from closing.

Seen in round v6's showcase take L5 (2026-10-18, roko cc69556c2) on pocket-sequencer. The showcase fixture had
stopped its roko serve (PID 3978) before the take. `roko plan import --mori .mori/plans/pocket-sequencer` then
printed "roko serve (PID 3978) runs in this workspace and lists the new plans", and 3978 was not running a minute
later. The take next started `roko serve --port 6811` itself (PID 6182), which bound the port without trouble, so
nothing held it.

After the take closed the terminal, `roko.lock` and `serve.json` still name 6182, which is not running, so the next
import will say the same thing again.

## Why it matters

The import's line tells the person their plans are already listed by a running serve. They then don't start one,
and the page stays offline. On camera, the import says serve runs, and the next command starts it.

## Where

- `crates/roko-cli/src/doctor.rs::serve_holding_workspace`
- `crates/roko-cli/src/workspace_lock.rs::workspace_lock_owner_pid` (reads the PID, with no liveness or lock test)
- `crates/roko-cli/src/commands/plan.rs`, where the import prints the line

## Current state

The PID is only read from files.

## Plan

Report a serve only when its PID is alive and still holds the workspace lock (try the lock, as the exclusive lock
holder check does), or when its endpoint answers `/health`. Otherwise say nothing, or say that a stale endpoint was
left.

## Done when

With `roko.lock` and `serve.json` naming a PID that isn't running, `roko plan import` and `roko doctor` don't
report a running serve.

## Notes

- Evidence: `tmp/hermes-roko-v6/media/pipeline/work/takes/l5-mori-0044` (the terminal frames around 6-9 s) and
  `/Users/Shared/pocket-sequencer/.roko/runtime/`.
