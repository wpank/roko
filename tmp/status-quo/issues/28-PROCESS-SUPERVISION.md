# Process Supervision Issues

## Critical

### Zombies on drop — no wait after SIGKILL
- `process.rs:1248-1268`: `force_kill_sync` sends SIGKILL but never `child.wait()`. All children become zombies until parent exits.

### CancelToken depth bug — intermediate parents skipped
- `cancel.rs:136-146`: For chains depth >= 3, only leaf and root monitored. Intermediate parent cancellation doesn't propagate to children.

## High

### No per-task timeout on CLI agent spawn path
- `event_loop.rs:5040-5114`: `spawn_streaming_cli_agent` never wrapped in `tokio::time::timeout`. Only plan-level timeout exists. Single hung agent stalls entire plan.

### ProcessSupervisor not used in Runner v2
- `event_loop.rs`: Agents tracked in plain `HashMap<String, AgentHandle>`. `reap_exited()` never called. `restart_wave()` / OTP restart logic completely unreachable.

### `forward_agent_events` spawned without tracking
- `event_loop.rs:5060-5066`: JoinHandle discarded. If event loop exits before agent closes channel, forwarding task leaks. Not aborted by `stop_all_agents`.

## Medium

### Stderr reader spawned without tracking
- `agent_stream.rs:223-239`: Only stdout reader has stored handle. `AgentHandle::kill` only aborts stdout reader — stderr reader keeps running.

### Detached cancellation task in ProcessSupervisor::spawn
- `process.rs:924-933`: `JoinHandle` immediately dropped. Races with `shutdown_all()`/`kill_all()`. No join or abort during `ProcessSupervisor::drop`.

### `feedback_tasks` drained non-blocking on cancellation
- `event_loop.rs:2277-2278`: `try_join_next()` only harvests completed tasks. Pending writes silently abandoned.

### Event bus broadcast overflow — silent message loss
- `event_bus.rs:226`: `broadcast::send` return value discarded. Slow subscriber gets `RecvError::Lagged(n)` with no handling at any call site.

### Default supervision strategy: `max_restarts: 0`
- `process.rs:130-138`: No caller overrides. All OTP restart machinery is dead code.
