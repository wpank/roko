# Batch AUD01: Fix stale status, LOC counts, and crate counts in top-level docs

**Audit refs**: 00-MASTER-SUMMARY.md (item 5), 07-doc-quality-audit.md (Issue C),
06-codebase-reality-check.md (sections 1-6). This is the foundation batch --
every subsequent batch assumes these numbers and statuses are correct.

Read these files first:

- `tmp/refinement-audit-runner/context-pack/00-AUDIT-RULES.md`
- `tmp/refinements-audit/00-MASTER-SUMMARY.md`
- `tmp/refinements-audit/06-codebase-reality-check.md` (sections 1-6: reality numbers)
- `tmp/refinements-audit/07-doc-quality-audit.md` (Issue C: stale implementation status)
- `docs/INDEX.md`
- `docs/STATUS.md`
- `docs/00-architecture/INDEX.md` (full file, especially the "Current Status and Implementation Gaps" section near the bottom)
- `CLAUDE.md` (the project CLAUDE.md at repo root, for ground-truth status)

## Task

Update the three top-level navigation docs (`docs/INDEX.md`, `docs/STATUS.md`,
`docs/00-architecture/INDEX.md`) so they reflect the actual codebase state as of
2026-04-17. The refinements-runner wrote these docs but never reconciled the
status sections against reality. The audit found specific factual errors that
must be corrected.

## Current state (evidence)

The audit found these specific errors:

1. **`docs/00-architecture/INDEX.md`** says `roko-serve: HTTP API not wired` --
   WRONG. roko-serve has 200+ routes (30K LOC) and is fully wired. CLAUDE.md
   marks it as **Wired**.

2. **`docs/00-architecture/INDEX.md`** says `TUI: Text-mode dashboard only, no
   interactive terminal UI` -- WRONG. The TUI has 58K LOC of ratatui code with
   F1-F7 tabs, WebSocket integration, and is fully wired. CLAUDE.md marks it
   as **Wired**.

3. **`docs/STATUS.md`** says `Interfaces` section is `Scaffold` with
   `roko-cli (text dashboard)` -- WRONG. The TUI is a full ratatui interactive
   terminal UI, and roko-serve provides the HTTP API.

4. **`docs/STATUS.md`** says `HTTP server + REST API: Crate exists, no routes`
   under Scaffold -- WRONG. roko-serve has 200+ routes.

5. **`docs/STATUS.md`** says `Text dashboard (TUI): Renders text pages, no
   interactive terminal UI` under Scaffold -- WRONG. Full ratatui.

6. **LOC counts**: CLAUDE.md says ~177K LOC and 18 crates. The reality check
   found 322,088 LOC and 36 workspace members. Docs that cite these numbers
   need correction.

7. **Test count**: STATUS.md says `Total: 1,568 tests`. The reality check found
   3,761 test functions. The per-crate breakdown may also be stale.

8. **Route count**: CLAUDE.md says ~85 routes. The reality check found 200+.
   Docs that cite the route count should say 200+.

9. **roko-learn**: STATUS.md says 101 tests. The reality check found 42 modules
   and 35,847 LOC -- the test count may be understated.

10. **Critical Path section in STATUS.md** says `Interactive TUI (Section 12) --
    Wire ratatui into the text dashboard scaffold` -- this is DONE.

## Implementation

### 1. Fix `docs/00-architecture/INDEX.md` status section

Find the "Current Status and Implementation Gaps" section (near the bottom of
the file). Update:

- Change `roko-serve` status from "HTTP API not wired" to "**Shipping**: 200+
  REST routes, SSE, WebSocket on :6677"
- Change TUI status from "Text-mode dashboard only" to "**Shipping**: ratatui
  interactive TUI with F1-F7 tabs, WebSocket, themes, modals"
- Update any stale test counts or LOC numbers in that section
- Update the "Sub-docs" count if it says 29 when there are now 36 files
- Update the generated date if present

### 2. Fix `docs/STATUS.md` master status matrix

- Change section 12 (Interfaces) from `Scaffold` to `Shipping` (at least for
  TUI and HTTP API; some subsections like web portal remain Specified)
- Move `HTTP server + REST API` and `Text dashboard (TUI)` from the Scaffold
  section to the Shipping section
- Update their descriptions to reflect reality
- Update the test count total and per-crate breakdown where verifiable
- Fix the Critical Path section: mark `Interactive TUI` as DONE
- Update LOC/crate counts if cited

### 3. Fix `docs/INDEX.md` if it cites stale numbers

- Check whether the top-level INDEX cites 177K LOC, 18 crates, or ~85 routes
- If so, update to 322K LOC, 36 crates, 200+ routes
- Do NOT rewrite the "Current Framing" block -- that is AUD07's scope

### 4. Verify consistency

After edits, confirm that all three files agree on:
- roko-serve status (Shipping/Wired, 200+ routes)
- TUI status (Shipping/Wired, ratatui)
- LOC count (322K or "300K+")
- Crate count (36)
- Test count (3,761 or "3,700+")

## Write scope

- `docs/00-architecture/INDEX.md`
- `docs/STATUS.md`
- `docs/INDEX.md` (only if it cites stale numbers)

## Rules

1. **Only fix factual status and numbers.** Do not rewrite prose, restructure
   sections, or change architectural framing. That is later batches' scope.
2. **Use conservative numbers.** If unsure of exact count, use "200+" not "247"
   or "300+" not "322,088". Round to avoid false precision.
3. **Preserve the existing section structure.** Move items between tiers (e.g.,
   Scaffold -> Shipping) but do not add or remove sections.
4. **Do not touch any file outside the write scope.** Other docs will be fixed
   in AUD02-AUD08.
5. **Cross-reference against CLAUDE.md** for ground truth on what is wired.
   If CLAUDE.md and the audit disagree, note both and prefer the more
   conservative claim.

## Done when

- `docs/00-architecture/INDEX.md` no longer says serve is "not wired" or TUI
  is "text-mode only"
- `docs/STATUS.md` section 12 (Interfaces) is at least `Shipping` for TUI and
  HTTP API subsystems
- `docs/STATUS.md` no longer lists `HTTP server` or `Text dashboard` under
  Scaffold
- `docs/STATUS.md` Critical Path no longer lists TUI as a pending item
- All three docs agree on serve/TUI status and use consistent LOC/crate/route
  numbers
- No new sections or structural changes were introduced
- Final message lists every number changed, the old value, and the new value
