+++
id = "bug-0b668a"
kind = "bug"
title = "Lib tests race with a live plan run on shared .roko/learn state"
status = "open"
triage = "unverified"
severity = "p3"
goal = "tooling"
subsystem = ["roko-cli/tests"]
created = 2026-09-26
updated = 2026-09-28
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#N-9. Lib tests race with a live plan run"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#N-9. Lib tests race with a live plan run"
anchors = ["dispatch::tests::efficiency_tracker_records_model_slug_not_template_name", "crates/roko-serve/src/dispatch.rs::tests::efficiency_tracker_records_model_slug_not_template_name"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
efficiency_tracker_records_model_slug_not_template_name failed during a plan run and passed in isolation; tests touching .roko/learn should use temp workspaces.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#N-9. Lib tests race with a live plan run`

How to verify: grep tests writing to .roko/learn without tempdir.

Check on 2026-09-28 was inconclusive: The claimed mechanism is absent: the only test with this name (crates/roko-serve/src/dispatch.rs:3335-3351) creates tempfile::tempdir() and EfficiencyTracker::new(tmp.path()), which resolves to <tmp>/.roko/learn/efficiency.jsonl (dispatch.rs:234-236, roko-fs/src/layout.rs:178-180,318-320), and it was already written that way before 9c6ec420c, so it cannot share the live workspace's .roko/learn. No test was found that builds a RokoLayout over the real workspace (no CARGO_MANIFEST_DIR, current_dir or "." layouts). The one-off failure in tmp/dogfood/2026-09-25-portal-programme-run.md:804-812 is real but its cause is unknown. To decide: rerun `cargo test -p roko-serve --lib` during a live plan run and capture the failing assertion; if it does not reproduce, close the item as a misdiagnosis.
