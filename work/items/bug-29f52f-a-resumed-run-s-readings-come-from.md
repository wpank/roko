+++
id = "bug-29f52f"
kind = "bug"
title = "A resumed run's readings come from its first part only: plan status's TIME is the first invocation's run_completed (phase 6: 11m 51s of three parts), and a resume's plan check and live spend are in the event index of its serve run id, which nothing on disk links to the run"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "M"
subsystem = ["roko-cli/commands"]
created = 2026-11-13
updated = 2026-11-13
last_verified = 2026-11-13
source = "round v8 lead, on gap-9bfb4e's phase 6 reading (6/6 tasks · 11m 51s · $5.77 · 7/7 checks)"
discovered_from = "agent:team-lead (round v8)"
anchors = ["crates/roko-cli/src/commands/diagnose.rs::RunIndex", "crates/roko-learn/src/telemetry/records.rs::RunInvocation"]
links = { depends_on = [], blocks = [], related = ["gap-9bfb4e", "gap-1ac9eb", "bug-addd24", "bug-7a62d9"], supersedes = [], duplicate_of = "" }
+++

## Problem

The readings of a resumed run come from its first part only. In round v8, phase 6 (Undertow `p6-the-window`, run
23837ede, three invocations) reads in `roko plan status` (gap-9bfb4e) as `6/6 tasks · 11m 51s · $5.77 · 7/7 checks`:

- **SPENT** and **CHECKS** count the whole run. SPENT comes from the checkpoint's cost ledger. CHECKS comes from the
  run's `attempts.jsonl`, which holds every invocation's tries (inv 1–3).
- **TIME** is 11m 51s, the first invocation's `run_completed.duration_ms` (711416 ms). It comes from the run's event
  index, `.roko/events-by-run/<sha256(23837ede)>.jsonl`, which holds only the first invocation's events. The third
  invocation's events, run_completed 1543382 ms (25m 43s), are in the index of its own run id, ede1f9a7. serve gave
  the resume that id, and nothing on disk links it to the run: the manifest's invocations don't name it
  (gap-1ac9eb). The second invocation's index can't be found at all.
- **The live readings miss the resume too.** While a resume runs, its plan check row (bug-addd24) and its open
  tries' live spend (bug-7a62d9) are in that other index, so `plan status` shows neither.

The design system's §4.8 doesn't yet say what TIME means for a resumed run.

## Why it matters

Goal tooling. The readings should mean the same on every surface, and SPENT and CHECKS already count the whole run.
A TIME that counts one part of it disagrees with them, and probably with Hermes and the portal, which take TIME from
serve's run handle.

## Where

- `crates/roko-cli/src/commands/diagnose.rs::RunIndex` (reads one index, by the checkpoint's run id).
- The run manifest's invocations (`crates/roko-learn/src/telemetry/records.rs::RunInvocation`), which don't name the
  event index a resume writes to (gap-1ac9eb).

## Plan

1. **Agree the definition with portal-v8,** so the web and Hermes match. The lead suggests TIME for a resumed run is
   the sum of its invocations' run times (each part's `run_completed.duration_ms`, and the running part's elapsed
   time so far), which matches SPENT's whole-run sense. Write it into §4.8.
2. **Name each invocation's event index** in the run manifest (with gap-1ac9eb). `RunIndex` then reads every part's
   index: TIME is the sum, and the plan check row and live spend come from the running part.

## Done when

A resumed run's TIME counts every part. While a resume runs, its plan check row and live spend show. The `[[verify]]`
is to be written with the fix (no fix this round, the lead's call).
