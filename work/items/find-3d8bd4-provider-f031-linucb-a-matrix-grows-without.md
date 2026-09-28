+++
id = "find-3d8bd4"
kind = "finding"
title = "LinUCB A matrix grows without bound, exploration bonus approaches zero"
status = "open"
triage = "verified"
severity = "p2"
goal = "learning"
subsystem = ["roko-learn/model_router"]
created = 2026-09-01
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F031"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F031"
anchors = ["crates/roko-learn/src/model_router.rs::update_features_internal", "crates/roko-learn/src/model_router.rs::alpha_for_observations"]
links = { depends_on = [], blocks = [], related = ["bug-6f1685", "bug-8da8ba"], supersedes = [], duplicate_of = "" }
+++
The LinUCB A matrix accumulates `x * x^T` on every observation without decay or sliding window. After thousands of observations, matrix entries grow large, making `A^{-1}` near-zero, and the exploration bonus `sqrt(x^T A^{-1} x)` approaches zero. The bandit becomes effectively greedy (no explorat...

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F031`

How to verify: Confirm in crates/roko-learn/src/model_router.rs whether still true: LinUCB A matrix grows without bound, exploration bonus approaches zero

Verified 2026-09-28: update_features_internal does A += x x^T and b += r x with no decay, forgetting factor or window (crates/roko-learn/src/model_router.rs:1110-1118). Alpha also decays with total observations (:771-777), so the exploration bonus shrinks from both sides. Only the Thompson arms have a discount (:519-521). Severity p2: learning quality, not a broken loop.
