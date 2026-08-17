# Audit and fix nunchi.network landing page fake metrics

**Status**: Backlog
**Priority**: P1 (reputational risk — fake numbers visible to investors and partners)
**Size**: S (1-2 hours)

---

## Background

Nunchi is the company behind roko, an agent toolkit. The public website is
`nunchi.network`. It is maintained in a separate repository named `nunchi-dashboard`,
which is not the roko repo. This backlog item is a reminder and pointer — the actual
work happens there.

---

## Problem

During an April 2026 dogfood audit (see **Origin** below), the nunchi.network landing
page was found to display hardcoded, never-updated placeholder metrics:

- "84,213 agents deployed"
- "12,425 tasks completed"
- "3,240 active users"

These numbers were originally inserted as design scaffolding and were never replaced
with real data or removed. They are visible to anyone who visits the site — including
investors, enterprise prospects, and technical evaluators who can trivially inspect the
page source or notice the suspiciously round-looking numbers.

Displaying fabricated traction metrics is a direct reputational risk. Nunchi is in
fundraising. An investor who asks "where does the 84,213 figure come from?" and receives
no satisfactory answer will draw the obvious conclusion.

The same audit flagged two secondary issues on the same page:

1. **Stale terminology.** The page may still use "Engram" — the old internal name for
   the core data primitive. The workspace-wide rename to "Signal" was completed in the
   2026-08-12 batch. Outward-facing material should match.

2. **Stale regulatory reference.** The landing page may include an EU AI Act section
   that references an August 2, 2026 compliance date as a future milestone. That date
   has now passed. Any forward-looking language keyed to that date reads as outdated.

---

## What to do

This is a content/frontend task in the `nunchi-dashboard` repo. No roko Rust code
changes are required.

### 1. Remove or replace fake counters

The metric counters must not show fabricated numbers. There are three acceptable
outcomes — pick whichever fits the current state of real data:

| Option | When to use |
|---|---|
| **Remove the counters entirely** | No live data pipeline exists; cleanest option |
| **Replace with honest placeholder text** | e.g., "Early access" or a waitlist CTA |
| **Wire to real data** | If an API exists that reports actual usage, use it |

Do not replace one set of fake numbers with a different set of fake numbers.

### 2. Search for "Engram" on the page and in component source

Run a case-insensitive search across the `nunchi-dashboard` frontend source:

```
grep -ri "engram" src/
```

Replace any occurrence with "Signal" (or the user-facing equivalent). If "Engram"
appears in marketing copy describing the data model, the updated term is "Signal" —
the primitive that flows through the roko graph.

### 3. Review the EU AI Act section

If the landing page contains a section about EU AI Act compliance and it refers to
August 2, 2026 as an upcoming date, update the language to reflect that the date has
passed. Options:

- Change "as of August 2, 2026" or "by August 2, 2026" to "as of August 2026" or
  "effective August 2026"
- Update to reflect actual current compliance posture
- Remove if the section is no longer accurate or relevant

---

## What not to do

- Do not add new placeholder metrics. Even "coming soon" counters with zeroes can read
  as misleading.
- Do not invent real-looking numbers to replace the fake ones.
- Do not make changes to the roko workspace (`nunchi/roko`) — this work is in
  `nunchi-dashboard`.

---

## Acceptance criteria

1. Loading nunchi.network shows no hardcoded numeric traction metrics (no "84,213
   agents deployed" or equivalent). If counters exist, they are either absent, replaced
   with honest placeholder text, or connected to a real data source.
2. A case-insensitive search for "engram" across the rendered page HTML returns zero
   matches.
3. Any EU AI Act language referencing August 2, 2026 as a future date has been updated
   to past tense or removed.
4. The page passes a manual review: a first-time visitor with no roko context reads the
   page and does not encounter a statistic, term, or date that is demonstrably false or
   stale.

---

## Origin

Discovered during the April 2026 dogfood audit. The original finding is recorded at:

```
tmp/archive/08-15-26/dogfood/11-LANDING-PAGE-UPDATES.md
```

(That path is inside the roko workspace, which served as a scratch area for audit
notes. The fix itself goes in `nunchi-dashboard`.)
