+++
id = "gap-edbf2f"
kind = "gap"
title = "TUI still has two data models (DashboardData and DashboardSnapshot) during plan runs"
status = "open"
triage = "unverified"
severity = "p3"
subsystem = ["roko-cli/tui"]
created = 2026-08-31
updated = 2026-09-28
source = "gaps-md#partial-10/121"
anchors = ["crates/roko-cli/src/tui/dashboard.rs:355", "crates/roko-core/src/dashboard_snapshot.rs:1082"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

UX #121 (audit RC-1): during a plan run, the file-based `DashboardData` (`crates/roko-cli/src/tui/dashboard.rs:355`) and the event-driven `DashboardSnapshot` (`crates/roko-core/src/dashboard_snapshot.rs:1082`) are not kept in sync. The audit linked this to empty sparklines and empty efficiency and cost-by-model panels. Phase A of the unification is done. Phases B and C (the migration) had not started at the last record. Backlogs #121 and #391 are archived without a status.

Fix: make `DashboardSnapshot` the only TUI data source and delete the file-based loader.
