# W12-C: Event Loop Safety -- DAG Sort, Output Cap, Epoch Fix, Timeout Guard, Hook Roles

**Priority**: P2 -- correctness (five event loop defects)
**Effort**: 1-2 hours
**Files to modify**: 4 files
**Dependencies**: None

## Cross-Batch Overlap Warning (W12 event_loop.rs)

All four W12 batches touch `event_loop.rs`. This batch (W12-C) overlaps with W12-B:

- **Lines 1009-1022 (Branch 5 timeout)**: W12-B changes `&mut agent_handle` to `&mut agent_handles` on line 1015. W12-C adds an `if !timed_out` guard to the branch and `timed_out = true;` after the call. These are interacting: if both are applied, the branch should read `_ = &mut plan_timeout, if !timed_out => {` AND the `handle_plan_timeout` call should pass `&mut agent_handles` (not `&mut agent_handle`). **If W12-B is applied first**, the "Find this code" blocks in Change 8 will not match because `agent_handle` was renamed to `agent_handles`. The agent should search for the actual current code.
- **Lines 1039-1052 (post-select timeout)**: Same interaction. W12-B renames the variable; W12-C adds a guard.

**Apply order**: Best to apply W12-C before W12-B, or apply both simultaneously. If W12-B is applied first, the agent must adapt the "Find this code" blocks for Changes 8 accordingly.

This batch also modifies `state.rs` (adding `start_epoch_ms`). W12-B also modifies `state.rs` (changing `iteration` to `iterations`). These are non-conflicting: different fields.

## Problem

Five independent correctness issues in the event loop:

1. **Sentinel task DAG sort uses string comparison**: `a.id.cmp(&b.id)` is lexicographic -- `"T1", "T10", "T2"` sort as `T1, T10, T2`. Named tasks execute in alphabetical order, not definition order.

2. **agent_output grows unbounded**: `state.agent_output` accumulates all text with no cap. A long-running agent can accumulate 10-100 MB.

3. **started_at_ms is elapsed, not epoch**: `started_at_ms: state.started_at.elapsed().as_millis()` computes elapsed duration, not an epoch timestamp.

4. **Plan timeout can fire twice**: Branch 5 fires in `select!`, AND a post-select check can also trigger `handle_plan_timeout`. Duplicate shutdown events result.

5. **Extension hooks hardcode role**: Both `fire_pre_inference_hook` and `fire_post_inference_hook` set `role: "implementer".to_string()` regardless of actual task role.

## Exact Code to Change

### File 1: `crates/roko-cli/src/task_parser.rs`

#### Change 1: Add `sequence` field to `TaskDef`

**Find this code** (line 96):
```rust
    /// Work domain — controls gate selection and git policy.
    pub domain: Option<TaskDomain>,
}
```

**Replace with:**
```rust
    /// Work domain — controls gate selection and git policy.
    pub domain: Option<TaskDomain>,
    /// Definition order index (0-based) from the TOML array. Used for
    /// tie-breaking in DAG resolution so tasks without dependency constraints
    /// execute in the order they were authored, not alphabetically.
    #[serde(default)]
    pub sequence: usize,
}
```

#### Change 2: Set `sequence` in `From<TaskDefSerde>` conversion

**Find this code** (line 183):
```rust
            domain: raw.domain,
        };
        task.apply_role_tool_defaults();
```

**Replace with:**
```rust
            domain: raw.domain,
            sequence: 0, // stamped by TasksFile::parse_str after deserialization
        };
        task.apply_role_tool_defaults();
```

#### Change 3: Stamp `sequence` from array index in `parse_str` and `parse_agent_output`

**Find this code** (line 652):
```rust
    pub fn parse_str(content: &str) -> Result<Self> {
        toml::from_str(content).context("parse tasks.toml")
    }

    /// Parse a `tasks.toml` payload returned inline by an agent.
    pub fn parse_agent_output(content: &str) -> Result<Self> {
        let payload = extract_toml_payload(content);
        Self::parse_str(&payload).context("parse tasks.toml from agent output")
    }
```

**Replace with:**
```rust
    pub fn parse_str(content: &str) -> Result<Self> {
        let mut parsed: Self = toml::from_str(content).context("parse tasks.toml")?;
        // Stamp definition order so DAG sort preserves author intent.
        for (i, task) in parsed.tasks.iter_mut().enumerate() {
            task.sequence = i;
        }
        Ok(parsed)
    }

    /// Parse a `tasks.toml` payload returned inline by an agent.
    pub fn parse_agent_output(content: &str) -> Result<Self> {
        let payload = extract_toml_payload(content);
        Self::parse_str(&payload).context("parse tasks.toml from agent output")
    }
```

### File 2: `crates/roko-cli/src/runner/event_loop.rs`

#### Change 4: Fix sentinel task sort to use sequence

**Find this code** (line 1980):
```rust
                    let mut all_tasks: Vec<&TaskDef> = tasks.values().collect();
                    all_tasks.sort_by(|a, b| a.id.cmp(&b.id));
```

**Replace with:**
```rust
                    let mut all_tasks: Vec<&TaskDef> = tasks.values().collect();
                    all_tasks.sort_by_key(|t| t.sequence);
```

#### Change 5: Fix secondary sort in sentinel task fallback

**Find this code** (line 1994):
```rust
                        .and_then(|tasks| tasks.values().min_by(|a, b| a.id.cmp(&b.id)))
```

**Replace with:**
```rust
                        .and_then(|tasks| tasks.values().min_by_key(|t| t.sequence))
```

#### Change 6: Fix RunVerify task sort

**Find this code** (line 2575):
```rust
                    let mut tasks: Vec<_> = tasks.values().collect();
                    tasks.sort_by(|a, b| a.id.cmp(&b.id));
```

**Replace with:**
```rust
                    let mut tasks: Vec<_> = tasks.values().collect();
                    tasks.sort_by_key(|t| t.sequence);
```

#### Change 7: Fix `started_at_ms` to use epoch timestamp

**Find this code** (line 1640):
```rust
        started_at_ms: state.started_at.elapsed().as_millis().saturating_sub(0) as u64,
```

**Replace with:**
```rust
        started_at_ms: state.start_epoch_ms,
```

#### Change 8: Guard plan timeout against double-fire

Add a `timed_out` flag before the main `loop {` (around line 396). Find:

```rust
    loop {
        // Cancel-safety analysis:
```

Insert before it:
```rust
    let mut timed_out = false;

```

Then guard both timeout paths.

**Find this code** (Branch 5, line 1009):
```rust
            // ─── Branch 5: Plan timeout ──────────────────────────────
            _ = &mut plan_timeout => {
```

**Replace with:**
```rust
            // ─── Branch 5: Plan timeout ──────────────────────────────
            _ = &mut plan_timeout, if !timed_out => {
```

After the `handle_plan_timeout` call in Branch 5 (line ~1022), before the closing `}`:
```rust
                timed_out = true;
```

**Find this code** (post-select check, line 1039):
```rust
        if tokio::time::Instant::now() >= plan_deadline {
```

**Replace with:**
```rust
        if !timed_out && tokio::time::Instant::now() >= plan_deadline {
```

After the `handle_plan_timeout` call in the post-select check, add:
```rust
            timed_out = true;
```

#### Change 9: Add `role` parameter to `fire_pre_inference_hook`

**Find this code** (line 2829):
```rust
async fn fire_pre_inference_hook(
    config: &RunConfig,
    plan_id: &str,
    task_id: &str,
    model: &str,
    tui: &TuiBridge,
) {
```

**Replace with:**
```rust
async fn fire_pre_inference_hook(
    config: &RunConfig,
    plan_id: &str,
    task_id: &str,
    model: &str,
    role: &str,
    tui: &TuiBridge,
) {
```

**Find this code** (line 2846):
```rust
        role: "implementer".to_string(),
```

(inside `fire_pre_inference_hook` -- the `InferenceRequest` construction)

**Replace with:**
```rust
        role: role.to_string(),
```

#### Change 10: Add `role` parameter to `fire_post_inference_hook`

**Find this code** (line 2859):
```rust
async fn fire_post_inference_hook(
    config: &RunConfig,
    plan_id: &str,
    task_id: &str,
    model: &str,
    success: bool,
    cost_usd: f64,
    wall_ms: u64,
    tui: &TuiBridge,
) {
```

**Replace with:**
```rust
async fn fire_post_inference_hook(
    config: &RunConfig,
    plan_id: &str,
    task_id: &str,
    model: &str,
    role: &str,
    success: bool,
    cost_usd: f64,
    wall_ms: u64,
    tui: &TuiBridge,
) {
```

**Find this code** (line 2879):
```rust
        role: "implementer".to_string(),
```

(inside `fire_post_inference_hook` -- the `InferenceResponse` construction)

**Replace with:**
```rust
        role: role.to_string(),
```

#### Change 11: Update pre-inference hook call site

The call site is in `dispatch_action` around line 2226. Find it by searching for `fire_pre_inference_hook(`:

**Find this code:**
```rust
            fire_pre_inference_hook(ctx.config, plan_id, &task_id, &requested_model, ctx.tui).await;
```

Note: This line is long. It may be formatted differently. Search for `fire_pre_inference_hook(ctx.config` to find it.

**Replace with:**
```rust
            let task_role = task_def.role.as_deref().unwrap_or("implementer");
            fire_pre_inference_hook(ctx.config, plan_id, &task_id, &requested_model, task_role, ctx.tui).await;
```

#### Change 12: Update post-inference hook call site

**Find this code** (line 466):
```rust
                    fire_post_inference_hook(
                        config,
                        &state.plan_id,
                        &state.current_task,
                        &state.agent_model,
                        !turn_error,
                        state.cost_usd,
                        state.task_elapsed_ms(),
                        &tui,
                    )
                    .await;
```

**Replace with:**
```rust
                    let task_role = task_index
                        .get(state.plan_id.as_str())
                        .and_then(|tasks| tasks.get(state.current_task.as_str()))
                        .and_then(|t| t.role.as_deref())
                        .unwrap_or("implementer");
                    fire_post_inference_hook(
                        config,
                        &state.plan_id,
                        &state.current_task,
                        &state.agent_model,
                        task_role,
                        !turn_error,
                        state.cost_usd,
                        state.task_elapsed_ms(),
                        &tui,
                    )
                    .await;
```

### File 3: `crates/roko-cli/src/runner/agent_events.rs`

#### Change 13: Cap agent_output growth

Add constant after imports (around line 13):

```rust
/// Maximum bytes retained in `agent_output`. When exceeded, the buffer is
/// trimmed to keep the tail (most recent output), which is what replan
/// context and diagnostics need.
const MAX_AGENT_OUTPUT: usize = 32_768;
```

**Find this code** (line 102):
```rust
        AgentEvent::MessageDelta { text } => {
            state.agent_output.push_str(text);
            let agent_id = agent_id_for_state(state);
            tui.agent_output(&agent_id, text);

            if stream_to_stderr {
                stream_buf.push(text);
            }
        }
```

**Replace with:**
```rust
        AgentEvent::MessageDelta { text } => {
            state.agent_output.push_str(text);
            if state.agent_output.len() > MAX_AGENT_OUTPUT {
                let trim_point = state.agent_output.len() - MAX_AGENT_OUTPUT / 2;
                let boundary = state.agent_output.ceil_char_boundary(trim_point);
                state.agent_output = format!(
                    "[...truncated {}B...]\n{}",
                    boundary,
                    &state.agent_output[boundary..],
                );
                debug!(
                    trimmed_to = state.agent_output.len(),
                    "agent_output exceeded cap, trimmed to tail"
                );
            }
            let agent_id = agent_id_for_state(state);
            tui.agent_output(&agent_id, text);

            if stream_to_stderr {
                stream_buf.push(text);
            }
        }
```

### File 4: `crates/roko-cli/src/runner/state.rs`

#### Change 14: Add `start_epoch_ms` field

**Find this code** (line 103):
```rust
    /// When the run started.
    pub started_at: Instant,
```

**Replace with:**
```rust
    /// When the run started.
    pub started_at: Instant,
    /// Epoch timestamp (ms since UNIX epoch) when the run started. Used in
    /// snapshots for cross-run comparisons and dashboard display.
    pub start_epoch_ms: u64,
```

#### Change 15: Initialize `start_epoch_ms` in `RunState::new`

**Find this code** (line 168):
```rust
            started_at: Instant::now(),
            task_started_at: Instant::now(),
```

**Replace with:**
```rust
            started_at: Instant::now(),
            start_epoch_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            task_started_at: Instant::now(),
```

## Verification

```bash
cd /Users/will/dev/nunchi/roko/roko

# Build check:
cargo check -p roko-cli 2>&1 | head -30

# Verify sequence field exists:
grep -n 'pub sequence' crates/roko-cli/src/task_parser.rs

# Verify no remaining hardcoded "implementer" in hooks:
grep -n '"implementer"' crates/roko-cli/src/runner/event_loop.rs
# Should only appear in unwrap_or("implementer") fallbacks, not in hook bodies

# Verify agent_output cap exists:
grep -n 'MAX_AGENT_OUTPUT' crates/roko-cli/src/runner/agent_events.rs

# Verify timeout guard:
grep -n 'timed_out' crates/roko-cli/src/runner/event_loop.rs

# Verify start_epoch_ms:
grep -n 'start_epoch_ms' crates/roko-cli/src/runner/state.rs
```

## Agent Prompt

```
Fix five event loop defects in the roko runner.

## Context

Read the full batch spec at `/Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W12-C-event-loop-safety.md` for exact find/replace blocks.

Files to modify:
- `crates/roko-cli/src/task_parser.rs` -- TaskDef struct, TasksFile::parse_str
- `crates/roko-cli/src/runner/event_loop.rs` -- sorts, timeout guard, hooks
- `crates/roko-cli/src/runner/agent_events.rs` -- output cap
- `crates/roko-cli/src/runner/state.rs` -- start_epoch_ms

## Summary

1. **TaskDef sequence field**: Add `pub sequence: usize` with `#[serde(default)]` to TaskDef (line ~96).
   Set `sequence: 0` in `From<TaskDefSerde>` (line ~183). Stamp from array index in `parse_str`
   (line ~652). Replace `a.id.cmp(&b.id)` sorts with `sort_by_key(|t| t.sequence)` at lines ~1980,
   ~1994, ~2575 in event_loop.rs.

2. **Agent output cap**: Add `const MAX_AGENT_OUTPUT: usize = 32_768;` to agent_events.rs.
   After `push_str` in `MessageDelta` handler (line ~102), check length and trim using
   `ceil_char_boundary` (stable in Rust 1.73+, toolchain is 1.95).

3. **Epoch timestamp**: Add `pub start_epoch_ms: u64` to RunState. Initialize from
   `SystemTime::now()`. Use it at line ~1640 instead of `state.started_at.elapsed()`.

4. **Timeout double-fire guard**: Add `let mut timed_out = false;` before the main loop.
   Guard Branch 5 with `if !timed_out`. Guard the post-select check with `if !timed_out`.
   Set `timed_out = true;` after each fires.

5. **Hook roles**: Add `role: &str` parameter to `fire_pre_inference_hook` (line ~2829) and
   `fire_post_inference_hook` (line ~2859). Replace hardcoded `"implementer"` with `role`.
   At pre-inference call site, derive from `task_def.role`. At post-inference call site,
   derive from `task_index`.

Run `cargo check -p roko-cli` to verify.
```

## Commit

This batch is committed with all Wave 12 batches together. Do not commit individually.

## Checklist

- [ ] `sequence: usize` field added to `TaskDef` with `#[serde(default)]`
- [ ] `sequence: 0` set in `From<TaskDefSerde>`
- [ ] `sequence` stamped from array index in `parse_str`
- [ ] Three task sorts use `sort_by_key(|t| t.sequence)` (lines ~1980, ~1994, ~2575)
- [ ] `MAX_AGENT_OUTPUT` constant added to `agent_events.rs`
- [ ] `agent_output` trimmed when exceeding cap (uses `ceil_char_boundary`)
- [ ] `tracing::debug!` on trim
- [ ] `start_epoch_ms: u64` added to `RunState` and initialized from `SystemTime`
- [ ] Snapshot uses `state.start_epoch_ms` instead of `state.started_at.elapsed()`
- [ ] `timed_out` flag added before main loop
- [ ] Branch 5 timeout guarded with `if !timed_out` and sets `timed_out = true`
- [ ] Post-select timeout guarded with `if !timed_out` and sets `timed_out = true`
- [ ] `role: &str` parameter added to `fire_pre_inference_hook`
- [ ] `role: &str` parameter added to `fire_post_inference_hook`
- [ ] Hardcoded `"implementer"` replaced with `role.to_string()` in both hooks
- [ ] Pre-inference call site passes `task_def.role.as_deref().unwrap_or("implementer")`
- [ ] Post-inference call site derives role from `task_index`
- [ ] `cargo check -p roko-cli` passes

## Audit Status

Audited: 2026-05-05. 1 issue fixed: added cross-batch overlap warning header documenting interactions with W12-B on timeout branches (lines 1009-1052) and state.rs. All code snippets verified against source -- exact matches confirmed. Line numbers accurate. `ceil_char_boundary` confirmed available (toolchain is 1.91+). `#[serde(default)]` on `sequence` is harmless (TaskDef uses custom Deserialize impl). `RunState::new` is the only RunState construction site -- covered. `task_index` confirmed in scope at post-inference hook call site (line 466).
