# StateHub → TUI Audit: `plan run --approval`

> Exhaustive end-to-end audit of the only TUI workflow that matters right now.
> Everything else (serve --tui, dashboard) can wait.

## The Architecture Today

```
Orchestrator Thread (tokio async)      TUI Thread (std::thread, no tokio)
  │                                      │
  ├─ emit_server_event(ServerEvent)      │
  │   └─ server_event_to_dashboard()     │
  │       └─ state_hub.publish()    ──►  drain_snapshot_channel()
  │           (watch::Sender)            │   └─ rx.borrow_and_update()
  │                                      │       └─ apply_dashboard_snapshot()
  ├─ publish_dashboard_event()      ──►  │           └─ tui_state.update_from_dashboard_snapshot()
  │   └─ state_hub.publish()             │
  │                                      │
  ├─ approval_tx.send(request)      ──►  drain_approval_requests()
  │   (mpsc, 16-msg buffer)             │   └─ accept_approval_request()
  │                                      │       └─ show Approval modal
  │   ◄── response_tx.send(bool)   ───  │   └─ user presses [y]/[n]
  │   (oneshot, NO timeout!)             │       └─ resolve_active_approval()
  │                                      │
  └─ (resumes dispatch)                 terminal.draw(|f| self.draw(f))
                                         (crossterm 60fps, ratatui double-buffer)
```

**Key files**:
- `crates/roko-cli/src/main.rs:5804-5832` — TUI thread spawn + channel setup
- `crates/roko-cli/src/orchestrate.rs:5118` — `emit_server_event()` (path 1)
- `crates/roko-cli/src/orchestrate.rs:5168` — `publish_dashboard_event()` (path 2)
- `crates/roko-cli/src/orchestrate.rs:18094` — `server_event_to_dashboard()` conversion
- `crates/roko-core/src/dashboard_snapshot.rs:800` — `apply()` event handler
- `crates/roko-core/src/state_hub.rs:134` — `publish()` → `snapshot_tx.send_modify()`
- `crates/roko-cli/src/tui/app.rs:2723` — `drain_snapshot_channel()`
- `crates/roko-cli/src/tui/state.rs:1978` — `update_from_dashboard_snapshot()`

---

## What's Broken (7 disconnects)

### D1: Task titles are empty strings
- **Emit site**: `orchestrate.rs:5224` — `ExecutionEvent::TaskStarted { task_id, phase }` has no title
- **Conversion**: `orchestrate.rs:18137` — `title: String::new()` hardcoded in `server_event_to_dashboard()`
- **Snapshot**: `dashboard_snapshot.rs:847` — `TaskState.title = title.clone()` (receives empty string)
- **TuiState**: `tui/state.rs:2048` — `TaskEntry.name = task.task_id.clone()` (ignores title entirely)
- **TuiState**: `tui/state.rs:2054` — `TaskRow.title` uses correct fallback logic, but TaskEntry doesn't
- **User sees**: "plan plan" instead of "Baseline verification snapshot"
- **Root cause**: `server_event_to_dashboard` is a free function (no access to task_trackers), and `ExecutionEvent::TaskStarted` doesn't carry title

### D2: Agent model is never transmitted
- **Event def**: `dashboard_snapshot.rs:52` — `AgentSpawned { agent_id: String, role: String }` — no model field
- **Server event def**: `roko-serve/src/events.rs:92` — `AgentSpawned { agent_id: String, role: String }` — also no model
- **Emit site**: `orchestrate.rs:7708` — `emit_server_event(ServerEvent::AgentSpawned { agent_id, role })` — model available at this point (e.g. from `selected_model` variable) but not included
- **Snapshot apply**: `dashboard_snapshot.rs:922` — `model: String::new()` initialized on insert
- **EfficiencyEvent("model")**: `dashboard_snapshot.rs:1010-1012` — explicitly skipped with comment "model name encoded as hash — skip (set via AgentSpawned or direct)" — but AgentSpawned doesn't have it
- **User sees**: Model column shows "-" in agent roster

### D3: Plan names duplicate plan_id
- **Snapshot**: `PlanState` has only `plan_id`, no title/name field
- **TuiState**: `tui/state.rs:2083` — `PlanEntry.name = plan.plan_id.clone()` — always the ID
- **No event** carries a plan title — `DashboardEvent::PlanStarted { plan_id }` has only the ID
- **User sees**: "unified-migration-phase0 run" repeated — no human-readable title

### D4: Agent-to-task binding broken (current_task never set)
- **Snapshot apply**: `dashboard_snapshot.rs:857-862` — on TaskStarted, sets `agent.current_task` for agents where `agent.role == *phase`
- **But**: `phase` is something like `"implementing"` and `role` is `"Implementer"` — **these don't match**
- **Result**: `agent.current_task` and `agent.current_plan` are never set
- **Downstream**: `find_agent_key_for_task()` (line 1112) can't match agents to tasks by assignment
- **Fallback**: Line 1122 — if exactly 1 active agent, uses it. Otherwise returns `None` (tokens lost)
- **Impact**: EfficiencyEvent token updates may silently fail to find the right agent
- **User sees**: Token counters at 0 even when EfficiencyEvents are published

### D5: Two event paths create confusion
- **Path 1**: `emit_server_event()` (line 5118) → ServerEvent → `server_event_to_dashboard()` → DashboardEvent → StateHub
- **Path 2**: `publish_dashboard_event()` (line 5168) → DashboardEvent → StateHub directly
- Some events (PlanStarted, AgentSpawned, TaskStarted) go through Path 1 only
- Some events (EfficiencyEvent, EpisodeRecorded, CFactorTrend) go through Path 2 only
- The `server_event_to_dashboard()` conversion is lossy (title dropped, model dropped)
- **No single place** to see all fields the TUI receives

### D6: Log bar garbled by tracing → stderr leak
- TUI enters crossterm raw mode + alternate screen (app.rs `enter_terminal()`)
- Tracing subscriber set up at app.rs:372-389 to write to `.roko/tui.log`
- **But**: the orchestrator thread also has tracing output going to stderr
- During `plan run --approval`, BOTH threads produce tracing output
- TUI thread's tracing goes to file (correct)
- Orchestrator thread's tracing goes to stderr (corrupts raw terminal)
- Ratatui's double-buffer doesn't know about extraneous stderr output
- **User sees**: garbled text overlapping the status bar at bottom

### D7: No approval timeout — orchestrator can hang forever
- `orchestrate.rs:5206` — `Ok(response_rx.await.unwrap_or(false))`
- If TUI thread panics or is killed, the oneshot channel drops
- `.await` on a dropped receiver returns `Err(Canceled)` → `unwrap_or(false)` → rejection
- **But**: if the TUI thread is alive but unresponsive (e.g. blocked on I/O), this hangs forever
- No `tokio::time::timeout()` wrapping the await

---

## Concrete Changes (with exact locations)

### C1: Add model to AgentSpawned

**Files to change** (4 files):

1. `crates/roko-core/src/dashboard_snapshot.rs:52`:
   ```rust
   // BEFORE:
   AgentSpawned { agent_id: String, role: String },
   // AFTER:
   AgentSpawned { agent_id: String, role: String, #[serde(default)] model: String },
   ```

2. `crates/roko-core/src/dashboard_snapshot.rs:899` (apply handler):
   ```rust
   // BEFORE:
   DashboardEvent::AgentSpawned { agent_id, role } => {
   // AFTER:
   DashboardEvent::AgentSpawned { agent_id, role, model } => {
   ```
   And in the Vacant insert (line 922): `model: model.clone(),` instead of `model: String::new(),`
   And in the Occupied update (after line 911): `if !model.is_empty() { agent.model.clone_from(model); }`

3. `crates/roko-serve/src/events.rs:92`:
   ```rust
   // BEFORE:
   AgentSpawned { agent_id: String, role: String },
   // AFTER:
   AgentSpawned { agent_id: String, role: String, #[serde(default)] model: String },
   ```

4. `crates/roko-cli/src/orchestrate.rs:7708`:
   ```rust
   // BEFORE:
   self.emit_server_event(ServerEvent::AgentSpawned {
       agent_id: format!("{plan_id}:{task}"),
       role: format!("{role:?}"),
   });
   // AFTER:
   self.emit_server_event(ServerEvent::AgentSpawned {
       agent_id: format!("{plan_id}:{task}"),
       role: format!("{role:?}"),
       model: selected_model.clone(),  // selected_model is available here
   });
   ```
   **Note**: `selected_model` may not be in scope at line 7708. Search nearby for where the model is determined. The dispatch flow computes `selected_model` around line 14974+. The emit at 7708 is in `dispatch_action` which is BEFORE agent dispatch. Need to verify that selected_model is available, or emit AgentSpawned AFTER model selection in dispatch_agent_with.

5. `crates/roko-cli/src/orchestrate.rs:18108` (conversion):
   ```rust
   // BEFORE:
   ServerEvent::AgentSpawned { agent_id, role } => Some(DashboardEvent::AgentSpawned {
       agent_id: agent_id.clone(),
       role: role.clone(),
   }),
   // AFTER:
   ServerEvent::AgentSpawned { agent_id, role, model } => Some(DashboardEvent::AgentSpawned {
       agent_id: agent_id.clone(),
       role: role.clone(),
       model: model.clone(),
   }),
   ```

6. **Also update ALL match patterns** that destructure AgentSpawned:
   ```bash
   grep -rn 'AgentSpawned {' crates/ --include='*.rs' | grep -v target
   ```
   Each needs the new `model` field (or `..` to ignore it).

7. **Also update test constructions**:
   ```bash
   grep -rn 'AgentSpawned {' crates/ --include='*.rs' | grep -v target | grep test
   ```

**Verification**: `cargo check --workspace`

---

### C2: Populate title in TaskStarted

**The problem**: `server_event_to_dashboard()` (line 18094) is a FREE function with no access to `self` or task_trackers. It can't look up the title.

**Recommended approach**: Add title to `ExecutionEvent::TaskStarted`.

**Files to change** (3 files):

1. `crates/roko-serve/src/events.rs` — find `ExecutionEvent::TaskStarted`:
   ```rust
   // BEFORE:
   TaskStarted { task_id: String, phase: String },
   // AFTER:
   TaskStarted { task_id: String, title: String, phase: String },
   ```

2. `crates/roko-cli/src/orchestrate.rs:5224` — where TaskStarted is emitted:
   ```rust
   // BEFORE:
   ExecutionEvent::TaskStarted {
       task_id: task_id.to_string(),
       phase: new_phase_label,
   }
   // AFTER:
   ExecutionEvent::TaskStarted {
       task_id: task_id.to_string(),
       title: self.task_trackers
           .get(plan_id)
           .and_then(|t| t.tasks_file.tasks.iter().find(|td| td.id == *task_id))
           .map(|td| td.title.clone())
           .unwrap_or_default(),
       phase: new_phase_label,
   }
   ```

3. `crates/roko-cli/src/orchestrate.rs:18133` — conversion uses the title:
   ```rust
   // BEFORE:
   ExecutionEvent::TaskStarted { task_id, phase } => {
       Some(DashboardEvent::TaskStarted {
           plan_id: plan_id.clone(),
           task_id: task_id.clone(),
           title: String::new(),
           phase: phase.clone(),
       })
   }
   // AFTER:
   ExecutionEvent::TaskStarted { task_id, title, phase } => {
       Some(DashboardEvent::TaskStarted {
           plan_id: plan_id.clone(),
           task_id: task_id.clone(),
           title: title.clone(),
           phase: phase.clone(),
       })
   }
   ```

4. **Also update** any other match patterns on `ExecutionEvent::TaskStarted`:
   ```bash
   grep -rn 'ExecutionEvent::TaskStarted' crates/ --include='*.rs' | grep -v target
   ```

**Verification**: `cargo check --workspace`

---

### C3: Fix TaskEntry.name to use title

**File**: `crates/roko-cli/src/tui/state.rs:2048`

```rust
// BEFORE:
TaskEntry {
    id: task.task_id.clone(),
    name: task.task_id.clone(),
    // ...
}

// AFTER:
TaskEntry {
    id: task.task_id.clone(),
    name: if task.title.is_empty() { task.task_id.clone() } else { task.title.clone() },
    // ...
}
```

**Verification**: `cargo check -p roko-cli`

---

### C4: Fix agent-to-task binding (role vs phase mismatch)

**File**: `crates/roko-core/src/dashboard_snapshot.rs:857-862`

```rust
// BEFORE (line 858):
if agent.active && agent.role == *phase {
    agent.current_task = task_id.clone();
    agent.current_plan = plan_id.clone();
}

// PROBLEM: phase = "implementing", role = "Implementer" — never matches
```

**Options**:
- **Option A**: Normalize comparison (lowercase both, strip suffixes)
- **Option B**: Pass role explicitly in TaskStarted event
- **Option C**: Match by agent_id pattern (`{plan_id}:{task_id}`)

**Recommended (Option C)** — match by agent_id prefix:
```rust
// AFTER:
for agent in self.agents.values_mut() {
    if agent.active && agent.agent_id.starts_with(&format!("{plan_id}:")) {
        agent.current_task = task_id.clone();
        agent.current_plan = plan_id.clone();
    }
}
```

This works because the agent_id is formatted as `{plan_id}:{task}` at orchestrate.rs:7709.

**Verification**: `cargo check -p roko-core && cargo test -p roko-core`

---

### C5: Fix EfficiencyEvent("model") to set model on agent

**File**: `crates/roko-core/src/dashboard_snapshot.rs:1010-1012`

```rust
// BEFORE:
"model" => {
    // model name encoded as hash — skip (set via AgentSpawned or direct).
}

// AFTER:
"model" => {
    // Set model name on the matching agent.
    if let Some(agent) = agent_key.as_deref().and_then(|k| self.agents.get_mut(k)) {
        if agent.model.is_empty() {
            agent.model = format!("{value}");
        }
    }
}
```

**Note**: This is a backup path. C1 (AgentSpawned with model) is the primary fix.
The value for "model" metric might be encoded as a float (hash) not a string —
verify what `orchestrate.rs` publishes for this metric. If it's a hash, this path
won't work and should be removed entirely.

**Verification**: `cargo check -p roko-core`

---

### C6: Fix log bar garbling

**Root cause**: Orchestrator thread writes tracing output to stderr while TUI has raw mode active.

**File**: `crates/roko-cli/src/main.rs:5806-5820` (TUI thread spawn area)

**Recommended fix**: Before spawning the TUI thread, redirect stderr to a log file:

```rust
// Before the TUI thread spawn:
if approval {
    // Redirect stderr so tracing from orchestrator thread doesn't corrupt TUI
    let stderr_log = std::fs::File::create(wd.join(".roko").join("stderr.log"))
        .context("create stderr log")?;
    // On Unix: dup2 the file descriptor over stderr
    #[cfg(unix)]
    {
        use std::os::unix::io::AsRawFd;
        unsafe { libc::dup2(stderr_log.as_raw_fd(), 2); }
    }
}
```

**Alternative** (cleaner but more work): Install a tracing subscriber that captures
ALL output to a ring buffer, displayed by the TUI's Logs tab. The TUI already has
a Logs tab (F5:logs) — wire it to a shared ring instead of .roko/tui.log file.

**Verification**: Manual — run `plan run --approval` and check log bar is clean.

---

### C7: Add approval timeout

**File**: `crates/roko-cli/src/orchestrate.rs:5206`

```rust
// BEFORE:
Ok(response_rx.await.unwrap_or(false))

// AFTER:
match tokio::time::timeout(Duration::from_secs(300), response_rx).await {
    Ok(Ok(approved)) => Ok(approved),
    Ok(Err(_)) => Ok(false),  // channel dropped (TUI exited)
    Err(_) => {
        tracing::warn!("[orchestrate] approval timed out after 5 minutes, auto-rejecting");
        Ok(false)
    }
}
```

**Also**: Import `use std::time::Duration;` if not already imported.

**Verification**: `cargo check -p roko-cli`

---

### C8: Emit token/cost EfficiencyEvents after dispatch

**File**: `crates/roko-cli/src/orchestrate.rs` — after `dispatch_agent_with()` returns successfully.

Search for where `AgentOutput` event is emitted (we added this earlier in this session) —
the EfficiencyEvents should go right next to it.

```rust
// After the AgentOutput emission:
for (metric, value) in [
    ("input_tokens", result.usage.input_tokens as f64),
    ("output_tokens", result.usage.output_tokens as f64),
    ("cost_usd", f64::from(result.usage.cost_usd)),
] {
    self.publish_dashboard_event(roko_core::DashboardEvent::EfficiencyEvent {
        plan_id: plan_id.to_string(),
        task_id: task.to_string(),
        metric: metric.to_string(),
        value,
    });
}
```

**Note**: Also emit a "model" EfficiencyEvent with the model name as the value
(but only if C5 is also done to handle it in apply()):
```rust
self.publish_dashboard_event(roko_core::DashboardEvent::EfficiencyEvent {
    plan_id: plan_id.to_string(),
    task_id: task.to_string(),
    metric: "model".to_string(),
    value: 0.0,  // model name needs different encoding — see C5 note
});
```
Actually, skip the "model" metric — C1 handles it via AgentSpawned.

**Verification**: `cargo check -p roko-cli`

---

### C9: Remove or simplify the dual event path

**Current state**: Two ways to publish events:
- `emit_server_event()` → ServerEvent → lossy conversion → DashboardEvent
- `publish_dashboard_event()` → DashboardEvent directly

**Recommended**: For `plan run --approval`, all events should go through
`publish_dashboard_event()` directly. The ServerEvent path exists for the HTTP
control plane which ISN'T RUNNING during `plan run`.

**Approach**:
1. In `emit_server_event()` (line 5118), check if HTTP server is active
2. If not: skip ServerEvent, publish DashboardEvent directly
3. If yes: keep current behavior for WebSocket/SSE consumers

**Or simpler**: Make `emit_server_event()` ALSO publish the DashboardEvent directly
(before or instead of converting). This ensures no data is lost in conversion.

This is a larger refactor — defer to a later session.

---

## Checklist

### Priority 1 — Fixes what users see immediately
- [ ] **C1**: Add `model: String` to AgentSpawned (4-6 files, ~20 lines changed)
- [ ] **C2**: Add `title: String` to ExecutionEvent::TaskStarted + conversion (3 files)
- [ ] **C3**: Fix TaskEntry.name to use title (1 file, 1 line)
- [ ] **C4**: Fix agent-to-task binding (role vs phase mismatch) (1 file, ~5 lines)
- [ ] **C8**: Emit token/cost EfficiencyEvents after dispatch (1 file, ~10 lines)

### Priority 2 — Reliability
- [ ] **C7**: Add 5-minute timeout on approval (1 file, ~5 lines)
- [ ] **C6**: Fix log bar garbling (1 file, ~10 lines)

### Priority 3 — Cleanup
- [ ] **C5**: Fix or remove EfficiencyEvent("model") skip (1 file, ~5 lines)
- [ ] **C9**: Simplify dual event path (larger refactor, defer)

---

## Verification After All Changes

```bash
cargo check --workspace
cargo +nightly fmt --all
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace

# Smoke test:
env -u CLAUDECODE cargo run -p roko-cli -- plan run .roko/plans/unified-migration-phase0 --approval
# Verify:
# - Task titles show human names (not IDs)
# - Agent model shows "claude-sonnet-4-6" (not "-")
# - Token counters update after agent dispatch
# - Log bar at bottom is clean (not garbled)
# - Approval modal works, timeout doesn't fire during normal use
```

---

## Why Mori Doesn't Have These Problems

Mori has NO StateHub indirection. In mori's `sequential.rs`:

```rust
// Agent event arrives:
AgentEvent::MessageDelta { content, .. } => {
    state.agent_state_mut(role).output.push_str(&content);
    // Done. No conversion, no event, no snapshot.
}

// TUI renders directly:
terminal.draw(|f| tui::render(f, &state))?;
```

Everything is a direct mutation on `RunState` from the same event loop.
Fields are set from source data, not relayed through serialized events.

The runner v2 (being built in the other session) adopts this pattern.
These C1-C9 fixes patch the old architecture to work. The runner v2
will eliminate the root cause.
