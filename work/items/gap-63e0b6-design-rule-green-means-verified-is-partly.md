+++
id = "gap-63e0b6"
kind = "gap"
title = "Design rule 'green means verified' is partly built: the header is never amber, nor is a running plan whose tasks are all dispatched"
status = "open"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["apps/portal"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "f99e45dba"
source = "plan:portal-programme/09-acceptance#T04"
discovered_from = "plan:portal-programme/09-acceptance#T04"
anchors = ["apps/portal/src/components/shell/Header.tsx:157", "apps/portal/src/lib/planRows.ts:150"]
links = { depends_on = [], blocks = [], related = ["bug-7e1b6b"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqF --include='*.test.ts' --include='*.test.tsx' 'header is amber' apps/portal/src && (cd apps/portal && npx vitest run src/components/shell src/lib/planRows)"
+++

## Problem

Design §6, rule 1: "Any `accepted_with_failures` task keeps the task, the plan row and the header
amber; so does a plan whose tasks are all dispatched while it is still running (mori,
`plan_tree.rs:572`)."

Built: the task row and a completed plan's row turn amber.

Not built:

- The header's run glyph is always `<StatusGlyph state="active" />` (`Header.tsx:157`).
- `planRows.ts:150-153` maps every running plan to `active`, so there is no amber state for a plan
  whose tasks are all dispatched while its checks run.

## Why it matters

Goal `visibility`. The design's colour rules carry information: amber says "not verified yet" or
"forced through". Accepted-with-failures is rare today (design §6: nothing forces an accept yet),
but the all-dispatched state occurs in every run.

## Where

`apps/portal/src/components/shell/Header.tsx:157` and the running case in
`apps/portal/src/lib/planRows.ts`.

## Plan

Derive the header glyph from the running plans (amber when any has an accepted task, or has every
task dispatched), and add that state to the rail row.

## Done when

A test whose name contains "header is amber" covers both cases. The `[[verify]]` runs it.

## Notes

After a reload the accepted count is lost; that is the reload item filed alongside this one.
Skipped tasks counted as passed is bug-7e1b6b. Found by plan 09 T04 (see VERDICT).
