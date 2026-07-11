# Learning and Feedback System Issues

## Critical

### LinUCB A/b matrices never persisted — reset on every restart
- `cascade_router.rs:1792-1795`: `snapshot_json()` sets `linucb_state: None`.
- `from_snapshot()` (line 1832): discards `linucb_state`. Only observation count restored.
- After restart: stage stays at UCB but all learned weights are gone → random exploration.

### `record_confidence_outcome` does not feed LinUCB arms
- `cascade_router.rs:1166-1188`: Only updates `confidence_stats` (Stage 2 pass-rate table). Never calls `linucb.update_features()`. Stage 3 bandit receives zero real observations.

### CostsDb is purely in-memory — no runtime persistence
- `costs_db.rs`, `event_subscriber.rs:165`: Starts empty every run, dropped on exit. Cross-run cost aggregation impossible.

## High

### `events.jsonl` is a 44MB write-only firehose
- `persist.rs:54`, `jsonl_logger.rs`: Written on every `RunnerEvent`. Nothing reads it outside tests.
- Real reader uses `runtime-events.jsonl` instead.
- TUI cursor tries to parse entire file as single JSON value → always fails silently.

### Learning subscriber creates fresh router, discards observations
- `orchestrate.rs:8538-8552`: `CascadeRouter::new()` (fresh, no load). Missing `router_persist_path` argument.
- Runner path (`event_loop.rs:981`): Also creates fresh `CascadeRouter::new()` rather than `load_or_new`.

## Medium

### `flush_efficiency_events` reports written count as zero
- `orchestrate.rs:6166-6184`: `len()` called after `drain(..)`. Always logs `written = 0`.

### `cost_usd_without_cache` always equals `cost_usd`
- `orchestrate.rs:19048`, `event_subscriber.rs:183`: Both fields set to same value. Cache savings always 0.

### Efficiency event `iteration` hardcoded 1 on success
- `orchestrate.rs:19076`: Retry-to-success events indistinguishable from first-attempt.

### Episode `started_at`/`completed_at` default to `Utc::now()` on deser
- `episode_logger.rs:214-218`: Old episodes get false wall-clock timestamps on load.
