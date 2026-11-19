+++
id = "bug-5eb605"
kind = "bug"
title = "roko-serve's trigger test cancel_running_and_disable_reload_cancel_active_flow races the 500 ms disk reloader: it disables its binding only in memory, so a reload can re-arm it before the submit"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-serve/triggers"]
created = 2026-11-06
updated = 2026-11-06
last_verified = 2026-11-06
source = "push to GitHub 2026-11-06: release/2026-11-06 pre-push checks (cargo test --workspace)"
anchors = ["crates/roko-serve/src/trigger_runtime.rs::cancel_running_and_disable_reload_cancel_active_flow", "crates/roko-serve/src/trigger_runtime.rs::start_disk_reloader"]
links = { depends_on = [], blocks = [], related = ["bug-904a1c"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "awk '/fn cancel_running_and_disable_reload_cancel_active_flow/,/^    }$/' crates/roko-serve/src/trigger_runtime.rs | grep -q save_all && cargo test -p roko-serve --lib trigger_runtime::tests::cancel_running_and_disable_reload_cancel_active_flow"
+++

## Problem

`roko-serve`'s `trigger_runtime::tests::cancel_running_and_disable_reload_cancel_active_flow` fails now and then:
`assertion failed: handle.submit(manual_event("replace", 3)).await.is_err()` at
`crates/roko-serve/src/trigger_runtime.rs:3273`. It failed once in a full `cargo test --workspace --no-fail-fast` of
`8904a1c49` (2026-11-06, scratch HOME, `CI=true`, 4 test threads, the release/2026-11-06 pre-push checks). The same
test binary then passed it 30 times of 30 alone (`--exact`) and in 14 runs of the whole roko-serve lib suite.
`trigger_runtime.rs` has not changed since `dd11b6700`.

The cause, from the code: `test_state` saves the test's bindings to the triggers dir enabled
(`TriggerBinding::save_all`), and `ensure_trigger_runtime` starts `start_disk_reloader`, which reads every binding
from disk every `RELOAD_INTERVAL` (500 ms) and reconciles them. The test disables `replace` only in memory
(`handle.reconcile(vec![binding])` with `enabled = false`), waits for the active flow to stop, then submits. When a
reload tick lands between that reconcile and the submit, the reloader puts the on-disk binding back: its signature
differs from the disabled one, so `reconcile` arms it again and the submit is admitted. The window is a few yields
on an idle machine and grows under load.

## Why it matters

As with bug-904a1c: a flaky test in a crate every Rust worker tests makes passing work look broken and costs a
rerun, and CI's Test + Clippy job runs the same suite.

## Where

`crates/roko-serve/src/trigger_runtime.rs`: the test, its `test_state` helper (saves the bindings) and
`start_disk_reloader` (`RELOAD_INTERVAL`).

## Current state

Open at `8904a1c49`. Nothing in the coordinator is wrong: the reloader does what it should, re-reading the
bindings on disk.

## Plan

Disable the binding the way a reload would see it: save it to the triggers dir with `enabled = false`
(`TriggerBinding::save_all(&state.layout.triggers_dir(), ...)`) before `handle.reconcile`, so a reload tick reads
the same disabled binding. Check the other tests that reconcile in memory over `test_state`'s on-disk bindings for
the same race.

## Done when

The test saves its disabled binding to disk before it submits, and the roko-serve lib suite passes 20 runs in a row
while another crate builds.

## Notes

Found while running the pre-push checks for release/2026-11-06; reported in that PR.
