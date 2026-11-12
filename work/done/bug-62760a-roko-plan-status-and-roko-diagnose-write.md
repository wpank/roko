+++
id = "bug-62760a"
kind = "bug"
title = "roko plan status and roko diagnose write the checkout's .git/index: the git status --porcelain behind the report's git state refreshes the index's stat cache and writes it back under index.lock; a view never writes (bug-399240)"
status = "done"
triage = "verified"
severity = "p1"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/commands"]
created = 2026-11-12
updated = 2026-11-13
last_verified = 2026-11-13
last_verified_rev = "85799278f"
source = "round v8 lead: plan status rewrote .git/index on a clone of Undertow's workspace (guide-v8's check)"
discovered_from = "agent:roko-recovery (round v8)"
anchors = ["crates/roko-cli/src/commands/diagnose.rs::run_git", "crates/roko-cli/src/commands/diagnose.rs::collect_git_state"]
links = { depends_on = [], blocks = [], related = ["bug-2e5023", "bug-399240"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn plan_status_writes_nothing' crates/roko-cli/src && cargo test -p roko-cli --lib plan_status_writes_nothing"

[closed]
at = 2026-11-13
at_ts = "2026-11-12T23:00:58Z"
commit = "85799278f"
by = "roko-recovery"
executor = "claude-agent"
size = "S"
claimed_at = "2026-11-12T22:17:37Z"
forced = false
evidence = "85799278f on work/bug-2e5023: diagnose::run_git passes --no-optional-locks, so the report's git status no longer refreshes the stat cache and rewrites .git/index (roko plan status and roko diagnose). Red first: plan_status_writes_nothing (stale stat cache for tasks.toml, every byte under the workspace compared) failed on bd49800b5 with 'plan status wrote [\".git/index\"]', now passes. On an APFS clone of ~/dev/undertow without GIT_OPTIONAL_LOCKS, roko-item7d changed .git/index (9f3d15633a4f56b7 -> d1c75206c362ef59); roko-item8a ran plan status for phases 1-6 and diagnose p6 and left it at 9f3d15633a4f56b7. Branch checks: roko-cli lib 4149 passed; bin 21 passed; clippy clean."
+++

## Problem

`roko plan status` isn't read-only. On a clone of Undertow's workspace it rewrote `.git/index` (round v8 lead,
guide-v8's check). Its report runs `diagnose::collect_git_state`, and through `run_git` that runs
`git status --porcelain`. When the stat data the index holds for a file is stale (its time moved and its bytes
didn't), `git status` refreshes the stat cache and writes the index back, taking `.git/index.lock` to do it. It
does this whenever it can get that optional lock. `roko diagnose` runs the same code.

## Why it matters

Goal tooling. A view must never write (bug-399240's rule). The write is in the person's own checkout, under
`index.lock`. A `roko plan status` that polls during a run can then make the person's own git commands fail with
"Unable to create '.git/index.lock': File exists". Watching a run must change nothing.

## Where

`crates/roko-cli/src/commands/diagnose.rs::run_git`, used by `collect_git_state` (rev-parse, status, branch --list).
The other git calls on the plan status path, `batch::merge_command`'s `merge-base --is-ancestor` and
`graph_checkpoint`'s `cat-file`, don't take optional locks.

## Plan

1. Red test first: a plan whose checkout has a stale stat cache for one file. `roko plan status` leaves every byte of
   the workspace as it was, `.git/index` included.
2. `run_git` runs git with `--no-optional-locks`, as `run_manifest.rs`, `checkout_state.rs` and serve's
   `run_result.rs` already do.

## Done when

The test passes and the `[[verify]]` passes.
