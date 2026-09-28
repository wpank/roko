+++
id = "bug-f68404"
kind = "bug"
title = "Manual --model overrides are always recorded as router successes"
status = "open"
triage = "verified"
severity = "p2"
goal = "learning"
subsystem = ["roko-learn/cascade-router"]
created = 2026-09-28
updated = 2026-09-28
last_verified = 2026-09-28
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-learn/src/cascade_router.rs::CascadeRouter::record_override_outcome", "crates/roko-learn/src/cascade_router.rs::CascadeRouter::observe_multi_objective"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'cargo test -p roko-learn cascade_router'
+++

`record_override_outcome` (`cascade_router.rs:1540`) forwards to `observe_multi_objective` (`:1703`), which increments both `trials` and `successes` unconditionally (`:1718-1719`); cost and latency are passed as 0.0.
A failed override run therefore counts as a free, instant success, and `override_learning_dampening` shrinks the reward instead of the observation weight.
Fix: honour the success flag, record real cost/latency, apply dampening as an importance weight, and test that a failed override lowers the arm's success rate.
