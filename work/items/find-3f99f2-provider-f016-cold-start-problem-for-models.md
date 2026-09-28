+++
id = "find-3f99f2"
kind = "finding"
title = "[provider F016] Cold start problem for models added after router maturity"
status = "open"
triage = "verified"
severity = "p2"
subsystem = ["roko-learn/model_router"]
created = 2026-09-01
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F016"
discovered_from = "audit:tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F016"
anchors = ["crates/roko-learn/src/cascade_router.rs::confidence_scores", "crates/roko-learn/src/cascade_router.rs::select_ucb_model", "crates/roko-learn/src/model_router.rs::alpha_for_observations"]
links = { depends_on = [], blocks = [], related = ["bug-497c2b"], supersedes = [], duplicate_of = "" }
+++
The cascade router transitions from Stage 2 (confidence) to Stage 3 (UCB) after 200+ observations per model. A new model added to the catalog after the router reaches Stage 3 for existing models starts at 0 observations. The UCB exploration bonus is normalized across all arms; a new model with 0...

Imported without verification from:
- `tmp/archive/provider-audit/29-FINDINGS-REGISTER.md#F016`
- `tmp/archive/provider-audit/08-cascade-router.md`

How to verify: Confirm in crates/roko-learn/src/model_router.rs whether still true: Cold start problem for models added after router maturity

Verified 2026-09-28: the cascade stage comes from the global observation count (crates/roko-learn/src/cascade_router.rs:421 stage_for_observations). The Confidence stage scores untried models at 0.2, so they are never preferred (:2689-2693), and a new LinUCB arm competes with alpha already decayed by the router-wide total_observations (model_router.rs:819). There is no per-model forced exploration. Same root cause as bug-497c2b (global stage counter). Severity p2.
