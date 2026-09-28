+++
id = "bug-0774cc"
kind = "bug"
title = "DF-0817ex E: Doctor/onboarding residuals"
status = "parked"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/doctor"]
created = 2026-08-17
updated = 2026-09-28
source = "tmp/archive/dogfood-2026-08-17-examples/DOGFOOD-DEBRIEF.md#E. Doctor / Onboarding UX (7 findings)"
discovered_from = "audit:tmp/archive/dogfood-2026-08-17-examples/DOGFOOD-DEBRIEF.md#E. Doctor / Onboarding UX (7 findings)"
anchors = ["doctor.rs", "roko setup"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[parked]
at = 2026-09-28
from_status = "open"
reason = "Triage 2026-09-28: unchecked import from an older document; not planned unless revived"
+++
doctor warns about missing API keys even when a CLI provider is primary; setup wizard does not write [providers.*] for detected keys; plans_dir_conflict fix (mv .roko/plans/* plans/) can clobber; CursorAcp exclusion undocumented.

Imported without verification from:
- `tmp/archive/dogfood-2026-08-17-examples/DOGFOOD-DEBRIEF.md#E. Doctor / Onboarding UX (7 findings)`

How to verify: Run roko doctor with only claude_cli configured; inspect warnings and fix hints.
