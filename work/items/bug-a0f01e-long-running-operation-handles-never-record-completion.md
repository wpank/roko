+++
id = "bug-a0f01e"
kind = "bug"
title = "Long-running operation handles never record completion"
status = "open"
triage = "verified"
severity = "p2"
goal = "visibility"
subsystem = ["roko-serve/operations"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-serve/src/routes/dream.rs:107", "crates/roko-serve/src/routes/research.rs:257", "crates/roko-serve/src/routes/gateway.rs:708"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

Nine handlers register an entry in `AppState.operations` (dream `routes/dream.rs:107`, research `:257`, PRDs `prds.rs:534/709/947`, templates `:207`, plans `:948/:1591`, inference batch `gateway.rs:728`), but only the inference batch ever sets `Completed` (`gateway.rs:708`).
A finished dream, research or plan-generation job reports `"Running"` until handle GC removes it, after which `GET /api/operations/{id}` is 404; the status is a Rust `Debug` string, and nothing survives a restart.
Fix: one registry wrapper that records Running -> Completed | Failed | Cancelled for every producer, returns a JSON status object and persists a bounded record.
