# Live token progress is not live

- Severity: medium
- Status: reproduced
- Area: agent telemetry

## Observation

The agent row shows `0k/200k` throughout long turns. The 200k value is a default (`crates/roko-cli/src/tui/state.rs:345-361`), while counters start at zero on spawn (`dashboard_snapshot.rs:1031-1041`) and update only on `AgentEvent::TokenUsage` (`runner/agent_events.rs:97-122`). Providers commonly emit final usage only when the turn ends.

Mori parses token notifications continuously and updates per-instance, per-role, and cumulative metrics in `apps/mori/src/app/parallel.rs:11934-11970`.

## Expected

Show prompt-estimated input immediately, update usage from streaming provider events where available, and label estimated versus final counts. A live turn should not appear to have consumed zero context.

