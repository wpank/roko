+++
id = "bug-a804b1"
kind = "bug"
title = "CLAUDE.md's CLI table is stale: diagnose prints text by default, and read-only roko prd commands no longer rebuild indexes"
status = "done"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["docs"]
created = 2026-10-01
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "bfd36512f"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-b06523"
anchors = ["CLAUDE.md"]
lane = "docs"
links = { depends_on = [], blocks = [], related = ["gap-b06523"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -n 'structured JSON output' CLAUDE.md"

[closed]
at = 2026-10-02
at_ts = "2026-10-02T20:40:17Z"
commit = "bfd36512f"
by = "roko-7d"
executor = "claude-session"
via = "manual"
forced = false
evidence = "Done in merge bfd36512f: CLAUDE.md's diagnose row says it prints a readable report (`--json` for JSON, `--verbose` adds completed tasks); the `roko prd` rows are gone with the PRD pipeline."
+++

## Problem

After gap-b06523, `roko diagnose` prints a readable report and `--json` gives JSON. After bug-48dc41, read-only `roko prd` subcommands no longer rebuild `plans/INDEX.md` and the `.roko/` INDEX files. CLAUDE.md's CLI table still states the old behaviour of both.

## Plan

Update the two rows. CLAUDE.md is the project's instruction file, so Will approves the edit.

## Done when

- The verify passes, and the prd row says which subcommands rebuild indexes.

## Notes

- Reported on 2026-10-01 by the worker on gap-b06523, during the evening close-out round.
