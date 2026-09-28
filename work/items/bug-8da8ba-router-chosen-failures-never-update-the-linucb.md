+++
id = "bug-8da8ba"
kind = "bug"
title = "Router-chosen failures never update the LinUCB model"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli/runtime-feedback", "roko-learn/cascade-router"]
created = 2026-09-28
updated = 2026-09-28
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-cli/src/runtime_feedback/routing.rs:125"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++

On a failed outcome the routing sink only calls `record_confidence_outcome(model, false)` (`runtime_feedback/routing.rs:125`); per a local audit that path updates counters but never the contextual bandit, so LinUCB learns from successes only.
Persisted router state inspected in that audit showed failures in the totals but none in the bandit arms.
Fix: feed failures to the bandit with reward 0 through the same path as successes; add a test.
