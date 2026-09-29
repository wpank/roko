+++
id = "gap-cb9274"
kind = "gap"
title = "Plan editor diagnostics show no rule id and no link to the task (design section 4a)"
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
anchors = ["apps/portal/src/components/stage/SourceEditor.tsx:151", "apps/portal/src/components/stage/SourceEditor.tsx:301"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqF --include='*.test.ts' --include='*.test.tsx' 'shows the rule id' apps/portal/src && (cd apps/portal && npx vitest run src/components/stage)"
+++

## Problem

After a rejected save (422), the editor lists each diagnostic as `SEVERITY: message [T02]`
(`SourceEditor.tsx:301-325`). The server sends a `rule_id` (for example `PLAN_005`), but it is not
shown, and the task id is plain text. The caret moves once, to the first exact `id = "<task_id>"`
string of the first diagnostic that names a task (`moveCaret`, `:151-172`). So `id="T02"` or
`id = 'T02'` is never found, and later diagnostics cannot be reached. Design §4a: a rejected save
"lists diagnostics (rule id, and a task id that jumps to that `[[task]]` line)".

## Why it matters

Goal `visibility`. Edit as text is the portal's lossless editing path (contract ask P-4, which
landed); its error list is the guide to fixing a rejected save.

## Where

`apps/portal/src/components/stage/SourceEditor.tsx`: the diagnostics list and `moveCaret`.

## Plan

Show `rule_id`, make each task id a button, and find the task's `[[task]]` block by parsing, or
with a whitespace- and quote-tolerant search.

## Done when

A test whose name contains "shows the rule id", plus a test that clicking a task id moves the caret.
The `[[verify]]` runs them.

## Notes

Found by plan 09 T04 (see VERDICT).
