+++
id = "bug-0522e8"
kind = "bug"
title = "Unsaved plan edits are unprotected: the r key runs the saved plan, and the Edit or Revise buttons discard the text without asking"
status = "done"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["apps/portal"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "843b9f972"
source = "plan:portal-programme/09-acceptance#T04"
discovered_from = "plan:portal-programme/09-acceptance#T04"
anchors = ["apps/portal/src/components/shell/Workspace.tsx:138", "apps/portal/src/components/stage/PlanView.tsx:384", "apps/portal/src/components/stage/PlanView.tsx:403"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqF --include='*.test.ts' --include='*.test.tsx' 'protects unsaved edits' apps/portal/src && (cd apps/portal && npx vitest run src/components/stage src/components/shell)"

[closed]
at = 2026-09-29
commit = "843b9f972"
by = "commit trailer"
evidence = "843b9f972: the unsaved-text mark moved to the dashboard store (unsavedPlan/setUnsaved), which usePrimaryAction reads, so the header Run and the r key share it; the Edit and Revise buttons (PlanView), leaving the plan (useSelection.select) and the generate field (Workspace openPrompt) confirm 'Discard unsaved edits?' first; Run all and group runs wait with 'Save or discard your edits first'; Stage keys PlanView by plan id so no plan inherits another plan's editor text. apps/portal/src/components/shell/unsavedEdits.test.tsx ('protects unsaved edits', 7 DOM tests: r key, Edit, Revise, another plan by click and arrow key, n and + New plan, Run all, no prompt when clean); the [[verify]] passes (16 files, 92 tests)."
+++

## Problem

Design §4.1 disables Run "while the editor holds unsaved text", and §4a says Esc discards, "confirming
when dirty". The header's Run honours that: PlanView builds its action with
`usePrimaryAction(planId, { editing })`. Three other paths do not:

- The `r` key uses a second action, `usePrimaryAction(resolved.plan, { onError })`, built without
  `editing` (`Workspace.tsx:138-140`). Keys are ignored while the textarea has focus, but once focus
  leaves it, `r` runs the saved plan with the edits still unsaved.
- Clicking ✎ Edit again closes the editor and clears its dirty flag with no confirm
  (`PlanView.tsx:403-409`).
- Clicking ✦ Revise closes the editor the same way (`PlanView.tsx:384-387`).

Only Esc inside the editor asks (`SourceEditor.tsx` `confirmClose`).

## Why it matters

Goal `visibility`. Typed edits are lost silently, and `r` runs a plan other than the one on screen.

## Where

The anchors: the `r` binding's action in `Workspace.tsx`, and the Revise and Edit button handlers in
`PlanView.tsx`.

## Plan

Share one primary-action state between the header button and the `r` key: lift `editing` and
`editorDirty` to the workspace, or into the store. Route every way of closing the editor through
`confirmClose`.

## Done when

Tests whose names contain "protects unsaved edits" cover the `r` key, the Edit toggle and the Revise
button with a dirty editor. The `[[verify]]` runs them.

## Notes

Found by plan 09 T04 (see VERDICT).
