# Dreams and Daimon Issues

## High

### Dreams staging buffer path mismatch — all entries stuck at Raw
- `cycle.rs:436`: Writes to `.roko/dreams/staging-buffer.json`.
- `orchestrate.rs:8259`: Reads from `.roko/dreams/staging.json` (wrong path).
- Result: 20 entries permanently stuck at stage `raw`, confidence `0.20`. Never promoted.

### `reinforce_batch` (income side) never called from orchestrator
- `roko-neuro/src/lifecycle.rs:277`: Built, never called by runner.
- Knowledge entries get taxed (demurrage) but never get income.
- Balances drift toward 0.0 without reinforcement.

## Medium

### No periodic dream trigger
- `maybe_auto_dream()` only fires at plan completion (`orchestrate.rs:8145`).
- No daemon loop, no cron, no background task. Dreams never fire in idle mode.
- `DreamSchedulePolicy::scheduled_cron` is fully implemented but never called.

### Dream routing advice written but never read
- `routing_advice.rs:79`: `generate_routing_advice()` writes file.
- Only consumer is `roko-acp/src/bridge_events.rs:3079` — never called from orchestrate.rs.

### `CrateConfidence` trackers always empty
- `orchestrate.rs:3164`: `extract_crate_name()` heuristic looks for `roko-` prefix.
- Live plans: `self-dev-ux`, `E01-execution-engine` → no match.
- Live state: `crate_trackers: {}`, `crate_confidence_map: {}`.

### Demurrage only runs under `roko serve`, not plan runner
- `roko-serve/src/lib.rs:2077`: Only runtime caller.
- Plan-only runs → knowledge entries never decay.

### `appraise()` return value silently discarded
- `roko-daimon/src/lib.rs:2191`: Returns `PadVector`.
- All 9 call sites use `if let Err(e) = self.daimon.appraise(...)` — never matches.

## Low

### Cross-episode files accumulate indefinitely
- `cycle.rs:2576`: Creates per-run `cross-episode-{timestamp}.json` (~196KB each). No rotation/cleanup.

### Phase-2 stubs entirely disconnected
- `mortality.rs`, `life_review.rs`, `goals.rs`: Zero external call sites.
