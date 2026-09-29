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
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-serve/src/lib.rs:1206", "crates/roko-serve/src/state.rs:1042", "crates/roko-serve/src/service_factory.rs:245", "crates/roko-serve/src/dispatch.rs::record_cascade_router_observation_at", "crates/roko-gateway/src/gateway.rs:530", "crates/roko-learn/src/feedback_service.rs:694"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -n 'CascadeRouter::new(gateway_models)' crates/roko-serve/src/state.rs && cargo test -p roko-serve shared_cascade_router_persists_gateway_and_feedback_observations"
+++

Besides `AppState.cascade_router` (loaded at `lib.rs:1096`, saved on shutdown), serve builds more routers with `CascadeRouter::load_or_new` in `dispatch.rs:2839` and `:3461` and for the inference gateway; only one template-dispatch path saves its copy (`dispatch.rs:2090`).
Observations recorded on the other instances are lost and the instances disagree.
Fix: one shared `Arc<CascadeRouter>` in `AppState`, handed to the gateway and dispatch paths, with periodic merge-on-save. Needs confirmation of exactly which instances are unsaved.

Verified 2026-09-28 (static check against 3d0ee4d02): Narrower than stated: dispatch.rs:2839 is inside record_cascade_router_observation_at, which loads, records and saves at once (crates/roko-serve/src/dispatch.rs:2839-2843), and dispatch.rs:3461 swaps a fresh load into AppState.cascade_router itself rather than building a separate instance. The unsaved instance is the gateway's: service_factory.rs:245 builds an Arc<CascadeRouter> that crates/roko-gateway/src/gateway.rs:530/:641/:648 records into, and neither service_factory.rs nor roko-gateway calls save. The independent copies (lib.rs:1096, per-observation load/save at dispatch.rs:2839, gateway) still diverge, and their saves are last-writer-wins.

Re-verified 2026-09-29 at d9e79e9d8: unchanged in substance. Correction to the 2026-09-28 note: the gateway does not record into the service_factory.rs:245 router; it records into its own CascadeRouter::new(gateway_models) built at crates/roko-serve/src/state.rs:1041-1042 (since 310860465), which is never loaded from or saved to cascade-router.json. The service_factory.rs:245 router is loaded from disk and receives FeedbackService observations (roko-learn feedback_service.rs:694-699) but is also never saved. dispatch.rs:3461 (now :3466) is inside the test module and can be dropped as an anchor.

## Notes

- **From wk-router (bug-8da8ba, 2026-09-29):** three roko-serve paths record router observations without the write-ahead log that bug-8da8ba adds (`ModelCallJournal` in roko-learn `model_call_feedback.rs`, on `work/bug-8da8ba`). `service_factory.rs` builds its `FeedbackService` with `with_cascade_router` but no `with_cascade_journal` (:255-257 and :440-442). The cached template-dispatch router in `dispatch.rs` (about :2089) calls `observe_model_call_on_router` and then saves the whole router, with no journal entry. Override outcomes reach `CascadeRouter` through its `ForceBackendOverrideRecorder` impl (roko-learn `cascade_router.rs:181`) in memory only. When this item gives serve one shared router, route all three through `ModelCallJournal`, so a crash between an observation and the next save loses nothing.
- Implemented on `work/bug-605a8a` at `d32a4609c`; cargo verification deferred to the batch check.
