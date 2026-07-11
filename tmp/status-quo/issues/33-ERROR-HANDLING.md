# Dangerous Error Handling Patterns

## Critical — Reachable `unreachable!()`

### `HealthStatus::Healthy` in orchestrate.rs
- `orchestrate.rs:6892`: Inside health-check polling loop. Prior `continue` at line 6862 is the only guard. Code refactor can reach this → panic kills entire plan run.

### Non-exhaustive `ExecutorAction` catch-all
- `orchestrate.rs:9350`: `_ => unreachable!("non-exhaustive ExecutorAction variant")`. Any new variant panics in hot path.

### workflow_engine.rs catch-all
- `workflow_engine.rs:388`: Only 3 terminal `Phase` variants matched. Enum growth → runtime panic.

## High — Production `todo!()` / `unreachable!()`

### Bench route stubs
- `roko-serve/routes/bench.rs:1130,1157`: `todo!("format the greeting")` and `unimplemented!("wrap_result...")`. Live in serve crate, reachable by bench tooling.

### `plans.rs` route handler
- `roko-serve/routes/plans.rs:989`: `_ => unreachable!("validated above")`. HTTP handler panics rather than returning 4xx.

### Production `.expect()` on fallible lookups
- `orchestrate.rs:17762`: `.expect("ratchet verdict should exist")` after `any()` check.
- `orchestrate.rs:11920`: `.expect("acceptance contract exists")` — unguarded assumption.
- `orchestrate.rs:18570,18598,20411,20420`: `.expect()` on `serde_json::to_value()` — NaN floats or non-string keys will fail.

## Medium — Silent error swallowing

### Agent event pipe drops
- `agent_stream.rs:181,219,231`: `let _ = event_tx.send(...)`. Dropped events = event loop doesn't see agent start/exit → indefinite hangs.

### Silent git merge abort
- `merge.rs:192`: `let _ = git merge --abort`. Failed abort leaves working tree conflicted.

### Audit chain append
- `roko-orchestrator/executor/mod.rs:538`: `let _ = chain.append(entry)`. Phase history may have gaps.

### PID file writes
- `roko-agent/openclaw/gateway_service.rs:227,229`: `let _ = create_dir_all; let _ = write`. Gateway starts but PID not recorded.

### SnapshotWriter flush ack
- `snapshot_writer.rs:130,138,141`: `let _ = flush_tx.send(())`. Flush can return without having actually flushed.
