+++
id = "bug-54880c"
kind = "bug"
title = "work.py claim and next can hang for minutes: worktree_changes() runs git status in every worktree with no timeout, and after the 2026-10-26 reboot git blocks on a unix socket (a stale fsmonitor) in another session's worktrees"
status = "done"
triage = "verified"
severity = "p2"
goal = "tooling"
size = "S"
subsystem = ["tools/work"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
last_verified_rev = "6f385718f"
source = "round-v7 roko-recovery's finding, via the lead (2026-10-26)"
discovered_from = "agent:roko-recovery (round v7)"
anchors = ["tools/work.py::worktree_changes", "tools/work.py::cmd_claim"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_a_worktree_whose_git_status_hangs_is_checked_as_unknown' tools/test_work.py && python3 tools/test_work.py TestWorktreeBusy.test_a_worktree_whose_git_status_hangs_is_checked_as_unknown"

[closed]
at = 2026-10-26
at_ts = "2026-10-26T12:15:31Z"
commit = "6f385718f"
by = "run-analyst (round v7)"
executor = "claude-agent"
forced = false
model = "claude-opus-5-5"
evidence = "6f385718f: worktree_status() runs git --no-optional-locks -c core.fsmonitor=false status with a 10 s timeout (WORK_GIT_STATUS_TIMEOUT_S overrides it); a worktree that times out keeps its committed files in the scan, is reported 'unknown (git status timed out after 10s)' on stderr, and claim names it for each item it claims. Verify passes: test_a_worktree_whose_git_status_hangs_is_checked_as_unknown fakes a git whose status sleeps 30 s; the scan returns at the 0.5 s timeout with that worktree unknown and its commit still counted, and claim's note names it; the test errors on the previous work.py. tools/test_work.py 46 tests OK; test_work_backfill, test_work_harvest, test_work_import_checklist and test_work_telemetry OK. A read-only scan of this repo's 94 worktrees (79 .claude/worktrees/agent-*) took 3.9 s, none timing out."
+++

## Problem

`python3 tools/work.py claim` and `next` can hang for minutes. `worktree_changes()` runs
`git -C <worktree> status --porcelain -z --no-renames` in every worktree of the repository, and the call has no
timeout.

roko-recovery found it on 2026-10-26, after the 10:57 reboot. In 13 of another session's `.claude/worktrees/agent-*`
worktrees, git blocks on a unix socket, probably a stale fsmonitor socket. One stuck worktree holds up every `next`
and every `claim`.

The call can also refresh the index of a worktree it only means to read, because it takes git's optional locks.

Expected:
- each worktree's status gets about 10 s, read without optional locks and without asking an fsmonitor daemon;
- a worktree that doesn't answer is reported as unknown on stderr, and its uncommitted files are not counted as
  clean;
- a claim says it couldn't check that tree, rather than passing silently.

## Why it matters

Tooling. Every agent's `/work-next`, `/work-batch` and `claim` goes through `worktree_changes()`, so one other
session's broken worktree stalls all of them. Treating an unreadable worktree as clean would let two workers take
overlapping files.

## Where

- `tools/work.py::worktree_changes`: the per-worktree scan, already run 16 at a time in a thread pool.
- `tools/work.py::cmd_claim`: uses the scan for its overlap check (`claim_conflicts`).
- `tools/work.py::pick_next`: uses the scan for `next`.

## Plan

1. Add a `worktree_status(path, timeout)` that runs
   `git --no-optional-locks -c core.fsmonitor=false -C <path> status --porcelain -z --no-renames` with a timeout of
   10 s by default (`WORK_GIT_STATUS_TIMEOUT_S` overrides it). It returns `None` on a timeout. `subprocess.run` kills
   the stuck git.
2. In `worktree_changes()`, a worktree whose status timed out still counts its committed changes, which come from a
   diff in this checkout. Its uncommitted files are reported as "unknown (git status timed out after N s)" on stderr,
   and the caller gets its label in an optional `unknown` list.
3. `claim` prints, for each item it claims, which worktrees it could not check.

## Done when

- A worktree whose `git status` hangs costs at most the timeout, and is reported unknown, not clean.
- `claim` names it.
- Test `test_a_worktree_whose_git_status_hangs_is_checked_as_unknown` passes, along with the rest of
  `tools/test_work.py`.

## Notes

- Never touch the stuck worktrees. The fix only reads them: `--no-optional-locks` writes no index, and the timeout
  kills only the git process.
- The repo sets `core.fsmonitor = true` in `.git/config`, which every worktree shares, so each worktree's
  `git status` asks its builtin fsmonitor daemon over a unix socket. With `-c core.fsmonitor=false`, the scan of
  this repo's 94 worktrees (79 of them `.claude/worktrees/agent-*`) took 3.9 s on 2026-10-26, with none timing out.
