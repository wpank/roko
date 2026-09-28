+++
id = "bug-012303"
kind = "bug"
title = "Serve dispatch paths build CascadeRouter instances whose learning is never saved"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-serve/dispatch", "roko-learn/cascade-router"]
created = 2026-09-28
updated = 2026-09-28
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-serve/src/dispatch.rs:2839", "crates/roko-serve/src/dispatch.rs:3461", "crates/roko-serve/src/lib.rs:1096"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

Besides `AppState.cascade_router` (loaded at `lib.rs:1096`, saved on shutdown), serve builds more routers with `CascadeRouter::load_or_new` in `dispatch.rs:2839` and `:3461` and for the inference gateway; only one template-dispatch path saves its copy (`dispatch.rs:2090`).
Observations recorded on the other instances are lost and the instances disagree.
Fix: one shared `Arc<CascadeRouter>` in `AppState`, handed to the gateway and dispatch paths, with periodic merge-on-save. Needs confirmation of exactly which instances are unsaved.
