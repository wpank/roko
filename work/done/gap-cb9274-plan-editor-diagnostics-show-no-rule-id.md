+++
id = "gap-cb9274"
kind = "gap"
title = "Plan editor diagnostics show no rule id and no link to the task (design section 4a)"
status = "done"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["apps/portal"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "217a6be45"
source = "plan:portal-programme/09-acceptance#T04"
discovered_from = "plan:portal-programme/09-acceptance#T04"
anchors = ["apps/portal/src/components/stage/SourceEditor.tsx:151", "apps/portal/src/components/stage/SourceEditor.tsx:301"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqF --include='*.test.ts' --include='*.test.tsx' 'shows the rule id' apps/portal/src && (cd apps/portal && npx vitest run src/components/stage)"

[closed]
at = 2026-09-29
commit = "217a6be45"
evidence = "217a6be45: the editor's 422 list shows each diagnostic as '<severity> <rule_id> <task>: <message>', the task id is a button that moves the caret to that task's [[task]] line (findTaskLine tolerates spacing and either quote style). components/stage/SourceEditor.test.tsx: 5 passed ('shows the rule id and the task of each diagnostic after a rejected save', 'clicking a task id moves the caret to that task's [[task]] line'), all 5 fail on 70820a74c; the item's verify passes (stage suite 11 files / 70 tests). Headless Chromium on a rejected save of live-b: rows 'error PLAN_005 T01: ...' and 'warning PLAN_010 T01: ...'; clicking T01 focuses the editor with the caret on its [[task]] line."
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
