# 143 — Dream Consolidation Trigger After Plan Completion

**Priority**: P2 — `DreamRunner` and `PlanCompletionTriggerPolicy` exist but dreams run only via manual CLI; automatic post-plan consolidation is required for episodes to become knowledge/playbook/routing recommendations, which is the core of the self-improvement loop.
**Size**: S (1 day)
**Crates**: `crates/roko-dreams/src/`, `crates/roko-cli/src/runner/event_loop.rs`
**Depends on**: Backlog #83 (Dream Consolidation Deadlock must be fixed first)
**Sources**: `tmp/backlog/_mori-diffs-gaps.md` §F-4 (suggested 127), `tmp/backlog/_mori-old-gaps.md` MO-23

---

## Background

`roko-dreams` implements offline consolidation: episodes (agent turns and their outcomes) are processed in a "dream cycle" that extracts patterns, updates playbooks, and promotes knowledge from `Transient` to `Working` tier. This is the mechanism by which roko learns from past runs.

The infrastructure is present:
- `DreamRunner` — runs the consolidation cycle.
- `DreamTriggerSink` — receives trigger events.
- `PlanCompletionTriggerPolicy` — a policy that should fire when a plan completes.

However, dreams only run when the operator manually calls `roko knowledge dream run`. There is no automatic trigger from the runner event loop. After a successful plan run, no consolidation happens until the operator explicitly runs the command.

This means the self-improvement loop is broken: episodes accumulate but never become knowledge or playbook rules without manual intervention.

## Current State

- `crates/roko-dreams/src/` — `DreamRunner`, `DreamTriggerSink`, `PlanCompletionTriggerPolicy` present.
- `crates/roko-cli/src/runner/event_loop.rs` — runner completes plans but does not invoke `DreamTriggerSink`.
- Backlog #83 — covers the deadlock bug in the dream consolidation path; must be fixed before this item.
- `.roko/dreams/journal.jsonl` — dream lifecycle events; currently only populated by manual runs.

## Implementation Plan

1. **Define the trigger policy**: In `roko.toml`, add a `[learning.dreams]` section:
   ```toml
   [learning.dreams]
   trigger_on_plan_complete = true
   trigger_after_n_episodes = 50  # also trigger when episode count crosses 50
   idle_trigger_after_secs = 300  # also trigger after 5 minutes of runner idle
   max_concurrent = 1  # only one dream cycle at a time
   ```

2. **Wire trigger from runner**: In `event_loop.rs`, when `RunnerEvent::PlanCompleted` is emitted and `learning.dreams.trigger_on_plan_complete = true`:
   - Check if a dream cycle is already running (use a `DreamRunning` flag).
   - If not running: spawn a background task running `DreamRunner::run_cycle()`.
   - Set `DreamRunning = true`; clear when the background task completes.
   - Emit `RunnerEvent::DreamStarted` and `RunnerEvent::DreamCompleted` events.

3. **Episode count trigger**: Track episode count in runner state. When the count crosses `trigger_after_n_episodes`, trigger the same way.

4. **Idle trigger**: Add a periodic check in the event loop: if no tasks have started in the last `idle_trigger_after_secs` seconds and there are new episodes since the last dream, trigger.

5. **Non-blocking**: Dream consolidation must run in a background `tokio::spawn` so it does not block the runner event loop. Gate the next plan on dream completion only if `[learning.dreams] gate_next_plan = true` (default false, because dreams can be slow).

6. **Emit lifecycle events**: Write dream lifecycle events to `.roko/dreams/journal.jsonl` with: trigger reason, episode count at trigger, start time, end time, knowledge entries created, playbook rules updated.

7. **Proof**: After a successful plan, verify that `.roko/dreams/journal.jsonl` has a new entry, and that `.roko/neuro/knowledge.jsonl` or `.roko/learn/playbook.json` has new entries created by the dream cycle.

## Acceptance Criteria

1. After a plan completes, dream consolidation starts automatically (if `trigger_on_plan_complete = true`).
2. Dream runs in the background; the runner continues with the next plan during the dream.
3. `.roko/dreams/journal.jsonl` has a new entry after each automatic trigger.
4. `roko knowledge query "<task-topic>"` returns entries after the dream completes.
5. Manual `roko knowledge dream run` still works when automatic trigger is enabled.
6. Dream does not start if one is already running.

## Verification Checklist

- [ ] Set `trigger_on_plan_complete = true`; run a plan; verify `journal.jsonl` has a new entry.
- [ ] Verify the runner continues with the next plan while the dream runs (overlapping timestamps in `events.jsonl`).
- [ ] After dream completes, verify `knowledge.jsonl` or `playbook.json` has new entries.
- [ ] Run two plans consecutively; verify dream runs exactly once per completion, not twice.
- [ ] Run `roko knowledge dream run` while a background dream is in progress; verify second run is skipped.

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/src/runner/event_loop.rs` | Wire `PlanCompleted` to `DreamTriggerSink`; spawn background dream task |
| `crates/roko-dreams/src/` | Verify `DreamRunner::run_cycle()` is non-blocking and returns a handle |
| `crates/roko-core/src/config/` | Add `[learning.dreams]` config section |
| `crates/roko-cli/src/runner/types.rs` | Add `DreamStarted`/`DreamCompleted` event variants |
