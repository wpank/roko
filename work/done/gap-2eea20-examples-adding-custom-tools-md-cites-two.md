+++
id = "gap-2eea20"
kind = "gap"
title = "examples/adding-custom-tools.md cites two files in a deleted worktree (roko-mr-stream-beta); the rest of old backlog 76's example-config fixes have landed"
status = "done"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["docs/examples"]
created = 2026-10-15
updated = 2026-10-16
last_verified = 2026-10-16
last_verified_rev = "3b5d17354"
source = "status-quo-2026-10-15#finding-44"
anchors = ["examples/adding-custom-tools.md", "examples/adding-a-provider.md"]
lane = "docs"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -rq 'roko-mr-stream-beta' examples/"

[closed]
at = 2026-10-16
at_ts = "2026-10-16T13:58:19Z"
commit = "3b5d17354"
by = "commit trailer"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-16T08:07:09Z"
forced = false
evidence = "3b5d17354 dropped examples/adding-custom-tools.md's 'Context files' section (lines 25-29), which pointed at /Users/will/dev/nunchi/roko/roko-mr-stream-beta/tmp/implementation-plans/modelrouting/{18-structural-cleanup,01-architecture}.md, a deleted worktree. Verify '! grep -rq roko-mr-stream-beta examples/' passes at 3b5d17354. Compared adding-a-provider.md's provider list against CLAUDE.md's roko-agent row (12 kinds): that doc only covers the openai_compat zero-code path and never enumerates provider kinds, so no gap found there."
+++

## Problem

Old backlog file `tmp/backlog/archive/76-example-config-quality.md` (P1 in the frozen summary: examples are the main
onboarding material) never became a work item. Most of it is fixed at HEAD: no `examples/*.toml` has the ignored
`[prompt]` section any more, and no example curl uses a port other than 6677. One defect remains:
`examples/adding-custom-tools.md:27-28` points readers at
`/Users/will/dev/nunchi/roko/roko-mr-stream-beta/tmp/implementation-plans/modelrouting/…`, an absolute path into a
worktree that no longer exists (and is private to one machine). Status-quo assessment 2026-10-15 at 7b6a5af44,
finding 44 (B's archive check).

## Why it matters

Goal `tooling`: a new user following the guide hits a dead reference.

## Where

`examples/adding-custom-tools.md` lines 27-28; also check `examples/adding-a-provider.md`'s provider list against
the provider kinds roko supports (the archived file said it was incomplete; this check did not settle it).

## Current state

The other example fixes landed at some point after 2026-09-07; no item recorded them.

## Plan

1. Replace the two references with repo paths that exist (or drop them if they add nothing).
2. Compare the provider list in `adding-a-provider.md` with the provider kinds in CLAUDE.md's roko-agent row and fix
   any gap.

## Done when

The `[[verify]]` command passes.

## Notes

Docs only.
