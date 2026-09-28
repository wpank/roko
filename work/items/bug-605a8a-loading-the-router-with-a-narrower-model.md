+++
id = "bug-605a8a"
kind = "bug"
title = "Loading the router with a narrower model list drops or misaligns other models' state"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-learn/cascade-router", "roko-cli/serve-runtime"]
created = 2026-09-28
updated = 2026-09-28
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-cli/src/serve_runtime.rs:840", "crates/roko-learn/src/cascade/persistence.rs"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

The serve runtime loads the router with just `[model, "claude-haiku-4-5"]` (`serve_runtime.rs:840`) and hands that instance to the routing sink and the run config, which is saved at run end.
Per a local audit, loading from a snapshot keeps only the active slugs' stats and LinUCB arms are imported by position, not slug, so a narrower or reordered list discards other models' counters and can attach one model's arm to another.
Fix: import arms by slug (reset mismatches with a warning), keep inactive slugs' stats on save, and load with the configured cascade slug list.
