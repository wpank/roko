+++
id = "bug-df12bc"
kind = "bug"
title = "The commit hook's sync closes another agent's item in your worktree when main carries its Closes: trailer before its close commit"
status = "open"
triage = "verified"
severity = "p3"
size = "S"
subsystem = ["tools/work"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
source = "hermes-roko round v5, roko-chathost, 2026-10-16"
anchors = ["tools/work.py::cmd_hook", "tools/work.py::sync_candidates"]
links = { depends_on = [], blocks = [], related = ["bug-e0624e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_hook_sync_skips_trailers_merged_from_main' tools/test_work.py && python3 tools/test_work.py -k test_hook_sync_skips_trailers_merged_from_main"
+++

## Problem

The Claude Code commit hook (`tools/work.py hook`) runs `cmd_sync` after every commit. `cmd_sync` closes any open
item that a `Closes: <id>` trailer anywhere in the checkout's history names. In a worker's worktree that history
includes main's commits merged in, so when main carries a fix with `Closes: <id>` before that item's close commit has
landed, the next commit in any worktree that merged main closes the item there, as uncommitted edits under `work/`.

Seen 2026-10-16: bug-e0624e's fix `bf57527fd` (trailer `Closes: bug-e0624e`) merged to main at 12:34 (`3b1655bc4`); its
close landed at 13:17 (`8b5474308`). In between, roko-chathost's worktree closed bug-e0624e twice through the hook, and
the agent had to restore the files by hand each time to avoid committing a second, conflicting closure.

## Why it matters

Parallel workers merge main into their branches often. A closure made in someone else's worktree either conflicts with
the owner's real close, or is committed under the wrong executor and evidence (`by = "commit trailer"`, executor
`unknown`).

## Where

- `tools/work.py::cmd_hook`: calls `cmd_sync` for the whole history after any commit.
- `tools/work.py::sync_candidates`: finds trailers without regard to which branch the commit came from.

## Plan

1. In the hook, only consider trailers in commits this branch made since it last merged main (first-parent commits
   that main does not contain), or skip the hook's sync outside the main checkout and leave sync to the merge there.
2. Keep `tools/work.py sync` run by hand unchanged.
3. Test: a worktree that merges a main commit with `Closes: <id>`, then commits something unrelated, leaves `<id>`
   open; its own commit with `Closes: <own-id>` still closes `<own-id>`.

## Done when

`test_hook_sync_skips_trailers_merged_from_main` passes (the `[[verify]]` command).

## Notes

Process side: merging a fix before its close commit opens the window; the documented flow closes in the same flow.
