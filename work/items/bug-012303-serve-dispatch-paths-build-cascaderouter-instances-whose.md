+++
id = "bug-012303"
kind = "bug"
title = "Serve dispatch paths build CascadeRouter instances whose learning is never saved"
status = "open"
triage = "verified"
severity = "p2"
goal = "learning"
subsystem = ["roko-serve/dispatch", "roko-learn/cascade-router"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-serve/src/dispatch.rs:2839", "crates/roko-serve/src/dispatch.rs:3461", "crates/roko-serve/src/lib.rs:1096", "crates/roko-serve/src/service_factory.rs:245", "crates/roko-gateway/src/gateway.rs:530"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

Besides `AppState.cascade_router` (loaded at `lib.rs:1096`, saved on shutdown), serve builds more routers with `CascadeRouter::load_or_new` in `dispatch.rs:2839` and `:3461` and for the inference gateway; only one template-dispatch path saves its copy (`dispatch.rs:2090`).
Observations recorded on the other instances are lost and the instances disagree.
Fix: one shared `Arc<CascadeRouter>` in `AppState`, handed to the gateway and dispatch paths, with periodic merge-on-save. Needs confirmation of exactly which instances are unsaved.

Verified 2026-09-28 (static check against 3d0ee4d02): Narrower than stated: dispatch.rs:2839 is inside record_cascade_router_observation_at, which loads, records and saves at once (crates/roko-serve/src/dispatch.rs:2839-2843), and dispatch.rs:3461 swaps a fresh load into AppState.cascade_router itself rather than building a separate instance. The unsaved instance is the gateway's: service_factory.rs:245 builds an Arc<CascadeRouter> that crates/roko-gateway/src/gateway.rs:530/:641/:648 records into, and neither service_factory.rs nor roko-gateway calls save. The independent copies (lib.rs:1096, per-observation load/save at dispatch.rs:2839, gateway) still diverge, and their saves are last-writer-wins.
