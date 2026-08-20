# 21 — Landing page fake metrics

**Priority**: P1 — Reputational risk: fabricated traction numbers are visible to investors and partners
**Size**: S (1-2 hours)
**Crates**: None (work is in the `nunchi-dashboard` frontend repo, not the roko workspace)
**Depends on**: None

---

## Background

Nunchi is the company behind roko, an agent toolkit for building self-improving AI systems. The public website at `nunchi.network` is maintained in a separate repository named `nunchi-dashboard`, which is not part of this roko workspace.

During an April 2026 dogfood audit, the landing page was found to display hardcoded placeholder metrics that were inserted during design scaffolding and never replaced with real data. These numbers have never been updated and have no relationship to actual usage. They are visible to anyone who views the page — including investors and enterprise evaluators.

Displaying fabricated traction numbers is a direct reputational risk, particularly during fundraising. An investor who notices a suspiciously round number like "84,213 agents deployed" and investigates will find it is hardcoded HTML. Separately, two pieces of stale content were flagged: the old internal name "Engram" (renamed to "Signal" workspace-wide on 2026-08-12) may still appear in marketing copy, and a section about EU AI Act compliance may refer to August 2, 2026 as a future date that has since passed.

## Current State

1. The `nunchi.network` landing page displays three hardcoded metrics:
   - "84,213 agents deployed"
   - "12,425 tasks completed"
   - "3,240 active users"

2. These are static HTML/JSX values in the `nunchi-dashboard` repository. They have no connection to any data source and do not update.

3. The Engram→Signal rename was completed on 2026-08-12 across all 38 roko crates. Marketing copy on the website was not updated as part of that batch. The term "Engram" may still appear in page content or component source.

4. If the landing page includes an EU AI Act compliance section, it may reference August 2, 2026 as a future compliance date. That date has passed.

5. No changes are needed in the roko workspace (`nunchi/roko`). All work goes in `nunchi-dashboard`.

## Implementation Plan

**Step 1: Find and remove the fake counters**

Search the `nunchi-dashboard` frontend source for the hardcoded numbers:

```bash
grep -ri "84,213\|12,425\|3,240\|agents deployed\|tasks completed\|active users" src/
```

Once located, choose one of three acceptable outcomes:

| Option | When to use |
|---|---|
| Remove the counter section entirely | No real data pipeline exists; cleanest approach |
| Replace with honest placeholder | Use "Early access" or a waitlist CTA instead of numbers |
| Wire to a real data source | Only if a live API reports actual usage statistics |

Do not replace one set of fake numbers with a different set of fake numbers. Do not add "coming soon" counters with zeroes.

**Step 2: Search for "Engram" across the frontend**

```bash
grep -ri "engram" src/
```

Replace any occurrence with "Signal" (or the user-facing equivalent). In marketing copy, "Signal" is the primitive data type that flows through a roko graph — the unit of information that agents store, route, and transform.

**Step 3: Update EU AI Act language if present**

If a section about EU AI Act compliance exists and refers to August 2, 2026 as an upcoming date, update the language. Acceptable changes:

- Change "as of August 2, 2026" or "by August 2, 2026" to "as of August 2026" or "effective August 2026"
- Update to reflect the actual current compliance posture
- Remove the section if it is no longer accurate

**Step 4: Manual review**

Read the page as a first-time visitor. Confirm there are no statistics, terms, or dates that are demonstrably false or outdated.

## Acceptance Criteria

1. Loading `nunchi.network` shows no hardcoded numeric traction metrics. If counters exist, they are either absent, replaced with honest placeholder text, or connected to a real data source.
2. A case-insensitive search for "engram" across the rendered page HTML returns zero matches.
3. Any EU AI Act language that referenced August 2, 2026 as a future date has been updated to past tense or removed.
4. A first-time visitor with no roko context reads the page and does not encounter a statistic, term, or date that is demonstrably false or stale.

## Verification Checklist

- [ ] Load `nunchi.network` in an incognito browser window and inspect the page for numeric traction metrics
- [ ] View page source (`Ctrl+U`) and search for `84,213`, `12,425`, `3,240`; confirm zero matches
- [ ] In browser dev tools, search rendered HTML for `engram` (case-insensitive); confirm zero matches
- [ ] If an EU AI Act section exists, confirm it uses past tense for the August 2, 2026 date
- [ ] Ask a colleague who has not seen the page to read it and flag anything that looks suspicious or outdated

## Files to Modify

| File | Change |
|---|---|
| `src/` (in `nunchi-dashboard` repo) | Remove or replace hardcoded metric counters |
| `src/` (in `nunchi-dashboard` repo) | Replace "Engram" with "Signal" in all component source and copy |
| `src/` (in `nunchi-dashboard` repo) | Update EU AI Act compliance date language if present |
