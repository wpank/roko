# Runtime Fixes Checklist (Post-Dogfood)

> **Context**: Nunchi's agent runtime ("Roko") has been dogfooded across 3 plan-runner runs. All P0 blockers are fixed on branch `wp-arch2`. This checklist covers the remaining P1/P2 issues that affect production readiness and demo quality.
>
> **Codebase**: `/Users/will/dev/nunchi/roko/roko/`
> **Key file**: `crates/roko-cli/src/orchestrate.rs` (~21K lines — the main orchestration loop)
> **TUI**: `crates/roko-cli/src/tui/`
> **Agent dispatch**: `crates/roko-agent/src/dispatcher/mod.rs`
> **Dogfood findings**: `tmp/dogfood/` (8 docs from 3 runs)
> **Branch**: `wp-arch2` (all P0 fixes already merged here)

---

## Batch 1: Agent Output Pipeline (makes TUI useful)

These are all connected — the root cause is that Roko uses a batch pipeline (wait for agent exit, collect result) instead of Mori's streaming pipeline (parse stdout per-line, emit events in real-time). Full streaming is a larger refactor; these fixes get partial visibility without it.

### 1A: Emit AgentOutput event after dispatch

**Problem**: `ServerEvent::AgentOutput` is defined, the converter to `DashboardEvent::AgentOutput` exists, but `emit_server_event(ServerEvent::AgentOutput { ... })` is NEVER CALLED — dead code.

**Fix**:
- [ ] In `orchestrate.rs`, find where agent dispatch completes (after `spawn_agent_with_layer` returns)
- [ ] Add `emit_server_event(ServerEvent::AgentOutput { task_id, output: agent_result.output.clone() })`
- [ ] Verify: TUI shows agent output text after each dispatch

**Effort**: ~5 lines of code. Biggest impact-to-effort ratio of any fix.

---

### 1B: Embed model name in AgentSpawned event

**Problem**: TUI shows "-" for model column because model name comes from `efficiency.jsonl` which doesn't exist during run.

**Fix**:
- [ ] When spawning an agent, include the selected model name in the `ServerEvent::AgentSpawned` event
- [ ] Update `DashboardEvent` handler to store model name immediately
- [ ] Verify: TUI shows "claude-haiku" or similar as soon as agent starts

**Effort**: ~10 lines.

---

### 1C: Emit EfficiencyUpdate after each dispatch

**Problem**: Token/cost columns show "0k/$0.00" for entire run.

**Fix**:
- [ ] After each agent dispatch completes, emit `DashboardEvent::EfficiencyUpdate` with tokens consumed and cost
- [ ] Read from the `AgentResult.usage` field (already populated)
- [ ] Verify: TUI shows running token count and cost during execution

**Effort**: ~15 lines.

---

### 1D: Add task title to TaskState

**Problem**: TUI shows "plan plan" instead of task title because `TaskState` has no `title` field.

**Fix**:
- [ ] Add `title: String` to `TaskState` struct in `dashboard_snapshot.rs`
- [ ] Populate from `TaskDef.title` when creating task state
- [ ] Update TUI rendering to use `task.title` instead of `task.task_id`
- [ ] Verify: TUI shows actual task names ("Wire SystemPromptBuilder" etc.)

**Effort**: ~10 lines. Already partially addressed in wp-arch2 (F4).

---

## Batch 2: Persistence During Run (crash safety)

### 2A: Periodic flush of learning state

**Problem**: Gate thresholds, cascade router, experiments only persisted at shutdown. Crash = total loss.

**Fix**:
- [ ] After each task completion, flush: `gate-thresholds.json`, `cascade-router.json`, `experiments.json`
- [ ] OR add a periodic flush timer (every 60 seconds)
- [ ] Verify: kill the process mid-run, restart — learning state preserved

---

### 2B: Signals substrate flush

**Problem**: `signals.jsonl` stays at 0 lines during entire run.

**Fix**:
- [ ] Trace where signals are supposed to be written (search for `SignalSubstrate` or `FileSubstrate` in the plan runner path)
- [ ] Either the substrate isn't being called, or writes are buffered without flush
- [ ] Add explicit `flush()` after each signal write
- [ ] Verify: `signals.jsonl` grows during run

---

## Batch 3: Input Parsing

### 3A: Strip markdown fences from TOML

**Problem**: LLMs wrap TOML output in ````toml ... ```` fences. Parser fails with "invalid table header."

**Fix**:
```rust
fn strip_code_fences(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.starts_with("```") {
        let without_opening = trimmed.splitn(2, '\n').nth(1).unwrap_or(trimmed);
        if without_opening.ends_with("```") {
            return without_opening[..without_opening.len()-3].trim().to_string();
        }
    }
    input.to_string()
}
```
- [ ] Add this function (or equivalent) near the TOML parsing code
- [ ] Call it before every `toml::from_str()` in the enrichment path
- [ ] Test with: ````toml\n[task]\nname = "test"\n```` → `[task]\nname = "test"`

---

### 3B: Increase enrichment timeout

**Problem**: 120s timeout too short for plans with 16+ files.

**Fix**:
- [ ] Find the timeout constant (likely in `orchestrate.rs` or agent dispatch config)
- [ ] Change from 120s to 300s
- [ ] Make configurable via `roko.toml` (`[orchestrator] enrichment_timeout_s = 300`)

---

## Batch 4: TUI Fixes

### 4A: Log routing through ring buffer

**Problem**: `tracing` subscriber writes to stderr while TUI has raw mode active, causing garbled output.

**Fix**:
- [ ] Create an in-memory ring buffer for log messages (like Mori's log tab)
- [ ] Route `tracing` subscriber output to the ring buffer instead of stderr
- [ ] Render the ring buffer contents in the TUI log tab
- [ ] Reference: Mori's implementation at `/Users/will/dev/uniswap/bardo/apps/mori/` if needed

---

## Batch 5: Memory

### 5A: Memory leak in enrichment

**Problem**: 9.5GB RSS after 17 minutes with 3 enrichment dispatches.

**Fix**:
- [ ] Profile with DHAT: `cargo run --features dhat-heap`
- [ ] Likely cause: enrichment artifact strings held in `TaskTracker` after being applied
- [ ] Fix: clear artifact strings after enrichment completes (`task.enrichment_artifacts.clear()`)
- [ ] Alternative: use `Arc<str>` or `Cow<'_, str>` for large strings
- [ ] Verify: RSS stays under 500MB for 30-minute run

---

## Batch 6: Missing HTTP Endpoints

### 6A: Individual plan detail route

- [ ] Add `GET /api/plans/:id` returning plan metadata + task list
- [ ] Add `GET /api/plans/:id/tasks` returning tasks with status

### 6B: Knowledge HTTP endpoint

- [ ] Add `GET /api/knowledge` returning knowledge entries from NeuroStore
- [ ] 14 entries exist in the store but no HTTP route exposes them

### 6C: Cascade router endpoint

- [ ] Fix `GET /api/learn/router` (currently returns 404)
- [ ] Should return current router state from `.roko/learn/cascade-router.json`

### 6D: Executor state endpoint

- [ ] Add `GET /api/executor/state` returning current plan execution state
- [ ] Read from `.roko/state/executor.json`

---

## Testing

After each batch:
```bash
cargo +nightly fmt --all
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace
```

Run a dogfood test:
```bash
cargo run -p roko-cli -- plan run .roko/plans/ --resume .roko/state/executor.json
```

Verify TUI shows: agent names (not "plan plan"), model names (not "-"), token counts (not "0k"), cost (not "$0.00"), and log output (not garbled).
