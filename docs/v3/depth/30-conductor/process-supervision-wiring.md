# Process Supervision Wiring

> Depth file for [30-CONDUCTOR.md](../../30-CONDUCTOR.md) section 14.
> Source: `crates/roko-runtime/`, `crates/roko-cli/src/runner/`

---

## 1. The Problem: Unsupervised Processes

Production batch runs (March-April 2026) exposed three categories of process
management failure:

1. **Spawn races** (Issue #6): Agent exits were confused between retry attempts.
   Without attempt tracking, exit events from dead processes were attributed to
   newly spawned processes.

2. **Orphaned cargo processes** (Issue #7): Timeouts killed the direct child
   process but not its descendants. `cargo check` processes survived their parent's
   death and accumulated, eventually starving CPU and memory.

3. **Cold start overhead** (Issue #8): Every agent turn spawned a new CLI
   subprocess, adding 2-5 seconds of startup overhead per turn. Over hundreds of
   turns, this accumulated to 10-30 minutes of pure waste.

All three failures share a root cause: processes were treated as fire-and-forget
rather than supervised entities. The structural fix is a `ProcessSupervisor` that
owns the full lifecycle of every spawned process (Design Principle #7: Process
isolation with supervision).

---

## 2. ProcessSupervisor Architecture

The `ProcessSupervisor` lives in `roko-runtime` and is wired into the plan
execution pipeline through `PlanRunner`:

```
+---------------------------------------------------+
|                  PlanRunner                        |
|                                                    |
|  +----------------------------------------------+ |
|  |         ProcessSupervisor                     | |
|  |                                               | |
|  |  +---------+  +---------+  +---------+        | |
|  |  | Agent 1 |  | Agent 2 |  | Agent 3 |        | |
|  |  | PID 4201|  | PID 4205|  | PID 4209|        | |
|  |  +----+----+  +----+----+  +----+----+        | |
|  |       |            |            |              | |
|  |  +----+----+  +----+----+  +----+----+        | |
|  |  | cargo   |  | cargo   |  | cargo   |        | |
|  |  | PID 4202|  | PID 4206|  | PID 4210|        | |
|  |  +----+----+  +---------+  +---------+        | |
|  |       |                                        | |
|  |  +----+----+                                   | |
|  |  | rustc   |                                   | |
|  |  | PID 4203|                                   | |
|  |  +---------+                                   | |
|  |                                                | |
|  |  PID Registry: {4201, 4202, 4203,              | |
|  |                  4205, 4206, 4209, 4210}        | |
|  +----------------------------------------------+ |
+---------------------------------------------------+
```

### Core Responsibilities

The ProcessSupervisor provides five guarantees:

1. **PID tracking**: Every spawned process is registered with its PID, parent PID,
   attempt ID, and plan association.

2. **Descendant discovery**: For any registered PID, the supervisor can enumerate
   the full process tree (children, grandchildren, etc.) via platform-specific
   mechanisms.

3. **Lifecycle management**: Spawn, monitor, timeout, and terminate are atomic
   operations on the process tree, not individual processes.

4. **Orphan prevention**: On parent exit, all registered descendants are terminated.
   No process outlives its supervisor.

5. **Attempt isolation**: Each spawn attempt gets a monotonically increasing attempt
   ID. Exit events carry the attempt ID, preventing confusion between retries.

---

## 3. PID Registry

The PID registry is the supervisor's core data structure -- a map from PID to
process metadata:

```rust
struct ProcessEntry {
    pid: u32,
    parent_pid: Option<u32>,
    plan_id: String,
    task_id: String,
    attempt_id: u64,          // monotonically increasing
    spawned_at: Instant,
    status: ProcessStatus,     // Running, Exited, Killed
}

enum ProcessStatus {
    Running,
    Exited { code: i32, at: Instant },
    Killed { signal: i32, at: Instant },
}
```

### Registration Flow

```
PlanRunner starts task "implement-auth"
    |
    +-> Supervisor.spawn(plan_id, task_id, cmd)
    |       |
    |       +-> Increment attempt counter -> attempt_id = 7
    |       +-> Execute command with setsid (new process group)
    |       +-> Register PID 4201, attempt_id=7, plan="plan-42"
    |       +-> Return (pid=4201, attempt_id=7)
    |
    +-> Agent 4201 spawns cargo check
    |       +-> Supervisor detects child PID 4202
    |           Register PID 4202, parent=4201
    |
    +-> Cargo spawns rustc
            +-> Supervisor detects grandchild PID 4203
                Register PID 4203, parent=4202
```

The registry enables precise cleanup: terminating PID 4201 also terminates 4202
and 4203. Without the registry, 4202 and 4203 survive as orphans.

---

## 4. Process Tree Cleanup

### 4.1 The Descendant Problem

Unix process semantics create the orphan problem:

```
Parent (PID 100) spawns Child (PID 200)
Child (PID 200) spawns Grandchild (PID 300)

kill(100) -> Parent dies
           -> Child is reparented to init (PID 1)
           -> Grandchild is reparented to init (PID 1)
           -> Both 200 and 300 continue running as orphans
```

Sending SIGTERM to the parent process does NOT propagate to descendants unless they
are in the same process group and the signal is sent to the group.

### 4.2 kill_all_descendants

The `kill_all_descendants(pid)` function walks the process tree and terminates
processes bottom-up (leaves first, then parents):

```
kill_all_descendants(4201):
    |
    +-> Discover tree: 4201 -> [4202 -> [4203]]
    |
    +-> Kill leaves first:
    |       kill(4203, SIGTERM)    # rustc
    |       wait 100ms
    |       if still alive: kill(4203, SIGKILL)
    |
    +-> Kill intermediate:
    |       kill(4202, SIGTERM)    # cargo
    |       wait 100ms
    |       if still alive: kill(4202, SIGKILL)
    |
    +-> Kill root:
            kill(4201, SIGTERM)    # agent CLI
            wait 100ms
            if still alive: kill(4201, SIGKILL)
```

Bottom-up ordering prevents a common race: if you kill the parent first, children
may detect the parent's death and change behavior (spawn new processes, write
emergency state) before you get to them. Killing leaves first ensures no new
processes are spawned during cleanup.

### 4.3 Platform-Specific Discovery

| Platform | Mechanism | Notes |
|----------|-----------|-------|
| Linux | cgroups | Most reliable -- kernel tracks all processes in the group |
| Linux (fallback) | `/proc/{pid}/task/*/children` | Reads kernel process tree directly |
| macOS | `pgrep -P {pid}` recursive | Walks the tree via parent PID relationships |
| macOS (fallback) | `ps -o pid,ppid` + manual tree construction | Parses process table |

The cgroups approach on Linux is strongest: a process cannot escape its cgroup, so
even double-forked processes are captured. On macOS, the `pgrep` approach can miss
processes that have changed their parent PID (via `setsid` or double-fork). The
periodic orphan sweep catches these stragglers.

---

## 5. SIGTERM to SIGKILL Escalation

The two-phase kill protocol gives processes a chance to clean up before forced
termination:

```
Phase 1: SIGTERM (graceful)
    +-> Process receives SIGTERM
    +-> Grace period starts (configurable, default 5s)
    +-> Process may: write checkpoint, flush buffers, close connections, exit
    +-> Grace period expires

Phase 2: SIGKILL (forced)
    +-> Process receives SIGKILL
    +-> Cannot be caught or ignored
    +-> Process terminated immediately
```

### Grace Period Configuration

| Process Type | Grace Period | Rationale |
|-------------|-------------|-----------|
| Agent CLI | 5s | Needs time to flush output, write session state |
| cargo check | 2s | Build processes have no important state to save |
| cargo test | 3s | May need to write partial test results |
| Gate scripts | 2s | Verification scripts are stateless |
| rustc | 1s | Compiler has no user-facing state |

---

## 6. Process Group Management

### 6.1 setsid for Isolation

Every agent process is spawned in its own process group using `setsid`:

```rust
let child = Command::new("claude")
    .args(&["--cli", "--model", model])
    .pre_exec(|| {
        // Create new session and process group
        unsafe { libc::setsid() };
        Ok(())
    })
    .spawn()?;
```

This provides two benefits:

1. **Signal isolation**: Signals sent to the orchestrator's process group do not
   propagate to agent process groups. A Ctrl+C in the terminal kills the
   orchestrator, which then gracefully shuts down agents via the supervisor -- not
   by SIGINT propagation.

2. **Group kill**: The supervisor can send signals to the entire process group with
   `kill(-pgid, signal)`, catching all processes in the group with a single system
   call.

---

## 7. Orphan Reaper

The orphan reaper is a background task that periodically scans for processes that
should have been cleaned up but were not:

```
Every 30 seconds:
    |
    +-> For each entry in PID registry where status == Running:
    |       +-> Check if process is still alive (kill(pid, 0))
    |       +-> If dead: update registry status to Exited
    |       +-> If alive AND parent task is complete/failed:
    |               +-> This is an orphan -- kill it
    |                   kill_all_descendants(pid)
    |                   Update registry status to Killed
    |
    +-> Scan for unregistered processes:
            +-> List all processes owned by current user
            +-> Filter to known agent executables (claude, cargo, rustc)
            +-> Check if any are NOT in the PID registry
            +-> If found: log warning, optionally kill
```

### Orphan Detection Heuristics

| Signal | Meaning |
|--------|---------|
| Parent PID is 1 (init) | Process was reparented -- original parent died |
| Task is in Completed/Failed state | Process outlived its task |
| Plan is in Failed/Aborted state | Process outlived its plan |
| No registry entry exists | Process was spawned without supervision |

---

## 8. Attempt Tracking

Attempt tracking eliminates spawn races (Issue #6) by associating every exit event
with a specific spawn attempt:

```
Attempt 1: spawn agent -> PID 4201, attempt_id=1
    +-> Agent produces near-zero output, exits
    +-> Exit event: (pid=4201, attempt_id=1, code=1)

Attempt 2: spawn agent -> PID 4205, attempt_id=2
    +-> Stale exit event arrives: (pid=4201, attempt_id=1, code=1)
    |       Supervisor checks: current_attempt_id=2, event_attempt_id=1
    |       -> Stale event, ignore
    +-> Agent completes successfully, exits
    +-> Exit event: (pid=4205, attempt_id=2, code=0)
            Supervisor checks: current_attempt_id=2, event_attempt_id=2
            -> Current event, process
```

### Spawn Backoff

| Attempt | Backoff | Rationale |
|---------|---------|-----------|
| 1 | 0s (immediate) | First attempt -- no delay needed |
| 2 | 2s | Brief cooldown, clears event queue |
| 3 | 4s | Longer cooldown, system may need time to stabilize |
| 4 | 30s | Extended cooldown -- persistent failure likely |
| 5+ | 60s | Maximum backoff -- prevent thrashing |

---

## 9. Graceful Shutdown Sequence

When the orchestrator shuts down (user Ctrl+C, budget exhaustion, or all tasks
complete), the supervisor executes a four-phase shutdown:

```
Phase 1: Stop Accepting (immediate)
    +-> Set supervisor.accepting_spawns = false
    +-> No new processes can be spawned

Phase 2: Drain Active (configurable timeout, default 30s)
    +-> Send SIGTERM to all registered Running processes
    +-> Wait for processes to exit cleanly
    +-> Every 5s: log "shutdown: {remaining} processes still active"
    +-> If drain timeout expires -> proceed to Phase 3

Phase 3: Force Kill (5s)
    +-> For each still-Running process:
            kill_all_descendants(pid)  // SIGTERM + SIGKILL

Phase 4: Checkpoint and Flush (2s)
    +-> Write final PID registry state to disk
    +-> Flush all log buffers
    +-> Write executor checkpoint (for --resume)
    +-> Exit
```

### Atomic Checkpoint Writes

The checkpoint write uses temp-file-then-rename to prevent corruption from
mid-write crashes:

```rust
fn save_snapshot_atomic(snapshot: &ExecutorSnapshot, path: &Path) -> Result<()> {
    let tmp_path = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(snapshot)?;
    std::fs::write(&tmp_path, &json)?;
    std::fs::rename(&tmp_path, path)?;  // Atomic on POSIX
    Ok(())
}
```

---

## 10. Integration with the Conductor

The ProcessSupervisor and Conductor operate at different levels but complement
each other:

| Aspect | ProcessSupervisor | Conductor |
|--------|------------------|-----------|
| Level | Process (OS-level) | Plan (task-level) |
| Monitors | PIDs, exit codes, resource usage | Watcher signals, quality metrics |
| Detects | Orphans, crashes, OOM, hangs | Stuck patterns, cost spikes, loops |
| Responds | Kill, restart process | Restart plan, fail plan |
| Persistence | PID registry in memory | Circuit breaker in DashMap |

The Conductor decides; the Supervisor executes:

```
Conductor: "Agent for plan-42 has 3 ghost turns -> restart"
    +-> Supervisor.kill_all_descendants(agent_pid_for_plan_42)
        Supervisor.spawn(plan_42, task, cmd)  // fresh attempt
```

---

## 11. File Reference

| File | What |
|------|------|
| `crates/roko-runtime/` | ProcessSupervisor, event bus, cancellation |
| `crates/roko-cli/src/runner/` | PlanRunner that uses the supervisor |
| `crates/roko-conductor/src/conductor.rs` | Conductor that issues decisions the supervisor executes |
