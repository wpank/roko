+++
id = "gap-2cd0ec"
kind = "gap"
title = "A resumed run's TIME on the web and in Hermes counts only its latest part: serve's last_run.started_at and run_completed.duration_ms are the resume's own, and roko reports no earlier part's run time, where §4.8 counts the sum of every part"
status = "open"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "M"
subsystem = ["apps/portal"]
created = 2026-11-18
updated = 2026-11-18
last_verified = 2026-11-18
source = "round v8 portal-v8, agreeing bug-29f52f's definition with roko-recovery (2026-11-13)"
discovered_from = "agent:portal-v8 (round v8)"
anchors = ["crates/roko-learn/src/telemetry/records.rs::RunInvocation", "crates/roko-serve/src/routes/plans/reads.rs", "apps/portal/src/lib/runReadings.ts::runClock"]
links = { depends_on = [], blocks = [], related = ["bug-29f52f", "gap-1ac9eb", "gap-c1726f"], supersedes = [], duplicate_of = "" }
+++

## Problem

§4.8 says a resumed run's TIME is the sum of its parts' run times. The rule was agreed on 2026-11-13 with
roko-recovery and the lead, for bug-29f52f. It counts each ended part's `run_completed.duration_ms` plus the running
part's time since roko accepted it. The time the run sat stopped between parts doesn't count.

The web counts only the latest part:

- While a resume runs, the portal counts from `last_run.started_at`. roko serve sets that when it accepts the resume.
- Once the run has ended, the portal reads `run_completed.duration_ms`. roko measures that from the resume's own start.

On phase 6 (run 23837ede, three parts) the web would read the third part's 25m 43s. The terminal reads the first
part's 11m 51s (bug-29f52f). Hermes takes TIME from the same serve fields (§4.8), so it reads the latest part too.

Nothing reports the earlier parts' time:

- The run manifest's invocations record when each part started (`RunInvocation.started_at`), but not when it ended
  or how long it ran.
- `budget_carried` carries the earlier parts' spend only.

## Why it matters

Goal visibility: SPENT and CHECKS count the whole run on every surface. A resumed run's TIME that counts one part
disagrees with them, and with the other surfaces.

## Where

- `crates/roko-learn/src/telemetry/records.rs::RunInvocation`: no end or run time per part.
- roko-cli's resume: `budget_carried` carries spend only.
- `crates/roko-serve/src/routes/plans/reads.rs`: `last_run`.
- `apps/portal/src/lib/runReadings.ts::runClock`.

## Plan

1. Each invocation records its run time when it ends, with gap-1ac9eb.
2. roko reports the earlier parts' sum as `carried_ms`:
   - on `budget_carried`, for surfaces watching live;
   - in `GET /api/plans/{id}`'s `last_run`, for a page opened later.
3. The portal's `runClock` adds it: TIME is `carried_ms` plus now − `started_at` while the run goes, and `carried_ms`
   plus `duration_ms` once it ends. Hermes reads the same fields.
4. Test: a resumed run's TIME on the glass, the chip and the list row is the sum.

## Done when

- A resumed run's TIME is the sum of its parts on the web.
- roko serve reports `carried_ms`, so Hermes can count it the same way.
- The `[[verify]]` passes; it's written with the fix.

## Notes

- Round v8 portal-v8, agreeing bug-29f52f's definition with roko-recovery. bug-29f52f is the terminal's part.
