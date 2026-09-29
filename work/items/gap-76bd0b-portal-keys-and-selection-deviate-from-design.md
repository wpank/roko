+++
id = "gap-76bd0b"
kind = "gap"
title = "Portal keys and selection deviate from design section 9: Esc leaves Revise open, an unknown task id is kept, a later-starting plan is auto-selected"
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
anchors = ["apps/portal/src/components/shell/Workspace.tsx:247", "apps/portal/src/lib/selection.ts::resolveSelection"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqF --include='*.test.ts' --include='*.test.tsx' 'Esc closes the revise prompt' apps/portal/src && (cd apps/portal && npx vitest run src/lib/selection src/components/shell src/components/stage)"
+++

## Problem

- **Esc:** the workspace's handler dismisses the alert, clears the filter or closes the generate
  prompt (`Workspace.tsx:247-255`). The ✦ Revise prompt lives in PlanView state and ignores Esc:
  `PromptPanel` handles only ⌘/Ctrl+Enter. Design §9: Esc closes "the editor or prompt".
- **Unknown task:** `resolveSelection` clears an unknown `plan` but never checks `task` against the
  plan's tasks. Design §9: "an unknown id clears silently".
- **Auto-select:** rule 3 of `resolveSelection` ("no plan selected → the first running plan") runs on
  every render, not only on load. With nothing selected, any plan that starts later gets selected.
  Design §9: "With no plan on load, the first running plan is selected. The selection never jumps on
  its own afterwards." (The filter's effect on the selection is filed separately.)

## Why it matters

Goal `visibility`. Small, but these are the six-key contract the design keeps instead of a command
bar.

## Where

The escape binding in `apps/portal/src/components/shell/Workspace.tsx` and
`apps/portal/src/lib/selection.ts::resolveSelection`.

## Plan

Let Esc close the Revise prompt: lift `revising`, or handle Escape in `PromptPanel`. Check `task`
against the loaded task ids. Apply the running-plan default only on first load.

## Done when

Tests whose names contain "Esc closes the revise prompt", plus tests for the other two rules. The
`[[verify]]` runs them.

## Notes

Found by plan 09 T04 (see VERDICT).
