# 141 — Per-Turn Efficiency Events (Not Just Per-Task Summaries)

**Priority**: P2 — Per-task efficiency events are too coarse to track which prompt sections were effective during a specific agent turn; per-turn granularity enables the section effectiveness loop and accurate mid-task cost attribution.
**Size**: XS (1-2 hours)
**Crates**: `crates/roko-cli/src/runner/event_loop.rs`
**Depends on**: None
**Sources**: `tmp/backlog/_mori-diffs-gaps.md` §F-1 (suggested 124)

---

## Background

`AgentEfficiencyEvent` is emitted once per task completion. Each event carries token counts, cost, prompt section metadata, and a HDC fingerprint. This is sufficient for post-task analysis but too coarse for two important use cases:

1. **Section effectiveness tracking**: `SectionEffectivenessRegistry` learns which prompt sections correlate with gate passes. If a task has 5 agent turns before passing the gate, a single per-task event cannot distinguish which turn's prompt composition led to the winning approach.

2. **Mid-task cost attribution**: The cost of a multi-turn task appears only at completion. Operators watching a long-running task have no per-turn cost signal.

Mori emitted per-turn efficiency events: every `TurnCompleted` event from the agent stream triggered an efficiency snapshot capturing the delta tokens and delta cost for that turn. This is mechanically straightforward: the agent stream already delivers turn-level token counts.

## Current State

- `crates/roko-cli/src/runner/event_loop.rs` — handles `AgentEvent::TurnCompleted` events from the agent stream. Each `TurnCompleted` carries `input_tokens_delta`, `output_tokens_delta`, `cost_delta_usd`.
- `AgentEfficiencyEvent` — emitted once at task completion; struct is defined in `roko-learn`.
- `.roko/learn/efficiency.jsonl` — where efficiency events are appended.
- No per-turn event is emitted in the current event handler for `TurnCompleted`.

## Implementation Plan

1. **Emit `AgentEfficiencyEvent` on each `TurnCompleted`**: In the `AgentEvent::TurnCompleted` handler in `event_loop.rs`:
   ```rust
   let per_turn_event = AgentEfficiencyEvent {
       task_id: current_task.id.clone(),
       turn_number: current_turn_count,
       is_final_turn: false,   // set to true only on task completion
       input_tokens: turn_result.input_tokens_delta,
       output_tokens: turn_result.output_tokens_delta,
       cost_usd: turn_result.cost_delta_usd,
       prompt_sections: Vec::new(),  // not available per-turn; populated on final turn
       hdc_fingerprint: None,  // not available per-turn
       ..Default::default()
   };
   learning_runtime.append_efficiency_event(per_turn_event).await?;
   ```

2. **Mark the final turn**: Keep the existing per-task event emission at task completion. On the final turn, set `is_final_turn: true` and populate `prompt_sections` and `hdc_fingerprint`. This way, the final turn event is the "rich" event and earlier turn events are "lightweight" cost records.

3. **`turn_number` counter**: Add `current_turn_count: u32` to the per-task runner state and increment on each `TurnCompleted`. Reset to 0 when a new task starts.

4. **Avoid double-counting**: The per-task summary event already includes total cost. Tools consuming efficiency events must handle both per-turn events (`is_final_turn: false`) and the summary event (`is_final_turn: true`). Document the distinction in `AgentEfficiencyEvent`.

5. **Add `AgentEfficiencyEvent::is_final_turn` field**: Add this boolean to the struct with a default of `true` for backward compatibility (existing single-event emitters are unaffected).

## Acceptance Criteria

1. After a multi-turn task, `.roko/learn/efficiency.jsonl` has N+1 entries for that task (N per-turn events + 1 final event).
2. Per-turn events have `is_final_turn: false` and non-zero `input_tokens` and `output_tokens`.
3. The final event has `is_final_turn: true` and populated `prompt_sections`.
4. Total cost across per-turn events equals the cost in the final event.
5. Single-turn tasks still produce exactly 1 efficiency event (the final-turn event).

## Verification Checklist

- [ ] Run a task that requires 3 agent turns (e.g., requires review cycle); verify 4 efficiency events (3 per-turn + 1 final).
- [ ] Verify each per-turn event has `is_final_turn: false`.
- [ ] Verify the final event has `is_final_turn: true` and non-empty `prompt_sections`.
- [ ] Verify per-turn cost sum matches final event total cost.

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/src/runner/event_loop.rs` | Emit per-turn efficiency event on each `TurnCompleted` |
| `crates/roko-learn/src/` | Add `is_final_turn: bool` to `AgentEfficiencyEvent`; default `true` |
