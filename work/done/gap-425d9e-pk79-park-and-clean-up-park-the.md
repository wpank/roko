+++
id = "gap-425d9e"
kind = "gap"
title = "PK79 Park and clean up: Park the chain-family HTTP routes behind the `chain` feature, with one typed 501… (+7 more)"
status = "done"
triage = "verified"
severity = "p2"
goal = "tooling"
rank = 79
size = "L"
subsystem = ["workspace/build"]
created = 2026-10-02
updated = 2026-10-03
last_verified = 2026-10-03
last_verified_rev = "059450273"
source = "tmp/backlog/2026-10-02-complete-and-wire PK79"
anchors = [".github/workflows/ci.yml", "crates/roko-cli/Cargo.toml", "crates/roko-core/src/obs/mod.rs", "crates/roko-runtime/Cargo.toml", "crates/roko-runtime/src/lib.rs", "crates/roko-serve/Cargo.toml", "crates/roko-serve/src/feed_agents/mod.rs", "crates/roko-serve/src/job_runner.rs", "crates/roko-serve/src/lib.rs", "crates/roko-serve/src/routes/chain_disabled.rs", "crates/roko-serve/src/routes/feeds.rs", "crates/roko-serve/src/routes/meta.rs", "crates/roko-serve/src/routes/middleware.rs", "crates/roko-serve/src/routes/mod.rs", "crates/roko-serve/src/state.rs", "crates/roko-serve/src/trigger_runtime.rs", "tools/http_route_inventory.snapshot.json"]
lane = "rust-cold"
parent = "spec-0b3a32"
links = { depends_on = ["gap-aea13a"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn chain_family_routes_return_501_without_chain_feature' crates/roko-serve/ && cargo test -p roko-serve --no-default-features --features hdc chain_family_routes_return_501_without_chain_feature"

[[verify]]
command = "grep -rqw 'fn paid_feed_is_refused_without_chain_feature' crates/roko-serve/ && cargo test -p roko-serve --no-default-features --features hdc paid_feed_is_refused_without_chain_feature"

[[verify]]
command = "grep -rqw 'fn chain_feed_agents_absent_without_chain_feature' crates/roko-serve/ && cargo test -p roko-serve --no-default-features --features hdc chain_feed_agents_absent_without_chain_feature"

[[verify]]
command = "grep -Eq '^roko-chain = \\{ path = \"../roko-chain\", optional = true' crates/roko-serve/Cargo.toml && grep -q 'roko-serve --no-default-features' .github/workflows/ci.yml && grep -rqw 'fn chain_monitor_job_fails_with_feature_hint_without_chain' crates/roko-serve/ && cargo test -p roko-serve --no-default-features --features hdc chain_monitor_job_fails_with_feature_hint_without_chain"

[[verify]]
command = "! grep -qE '^default = \\[[^]]*\"chain\"' crates/roko-cli/Cargo.toml crates/roko-serve/Cargo.toml && grep -q 'roko-cli --features chain' .github/workflows/ci.yml"

[[verify]]
command = "grep -rqw 'fn group_routes_return_501_by_default' crates/roko-serve/ && cargo test -p roko-serve group_routes_return_501_by_default"

[[verify]]
command = "grep -rqw 'fn relay_routes_return_501_by_default' crates/roko-serve/ && cargo test -p roko-serve relay_routes_return_501_by_default"

[[verify]]
command = "test ! -e crates/roko-core/src/obs/mod.rs && grep -q '^cognitive-clock' crates/roko-runtime/Cargo.toml && grep -B1 'pub mod heartbeat_attention' crates/roko-runtime/src/lib.rs | grep -q 'feature = \"cognitive-clock\"'"

[closed]
at = 2026-10-03
at_ts = "2026-10-03T04:05:53Z"
commit = "059450273"
executor = "claude-agent"
via = "work-batch"
size = "L"
claimed_at = "2026-10-03T01:27:36Z"
forced = false
evidence = "Gate 6b (work/backlog-batch-6b, merged into main as 059450273): cargo check --workspace --tests, nightly fmt, cargo clippy --workspace -D warnings on the chainless default build, nextest --lib 11,842 passed over 10 crates (roko-acp, -agent, -agent-server, -cli, -core, -gate, -graph, -learn, -runtime, -serve), roko-cli bin 429 passed and the golden-path canaries pass (plan_validate: only bug-2a31bc's two known alias tests fail), roko-agent sse_replay + provider_parity and roko-learn legacy_rule_live + loop_audit_cs_reference pass, PK79's CI feature checks pass; every [[verify]] passes. PK79 8/8: the default roko-cli tree has neither roko-chain nor the Alloy provider graph; alloy-backend+acp, roko-serve --no-default-features --features hdc, roko-cli --features chain, roko-std chain handlers, roko-serve groups and relay, roko-runtime cognitive-clock all build, and provider_parity passes with chain + roko-std/chain. Gate fixes for the chainless build in f5de1bd52."
+++

## Problem

This package delivers 8 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK79, slice 92xx, phase 9), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 9214 | M | p3 | Park the chain-family HTTP routes behind the `chain` feature, with one typed 501 router otherwise | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9214-park-chain-family-routes-behind-chain-feature.md` |
| 2 | 9215 | S | p2 | Paid feeds must fail closed when serve is built without `chain` (the x402 check) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9215-x402-paid-feeds-fail-closed-without-chain.md` |
| 3 | 9216 | M | p3 | Gate serve's `ChainState`, block watcher and seven chain feed agents behind `chain` | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9216-gate-chain-state-and-chain-feed-agents.md` |
| 4 | 9217 | M | p3 | Make `roko-chain` an optional dependency of roko-serve, and check the chain-less build in CI | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9217-make-roko-chain-optional-in-roko-serve.md` |
| 5 | 9218 | S | p3 | Take `chain` out of the default features of roko-cli and roko-serve | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9218-take-chain-out-of-default-features.md` |
| 6 | 9219 | M | p3 | Park agent groups and pheromone state behind a `groups` feature of roko-serve | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9219-park-agent-groups-behind-groups-feature.md` |
| 7 | 9220 | M | p3 | Park serve's relay (registration, feed bridge, relay proxy) behind a `relay` feature | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9220-park-relay-behind-relay-feature.md` |
| 8 | 9221 | S | p3 | Park the uncalled heartbeat attention auction and heartbeat probes; delete roko-core's orphan `obs/mod.rs` | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9221-park-heartbeat-attention-and-probes.md` |

## Why it matters

Phase 9: domains, assistant, held and parked work, cleanup, showcase and deploy. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9200-held-parked-and-cleanup.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `.github/workflows/ci.yml`, `crates/roko-cli/Cargo.toml`, `crates/roko-core/src/obs/mod.rs`, `crates/roko-runtime/Cargo.toml`, `crates/roko-runtime/src/lib.rs`, `crates/roko-serve/Cargo.toml`, `crates/roko-serve/src/feed_agents/mod.rs`, `crates/roko-serve/src/job_runner.rs`, `crates/roko-serve/src/lib.rs`, `crates/roko-serve/src/routes/chain_disabled.rs`, `crates/roko-serve/src/routes/feeds.rs`, `crates/roko-serve/src/routes/meta.rs`, `crates/roko-serve/src/routes/middleware.rs`, `crates/roko-serve/src/routes/mod.rs`, `crates/roko-serve/src/state.rs`, `crates/roko-serve/src/trigger_runtime.rs`, `crates/roko-serve/tests/parked_routes.rs`, `tools/http_route_inventory.snapshot.json`.

## Current state

The tasks were checked against `2c3ea9f73` on 2026-10-02. Re-check each task's anchors and premise at your base commit before implementing it, and report a task that is already done instead of redoing it.

## Plan

1. Work through the tasks in the order above. For each: read its file, implement its Plan, write the test it names, and make one commit per task whose message ends with `Backlog-Task: <task id>`, `Work-Item: <this item's id>` and `Executor: claude-agent`.
2. Follow `BUILD-RULES.md` in the backlog folder. Workers run no cargo: Rust is checked by the coordinator's batched gate. Python and doc checks you may run.
3. If a task cannot be done (a premise is false, a decision is missing, or its verify cannot pass), stop at that task, keep the earlier commits, and report it; do not skip ahead to tasks that depend on it.

## Done when

- Every task's verify command passes (this item's `[[verify]]` list, one entry per task), after the coordinator's batched gate.
- Each task's own "Done when" holds (see its file).

## Notes

- Waits on: PK33 (gap-aea13a).
- Suggested model: opus.

## Progress

Implemented on `work/gap-425d9e`; cargo verification is deferred to the batch gate.

- 9214: implemented at 4dab209c9
- 9215: implemented at 1b426fc4b
- 9216: implemented at 2f8a649c7
- 9217: implemented at 60659da57
- 9218: implemented at 242115dfe
- 9219: implemented at fe2d1eb81
- 9220: implemented at ebbd5166c (subscription_relay.rs stays compiled: its status types back /api/subscriptions/relay/status and the OpenAPI document)
- 9221: implemented at 38d5eadc1
