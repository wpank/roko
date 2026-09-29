+++
id = "bug-605a8a"
kind = "bug"
title = "Loading the router with a narrower model list drops or misaligns other models' state"
status = "open"
triage = "verified"
severity = "p2"
goal = "learning"
subsystem = ["roko-learn/cascade-router", "roko-cli/serve-runtime"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-cli/src/serve_runtime.rs:1215", "crates/roko-learn/src/cascade/persistence.rs::migrated_confidence_stats", "crates/roko-learn/src/cascade_router.rs::CascadeRouter::load_or_new", "crates/roko-learn/src/model_router.rs::import_linucb_snapshot"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn narrower_slug_list_keeps_other_models_state' crates/roko-learn/ && cargo test -p roko-learn narrower_slug_list_keeps_other_models_state"
+++

The serve runtime loads the router with just `[model, "claude-haiku-4-5"]` (`serve_runtime.rs:840`) and hands that instance to the routing sink and the run config, which is saved at run end.
Per a local audit, loading from a snapshot keeps only the active slugs' stats and LinUCB arms are imported by position, not slug, so a narrower or reordered list discards other models' counters and can attach one model's arm to another.
Fix: import arms by slug (reset mismatches with a warning), keep inactive slugs' stats on save, and load with the configured cascade slug list.

Partly fixed (checked 2026-09-28 against 3d0ee4d02): Fixed: the serve runtime no longer loads with [model, "claude-haiku-4-5"]; it uses capture_runtime_model_slugs, which is the configured cascade slugs plus the episode model (crates/roko-cli/src/serve_runtime.rs:894-899, learning_helpers.rs:48-56). Still true: loading keeps only active slugs' stats (crates/roko-learn/src/cascade/persistence.rs:139-145), and import_linucb_snapshot copies arms by index, not by slug (crates/roko-learn/src/model_router.rs:1440-1470). Because capture_runtime_model_slugs sorts the list (learning_helpers.rs:53), another loader that uses config order can still attach arms to the wrong model.

Rechecked 2026-09-29: unchanged since the last check. Remaining: persistence.rs::migrated_confidence_stats drops stats for slugs not in the loaded list, so the next save erases them; model_router.rs::import_linucb_snapshot restores arms by index, so any loader whose slug list differs in content or order from the saved one (for example dispatch_v2.rs:139, chat_session.rs:79, roko-execution builder.rs:598, roko-serve lib.rs:1206) can attach one model's arm to another.

## Notes

- Implemented on `work/bug-605a8a` at `0651270df`; cargo verification deferred to the batch check. It builds on `7769a5ae4` (bug-9c88ac): the merge-on-save is what keeps the models a narrower router does not track.
