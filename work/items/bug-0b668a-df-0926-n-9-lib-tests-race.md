+++
id = "bug-0b668a"
kind = "bug"
title = "DF-0926 N-9: Lib tests race with a live plan run on shared .roko/learn state"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/tests"]
created = 2026-09-26
updated = 2026-09-28
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#N-9. Lib tests race with a live plan run"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#N-9. Lib tests race with a live plan run"
anchors = ["dispatch::tests::efficiency_tracker_records_model_slug_not_template_name"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
efficiency_tracker_records_model_slug_not_template_name failed during a plan run and passed in isolation; tests touching .roko/learn should use temp workspaces.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#N-9. Lib tests race with a live plan run`

How to verify: grep tests writing to .roko/learn without tempdir.
