+++
id = "bug-55b151"
kind = "bug"
title = "Cascade router has no observations for providers used in plan execution"
status = "done"
triage = "verified"
severity = "p2"
goal = "learning"
subsystem = ["roko-learn/cascade-router"]
created = 2026-09-18
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "tmp/dogfood/2026-09-18-session.md#ISSUE-10: Cascade router shows all cerebras/groq models as \"unavailable\" (0 obs)"
discovered_from = "audit:tmp/dogfood/2026-09-18-session.md#ISSUE-10: Cascade router shows all cerebras/groq models as \"unavailable\" (0 obs)"
anchors = ["crates/roko-cli/src/runner/types.rs::RunConfig::from_roko_config", "crates/roko-cli/src/graph_execution/plan_runner.rs:941", "crates/roko-cli/src/runtime_feedback/routing.rs::RoutingObservationSink", "crates/roko-learn/src/cascade_router.rs::record_confidence_outcome"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'roko_config.model_slugs_for_cascade()' crates/roko-cli/src/runner/types.rs && grep -q 'RoutingObservationSink::new' crates/roko-cli/src/graph_execution/plan_runner.rs"

[closed]
at = 2026-09-29
commit = "244f564e1"
by = "work sweep 2026-09-29 (static check against HEAD; cargo verify not re-run while the portal plan run held the build lock)"
evidence = "Graph plan runs load the router in RunConfig::from_roko_config (crates/roko-cli/src/runner/types.rs:2531-2546, called from graph_execution/plan_runner.rs:856) with model_slugs_for_cascade(), the API slugs of every configured non-embedding model (roko-core config/schema.rs:950-960); 244f564e1 (2026-09-21) replaced the config-key list whose keys (e.g. cerebras-gptoss) never matched the API slugs outcomes carry (gpt-oss-120b), the likely cause of ISSUE-10/15. Graph dispatch keys outcomes on dispatch.target.model_slug = profile.slug (dispatch_v2.rs:1355-1358; graph_task_dispatch.rs:1504-1507), RoutingObservationSink is attached at plan_runner.rs:941-943 and the router is saved at :1542-1548. Not confirmed by a live run. Live data 2026-09-29: .roko/learn/cascade-router.json (written during today's Graph runs) holds 124 trials for the Cerebras slug gpt-oss-120b, and .roko/learn/efficiency.jsonl has 232 gpt-oss-120b events since the 2026-09-21 fix."
+++
Cerebras/groq models showed 0 observations despite successful plan runs; the router loaded a stale snapshot and plan execution apparently does not record observations for all providers.

Imported without verification from:
- `tmp/dogfood/2026-09-18-session.md#ISSUE-10: Cascade router shows all cerebras/groq models as "unavailable" (0 obs)`
- `tmp/dogfood/2026-09-18-session.md#ISSUE-15: Cascade router shows 0 obs for kimi-k2.6 despite prior successful runs`

How to verify: Run a Graph-engine task on a non-default provider; check router observation counts.

Check on 2026-09-28 was inconclusive: Graph plan execution now has its own routing feedback sink (crates/roko-cli/src/graph_execution/feedback.rs:58, RoutingSink at :358-363, a save at :433; file touched by 725f21e05), so 'plan execution records no router observations' may no longer hold. Observations are keyed by the loaded slug list (record_confidence_outcome returns bool, cascade_router.rs:1455) and could still be dropped for cerebras/groq/kimi slugs missing from that list. Deciding needs the item's live check: run a Graph-engine task on a non-default provider, then compare `roko learn router` observation counts.

Rechecked 2026-09-29 (static): the key/slug mismatch was fixed in 244f564e1 and every Graph plan run records and saves routing observations. Residual risks tracked elsewhere or minor: outcomes for slugs not listed in [models] (for example failover to an unlisted Claude slug on claude_cli) are still dropped with a warning at crates/roko-learn/src/cascade_router.rs:1455-1464, and concurrent writers can still erase observations (bug-9c88ac). The old anchor crates/roko-cli/src/graph_execution/feedback.rs:358 (RoutingSink) is not on a live path: build_settler has no production caller, only the re-export at graph_execution/mod.rs:46 and tests.
