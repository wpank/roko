+++
id = "bug-0b668a"
kind = "bug"
title = "Lib tests race with a live plan run on shared .roko/learn state"
status = "done"
triage = "verified"
severity = "p3"
goal = "tooling"
subsystem = ["roko-cli/tests"]
created = 2026-09-26
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "tmp/dogfood/2026-09-25-portal-programme-run.md#N-9. Lib tests race with a live plan run"
discovered_from = "audit:tmp/dogfood/2026-09-25-portal-programme-run.md#N-9. Lib tests race with a live plan run"
anchors = ["crates/roko-serve/src/dispatch.rs::EfficiencyTracker::record_event", "crates/roko-serve/src/dispatch.rs::tests::efficiency_tracker_records_model_slug_not_template_name"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "sed -n '/async fn record_event/,/^    }/p' crates/roko-serve/src/dispatch.rs | grep -q 'flush().await' && cargo test -p roko-serve --lib dispatch::tests::efficiency_tracker_records_model_slug_not_template_name"

[closed]
at = 2026-09-29
commit = "352f13f23"
by = "work sweep 2026-09-29 (static check against HEAD; cargo verify not re-run while the portal plan run held the build lock)"
evidence = "352f13f23 ('flush tokio::fs writes before returning (the N-9 flake and 16 siblings)') found the real cause of this flake. EfficiencyTracker::record_event (roko-serve/src/dispatch.rs:246) returned after write_all on a tokio::fs::File without a flush, so an immediate reader could see an empty file ('EOF while parsing a value'). It now calls file.flush().await? (dispatch.rs:313). The test (dispatch.rs:3341) already used a tempdir, so the shared .roko/learn race in the title was a misdiagnosis."
+++
efficiency_tracker_records_model_slug_not_template_name failed during a plan run and passed in isolation; tests touching .roko/learn should use temp workspaces.

Imported without verification from:
- `tmp/dogfood/2026-09-25-portal-programme-run.md#N-9. Lib tests race with a live plan run`

How to verify: grep tests writing to .roko/learn without tempdir.

Check on 2026-09-28 was inconclusive: The claimed mechanism is absent: the only test with this name (crates/roko-serve/src/dispatch.rs:3335-3351) creates tempfile::tempdir() and EfficiencyTracker::new(tmp.path()), which resolves to <tmp>/.roko/learn/efficiency.jsonl (dispatch.rs:234-236, roko-fs/src/layout.rs:178-180,318-320), and it was already written that way before 9c6ec420c, so it cannot share the live workspace's .roko/learn. No test was found that builds a RokoLayout over the real workspace (no CARGO_MANIFEST_DIR, current_dir or "." layouts). The one-off failure in tmp/dogfood/2026-09-25-portal-programme-run.md:804-812 is real but its cause is unknown. To decide: rerun `cargo test -p roko-serve --lib` during a live plan run and capture the failing assertion; if it does not reproduce, close the item as a misdiagnosis.

Closed 2026-09-29: the cause was not shared .roko/learn state. The test uses a tempdir. The cause was an unflushed tokio::fs write in EfficiencyTracker::record_event, fixed in 352f13f23 (roko-serve/src/dispatch.rs:313) together with 16 sibling write sites, with regression test retrieval_outcome append_without_fsync_is_visible_on_return.
